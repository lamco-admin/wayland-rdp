//! Compositor integration tests
//!
//! Tests the complete compositor stack including Wayland protocols.

#[cfg(feature = "headless-compositor")]
mod compositor_tests {
    use wrd_server::compositor::{
        CompositorConfig, CompositorState, CompositorRdpIntegration,
        SoftwareRenderer,
    };
    use wrd_server::compositor::types::{PixelFormat, Rectangle};

    #[test]
    fn test_compositor_state_creation() {
        let config = CompositorConfig::default();
        let state = CompositorState::new(config);
        assert!(state.is_ok());
    }

    #[test]
    fn test_software_renderer() {
        let mut renderer = SoftwareRenderer::new(800, 600, PixelFormat::BGRA8888);

        // Test clear
        renderer.clear();

        // Test damage tracking
        renderer.damage_region(Rectangle::new(0, 0, 100, 100));
        let damage = renderer.take_damage();
        assert!(damage.regions.len() > 0);
    }

    #[test]
    fn test_rdp_integration() {
        let config = CompositorConfig::default();
        let integration = CompositorRdpIntegration::new(config);
        assert!(integration.is_ok());

        let integration = integration.unwrap();

        // Test frame rendering
        let frame = integration.render_frame();
        assert!(frame.is_ok());

        let frame = frame.unwrap();
        assert_eq!(frame.dimensions(), (1920, 1080));
    }

    #[test]
    fn test_input_injection() {
        let config = CompositorConfig::default();
        let integration = CompositorRdpIntegration::new(config).unwrap();

        // Test keyboard input
        let result = integration.handle_rdp_keyboard(0x10, true); // Q key
        assert!(result.is_ok());

        // Test pointer motion
        let result = integration.handle_rdp_pointer_motion(100, 200);
        assert!(result.is_ok());

        // Test pointer button
        let result = integration.handle_rdp_pointer_button(1, true); // Left click
        assert!(result.is_ok());
    }

    #[test]
    fn test_clipboard_sync() {
        let config = CompositorConfig::default();
        let integration = CompositorRdpIntegration::new(config).unwrap();

        // Set clipboard
        let test_data = b"Hello from RDP!".to_vec();
        let result = integration.set_clipboard(test_data.clone());
        assert!(result.is_ok());

        // Get clipboard
        let retrieved = integration.get_clipboard().unwrap();
        assert_eq!(retrieved, test_data);
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

        // Render frame with window
        let frame = integration.render_frame().unwrap();
        assert!(frame.damage_regions().len() > 0);
    }
}
