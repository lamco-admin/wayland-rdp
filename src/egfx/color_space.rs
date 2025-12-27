//! Comprehensive color space configuration for H.264 encoding.
//!
//! This module provides a unified color space configuration that:
//! - Supports all major color standards (BT.709, BT.601, sRGB, BT.2020)
//! - Handles both full (PC) and limited (TV) value ranges
//! - Provides correct H.264 VUI parameters for each configuration
//! - Supports auto-detection based on resolution
//! - Works across all encoder backends (OpenH264, VA-API, NVENC)
//!
//! # H.264 VUI (Video Usability Information)
//!
//! VUI parameters are embedded in the SPS NAL unit and tell decoders
//! how to interpret color data. The key fields are:
//!
//! - `colour_primaries`: Which RGB primaries (chromaticity coordinates)
//! - `transfer_characteristics`: Gamma/transfer function (OETF/EOTF)
//! - `matrix_coefficients`: RGB↔YCbCr conversion matrix
//! - `video_full_range_flag`: Limited (0) or Full (1) range
//!
//! # References
//!
//! - ITU-T H.264 Annex E (VUI syntax and semantics)
//! - ITU-R BT.709-6 (HD television)
//! - ITU-R BT.601-7 (SD television)
//! - ITU-R BT.2020-2 (UHDTV)
//! - IEC 61966-2-1 (sRGB)

use serde::{Deserialize, Serialize};
use std::fmt;

// =============================================================================
// H.264 VUI Standard Values (ITU-T H.264 Table E-3, E-4, E-5)
// =============================================================================

/// H.264 colour_primaries values (Table E-3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ColorPrimaries {
    /// Reserved (treat as unspecified)
    Reserved = 0,
    /// ITU-R BT.709-6 / sRGB / IEC 61966-2-1
    BT709 = 1,
    /// Unspecified (decoder determines)
    Unspecified = 2,
    /// ITU-R BT.470-6 System M (historical NTSC)
    BT470M = 4,
    /// ITU-R BT.470-6 System B, G / ITU-R BT.601-7 625
    BT470BG = 5,
    /// SMPTE 170M / ITU-R BT.601-7 525 (NTSC)
    SMPTE170M = 6,
    /// SMPTE 240M (historical)
    SMPTE240M = 7,
    /// Generic film (C illuminant)
    Film = 8,
    /// ITU-R BT.2020-2 / ITU-R BT.2100-2
    BT2020 = 9,
    /// SMPTE ST 428-1 (D-Cinema DCI)
    SMPTE428 = 10,
    /// SMPTE RP 431-2 (D-Cinema P3 D65)
    SMPTE431 = 11,
    /// SMPTE EG 432-1 (P3 D65 Display)
    SMPTE432 = 12,
    /// EBU Tech 3213-E (PAL)
    EBU3213 = 22,
}

impl Default for ColorPrimaries {
    fn default() -> Self {
        Self::BT709
    }
}

impl ColorPrimaries {
    /// Get the raw u8 value for H.264 VUI
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// H.264 transfer_characteristics values (Table E-4)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TransferCharacteristics {
    /// Reserved
    Reserved = 0,
    /// ITU-R BT.709-6 / ITU-R BT.1361
    BT709 = 1,
    /// Unspecified
    Unspecified = 2,
    /// ITU-R BT.470-6 System M (2.2 gamma)
    BT470M = 4,
    /// ITU-R BT.470-6 System B, G (2.8 gamma)
    BT470BG = 5,
    /// SMPTE 170M / BT.601 (same as BT.709)
    SMPTE170M = 6,
    /// SMPTE 240M
    SMPTE240M = 7,
    /// Linear transfer (gamma 1.0)
    Linear = 8,
    /// Logarithmic (100:1)
    Log100 = 9,
    /// Logarithmic (100*sqrt(10):1)
    Log316 = 10,
    /// IEC 61966-2-4 (xvYCC)
    IEC61966_2_4 = 11,
    /// ITU-R BT.1361
    BT1361 = 12,
    /// IEC 61966-2-1 (sRGB)
    SRGB = 13,
    /// ITU-R BT.2020 10-bit (same as BT.709)
    BT2020_10 = 14,
    /// ITU-R BT.2020 12-bit (same as BT.709)
    BT2020_12 = 15,
    /// SMPTE ST 2084 (PQ / HDR10)
    SMPTE2084 = 16,
    /// SMPTE ST 428-1
    SMPTE428 = 17,
    /// ARIB STD-B67 (HLG)
    HLG = 18,
}

impl Default for TransferCharacteristics {
    fn default() -> Self {
        Self::BT709
    }
}

impl TransferCharacteristics {
    /// Get the raw u8 value for H.264 VUI
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// H.264 matrix_coefficients values (Table E-5)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum MatrixCoefficients {
    /// Identity (RGB, no conversion)
    Identity = 0,
    /// ITU-R BT.709-6 (Kr=0.2126, Kb=0.0722)
    BT709 = 1,
    /// Unspecified
    Unspecified = 2,
    /// FCC 73.682 (historical)
    FCC = 4,
    /// ITU-R BT.470-6 System B, G (BT.601-7 625)
    BT470BG = 5,
    /// SMPTE 170M / ITU-R BT.601-7 525 (Kr=0.299, Kb=0.114)
    SMPTE170M = 6,
    /// SMPTE 240M
    SMPTE240M = 7,
    /// YCgCo
    YCgCo = 8,
    /// ITU-R BT.2020 non-constant luminance
    BT2020_NCL = 9,
    /// ITU-R BT.2020 constant luminance
    BT2020_CL = 10,
    /// SMPTE ST 2085 (Y'D'zD'x)
    SMPTE2085 = 11,
    /// Chromaticity-derived non-constant luminance
    ChromaNCL = 12,
    /// Chromaticity-derived constant luminance
    ChromaCL = 13,
    /// ICtCp
    ICtCp = 14,
}

impl Default for MatrixCoefficients {
    fn default() -> Self {
        Self::BT709
    }
}

impl MatrixCoefficients {
    /// Get the raw u8 value for H.264 VUI
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Get the RGB to Y (luma) coefficients (Kr, Kg, Kb)
    ///
    /// Returns coefficients in floating-point format.
    /// Note: Kg = 1.0 - Kr - Kb
    pub const fn luma_coefficients(&self) -> (f32, f32, f32) {
        match self {
            // BT.709: Kr=0.2126, Kb=0.0722
            Self::BT709 | Self::Unspecified => (0.2126, 0.7152, 0.0722),
            // BT.601/SMPTE 170M: Kr=0.299, Kb=0.114
            Self::BT470BG | Self::SMPTE170M | Self::FCC => (0.299, 0.587, 0.114),
            // SMPTE 240M: Kr=0.212, Kb=0.087
            Self::SMPTE240M => (0.212, 0.701, 0.087),
            // BT.2020: Kr=0.2627, Kb=0.0593
            Self::BT2020_NCL | Self::BT2020_CL => (0.2627, 0.6780, 0.0593),
            // Identity (no conversion)
            Self::Identity => (1.0, 0.0, 0.0),
            // Default to BT.709 for others
            _ => (0.2126, 0.7152, 0.0722),
        }
    }

    /// Get the fixed-point luma coefficients (scaled by 65536)
    ///
    /// For efficient integer arithmetic in color conversion loops.
    pub const fn luma_coefficients_fixed(&self) -> (i32, i32, i32) {
        match self {
            Self::BT709 | Self::Unspecified => (13933, 46871, 4732),
            Self::BT470BG | Self::SMPTE170M | Self::FCC => (19595, 38470, 7471),
            Self::SMPTE240M => (13893, 45941, 5702),
            Self::BT2020_NCL | Self::BT2020_CL => (17218, 44430, 3888),
            Self::Identity => (65536, 0, 0),
            _ => (13933, 46871, 4732),
        }
    }
}

/// Color value range (full PC range vs limited TV range)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ColorRange {
    /// Limited range (Y: 16-235, UV: 16-240)
    ///
    /// Standard for broadcast/TV content. Also called "studio swing".
    #[default]
    Limited,

    /// Full range (Y: 0-255, UV: 0-255)
    ///
    /// Standard for computer graphics/PC content. Also called "full swing".
    Full,
}

impl ColorRange {
    /// Get the H.264 video_full_range_flag value
    pub const fn as_vui_flag(self) -> bool {
        matches!(self, Self::Full)
    }

    /// Get Y (luma) value range
    pub const fn y_range(self) -> (u8, u8) {
        match self {
            Self::Limited => (16, 235),
            Self::Full => (0, 255),
        }
    }

    /// Get UV (chroma) value range
    pub const fn uv_range(self) -> (u8, u8) {
        match self {
            Self::Limited => (16, 240),
            Self::Full => (0, 255),
        }
    }
}

// =============================================================================
// High-Level Color Space Presets
// =============================================================================

/// Color space preset for common use cases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorSpacePreset {
    /// Automatic selection based on resolution
    ///
    /// - ≥720p: BT.709 limited range (HD broadcast standard)
    /// - <720p: BT.601 limited range (SD broadcast standard)
    #[default]
    Auto,

    /// ITU-R BT.709 (HD content)
    ///
    /// Standard for HD television (1080p, 720p).
    /// Uses limited range by default.
    BT709,

    /// ITU-R BT.601 (SD content)
    ///
    /// Standard for SD television (480i, 576i).
    /// Uses limited range by default.
    BT601,

    /// sRGB (computer graphics)
    ///
    /// Standard for web content and computer displays.
    /// Uses BT.709 primaries with sRGB transfer function.
    /// Uses full range.
    SRGB,

    /// ITU-R BT.2020 (UHD/HDR content)
    ///
    /// Standard for 4K/8K content with wide color gamut.
    /// Uses limited range by default.
    BT2020,

    /// Custom configuration (use ColorSpaceConfig fields directly)
    Custom,
}

impl ColorSpacePreset {
    /// Parse from string (case-insensitive)
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "auto" | "automatic" => Self::Auto,
            "bt709" | "rec709" | "709" | "hd" => Self::BT709,
            "bt601" | "rec601" | "601" | "sd" => Self::BT601,
            "srgb" | "rgb" | "pc" => Self::SRGB,
            "bt2020" | "rec2020" | "2020" | "uhd" | "hdr" => Self::BT2020,
            "custom" => Self::Custom,
            _ => Self::Auto,
        }
    }
}

impl fmt::Display for ColorSpacePreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::BT709 => write!(f, "bt709"),
            Self::BT601 => write!(f, "bt601"),
            Self::SRGB => write!(f, "srgb"),
            Self::BT2020 => write!(f, "bt2020"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

// =============================================================================
// Unified Color Space Configuration
// =============================================================================

/// Complete color space configuration for H.264 encoding.
///
/// This struct encapsulates all parameters needed for correct color handling:
/// - RGB↔YCbCr conversion matrix (for our software conversion)
/// - H.264 VUI parameters (for signaling to decoders)
/// - Value range (full vs limited)
///
/// # Usage
///
/// ```
/// use lamco_rdp_server::egfx::color_space::{ColorSpaceConfig, ColorSpacePreset};
///
/// // Auto-detect based on resolution
/// let config = ColorSpaceConfig::from_resolution(1920, 1080);
///
/// // Use preset
/// let config = ColorSpaceConfig::from_preset(ColorSpacePreset::SRGB);
///
/// // Full customization
/// let config = ColorSpaceConfig::bt709_full();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColorSpaceConfig {
    /// Which preset was used to create this config (for reference)
    pub preset: ColorSpacePreset,

    /// RGB↔YCbCr conversion matrix coefficients
    pub matrix: MatrixCoefficients,

    /// Color value range (full vs limited)
    pub range: ColorRange,

    /// H.264 VUI colour_primaries
    pub primaries: ColorPrimaries,

    /// H.264 VUI transfer_characteristics
    pub transfer: TransferCharacteristics,
}

impl Default for ColorSpaceConfig {
    fn default() -> Self {
        Self::bt709_limited()
    }
}

impl ColorSpaceConfig {
    // =========================================================================
    // Standard Presets
    // =========================================================================

    /// BT.709 with limited range (default for HD broadcast)
    ///
    /// This is the most compatible setting for HD content.
    pub const fn bt709_limited() -> Self {
        Self {
            preset: ColorSpacePreset::BT709,
            matrix: MatrixCoefficients::BT709,
            range: ColorRange::Limited,
            primaries: ColorPrimaries::BT709,
            transfer: TransferCharacteristics::BT709,
        }
    }

    /// BT.709 with full range (for PC content)
    ///
    /// Use for screen sharing where full color fidelity is important.
    pub const fn bt709_full() -> Self {
        Self {
            preset: ColorSpacePreset::BT709,
            matrix: MatrixCoefficients::BT709,
            range: ColorRange::Full,
            primaries: ColorPrimaries::BT709,
            transfer: TransferCharacteristics::BT709,
        }
    }

    /// BT.601 with limited range (default for SD broadcast)
    pub const fn bt601_limited() -> Self {
        Self {
            preset: ColorSpacePreset::BT601,
            matrix: MatrixCoefficients::SMPTE170M,
            range: ColorRange::Limited,
            primaries: ColorPrimaries::SMPTE170M,
            transfer: TransferCharacteristics::SMPTE170M,
        }
    }

    /// BT.601 with full range
    pub const fn bt601_full() -> Self {
        Self {
            preset: ColorSpacePreset::BT601,
            matrix: MatrixCoefficients::SMPTE170M,
            range: ColorRange::Full,
            primaries: ColorPrimaries::SMPTE170M,
            transfer: TransferCharacteristics::SMPTE170M,
        }
    }

    /// sRGB with full range (ideal for desktop/web content)
    ///
    /// Uses BT.709 primaries (same as sRGB) but with sRGB transfer
    /// function for accurate computer graphics rendering.
    pub const fn srgb_full() -> Self {
        Self {
            preset: ColorSpacePreset::SRGB,
            matrix: MatrixCoefficients::BT709,
            range: ColorRange::Full,
            primaries: ColorPrimaries::BT709,
            transfer: TransferCharacteristics::SRGB,
        }
    }

    /// BT.2020 with limited range (for UHD content)
    pub const fn bt2020_limited() -> Self {
        Self {
            preset: ColorSpacePreset::BT2020,
            matrix: MatrixCoefficients::BT2020_NCL,
            range: ColorRange::Limited,
            primaries: ColorPrimaries::BT2020,
            transfer: TransferCharacteristics::BT2020_10,
        }
    }

    /// BT.2020 with full range
    pub const fn bt2020_full() -> Self {
        Self {
            preset: ColorSpacePreset::BT2020,
            matrix: MatrixCoefficients::BT2020_NCL,
            range: ColorRange::Full,
            primaries: ColorPrimaries::BT2020,
            transfer: TransferCharacteristics::BT2020_10,
        }
    }

    // =========================================================================
    // Factory Methods
    // =========================================================================

    /// Create configuration from a preset
    pub fn from_preset(preset: ColorSpacePreset) -> Self {
        match preset {
            ColorSpacePreset::Auto => Self::bt709_limited(), // Will be overridden by from_resolution
            ColorSpacePreset::BT709 => Self::bt709_limited(),
            ColorSpacePreset::BT601 => Self::bt601_limited(),
            ColorSpacePreset::SRGB => Self::srgb_full(),
            ColorSpacePreset::BT2020 => Self::bt2020_limited(),
            ColorSpacePreset::Custom => Self::bt709_limited(),
        }
    }

    /// Create configuration from a preset with specified range
    pub fn from_preset_with_range(preset: ColorSpacePreset, range: ColorRange) -> Self {
        let mut config = Self::from_preset(preset);
        config.range = range;
        config
    }

    /// Auto-select configuration based on resolution
    ///
    /// - ≥1280×720 (720p+): BT.709 limited
    /// - <1280×720: BT.601 limited
    ///
    /// For desktop sharing (where content is sRGB), consider using
    /// `srgb_full()` instead for accurate color reproduction.
    pub fn from_resolution(width: u32, height: u32) -> Self {
        let mut config = if width >= 1280 && height >= 720 {
            Self::bt709_limited()
        } else {
            Self::bt601_limited()
        };
        config.preset = ColorSpacePreset::Auto;
        config
    }

    /// Parse configuration from TOML config strings
    ///
    /// # Arguments
    ///
    /// * `preset_str` - Color space preset (e.g., "auto", "bt709", "srgb")
    /// * `range_str` - Optional range override (e.g., "full", "limited")
    /// * `width` - Resolution width (for auto mode)
    /// * `height` - Resolution height (for auto mode)
    pub fn from_config(
        preset_str: Option<&str>,
        range_str: Option<&str>,
        width: u32,
        height: u32,
    ) -> Self {
        let preset = preset_str
            .map(ColorSpacePreset::from_str_lossy)
            .unwrap_or(ColorSpacePreset::Auto);

        let mut config = match preset {
            ColorSpacePreset::Auto => Self::from_resolution(width, height),
            _ => Self::from_preset(preset),
        };

        // Apply range override if specified
        if let Some(range) = range_str {
            config.range = match range.to_lowercase().as_str() {
                "full" | "pc" => ColorRange::Full,
                "limited" | "tv" | "studio" => ColorRange::Limited,
                _ => config.range,
            };
        }

        config
    }

    // =========================================================================
    // Accessors for Color Conversion
    // =========================================================================

    /// Get the luma coefficients for RGB to Y conversion
    pub const fn luma_coefficients(&self) -> (f32, f32, f32) {
        self.matrix.luma_coefficients()
    }

    /// Get fixed-point luma coefficients (scaled by 65536)
    pub const fn luma_coefficients_fixed(&self) -> (i32, i32, i32) {
        self.matrix.luma_coefficients_fixed()
    }

    /// Check if this uses full range
    pub const fn is_full_range(&self) -> bool {
        matches!(self.range, ColorRange::Full)
    }

    /// Check if this uses limited range
    pub const fn is_limited_range(&self) -> bool {
        matches!(self.range, ColorRange::Limited)
    }

    // =========================================================================
    // VUI Parameter Access
    // =========================================================================

    /// Get all VUI parameters as a tuple for encoder configuration
    ///
    /// Returns (primaries, transfer, matrix, full_range_flag)
    pub const fn vui_params(&self) -> (u8, u8, u8, bool) {
        (
            self.primaries.as_u8(),
            self.transfer.as_u8(),
            self.matrix.as_u8(),
            self.range.as_vui_flag(),
        )
    }

    /// Get the H.264 video_full_range_flag value
    pub const fn vui_full_range_flag(&self) -> bool {
        self.range.as_vui_flag()
    }

    /// Get the H.264 colour_primaries value
    pub const fn vui_colour_primaries(&self) -> u8 {
        self.primaries.as_u8()
    }

    /// Get the H.264 transfer_characteristics value
    pub const fn vui_transfer_characteristics(&self) -> u8 {
        self.transfer.as_u8()
    }

    /// Get the H.264 matrix_coefficients value
    pub const fn vui_matrix_coefficients(&self) -> u8 {
        self.matrix.as_u8()
    }
}

impl fmt::Display for ColorSpaceConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let range_str = match self.range {
            ColorRange::Full => "full",
            ColorRange::Limited => "limited",
        };
        write!(f, "{} ({})", self.preset, range_str)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_from_str() {
        assert_eq!(ColorSpacePreset::from_str_lossy("auto"), ColorSpacePreset::Auto);
        assert_eq!(ColorSpacePreset::from_str_lossy("BT709"), ColorSpacePreset::BT709);
        assert_eq!(ColorSpacePreset::from_str_lossy("rec601"), ColorSpacePreset::BT601);
        assert_eq!(ColorSpacePreset::from_str_lossy("SRGB"), ColorSpacePreset::SRGB);
        assert_eq!(ColorSpacePreset::from_str_lossy("bt2020"), ColorSpacePreset::BT2020);
        assert_eq!(ColorSpacePreset::from_str_lossy("invalid"), ColorSpacePreset::Auto);
    }

    #[test]
    fn test_resolution_detection() {
        // HD content
        let hd = ColorSpaceConfig::from_resolution(1920, 1080);
        assert_eq!(hd.matrix, MatrixCoefficients::BT709);

        // 720p boundary
        let hd720 = ColorSpaceConfig::from_resolution(1280, 720);
        assert_eq!(hd720.matrix, MatrixCoefficients::BT709);

        // SD content
        let sd = ColorSpaceConfig::from_resolution(640, 480);
        assert_eq!(sd.matrix, MatrixCoefficients::SMPTE170M);
    }

    #[test]
    fn test_vui_params() {
        let bt709 = ColorSpaceConfig::bt709_limited();
        let (primaries, transfer, matrix, full_range) = bt709.vui_params();

        assert_eq!(primaries, 1);  // BT.709
        assert_eq!(transfer, 1);   // BT.709
        assert_eq!(matrix, 1);     // BT.709
        assert!(!full_range);      // Limited

        let srgb = ColorSpaceConfig::srgb_full();
        let (_, transfer, _, full_range) = srgb.vui_params();

        assert_eq!(transfer, 13);  // sRGB
        assert!(full_range);       // Full
    }

    #[test]
    fn test_luma_coefficients() {
        let bt709 = ColorSpaceConfig::bt709_limited();
        let (kr, kg, kb) = bt709.luma_coefficients();

        // BT.709 coefficients
        assert!((kr - 0.2126).abs() < 0.001);
        assert!((kg - 0.7152).abs() < 0.001);
        assert!((kb - 0.0722).abs() < 0.001);

        let bt601 = ColorSpaceConfig::bt601_limited();
        let (kr, kg, kb) = bt601.luma_coefficients();

        // BT.601 coefficients
        assert!((kr - 0.299).abs() < 0.001);
        assert!((kg - 0.587).abs() < 0.001);
        assert!((kb - 0.114).abs() < 0.001);
    }

    #[test]
    fn test_range_values() {
        assert_eq!(ColorRange::Limited.y_range(), (16, 235));
        assert_eq!(ColorRange::Limited.uv_range(), (16, 240));
        assert_eq!(ColorRange::Full.y_range(), (0, 255));
        assert_eq!(ColorRange::Full.uv_range(), (0, 255));
    }

    #[test]
    fn test_config_from_toml() {
        // Auto mode
        let auto_hd = ColorSpaceConfig::from_config(Some("auto"), None, 1920, 1080);
        assert_eq!(auto_hd.matrix, MatrixCoefficients::BT709);

        let auto_sd = ColorSpaceConfig::from_config(Some("auto"), None, 640, 480);
        assert_eq!(auto_sd.matrix, MatrixCoefficients::SMPTE170M);

        // Range override
        let bt709_full = ColorSpaceConfig::from_config(Some("bt709"), Some("full"), 1920, 1080);
        assert!(bt709_full.is_full_range());

        // sRGB preset
        let srgb = ColorSpaceConfig::from_config(Some("srgb"), None, 1920, 1080);
        assert_eq!(srgb.transfer, TransferCharacteristics::SRGB);
        assert!(srgb.is_full_range());
    }
}
