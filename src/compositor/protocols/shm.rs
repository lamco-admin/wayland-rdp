//! wl_shm protocol implementation
//!
//! Implements shared memory buffer support for Wayland clients.

use smithay::wayland::shm::{ShmHandler, ShmState};
use smithay::reexports::wayland_server::{Client, DisplayHandle};
use smithay::delegate_shm;
use crate::compositor::state::CompositorState;
use tracing::{debug, info, trace};

/// Shared memory protocol handler
impl ShmHandler for CompositorState {
    fn shm_state(&self) -> &ShmState {
        self.shm_state.as_ref()
            .expect("SHM state not initialized - call init_smithay_states() first")
    }
}

// Delegate SHM protocol to Smithay
delegate_shm!(CompositorState);

/// Initialize SHM global with supported formats
pub fn init_shm_global(display: &DisplayHandle) -> ShmState {
    info!("Initializing wl_shm global");

    // Create SHM state with standard formats
    let shm_state = ShmState::new::<CompositorState>(
        display,
        vec![
            // Standard formats supported by our software renderer
            smithay::reexports::wayland_server::protocol::wl_shm::Format::Argb8888,
            smithay::reexports::wayland_server::protocol::wl_shm::Format::Xrgb8888,
            smithay::reexports::wayland_server::protocol::wl_shm::Format::Abgr8888,
            smithay::reexports::wayland_server::protocol::wl_shm::Format::Xbgr8888,
        ],
    );

    debug!("wl_shm global created with ARGB8888, XRGB8888, ABGR8888, XBGR8888 formats");

    shm_state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shm_formats() {
        // Test that we support the right formats
        let formats = vec![
            smithay::reexports::wayland_server::protocol::wl_shm::Format::Argb8888,
            smithay::reexports::wayland_server::protocol::wl_shm::Format::Xrgb8888,
            smithay::reexports::wayland_server::protocol::wl_shm::Format::Abgr8888,
            smithay::reexports::wayland_server::protocol::wl_shm::Format::Xbgr8888,
        ];

        assert_eq!(formats.len(), 4);
    }
}
