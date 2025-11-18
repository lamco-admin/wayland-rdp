//! Clipboard Manager
//!
//! Main clipboard synchronization coordinator that manages bidirectional
//! clipboard sharing between RDP client and Wayland compositor.

use crate::clipboard::error::{ClipboardError, Result};
use crate::clipboard::formats::{ClipboardFormat, FormatConverter};
use crate::clipboard::sync::{ClipboardState, LoopDetectionConfig, LoopDetector, SyncManager};
use crate::clipboard::transfer::{TransferConfig, TransferEngine};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
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

/// Clipboard events from RDP or Portal
#[derive(Debug, Clone)]
pub enum ClipboardEvent {
    /// RDP client announced available formats
    RdpFormatList(Vec<ClipboardFormat>),

    /// RDP client requests data in specific format
    RdpDataRequest(u32),

    /// RDP client provides requested data
    RdpDataResponse(Vec<u8>),

    /// Portal announced available MIME types
    PortalFormatsAvailable(Vec<String>),

    /// Portal requests data in specific MIME type
    PortalDataRequest(String),

    /// Portal provides requested data
    PortalDataResponse(Vec<u8>),
}

/// Clipboard manager coordinates all clipboard operations
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

        let manager = Self {
            config,
            converter,
            transfer_engine,
            sync_manager,
            event_tx,
            shutdown_tx: None,
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

    /// Start event processing loop
    fn start_event_processor(&mut self, mut event_rx: mpsc::Receiver<ClipboardEvent>) {
        let converter = self.converter.clone();
        let sync_manager = self.sync_manager.clone();
        let transfer_engine = self.transfer_engine.clone();
        let config = self.config.clone();

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
        config: &ClipboardConfig,
    ) -> Result<()> {
        match event {
            ClipboardEvent::RdpFormatList(formats) => {
                Self::handle_rdp_format_list(formats, converter, sync_manager).await
            }

            ClipboardEvent::RdpDataRequest(format_id) => {
                Self::handle_rdp_data_request(format_id, converter, sync_manager).await
            }

            ClipboardEvent::RdpDataResponse(data) => {
                Self::handle_rdp_data_response(data, sync_manager, transfer_engine).await
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

        // TODO: When Portal clipboard integration is ready, announce formats to Portal
        // portal_clipboard.advertise_formats(mime_types).await?;

        Ok(())
    }

    /// Handle RDP data request
    async fn handle_rdp_data_request(
        format_id: u32,
        converter: &FormatConverter,
        sync_manager: &Arc<RwLock<SyncManager>>,
    ) -> Result<()> {
        debug!("RDP data request for format ID: {}", format_id);

        // Get MIME type for format
        let mime_type = converter.format_id_to_mime(format_id)?;

        // Check current state
        let state = sync_manager.read().await.state().clone();

        match state {
            ClipboardState::PortalOwned(_mime_types) => {
                // TODO: When Portal integration is ready
                // 1. Request data from Portal for mime_type
                // 2. Convert to RDP format
                // 3. Send response via RDP

                debug!("Would fetch data from Portal for: {}", mime_type);
                Ok(())
            }
            _ => {
                warn!("RDP data request in invalid state: {:?}", state);
                Err(ClipboardError::InvalidState(format!(
                    "Cannot handle RDP data request in state: {:?}",
                    state
                )))
            }
        }
    }

    /// Handle RDP data response
    async fn handle_rdp_data_response(
        data: Vec<u8>,
        sync_manager: &Arc<RwLock<SyncManager>>,
        _transfer_engine: &TransferEngine,
    ) -> Result<()> {
        debug!("RDP data response received: {} bytes", data.len());

        // Check for content loop
        let should_transfer = sync_manager.write().await.check_content(&data, true)?;

        if !should_transfer {
            debug!("Skipping RDP data due to content loop detection");
            return Ok(());
        }

        // TODO: When Portal integration is ready, forward data to Portal

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

        // TODO: When RDP integration is ready, send format list to RDP client

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
                // TODO: When RDP integration is ready
                // 1. Request data from RDP for format_id
                // 2. Convert from RDP format
                // 3. Send response to Portal

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

        // TODO: When RDP integration is ready, forward data to RDP client

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
        let manager = ClipboardManager::new(config).await.unwrap();

        assert!(manager.event_tx.capacity() > 0);
    }

    #[tokio::test]
    async fn test_rdp_format_list_handling() {
        let config = ClipboardConfig::default();
        let manager = ClipboardManager::new(config).await.unwrap();

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
        let manager = ClipboardManager::new(config).await.unwrap();

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
        let manager = ClipboardManager::new(config).await.unwrap();

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
        let manager = ClipboardManager::new(config).await.unwrap();

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
