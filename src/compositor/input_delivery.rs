//! Input delivery to Wayland clients
//!
//! Delivers keyboard and pointer input from RDP to focused Wayland surfaces.

use super::state::CompositorState;
use super::input::{KeyboardEvent, PointerEvent};
use smithay::input::{Seat, keyboard::Keycode, pointer::{AxisFrame, ButtonEvent, MotionEvent}};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point, SERIAL_COUNTER};
use anyhow::Result;
use tracing::{debug, trace};

/// Input delivery manager
pub struct InputDelivery {
    /// Last known pointer position
    pointer_position: Point<f64, Logical>,
}

impl InputDelivery {
    /// Create new input delivery manager
    pub fn new() -> Self {
        Self {
            pointer_position: Point::from((0.0, 0.0)),
        }
    }

    /// Deliver keyboard event to focused surface
    pub fn deliver_keyboard(
        &mut self,
        event: &KeyboardEvent,
        state: &mut CompositorState,
    ) -> Result<()> {
        trace!("Delivering keyboard event: key={}, state={:?}",
            event.key, event.state);

        // Get seat
        let seat = match &mut state.seat {
            Some(seat) => seat,
            None => {
                trace!("No seat available");
                return Ok(());
            }
        };

        // Get keyboard
        let keyboard = match seat.get_keyboard() {
            Some(kbd) => kbd,
            None => {
                trace!("No keyboard on seat");
                return Ok(());
            }
        };

        // Get serial
        let serial = state.next_serial();

        // Convert to Smithay key code
        let key_code = Keycode::new(event.key);

        // Convert key state
        let key_state = match event.state {
            super::types::KeyState::Pressed => smithay::backend::input::KeyState::Pressed,
            super::types::KeyState::Released => smithay::backend::input::KeyState::Released,
        };

        // Deliver key event
        keyboard.input::<(), _>(
            state,
            key_code,
            key_state,
            serial,
            event.timestamp as u32,
            |_, _modifiers, _keysym| {
                // Filter function - we accept all keys
                smithay::input::keyboard::FilterResult::Forward
            },
        );

        debug!("Keyboard event delivered: key={}", event.key);

        Ok(())
    }

    /// Deliver pointer motion event
    pub fn deliver_pointer_motion(
        &mut self,
        event: &PointerEvent,
        state: &mut CompositorState,
    ) -> Result<()> {
        trace!("Delivering pointer motion: ({}, {})", event.x, event.y);

        // Update pointer position
        self.pointer_position = Point::from((event.x as f64, event.y as f64));

        // Get seat
        let seat = match &mut state.seat {
            Some(seat) => seat,
            None => {
                trace!("No seat available");
                return Ok(());
            }
        };

        // Get pointer
        let pointer = match seat.get_pointer() {
            Some(ptr) => ptr,
            None => {
                trace!("No pointer on seat");
                return Ok(());
            }
        };

        // Find surface under pointer
        let under = self.surface_under_pointer(state);

        // Get serial
        let serial = state.next_serial();

        // Deliver motion event
        pointer.motion(
            state,
            under.as_ref().map(|(s, p)| (s.clone(), p.to_f64())),
            &MotionEvent {
                location: self.pointer_position,
                serial,
                time: event.timestamp as u32,
            },
        );

        trace!("Pointer motion delivered");

        Ok(())
    }

    /// Deliver pointer button event
    pub fn deliver_pointer_button(
        &mut self,
        event: &PointerEvent,
        state: &mut CompositorState,
    ) -> Result<()> {
        trace!("Delivering pointer button: button={:?}, pressed={}",
            event.button, event.button.is_some());

        // Get seat
        let seat = match &mut state.seat {
            Some(seat) => seat,
            None => {
                trace!("No seat available");
                return Ok(());
            }
        };

        // Get pointer
        let pointer = match seat.get_pointer() {
            Some(ptr) => ptr,
            None => {
                trace!("No pointer on seat");
                return Ok(());
            }
        };

        // Get button and state
        if let Some((button_code, btn_state)) = event.button {
            let button_state = match btn_state {
                super::types::ButtonState::Pressed => smithay::backend::input::ButtonState::Pressed,
                super::types::ButtonState::Released => smithay::backend::input::ButtonState::Released,
            };

            // Get serial
            let serial = state.next_serial();

            // Deliver button event
            pointer.button(
                state,
                &ButtonEvent {
                    serial,
                    time: event.timestamp as u32,
                    button: button_code,
                    state: button_state,
                },
            );

            debug!("Pointer button delivered: button={}, state={:?}", button_code, btn_state);
        }

        Ok(())
    }

    /// Deliver pointer axis (scroll) event
    pub fn deliver_pointer_axis(
        &mut self,
        horizontal: f64,
        vertical: f64,
        state: &mut CompositorState,
    ) -> Result<()> {
        trace!("Delivering pointer axis: h={}, v={}", horizontal, vertical);

        // Get seat
        let seat = match &mut state.seat {
            Some(seat) => seat,
            None => {
                trace!("No seat available");
                return Ok(());
            }
        };

        // Get pointer
        let pointer = match seat.get_pointer() {
            Some(ptr) => ptr,
            None => {
                trace!("No pointer on seat");
                return Ok(());
            }
        };

        // Create axis frame
        let mut frame = AxisFrame::new(0); // timestamp

        if horizontal != 0.0 {
            frame = frame.value(smithay::backend::input::Axis::Horizontal, horizontal);
        }

        if vertical != 0.0 {
            frame = frame.value(smithay::backend::input::Axis::Vertical, vertical);
        }

        // Deliver axis event
        pointer.axis(state, frame);

        debug!("Pointer axis delivered");

        Ok(())
    }

    /// Find surface under pointer
    fn surface_under_pointer(&self, state: &CompositorState) -> Option<(WlSurface, Point<i32, Logical>)> {
        // Get space
        let space = match &state.space {
            Some(space) => space,
            None => return None,
        };

        // Find window under pointer
        let pointer_pos = self.pointer_position.to_i32_round();

        // Iterate through windows in reverse Z-order (top to bottom)
        for window in space.elements().rev() {
            let window_geo = space.element_geometry(window)?;

            // Check if pointer is within window bounds
            if window_geo.contains(pointer_pos) {
                // Get window's surface
                let surface = window.toplevel().and_then(|t| Some(t.wl_surface().clone()))?;

                // Calculate position relative to window
                let relative_pos = Point::from((
                    pointer_pos.x - window_geo.loc.x,
                    pointer_pos.y - window_geo.loc.y,
                ));

                return Some((surface, relative_pos));
            }
        }

        None
    }

    /// Get current pointer position
    pub fn pointer_position(&self) -> Point<f64, Logical> {
        self.pointer_position
    }

    /// Set keyboard focus to surface
    pub fn set_keyboard_focus(
        &mut self,
        surface: Option<&WlSurface>,
        state: &mut CompositorState,
    ) -> Result<()> {
        debug!("Setting keyboard focus");

        // Get seat
        let seat = match &mut state.seat {
            Some(seat) => seat,
            None => return Ok(()),
        };

        // Get keyboard
        let keyboard = match seat.get_keyboard() {
            Some(kbd) => kbd,
            None => return Ok(()),
        };

        // Get serial
        let serial = state.next_serial();

        // Set focus
        keyboard.set_focus(state, surface.cloned(), serial);

        Ok(())
    }

    /// Set pointer focus to surface
    pub fn set_pointer_focus(
        &mut self,
        surface: Option<&WlSurface>,
        position: Point<i32, Logical>,
        state: &mut CompositorState,
    ) -> Result<()> {
        debug!("Setting pointer focus");

        // Get seat
        let seat = match &mut state.seat {
            Some(seat) => seat,
            None => return Ok(()),
        };

        // Get pointer
        let pointer = match seat.get_pointer() {
            Some(ptr) => ptr,
            None => return Ok(()),
        };

        // Get serial
        let serial = state.next_serial();

        // Set focus
        let under = surface.map(|s| (s.clone(), position));
        pointer.motion(
            state,
            under.as_ref().map(|(s, p)| (s.clone(), p.to_f64())),
            &MotionEvent {
                location: position.to_f64(),
                serial,
                time: 0,
            },
        );

        Ok(())
    }
}

impl Default for InputDelivery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::types::CompositorConfig;

    #[test]
    fn test_input_delivery_creation() {
        let delivery = InputDelivery::new();
        assert_eq!(delivery.pointer_position.x, 0.0);
        assert_eq!(delivery.pointer_position.y, 0.0);
    }

    #[test]
    fn test_pointer_position() {
        let delivery = InputDelivery::new();
        let pos = delivery.pointer_position();
        assert_eq!(pos.x, 0.0);
        assert_eq!(pos.y, 0.0);
    }
}
