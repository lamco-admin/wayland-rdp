//! Portal session management
//!
//! Manages the lifecycle of portal sessions and associated resources.

use anyhow::Result;
use std::os::fd::RawFd;
use tracing::info;

/// Information about a PipeWire stream from the portal
#[derive(Debug, Clone)]
pub struct StreamInfo {
    /// PipeWire node ID
    pub node_id: u32,

    /// Stream position (for multi-monitor)
    pub position: (i32, i32),

    /// Stream size
    pub size: (u32, u32),

    /// Source type (monitor, window, etc.)
    pub source_type: SourceType,
}

/// Source type for streams
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Monitor,
    Window,
    Virtual,
}

/// Handle to an active portal session
pub struct PortalSessionHandle {
    /// Session identifier from portal
    pub session_id: String,

    /// PipeWire file descriptor
    pub pipewire_fd: RawFd,

    /// Available streams (one per monitor typically)
    pub streams: Vec<StreamInfo>,

    /// RemoteDesktop session for input injection
    pub remote_desktop_session: Option<String>,
}

impl PortalSessionHandle {
    /// Create new session handle
    pub fn new(
        session_id: String,
        pipewire_fd: RawFd,
        streams: Vec<StreamInfo>,
        remote_desktop_session: Option<String>,
    ) -> Self {
        info!(
            "Created portal session handle: {}, {} streams, fd: {}",
            session_id,
            streams.len(),
            pipewire_fd
        );

        Self {
            session_id,
            pipewire_fd,
            streams,
            remote_desktop_session,
        }
    }

    /// Get PipeWire file descriptor
    pub fn pipewire_fd(&self) -> RawFd {
        self.pipewire_fd
    }

    /// Get stream information
    pub fn streams(&self) -> &[StreamInfo] {
        &self.streams
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get remote desktop session (for input injection)
    pub fn remote_desktop_session(&self) -> Option<&str> {
        self.remote_desktop_session.as_deref()
    }

    /// Close the portal session
    pub async fn close(self) -> Result<()> {
        info!("Closing portal session: {}", self.session_id);

        // Close PipeWire FD
        unsafe {
            libc::close(self.pipewire_fd);
        }

        // Portal sessions are automatically cleaned up when dropped

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_handle_creation() {
        let streams = vec![StreamInfo {
            node_id: 42,
            position: (0, 0),
            size: (1920, 1080),
            source_type: SourceType::Monitor,
        }];

        let handle = PortalSessionHandle::new(
            "test-session".to_string(),
            3,
            streams.clone(),
            Some("rd-session".to_string()),
        );

        assert_eq!(handle.session_id(), "test-session");
        assert_eq!(handle.pipewire_fd(), 3);
        assert_eq!(handle.streams().len(), 1);
        assert_eq!(handle.remote_desktop_session(), Some("rd-session"));
    }
}
