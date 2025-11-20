//! Buffer management
//!
//! Connects wl_shm buffers from Wayland clients to the software renderer.

use super::types::{PixelFormat, Rectangle};
use super::state::{CompositorState, Surface, SurfaceBuffer};
use super::software_renderer::SoftwareRenderer;
use smithay::backend::renderer::utils::RendererSurfaceState;
use smithay::wayland::compositor::{with_surface_tree_downward, TraversalAction};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use anyhow::{Context, Result};
use tracing::{debug, trace, warn};

/// Buffer manager for connecting wl_shm to renderer
pub struct BufferManager {
    /// Default pixel format
    default_format: PixelFormat,
}

impl BufferManager {
    /// Create new buffer manager
    pub fn new() -> Self {
        Self {
            default_format: PixelFormat::BGRA8888,
        }
    }

    /// Import buffer from Wayland surface
    pub fn import_buffer(
        &self,
        surface: &WlSurface,
        state: &CompositorState,
    ) -> Result<Option<SurfaceBuffer>> {
        trace!("Importing buffer from surface");

        // TODO: In Smithay 0.7, buffer access APIs have changed significantly.
        // For now, return a stub to allow compilation. This needs to be
        // reimplemented using the new buffer API or using a proper renderer.

        // Stub implementation - returns a small empty buffer
        debug!("Buffer import stubbed - needs Smithay 0.7 buffer API reimplementation");

        Ok(Some(SurfaceBuffer {
            width: 64,
            height: 64,
            data: vec![0; 64 * 64 * 4],
            format: self.default_format,
        }))
    }

    /// Render surface tree to renderer
    pub fn render_surface_tree(
        &self,
        surface: &WlSurface,
        renderer: &mut SoftwareRenderer,
        position: (i32, i32),
        state: &CompositorState,
    ) -> Result<()> {
        trace!("Rendering surface tree at {:?}", position);

        let (x, y) = position;

        // Traverse surface tree and render each surface
        with_surface_tree_downward(
            surface,
            (),
            |_, _, _| TraversalAction::DoChildren(()),
            |surface, states, _| {
                // Get surface buffer
                if let Ok(Some(buffer)) = self.import_buffer(surface, state) {
                    // Get surface position from subsurface state if applicable
                    let surface_pos = smithay::wayland::compositor::with_states(surface, |states| {
                        states.cached_state.current::<smithay::desktop::space::SpaceRenderElements>()
                            .map(|elem| elem.location)
                            .unwrap_or((0, 0))
                    });

                    // Render the buffer
                    let render_x = x + surface_pos.0;
                    let render_y = y + surface_pos.1;

                    // Create temporary surface for rendering
                    let temp_surface = Surface {
                        id: super::types::SurfaceId::new(),
                        buffer: Some(buffer),
                        damage: vec![],
                        scale: 1,
                    };

                    if let Err(e) = renderer.render_surface(&temp_surface, render_x, render_y) {
                        warn!("Failed to render surface: {}", e);
                    }
                }
            },
            |_, _, _| true,
        );

        Ok(())
    }

    /// Convert Smithay SHM format to our PixelFormat
    fn convert_shm_format(&self, format: smithay::reexports::wayland_server::protocol::wl_shm::Format) -> PixelFormat {
        use smithay::reexports::wayland_server::protocol::wl_shm::Format;

        match format {
            Format::Argb8888 => PixelFormat::BGRA8888, // Note: ARGB in memory is BGRA
            Format::Xrgb8888 => PixelFormat::BGRX8888,
            Format::Abgr8888 => PixelFormat::RGBA8888,
            Format::Xbgr8888 => PixelFormat::RGBX8888,
            _ => {
                warn!("Unsupported SHM format: {:?}, using default", format);
                self.default_format
            }
        }
    }

    /// Get surface damage regions
    pub fn get_surface_damage(&self, surface: &WlSurface) -> Vec<Rectangle> {
        smithay::wayland::compositor::with_states(surface, |states| {
            states.cached_state.current::<RendererSurfaceState>()
                .damage
                .iter()
                .map(|rect| Rectangle::new(
                    rect.loc.x,
                    rect.loc.y,
                    rect.size.w as u32,
                    rect.size.h as u32,
                ))
                .collect()
        })
    }
}

impl Default for BufferManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_manager_creation() {
        let manager = BufferManager::new();
        assert_eq!(manager.default_format, PixelFormat::BGRA8888);
    }

    #[test]
    fn test_format_conversion() {
        let manager = BufferManager::new();

        use smithay::reexports::wayland_server::protocol::wl_shm::Format;

        assert_eq!(
            manager.convert_shm_format(Format::Argb8888),
            PixelFormat::BGRA8888
        );
        assert_eq!(
            manager.convert_shm_format(Format::Xrgb8888),
            PixelFormat::BGRX8888
        );
        assert_eq!(
            manager.convert_shm_format(Format::Abgr8888),
            PixelFormat::RGBA8888
        );
        assert_eq!(
            manager.convert_shm_format(Format::Xbgr8888),
            PixelFormat::RGBX8888
        );
    }
}
