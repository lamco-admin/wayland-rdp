//! X11 Backend for Lamco Compositor

use anyhow::Result;
use tracing::info;

use crate::compositor::types::CompositorConfig;

/// X11 backend
pub struct LamcoX11Backend {
    config: CompositorConfig,
}

impl LamcoX11Backend {
    pub fn new(config: CompositorConfig) -> Result<Self> {
        info!("X11 backend initialized: {}x{}", config.width, config.height);
        Ok(Self { config })
    }
}
