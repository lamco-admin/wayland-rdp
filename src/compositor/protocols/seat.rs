//! wl_seat protocol implementation
//!
//! Implements input device management (keyboard, pointer, touch).

use smithay::input::{Seat, SeatHandler, SeatState, keyboard::XkbConfig, pointer::CursorImageStatus};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::delegate_seat;
use smithay::reexports::wayland_server::DisplayHandle;
use crate::compositor::state::CompositorState;
use tracing::{debug, info, trace};

/// Seat (input) protocol handler
impl SeatHandler for CompositorState {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        self.seat_state.as_mut()
            .expect("Seat state not initialized - call init_smithay_states() first")
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&WlSurface>) {
        if focused.is_some() {
            debug!("Focus changed to surface");
        } else {
            debug!("Focus cleared");
        }
    }

    fn cursor_image(&mut self, seat: &Seat<Self>, image: CursorImageStatus) {
        trace!("Cursor image changed: {:?}", image);

        // Update cursor state based on the image
        match image {
            CursorImageStatus::Hidden => {
                debug!("Cursor hidden");
                self.pointer.cursor.visible = false;
            }
            CursorImageStatus::Named(name) => {
                debug!("Cursor changed to named: {}", name);
                self.pointer.cursor.visible = true;
                // In a real implementation, we'd load the cursor theme
            }
            CursorImageStatus::Surface(surface) => {
                debug!("Cursor changed to custom surface");
                self.pointer.cursor.visible = true;
                // Custom cursor surface would be rendered
            }
        }

        self.damage_all();
    }
}

// Delegate seat protocol to Smithay
delegate_seat!(CompositorState);

/// Initialize seat global with keyboard and pointer capabilities
pub fn init_seat_global(display: &DisplayHandle, seat_name: &str) -> (SeatState<CompositorState>, Seat<CompositorState>) {
    info!("Initializing wl_seat global: {}", seat_name);

    let mut seat_state = SeatState::new();

    // Create seat
    let mut seat = seat_state.new_wl_seat(display, seat_name);

    // Add keyboard capability
    seat.add_keyboard(
        XkbConfig::default(),
        200, // Repeat delay (ms)
        25,  // Repeat rate (per second)
    ).expect("Failed to add keyboard to seat");

    debug!("Keyboard capability added to seat");

    // Add pointer capability
    seat.add_pointer();
    debug!("Pointer capability added to seat");

    // Touch capability can be added if needed
    // seat.add_touch();

    info!("Seat '{}' initialized with keyboard and pointer", seat_name);

    (seat_state, seat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seat_capabilities() {
        // Test that we can create a seat with capabilities
        // This would require a full Wayland display setup in a real test
    }
}
