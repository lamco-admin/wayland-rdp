//! RemoteDesktop portal integration
//!
//! Provides input injection and screen capture via RemoteDesktop portal.

use anyhow::{Context, Result};
use ashpd::desktop::remote_desktop::{DeviceType, KeyState, RemoteDesktop};
use ashpd::desktop::PersistMode;
use enumflags2::BitFlags;
use std::os::fd::{AsRawFd, RawFd};
use std::sync::Arc;
use tracing::{debug, info};

use super::session::StreamInfo;
use crate::config::Config;

/// RemoteDesktop portal manager
pub struct RemoteDesktopManager {
    config: Arc<Config>,
}

impl RemoteDesktopManager {
    /// Create new RemoteDesktop manager
    pub async fn new(_connection: zbus::Connection, config: Arc<Config>) -> Result<Self> {
        info!("Initializing RemoteDesktop portal manager");
        Ok(Self { config })
    }

    /// Create a remote desktop session
    pub async fn create_session(
        &self,
    ) -> Result<ashpd::desktop::Session<'static, RemoteDesktop<'static>>> {
        info!("Creating RemoteDesktop session");

        let proxy = RemoteDesktop::new().await?;
        let session = proxy.create_session().await?;

        debug!("RemoteDesktop session created");

        // Note: Session can't be cloned in ashpd 0.12.0, so we return it
        // The caller is responsible for managing the session lifetime

        Ok(session)
    }

    /// Select devices for remote control
    pub async fn select_devices(
        &self,
        session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
        devices: BitFlags<DeviceType>,
    ) -> Result<()> {
        info!("Selecting devices: {:?}", devices);

        let proxy = RemoteDesktop::new().await?;

        proxy
            .select_devices(
                session,
                devices,
                None,               // No restore token yet
                PersistMode::DoNot, // Don't persist for now
            )
            .await
            .context("Failed to select devices")?;

        info!("Devices selected successfully");
        Ok(())
    }

    /// Start the remote desktop session
    pub async fn start_session(
        &self,
        session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
    ) -> Result<(RawFd, Vec<StreamInfo>)> {
        info!("Starting RemoteDesktop session");

        let proxy = RemoteDesktop::new().await?;

        // Start returns a Request that resolves to SelectedDevices
        // None for headless/no parent window
        let response = proxy
            .start(session, None)
            .await
            .context("Failed to start remote desktop session")?;

        // Get the selected devices from the request response
        let selected = response.response()?;

        let stream_count = selected.streams().map(|s| s.len()).unwrap_or(0);
        info!(
            "RemoteDesktop started with {} devices and {} streams",
            selected.devices().bits(),
            stream_count
        );

        // Get PipeWire FD - note: open_pipe_wire_remote is on the Screencast trait/methods
        // For RemoteDesktop, we need to access streams differently
        // Actually, RemoteDesktop in 0.12.0 uses the screencast portal internally
        use ashpd::desktop::screencast::Screencast;
        let screencast_proxy = Screencast::new().await?;
        let fd = screencast_proxy
            .open_pipe_wire_remote(session)
            .await
            .context("Failed to open PipeWire remote")?;

        let raw_fd = fd.as_raw_fd();
        info!("PipeWire FD obtained: {}", raw_fd);

        // Convert stream info using new API
        let stream_info: Vec<StreamInfo> = selected
            .streams()
            .map(|streams| {
                streams
                    .iter()
                    .map(|stream| {
                        let size = stream.size().unwrap_or((0, 0));
                        StreamInfo {
                            node_id: stream.pipe_wire_node_id(),
                            position: stream.position().unwrap_or((0, 0)),
                            size: (size.0 as u32, size.1 as u32), // Convert from i32 to u32
                            source_type: super::session::SourceType::Monitor,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Don't close fd - we need to keep it
        std::mem::forget(fd);

        Ok((raw_fd, stream_info))
    }

    /// Inject pointer motion (relative)
    pub async fn notify_pointer_motion(
        &self,
        session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
        dx: f64,
        dy: f64,
    ) -> Result<()> {
        let proxy = RemoteDesktop::new().await?;
        proxy.notify_pointer_motion(session, dx, dy).await?;
        Ok(())
    }

    /// Inject pointer motion (absolute in stream coordinates)
    pub async fn notify_pointer_motion_absolute(
        &self,
        session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
        stream: u32,
        x: f64,
        y: f64,
    ) -> Result<()> {
        let proxy = RemoteDesktop::new().await?;
        proxy
            .notify_pointer_motion_absolute(session, stream, x, y)
            .await?;
        Ok(())
    }

    /// Inject pointer button
    pub async fn notify_pointer_button(
        &self,
        session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
        button: i32,
        pressed: bool,
    ) -> Result<()> {
        let proxy = RemoteDesktop::new().await?;
        let state = if pressed {
            KeyState::Pressed
        } else {
            KeyState::Released
        };
        proxy.notify_pointer_button(session, button, state).await?;
        Ok(())
    }

    /// Inject pointer axis (scroll)
    pub async fn notify_pointer_axis(
        &self,
        session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
        dx: f64,
        dy: f64,
    ) -> Result<()> {
        let proxy = RemoteDesktop::new().await?;
        // In ashpd 0.12.0, notify_pointer_axis takes (session, dx, dy, finish)
        // We send both axes together with finish=true
        proxy.notify_pointer_axis(session, dx, dy, true).await?;
        Ok(())
    }

    /// Inject keyboard key
    pub async fn notify_keyboard_keycode(
        &self,
        session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
        keycode: i32,
        pressed: bool,
    ) -> Result<()> {
        let proxy = RemoteDesktop::new().await?;
        let state = if pressed {
            KeyState::Pressed
        } else {
            KeyState::Released
        };
        proxy
            .notify_keyboard_keycode(session, keycode, state)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_remote_desktop_session_creation() {
        let connection = zbus::Connection::session().await.unwrap();
        let config = Arc::new(Config::default_config().unwrap());

        let manager = RemoteDesktopManager::new(connection, config).await.unwrap();

        // This will trigger permission dialog
        // let session = manager.create_session().await;
        // assert!(session.is_ok());
    }
}
