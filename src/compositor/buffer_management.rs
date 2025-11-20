//! Buffer management
//!
//! Connects wl_shm buffers from Wayland clients to the software renderer.

use super::types::{PixelFormat, Rectangle, Surface, SurfaceBuffer};
use super::state::CompositorState;
use super::software_renderer::SoftwareRenderer;
use smithay::backend::renderer::utils::RendererSurfaceState;
use smithay::wayland::buffer::BufferData;
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
        trace!("Importing buffer from surface: {:?}", surface.id());

        // Get buffer from surface
        let buffer_data = match smithay::wayland::compositor::with_states(surface, |states| {
            states.cached_state.current::<RendererSurfaceState>().buffer.clone()
        }) {
            Some(buffer) => buffer,
            None => {
                trace!("No buffer attached to surface");
                return Ok(None);
            }
        };

        // Get buffer data
        let shm_buffer = match buffer_data {
            Some(smithay::backend::renderer::utils::Buffer::Shm(shm)) => shm,
            Some(_) => {
                warn!("Non-SHM buffer not supported in software renderer");
                return Ok(None);
            }
            None => {
                return Ok(None);
            }
        };

        // Access SHM buffer data
        let (width, height, format, data) = match smithay::wayland::shm::with_buffer_contents(
            &shm_buffer,
            |data, len, buffer_data| {
                let width = buffer_data.width;
                let height = buffer_data.height;
                let stride = buffer_data.stride as usize;
                let format = self.convert_shm_format(buffer_data.format);

                // Copy buffer data
                let mut buffer = Vec::with_capacity(len);
                buffer.extend_from_slice(&data[..len]);

                (width, height, format, buffer)
            }
        ) {
            Ok(result) => result,
            Err(e) => {
                warn!("Failed to access SHM buffer: {}", e);
                return Ok(None);
            }
        };

        debug!(
            "Imported buffer: {}x{} format={:?} size={}",
            width, height, format, data.len()
        );

        Ok(Some(SurfaceBuffer {
            width,
            height,
            data,
            format,
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
