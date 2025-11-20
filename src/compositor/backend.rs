//! Compositor backend module
//!
//! Provides backend implementations for the compositor.
//! Currently implements a simple headless backend.

use super::types::*;
use anyhow::Result;

/// Backend trait for compositor
pub trait Backend {
    /// Initialize the backend
    fn init(&mut self) -> Result<()>;

    /// Get backend name
    fn name(&self) -> &str;

    /// Present frame (for hardware backends)
    fn present(&mut self, _framebuffer: &FrameBuffer) -> Result<()> {
        // No-op for headless
        Ok(())
    }
}

/// Headless backend (no physical output)
pub struct HeadlessBackend {
    config: CompositorConfig,
}

impl HeadlessBackend {
    pub fn new(config: CompositorConfig) -> Self {
        Self { config }
    }
}

impl Backend for HeadlessBackend {
    fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing headless backend: {}x{}",
            self.config.width, self.config.height);
        Ok(())
    }

    fn name(&self) -> &str {
        "headless"
    }
}
pub mod x11;
