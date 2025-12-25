//! Extended OpenH264 Encoder with Level Configuration
//!
//! This module wraps the standard `openh264::Encoder` to provide access
//! to advanced configuration options not exposed by the Rust crate, specifically
//! H.264 level configuration which is critical for RDP compliance.
//!
//! # Safety
//!
//! Uses unsafe FFI calls to OpenH264 C API via openh264-sys2.

use openh264::encoder::{Encoder, EncoderConfig};
use openh264::error::NativeErrorExt;  // For .ok() method on i32 error codes
use openh264_sys2::{SEncParamExt, ENCODER_OPTION_SVC_ENCODE_PARAM_EXT};
use std::ptr;
use tracing::{debug, warn};

use super::h264_level::H264Level;
use super::{EncoderError, EncoderResult};

/// Extended encoder with level configuration support
pub struct LevelAwareEncoder {
    encoder: Encoder,
    configured_level: H264Level,
    width: u16,
    height: u16,
}

impl LevelAwareEncoder {
    /// Create encoder with explicit level configuration
    ///
    /// This uses OpenH264's C API to set the H.264 level constraint.
    ///
    /// # Arguments
    ///
    /// * `config` - Base encoder configuration
    /// * `level` - H.264 level to configure
    /// * `width` - Initial frame width
    /// * `height` - Initial frame height
    ///
    /// # Safety
    ///
    /// Uses unsafe FFI calls to access OpenH264 C API.
    pub fn new(
        config: EncoderConfig,
        level: H264Level,
        width: u16,
        height: u16,
    ) -> EncoderResult<Self> {
        let encoder = Encoder::with_api_config(openh264::OpenH264API::from_source(), config)
            .map_err(|e| EncoderError::InitFailed(format!("OpenH264 init failed: {:?}", e)))?;

        let mut level_encoder = Self {
            encoder,
            configured_level: level,
            width,
            height,
        };

        // Configure level via C API
        level_encoder.set_level(level)?;

        debug!(
            "Created H.264 encoder: {}x{}, {}, bitrate={}kbps",
            width,
            height,
            level,
            config.target_bitrate / 1000
        );

        Ok(level_encoder)
    }

    /// Set H.264 level via OpenH264 C API
    ///
    /// # Safety
    ///
    /// Accesses the raw OpenH264 encoder pointer through the Rust wrapper's
    /// `raw_api()` method. This is safe because:
    /// 1. The encoder is initialized and valid
    /// 2. We're only reading/writing the params structure
    /// 3. SetOption is thread-safe according to OpenH264 docs
    fn set_level(&mut self, level: H264Level) -> EncoderResult<()> {
        unsafe {
            // Access raw encoder API
            let raw_api = self.encoder.raw_api();

            // Get default parameters
            let mut params = SEncParamExt::default();
            raw_api
                .get_default_params(&mut params)
                .ok()
                .map_err(|e| EncoderError::InitFailed(format!("GetDefaultParams failed: {:?}", e)))?;

            // Configure level in the spatial layer
            // For single-layer encoding, we use sSpatialLayers[0]
            params.sSpatialLayers[0].uiLevelIdc = level.to_openh264_level();

            // Also set dimensions to help auto-calculation
            params.sSpatialLayers[0].iVideoWidth = self.width as i32;
            params.sSpatialLayers[0].iVideoHeight = self.height as i32;

            // Apply the parameters
            raw_api
                .set_option(
                    ENCODER_OPTION_SVC_ENCODE_PARAM_EXT,
                    ptr::addr_of_mut!(params).cast(),
                )
                .ok()
                .map_err(|e| EncoderError::InitFailed(format!("SetOption(PARAM_EXT) failed: {:?}", e)))?;

            debug!(
                "Configured OpenH264 level: {} ({}x{} @ level={})",
                level,
                self.width,
                self.height,
                level.to_openh264_level()
            );
        }

        Ok(())
    }

    /// Force next frame to be keyframe
    pub fn force_keyframe(&mut self) {
        self.encoder.force_intra_frame();
    }

    /// Get the configured level
    pub fn level(&self) -> H264Level {
        self.configured_level
    }

    /// Update level configuration (useful if resolution changes)
    pub fn update_level(&mut self, level: H264Level) -> EncoderResult<()> {
        if level != self.configured_level {
            warn!(
                "Changing H.264 level from {} to {}",
                self.configured_level, level
            );
            self.set_level(level)?;
            self.configured_level = level;

            // Force keyframe after level change
            self.force_keyframe();
        }
        Ok(())
    }

    /// Access underlying encoder for encoding operations
    pub fn encoder_mut(&mut self) -> &mut Encoder {
        &mut self.encoder
    }

    /// Update dimensions (useful if resolution changes)
    pub fn set_dimensions(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }
}

// Tests require H.264 encoder feature and OpenH264 binary
#[cfg(all(test, feature = "h264"))]
mod tests {
    use super::*;
    use openh264::encoder::UsageType;

    // Note: These tests require OpenH264 to be available at runtime
    // They validate the level configuration API, not encoding itself

    #[test]
    fn test_level_constraints() {
        // 1280×720 @ 30fps fits in Level 3.1
        let constraints = super::super::h264_level::LevelConstraints::new(1280, 720);
        assert!(constraints.validate(30.0, H264Level::L3_1).is_ok());

        // 1280×800 @ 30fps requires Level 4.0
        let constraints = super::super::h264_level::LevelConstraints::new(1280, 800);
        assert!(constraints.validate(30.0, H264Level::L3_2).is_err());
        assert!(constraints.validate(30.0, H264Level::L4_0).is_ok());
    }

    #[test]
    fn test_level_recommendation() {
        use super::super::h264_level::LevelConstraints;

        // 720p @ 30fps → Level 3.1
        assert_eq!(
            LevelConstraints::new(1280, 720).recommend_level(30.0),
            H264Level::L3_1
        );

        // 1280×800 @ 30fps → Level 4.0
        assert_eq!(
            LevelConstraints::new(1280, 800).recommend_level(30.0),
            H264Level::L4_0
        );

        // 1080p @ 30fps → Level 4.0
        assert_eq!(
            LevelConstraints::new(1920, 1080).recommend_level(30.0),
            H264Level::L4_0
        );

        // 4K @ 30fps → Level 5.1
        assert_eq!(
            LevelConstraints::new(3840, 2160).recommend_level(30.0),
            H264Level::L5_1
        );
    }
}
