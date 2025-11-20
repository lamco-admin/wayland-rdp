//! RDP-Compositor integration module
//!
//! Provides utilities for integrating the compositor with the RDP server.

use super::types::*;
use super::state::CompositorState;
use super::software_renderer::SoftwareRenderer;
use super::input::{InputManager, KeyboardEvent, PointerEvent};
use anyhow::Result;
use std::sync::Arc;
use parking_lot::Mutex;
use tracing::{debug, info, trace};

/// Complete compositor-RDP integration
pub struct CompositorRdpIntegration {
    /// Compositor state
    state: Arc<Mutex<CompositorState>>,

    /// Software renderer
    renderer: Arc<Mutex<SoftwareRenderer>>,

    /// Input manager
    input_manager: Arc<Mutex<InputManager>>,

    /// Frame sequence counter
    frame_counter: Arc<std::sync::atomic::AtomicU64>,
}

impl CompositorRdpIntegration {
    /// Create new integration
    pub fn new(config: CompositorConfig) -> Result<Self> {
        info!("Creating compositor-RDP integration");

        let state = Arc::new(Mutex::new(CompositorState::new(config.clone())?));

        let renderer = Arc::new(Mutex::new(SoftwareRenderer::new(
            config.width,
            config.height,
            config.pixel_format,
        )));

        let input_manager = Arc::new(Mutex::new(InputManager::new(
            config.width,
            config.height,
        )));

        Ok(Self {
            state,
            renderer,
            input_manager,
            frame_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        })
    }

    /// Render a complete frame
    pub fn render_frame(&self) -> Result<RenderedFrame> {
        trace!("Rendering frame");

        let mut state = self.state.lock();
        let mut renderer = self.renderer.lock();

        // Clear to background
        renderer.clear();

        // Render all windows in Z-order
        for window_id in &state.z_order {
            if let Some(window) = state.windows.get(window_id) {
                if window.state != WindowState::Minimized {
                    // Get surface
                    if let Some(surface_id) = window.surface_id {
                        if let Some(surface) = state.surfaces.get(&surface_id) {
                            renderer.render_surface(
                                surface,
                                window.geometry.x,
                                window.geometry.y,
                            )?;
                        }
                    }
                }
            }
        }

        // Render cursor
        renderer.render_cursor(&state.pointer.cursor)?;

        // Update damage
        renderer.update_framebuffer_damage();

        // Get framebuffer
        let framebuffer = renderer.framebuffer().clone();

        // Increment frame counter
        let sequence = self.frame_counter.fetch_add(
            1,
            std::sync::atomic::Ordering::SeqCst,
        );

        Ok(RenderedFrame {
            framebuffer,
            sequence,
        })
    }

    /// Handle RDP keyboard input
    pub fn handle_rdp_keyboard(&self, scancode: u32, pressed: bool) -> Result<()> {
        debug!("RDP keyboard: scancode={}, pressed={}", scancode, pressed);

        let mut input_manager = self.input_manager.lock();

        if let Some(event) = input_manager.translate_keyboard(scancode, pressed) {
            let mut state = self.state.lock();
            state.inject_keyboard(event)?;
        }

        Ok(())
    }

    /// Handle RDP pointer motion
    pub fn handle_rdp_pointer_motion(&self, x: u16, y: u16) -> Result<()> {
        trace!("RDP pointer motion: ({}, {})", x, y);

        let mut input_manager = self.input_manager.lock();
        let event = input_manager.translate_pointer(x, y);

        let mut state = self.state.lock();
        state.inject_pointer(event)?;

        Ok(())
    }

    /// Handle RDP pointer button
    pub fn handle_rdp_pointer_button(&self, button: u32, pressed: bool) -> Result<()> {
        debug!("RDP pointer button: button={}, pressed={}", button, pressed);

        let mut input_manager = self.input_manager.lock();

        if let Some(event) = input_manager.translate_button(button, pressed) {
            let mut state = self.state.lock();
            state.inject_pointer(event)?;
        }

        Ok(())
    }

    /// Get clipboard data
    pub fn get_clipboard(&self) -> Result<Vec<u8>> {
        let state = self.state.lock();
        state.get_clipboard()
    }

    /// Set clipboard data
    pub fn set_clipboard(&self, data: Vec<u8>) -> Result<()> {
        let mut state = self.state.lock();
        state.set_clipboard(data)
    }

    /// Add a window (for testing/demo)
    pub fn add_test_window(&self, x: i32, y: i32, width: u32, height: u32) -> WindowId {
        let mut state = self.state.lock();

        let window = super::state::Window::new(Rectangle::new(x, y, width, height));
        state.add_window(window)
    }

    /// Get statistics
    pub fn get_stats(&self) -> IntegrationStats {
        let state = self.state.lock();
        let frame_count = self.frame_counter.load(std::sync::atomic::Ordering::SeqCst);

        IntegrationStats {
            window_count: state.windows.len(),
            surface_count: state.surfaces.len(),
            frame_count,
            focused_window: state.focused_window,
        }
    }
}

/// Rendered frame ready for RDP encoding
#[derive(Debug, Clone)]
pub struct RenderedFrame {
    /// Framebuffer data
    pub framebuffer: FrameBuffer,

    /// Frame sequence number
    pub sequence: u64,
}

impl RenderedFrame {
    /// Get raw pixel data
    pub fn pixel_data(&self) -> &[u8] {
        &self.framebuffer.data
    }

    /// Get dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.framebuffer.width, self.framebuffer.height)
    }

    /// Get pixel format
    pub fn pixel_format(&self) -> PixelFormat {
        self.framebuffer.format
    }

    /// Get damage regions
    pub fn damage_regions(&self) -> &[Rectangle] {
        &self.framebuffer.damage.regions
    }

    /// Check if full frame update needed
    pub fn is_full_update(&self) -> bool {
        self.framebuffer.damage.full_damage
    }
}

/// Integration statistics
#[derive(Debug, Clone)]
pub struct IntegrationStats {
    pub window_count: usize,
    pub surface_count: usize,
    pub frame_count: u64,
    pub focused_window: Option<WindowId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integration_creation() {
        let config = CompositorConfig::default();
        let integration = CompositorRdpIntegration::new(config);
        assert!(integration.is_ok());
    }

    #[test]
    fn test_render_frame() {
        let config = CompositorConfig::default();
        let integration = CompositorRdpIntegration::new(config).unwrap();

        let frame = integration.render_frame();
        assert!(frame.is_ok());

        let frame = frame.unwrap();
        assert_eq!(frame.dimensions(), (1920, 1080));
    }

    #[test]
    fn test_input_handling() {
        let config = CompositorConfig::default();
        let integration = CompositorRdpIntegration::new(config).unwrap();

        // Test keyboard
        let result = integration.handle_rdp_keyboard(0x10, true); // Q key
        assert!(result.is_ok());

        // Test pointer
        let result = integration.handle_rdp_pointer_motion(100, 200);
        assert!(result.is_ok());

        // Test button
        let result = integration.handle_rdp_pointer_button(1, true); // Left click
        assert!(result.is_ok());
    }

    #[test]
    fn test_clipboard() {
        let config = CompositorConfig::default();
        let integration = CompositorRdpIntegration::new(config).unwrap();

        // Set clipboard
        let data = b"Hello, RDP!".to_vec();
        let result = integration.set_clipboard(data.clone());
        assert!(result.is_ok());

        // Get clipboard
        let retrieved = integration.get_clipboard().unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_window_management() {
        let config = CompositorConfig::default();
        let integration = CompositorRdpIntegration::new(config).unwrap();

        // Add test window
        let window_id = integration.add_test_window(100, 100, 640, 480);

        // Check stats
        let stats = integration.get_stats();
        assert_eq!(stats.window_count, 1);

        // Render with window
        let frame = integration.render_frame().unwrap();
        assert!(frame.damage_regions().len() > 0);
    }
}
