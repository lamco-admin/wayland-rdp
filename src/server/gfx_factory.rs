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
//!       ├─► Creates Arc<Mutex<GraphicsPipelineServer>>
//!       │
//!       ├─► Returns GfxDvcBridge for DrdynvcServer (handles client messages)
//!       │
//!       └─► Stores GfxServerHandle for display handler (frame sending)
//! ```
//!
//! # Hybrid Architecture
//!
//! This factory implements the "Hybrid" approach (Option E) for proactive EGFX
//! frame sending:
//!
//! 1. **GfxDvcBridge** - Wraps the GraphicsPipelineServer in Arc<Mutex<>>
//!    and implements DvcProcessor for the DrdynvcServer to use
//!
//! 2. **GfxServerHandle** - Clone of the Arc given to display handler for
//!    calling send_avc420_frame() directly
//!
//! 3. **ServerEvent::Egfx** - Routes the resulting DVC messages to the wire

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use ironrdp_egfx::server::{GraphicsPipelineHandler, GraphicsPipelineServer};
use ironrdp_server::{GfxDvcBridge, GfxServerFactory, GfxServerHandle};

use crate::egfx::WrdGraphicsHandler;

/// Factory for creating EGFX graphics pipeline handlers
///
/// This factory is passed to the RdpServer builder and creates
/// a shared `GraphicsPipelineServer` for each client connection.
///
/// # Usage
///
/// ```ignore
/// let gfx_factory = WrdGfxFactory::new(width, height);
///
/// // Get handle for display handler before passing to RdpServer
/// let gfx_handle = gfx_factory.server_handle();
///
/// let server = RdpServer::builder()
///     .with_gfx_handler(gfx_factory)
///     // ...
///     .build();
///
/// // Display handler uses gfx_handle to send frames
/// display_handler.set_gfx_server(gfx_handle);
/// ```
pub struct WrdGfxFactory {
    /// Initial desktop dimensions
    width: u16,
    height: u16,

    /// Shared state for checking handler readiness from other parts of the server
    handler_state: Arc<RwLock<Option<HandlerState>>>,

    /// Shared GraphicsPipelineServer for proactive frame sending
    /// Created lazily on first call to build_server_with_handle()
    server_handle: Arc<RwLock<Option<GfxServerHandle>>>,
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
            server_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// Get shared reference to handler state
    ///
    /// This can be used by the display handler to check if EGFX is ready
    /// and which codecs are available.
    pub fn handler_state(&self) -> Arc<RwLock<Option<HandlerState>>> {
        Arc::clone(&self.handler_state)
    }

    /// Get the shared GraphicsPipelineServer handle
    ///
    /// This returns the handle that was created by `build_server_with_handle()`.
    /// Use this to access the server for frame sending from the display handler.
    ///
    /// Returns `None` if `build_server_with_handle()` hasn't been called yet
    /// (i.e., the RDP connection hasn't started the channel attachment phase).
    pub fn server_handle(&self) -> Arc<RwLock<Option<GfxServerHandle>>> {
        Arc::clone(&self.server_handle)
    }
}

impl GfxServerFactory for WrdGfxFactory {
    fn build_gfx_handler(&self) -> Box<dyn GraphicsPipelineHandler> {
        // Basic mode: just return the handler without shared access
        let handler = WrdGraphicsHandler::new(self.width, self.height);
        Box::new(handler)
    }

    fn build_server_with_handle(&self) -> Option<(GfxDvcBridge, GfxServerHandle)> {
        // Create the handler
        let handler = WrdGraphicsHandler::new(self.width, self.height);

        // Create the GraphicsPipelineServer wrapped in Arc<Mutex<>>
        let server = Arc::new(Mutex::new(GraphicsPipelineServer::new(Box::new(handler))));

        // Store handle for later access by display handler
        // Note: We use blocking_write here because this is called during sync channel setup
        if let Ok(mut handle_guard) = self.server_handle.try_write() {
            *handle_guard = Some(Arc::clone(&server));
        }

        // Create bridge for DVC infrastructure
        let bridge = GfxDvcBridge::new(Arc::clone(&server));

        Some((bridge, server))
    }
}
