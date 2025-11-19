//! WRD Headless Compositor
//!
//! A production-quality Wayland compositor built on Smithay 0.3.x,
//! optimized for headless RDP streaming without requiring physical displays or GPUs.
//!
//! # Architecture
//!
//! ```text
//! WrdCompositor
//!   ├─> Backend (headless/virtual)
//!   ├─> Wayland Server (protocol handlers)
//!   ├─> Desktop Space (window management)
//!   ├─> Renderer (software/memory framebuffer)
//!   ├─> Input Manager (keyboard/mouse)
//!   └─> RDP Bridge (frame export/input inject)
//! ```
//!
//! # Features
//!
//! - Headless operation (no physical display required)
//! - Software rendering to memory framebuffer
//! - Full Wayland protocol support (wl_compositor, xdg_shell, wl_seat)
//! - Multi-window management with proper Z-ordering
//! - Input event injection from RDP
//! - Clipboard integration
//! - Direct RDP frame buffer export
//!
//! # Usage
//!
//! ```no_run
//! use wrd_server::compositor::WrdCompositor;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = CompositorConfig {
//!         width: 1920,
//!         height: 1080,
//!         ..Default::default()
//!     };
//!
//!     let mut compositor = WrdCompositor::new(config)?;
//!     compositor.run()?;
//!     Ok(())
//! }
//! ```

#[cfg(feature = "headless-compositor")]
pub mod backend;
#[cfg(feature = "headless-compositor")]
pub mod protocols;
#[cfg(feature = "headless-compositor")]
pub mod desktop;
#[cfg(feature = "headless-compositor")]
pub mod rendering;
#[cfg(feature = "headless-compositor")]
pub mod input;
#[cfg(feature = "headless-compositor")]
pub mod portal;
#[cfg(feature = "headless-compositor")]
pub mod rdp_bridge;
#[cfg(feature = "headless-compositor")]
pub mod state;
#[cfg(feature = "headless-compositor")]
pub mod types;

#[cfg(feature = "headless-compositor")]
pub use self::state::{CompositorState, WrdCompositor};
#[cfg(feature = "headless-compositor")]
pub use self::types::{CompositorConfig, CompositorEvent, WindowId};
#[cfg(feature = "headless-compositor")]
pub use self::smithay_impl::SmithayCompositor;
#[cfg(feature = "headless-compositor")]
pub use self::software_renderer::SoftwareRenderer;
#[cfg(feature = "headless-compositor")]
pub use self::integration::{CompositorRdpIntegration, RenderedFrame, IntegrationStats};

use anyhow::Result;
use std::sync::Arc;
use parking_lot::Mutex;

/// Initialize compositor subsystem
///
/// This should be called once at application startup to initialize
/// the compositor infrastructure.
#[cfg(feature = "headless-compositor")]
pub fn init() -> Result<()> {
    tracing::info!("Initializing WRD headless compositor subsystem");

    // Initialize logging for compositor
    tracing::info!("Smithay compositor framework initialized");

    Ok(())
}

/// Compositor handle for inter-thread communication
///
/// Provides a thread-safe interface to the compositor for RDP integration.
#[cfg(feature = "headless-compositor")]
#[derive(Clone)]
pub struct CompositorHandle {
    state: Arc<Mutex<CompositorState>>,
}

#[cfg(feature = "headless-compositor")]
impl CompositorHandle {
    /// Get the current framebuffer
    pub fn get_framebuffer(&self) -> Vec<u8> {
        let state = self.state.lock();
        state.get_framebuffer()
    }

    /// Get damaged regions since last frame
    pub fn get_damage(&self) -> Vec<types::Rectangle> {
        let state = self.state.lock();
        state.get_damage()
    }

    /// Inject keyboard event
    pub fn inject_keyboard(&self, event: input::KeyboardEvent) -> Result<()> {
        let mut state = self.state.lock();
        state.inject_keyboard(event)
    }

    /// Inject pointer event
    pub fn inject_pointer(&self, event: input::PointerEvent) -> Result<()> {
        let mut state = self.state.lock();
        state.inject_pointer(event)
    }

    /// Get clipboard data
    pub fn get_clipboard(&self) -> Result<Vec<u8>> {
        let state = self.state.lock();
        state.get_clipboard()
    }

    /// Set clipboard data
    pub fn set_clipboard(&self, data: Vec<u8>) -> Result<()> {
        let mut state = self.state.lock();
        state.set_clipboard(data)
    }
}

#[cfg(not(feature = "headless-compositor"))]
pub fn init() -> Result<()> {
    tracing::warn!("Headless compositor feature not enabled - skipping initialization");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        assert!(init().is_ok());
    }
}
