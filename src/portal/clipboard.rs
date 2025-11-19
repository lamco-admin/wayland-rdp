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
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info};

/// Selection transfer event from Portal
#[derive(Debug, Clone)]
pub struct SelectionTransferEvent {
    pub mime_type: String,
    pub serial: u32,
}

/// Portal Clipboard Manager
///
/// Integrates RDP clipboard with Wayland via Portal Clipboard API.
/// Supports delayed rendering where formats are announced without data,
/// and data is only transferred when actually requested.
pub struct ClipboardManager {
    /// Portal Clipboard interface (Arc-wrapped for sharing across tasks)
    clipboard: Arc<Clipboard<'static>>,
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
            clipboard: Arc::new(clipboard),
        };

        Ok(manager)
    }

    /// Start listening for SelectionTransfer events (delayed rendering requests)
    ///
    /// When a Linux application pastes, Portal sends SelectionTransfer with:
    /// - mime_type: The requested data format
    /// - serial: Unique ID to track this request
    ///
    /// Spawns a background task that listens for SelectionTransfer signals and
    /// sends events to the provided channel.
    pub async fn start_selection_transfer_listener(
        &self,
        event_tx: mpsc::UnboundedSender<SelectionTransferEvent>,
    ) -> anyhow::Result<()> {
        // Clone the Arc clipboard reference (cheap)
        let clipboard = Arc::clone(&self.clipboard);

        // Start the stream in a task to avoid lifetime issues
        tokio::spawn(async move {
            use futures_util::stream::StreamExt;

            // Create stream inside the task
            let stream_result = clipboard.receive_selection_transfer().await;

            match stream_result {
                Ok(stream) => {
                    let mut stream = Box::pin(stream);

                    while let Some((_, mime_type, serial)) = stream.next().await {
                        debug!("📥 SelectionTransfer signal: mime={}, serial={}", mime_type, serial);

                        let event = SelectionTransferEvent {
                            mime_type,
                            serial,
                        };

                        if event_tx.send(event).is_err() {
                            info!("SelectionTransfer listener stopping (receiver dropped)");
                            break;
                        }
                    }

                    info!("SelectionTransfer listener task ended");
                }
                Err(e) => {
                    info!("Failed to receive SelectionTransfer stream: {:#}", e);
                }
            }
        });

        info!("✅ SelectionTransfer listener started - ready for delayed rendering");
        Ok(())
    }

    /// Start listening for SelectionOwnerChanged events (local clipboard changes)
    ///
    /// When clipboard ownership changes in the system (user copies in Linux app),
    /// Portal sends SelectionOwnerChanged signal with available MIME types.
    ///
    /// Spawns a background task that listens for these signals and sends events
    /// to the provided channel for announcing to RDP clients.
    pub async fn start_owner_changed_listener(
        &self,
        event_tx: mpsc::UnboundedSender<Vec<String>>,
    ) -> anyhow::Result<()> {
        use futures_util::stream::StreamExt;

        let clipboard = Arc::clone(&self.clipboard);

        tokio::spawn(async move {
            let stream_result = clipboard.receive_selection_owner_changed().await;

            match stream_result {
                Ok(stream) => {
                    let mut stream = Box::pin(stream);

                    while let Some((_, change)) = stream.next().await {
                        // Check if we are the owner (we just set the clipboard)
                        let is_owner = change.session_is_owner().unwrap_or(false);

                        if is_owner {
                            // We own the clipboard (we just announced RDP data) - ignore
                            debug!("Ignoring SelectionOwnerChanged - we are the owner");
                            continue;
                        }

                        // Another application owns the clipboard - announce to RDP clients
                        let mime_types = change.mime_types();
                        info!("📋 Local clipboard changed - new owner has {} formats: {:?}",
                            mime_types.len(), mime_types);

                        if event_tx.send(mime_types).is_err() {
                            info!("SelectionOwnerChanged listener stopping (receiver dropped)");
                            break;
                        }
                    }

                    info!("SelectionOwnerChanged listener task ended");
                }
                Err(e) => {
                    info!("Failed to receive SelectionOwnerChanged stream: {:#}", e);
                }
            }
        });

        info!("✅ SelectionOwnerChanged listener started - monitoring local clipboard");
        Ok(())
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

    /// Write clipboard data to Portal via file descriptor
    ///
    /// This is called in response to a SelectionTransfer event.
    /// The data is written to the file descriptor returned by selection_write(),
    /// and then selection_write_done() is called to notify success/failure.
    pub async fn write_selection_data(
        &self,
        session: &Session<'_, RemoteDesktop<'_>>,
        serial: u32,
        data: Vec<u8>,
    ) -> anyhow::Result<()> {
        use tokio::io::AsyncWriteExt;

        debug!("Writing {} bytes to Portal (serial {})", data.len(), serial);

        // Get write file descriptor from Portal
        let fd = self.clipboard.selection_write(session, serial).await
            .context("Failed to get SelectionWrite fd")?;

        // Convert zvariant::OwnedFd to tokio File
        let std_fd: std::os::fd::OwnedFd = fd.into();
        let std_file = std::fs::File::from(std_fd);
        let mut file = tokio::fs::File::from_std(std_file);

        // Write data to file descriptor
        match file.write_all(&data).await {
            Ok(()) => {
                file.flush().await?;
                drop(file); // Close fd before notifying Portal

                // Notify Portal of successful write
                self.clipboard.selection_write_done(session, serial, true).await
                    .context("Failed to notify Portal of write completion")?;

                info!("✅ Wrote {} bytes to Portal clipboard (serial {})", data.len(), serial);
                Ok(())
            }
            Err(e) => {
                drop(file); // Close fd on error
                // Notify Portal of failed write
                let _ = self.clipboard.selection_write_done(session, serial, false).await;
                Err(anyhow::anyhow!("Failed to write clipboard data: {}", e))
            }
        }
    }
}

impl std::fmt::Debug for ClipboardManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PortalClipboardManager")
            .field("clipboard", &"<Portal Clipboard Proxy>")
            .finish()
    }
}
