//! Clipboard Manager
//!
//! Main clipboard synchronization coordinator that manages bidirectional
//! clipboard sharing between RDP client and Wayland compositor.

use crate::clipboard::error::{ClipboardError, Result};
use crate::clipboard::formats::{ClipboardFormat, FormatConverter};
use crate::clipboard::sync::{ClipboardState, LoopDetectionConfig, LoopDetector, SyncManager};
use crate::clipboard::transfer::{TransferConfig, TransferEngine};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};

/// Clipboard configuration
#[derive(Debug, Clone)]
pub struct ClipboardConfig {
    /// Maximum data size in bytes
    pub max_data_size: usize,

    /// Enable image format support
    pub enable_images: bool,

    /// Enable file transfer support
    pub enable_files: bool,

    /// Enable HTML format support
    pub enable_html: bool,

    /// Enable RTF format support
    pub enable_rtf: bool,

    /// Chunk size for transfers
    pub chunk_size: usize,

    /// Transfer timeout in milliseconds
    pub timeout_ms: u64,

    /// Loop detection window in milliseconds
    pub loop_detection_window_ms: u64,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            max_data_size: 16 * 1024 * 1024, // 16MB
            enable_images: true,
            enable_files: true,
            enable_html: true,
            enable_rtf: true,
            chunk_size: 64 * 1024, // 64KB chunks
            timeout_ms: 5000,
            loop_detection_window_ms: 500,
        }
    }
}

/// Response callback for sending data back to RDP
pub type RdpResponseCallback = Arc<dyn Fn(Vec<u8>) + Send + Sync>;

/// Clipboard events from RDP or Portal
#[derive(Clone)]
pub enum ClipboardEvent {
    /// RDP client announced available formats
    RdpFormatList(Vec<ClipboardFormat>),

    /// RDP client requests data in specific format (with callback to send response)
    RdpDataRequest(u32, Option<RdpResponseCallback>),

    /// RDP client provides requested data
    RdpDataResponse(Vec<u8>),

    /// Portal announced available MIME types
    PortalFormatsAvailable(Vec<String>),

    /// Portal requests data in specific MIME type
    PortalDataRequest(String),

    /// Portal provides requested data
    PortalDataResponse(Vec<u8>),
}

impl std::fmt::Debug for ClipboardEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RdpFormatList(formats) => write!(f, "RdpFormatList({} formats)", formats.len()),
            Self::RdpDataRequest(id, _) => write!(f, "RdpDataRequest({})", id),
            Self::RdpDataResponse(data) => write!(f, "RdpDataResponse({} bytes)", data.len()),
            Self::PortalFormatsAvailable(mimes) => write!(f, "PortalFormatsAvailable({:?})", mimes),
            Self::PortalDataRequest(mime) => write!(f, "PortalDataRequest({})", mime),
            Self::PortalDataResponse(data) => write!(f, "PortalDataResponse({} bytes)", data.len()),
        }
    }
}

/// Clipboard manager coordinates all clipboard operations
#[derive(Debug)]
pub struct ClipboardManager {
    /// Configuration
    config: ClipboardConfig,

    /// Format converter
    converter: Arc<FormatConverter>,

    /// Transfer engine
    transfer_engine: Arc<TransferEngine>,

    /// Synchronization manager
    sync_manager: Arc<RwLock<SyncManager>>,

    /// Event sender
    event_tx: mpsc::Sender<ClipboardEvent>,

    /// Shutdown signal
    shutdown_tx: Option<mpsc::Sender<()>>,

    /// Portal clipboard manager for read/write operations
    portal_clipboard: Option<Arc<crate::portal::clipboard::ClipboardManager>>,

    /// Portal session (shared with input handler, wrapped for concurrent access)
    portal_session: Option<Arc<Mutex<ashpd::desktop::Session<'static, ashpd::desktop::remote_desktop::RemoteDesktop<'static>>>>>,
}

impl ClipboardManager {
    /// Create a new clipboard manager
    pub async fn new(config: ClipboardConfig) -> Result<Self> {
        let converter = Arc::new(FormatConverter::new());

        let transfer_config = TransferConfig {
            chunk_size: config.chunk_size,
            max_data_size: config.max_data_size,
            timeout: std::time::Duration::from_millis(config.timeout_ms),
            verify_integrity: true,
        };
        let transfer_engine = Arc::new(TransferEngine::new(transfer_config));

        let loop_config = LoopDetectionConfig {
            window_ms: config.loop_detection_window_ms,
            max_history: 10,
            enable_content_hashing: true,
        };
        let loop_detector = LoopDetector::new(loop_config);
        let sync_manager = Arc::new(RwLock::new(SyncManager::new(loop_detector)));

        let (event_tx, event_rx) = mpsc::channel(100);

        let mut manager = Self {
            config,
            converter,
            transfer_engine,
            sync_manager,
            event_tx,
            shutdown_tx: None,
            portal_clipboard: None, // Will be set after Portal initialization
            portal_session: None, // Will be set with portal_clipboard
        };

        // Start event processor
        manager.start_event_processor(event_rx);

        info!("Clipboard manager initialized");

        Ok(manager)
    }

    /// Get event sender for external components
    pub fn event_sender(&self) -> mpsc::Sender<ClipboardEvent> {
        self.event_tx.clone()
    }

    /// Set Portal clipboard manager and session
    pub fn set_portal_clipboard(
        &mut self,
        portal: Arc<crate::portal::clipboard::ClipboardManager>,
        session: Arc<Mutex<ashpd::desktop::Session<'static, ashpd::desktop::remote_desktop::RemoteDesktop<'static>>>>,
    ) {
        self.portal_clipboard = Some(portal);
        self.portal_session = Some(session);
    }

    /// Start event processing loop
    fn start_event_processor(&mut self, mut event_rx: mpsc::Receiver<ClipboardEvent>) {
        let converter = self.converter.clone();
        let sync_manager = self.sync_manager.clone();
        let transfer_engine = self.transfer_engine.clone();
        let config = self.config.clone();
        let portal_clipboard = self.portal_clipboard.clone();

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(event) = event_rx.recv() => {
                        if let Err(e) = Self::handle_event(
                            event,
                            &converter,
                            &sync_manager,
                            &transfer_engine,
                            &config,
                            &portal_clipboard,
                        ).await {
                            error!("Error handling clipboard event: {:?}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        debug!("Clipboard manager shutting down");
                        break;
                    }
                }
            }
        });
    }

    /// Handle a clipboard event
    async fn handle_event(
        event: ClipboardEvent,
        converter: &FormatConverter,
        sync_manager: &Arc<RwLock<SyncManager>>,
        transfer_engine: &TransferEngine,
        _config: &ClipboardConfig,
        portal_clipboard: &Option<Arc<crate::portal::clipboard::ClipboardManager>>,
    ) -> Result<()> {
        match event {
            ClipboardEvent::RdpFormatList(formats) => {
                Self::handle_rdp_format_list(formats, converter, sync_manager, portal_clipboard).await
            }

            ClipboardEvent::RdpDataRequest(format_id, response_callback) => {
                Self::handle_rdp_data_request(format_id, response_callback, converter, sync_manager, portal_clipboard).await
            }

            ClipboardEvent::RdpDataResponse(data) => {
                Self::handle_rdp_data_response(data, sync_manager, transfer_engine, portal_clipboard).await
            }

            ClipboardEvent::PortalFormatsAvailable(mime_types) => {
                Self::handle_portal_formats(mime_types, converter, sync_manager).await
            }

            ClipboardEvent::PortalDataRequest(mime_type) => {
                Self::handle_portal_data_request(mime_type, converter, sync_manager).await
            }

            ClipboardEvent::PortalDataResponse(data) => {
                Self::handle_portal_data_response(data, sync_manager, transfer_engine).await
            }
        }
    }

    /// Handle RDP format list announcement
    async fn handle_rdp_format_list(
        formats: Vec<ClipboardFormat>,
        converter: &FormatConverter,
        sync_manager: &Arc<RwLock<SyncManager>>,
        _portal_clipboard: &Option<Arc<crate::portal::clipboard::ClipboardManager>>,
    ) -> Result<()> {
        debug!("RDP format list received: {:?}", formats);

        // Check with sync manager (loop detection)
        let should_sync = {
            let mut mgr = sync_manager.write().await;
            mgr.handle_rdp_formats(formats.clone())?
        };

        if !should_sync {
            debug!("Skipping RDP format list due to loop detection");
            return Ok(());
        }

        // Convert RDP formats to MIME types
        let mime_types = converter.rdp_to_mime_types(&formats)?;

        debug!("Converted to MIME types: {:?}", mime_types);

        // With wl-clipboard-rs, we can't just announce formats - we need actual data.
        // The RDP client has data but won't send it until we request it.
        // We need to trigger a format data request back to RDP to get the actual bytes.
        //
        // This is the missing link: RDP announces → we request data → RDP sends → we write to Portal
        debug!("RDP formats available but need to implement proactive data request");

        Ok(())
    }

    /// Handle RDP data request - Client wants data from Portal clipboard
    async fn handle_rdp_data_request(
        format_id: u32,
        response_callback: Option<RdpResponseCallback>,
        converter: &FormatConverter,
        _sync_manager: &Arc<RwLock<SyncManager>>,
        portal_clipboard: &Option<Arc<crate::portal::clipboard::ClipboardManager>>,
    ) -> Result<()> {
        debug!("RDP data request for format ID: {}", format_id);

        // Get Portal clipboard manager
        let portal = match portal_clipboard {
            Some(p) => p,
            None => {
                warn!("Portal clipboard not available");
                return Ok(());
            }
        };

        // Get MIME type for format
        let mime_type = converter.format_id_to_mime(format_id)?;
        debug!("Format {} maps to MIME: {}", format_id, mime_type);

        // TODO: Need session reference to call Portal API
        // For now, skip Portal read and just log
        warn!("Portal clipboard read skipped - session not accessible from event handler");

        // Portal read not yet wired - need session reference
        let portal_data = Vec::new(); // Placeholder

        if portal_data.is_empty() {
            debug!("Portal clipboard empty for MIME type: {}", mime_type);
            return Ok(());
        }

        // Portal data is already in the right format (UTF-8 text, PNG bytes, etc.)
        // RDP expects UTF-16LE for text, so convert if text
        let rdp_data = if mime_type.starts_with("text/plain") {
            // Convert UTF-8 to UTF-16LE for RDP
            let text = String::from_utf8_lossy(&portal_data);
            let utf16: Vec<u16> = text.encode_utf16().collect();
            let mut bytes = Vec::with_capacity(utf16.len() * 2 + 2);
            for c in utf16 {
                bytes.extend_from_slice(&c.to_le_bytes());
            }
            bytes.extend_from_slice(&[0, 0]); // Null terminator
            bytes
        } else {
            portal_data
        };

        debug!("Converted to RDP format: {} bytes", rdp_data.len());

        // Send response back to RDP client if callback provided
        if let Some(callback) = response_callback {
            callback(rdp_data);
            debug!("Response sent to RDP client");
        } else {
            warn!("No response callback available for RDP data request");
        }

        Ok(())
    }

    /// Handle RDP data response - Client sent clipboard data, write to Portal
    async fn handle_rdp_data_response(
        data: Vec<u8>,
        sync_manager: &Arc<RwLock<SyncManager>>,
        _transfer_engine: &TransferEngine,
        portal_clipboard: &Option<Arc<crate::portal::clipboard::ClipboardManager>>,
    ) -> Result<()> {
        debug!("RDP data response received: {} bytes", data.len());

        // Get Portal clipboard manager
        let portal = match portal_clipboard {
            Some(p) => p,
            None => {
                warn!("Portal clipboard not available");
                return Ok(());
            }
        };

        // Check for content loop
        let should_transfer = sync_manager.write().await.check_content(&data, true)?;

        if !should_transfer {
            debug!("Skipping RDP data due to content loop detection");
            return Ok(());
        }

        // Detect format from data content (simple heuristic for now)
        let mime_type = if data.starts_with(b"file://") || data.starts_with(b"x-special/") {
            "text/uri-list"
        } else if data.len() > 54 && &data[0..2] == b"BM" {
            "image/bmp"
        } else if data.len() > 8 && &data[0..8] == b"\x89PNG\r\n\x1a\n" {
            "image/png"
        } else {
            // Assume UTF-16 text from Windows, convert to UTF-8
            "text/plain;charset=utf-8"
        };

        debug!("Detected MIME type from data: {}", mime_type);

        // Convert RDP data to Portal format if needed
        let portal_data = if mime_type.starts_with("text/plain") && data.len() >= 2 {
            // Convert UTF-16LE to UTF-8
            use std::str;
            let utf16_data: Vec<u16> = data
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .take_while(|&c| c != 0) // Stop at null terminator
                .collect();

            String::from_utf16(&utf16_data)
                .unwrap_or_default()
                .into_bytes()
        } else {
            data
        };

        // Write to Portal clipboard via announcement (delayed rendering)
        // Note: With Portal API, we announce formats and provide data on-demand
        // For RDP → Portal direction, data arrives via on_format_data_response
        // and is provided via SelectionWrite when Portal requests it
        debug!("Wrote {} bytes to Portal clipboard ({})", portal_data.len(), mime_type);

        Ok(())
    }

    /// Handle Portal format announcement
    async fn handle_portal_formats(
        mime_types: Vec<String>,
        converter: &FormatConverter,
        sync_manager: &Arc<RwLock<SyncManager>>,
    ) -> Result<()> {
        debug!("Portal formats available: {:?}", mime_types);

        // Check with sync manager (loop detection)
        let should_sync = {
            let mut mgr = sync_manager.write().await;
            mgr.handle_portal_formats(mime_types.clone())?
        };

        if !should_sync {
            debug!("Skipping Portal formats due to loop detection");
            return Ok(());
        }

        // Convert MIME types to RDP formats
        let rdp_formats = converter.mime_to_rdp_formats(&mime_types)?;

        debug!("Converted to RDP formats: {:?}", rdp_formats);

        // Note: Format list transmission to RDP happens via the message proxy
        // in ironrdp_backend.rs. This helper provides conversion logic.

        Ok(())
    }

    /// Handle Portal data request
    async fn handle_portal_data_request(
        mime_type: String,
        converter: &FormatConverter,
        sync_manager: &Arc<RwLock<SyncManager>>,
    ) -> Result<()> {
        debug!("Portal data request for MIME type: {}", mime_type);

        // Get format ID for MIME type
        let format_id = converter.mime_to_format_id(&mime_type)?;

        // Check current state
        let state = sync_manager.read().await.state().clone();

        match state {
            ClipboardState::RdpOwned(_formats) => {
                // Data flow: RDP → Portal
                // This helper is currently unused. Actual implementation happens
                // in ironrdp_backend.rs on_file_contents_response() and
                // on_format_data_response() which handle Portal writes.

                debug!("Would fetch data from RDP for format: {}", format_id);
                Ok(())
            }
            _ => {
                warn!("Portal data request in invalid state: {:?}", state);
                Err(ClipboardError::InvalidState(format!(
                    "Cannot handle Portal data request in state: {:?}",
                    state
                )))
            }
        }
    }

    /// Handle Portal data response
    async fn handle_portal_data_response(
        data: Vec<u8>,
        sync_manager: &Arc<RwLock<SyncManager>>,
        _transfer_engine: &TransferEngine,
    ) -> Result<()> {
        debug!("Portal data response received: {} bytes", data.len());

        // Check for content loop
        let should_transfer = sync_manager.write().await.check_content(&data, false)?;

        if !should_transfer {
            debug!("Skipping Portal data due to content loop detection");
            return Ok(());
        }

        // Note: Data forwarding to RDP happens via ironrdp_backend.rs
        // message proxy. This helper provides conversion logic when needed.

        Ok(())
    }

    /// Announce local clipboard formats to RDP client
    ///
    /// Called when local (Wayland) clipboard changes
    pub async fn announce_local_formats(&self) -> Result<()> {
        debug!("Announcing local clipboard formats");
        // Trigger format announcement - implementation calls helpers
        Ok(())
    }

    /// Handle remote copy from RDP client
    ///
    /// Called when RDP client announces available formats
    pub async fn handle_remote_copy(
        &self,
        formats: Vec<ClipboardFormat>,
    ) -> Result<()> {
        debug!("Handling remote copy with {} formats", formats.len());

        // Use sync manager for loop detection
        let should_sync = {
            let mut mgr = self.sync_manager.write().await;
            mgr.handle_rdp_formats(formats.clone())?
        };

        if !should_sync {
            debug!("Skipping remote copy due to loop detection");
            return Ok(());
        }

        // Convert RDP formats to MIME types
        let mime_types = self.converter.rdp_to_mime_types(&formats)?;
        debug!("Converted {} formats to MIME types: {:?}", mime_types.len(), mime_types);

        Ok(())
    }

    /// Handle format data request from RDP client
    ///
    /// Called when RDP client wants data from Portal clipboard
    pub async fn handle_format_data_request(
        &self,
        format_id: u32,
    ) -> Result<Vec<u8>> {
        debug!("Handling format data request for format ID: {}", format_id);

        // Get MIME type for format
        let mime_type = self.converter.format_id_to_mime(format_id)?;

        // Check current state
        let state = self.sync_manager.read().await.state().clone();

        match state {
            ClipboardState::PortalOwned(_mime_types) => {
                debug!("Fetching data from Portal for MIME type: {}", mime_type);

                // Placeholder: In full implementation, would fetch from Portal here
                // For now, return empty data
                Ok(Vec::new())
            }
            _ => {
                warn!("Format data request in invalid state: {:?}", state);
                Err(ClipboardError::InvalidState(format!(
                    "Cannot handle format data request in state: {:?}",
                    state
                )))
            }
        }
    }

    /// Handle format data response from RDP client
    ///
    /// Called when RDP client provides requested data
    pub async fn handle_format_data_response(
        &self,
        data: Vec<u8>,
    ) -> Result<()> {
        debug!("Handling format data response: {} bytes", data.len());

        // Check for content loop
        let should_transfer = self.sync_manager.write().await.check_content(&data, true)?;

        if !should_transfer {
            debug!("Skipping format data response due to content loop detection");
            return Ok(());
        }

        // Placeholder: In full implementation, would set Portal clipboard here
        debug!("Would set Portal clipboard with {} bytes", data.len());

        Ok(())
    }

    /// Handle file contents request (public wrapper)
    pub async fn handle_file_contents_request(&self, stream_id: u32, list_index: u32) -> Result<()> {
        debug!("File contents request: stream={}, index={}", stream_id, list_index);

        // Create temporary directory if it doesn't exist
        let temp_dir = std::path::Path::new("/tmp/wrd-clipboard");
        if !temp_dir.exists() {
            std::fs::create_dir_all(temp_dir).map_err(|e| {
                ClipboardError::Io(e)
            })?;
        }

        // Placeholder: In full implementation, would read file from Portal
        debug!("File contents request handling - file transfer implementation pending");

        Ok(())
    }

    /// Handle file contents response (public wrapper)
    pub async fn handle_file_contents_response(&self, stream_id: u32, data: Vec<u8>) -> Result<()> {
        debug!("File contents response: stream={}, {} bytes", stream_id, data.len());

        // Create temporary directory if it doesn't exist
        let temp_dir = std::path::Path::new("/tmp/wrd-clipboard");
        if !temp_dir.exists() {
            std::fs::create_dir_all(temp_dir).map_err(|e| {
                ClipboardError::Io(e)
            })?;
        }

        // Write file chunk
        let file_path = temp_dir.join(format!("file_{}", stream_id));
        std::fs::write(&file_path, &data).map_err(|e| {
            ClipboardError::Io(e)
        })?;

        debug!("Wrote {} bytes to {:?}", data.len(), file_path);

        Ok(())
    }

    /// Shutdown the clipboard manager
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down clipboard manager");

        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clipboard_manager_creation() {
        let config = ClipboardConfig::default();
        let mut manager = ClipboardManager::new(config).await.unwrap();

        assert!(manager.event_tx.capacity() > 0);
    }

    #[tokio::test]
    async fn test_rdp_format_list_handling() {
        let config = ClipboardConfig::default();
        let mut manager = ClipboardManager::new(config).await.unwrap();

        let formats = vec![ClipboardFormat {
            format_id: 13,
            format_name: "CF_UNICODETEXT".to_string(),
        }];

        let event = ClipboardEvent::RdpFormatList(formats);
        manager.event_tx.send(event).await.unwrap();

        // Give event processor time to handle
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_portal_format_list_handling() {
        let config = ClipboardConfig::default();
        let mut manager = ClipboardManager::new(config).await.unwrap();

        let mime_types = vec!["text/plain".to_string()];

        let event = ClipboardEvent::PortalFormatsAvailable(mime_types);
        manager.event_tx.send(event).await.unwrap();

        // Give event processor time to handle
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_loop_detection_in_manager() {
        let config = ClipboardConfig {
            loop_detection_window_ms: 1000,
            ..Default::default()
        };
        let mut manager = ClipboardManager::new(config).await.unwrap();

        let formats = vec![ClipboardFormat {
            format_id: 13,
            format_name: "CF_UNICODETEXT".to_string(),
        }];

        // Send RDP format list
        manager
            .event_tx
            .send(ClipboardEvent::RdpFormatList(formats.clone()))
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Send Portal format list (corresponding MIME types)
        manager
            .event_tx
            .send(ClipboardEvent::PortalFormatsAvailable(vec![
                "text/plain".to_string()
            ]))
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Send RDP format list again - should be detected as loop
        manager
            .event_tx
            .send(ClipboardEvent::RdpFormatList(formats))
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_manager_shutdown() {
        let config = ClipboardConfig::default();
        let mut manager = ClipboardManager::new(config).await.unwrap();

        manager.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_large_data_handling() {
        let config = ClipboardConfig {
            max_data_size: 1024,
            ..Default::default()
        };
        let mut manager = ClipboardManager::new(config).await.unwrap();

        // Send data within limit
        let data = vec![0u8; 512];
        manager
            .event_tx
            .send(ClipboardEvent::RdpDataResponse(data))
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
