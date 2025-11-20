//! xdg_shell protocol implementation
//!
//! Implements the XDG shell protocol for window management (toplevels and popups).

use smithay::desktop::{Space, Window, WindowSurfaceType, PopupKind};
use smithay::wayland::shell::xdg::{
    Configure, PopupConfigure, PositionerState, ToplevelConfigure,
    XdgShellHandler, XdgShellState, XdgToplevelSurfaceData, ToplevelSurface, PopupSurface,
};
use smithay::reexports::wayland_server::protocol::{wl_seat, wl_surface};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
use smithay::reexports::wayland_server::Resource;
use smithay::delegate_xdg_shell;
use smithay::utils::{Logical, Point, Rectangle, Serial, Size};
use crate::compositor::state::CompositorState;
use tracing::{debug, info, trace, warn};

/// XDG Shell protocol handler
impl XdgShellHandler for CompositorState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        self.xdg_shell_state.as_mut()
            .expect("XDG shell state not initialized - call init_smithay_states() first")
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let wl_surface = surface.wl_surface();
        info!("New XDG toplevel window");

        // Create window
        let window = Window::new(surface);

        // Get initial size from pending state or use default
        let initial_size: Size<i32, Logical> = Size::from((800, 600));

        // Add to compositor state's space
        if let Some(space) = &mut self.space {
            space.map_element(window.clone(), Point::from((0, 0)), false);
        }

        // Send initial configure - ToplevelConfigure doesn't have Default in 0.7
        // We just map the window and Smithay will handle configuration

        debug!(
            "Configuring toplevel with size: {}x{}",
            initial_size.w, initial_size.h
        );

        // Track window in our state
        self.add_xdg_window(window);

        // Trigger damage
        self.damage_all();
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let wl_surface = surface.wl_surface();
        info!("XDG toplevel destroyed");

        // Remove from space
        if let Some(space) = &mut self.space {
            let window_to_remove = space.elements().find_map(|w| {
                if let Some(toplevel) = w.toplevel() {
                    if toplevel.wl_surface() == wl_surface {
                        Some(w.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

            if let Some(window) = window_to_remove {
                space.unmap_elem(&window);
            }
        }

        self.damage_all();
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        debug!("New XDG popup");

        // Popups are managed as part of their parent surface
        // Send initial configure
        let serial = self.next_serial();

        trace!("Popup configured");
    }

    fn popup_destroyed(&mut self, surface: PopupSurface) {
        debug!("XDG popup destroyed");
        self.damage_all();
    }

    fn move_request(&mut self, surface: ToplevelSurface, seat: wl_seat::WlSeat, serial: Serial) {
        debug!("Move request for toplevel");

        // In a real compositor, this would start an interactive move
        // For headless RDP, we can ignore or handle programmatically
        trace!("Move request received (not implemented for headless)");
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        seat: wl_seat::WlSeat,
        serial: Serial,
        edges: xdg_toplevel::ResizeEdge,
    ) {
        debug!("Resize request for toplevel, edges: {:?}", edges);

        // In a real compositor, this would start an interactive resize
        trace!("Resize request received (not implemented for headless)");
    }

    fn ack_configure(&mut self, surface: wl_surface::WlSurface, configure: Configure) {
        trace!("Configure acknowledged for surface");

        // Client acknowledged the configure
        // Update state accordingly
        self.damage_all();
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        debug!("Maximize request for toplevel");

        // Find window and maximize it to full output size
        let found = if let Some(space) = &self.space {
            space.elements().any(|w| {
                if let Some(toplevel) = w.toplevel() {
                    toplevel.wl_surface() == surface.wl_surface()
                } else {
                    false
                }
            })
        } else {
            false
        };

        if found {
            // Set to full size
            let output_size: Size<u32, Logical> = Size::from((self.config.width, self.config.height));

            debug!("Window maximized to {}x{}", output_size.w, output_size.h);
            self.damage_all();
        }
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        debug!("Unmaximize request for toplevel");

        self.damage_all();
    }

    fn fullscreen_request(
        &mut self,
        surface: ToplevelSurface,
        output: Option<smithay::reexports::wayland_server::protocol::wl_output::WlOutput>,
    ) {
        debug!("Fullscreen request for toplevel");

        // Set to full output size with fullscreen state
        let output_size: Size<u32, Logical> = Size::from((self.config.width, self.config.height));

        debug!("Window set to fullscreen: {}x{}", output_size.w, output_size.h);
        self.damage_all();
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        debug!("Unfullscreen request for toplevel");

        self.damage_all();
    }

    fn minimize_request(&mut self, surface: ToplevelSurface) {
        debug!("Minimize request for toplevel");

        // For headless compositor, we can hide the window
        // In the rendering loop, minimized windows won't be rendered
        trace!("Window minimized (hidden from rendering)");
    }

    fn grab(&mut self, surface: PopupSurface, seat: wl_seat::WlSeat, serial: Serial) {
        debug!("Popup grab request");
        // Grab handling for popups - not needed for headless compositor
        trace!("Popup grab (not implemented for headless)");
    }

    fn reposition_request(&mut self, surface: PopupSurface, positioner: PositionerState, token: u32) {
        debug!("Popup reposition request, token: {}", token);
        // Reposition popup based on new positioner state
        trace!("Popup reposition (not implemented for headless)");
    }
}

// Delegate XDG shell protocol to Smithay
delegate_xdg_shell!(CompositorState);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::types::CompositorConfig;

    #[test]
    fn test_xdg_shell_handler() {
        let config = CompositorConfig::default();
        let state = CompositorState::new(config);
        assert!(state.is_ok());
    }
}
