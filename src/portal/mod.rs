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
    clipboard: Option<Arc<ClipboardManager>>,
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

        // Clipboard manager requires a RemoteDesktop session
        // It will be created after session is established in create_session_with_clipboard()

        info!("Portal Manager initialized successfully");

        Ok(Self {
            config: config.clone(),
            connection,
            screencast,
            remote_desktop,
            clipboard: None, // Created later with session
        })
    }

    /// Create a complete portal session (ScreenCast for video, RemoteDesktop for input)
    ///
    /// This triggers the user permission dialog and returns a session handle
    /// with PipeWire access for video and input injection capabilities.
    ///
    /// # Flow
    ///
    /// 1. Create combined RemoteDesktop session (includes ScreenCast capability)
    /// 2. Select devices (keyboard + pointer for input injection)
    /// 3. Select sources (monitors to capture for screen sharing)
    /// 4. Start session (triggers permission dialog)
    /// 5. Get PipeWire FD and stream information
    ///
    /// # Returns
    ///
    /// PortalSessionHandle with PipeWire FD, stream information, and session reference
    pub async fn create_session(&self) -> Result<PortalSessionHandle> {
        info!("Creating combined portal session (ScreenCast + RemoteDesktop)");

        // Create RemoteDesktop session (this type of session can include screen sharing)
        let remote_desktop_session = self.remote_desktop.create_session().await
            .context("Failed to create RemoteDesktop session")?;

        info!("RemoteDesktop session created");

        // Select devices for input injection
        use ashpd::desktop::remote_desktop::DeviceType;
        use enumflags2::BitFlags;
        let devices = BitFlags::from(DeviceType::Keyboard) | DeviceType::Pointer;
        self.remote_desktop
            .select_devices(&remote_desktop_session, devices)
            .await
            .context("Failed to select input devices")?;

        info!("Input devices selected (keyboard + pointer)");

        // CRITICAL FIX: Also use ScreenCast to select screen sources
        // This is what makes screens available for sharing
        let screencast_proxy = ashpd::desktop::screencast::Screencast::new().await?;

        use ashpd::desktop::screencast::{CursorMode, SourceType};
        use ashpd::desktop::PersistMode;

        screencast_proxy
            .select_sources(
                &remote_desktop_session,  // Use same session
                CursorMode::Metadata,      // Include cursor metadata
                SourceType::Monitor.into(), // Request monitor sources
                true,                       // Allow multiple monitors
                None,                       // No restore token
                PersistMode::DoNot,        // Don't persist
            )
            .await
            .context("Failed to select screen sources")?;

        info!("Screen sources selected - permission dialog will appear");

        // Start the combined session (triggers permission dialog)
        let (pipewire_fd, streams) = self.remote_desktop.start_session(&remote_desktop_session).await
            .context("Failed to start RemoteDesktop session")?;

        info!("Portal session started successfully");
        info!("  PipeWire FD: {}", pipewire_fd);
        info!("  Streams: {}", streams.len());

        if streams.is_empty() {
            anyhow::bail!("No streams available - user may have denied permission or no screens to share");
        }

        // Create session handle with session reference
        // We need to keep the session alive for input injection
        let session_id = format!("portal-session-{}", uuid::Uuid::new_v4());
        let stream_count = streams.len();
        let handle = PortalSessionHandle::new(
            session_id.clone(),
            pipewire_fd,
            streams,
            Some(session_id.clone()), // Store session ID for input operations
            remote_desktop_session, // Pass the actual ashpd session for input injection
        );

        info!("Portal session handle created with {} streams", stream_count);

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
    pub fn clipboard(&self) -> Option<&Arc<ClipboardManager>> {
        self.clipboard.as_ref()
    }

    /// Set clipboard manager (called after session creation)
    pub fn set_clipboard(&mut self, clipboard: Arc<ClipboardManager>) {
        self.clipboard = Some(clipboard);
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
