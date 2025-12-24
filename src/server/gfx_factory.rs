//! GFX Server Factory for EGFX/H.264 Video Streaming
//!
//! This module implements `ironrdp_server::GfxServerFactory` to integrate
//! our EGFX handler with IronRDP's server infrastructure.
//!
//! # Architecture
//!
//! ```text
//! WrdGfxFactory (implements GfxServerFactory)
//!       │
//!       └─> Creates WrdGraphicsHandler per connection
//!                    │
//!                    ├─> Tracks capability negotiation
//!                    ├─> Receives frame acknowledgments
//!                    └─> Reports QoE metrics
//! ```

use std::sync::Arc;
use tokio::sync::RwLock;

use ironrdp_egfx::server::GraphicsPipelineHandler;
use ironrdp_server::GfxServerFactory;

use crate::egfx::WrdGraphicsHandler;

/// Factory for creating EGFX graphics pipeline handlers
///
/// This factory is passed to the RdpServer builder and creates
/// a new `WrdGraphicsHandler` for each client connection.
pub struct WrdGfxFactory {
    /// Initial desktop dimensions
    width: u16,
    height: u16,

    /// Shared state for checking handler readiness from other parts of the server
    handler_state: Arc<RwLock<Option<HandlerState>>>,
}

/// Shared handler state accessible from display handler
#[derive(Debug, Clone)]
pub struct HandlerState {
    pub is_ready: bool,
    pub is_avc420_enabled: bool,
    pub is_avc444_enabled: bool,
    pub primary_surface_id: u16,
}

impl WrdGfxFactory {
    /// Create a new GFX factory
    ///
    /// # Arguments
    ///
    /// * `width` - Initial desktop width
    /// * `height` - Initial desktop height
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            handler_state: Arc::new(RwLock::new(None)),
        }
    }

    /// Get shared reference to handler state
    ///
    /// This can be used by the display handler to check if EGFX is ready
    /// and which codecs are available.
    pub fn handler_state(&self) -> Arc<RwLock<Option<HandlerState>>> {
        Arc::clone(&self.handler_state)
    }
}

impl GfxServerFactory for WrdGfxFactory {
    fn build_gfx_handler(&self) -> Box<dyn GraphicsPipelineHandler> {
        let handler = WrdGraphicsHandler::new(self.width, self.height);

        // TODO: Connect handler callbacks to update handler_state
        // This will allow the display handler to query EGFX readiness

        Box::new(handler)
    }
}
