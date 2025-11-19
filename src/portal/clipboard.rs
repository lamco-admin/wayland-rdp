//! Portal Clipboard Integration
//!
//! Implements delayed rendering clipboard using Portal Clipboard D-Bus API.
//! This replaces wl-clipboard-rs with proper Portal integration that supports
//! format announcement without data transfer (delayed rendering model).
//!
//! Architecture:
//! - SetSelection() announces available formats to Wayland
//! - SelectionTransfer signal notifies when data is requested
//! - SelectionWrite() provides data via file descriptor
//! - SelectionOwnerChanged signal monitors local clipboard changes
//! - SelectionRead() reads local clipboard data

use anyhow::{Context, Result};
use ashpd::desktop::clipboard::Clipboard;
use ashpd::desktop::remote_desktop::RemoteDesktop;
use ashpd::desktop::Session;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Pending clipboard data request from Portal
#[derive(Debug, Clone)]
pub struct PendingRequest {
    pub serial: u32,
    pub mime_type: String,
    pub requested_at: SystemTime,
}

/// Callback type for requesting clipboard data from RDP client
pub type RdpDataRequester = Arc<dyn Fn(u32) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>>> + Send>> + Send + Sync>;

/// Callback type for notifying local clipboard changes
pub type LocalClipboardChangeHandler = Arc<dyn Fn(Vec<String>) + Send + Sync>;

/// Portal Clipboard Manager
///
/// Integrates RDP clipboard with Wayland via Portal Clipboard API.
/// Supports delayed rendering where formats are announced without data,
/// and data is only transferred when actually requested.
pub struct ClipboardManager {
    /// Portal Clipboard interface
    clipboard: Clipboard<'static>,

    /// Pending Portal requests (serial → request info)
    pending_requests: Arc<RwLock<HashMap<u32, PendingRequest>>>,
}

impl ClipboardManager {
    /// Create new Portal Clipboard manager
    ///
    /// # Arguments
    ///
    /// * `session` - Reference to RemoteDesktop session to associate clipboard with
    ///
    /// # Returns
    ///
    /// Clipboard manager instance with listeners started
    pub async fn new() -> Result<Self> {
        info!("Initializing Portal Clipboard manager");

        let clipboard = Clipboard::new().await
            .context("Failed to create Portal Clipboard")?;

        info!("Portal Clipboard created (will be enabled when session is ready)");

        let manager = Self {
            clipboard,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        };

        Ok(manager)
    }

    /// Request clipboard access for session
    pub async fn enable_for_session(&self, session: &Session<'_, RemoteDesktop<'_>>) -> Result<()> {
        self.clipboard.request(session).await
            .context("Failed to request clipboard access for session")?;
        info!("✅ Portal Clipboard enabled for session");
        Ok(())
    }

    /// Announce RDP clipboard formats to Wayland (delayed rendering)
    ///
    /// When RDP client copies, we announce what formats are available WITHOUT
    /// transferring the actual data. Data is only fetched when user pastes.
    ///
    /// # Arguments
    ///
    /// * `session` - RemoteDesktop session
    /// * `mime_types` - List of MIME types available (e.g., ["text/plain", "image/png"])
    pub async fn announce_rdp_formats(
        &self,
        session: &Session<'_, RemoteDesktop<'_>>,
        mime_types: Vec<String>,
    ) -> Result<()> {
        if mime_types.is_empty() {
            debug!("No formats to announce");
            return Ok(());
        }

        let mime_refs: Vec<&str> = mime_types.iter().map(|s| s.as_str()).collect();

        self.clipboard.set_selection(session, &mime_refs).await
            .context("Failed to set Portal selection")?;

        info!("📋 Announced {} RDP formats to Portal: {:?}", mime_types.len(), mime_types);
        Ok(())
    }

    /// Get reference to Portal Clipboard for direct API access
    pub fn portal_clipboard(&self) -> &Clipboard<'static> {
        &self.clipboard
    }

    /// Provide clipboard data to Portal via file descriptor (static version for spawned tasks)
    async fn write_to_portal_fd_static(
        clipboard: &Clipboard<'_>,
        session: &Session<'_, RemoteDesktop<'_>>,
        serial: u32,
        data: &[u8],
    ) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        // Get write file descriptor from Portal
        let fd = clipboard.selection_write(session, serial).await
            .context("Failed to get SelectionWrite fd")?;

        // Convert zvariant::OwnedFd to File
        // zvariant OwnedFd is an enum Fd::Owned(std::os::fd::OwnedFd)
        // Extract the inner OwnedFd and convert to File
        let std_fd: std::os::fd::OwnedFd = fd.into();
        let std_file = std::fs::File::from(std_fd);
        let mut file = tokio::fs::File::from_std(std_file);

        // Write data to fd
        file.write_all(data).await
            .context("Failed to write clipboard data to fd")?;
        file.flush().await?;
        drop(file); // Close fd

        // Notify Portal of success
        clipboard.selection_write_done(session, serial, true).await
            .context("Failed to notify Portal of write completion")?;

        info!("✅ Provided {} bytes to Portal (serial {})", data.len(), serial);
        Ok(())
    }

    /// Provide clipboard data to Portal
    pub async fn provide_data(
        &self,
        session: &Session<'_, RemoteDesktop<'_>>,
        serial: u32,
        data: Vec<u8>,
    ) -> Result<()> {
        Self::write_to_portal_fd_static(&self.clipboard, session, serial, &data).await
    }

    // Signal streams removed - caller uses portal_clipboard() directly for signal access

    /// Read from local Wayland clipboard
    ///
    /// Used when RDP client requests our clipboard data (Linux → Windows copy).
    ///
    /// # Arguments
    ///
    /// * `session` - RemoteDesktop session
    /// * `mime_type` - MIME type to read (e.g., "text/plain")
    ///
    /// # Returns
    ///
    /// Clipboard data in requested format
    pub async fn read_local_clipboard(
        &self,
        session: &Session<'_, RemoteDesktop<'_>>,
        mime_type: &str,
    ) -> Result<Vec<u8>> {
        use tokio::io::AsyncReadExt;

        debug!("Reading local clipboard: {}", mime_type);

        let fd = self.clipboard.selection_read(session, mime_type).await
            .context("Failed to get SelectionRead fd")?;

        // Convert zvariant::OwnedFd to File
        let std_fd: std::os::fd::OwnedFd = fd.into();
        let std_file = std::fs::File::from(std_fd);
        let mut file = tokio::fs::File::from_std(std_file);
        let mut data = Vec::new();
        file.read_to_end(&mut data).await
            .context("Failed to read clipboard data from fd")?;

        info!("📖 Read {} bytes from local clipboard ({})", data.len(), mime_type);
        Ok(data)
    }

    /// Get pending request by serial
    pub async fn get_pending_request(&self, serial: u32) -> Option<PendingRequest> {
        self.pending_requests.read().await.get(&serial).cloned()
    }
}

impl std::fmt::Debug for ClipboardManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipboardManager")
            .field("session", &"<session>")
            .field("pending_requests_count", &self.pending_requests.try_read().map(|r| r.len()).unwrap_or(0))
            .finish()
    }
}
