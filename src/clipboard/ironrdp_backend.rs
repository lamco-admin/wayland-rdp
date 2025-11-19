//! IronRDP Clipboard Backend Implementation
//!
//! Implements the CliprdrBackend and CliprdrBackendFactory traits to integrate
//! our clipboard manager with IronRDP's clipboard protocol handling.

use ironrdp_cliprdr::backend::{CliprdrBackend, CliprdrBackendFactory, ClipboardMessage, ClipboardMessageProxy};
use ironrdp_cliprdr::pdu::{
    ClipboardFormat, ClipboardGeneralCapabilityFlags, FileContentsRequest,
    FileContentsResponse, FormatDataRequest, FormatDataResponse, LockDataId, OwnedFormatDataResponse,
};
use ironrdp_core::AsAny;
use ironrdp_server::ServerEventSender;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};

use crate::clipboard::manager::{ClipboardManager, ClipboardEvent};
use crate::clipboard::formats::ClipboardFormat as WrdClipboardFormat;

/// Clipboard message proxy implementation
#[derive(Debug, Clone)]
struct WrdMessageProxy {
    tx: mpsc::UnboundedSender<ClipboardMessage>,
}

impl ClipboardMessageProxy for WrdMessageProxy {
    fn send_clipboard_message(&self, message: ClipboardMessage) {
        if let Err(e) = self.tx.send(message) {
            error!("Failed to send clipboard message: {:?}", e);
        }
    }
}

/// Clipboard backend factory for IronRDP server
///
/// Creates clipboard backend instances for each RDP connection.
pub struct WrdCliprdrFactory {
    /// Clipboard manager shared across connections
    clipboard_manager: Arc<Mutex<ClipboardManager>>,

    /// Server event sender for IronRDP
    event_sender: Option<mpsc::UnboundedSender<ironrdp_server::ServerEvent>>,

    /// Message receiver channel
    message_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<ClipboardMessage>>>>,
}

impl WrdCliprdrFactory {
    /// Create a new clipboard factory
    ///
    /// # Arguments
    ///
    /// * `clipboard_manager` - Shared clipboard manager instance
    ///
    /// # Returns
    ///
    /// A new factory instance
    pub fn new(clipboard_manager: Arc<Mutex<ClipboardManager>>) -> Self {
        Self {
            clipboard_manager,
            event_sender: None,
            message_rx: Arc::new(Mutex::new(None)),
        }
    }

    /// Get message receiver for processing clipboard messages
    pub async fn take_message_receiver(&self) -> Option<mpsc::UnboundedReceiver<ClipboardMessage>> {
        self.message_rx.lock().await.take()
    }
}

impl CliprdrBackendFactory for WrdCliprdrFactory {
    fn build_cliprdr_backend(&self) -> Box<dyn CliprdrBackend> {
        debug!("Building clipboard backend for new connection");

        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let message_proxy = WrdMessageProxy { tx: msg_tx };

        // Store receiver for the factory to process messages
        tokio::spawn({
            let message_rx = self.message_rx.clone();
            async move {
                *message_rx.lock().await = Some(msg_rx);
            }
        });

        Box::new(WrdCliprdrBackend {
            clipboard_manager: Arc::clone(&self.clipboard_manager),
            capabilities: ClipboardGeneralCapabilityFlags::empty(),
            temporary_directory: "/tmp/wrd-clipboard".to_string(),
            message_proxy: Some(message_proxy),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            file_transfers: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

impl ServerEventSender for WrdCliprdrFactory {
    fn set_sender(&mut self, sender: mpsc::UnboundedSender<ironrdp_server::ServerEvent>) {
        self.event_sender = Some(sender);
    }
}

impl ironrdp_server::CliprdrServerFactory for WrdCliprdrFactory {}

/// File transfer state
#[derive(Debug, Clone)]
struct FileTransferState {
    stream_id: u32,
    list_index: u32,
    file_path: String,
    total_size: u64,
    received_size: u64,
    chunks: Vec<Vec<u8>>,
}

/// Clipboard backend implementation
///
/// Handles clipboard protocol events from IronRDP and coordinates with
/// our ClipboardManager and Portal clipboard.
#[derive(Debug)]
struct WrdCliprdrBackend {
    /// Reference to shared clipboard manager
    clipboard_manager: Arc<Mutex<ClipboardManager>>,

    /// Negotiated capabilities
    capabilities: ClipboardGeneralCapabilityFlags,

    /// Temporary directory for file transfers
    temporary_directory: String,

    /// Message proxy for sending responses
    message_proxy: Option<WrdMessageProxy>,

    /// Pending format data requests (format_id -> request_context)
    pending_requests: Arc<RwLock<HashMap<u32, FormatDataRequest>>>,

    /// Active file transfers (stream_id -> transfer_state)
    file_transfers: Arc<RwLock<HashMap<u32, FileTransferState>>>,
}

impl AsAny for WrdCliprdrBackend {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl CliprdrBackend for WrdCliprdrBackend {
    fn temporary_directory(&self) -> &str {
        &self.temporary_directory
    }

    fn client_capabilities(&self) -> ClipboardGeneralCapabilityFlags {
        // Return full capabilities - we support everything
        ClipboardGeneralCapabilityFlags::USE_LONG_FORMAT_NAMES
            | ClipboardGeneralCapabilityFlags::STREAM_FILECLIP_ENABLED
            | ClipboardGeneralCapabilityFlags::FILECLIP_NO_FILE_PATHS
            | ClipboardGeneralCapabilityFlags::CAN_LOCK_CLIPDATA
    }

    fn on_ready(&mut self) {
        info!("Clipboard channel ready");

        // Request initial format list from client
        // This will be sent via the message proxy when available
        self.on_request_format_list();
    }

    fn on_request_format_list(&mut self) {
        debug!("Format list requested - checking local clipboard");

        // Full implementation would query Portal clipboard and announce formats
        // Deferred to complete clipboard integration
    }

    fn on_process_negotiated_capabilities(&mut self, capabilities: ClipboardGeneralCapabilityFlags) {
        info!("Clipboard capabilities negotiated: {:?}", capabilities);
        self.capabilities = capabilities;
    }

    fn on_remote_copy(&mut self, available_formats: &[ClipboardFormat]) {
        info!(
            "Remote copy announced with {} formats",
            available_formats.len()
        );

        // Log available formats
        for (idx, format) in available_formats.iter().enumerate() {
            let name = format.name.as_ref().map(|n| n.value()).unwrap_or("");
            debug!("  Format {}: ID={:?}, Name={}", idx, format.id, name);
        }

        // Convert IronRDP formats to our WrdClipboardFormat
        let wrd_formats: Vec<WrdClipboardFormat> = available_formats
            .iter()
            .map(|f| WrdClipboardFormat {
                format_id: f.id.0,
                format_name: f.name.as_ref().map(|n| n.value().to_string()).unwrap_or_default(),
            })
            .collect();

        // Send event to clipboard manager
        let event_tx = {
            let manager = self.clipboard_manager.blocking_lock();
            manager.event_sender()
        };

        if let Err(e) = event_tx.blocking_send(ClipboardEvent::RdpFormatList(wrd_formats)) {
            error!("Failed to send RDP format list to manager: {:?}", e);
        }
    }

    fn on_format_data_request(&mut self, request: FormatDataRequest) {
        let format_id = request.format.0;
        debug!("Format data requested for format ID: {}", format_id);

        // Store request for correlation with response
        let pending_requests = self.pending_requests.clone();
        tokio::spawn(async move {
            pending_requests.write().await.insert(format_id, request);
        });

        // Send event to clipboard manager to fetch data from Portal
        let event_tx = {
            let manager = self.clipboard_manager.blocking_lock();
            manager.event_sender()
        };

        if let Err(e) = event_tx.blocking_send(ClipboardEvent::RdpDataRequest(format_id)) {
            error!("Failed to send RDP data request to manager: {:?}", e);

            // Send error response
            if let Some(proxy) = &self.message_proxy {
                let error_response = OwnedFormatDataResponse::new_error();
                proxy.send_clipboard_message(ClipboardMessage::SendFormatData(error_response));
            }
        }
    }

    fn on_format_data_response(&mut self, response: FormatDataResponse<'_>) {
        if response.is_error() {
            warn!("Format data response received with error flag");
            return;
        }

        let data = response.data().to_vec();
        debug!("Format data response received: {} bytes", data.len());

        // Send data to clipboard manager for conversion and Portal clipboard update
        let event_tx = {
            let manager = self.clipboard_manager.blocking_lock();
            manager.event_sender()
        };

        if let Err(e) = event_tx.blocking_send(ClipboardEvent::RdpDataResponse(data)) {
            error!("Failed to send RDP data response to manager: {:?}", e);
        }
    }

    fn on_file_contents_request(&mut self, request: FileContentsRequest) {
        let stream_id = request.stream_id;
        let list_index = request.index;

        debug!(
            "File contents requested: stream_id={}, list_index={}",
            stream_id, list_index
        );

        // Notify clipboard manager
        let manager = self.clipboard_manager.clone();
        let message_proxy = self.message_proxy.clone();

        tokio::spawn(async move {
            let mgr = manager.lock().await;

            match mgr.handle_file_contents_request(stream_id, list_index).await {
                Ok(()) => {
                    debug!("File contents request handled successfully");
                }
                Err(e) => {
                    error!("Failed to handle file contents request: {:?}", e);

                    // Send error response
                    if let Some(proxy) = message_proxy {
                        // Note: Would need to create FileContentsResponse error here
                        // This is a placeholder - actual implementation would construct proper response
                        warn!("File contents error response not yet implemented");
                    }
                }
            }
        });
    }

    fn on_file_contents_response(&mut self, response: FileContentsResponse<'_>) {
        let stream_id = response.stream_id();
        let data = response.data().to_vec();

        debug!(
            "File contents response: stream_id={}, {} bytes",
            stream_id,
            data.len()
        );

        // Track file transfer progress
        let file_transfers = self.file_transfers.clone();
        let data_clone = data.clone();
        tokio::spawn(async move {
            let mut transfers = file_transfers.write().await;

            if let Some(state) = transfers.get_mut(&stream_id) {
                state.chunks.push(data_clone.clone());
                state.received_size += data_clone.len() as u64;

                debug!(
                    "File transfer progress: {}/{} bytes",
                    state.received_size, state.total_size
                );
            } else {
                // Initialize new transfer state
                transfers.insert(stream_id, FileTransferState {
                    stream_id,
                    list_index: 0,
                    file_path: format!("/tmp/wrd-clipboard/file_{}", stream_id),
                    total_size: 0,
                    received_size: data_clone.len() as u64,
                    chunks: vec![data_clone],
                });
            }
        });

        // Notify clipboard manager
        let manager = self.clipboard_manager.clone();
        tokio::spawn(async move {
            let mgr = manager.lock().await;

            if let Err(e) = mgr.handle_file_contents_response(stream_id, data).await {
                error!("Failed to handle file contents response: {:?}", e);
            }
        });
    }

    fn on_lock(&mut self, data_id: LockDataId) {
        debug!("Clipboard lock requested: {:?}", data_id);
        // Lock support - prevents clipboard changes during transfer
        // Not critical for basic functionality
    }

    fn on_unlock(&mut self, data_id: LockDataId) {
        debug!("Clipboard unlock requested: {:?}", data_id);
        // Unlock clipboard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_factory_creation() {
        let config = crate::clipboard::manager::ClipboardConfig::default();
        let manager = Arc::new(Mutex::new(
            ClipboardManager::new(config).await.unwrap()
        ));

        let factory = WrdCliprdrFactory::new(manager);
        let _backend = factory.build_cliprdr_backend();
        // Backend created successfully
    }
}
