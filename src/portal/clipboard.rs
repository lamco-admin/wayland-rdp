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
#[derive(Clone)]
pub struct ClipboardManager {
    /// Portal Clipboard interface
    clipboard: Clipboard<'static>,

    /// RemoteDesktop session (clipboard is scoped to this session)
    session: Session<'static, RemoteDesktop<'static>>,

    /// Pending Portal requests (serial → request info)
    pending_requests: Arc<RwLock<HashMap<u32, PendingRequest>>>,
}

impl ClipboardManager {
    /// Create new Portal Clipboard manager
    ///
    /// # Arguments
    ///
    /// * `session` - RemoteDesktop session to associate clipboard with
    ///
    /// # Returns
    ///
    /// Clipboard manager instance with listeners started
    pub async fn new(
        session: Session<'static, RemoteDesktop<'static>>,
    ) -> Result<Self> {
        info!("Initializing Portal Clipboard manager");

        let clipboard = Clipboard::new().await
            .context("Failed to create Portal Clipboard")?;

        // Request clipboard access for RemoteDesktop session
        clipboard.request(&session).await
            .context("Failed to request clipboard access for session")?;

        info!("✅ Portal Clipboard enabled for RemoteDesktop session");

        let manager = Self {
            clipboard,
            session,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        };

        Ok(manager)
    }

    /// Announce RDP clipboard formats to Wayland (delayed rendering)
    ///
    /// When RDP client copies, we announce what formats are available WITHOUT
    /// transferring the actual data. Data is only fetched when user pastes.
    ///
    /// # Arguments
    ///
    /// * `mime_types` - List of MIME types available (e.g., ["text/plain", "image/png"])
    pub async fn announce_rdp_formats(&self, mime_types: Vec<String>) -> Result<()> {
        if mime_types.is_empty() {
            debug!("No formats to announce");
            return Ok(());
        }

        let mime_refs: Vec<&str> = mime_types.iter().map(|s| s.as_str()).collect();

        self.clipboard.set_selection(&self.session, &mime_refs).await
            .context("Failed to set Portal selection")?;

        info!("📋 Announced {} RDP formats to Portal: {:?}", mime_types.len(), mime_types);
        Ok(())
    }

    /// Start listening for SelectionTransfer signals
    ///
    /// Called when Linux app requests clipboard data (user pasted).
    /// Handler should fetch data from RDP client and call provide_data().
    ///
    /// # Arguments
    ///
    /// * `handler` - Async function to request data from RDP: (mime_type, serial) -> data
    pub async fn start_selection_transfer_listener<F, Fut>(
        &self,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(String, u32) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Vec<u8>>> + Send + 'static,
    {
        let mut stream = self.clipboard.receive_selection_transfer().await
            .context("Failed to receive selection transfer signal")?;

        let pending = Arc::clone(&self.pending_requests);
        let clipboard = self.clipboard.clone();
        let session_path = self.session.path().to_owned();

        tokio::spawn(async move {
            info!("🎧 Selection transfer listener started");

            while let Some((_sess, mime_type, serial)) = stream.next().await {
                info!("🔔 Portal requesting data: {} (serial {})", mime_type, serial);

                // Track this request
                {
                    let mut requests = pending.write().await;
                    requests.insert(serial, PendingRequest {
                        serial,
                        mime_type: mime_type.clone(),
                        requested_at: SystemTime::now(),
                    });
                }

                // Request data from RDP client and provide to Portal
                match handler(mime_type.clone(), serial).await {
                    Ok(data) => {
                        info!("Received {} bytes from RDP for serial {}", data.len(), serial);

                        if let Err(e) = Self::write_to_portal_fd(
                            &clipboard,
                            &session,
                            serial,
                            &data,
                        ).await {
                            error!("Failed to provide data to Portal: {}", e);
                            let _ = clipboard.selection_write_done(&session, serial, false).await;
                        }
                    }
                    Err(e) => {
                        error!("Failed to get data from RDP for {}: {}", mime_type, e);
                        let _ = clipboard.selection_write_done(&session, serial, false).await;
                    }
                }

                // Cleanup request
                pending.write().await.remove(&serial);
            }

            warn!("Selection transfer listener exited");
        });

        Ok(())
    }

    /// Provide clipboard data to Portal via file descriptor
    ///
    /// Called after getting data from RDP client in response to SelectionTransfer.
    ///
    /// # Arguments
    ///
    /// * `serial` - Portal request serial number
    /// * `data` - Clipboard data to provide
    async fn write_to_portal_fd(
        clipboard: &Clipboard<'_>,
        session: &Session<'_, RemoteDesktop<'_>>,
        serial: u32,
        data: &[u8],
    ) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        // Get write file descriptor from Portal
        let fd = clipboard.selection_write(session, serial).await
            .context("Failed to get SelectionWrite fd")?;

        // Write data to fd
        let mut file = tokio::fs::File::from_std(fd.into());
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

    /// Start listening for SelectionOwnerChanged signals
    ///
    /// Called when local Linux clipboard changes (Linux app copies).
    /// Handler should announce formats to RDP clients.
    ///
    /// # Arguments
    ///
    /// * `handler` - Function called when clipboard changes: (mime_types)
    pub async fn start_owner_changed_listener<F>(
        &self,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(Vec<String>) + Send + Sync + 'static,
    {
        let mut stream = self.clipboard.receive_selection_owner_changed().await
            .context("Failed to receive selection owner changed signal")?;

        tokio::spawn(async move {
            info!("🎧 Selection owner changed listener started");

            while let Some((_sess, change)) = stream.next().await {
                if change.session_is_owner().unwrap_or(false) {
                    // We own it (we just announced RDP data via SetSelection) - ignore
                    debug!("Ignoring self-owned clipboard change");
                    continue;
                }

                // Another app owns clipboard - announce to RDP clients
                let mime_types = change.mime_types();

                if !mime_types.is_empty() {
                    info!("🔔 Local clipboard changed: {:?}", mime_types);
                    handler(mime_types);
                } else {
                    debug!("Clipboard cleared");
                }
            }

            warn!("Selection owner changed listener exited");
        });

        Ok(())
    }

    /// Read from local Wayland clipboard
    ///
    /// Used when RDP client requests our clipboard data (Linux → Windows copy).
    ///
    /// # Arguments
    ///
    /// * `mime_type` - MIME type to read (e.g., "text/plain")
    ///
    /// # Returns
    ///
    /// Clipboard data in requested format
    pub async fn read_local_clipboard(&self, mime_type: &str) -> Result<Vec<u8>> {
        use tokio::io::AsyncReadExt;

        debug!("Reading local clipboard: {}", mime_type);

        let fd = self.clipboard.selection_read(&self.session, mime_type).await
            .context("Failed to get SelectionRead fd")?;

        let mut file = tokio::fs::File::from_std(fd.into());
        let mut data = Vec::new();
        file.read_to_end(&mut data).await
            .context("Failed to read clipboard data from fd")?;

        info!("📖 Read {} bytes from local clipboard ({})", data.len(), mime_type);
        Ok(data)
    }

    /// Get clipboard session
    pub fn session(&self) -> &Session<'static, RemoteDesktop<'static>> {
        &self.session
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
