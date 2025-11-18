//! XDG Desktop Portal integration
//!
//! Provides unified access to ScreenCast, RemoteDesktop, and Clipboard portals.

use anyhow::{Context, Result};
use ashpd::desktop::remote_desktop::DeviceType;
use enumflags2::BitFlags;
use std::sync::Arc;
use tracing::{debug, info};

pub mod clipboard;
pub mod remote_desktop;
pub mod screencast;
pub mod session;

pub use clipboard::ClipboardManager;
pub use remote_desktop::RemoteDesktopManager;
pub use screencast::ScreenCastManager;
pub use session::{PortalSessionHandle, SourceType, StreamInfo};

use crate::config::Config;

/// Portal manager coordinates all portal interactions
pub struct PortalManager {
    config: Arc<Config>,
    connection: zbus::Connection,
    screencast: Arc<ScreenCastManager>,
    remote_desktop: Arc<RemoteDesktopManager>,
    clipboard: Arc<ClipboardManager>,
}

impl PortalManager {
    /// Create new portal manager
    pub async fn new(config: &Arc<Config>) -> Result<Self> {
        info!("Initializing Portal Manager");

        // Connect to session D-Bus
        let connection = zbus::Connection::session()
            .await
            .context("Failed to connect to D-Bus session bus")?;

        debug!("Connected to D-Bus session bus");

        // Initialize portal managers
        let screencast =
            Arc::new(ScreenCastManager::new(connection.clone(), config.clone()).await?);

        let remote_desktop =
            Arc::new(RemoteDesktopManager::new(connection.clone(), config.clone()).await?);

        let clipboard = Arc::new(ClipboardManager::new(connection.clone(), config.clone()).await?);

        info!("Portal Manager initialized successfully");

        Ok(Self {
            config: config.clone(),
            connection,
            screencast,
            remote_desktop,
            clipboard,
        })
    }

    /// Create a complete portal session (RemoteDesktop + ScreenCast)
    ///
    /// This triggers the user permission dialog and returns a session handle
    /// with PipeWire access for video and input injection capabilities.
    pub async fn create_session(&self) -> Result<PortalSessionHandle> {
        info!("Creating portal session (RemoteDesktop + ScreenCast)");

        // Create RemoteDesktop session (this includes ScreenCast capabilities)
        let session = self.remote_desktop.create_session().await?;

        // Select devices for input injection using BitFlags
        let devices: BitFlags<DeviceType> = DeviceType::Keyboard | DeviceType::Pointer;
        self.remote_desktop
            .select_devices(&session, devices)
            .await?;

        // Note: We can also use ScreenCast directly for screen-only capture
        // But RemoteDesktop gives us both screen + input

        // Start the session (triggers permission dialog)
        let (pipewire_fd, streams) = self.remote_desktop.start_session(&session).await?;

        info!("Portal session created successfully");
        debug!("  PipeWire FD: {}", pipewire_fd);
        debug!("  Streams: {}", streams.len());

        // Create session handle - use a placeholder string ID
        let session_id = format!("portal-session-{}", uuid::Uuid::new_v4());
        let handle =
            PortalSessionHandle::new(session_id.clone(), pipewire_fd, streams, Some(session_id));

        Ok(handle)
    }

    /// Access screencast manager
    pub fn screencast(&self) -> &Arc<ScreenCastManager> {
        &self.screencast
    }

    /// Access remote desktop manager
    pub fn remote_desktop(&self) -> &Arc<RemoteDesktopManager> {
        &self.remote_desktop
    }

    /// Access clipboard manager
    pub fn clipboard(&self) -> &Arc<ClipboardManager> {
        &self.clipboard
    }

    /// Cleanup all portal resources
    pub async fn cleanup(&self) -> Result<()> {
        info!("Cleaning up portal resources");
        // Portal sessions are automatically cleaned up
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Wayland session
    async fn test_portal_manager_creation() {
        let config = Arc::new(Config::default_config().unwrap());
        let manager = PortalManager::new(&config).await;

        // May fail if not in Wayland session or portal not available
        if manager.is_err() {
            eprintln!("Portal manager creation failed (expected if not in Wayland session)");
        }
    }
}
