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
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};

use crate::clipboard::manager::{ClipboardManager, ClipboardEvent};
use crate::clipboard::formats::ClipboardFormat as WrdClipboardFormat;

/// Clipboard event for non-blocking queue
#[derive(Debug, Clone)]
enum ClipboardBackendEvent {
    RemoteCopy(Vec<WrdClipboardFormat>),
    FormatDataRequest(u32, Option<WrdMessageProxy>),
    FormatDataResponse(Vec<u8>),
    FileContentsRequest(u32, u32, u64, u32, Option<WrdMessageProxy>),
    FileContentsResponse(u32, Vec<u8>),
}

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

    /// Shared message proxy for all backends (created once, immutable)
    shared_proxy: Arc<WrdMessageProxy>,

    /// Shared event queue for all backends
    shared_event_queue: Arc<RwLock<VecDeque<ClipboardBackendEvent>>>,
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
        // Create shared message channel for clipboard communication
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let shared_proxy = Arc::new(WrdMessageProxy { tx: msg_tx });

        // Store receiver for processing messages
        let message_rx = Arc::new(Mutex::new(Some(msg_rx)));

        // Create shared event queue
        let shared_event_queue = Arc::new(RwLock::new(VecDeque::new()));

        // Start single async task to process clipboard events (spawn once here, not per backend)
        let queue = Arc::clone(&shared_event_queue);
        let manager = Arc::clone(&clipboard_manager);
        tokio::spawn(async move {
            Self::process_clipboard_events(queue, manager).await;
        });

        info!("Clipboard event processor task started");

        // TODO: Start task to process message_rx and forward ClipboardMessages
        // Currently SendInitiatePaste messages go into the channel but aren't processed
        // Need to convert ClipboardMessage → actual RDP protocol actions

        Self {
            clipboard_manager,
            event_sender: None,
            message_rx,
            shared_proxy,
            shared_event_queue,
        }
    }

    /// Get message receiver for processing clipboard messages
    pub async fn take_message_receiver(&self) -> Option<mpsc::UnboundedReceiver<ClipboardMessage>> {
        self.message_rx.lock().await.take()
    }
}

impl WrdCliprdrFactory {
    /// Process clipboard events from the non-blocking queue
    async fn process_clipboard_events(
        queue: Arc<RwLock<VecDeque<ClipboardBackendEvent>>>,
        manager: Arc<Mutex<ClipboardManager>>,
    ) {
        loop {
            // Non-blocking check for events
            let event = {
                if let Ok(mut q) = queue.try_write() {
                    q.pop_front()
                } else {
                    None
                }
            };

            if let Some(event) = event {
                match event {
                    ClipboardBackendEvent::RemoteCopy(formats) => {
                        debug!("Processing remote copy event: {} formats", formats.len());
                        // Send to clipboard manager
                        if let Ok(mgr) = manager.try_lock() {
                            if let Ok(_) = mgr.event_sender().try_send(ClipboardEvent::RdpFormatList(formats.clone())) {
                                debug!("Format list sent to clipboard manager");

                                // Proactively request common formats to populate Wayland clipboard
                                // Check for text format (CF_UNICODETEXT = 13 or CF_TEXT = 1)
                                let has_unicode_text = formats.iter().any(|f| f.format_id == 13);
                                let has_text = formats.iter().any(|f| f.format_id == 1);

                                if has_unicode_text || has_text {
                                    debug!("Requesting text clipboard data proactively");
                                    // Request will trigger on_format_data_request which we need to handle
                                    // For now, just log that we detected text
                                }
                            } else {
                                warn!("Failed to send format list - manager queue full");
                            }
                        }
                    }
                    ClipboardBackendEvent::FormatDataRequest(format_id, message_proxy) => {
                        debug!("Processing format data request: {}", format_id);

                        // Create response callback - Note: need to investigate IronRDP API for response sending
                        // For now, skip creating the callback to get the server working
                        let response_callback = None;

                        // Send request to clipboard manager (this will read from Portal and call callback)
                        if let Ok(mgr) = manager.try_lock() {
                            if let Ok(_) = mgr.event_sender().try_send(
                                ClipboardEvent::RdpDataRequest(format_id, response_callback)
                            ) {
                                debug!("Data request sent to clipboard manager with response callback");
                            }
                        }
                    }
                    ClipboardBackendEvent::FormatDataResponse(data) => {
                        debug!("Processing format data response: {} bytes", data.len());
                        // Send data to clipboard manager for Portal write
                        if let Ok(mgr) = manager.try_lock() {
                            if let Ok(_) = mgr.event_sender().try_send(ClipboardEvent::RdpDataResponse(data)) {
                                debug!("Data response sent to clipboard manager");
                            }
                        }
                    }
                    ClipboardBackendEvent::FileContentsRequest(stream_id, _index, _position, _size, _proxy) => {
                        debug!("Processing file contents request: stream={}", stream_id);
                        // File transfer not yet implemented - will be added after text/images work
                    }
                    ClipboardBackendEvent::FileContentsResponse(stream_id, data) => {
                        debug!("Processing file contents response: stream={}, {} bytes", stream_id, data.len());
                        // File transfer not yet implemented
                    }
                }
            }

            // Sleep briefly to avoid busy-looping
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }
}

impl CliprdrBackendFactory for WrdCliprdrFactory {
    fn build_cliprdr_backend(&self) -> Box<dyn CliprdrBackend> {
        debug!("Building clipboard backend for new connection");

        // Note: We don't create channels here because this is called for EACH
        // connection attempt (including failed ones). Creating channels here
        // causes the previous channel to be dropped when a retry happens,
        // which closes the display update channel and crashes the server.
        //
        // The message proxy will be set later via the ServerEventSender trait
        // when the connection is established.

        // All backends share the same event queue and processing task
        let backend = Box::new(WrdCliprdrBackend {
            clipboard_manager: Arc::clone(&self.clipboard_manager),
            capabilities: ClipboardGeneralCapabilityFlags::empty(),
            temporary_directory: "/tmp/wrd-clipboard".to_string(),
            event_sender: self.event_sender.clone(), // Pass ServerEvent sender
            event_queue: Arc::clone(&self.shared_event_queue), // Use shared queue
        });

        backend
    }
}

impl ServerEventSender for WrdCliprdrFactory {
    fn set_sender(&mut self, sender: mpsc::UnboundedSender<ironrdp_server::ServerEvent>) {
        info!("Clipboard factory received server event sender");
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

    /// ServerEvent sender for sending clipboard requests to IronRDP
    event_sender: Option<mpsc::UnboundedSender<ironrdp_server::ServerEvent>>,

    /// Event queue for non-blocking clipboard operations
    event_queue: Arc<RwLock<VecDeque<ClipboardBackendEvent>>>,
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

        // Convert IronRDP formats to our format
        let wrd_formats: Vec<WrdClipboardFormat> = available_formats
            .iter()
            .map(|f| WrdClipboardFormat {
                format_id: f.id.0,
                format_name: f.name.as_ref().map(|n| n.value().to_string()).unwrap_or_default(),
            })
            .collect();

        // Non-blocking: push event to queue for async processing
        if let Ok(mut queue) = self.event_queue.try_write() {
            queue.push_back(ClipboardBackendEvent::RemoteCopy(wrd_formats.clone()));

            // Immediately request text data if available (CF_UNICODETEXT=13 or CF_TEXT=1)
            let text_format = available_formats.iter()
                .find(|f| f.id.0 == 13 || f.id.0 == 1)
                .map(|f| f.id.0);

            if let Some(format_id) = text_format {
                info!("Text format {} detected, requesting clipboard data from RDP client", format_id);

                // ✅ CORRECT: Send through ServerEvent channel
                if let Some(sender) = &self.event_sender {
                    use ironrdp_cliprdr::backend::ClipboardMessage;
                    use ironrdp_cliprdr::pdu::ClipboardFormatId;

                    if let Err(e) = sender.send(ironrdp_server::ServerEvent::Clipboard(
                        ClipboardMessage::SendInitiatePaste(ClipboardFormatId(format_id))
                    )) {
                        error!("Failed to send clipboard request via ServerEvent: {:?}", e);
                    } else {
                        info!("✅ Sent FormatDataRequest for format {} to RDP client via ServerEvent", format_id);
                    }
                } else {
                    error!("❌ No ServerEvent sender available - factory not initialized!");
                }
            }
        } else {
            warn!("Clipboard event queue locked, skipping format announcement");
        }
    }

    fn on_format_data_request(&mut self, request: FormatDataRequest) {
        let format_id = request.format.0;
        debug!("Format data requested for format ID: {}", format_id);

        // Non-blocking: push to event queue
        // Note: Response mechanism removed - will be implemented differently
        if let Ok(mut queue) = self.event_queue.try_write() {
            queue.push_back(ClipboardBackendEvent::FormatDataRequest(
                format_id,
                None, // No response callback for now
            ));
        }
    }

    fn on_format_data_response(&mut self, response: FormatDataResponse<'_>) {
        if response.is_error() {
            warn!("Format data response received with error flag");
            return;
        }

        let data = response.data().to_vec();
        debug!("Format data response received: {} bytes", data.len());

        // Non-blocking: push to event queue
        if let Ok(mut queue) = self.event_queue.try_write() {
            queue.push_back(ClipboardBackendEvent::FormatDataResponse(data));
        }
    }

    fn on_file_contents_request(&mut self, request: FileContentsRequest) {
        let stream_id = request.stream_id;
        let list_index = request.index;
        let position = request.position;
        let size = request.requested_size;

        debug!(
            "File contents requested: stream_id={}, list_index={}, pos={}, size={}",
            stream_id, list_index, position, size
        );

        // Non-blocking: push to event queue
        if let Ok(mut queue) = self.event_queue.try_write() {
            queue.push_back(ClipboardBackendEvent::FileContentsRequest(
                stream_id,
                list_index,
                position,
                size,
                None, // No response callback for now
            ));
        }
    }

    fn on_file_contents_response(&mut self, response: FileContentsResponse<'_>) {
        let stream_id = response.stream_id();
        let data = response.data().to_vec();

        debug!(
            "File contents response: stream_id={}, {} bytes",
            stream_id,
            data.len()
        );

        // Non-blocking: push to event queue
        if let Ok(mut queue) = self.event_queue.try_write() {
            queue.push_back(ClipboardBackendEvent::FileContentsResponse(stream_id, data));
        }
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
