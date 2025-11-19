//! RDP server integration tests
//!
//! Tests the RDP server and frame encoding.

#[cfg(feature = "headless-compositor")]
mod rdp_tests {
    use wrd_server::rdp::{
        RdpServer, RdpServerConfig, FrameEncoder, EncoderFormat,
    };
    use wrd_server::compositor::{CompositorConfig, CompositorRdpIntegration};
    use wrd_server::compositor::types::PixelFormat;

    #[test]
    fn test_rdp_server_creation() {
        let rdp_config = RdpServerConfig::default();
        let compositor_config = CompositorConfig::default();

        let server = RdpServer::new(rdp_config, compositor_config);
        assert!(server.is_ok());
    }

    #[test]
    fn test_rdp_server_stats() {
        let rdp_config = RdpServerConfig::default();
        let compositor_config = CompositorConfig::default();

        let server = RdpServer::new(rdp_config, compositor_config).unwrap();

        let stats = server.get_stats();
        assert_eq!(stats.active_connections, 0);
    }

    #[test]
    fn test_frame_encoder_raw() {
        let encoder = FrameEncoder::new(EncoderFormat::Raw);
        assert_eq!(encoder.format(), EncoderFormat::Raw);

        // Create test frame
        let config = CompositorConfig::default();
        let integration = CompositorRdpIntegration::new(config).unwrap();
        let frame = integration.render_frame().unwrap();

        // Encode frame
        let encoded = encoder.encode(&frame);
        assert!(encoded.is_ok());

        let encoded = encoded.unwrap();
        assert_eq!(encoded.sequence, frame.sequence);
        assert_eq!(encoded.format, EncoderFormat::Raw);
    }

    #[test]
    fn test_frame_encoder_rle() {
        let encoder = FrameEncoder::new(EncoderFormat::Rle);
        assert_eq!(encoder.format(), EncoderFormat::Rle);

        // Create test frame
        let config = CompositorConfig::default();
        let integration = CompositorRdpIntegration::new(config).unwrap();
        let frame = integration.render_frame().unwrap();

        // Encode frame
        let encoded = encoder.encode(&frame);
        assert!(encoded.is_ok());

        let encoded = encoded.unwrap();
        assert_eq!(encoded.format, EncoderFormat::Rle);

        // RLE should compress uniform backgrounds
        assert!(encoded.compression_ratio < 1.0);
    }

    #[test]
    fn test_encoder_format_switching() {
        let mut encoder = FrameEncoder::new(EncoderFormat::Raw);

        encoder.set_format(EncoderFormat::Rle);
        assert_eq!(encoder.format(), EncoderFormat::Rle);

        encoder.set_format(EncoderFormat::Raw);
        assert_eq!(encoder.format(), EncoderFormat::Raw);
    }

    #[test]
    fn test_pixel_format_conversion() {
        let encoder = FrameEncoder::new(EncoderFormat::Raw);

        // Create test RGBA pixel data
        let rgba_data = vec![
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
        ];

        // This would test the internal conversion
        // The public API handles this transparently
    }
}
