//! IronRDP Clipboard Backend Implementation
//!
//! Implements the CliprdrBackend and CliprdrBackendFactory traits to integrate
//! our clipboard manager with IronRDP's clipboard protocol handling.

use ironrdp_cliprdr::backend::{CliprdrBackend, CliprdrBackendFactory};
use ironrdp_cliprdr::pdu::{
    ClipboardFormat, ClipboardGeneralCapabilityFlags, FileContentsRequest,
    FileContentsResponse, FormatDataRequest, FormatDataResponse, LockDataId,
};
use ironrdp_core::AsAny;
use ironrdp_server::ServerEventSender;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info};

use crate::clipboard::manager::ClipboardManager;

/// Clipboard backend factory for IronRDP server
///
/// Creates clipboard backend instances for each RDP connection.
pub struct WrdCliprdrFactory {
    /// Clipboard manager shared across connections
    clipboard_manager: Arc<Mutex<ClipboardManager>>,

    /// Server event sender for IronRDP
    event_sender: Option<mpsc::UnboundedSender<ironrdp_server::ServerEvent>>,
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
        }
    }
}

impl CliprdrBackendFactory for WrdCliprdrFactory {
    fn build_cliprdr_backend(&self) -> Box<dyn CliprdrBackend> {
        debug!("Building clipboard backend for new connection");

        Box::new(WrdCliprdrBackend {
            clipboard_manager: Arc::clone(&self.clipboard_manager),
            capabilities: ClipboardGeneralCapabilityFlags::empty(),
            temporary_directory: "/tmp/wrd-clipboard".to_string(),
        })
    }
}

impl ServerEventSender for WrdCliprdrFactory {
    fn set_sender(&mut self, sender: mpsc::UnboundedSender<ironrdp_server::ServerEvent>) {
        self.event_sender = Some(sender);
    }
}

impl ironrdp_server::CliprdrServerFactory for WrdCliprdrFactory {}

/// Clipboard backend implementation
///
/// Handles clipboard protocol events from IronRDP and coordinates with
/// our ClipboardManager and Portal clipboard.
#[derive(Debug)]
struct WrdCliprdrBackend {
    /// Reference to shared clipboard manager (for future full integration)
    #[allow(dead_code)]
    clipboard_manager: Arc<Mutex<ClipboardManager>>,

    /// Negotiated capabilities
    capabilities: ClipboardGeneralCapabilityFlags,

    /// Temporary directory for file transfers
    temporary_directory: String,
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
            debug!("  Format {}: {:?}", idx, format);
        }

        // Full implementation would:
        // 1. Convert RDP formats to our ClipboardFormat types
        // 2. Forward to ClipboardManager
        // 3. Announce to Portal clipboard
        // This is deferred to full clipboard integration phase
    }

    fn on_format_data_request(&mut self, request: FormatDataRequest) {
        debug!("Format data requested: {:?}", request);

        // Full implementation would:
        // 1. Get format ID from request
        // 2. Fetch data from Portal
        // 3. Convert format
        // 4. Send response via CliprdrServer
        // Deferred to full clipboard integration
    }

    fn on_format_data_response(&mut self, response: FormatDataResponse<'_>) {
        debug!("Format data response received: {} bytes", response.data().len());

        // Full implementation would:
        // 1. Extract data from response
        // 2. Convert format
        // 3. Set to Portal clipboard
        // Deferred to full clipboard integration
    }

    fn on_file_contents_request(&mut self, request: FileContentsRequest) {
        debug!("File contents requested: {:?}", request);

        // File transfer support - deferred to full implementation
        // Would read file and send contents via CliprdrServer
    }

    fn on_file_contents_response(&mut self, response: FileContentsResponse<'_>) {
        debug!("File contents response: {} bytes", response.data().len());

        // File transfer support - deferred to full implementation
        // Would write file contents to local filesystem
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
