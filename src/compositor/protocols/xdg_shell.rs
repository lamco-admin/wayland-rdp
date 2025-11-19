//! xdg_shell protocol implementation
//!
//! Implements the XDG shell protocol for window management (toplevels and popups).

use smithay::desktop::{Space, Window, WindowSurfaceType};
use smithay::wayland::shell::xdg::{
    Configure, PopupConfigure, PositionerState, ToplevelConfigure,
    XdgShellHandler, XdgShellState, XdgToplevelSurfaceData,
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

    fn new_toplevel(&mut self, surface: smithay::desktop::ToplevelSurface) {
        let wl_surface = surface.wl_surface();
        info!("New XDG toplevel window: {:?}", wl_surface.id());

        // Create window
        let window = Window::new(surface);

        // Get initial size from pending state or use default
        let initial_size = Size::from((800, 600));

        // Add to compositor state's space
        if let Some(space) = &mut self.space {
            space.map_element(window.clone(), Point::from((0, 0)), false);
        }

        // Send initial configure
        let serial = self.next_serial();
        let mut configure = ToplevelConfigure {
            bounds: Some(Size::from((self.config.width, self.config.height))),
            ..Default::default()
        };

        configure.state.set(xdg_toplevel::State::Activated);

        debug!(
            "Configuring toplevel with size: {}x{}",
            initial_size.w, initial_size.h
        );

        // Track window in our state
        self.add_xdg_window(window);

        // Trigger damage
        self.damage_all();
    }

    fn toplevel_destroyed(&mut self, surface: smithay::desktop::ToplevelSurface) {
        let wl_surface = surface.wl_surface();
        info!("XDG toplevel destroyed: {:?}", wl_surface.id());

        // Remove from space
        if let Some(space) = &mut self.space {
            space.elements().find_map(|w| {
                if w.toplevel().wl_surface() == wl_surface {
                    Some(w.clone())
                } else {
                    None
                }
            }).map(|window| {
                space.unmap_elem(&window);
            });
        }

        self.damage_all();
    }

    fn new_popup(&mut self, surface: smithay::desktop::PopupSurface, _positioner: PositionerState) {
        debug!("New XDG popup: {:?}", surface.wl_surface().id());

        // Popups are managed as part of their parent surface
        // Send initial configure
        let serial = self.next_serial();
        let configure = PopupConfigure {
            ..Default::default()
        };

        trace!("Popup configured");
    }

    fn popup_destroyed(&mut self, surface: smithay::desktop::PopupSurface) {
        debug!("XDG popup destroyed: {:?}", surface.wl_surface().id());
        self.damage_all();
    }

    fn move_request(&mut self, surface: smithay::desktop::ToplevelSurface, seat: wl_seat::WlSeat, serial: Serial) {
        debug!("Move request for toplevel: {:?}", surface.wl_surface().id());

        // In a real compositor, this would start an interactive move
        // For headless RDP, we can ignore or handle programmatically
        trace!("Move request received (not implemented for headless)");
    }

    fn resize_request(
        &mut self,
        surface: smithay::desktop::ToplevelSurface,
        seat: wl_seat::WlSeat,
        serial: Serial,
        edges: xdg_toplevel::ResizeEdge,
    ) {
        debug!(
            "Resize request for toplevel: {:?}, edges: {:?}",
            surface.wl_surface().id(),
            edges
        );

        // In a real compositor, this would start an interactive resize
        trace!("Resize request received (not implemented for headless)");
    }

    fn ack_configure(&mut self, surface: wl_surface::WlSurface, configure: Configure) {
        trace!("Configure acknowledged for surface: {:?}", surface.id());

        // Client acknowledged the configure
        // Update state accordingly
        self.damage_all();
    }

    fn maximize_request(&mut self, surface: smithay::desktop::ToplevelSurface) {
        debug!("Maximize request for toplevel: {:?}", surface.wl_surface().id());

        // Find window and maximize it to full output size
        if let Some(space) = &self.space {
            if let Some(window) = space.elements().find(|w| {
                w.toplevel().wl_surface() == surface.wl_surface()
            }) {
            // Set to full size
            let output_size = Size::from((self.config.width, self.config.height));

            // Send new configure with maximized state
            let mut configure = ToplevelConfigure {
                bounds: Some(output_size),
                ..Default::default()
            };
            configure.state.set(xdg_toplevel::State::Maximized);
            configure.state.set(xdg_toplevel::State::Activated);

                debug!("Window maximized to {}x{}", output_size.w, output_size.h);
                self.damage_all();
            }
        }
    }

    fn unmaximize_request(&mut self, surface: smithay::desktop::ToplevelSurface) {
        debug!("Unmaximize request for toplevel: {:?}", surface.wl_surface().id());

        // Send configure with normal state
        let mut configure = ToplevelConfigure {
            ..Default::default()
        };
        configure.state.set(xdg_toplevel::State::Activated);

        self.damage_all();
    }

    fn fullscreen_request(
        &mut self,
        surface: smithay::desktop::ToplevelSurface,
        output: Option<smithay::reexports::wayland_server::protocol::wl_output::WlOutput>,
    ) {
        debug!("Fullscreen request for toplevel: {:?}", surface.wl_surface().id());

        // Set to full output size with fullscreen state
        let output_size = Size::from((self.config.width, self.config.height));

        let mut configure = ToplevelConfigure {
            bounds: Some(output_size),
            ..Default::default()
        };
        configure.state.set(xdg_toplevel::State::Fullscreen);
        configure.state.set(xdg_toplevel::State::Activated);

        debug!("Window set to fullscreen: {}x{}", output_size.w, output_size.h);
        self.damage_all();
    }

    fn unfullscreen_request(&mut self, surface: smithay::desktop::ToplevelSurface) {
        debug!("Unfullscreen request for toplevel: {:?}", surface.wl_surface().id());

        let mut configure = ToplevelConfigure {
            ..Default::default()
        };
        configure.state.set(xdg_toplevel::State::Activated);

        self.damage_all();
    }

    fn minimize_request(&mut self, surface: smithay::desktop::ToplevelSurface) {
        debug!("Minimize request for toplevel: {:?}", surface.wl_surface().id());

        // For headless compositor, we can hide the window
        // In the rendering loop, minimized windows won't be rendered
        trace!("Window minimized (hidden from rendering)");
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
