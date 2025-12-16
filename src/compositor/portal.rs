//! Embedded portal backend
//!
//! Provides internal portal-like APIs without D-Bus overhead.

use super::CompositorHandle;
use anyhow::Result;

/// Embedded portal backend
pub struct EmbeddedPortal {
    compositor: CompositorHandle,
}

impl EmbeddedPortal {
    pub fn new(compositor: CompositorHandle) -> Self {
        Self { compositor }
    }

    /// Get framebuffer (ScreenCast equivalent)
    pub fn get_framebuffer(&self) -> Vec<u8> {
        self.compositor.get_framebuffer()
    }

    /// Get clipboard (clipboard portal equivalent)
    pub fn get_clipboard(&self) -> Result<Vec<u8>> {
        self.compositor.get_clipboard()
    }

    /// Set clipboard
    pub fn set_clipboard(&self, data: Vec<u8>) -> Result<()> {
        self.compositor.set_clipboard(data)
    }
}
