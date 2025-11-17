//! ScreenCast portal integration
//!
//! Provides access to screen content via xdg-desktop-portal ScreenCast interface.

use ashpd::desktop::screencast::{Screencast, SourceType, CursorMode};
use ashpd::desktop::PersistMode;
use ashpd::WindowIdentifier;
use std::sync::Arc;
use std::os::fd::{AsRawFd, RawFd};
use anyhow::{Result, Context};
use tracing::{info, debug};

use crate::config::Config;
use super::session::StreamInfo;

/// ScreenCast portal manager
pub struct ScreenCastManager {
    config: Arc<Config>,
}

impl ScreenCastManager {
    /// Create new ScreenCast manager
    pub async fn new(_connection: zbus::Connection, config: Arc<Config>) -> Result<Self> {
        info!("Initializing ScreenCast portal manager");
        Ok(Self { config })
    }

    /// Create a screencast session
    pub async fn create_session(&self) -> Result<ashpd::desktop::Session<'static, Screencast<'static>>> {
        info!("Creating ScreenCast session");

        let proxy = Screencast::new().await?;
        let session = proxy.create_session().await?;

        debug!("ScreenCast session created");
        Ok(session)
    }

    /// Select sources (monitors, windows, etc.)
    pub async fn select_sources(
        &self,
        session: &ashpd::desktop::Session<'_, Screencast<'_>>,
    ) -> Result<()> {
        info!("Selecting screencast sources");

        let proxy = Screencast::new().await?;

        // Parse cursor mode from config
        let cursor_mode = match self.config.video.cursor_mode.as_str() {
            "hidden" => CursorMode::Hidden,
            "embedded" => CursorMode::Embedded,
            "metadata" => CursorMode::Metadata,
            _ => CursorMode::Metadata,
        };

        // Select sources: monitors + windows if available
        let source_types = SourceType::Monitor | SourceType::Window;

        proxy.select_sources(
            session,
            cursor_mode,
            source_types.into(),
            true,  // multiple sources
            None,  // no restore token yet
            PersistMode::DoNot,  // don't persist for now
        ).await.context("Failed to select sources")?;

        info!("Sources selected successfully");
        Ok(())
    }

    /// Start the screencast and get PipeWire details
    pub async fn start(
        &self,
        session: &ashpd::desktop::Session<'_, Screencast<'_>>,
    ) -> Result<(RawFd, Vec<StreamInfo>)> {
        info!("Starting screencast session");

        let proxy = Screencast::new().await?;

        // Start returns a Request that resolves to Streams
        // None for headless/no parent window
        let streams_request = proxy.start(session, None)
            .await
            .context("Failed to start screencast")?;

        // Get the streams from the request response
        let streams = streams_request.response()?;

        info!("Screencast started with {} streams", streams.streams().len());

        // Get PipeWire FD
        let fd = proxy.open_pipe_wire_remote(session)
            .await
            .context("Failed to open PipeWire remote")?;

        let raw_fd = fd.as_raw_fd();
        info!("PipeWire FD obtained: {}", raw_fd);

        // Convert stream info using new API
        let stream_info: Vec<StreamInfo> = streams.streams().iter().map(|stream| {
            let size = stream.size().unwrap_or((0, 0));
            StreamInfo {
                node_id: stream.pipe_wire_node_id(),
                position: stream.position().unwrap_or((0, 0)),
                size: (size.0 as u32, size.1 as u32), // Convert from i32 to u32
                source_type: super::session::SourceType::Monitor, // Simplified for now
            }
        }).collect();

        // Don't close fd - we need to keep it
        std::mem::forget(fd);

        Ok((raw_fd, stream_info))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Portal tests require a running Wayland session with portal
    // These are integration tests that may not work in CI

    #[tokio::test]
    #[ignore] // Ignore in CI, run manually
    async fn test_screencast_manager_creation() {
        let connection = zbus::Connection::session().await.unwrap();
        let config = Arc::new(Config::default_config().unwrap());

        let manager = ScreenCastManager::new(connection, config).await;
        assert!(manager.is_ok());
    }
}
