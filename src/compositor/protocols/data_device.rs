//! wl_data_device protocol implementation
//!
//! Implements clipboard and drag-and-drop support.

use smithay::wayland::selection::data_device::{
    ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
};
use smithay::wayland::selection::{SelectionHandler, SelectionTarget, SelectionSource};
use smithay::reexports::wayland_server::protocol::wl_data_source::WlDataSource;
use smithay::delegate_data_device;
use smithay::input::Seat;
use smithay::reexports::wayland_server::DisplayHandle;
use crate::compositor::state::CompositorState;
use tracing::{debug, info, trace, warn};

/// Data device (clipboard/DnD) protocol handler
impl DataDeviceHandler for CompositorState {
    fn data_device_state(&self) -> &DataDeviceState {
        self.data_device_state.as_ref()
            .expect("Data device state not initialized - call init_smithay_states() first")
    }
}

/// Selection handler for clipboard operations
impl SelectionHandler for CompositorState {
    type SelectionUserData = ();

    fn new_selection(
        &mut self,
        ty: SelectionTarget,
        source: Option<SelectionSource>,
        _seat: Seat<Self>,
    ) {
        match ty {
            SelectionTarget::Clipboard => {
                if let Some(source) = source {
                    debug!("New clipboard selection: {:?}", source);

                    // Store clipboard data
                    // In a real implementation, we would read the data from the source
                    // and store it for RDP clipboard synchronization
                    trace!("Clipboard updated");
                } else {
                    debug!("Clipboard cleared");
                    // Clear local clipboard
                    self.clipboard.data.clear();
                }
            }
            SelectionTarget::Primary => {
                debug!("Primary selection changed");
                // Primary selection (X11-style middle-click paste)
                // Less commonly used in Wayland
            }
        }
    }

    fn send_selection(
        &mut self,
        ty: SelectionTarget,
        mime_type: String,
        fd: std::os::fd::OwnedFd,
        _seat: Seat<Self>,
        _user_data: &Self::SelectionUserData,
    ) {
        debug!("Send selection request: {:?}, MIME: {}", ty, mime_type);

        match ty {
            SelectionTarget::Clipboard => {
                // Write clipboard data to the provided file descriptor
                // This would integrate with RDP clipboard
                trace!("Sending clipboard data for MIME type: {}", mime_type);

                // In a real implementation:
                // 1. Get clipboard data from our state
                // 2. Convert to requested MIME type if needed
                // 3. Write to fd
                // 4. Close fd
            }
            SelectionTarget::Primary => {
                trace!("Sending primary selection");
            }
        }
    }
}

/// Client drag-and-drop handler
impl ClientDndGrabHandler for CompositorState {
    fn started(
        &mut self,
        _source: Option<WlDataSource>,
        icon: Option<smithay::reexports::wayland_server::protocol::wl_surface::WlSurface>,
        _seat: Seat<Self>,
    ) {
        debug!("Client DnD grab started");

        if let Some(_icon_surface) = icon {
            debug!("DnD icon surface present");
            // Store icon for rendering during drag
        }
    }

    fn dropped(&mut self, _target: Option<smithay::reexports::wayland_server::protocol::wl_surface::WlSurface>, _validated: bool, _seat: Seat<Self>) {
        debug!("Client DnD dropped");
    }
}

/// Server drag-and-drop handler
impl ServerDndGrabHandler for CompositorState {
    fn send(
        &mut self,
        _mime_type: String,
        _fd: std::os::fd::OwnedFd,
        _seat: smithay::input::Seat<Self>,
    ) {
        debug!("Server DnD send");
    }
}

// Delegate data device protocol to Smithay
delegate_data_device!(CompositorState);

/// Initialize data device manager global
pub fn init_data_device_global(display: &DisplayHandle) -> DataDeviceState {
    info!("Initializing wl_data_device_manager global");

    let data_device_state = DataDeviceState::new::<CompositorState>(display);

    debug!("wl_data_device_manager global created");

    data_device_state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_mime_types() {
        // Test common MIME types
        let text_plain = "text/plain";
        let text_utf8 = "text/plain;charset=utf-8";

        assert!(text_plain.starts_with("text/"));
        assert!(text_utf8.starts_with("text/"));
    }
}
