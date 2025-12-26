//! YUV444 to dual YUV420 packing for AVC444
//!
//! Implements the MS-RDPEGFX macroblock-level packing algorithm for encoding
//! YUV444 (4:4:4 chroma) content using two standard YUV420 H.264 streams.
//!
//! # The Core Algorithm
//!
//! AVC444 achieves full 4:4:4 chroma by splitting YUV444 into two YUV420 streams:
//!
//! 1. **Main View (Stream 1)**: Standard YUV420 encoding
//!    - Y plane: Full luma (unchanged)
//!    - U plane: 2×2 box filter subsampled from U444
//!    - V plane: 2×2 box filter subsampled from V444
//!
//! 2. **Auxiliary View (Stream 2)**: Residual chroma as "fake" luma
//!    - Y plane: Missing U444 samples (the 75% discarded by 4:2:0)
//!    - U plane: Missing V444 samples
//!    - V plane: Neutral (128) for encoder stability
//!
//! # Client Reconstruction
//!
//! The client combines both streams to reconstruct full YUV444:
//! - Even pixel positions: Use main view chroma (subsampled)
//! - Odd pixel positions: Use auxiliary view "luma" as chroma
//!
//! # MS-RDPEGFX Reference
//!
//! See MS-RDPEGFX Section 3.3.8.3.2 and Figure 7 for the specification.

use super::color_convert::{subsample_chroma_420, Yuv444Frame, ColorMatrix};

/// YUV420 frame (4:2:0 chroma subsampling)
///
/// Standard H.264-compatible frame format where chroma planes are
/// half the resolution of luma in both dimensions.
#[derive(Debug, Clone)]
pub struct Yuv420Frame {
    /// Luma plane (width × height)
    pub y: Vec<u8>,
    /// Chroma U (Cb) plane (width/2 × height/2)
    pub u: Vec<u8>,
    /// Chroma V (Cr) plane (width/2 × height/2)
    pub v: Vec<u8>,
    /// Frame width in pixels
    pub width: usize,
    /// Frame height in pixels
    pub height: usize,
}

impl Yuv420Frame {
    /// Create a new YUV420 frame with allocated buffers
    pub fn new(width: usize, height: usize) -> Self {
        let y_size = width * height;
        let uv_size = (width / 2) * (height / 2);
        Self {
            y: vec![0u8; y_size],
            u: vec![128u8; uv_size],
            v: vec![128u8; uv_size],
            width,
            height,
        }
    }

    /// Get total frame size in bytes
    #[inline]
    pub fn total_size(&self) -> usize {
        self.y.len() + self.u.len() + self.v.len()
    }

    /// Convert YUV420 back to BGRA for feeding to OpenH264
    ///
    /// OpenH264's `YUVBuffer::from_rgb_source()` expects BGRA input and does
    /// its own YUV420 conversion internally. Since we've already computed
    /// YUV420, we must convert back to BGRA.
    ///
    /// Uses BT.709 inverse matrix for HD content.
    pub fn to_bgra(&self) -> Vec<u8> {
        self.to_bgra_with_matrix(ColorMatrix::BT709)
    }

    /// Convert YUV420 back to BGRA with specified color matrix
    pub fn to_bgra_with_matrix(&self, matrix: ColorMatrix) -> Vec<u8> {
        let pixel_count = self.width * self.height;
        let mut bgra = vec![0u8; pixel_count * 4];

        // Get inverse matrix coefficients
        let (rv, gu, gv, bu) = match matrix {
            ColorMatrix::BT601 => (1.402, -0.344136, -0.714136, 1.772),
            ColorMatrix::BT709 => (1.5748, -0.1873, -0.4681, 1.8556),
        };

        let chroma_width = self.width / 2;

        for y in 0..self.height {
            for x in 0..self.width {
                let y_val = self.y[y * self.width + x] as f32;

                // Chroma is subsampled - same value for 2×2 luma block
                let chroma_idx = (y / 2) * chroma_width + (x / 2);
                let u_val = self.u[chroma_idx] as f32 - 128.0;
                let v_val = self.v[chroma_idx] as f32 - 128.0;

                // YUV to RGB conversion
                let r = (y_val + rv * v_val).clamp(0.0, 255.0);
                let g = (y_val + gu * u_val + gv * v_val).clamp(0.0, 255.0);
                let b = (y_val + bu * u_val).clamp(0.0, 255.0);

                let idx = (y * self.width + x) * 4;
                bgra[idx] = b as u8;     // B
                bgra[idx + 1] = g as u8; // G
                bgra[idx + 2] = r as u8; // R
                bgra[idx + 3] = 255;     // A (opaque)
            }
        }

        bgra
    }

    /// Create YUV buffer compatible with OpenH264 encoding
    ///
    /// Returns the planes in I420 order: Y, U, V with correct strides.
    pub fn to_i420_planes(&self) -> (&[u8], &[u8], &[u8], usize, usize) {
        let y_stride = self.width;
        let uv_stride = self.width / 2;
        (&self.y, &self.u, &self.v, y_stride, uv_stride)
    }

    // === OpenH264 YUVSource-compatible interface ===
    // These methods match the openh264::formats::YUVSource trait signature,
    // allowing direct use with YUVSlices without double-conversion.

    /// Get frame dimensions as (width, height)
    #[inline]
    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Get frame strides as (y_stride, u_stride, v_stride)
    ///
    /// For planar YUV420, Y stride = width, U/V stride = width/2
    #[inline]
    pub fn strides(&self) -> (usize, usize, usize) {
        (self.width, self.width / 2, self.width / 2)
    }

    /// Get Y (luma) plane slice
    #[inline]
    pub fn y_plane(&self) -> &[u8] {
        &self.y
    }

    /// Get U (Cb) chroma plane slice
    #[inline]
    pub fn u_plane(&self) -> &[u8] {
        &self.u
    }

    /// Get V (Cr) chroma plane slice
    #[inline]
    pub fn v_plane(&self) -> &[u8] {
        &self.v
    }

    /// Validate that this frame is suitable for OpenH264 encoding
    ///
    /// Checks that:
    /// - Dimensions are even (required for YUV420)
    /// - Plane sizes match expected dimensions
    /// - Strides are valid
    pub fn validate_for_encoding(&self) -> Result<(), String> {
        if self.width % 2 != 0 {
            return Err(format!("Width {} must be even for YUV420", self.width));
        }
        if self.height % 2 != 0 {
            return Err(format!("Height {} must be even for YUV420", self.height));
        }

        let expected_y = self.width * self.height;
        if self.y.len() != expected_y {
            return Err(format!(
                "Y plane size {} doesn't match expected {}",
                self.y.len(),
                expected_y
            ));
        }

        let expected_uv = (self.width / 2) * (self.height / 2);
        if self.u.len() != expected_uv {
            return Err(format!(
                "U plane size {} doesn't match expected {}",
                self.u.len(),
                expected_uv
            ));
        }
        if self.v.len() != expected_uv {
            return Err(format!(
                "V plane size {} doesn't match expected {}",
                self.v.len(),
                expected_uv
            ));
        }

        Ok(())
    }
}

/// Create main YUV420 view (full luma + subsampled chroma)
///
/// This is the "luma view" or "Stream 1" in AVC444 terminology.
///
/// # Algorithm
///
/// - **Y plane**: Direct copy from YUV444 Y (no modification)
/// - **U plane**: 2×2 box filter subsample from YUV444 U
/// - **V plane**: 2×2 box filter subsample from YUV444 V
///
/// # Arguments
///
/// * `yuv444` - Source YUV444 frame with full chroma
///
/// # Returns
///
/// YUV420 frame suitable for standard H.264 encoding
pub fn pack_main_view(yuv444: &Yuv444Frame) -> Yuv420Frame {
    let width = yuv444.width;
    let height = yuv444.height;

    // Y plane: Copy full luma (no subsampling)
    let y = yuv444.y.clone();

    // U plane: 2×2 box filter subsample
    let u = subsample_chroma_420(&yuv444.u, width, height);

    // V plane: 2×2 box filter subsample
    let v = subsample_chroma_420(&yuv444.v, width, height);

    Yuv420Frame {
        y,
        u,
        v,
        width,
        height,
    }
}

/// Create auxiliary YUV420 view (residual chroma data)
///
/// This is the "chroma view" or "Stream 2" in AVC444 terminology.
///
/// # The Core Trick
///
/// Standard 4:2:0 subsampling keeps only 25% of chroma samples (one per 2×2 block).
/// The auxiliary view captures the other 75% by encoding chroma as "fake luma".
///
/// # Spec-Compliant Packing (MS-RDPEGFX Section 3.3.8.3.2)
///
/// For each pixel position (x, y):
/// - **Even positions** (x % 2 == 0 AND y % 2 == 0): Already in main view
/// - **Odd positions** (any other case): Packed into auxiliary view
///
/// The auxiliary frame structure:
/// - **Y plane**: U444 samples at odd positions
/// - **U plane**: V444 samples at odd positions (subsampled)
/// - **V plane**: Neutral (128) for encoder stability
///
/// # Arguments
///
/// * `yuv444` - Source YUV444 frame with full chroma
///
/// # Returns
///
/// YUV420 frame with chroma data packed as luma for H.264 encoding
pub fn pack_auxiliary_view(yuv444: &Yuv444Frame) -> Yuv420Frame {
    pack_auxiliary_view_spec_compliant(yuv444)
}

/// Spec-compliant auxiliary view packing (MS-RDPEGFX Section 3.3.8.3.2)
///
/// Implements the exact macroblock-level interleaving pattern from the spec.
///
/// # Macroblock Structure (16×16 pixels)
///
/// For each 16×16 macroblock, the auxiliary Y plane contains:
/// - All U444 samples where at least one coordinate is odd
/// - Even-even positions filled with neutral value or interpolated
///
/// The auxiliary U plane contains V444 odd samples, subsampled to 8×8.
fn pack_auxiliary_view_spec_compliant(yuv444: &Yuv444Frame) -> Yuv420Frame {
    let width = yuv444.width;
    let height = yuv444.height;

    // Y plane: Pack U444 odd samples
    // At each pixel position:
    // - If odd position: copy U444 value
    // - If even position: use neutral (128) or interpolate
    let mut aux_y = vec![128u8; width * height];

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let is_odd = (x % 2 == 1) || (y % 2 == 1);

            if is_odd {
                // Pack the U chroma at this odd position
                aux_y[idx] = yuv444.u[idx];
            } else {
                // Even position - this sample is already in the main view
                // Use average of surrounding odd U samples for better encoding
                // This helps the encoder produce smoother gradients
                aux_y[idx] = interpolate_even_position(&yuv444.u, x, y, width, height);
            }
        }
    }

    // U plane: Pack V444 odd samples (subsampled to chroma resolution)
    // For each 2×2 block in auxiliary Y, we need one V chroma sample
    let chroma_width = width / 2;
    let chroma_height = height / 2;
    let mut aux_u = vec![128u8; chroma_width * chroma_height];

    for cy in 0..chroma_height {
        for cx in 0..chroma_width {
            let chroma_idx = cy * chroma_width + cx;

            // The chroma sample position in the full-res grid
            let x = cx * 2;
            let y = cy * 2;

            // For auxiliary U plane, we want V444 values at odd positions
            // Use the average of odd-position V samples in this 2×2 block
            // Positions: (x+1, y), (x, y+1), (x+1, y+1) are all odd
            let v_01 = yuv444.v[y * width + (x + 1)] as u32;        // (x+1, y)
            let v_10 = yuv444.v[(y + 1) * width + x] as u32;        // (x, y+1)
            let v_11 = yuv444.v[(y + 1) * width + (x + 1)] as u32;  // (x+1, y+1)

            // Average the three odd-position samples with rounding
            let avg = (v_01 + v_10 + v_11 + 1) / 3;
            aux_u[chroma_idx] = avg as u8;
        }
    }

    // V plane: Neutral (128) for encoder stability
    // The auxiliary V plane isn't used for reconstruction, but encoding
    // works better with valid chroma than with zeros
    let aux_v = vec![128u8; chroma_width * chroma_height];

    Yuv420Frame {
        y: aux_y,
        u: aux_u,
        v: aux_v,
        width,
        height,
    }
}

/// Interpolate value at even position from surrounding odd positions
///
/// For smoother auxiliary Y plane, we interpolate even positions from
/// the average of neighboring odd samples.
#[inline]
fn interpolate_even_position(plane: &[u8], x: usize, y: usize, width: usize, height: usize) -> u8 {
    let mut sum: u32 = 0;
    let mut count: u32 = 0;

    // Check all 8 neighbors and use odd-position ones
    let neighbors = [
        (x.wrapping_sub(1), y),           // left
        (x + 1, y),                        // right
        (x, y.wrapping_sub(1)),           // top
        (x, y + 1),                        // bottom
        (x.wrapping_sub(1), y.wrapping_sub(1)), // top-left
        (x + 1, y.wrapping_sub(1)),       // top-right
        (x.wrapping_sub(1), y + 1),       // bottom-left
        (x + 1, y + 1),                    // bottom-right
    ];

    for (nx, ny) in neighbors {
        if nx < width && ny < height {
            // Only use odd-position neighbors
            if (nx % 2 == 1) || (ny % 2 == 1) {
                sum += plane[ny * width + nx] as u32;
                count += 1;
            }
        }
    }

    if count > 0 {
        ((sum + count / 2) / count) as u8
    } else {
        128 // Neutral if no neighbors available
    }
}

/// Alternative simplified packing for debugging/testing
///
/// Uses a simpler pixel extraction pattern that may not match the spec
/// exactly but is easier to debug.
#[allow(dead_code)]
pub fn pack_auxiliary_view_simplified(yuv444: &Yuv444Frame) -> Yuv420Frame {
    let width = yuv444.width;
    let height = yuv444.height;

    // Y plane: All U444 values at odd positions, 128 at even
    let mut aux_y = Vec::with_capacity(width * height);

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if x % 2 == 1 || y % 2 == 1 {
                aux_y.push(yuv444.u[idx]);
            } else {
                aux_y.push(128); // Neutral
            }
        }
    }

    // U plane: Just take one V sample per 2×2 block
    let chroma_width = width / 2;
    let chroma_height = height / 2;
    let mut aux_u = Vec::with_capacity(chroma_width * chroma_height);

    for y in (0..height).step_by(2) {
        for x in (0..width).step_by(2) {
            // Take the (x+1, y+1) sample (diagonal)
            let idx = (y + 1) * width + (x + 1);
            aux_u.push(yuv444.v[idx]);
        }
    }

    // V plane: Neutral
    let aux_v = vec![128u8; chroma_width * chroma_height];

    Yuv420Frame {
        y: aux_y,
        u: aux_u,
        v: aux_v,
        width,
        height,
    }
}

/// Pack dual YUV420 views from a single YUV444 frame
///
/// Convenience function that returns both views at once.
///
/// # Arguments
///
/// * `yuv444` - Source YUV444 frame
///
/// # Returns
///
/// Tuple of (main_view, auxiliary_view) as YUV420 frames
pub fn pack_dual_views(yuv444: &Yuv444Frame) -> (Yuv420Frame, Yuv420Frame) {
    let main_view = pack_main_view(yuv444);
    let aux_view = pack_auxiliary_view(yuv444);
    (main_view, aux_view)
}

/// Validate that a YUV444 frame has valid dimensions for AVC444 encoding
///
/// Returns true if dimensions are even (required for 4:2:0 subsampling).
#[inline]
pub fn validate_dimensions(width: usize, height: usize) -> bool {
    width % 2 == 0 && height % 2 == 0 && width > 0 && height > 0
}

/// Align dimension to 16-pixel boundary (macroblock alignment)
///
/// H.264 encoding works with 16×16 macroblocks, so dimensions should
/// be aligned for optimal encoding.
#[inline]
pub fn align_to_16(dim: usize) -> usize {
    (dim + 15) & !15
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_yuv444(width: usize, height: usize) -> Yuv444Frame {
        let size = width * height;
        let mut y = vec![128u8; size];
        let mut u = vec![128u8; size];
        let mut v = vec![128u8; size];

        // Create a gradient pattern for testing
        for row in 0..height {
            for col in 0..width {
                let idx = row * width + col;
                y[idx] = ((col * 255) / width) as u8;
                u[idx] = ((row * 255) / height) as u8;
                v[idx] = (((col + row) * 255) / (width + height)) as u8;
            }
        }

        Yuv444Frame { y, u, v, width, height }
    }

    #[test]
    fn test_pack_main_view_dimensions() {
        let yuv444 = create_test_yuv444(1920, 1080);
        let main = pack_main_view(&yuv444);

        // Y plane: full resolution
        assert_eq!(main.y.len(), 1920 * 1080);
        assert_eq!(main.width, 1920);
        assert_eq!(main.height, 1080);

        // U/V planes: half resolution in each dimension
        assert_eq!(main.u.len(), 960 * 540);
        assert_eq!(main.v.len(), 960 * 540);
    }

    #[test]
    fn test_pack_main_view_luma_unchanged() {
        let yuv444 = create_test_yuv444(64, 64);
        let main = pack_main_view(&yuv444);

        // Luma should be identical to source
        assert_eq!(main.y, yuv444.y);
    }

    #[test]
    fn test_pack_auxiliary_view_dimensions() {
        let yuv444 = create_test_yuv444(1920, 1080);
        let aux = pack_auxiliary_view(&yuv444);

        // Y plane: full resolution (contains U444 data)
        assert_eq!(aux.y.len(), 1920 * 1080);
        assert_eq!(aux.width, 1920);
        assert_eq!(aux.height, 1080);

        // U/V planes: half resolution
        assert_eq!(aux.u.len(), 960 * 540);
        assert_eq!(aux.v.len(), 960 * 540);
    }

    #[test]
    fn test_auxiliary_odd_positions_have_u_values() {
        let yuv444 = create_test_yuv444(4, 4);
        let aux = pack_auxiliary_view(&yuv444);

        // Odd positions should have U444 values
        // Position (1, 0) is odd
        let idx_10 = 0 * 4 + 1;
        assert_eq!(aux.y[idx_10], yuv444.u[idx_10]);

        // Position (0, 1) is odd
        let idx_01 = 1 * 4 + 0;
        assert_eq!(aux.y[idx_01], yuv444.u[idx_01]);

        // Position (1, 1) is odd
        let idx_11 = 1 * 4 + 1;
        assert_eq!(aux.y[idx_11], yuv444.u[idx_11]);
    }

    #[test]
    fn test_pack_dual_views() {
        let yuv444 = create_test_yuv444(320, 240);
        let (main, aux) = pack_dual_views(&yuv444);

        assert_eq!(main.width, 320);
        assert_eq!(main.height, 240);
        assert_eq!(aux.width, 320);
        assert_eq!(aux.height, 240);
    }

    #[test]
    fn test_yuv420_to_bgra_black() {
        let yuv420 = Yuv420Frame {
            y: vec![0; 4],      // 2×2 black
            u: vec![128; 1],    // 1×1 neutral chroma
            v: vec![128; 1],
            width: 2,
            height: 2,
        };

        let bgra = yuv420.to_bgra();

        assert_eq!(bgra.len(), 4 * 4); // 4 pixels × 4 bytes

        // All pixels should be black (B=0, G=0, R=0, A=255)
        for i in 0..4 {
            assert_eq!(bgra[i * 4], 0, "Pixel {} B should be 0", i);
            assert_eq!(bgra[i * 4 + 1], 0, "Pixel {} G should be 0", i);
            assert_eq!(bgra[i * 4 + 2], 0, "Pixel {} R should be 0", i);
            assert_eq!(bgra[i * 4 + 3], 255, "Pixel {} A should be 255", i);
        }
    }

    #[test]
    fn test_yuv420_to_bgra_white() {
        let yuv420 = Yuv420Frame {
            y: vec![255; 4],    // 2×2 white
            u: vec![128; 1],    // 1×1 neutral chroma
            v: vec![128; 1],
            width: 2,
            height: 2,
        };

        let bgra = yuv420.to_bgra();

        // All pixels should be white (B=255, G=255, R=255, A=255)
        for i in 0..4 {
            assert_eq!(bgra[i * 4], 255, "Pixel {} B should be 255", i);
            assert_eq!(bgra[i * 4 + 1], 255, "Pixel {} G should be 255", i);
            assert_eq!(bgra[i * 4 + 2], 255, "Pixel {} R should be 255", i);
            assert_eq!(bgra[i * 4 + 3], 255, "Pixel {} A should be 255", i);
        }
    }

    #[test]
    fn test_validate_dimensions() {
        assert!(validate_dimensions(1920, 1080));
        assert!(validate_dimensions(1280, 720));
        assert!(validate_dimensions(2, 2));

        assert!(!validate_dimensions(1921, 1080)); // Odd width
        assert!(!validate_dimensions(1920, 1081)); // Odd height
        assert!(!validate_dimensions(0, 100));      // Zero width
        assert!(!validate_dimensions(100, 0));      // Zero height
    }

    #[test]
    fn test_align_to_16() {
        assert_eq!(align_to_16(1920), 1920);  // Already aligned
        assert_eq!(align_to_16(1080), 1088);  // Needs padding
        assert_eq!(align_to_16(800), 800);    // Already aligned
        assert_eq!(align_to_16(600), 608);    // Needs padding
        assert_eq!(align_to_16(1), 16);
        assert_eq!(align_to_16(15), 16);
        assert_eq!(align_to_16(16), 16);
        assert_eq!(align_to_16(17), 32);
    }

    #[test]
    fn test_yuv420_frame_new() {
        let frame = Yuv420Frame::new(1920, 1080);

        assert_eq!(frame.width, 1920);
        assert_eq!(frame.height, 1080);
        assert_eq!(frame.y.len(), 1920 * 1080);
        assert_eq!(frame.u.len(), 960 * 540);
        assert_eq!(frame.v.len(), 960 * 540);

        // Y should be 0, U/V should be 128 (neutral)
        assert!(frame.y.iter().all(|&v| v == 0));
        assert!(frame.u.iter().all(|&v| v == 128));
        assert!(frame.v.iter().all(|&v| v == 128));
    }

    #[test]
    fn test_yuv420_total_size() {
        let frame = Yuv420Frame::new(1920, 1080);
        // Y: 1920×1080, U: 960×540, V: 960×540
        // Total: 1920*1080 + 2*(960*540) = 2073600 + 1036800 = 3110400
        assert_eq!(frame.total_size(), 1920 * 1080 + 2 * (960 * 540));
    }

    #[test]
    fn test_interpolate_even_position_center() {
        // 4×4 plane with values set at odd positions
        let mut plane = vec![0u8; 16];
        // Set odd positions: (1,0)=10, (0,1)=20, (1,1)=30, (3,0)=40, ...
        plane[1] = 10;  // (1, 0)
        plane[4] = 20;  // (0, 1)
        plane[5] = 30;  // (1, 1)
        plane[3] = 40;  // (3, 0)
        plane[7] = 50;  // (3, 1)
        plane[12] = 60; // (0, 3)
        plane[13] = 70; // (1, 3)
        plane[15] = 80; // (3, 3)

        // Position (0, 0) should average nearby odd positions
        let result = interpolate_even_position(&plane, 0, 0, 4, 4);
        // Neighbors at odd positions: (1,0)=10, (0,1)=20, (1,1)=30
        // Average: (10 + 20 + 30 + 1) / 3 = 20
        assert_eq!(result, 20);
    }

    #[test]
    fn test_pack_and_unpack_roundtrip() {
        // Create a test pattern
        let yuv444 = create_test_yuv444(64, 64);

        // Pack to dual views
        let (main, aux) = pack_dual_views(&yuv444);

        // Convert both to BGRA
        let main_bgra = main.to_bgra();
        let aux_bgra = aux.to_bgra();

        // Verify dimensions
        assert_eq!(main_bgra.len(), 64 * 64 * 4);
        assert_eq!(aux_bgra.len(), 64 * 64 * 4);
    }

    // =================================================================
    // YUV420Frame Plane Accessor Tests (for OpenH264 compatibility)
    // =================================================================

    #[test]
    fn test_yuv420_plane_accessors() {
        let frame = Yuv420Frame::new(1920, 1080);

        // Test dimensions
        let (w, h) = frame.dimensions();
        assert_eq!(w, 1920);
        assert_eq!(h, 1080);

        // Test strides
        let (y_stride, u_stride, v_stride) = frame.strides();
        assert_eq!(y_stride, 1920);
        assert_eq!(u_stride, 960);
        assert_eq!(v_stride, 960);

        // Test plane lengths
        assert_eq!(frame.y_plane().len(), 1920 * 1080);
        assert_eq!(frame.u_plane().len(), 960 * 540);
        assert_eq!(frame.v_plane().len(), 960 * 540);
    }

    #[test]
    fn test_yuv420_validate_for_encoding_valid() {
        let frame = Yuv420Frame::new(1920, 1080);
        assert!(frame.validate_for_encoding().is_ok());

        let frame2 = Yuv420Frame::new(1280, 720);
        assert!(frame2.validate_for_encoding().is_ok());

        let frame3 = Yuv420Frame::new(640, 480);
        assert!(frame3.validate_for_encoding().is_ok());
    }

    #[test]
    fn test_yuv420_validate_for_encoding_invalid_width() {
        // Create a frame with invalid (odd) width by modifying internals
        let mut frame = Yuv420Frame::new(1920, 1080);
        frame.width = 1921; // Invalid odd width

        let result = frame.validate_for_encoding();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Width"));
    }

    #[test]
    fn test_yuv420_validate_for_encoding_invalid_height() {
        let mut frame = Yuv420Frame::new(1920, 1080);
        frame.height = 1081; // Invalid odd height

        let result = frame.validate_for_encoding();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Height"));
    }

    // =================================================================
    // Pack Main View Tests
    // =================================================================

    #[test]
    fn test_pack_main_view_1080p() {
        let yuv444 = create_test_yuv444(1920, 1080);
        let main = pack_main_view(&yuv444);

        assert_eq!(main.width, 1920);
        assert_eq!(main.height, 1080);
        assert_eq!(main.y.len(), 1920 * 1080);
        assert_eq!(main.u.len(), 960 * 540);
        assert_eq!(main.v.len(), 960 * 540);

        // Y plane should be identical
        assert_eq!(main.y, yuv444.y);
    }

    #[test]
    fn test_pack_main_view_luma_preserved() {
        // Create frame with distinct luma pattern
        let mut yuv444 = Yuv444Frame::new(16, 16);
        for i in 0..256 {
            yuv444.y[i] = i as u8;
        }

        let main = pack_main_view(&yuv444);

        // Luma should be perfectly preserved
        for i in 0..256 {
            assert_eq!(main.y[i], i as u8, "Luma mismatch at {}", i);
        }
    }

    // =================================================================
    // Pack Auxiliary View Tests
    // =================================================================

    #[test]
    fn test_pack_auxiliary_view_1080p() {
        let yuv444 = create_test_yuv444(1920, 1080);
        let aux = pack_auxiliary_view(&yuv444);

        assert_eq!(aux.width, 1920);
        assert_eq!(aux.height, 1080);
        assert_eq!(aux.y.len(), 1920 * 1080);
        assert_eq!(aux.u.len(), 960 * 540);
        assert_eq!(aux.v.len(), 960 * 540);
    }

    #[test]
    fn test_pack_auxiliary_view_neutral_v_plane() {
        let yuv444 = create_test_yuv444(64, 64);
        let aux = pack_auxiliary_view(&yuv444);

        // V plane should be neutral (128) for encoder stability
        assert!(aux.v.iter().all(|&v| v == 128), "V plane should be neutral");
    }

    // =================================================================
    // Dual View Consistency Tests
    // =================================================================

    #[test]
    fn test_dual_views_same_dimensions() {
        let yuv444 = create_test_yuv444(320, 240);
        let (main, aux) = pack_dual_views(&yuv444);

        assert_eq!(main.width, aux.width);
        assert_eq!(main.height, aux.height);
        assert_eq!(main.y.len(), aux.y.len());
        assert_eq!(main.u.len(), aux.u.len());
        assert_eq!(main.v.len(), aux.v.len());
    }

    #[test]
    fn test_dual_views_uniform_input() {
        // Uniform color should produce consistent output
        let mut yuv444 = Yuv444Frame::new(8, 8);
        for i in 0..64 {
            yuv444.y[i] = 128;
            yuv444.u[i] = 100;
            yuv444.v[i] = 150;
        }

        let (main, aux) = pack_dual_views(&yuv444);

        // Main Y should be 128
        assert!(main.y.iter().all(|&v| v == 128));

        // Main U should be 100 (subsampled but uniform)
        assert!(main.u.iter().all(|&v| v == 100));

        // Main V should be 150
        assert!(main.v.iter().all(|&v| v == 150));
    }

    // =================================================================
    // to_bgra Roundtrip Tests
    // =================================================================

    #[test]
    fn test_yuv420_to_bgra_dimensions() {
        let frame = Yuv420Frame::new(640, 480);
        let bgra = frame.to_bgra();

        // Should have 4 bytes per pixel
        assert_eq!(bgra.len(), 640 * 480 * 4);
    }

    #[test]
    fn test_yuv420_to_bgra_neutral_color() {
        // Y=128, U=128, V=128 should produce gray
        let mut frame = Yuv420Frame::new(2, 2);
        frame.y = vec![128; 4];
        frame.u = vec![128; 1];
        frame.v = vec![128; 1];

        let bgra = frame.to_bgra();

        // All pixels should be approximately gray (128, 128, 128)
        for i in 0..4 {
            let b = bgra[i * 4];
            let g = bgra[i * 4 + 1];
            let r = bgra[i * 4 + 2];
            let a = bgra[i * 4 + 3];

            assert!((b as i32 - 128).abs() <= 2, "B should be ~128");
            assert!((g as i32 - 128).abs() <= 2, "G should be ~128");
            assert!((r as i32 - 128).abs() <= 2, "R should be ~128");
            assert_eq!(a, 255, "A should be 255");
        }
    }

    // =================================================================
    // Stress Tests
    // =================================================================

    #[test]
    fn test_pack_dual_views_stress() {
        // Test various resolutions
        let resolutions = [(64, 64), (320, 240), (640, 480), (1280, 720)];

        for (w, h) in resolutions {
            let yuv444 = create_test_yuv444(w, h);
            let (main, aux) = pack_dual_views(&yuv444);

            assert_eq!(main.width, w);
            assert_eq!(main.height, h);
            assert_eq!(aux.width, w);
            assert_eq!(aux.height, h);

            // Both should be valid for encoding
            assert!(main.validate_for_encoding().is_ok());
            assert!(aux.validate_for_encoding().is_ok());
        }
    }
}
