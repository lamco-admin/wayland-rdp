//! wl_compositor protocol implementation
//!
//! Implements the core Wayland compositor protocol for surface and region management.

use smithay::wayland::compositor::{
    CompositorClientState, CompositorHandler, CompositorState as SmithayCompositorState,
};
use smithay::backend::renderer::utils::on_commit_buffer_handler;
use smithay::reexports::wayland_server::protocol::{wl_surface, wl_region, wl_buffer::WlBuffer};
use smithay::reexports::wayland_server::{Client, DataInit, Dispatch, DisplayHandle, Resource};
use smithay::wayland::buffer::BufferHandler;
use smithay::delegate_compositor;
use crate::compositor::state::CompositorState;
use tracing::{debug, trace, warn};

/// Compositor protocol handler implementation
impl CompositorHandler for CompositorState {
    fn compositor_state(&mut self) -> &mut SmithayCompositorState {
        self.smithay_compositor_state.as_mut()
            .expect("Smithay compositor state not initialized - call init_smithay_states() first")
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn new_surface(&mut self, surface: &wl_surface::WlSurface) {
        debug!("New Wayland surface created");

        // Surface data is automatically managed by Smithay
        // We track it in our compositor state when it gets a buffer
        trace!("Surface registered with compositor");
    }

    fn new_subsurface(
        &mut self,
        surface: &wl_surface::WlSurface,
        parent: &wl_surface::WlSurface,
    ) {
        debug!("New subsurface created with parent");

        // Subsurface hierarchy is managed by Smithay
        // We just log the event for monitoring
    }

    fn commit(&mut self, surface: &wl_surface::WlSurface) {
        trace!("Surface commit");

        // Handle buffer attachment and damage
        on_commit_buffer_handler::<Self>(surface);

        // Trigger frame rendering
        self.damage_all();
    }

    fn destroyed(&mut self, surface: &wl_surface::WlSurface) {
        debug!("Surface destroyed");

        // Cleanup will be handled by Smithay's state management
        // We just need to remove from our tracking if needed
        self.damage_all();
    }
}

/// Buffer handler for managing wl_buffer lifecycle
impl BufferHandler for CompositorState {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {
        trace!("Buffer destroyed");
        // Buffer cleanup is handled by Smithay
    }
}

/// Client state for per-client compositor data
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientState {
    pub fn new(compositor_state: CompositorClientState) -> Self {
        Self { compositor_state }
    }
}

// Delegate compositor protocol to Smithay
delegate_compositor!(CompositorState);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::types::CompositorConfig;

    #[test]
    fn test_compositor_handler_creation() {
        let config = CompositorConfig::default();
        let state = CompositorState::new(config);
        assert!(state.is_ok());
    }
}
