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
        seat: Seat<Self>,
    ) {
        match ty {
            SelectionTarget::Clipboard => {
                if let Some(source) = source {
                    info!("🎯 Wayland clipboard changed - new selection detected!");

                    // Get MIME types offered by the source
                    let mime_types = source.mime_types();
                    debug!("Available MIME types: {:?}", mime_types);

                    // Notify RDP clients about new clipboard formats
                    if let Some(ref tx) = self.clipboard_event_tx {
                        info!("Announcing {} clipboard formats to RDP clients", mime_types.len());
                        if let Err(e) = tx.send(crate::clipboard::ClipboardEvent::PortalFormatsAvailable(mime_types)) {
                            warn!("Failed to send clipboard event to RDP: {}", e);
                        } else {
                            info!("✅ Clipboard formats announced to RDP - ready for paste");
                        }
                    } else {
                        warn!("No RDP clipboard channel configured - clipboard not synced");
                    }

                    // Emit compositor event (if needed by other listeners)
                    // self.emit_event is private, events sent via clipboard_event_tx instead
                } else {
                    debug!("Clipboard cleared");
                    self.clipboard.data.clear();
                }
            }
            SelectionTarget::Primary => {
                debug!("Primary selection changed (middle-click paste)");
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
        debug!("🎯 Wayland app requesting clipboard paste: {:?}, MIME: {}", ty, mime_type);

        match ty {
            SelectionTarget::Clipboard => {
                // Wayland app is pasting - provide clipboard data
                info!("Wayland app pasting clipboard (MIME: {})", mime_type);

                // Get clipboard data from our state
                let data = self.clipboard.data.clone();

                if !data.is_empty() {
                    // Write data to the file descriptor
                    use std::io::Write;
                    use std::os::fd::{FromRawFd, IntoRawFd};

                    let raw_fd = fd.into_raw_fd();
                    let mut file = unsafe { std::fs::File::from_raw_fd(raw_fd) };
                    if let Err(e) = file.write_all(&data) {
                        warn!("Failed to write clipboard data to fd: {}", e);
                    } else {
                        info!("✅ Wrote {} bytes to Wayland app clipboard", data.len());
                    }
                    // file closes automatically when dropped
                } else {
                    debug!("Clipboard empty - nothing to send");
                }
            }
            SelectionTarget::Primary => {
                debug!("Primary selection paste request");
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
