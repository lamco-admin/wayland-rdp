//! H.264/AVC420 Encoder for EGFX
//!
//! This module provides H.264 encoding using OpenH264 for use with the
//! EGFX AVC420 codec. OpenH264 handles color conversion internally.
//!
//! # MS-RDPEGFX Compliance
//!
//! MS-RDPEGFX requires length-prefixed NAL units (AVC format per ISO/IEC 14496-15),
//! not Annex B format. OpenH264 outputs Annex B by default, so this module
//! automatically converts the bitstream to length-prefixed format.
//!
//! # OpenH264 Licensing
//!
//! OpenH264 is licensed under BSD by Cisco. The openh264 Rust crate
//! bundles the source code and builds it automatically.
//!
//! # Performance Notes
//!
//! - For best performance, ensure `nasm` is in PATH (~3x speedup)
//! - Target 30 FPS for typical desktop sharing scenarios

#[cfg(feature = "h264")]
use openh264::encoder::{Encoder, EncoderConfig as OpenH264Config, FrameType, UsageType};
#[cfg(feature = "h264")]
use openh264::formats::{BgraSliceU8, YUVBuffer};

use thiserror::Error;
#[cfg(feature = "h264")]
use tracing::{debug, info, trace, warn};

/// Errors that can occur during H.264 encoding
#[derive(Debug, Error)]
pub enum EncoderError {
    #[error("Encoder initialization failed: {0}")]
    InitFailed(String),

    #[error("Encoding failed: {0}")]
    EncodeFailed(String),

    #[error("Invalid frame dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    #[error("H.264 feature not enabled")]
    FeatureDisabled,
}

/// Result type for encoder operations
pub type EncoderResult<T> = Result<T, EncoderError>;

/// Encoder configuration
#[derive(Debug, Clone)]
pub struct EncoderConfig {
    /// Target bitrate in kbps (default: 5000)
    pub bitrate_kbps: u32,

    /// Maximum frame rate (default: 30)
    pub max_fps: f32,

    /// Enable frame skipping for rate control (default: true)
    pub enable_skip_frame: bool,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            bitrate_kbps: 5000,
            max_fps: 30.0,
            enable_skip_frame: true,
        }
    }
}

impl EncoderConfig {
    /// Create config for high quality encoding
    pub fn high_quality() -> Self {
        Self {
            bitrate_kbps: 10000,
            max_fps: 30.0,
            enable_skip_frame: false,
        }
    }

    /// Create config for low bandwidth
    pub fn low_bandwidth() -> Self {
        Self {
            bitrate_kbps: 1000,
            max_fps: 15.0,
            enable_skip_frame: true,
        }
    }
}

/// Convert H.264 Annex B format to AVC length-prefixed format
///
/// MS-RDPEGFX requires length-prefixed NAL units (ISO/IEC 14496-15 AVC format),
/// but OpenH264 outputs Annex B format (start code prefixed).
///
/// # Format Conversion
///
/// - **Annex B input**: `[0x00 0x00 0x00 0x01][NAL]` or `[0x00 0x00 0x01][NAL]`
/// - **AVC output**: `[4-byte big-endian length][NAL]`
///
/// # Arguments
///
/// * `annex_b_data` - H.264 bitstream in Annex B format
///
/// # Returns
///
/// H.264 bitstream in AVC length-prefixed format
pub fn annex_b_to_avc(annex_b_data: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(annex_b_data.len());
    let mut i = 0;

    while i < annex_b_data.len() {
        // Find start code (0x00 0x00 0x01 or 0x00 0x00 0x00 0x01)
        let start_code_len = if i + 4 <= annex_b_data.len()
            && annex_b_data[i] == 0x00
            && annex_b_data[i + 1] == 0x00
            && annex_b_data[i + 2] == 0x00
            && annex_b_data[i + 3] == 0x01
        {
            4
        } else if i + 3 <= annex_b_data.len()
            && annex_b_data[i] == 0x00
            && annex_b_data[i + 1] == 0x00
            && annex_b_data[i + 2] == 0x01
        {
            3
        } else {
            // Not at a start code, skip byte
            i += 1;
            continue;
        };

        // Move past start code
        let nal_start = i + start_code_len;

        // Find next start code or end of data
        let mut nal_end = annex_b_data.len();
        let mut j = nal_start;
        while j + 3 <= annex_b_data.len() {
            if annex_b_data[j] == 0x00
                && annex_b_data[j + 1] == 0x00
                && (annex_b_data[j + 2] == 0x01
                    || (j + 3 < annex_b_data.len()
                        && annex_b_data[j + 2] == 0x00
                        && annex_b_data[j + 3] == 0x01))
            {
                nal_end = j;
                break;
            }
            j += 1;
        }

        // Extract NAL unit
        let nal_data = &annex_b_data[nal_start..nal_end];

        if !nal_data.is_empty() {
            // Write 4-byte big-endian length prefix
            let nal_len = nal_data.len() as u32;
            output.extend_from_slice(&nal_len.to_be_bytes());
            // Write NAL unit data
            output.extend_from_slice(nal_data);
        }

        i = nal_end;
    }

    output
}

/// Encoded H.264 frame
#[derive(Debug)]
pub struct H264Frame {
    /// Encoded NAL units (in AVC length-prefixed format)
    pub data: Vec<u8>,

    /// Whether this is a keyframe (IDR)
    pub is_keyframe: bool,

    /// Frame timestamp in milliseconds
    pub timestamp_ms: u64,

    /// Encoded frame size in bytes
    pub size: usize,
}

// Note: Avc420Region and create_avc420_bitmap_stream are provided by ironrdp-egfx
// See: ironrdp_egfx::pdu::Avc420Region and ironrdp_egfx::pdu::encode_avc420_bitmap_stream

/// Align dimension to multiple of 16 as required by MS-RDPEGFX
///
/// MS-RDPEGFX requires bitmap dimensions to be aligned to 16-pixel boundaries.
/// The encoded area is then cropped to the actual target region.
#[inline]
pub fn align_to_16(dimension: u32) -> u32 {
    (dimension + 15) & !15
}

/// H.264 encoder using OpenH264
///
/// # Feature Gate
///
/// Requires the `h264` feature to be enabled.
#[cfg(feature = "h264")]
pub struct Avc420Encoder {
    encoder: Encoder,
    config: EncoderConfig,
    frame_count: u64,
}

#[cfg(feature = "h264")]
impl Avc420Encoder {
    /// Create a new H.264 encoder
    ///
    /// # Arguments
    ///
    /// * `config` - Encoder configuration
    ///
    /// Note: OpenH264 auto-detects dimensions from the input YUV source.
    /// Dimensions are taken from the BGRA input during encoding.
    pub fn new(config: EncoderConfig) -> EncoderResult<Self> {
        // Configure OpenH264 encoder
        let encoder_config = OpenH264Config::new()
            .set_bitrate_bps(config.bitrate_kbps * 1000)
            .max_frame_rate(config.max_fps)
            .enable_skip_frame(config.enable_skip_frame)
            .usage_type(UsageType::ScreenContentRealTime);

        let encoder = Encoder::with_api_config(
            openh264::OpenH264API::from_source(),
            encoder_config,
        )
        .map_err(|e| EncoderError::InitFailed(format!("OpenH264 init failed: {:?}", e)))?;

        debug!(
            "Created H.264 encoder: bitrate={}kbps, max_fps={}",
            config.bitrate_kbps, config.max_fps
        );

        Ok(Self {
            encoder,
            config,
            frame_count: 0,
        })
    }

    /// Encode a BGRA frame to H.264
    ///
    /// # Arguments
    ///
    /// * `bgra_data` - Raw BGRA pixel data (4 bytes per pixel)
    /// * `width` - Frame width in pixels (must be multiple of 2)
    /// * `height` - Frame height in pixels (must be multiple of 2)
    /// * `timestamp_ms` - Frame timestamp in milliseconds
    ///
    /// # Returns
    ///
    /// Encoded H.264 frame, or None if the encoder produced no output
    /// (can happen with frame skipping)
    pub fn encode_bgra(
        &mut self,
        bgra_data: &[u8],
        width: u32,
        height: u32,
        timestamp_ms: u64,
    ) -> EncoderResult<Option<H264Frame>> {
        // Validate dimensions (must be multiples of 2 for YUV420)
        if width == 0 || height == 0 || width % 2 != 0 || height % 2 != 0 {
            return Err(EncoderError::InvalidDimensions { width, height });
        }

        let expected_size = (width * height * 4) as usize;
        if bgra_data.len() < expected_size {
            return Err(EncoderError::EncodeFailed(format!(
                "BGRA buffer too small: {} < {}",
                bgra_data.len(),
                expected_size
            )));
        }

        // Create BGRA source and convert to YUV420
        // OpenH264 handles the color conversion internally
        let bgra_source = BgraSliceU8::new(bgra_data, (width as usize, height as usize));
        let yuv = YUVBuffer::from_rgb_source(bgra_source);

        // Encode
        let bitstream = self.encoder.encode(&yuv).map_err(|e| {
            EncoderError::EncodeFailed(format!("OpenH264 encode failed: {:?}", e))
        })?;

        // Convert bitstream to Vec<u8> (Annex B format)
        let annex_b_data = bitstream.to_vec();
        if annex_b_data.is_empty() {
            return Ok(None);
        }

        // Check frame type for keyframe detection (before conversion)
        let is_keyframe = matches!(bitstream.frame_type(), FrameType::IDR | FrameType::I);

        // Convert from Annex B to AVC length-prefixed format for MS-RDPEGFX compliance
        let data = annex_b_to_avc(&annex_b_data);

        if data.is_empty() {
            warn!("NAL conversion produced empty output from {} byte Annex B stream", annex_b_data.len());
            return Ok(None);
        }

        self.frame_count += 1;

        trace!(
            "Encoded frame {}: {} bytes Annex B -> {} bytes AVC, keyframe={}",
            self.frame_count,
            annex_b_data.len(),
            data.len(),
            is_keyframe
        );

        Ok(Some(H264Frame {
            size: data.len(),
            data,
            is_keyframe,
            timestamp_ms,
        }))
    }

    /// Force next frame to be a keyframe (IDR)
    pub fn force_keyframe(&mut self) {
        self.encoder.force_intra_frame();
        debug!("Forced keyframe on next encode");
    }

    /// Get encoder statistics
    pub fn stats(&self) -> EncoderStats {
        EncoderStats {
            frames_encoded: self.frame_count,
            bitrate_kbps: self.config.bitrate_kbps,
        }
    }
}

/// Encoder statistics
#[derive(Debug, Clone)]
pub struct EncoderStats {
    /// Total frames encoded
    pub frames_encoded: u64,
    /// Configured bitrate in kbps
    pub bitrate_kbps: u32,
}

// Stub implementation when h264 feature is disabled
#[cfg(not(feature = "h264"))]
pub struct Avc420Encoder;

#[cfg(not(feature = "h264"))]
impl Avc420Encoder {
    pub fn new(_config: EncoderConfig) -> EncoderResult<Self> {
        Err(EncoderError::FeatureDisabled)
    }

    pub fn encode_bgra(
        &mut self,
        _bgra_data: &[u8],
        _width: u32,
        _height: u32,
        _timestamp_ms: u64,
    ) -> EncoderResult<Option<H264Frame>> {
        Err(EncoderError::FeatureDisabled)
    }

    pub fn force_keyframe(&mut self) {}

    pub fn stats(&self) -> EncoderStats {
        EncoderStats {
            frames_encoded: 0,
            bitrate_kbps: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_config_defaults() {
        let config = EncoderConfig::default();
        assert_eq!(config.bitrate_kbps, 5000);
        assert!((config.max_fps - 30.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_encoder_config_presets() {
        let hq = EncoderConfig::high_quality();
        assert_eq!(hq.bitrate_kbps, 10000);

        let lb = EncoderConfig::low_bandwidth();
        assert_eq!(lb.bitrate_kbps, 1000);
    }

    #[cfg(feature = "h264")]
    #[test]
    fn test_encoder_creation() {
        let config = EncoderConfig::default();
        let encoder = Avc420Encoder::new(config);
        assert!(encoder.is_ok());
    }

    #[cfg(feature = "h264")]
    #[test]
    fn test_encode_small_frame() {
        let config = EncoderConfig::default();
        let mut encoder = Avc420Encoder::new(config).unwrap();

        // Create a 64x64 black BGRA frame
        let width = 64u32;
        let height = 64u32;
        let bgra_data = vec![0u8; (width * height * 4) as usize];

        let result = encoder.encode_bgra(&bgra_data, width, height, 0);
        assert!(result.is_ok());
    }

    #[cfg(feature = "h264")]
    #[test]
    fn test_invalid_dimensions() {
        let config = EncoderConfig::default();
        let mut encoder = Avc420Encoder::new(config).unwrap();

        // Odd dimensions should fail
        let bgra_data = vec![0u8; 63 * 64 * 4];
        let result = encoder.encode_bgra(&bgra_data, 63, 64, 0);
        assert!(matches!(result, Err(EncoderError::InvalidDimensions { .. })));
    }

    #[test]
    fn test_annex_b_to_avc_4byte_start_code() {
        // Single NAL with 4-byte start code: 0x00 0x00 0x00 0x01 + NAL data
        let annex_b = vec![0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1e];
        let avc = annex_b_to_avc(&annex_b);

        // Expected: 4-byte length (4) + NAL data
        assert_eq!(avc.len(), 8);
        // Length prefix: 0x00 0x00 0x00 0x04
        assert_eq!(&avc[0..4], &[0x00, 0x00, 0x00, 0x04]);
        // NAL data unchanged
        assert_eq!(&avc[4..8], &[0x67, 0x42, 0x00, 0x1e]);
    }

    #[test]
    fn test_annex_b_to_avc_3byte_start_code() {
        // Single NAL with 3-byte start code: 0x00 0x00 0x01 + NAL data
        let annex_b = vec![0x00, 0x00, 0x01, 0x68, 0xce, 0x3c, 0x80];
        let avc = annex_b_to_avc(&annex_b);

        // Expected: 4-byte length (4) + NAL data
        assert_eq!(avc.len(), 8);
        assert_eq!(&avc[0..4], &[0x00, 0x00, 0x00, 0x04]);
        assert_eq!(&avc[4..8], &[0x68, 0xce, 0x3c, 0x80]);
    }

    #[test]
    fn test_annex_b_to_avc_multiple_nals() {
        // Two NALs: SPS + PPS typical pattern
        // Note: NAL data must not contain sequences that look like start codes
        let annex_b = vec![
            // NAL 1: 4-byte start code + 3 bytes data (no 0x00 0x00 sequences)
            0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x1e,
            // NAL 2: 3-byte start code + 2 bytes data
            0x00, 0x00, 0x01, 0x68, 0xce,
        ];
        let avc = annex_b_to_avc(&annex_b);

        // First NAL: length 3 + data
        assert_eq!(&avc[0..4], &[0x00, 0x00, 0x00, 0x03]);
        assert_eq!(&avc[4..7], &[0x67, 0x42, 0x1e]);

        // Second NAL: length 2 + data
        assert_eq!(&avc[7..11], &[0x00, 0x00, 0x00, 0x02]);
        assert_eq!(&avc[11..13], &[0x68, 0xce]);
    }

    #[test]
    fn test_annex_b_to_avc_empty() {
        let annex_b: Vec<u8> = vec![];
        let avc = annex_b_to_avc(&annex_b);
        assert!(avc.is_empty());
    }

    #[test]
    fn test_annex_b_to_avc_no_start_code() {
        // Data without start code should produce empty output
        let annex_b = vec![0x67, 0x42, 0x00, 0x1e];
        let avc = annex_b_to_avc(&annex_b);
        assert!(avc.is_empty());
    }

    #[test]
    fn test_align_to_16() {
        assert_eq!(align_to_16(0), 0);
        assert_eq!(align_to_16(1), 16);
        assert_eq!(align_to_16(15), 16);
        assert_eq!(align_to_16(16), 16);
        assert_eq!(align_to_16(17), 32);
        assert_eq!(align_to_16(1920), 1920); // Already aligned
        assert_eq!(align_to_16(1080), 1088); // Needs padding
        assert_eq!(align_to_16(1921), 1936);
    }

    // Note: Avc420Region and create_avc420_bitmap_stream tests are in ironrdp-egfx
}
