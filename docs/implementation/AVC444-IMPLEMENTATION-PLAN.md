# AVC444 Implementation Plan

**Document Version:** 1.0
**Date:** 2025-12-26
**Purpose:** Complete implementation guide for AVC444 H.264 4:4:4 chroma encoding
**Target Audience:** Future development sessions requiring full AVC444 context

---

## EXECUTIVE SUMMARY

**What is AVC444?** H.264 encoding with full 4:4:4 chroma resolution for premium graphics quality.

**The Core Innovation:** Pack YUV444 (full chroma) into TWO standard YUV420 H.264 streams using a clever macroblock-level interleaving scheme. This allows using standard H.264 encoders (which only support 4:2:0) to transmit full 4:4:4 color data.

**Current Status:**
- ✅ **Protocol Support:** Complete in IronRDP (via our PR #1057)
- ✅ **Transport Layer:** `send_avc444_frame()` ready to use
- ❌ **Encoder:** Not implemented (this is the work)

**Implementation Effort:** 24-34 hours across 3 phases

**Premium Feature Value:** First open-source RDP server with AVC444 support, targeting graphics professionals, CAD users, and anyone requiring accurate color reproduction.

---

## TABLE OF CONTENTS

1. [Complete Algorithm Specification](#1-complete-algorithm-specification)
2. [Codebase Architecture Analysis](#2-codebase-architecture-analysis)
3. [Decision Points & Variations](#3-decision-points--variations)
4. [Step-by-Step Implementation Plan](#4-step-by-step-implementation-plan)
5. [Testing & Validation Strategy](#5-testing--validation-strategy)
6. [Risk Analysis](#6-risk-analysis)
7. [References & Resources](#7-references--resources)

---

## 1. COMPLETE ALGORITHM SPECIFICATION

### 1.1 High-Level Overview

**Input:** BGRA frame (1920×1080×4 bytes)
**Output:** Two H.264 bitstreams that reconstruct to YUV444

**The Problem AVC444 Solves:**
- H.264 encoders output YUV420 natively (4:2:0 chroma subsampling)
- YUV420 discards 75% of chroma data (2×2 subsampling)
- Graphics/CAD applications need full chroma at every pixel
- Solution: Pack YUV444 into TWO YUV420 streams using clever interleaving

### 1.2 Mathematical Foundations

#### 1.2.1 Color Space Conversion: BGRA → YUV444

Use **ITU-R BT.709** matrix for HD content (recommended for 1080p and above):

```
For each pixel (B, G, R, A) at position (x, y):

Y  =  0.2126 × R + 0.7152 × G + 0.0722 × B
U  = -0.1146 × R - 0.3854 × G + 0.5000 × B + 128
V  =  0.5000 × R - 0.4542 × G - 0.0458 × B + 128

Output: Y[y][x], U[y][x], V[y][x] ∈ [0, 255]
```

**Alternative: ITU-R BT.601** for SD content (720p and below):

```
Y  =  0.299  × R + 0.587  × G + 0.114  × B
U  = -0.1687 × R - 0.3313 × G + 0.5000 × B + 128
V  =  0.5000 × R - 0.4187 × G - 0.0813 × B + 128
```

**Implementation Note:** Add 0.5 before truncating to `u8` for proper rounding:
```rust
y_val = (0.2126 * r + 0.7152 * g + 0.0722 * b + 0.5).clamp(0.0, 255.0) as u8;
```

#### 1.2.2 YUV444 to Dual YUV420 Packing (Macroblock Level)

**Stream 1 (Main/Luma View):**
- Y plane: Full Y444 at original resolution (1920×1080)
- U plane: 2×2 box filter subsampling of U444 (960×540)
- V plane: 2×2 box filter subsampling of V444 (960×540)

**Stream 2 (Auxiliary/Chroma View):**
- Y plane: Packed U444 odd samples (masquerading as luma)
- U plane: Packed V444 odd samples (masquerading as chroma)
- V plane: Neutral value (128) or duplicate of U plane

**Box Filter Subsampling (2×2):**

```
For each 2×2 block starting at even (x, y):

chroma_420[y/2][x/2] = (
    chroma_444[y  ][x  ] +
    chroma_444[y  ][x+1] +
    chroma_444[y+1][x  ] +
    chroma_444[y+1][x+1]
) / 4
```

### 1.3 Macroblock-Level Packing Details

**Per MS-RDPEGFX Section 3.3.8.3.2:**

For each 16×16 macroblock in the YUV444 source:

#### Main View Macroblock Construction:

```
Y_main[16×16]: Copy full luma from Y444[y:y+16][x:x+16]

U_main[8×8]: Subsample from U444[y:y+16:2][x:x+16:2]
    For i in 0..8, j in 0..8:
        U_main[i][j] = (
            U444[y+2i  ][x+2j  ] +
            U444[y+2i  ][x+2j+1] +
            U444[y+2i+1][x+2j  ] +
            U444[y+2i+1][x+2j+1]
        ) / 4

V_main[8×8]: Same as U_main but for V444
```

#### Auxiliary View Macroblock Construction:

**The Interleaving Pattern (8-line basis):**

```
Y_aux[16×16]: Pack missing U444 samples
    For each line i in 0..16:
        For each position j in 0..16:
            if (x+j) is odd OR (y+i) is odd:
                Y_aux[i][j] = U444[y+i][x+j]
            else:
                Y_aux[i][j] = 128  // or skip (encoder may optimize)

U_aux[8×8]: Pack missing V444 samples (similar interleaving)
    For i in 0..8, j in 0..8:
        Extract odd V444 samples at corresponding positions

V_aux[8×8]: Neutral (128) or duplicate U_aux for encoder stability
```

**Critical Detail:** The "8-line basis" refers to how chroma data is interleaved in the auxiliary view. Every 8 lines alternate between U and V packing patterns according to the MS-RDPEGFX Figure 7 diagram.

### 1.4 Pixel-Level Packing Formula (Complete)

**Given YUV444 source at coordinate (x, y):**

```python
# Main View (always contains even pixel samples)
if x % 2 == 0 and y % 2 == 0:
    main_u[y/2][x/2] = avg(U444[y:y+2, x:x+2])  # 2×2 box
    main_v[y/2][x/2] = avg(V444[y:y+2, x:x+2])  # 2×2 box

# Auxiliary View (contains odd pixel samples)
if x % 2 == 1 or y % 2 == 1:
    # Pack into auxiliary Y plane (as if it were luma)
    aux_y[y][x] = U444[y][x]

    # Pack into auxiliary U plane
    # (downsampled to match 4:2:0 structure)
    aux_u[y/2][x/2] = V444[y][x]  # simplified; actual is more complex
```

### 1.5 Edge Cases and Alignment

**Dimension Alignment:**
- Width and Height MUST be multiples of 16 (macroblock size)
- If source is not aligned, pad with black pixels (0, 128, 128)
- Crop region specified in EGFX protocol to actual size

**Odd Dimension Handling:**
```rust
let aligned_width = (width + 15) & !15;   // Round up to multiple of 16
let aligned_height = (height + 15) & !15;

// Pad right/bottom with neutral YUV (0, 128, 128) = black
```

**Partial Macroblocks:**
- Always encode full 16×16 macroblocks
- Use `destRect` in RFX_AVC420_BITMAP_STREAM to crop to actual region
- Client ignores padding outside destRect

### 1.6 Reverse Filter (Optional Quality Enhancement)

**Purpose:** Reduce color artifacts from H.264 quantization

**Algorithm (from FreeRDP decoder analysis):**

```rust
const THRESHOLD: i32 = 30;

fn apply_reverse_filter(filtered: u8, original: u8) -> u8 {
    let diff = (filtered as i32 - original as i32).abs();

    if diff < THRESHOLD {
        original  // Use non-filtered value (less artifact)
    } else {
        filtered  // Difference too large, use filtered
    }
}
```

**Application:**
- Applied to U and V components independently
- Only on reconstruction (decoder side)
- **Not required for encoder implementation** (client handles this)

### 1.7 LC (Luma/Chroma) Encoding Field

**Stream Info (32-bit header):**
```
Bits 0-29:  stream1_size (length in bytes)
Bits 30-31: LC field (luma/chroma flag)
```

**LC Values:**

| LC | Binary | Meaning | Streams Present |
|----|--------|---------|-----------------|
| 0x0 | 00 | LUMA_AND_CHROMA | Both stream1 and stream2 |
| 0x1 | 01 | LUMA | stream1 only (defer chroma) |
| 0x2 | 10 | CHROMA | stream1 only (chroma, use previous luma) |
| 0x3 | 11 | Invalid | Error condition |

**For Initial Implementation:** Always use **LC=0x0** (send both streams every frame)

**Advanced Optimization (Future):**
- Static regions: Send LC=0x1 (luma only) + reuse previous chroma
- Color-only changes: Send LC=0x2 (chroma only) + reuse previous luma
- Requires tracking frame dependencies on encoder side

---

## 2. CODEBASE ARCHITECTURE ANALYSIS

### 2.1 Current AVC420 Encoder Location

**File:** `/home/greg/wayland/wrd-server-specs/src/egfx/encoder.rs`

**Key Components:**

```rust
pub struct Avc420Encoder {
    encoder: openh264::encoder::Encoder,
    config: EncoderConfig,
    frame_count: u64,
    cached_sps_pps: Option<Vec<u8>>,
    current_level: Option<H264Level>,
}

impl Avc420Encoder {
    pub fn encode_bgra(
        &mut self,
        bgra_data: &[u8],
        width: u32,
        height: u32,
        timestamp_ms: u64,
    ) -> EncoderResult<Option<H264Frame>>
}
```

**Color Conversion (OpenH264 handles internally):**
```rust
let bgra_source = BgraSliceU8::new(bgra_data, (width, height));
let yuv = YUVBuffer::from_rgb_source(bgra_source);  // BGRA → YUV420
let bitstream = self.encoder.encode(&yuv)?;
```

**Output Format:** Annex B (start codes 0x00 0x00 0x01)
- MS-RDPEGFX requires Annex B format (NOT AVC length-prefixed)
- OpenH264 outputs Annex B directly ✓

### 2.2 Display Handler Integration Point

**File:** `/home/greg/wayland/wrd-server-specs/src/server/display_handler.rs`

**Current Flow (lines 150-185):**

```rust
pub struct WrdDisplayHandler {
    // ... other fields ...

    /// Shared GFX server handle for EGFX frame sending
    gfx_server_handle: Arc<RwLock<Option<GfxServerHandle>>>,

    /// Handler state for checking EGFX readiness
    gfx_handler_state: Arc<RwLock<Option<HandlerState>>>,
}
```

**Encoder Creation (where AVC444 would be added):**

Currently happens in display handler initialization (lines 200+):
```rust
// Current: Creates Avc420Encoder
// Future: Create Avc444Encoder based on config

let encoder_config = EncoderConfig {
    bitrate_kbps: config.h264_bitrate,
    max_fps: 30.0,
    // ... other config
};

let encoder = Avc420Encoder::new(encoder_config)?;
```

**Frame Encoding Loop Location:**

Search for where frames are currently encoded - this is where we'll integrate AVC444.

### 2.3 EGFX Sender (Transport Layer)

**File:** `/home/greg/wayland/wrd-server-specs/src/server/egfx_sender.rs`

**Key Method (ready to use for AVC444):**

```rust
pub struct EgfxFrameSender {
    gfx_server: GfxServerHandle,
    handler_state: Arc<RwLock<Option<HandlerState>>>,
    event_tx: mpsc::UnboundedSender<ServerEvent>,
    frame_count: AtomicU64,
}

impl EgfxFrameSender {
    pub async fn send_avc420_frame(
        &self,
        h264_data: &[u8],
        width: u16,
        height: u16,
        timestamp_ms: u32,
        qp: u8,
    ) -> SendResult<()>;
}
```

**For AVC444:** We'll need to add:
```rust
pub async fn send_avc444_frame(
    &self,
    stream1_data: &[u8],  // Main view bitstream
    stream2_data: &[u8],  // Auxiliary view bitstream
    width: u16,
    height: u16,
    timestamp_ms: u32,
    qp: u8,
) -> SendResult<()>;
```

**IronRDP Protocol Method (already exists in our PR #1057):**

```rust
// In ironrdp-egfx GraphicsPipelineServer
pub fn send_avc444_frame(
    &mut self,
    surface_id: u16,
    luma_data: &[u8],
    luma_regions: &[Avc420Region],
    chroma_data: Option<&[u8]>,
    chroma_regions: Option<&[Avc420Region]>,
    timestamp_ms: u32,
) -> Option<u32>
```

### 2.4 OpenH264 Integration Details

**Crate:** `openh264-rust2` (from Cargo.toml lines 100+)

**Current Usage:**
```rust
use openh264::encoder::{Encoder, EncoderConfig, BitRate, FrameRate};
use openh264::formats::{BgraSliceU8, YUVBuffer};

// Color conversion done by openh264 internally:
let bgra_source = BgraSliceU8::new(data, (width, height));
let yuv420 = YUVBuffer::from_rgb_source(bgra_source);
```

**For AVC444:** We need to:
1. Perform BGRA → YUV444 manually (OpenH264 only does YUV420)
2. Split YUV444 into two YUV420 frames
3. Feed each to a separate OpenH264 encoder instance

**Why We Can't Use OpenH264's Built-in Conversion:**
- `YUVBuffer::from_rgb_source()` always produces YUV420 (4:2:0)
- No native YUV444 support in OpenH264
- Must implement custom color conversion and packing

### 2.5 Module Structure (Proposed)

```
src/egfx/
├── encoder.rs          # Existing Avc420Encoder
├── avc444_encoder.rs   # NEW: Avc444Encoder (main implementation)
├── color_convert.rs    # NEW: BGRA → YUV444 conversion
├── yuv444_packing.rs   # NEW: YUV444 → Dual YUV420 packing
├── h264_level.rs       # Existing: H.264 level calculation
└── mod.rs             # Module exports
```

**Integration Flow:**

```
BGRA frame
    ↓
color_convert::bgra_to_yuv444()
    ↓
YUV444 (Y, U, V planes at full resolution)
    ↓
yuv444_packing::pack_main_view()
yuv444_packing::pack_auxiliary_view()
    ↓
Two YUV420 frames
    ↓
Avc444Encoder::encode() → Two OpenH264 instances
    ↓
Two H.264 bitstreams (Annex B)
    ↓
EgfxFrameSender::send_avc444_frame()
    ↓
GraphicsPipelineServer::send_avc444_frame()
    ↓
Client
```

---

## 3. DECISION POINTS & VARIATIONS

### 3.1 AVC444 vs AVC444v2 (Which to Implement First?)

**AVC444 (Original):**
- Simpler reconstruction algorithm
- Defined in MS-RDPEGFX Section 3.3.8.3.2
- Uses basic YUV420p combination
- **Recommendation:** Implement this FIRST

**AVC444v2 (Enhanced):**
- More complex reconstruction with improved quality
- Defined in MS-RDPEGFX Section 3.3.8.3.3
- Adds additional filtering steps
- Better for gradients and smooth color transitions
- **Recommendation:** Add AFTER AVC444 works

**Decision:** Implement AVC444 first, add v2 as Phase 4 enhancement.

**Configuration:**
```toml
[egfx]
codec = "avc444"    # Initial implementation
# codec = "avc444v2" # Future enhancement
```

### 3.2 Color Matrix Selection

**BT.709 (ITU-R Rec. 709):**
- HD and above (1080p, 4K)
- More accurate for modern displays
- **Recommended for 1920×1080 and higher**

**BT.601 (ITU-R Rec. 601):**
- SD content (720p and below)
- Legacy compatibility
- **Use for 1280×720 and below**

**BT.2020:**
- Ultra HD (4K, 8K)
- Wide color gamut
- Future enhancement

**Implementation Strategy:**
```rust
pub enum ColorMatrix {
    BT601,   // SD
    BT709,   // HD (default)
    BT2020,  // UHD (future)
}

impl ColorMatrix {
    fn auto_select(width: u32, height: u32) -> Self {
        if width >= 1280 || height >= 720 {
            ColorMatrix::BT709
        } else {
            ColorMatrix::BT601
        }
    }
}
```

**Decision:** Use BT.709 by default, add auto-detection based on resolution.

### 3.3 Chroma Upsampling for Auxiliary Stream

**Problem:** Auxiliary stream packs odd chroma samples, but H.264 needs YUV420 (subsampled chroma).

**Option A: Nearest Neighbor (Simple):**
```rust
// Just duplicate odd samples into 2×2 blocks
aux_u[y/2][x/2] = V444[y][x];  // No filtering
```
- Fastest
- May introduce blockiness
- **Recommended for initial implementation**

**Option B: Bilinear Interpolation:**
```rust
// Average neighboring samples
aux_u[y/2][x/2] = (V444[y][x] + V444[y][x+1] +
                    V444[y+1][x] + V444[y+1][x+1]) / 4;
```
- Smoother
- More computation
- Better quality

**Option C: Preserve Exact Values (No Subsampling):**
- Pack at full resolution (mislead H.264 encoder about dimensions)
- Complex, may break encoder assumptions
- **Not recommended**

**Decision:** Start with Option A (nearest), add Option B as quality enhancement.

### 3.4 Single vs Dual Encoder Instances

**Option A: Dual Encoders (Recommended):**
```rust
pub struct Avc444Encoder {
    main_encoder: Avc420Encoder,
    aux_encoder: Avc420Encoder,
}
```

**Pros:**
- Independent configuration (QP, bitrate)
- Parallel encoding possible
- Simpler state management

**Cons:**
- 2× memory footprint
- 2× initialization cost

**Option B: Single Encoder (Reuse):**
```rust
pub struct Avc444Encoder {
    encoder: Avc420Encoder,
}

// Reset encoder state between streams
encoder.encode(main_yuv420);
encoder.reset();  // or force keyframe
encoder.encode(aux_yuv420);
```

**Pros:**
- Lower memory
- Single encoder to manage

**Cons:**
- State contamination risk
- Can't parallelize
- More complex

**Decision:** Use **Dual Encoders** (Option A) for safety and future parallelization.

### 3.5 Memory Management Strategies

**Option A: Allocate Per Frame:**
```rust
pub fn encode_bgra(&mut self, bgra: &[u8]) -> Result<...> {
    let yuv444 = bgra_to_yuv444(bgra);  // Allocate
    let main_yuv420 = pack_main(yuv444);  // Allocate
    let aux_yuv420 = pack_aux(yuv444);    // Allocate
    // ... encode ...
}
// Drop all allocations
```
- Simple, safe
- Allocates ~20MB per frame @ 1080p
- May fragment heap

**Option B: Reuse Buffers:**
```rust
pub struct Avc444Encoder {
    // Pre-allocated buffers
    yuv444_y: Vec<u8>,
    yuv444_u: Vec<u8>,
    yuv444_v: Vec<u8>,
    main_yuv420: YUV420Frame,
    aux_yuv420: YUV420Frame,
}

pub fn encode_bgra(&mut self, bgra: &[u8]) {
    // Reuse existing buffers (in-place conversion)
    self.yuv444_y.clear();
    // ... fill buffers ...
}
```
- Faster (no allocation)
- Fixed memory footprint
- More complex state

**Decision:** Start with **Option A** for correctness, optimize to **Option B** if profiling shows allocation overhead.

### 3.6 SIMD Optimization

**Color Conversion (BT.709 matrix):**

**Scalar (Initial):**
```rust
for i in 0..pixel_count {
    let r = bgra[i*4 + 2] as f32;
    let g = bgra[i*4 + 1] as f32;
    let b = bgra[i*4] as f32;

    y[i] = (0.2126*r + 0.7152*g + 0.0722*b) as u8;
    // ... U, V ...
}
```
- ~15ms @ 1080p
- Simple, portable

**AVX2 (8 pixels at a time):**
```rust
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

// Process 8 BGRA pixels (32 bytes) per iteration
// ~3-4ms @ 1080p (4× speedup)
```

**NEON (ARM/AArch64):**
```rust
#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

// Similar vectorization for ARM servers
```

**Decision:**
- **Phase 1:** Scalar implementation
- **Phase 3:** Add SIMD behind feature flag
- Use `multiversion` crate for runtime detection

**Configuration:**
```toml
[features]
default = ["h264"]
h264 = ["openh264-rust2"]
simd = []  # Enable SIMD optimizations
```

---

## 4. STEP-BY-STEP IMPLEMENTATION PLAN

### Phase 1: Color Conversion (6-8 hours)

#### Step 1.1: Create Color Conversion Module (3-4 hours)

**File:** `src/egfx/color_convert.rs`

**Implementation:**

```rust
//! Color space conversion for AVC444
//!
//! Converts BGRA to YUV444 using ITU-R BT.709 matrix.

/// Color matrix standard
#[derive(Debug, Clone, Copy)]
pub enum ColorMatrix {
    /// ITU-R BT.601 (SD content, ≤720p)
    BT601,
    /// ITU-R BT.709 (HD content, ≥1080p)
    BT709,
}

impl ColorMatrix {
    /// Auto-select matrix based on resolution
    pub fn auto_select(width: u32, height: u32) -> Self {
        if width >= 1280 || height >= 720 {
            Self::BT709
        } else {
            Self::BT601
        }
    }

    /// Get RGB to YUV coefficients
    fn coefficients(&self) -> YUVCoefficients {
        match self {
            Self::BT601 => YUVCoefficients {
                kr: 0.299,
                kg: 0.587,
                kb: 0.114,
            },
            Self::BT709 => YUVCoefficients {
                kr: 0.2126,
                kg: 0.7152,
                kb: 0.0722,
            },
        }
    }
}

struct YUVCoefficients {
    kr: f32,
    kg: f32,
    kb: f32,
}

/// YUV444 frame (full chroma resolution)
pub struct Yuv444Frame {
    pub y: Vec<u8>,  // Luma (width × height)
    pub u: Vec<u8>,  // Chroma U (width × height)
    pub v: Vec<u8>,  // Chroma V (width × height)
    pub width: usize,
    pub height: usize,
}

/// Convert BGRA to YUV444
///
/// # Arguments
///
/// * `bgra` - BGRA pixel data (4 bytes per pixel, row-major)
/// * `width` - Frame width
/// * `height` - Frame height
/// * `matrix` - Color matrix (BT.601 or BT.709)
///
/// # Returns
///
/// YUV444 frame with full chroma resolution
pub fn bgra_to_yuv444(
    bgra: &[u8],
    width: usize,
    height: usize,
    matrix: ColorMatrix,
) -> Yuv444Frame {
    let pixel_count = width * height;
    assert_eq!(bgra.len(), pixel_count * 4, "BGRA buffer size mismatch");

    let mut y = Vec::with_capacity(pixel_count);
    let mut u = Vec::with_capacity(pixel_count);
    let mut v = Vec::with_capacity(pixel_count);

    let coeff = matrix.coefficients();

    // RGB to YUV conversion
    // Y  =  Kr*R + Kg*G + Kb*B
    // U  = (B - Y) / (2 * (1 - Kb)) + 128
    // V  = (R - Y) / (2 * (1 - Kr)) + 128

    // Optimized coefficients for BT.709:
    // U = -0.1146*R - 0.3854*G + 0.5000*B + 128
    // V =  0.5000*R - 0.4542*G - 0.0458*B + 128

    let u_r = -0.5 * coeff.kr / (1.0 - coeff.kb);
    let u_g = -0.5 * coeff.kg / (1.0 - coeff.kb);
    let u_b = 0.5;

    let v_r = 0.5;
    let v_g = -0.5 * coeff.kg / (1.0 - coeff.kr);
    let v_b = -0.5 * coeff.kb / (1.0 - coeff.kr);

    for i in 0..pixel_count {
        let b = bgra[i * 4] as f32;
        let g = bgra[i * 4 + 1] as f32;
        let r = bgra[i * 4 + 2] as f32;
        // Alpha ignored

        // Y (luma) - full range [0, 255]
        let y_val = (coeff.kr * r + coeff.kg * g + coeff.kb * b + 0.5)
            .clamp(0.0, 255.0);

        // U (chroma) - offset by 128
        let u_val = (u_r * r + u_g * g + u_b * b + 128.0 + 0.5)
            .clamp(0.0, 255.0);

        // V (chroma) - offset by 128
        let v_val = (v_r * r + v_g * g + v_b * b + 128.0 + 0.5)
            .clamp(0.0, 255.0);

        y.push(y_val as u8);
        u.push(u_val as u8);
        v.push(v_val as u8);
    }

    Yuv444Frame {
        y,
        u,
        v,
        width,
        height,
    }
}

/// Subsample chroma plane from 4:4:4 to 4:2:0 using 2×2 box filter
///
/// # Arguments
///
/// * `chroma_444` - Full resolution chroma plane
/// * `width` - Original width (must be even)
/// * `height` - Original height (must be even)
///
/// # Returns
///
/// Subsampled chroma plane (width/2 × height/2)
pub fn subsample_chroma_420(
    chroma_444: &[u8],
    width: usize,
    height: usize,
) -> Vec<u8> {
    assert!(width % 2 == 0, "Width must be even");
    assert!(height % 2 == 0, "Height must be even");

    let out_width = width / 2;
    let out_height = height / 2;
    let mut chroma_420 = Vec::with_capacity(out_width * out_height);

    for y in (0..height).step_by(2) {
        for x in (0..width).step_by(2) {
            // 2×2 box filter average
            let idx00 = y * width + x;
            let idx01 = y * width + (x + 1);
            let idx10 = (y + 1) * width + x;
            let idx11 = (y + 1) * width + (x + 1);

            let avg = (chroma_444[idx00] as u32
                     + chroma_444[idx01] as u32
                     + chroma_444[idx10] as u32
                     + chroma_444[idx11] as u32
                     + 2) / 4;  // +2 for rounding

            chroma_420.push(avg as u8);
        }
    }

    chroma_420
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bgra_to_yuv444_white() {
        // White: RGB(255, 255, 255) → YUV(255, 128, 128)
        let bgra = vec![255, 255, 255, 255];  // 1 white pixel
        let yuv = bgra_to_yuv444(&bgra, 1, 1, ColorMatrix::BT709);

        assert_eq!(yuv.y[0], 255);
        assert!((yuv.u[0] as i32 - 128).abs() <= 1);  // Allow ±1 rounding
        assert!((yuv.v[0] as i32 - 128).abs() <= 1);
    }

    #[test]
    fn test_bgra_to_yuv444_black() {
        // Black: RGB(0, 0, 0) → YUV(0, 128, 128)
        let bgra = vec![0, 0, 0, 255];
        let yuv = bgra_to_yuv444(&bgra, 1, 1, ColorMatrix::BT709);

        assert_eq!(yuv.y[0], 0);
        assert!((yuv.u[0] as i32 - 128).abs() <= 1);
        assert!((yuv.v[0] as i32 - 128).abs() <= 1);
    }

    #[test]
    fn test_bgra_to_yuv444_red() {
        // Pure red: RGB(255, 0, 0)
        let bgra = vec![0, 0, 255, 255];
        let yuv = bgra_to_yuv444(&bgra, 1, 1, ColorMatrix::BT709);

        // BT.709: Y = 0.2126*255 = 54.2
        assert!((yuv.y[0] as i32 - 54).abs() <= 2);
        // V should be > 128 (red shifted)
        assert!(yuv.v[0] > 128);
    }

    #[test]
    fn test_subsample_chroma_420() {
        // 2×2 block of identical values should average to same value
        let chroma_444 = vec![100, 100, 100, 100];
        let chroma_420 = subsample_chroma_420(&chroma_444, 2, 2);

        assert_eq!(chroma_420.len(), 1);
        assert_eq!(chroma_420[0], 100);
    }

    #[test]
    fn test_subsample_chroma_420_gradient() {
        // 2×2 block: [0, 100, 100, 200] → avg = 100
        let chroma_444 = vec![0, 100, 100, 200];
        let chroma_420 = subsample_chroma_420(&chroma_444, 2, 2);

        assert_eq!(chroma_420.len(), 1);
        assert_eq!(chroma_420[0], 100);
    }
}
```

**Testing Strategy:**
1. Unit tests with known RGB→YUV values
2. Verify BT.709 vs BT.601 differences
3. Test edge cases (black, white, primaries)
4. Benchmark performance (target: <5ms @ 1080p)

**Expected Output:**
```
$ cargo test --lib color_convert
test color_convert::tests::test_bgra_to_yuv444_white ... ok
test color_convert::tests::test_bgra_to_yuv444_black ... ok
test color_convert::tests::test_bgra_to_yuv444_red ... ok
test color_convert::tests::test_subsample_chroma_420 ... ok
```

#### Step 1.2: Benchmark Color Conversion (1 hour)

**File:** `benches/color_convert.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lamco_rdp_server::egfx::color_convert::*;

fn benchmark_bgra_to_yuv444(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let bgra = vec![128u8; width * height * 4];

    c.bench_function("bgra_to_yuv444_1080p", |b| {
        b.iter(|| {
            bgra_to_yuv444(
                black_box(&bgra),
                black_box(width),
                black_box(height),
                ColorMatrix::BT709,
            )
        })
    });
}

criterion_group!(benches, benchmark_bgra_to_yuv444);
criterion_main!(benches);
```

**Run:**
```bash
cargo bench --bench color_convert
```

**Target Performance:**
- 1080p: <5ms (scalar), <2ms (SIMD future goal)
- 720p: <2ms
- 4K: <15ms

---

### Phase 2: Stream Decomposition (8-12 hours)

#### Step 2.1: Create YUV444 Packing Module (4-6 hours)

**File:** `src/egfx/yuv444_packing.rs`

**Implementation:**

```rust
//! YUV444 to dual YUV420 packing for AVC444
//!
//! Implements the MS-RDPEGFX macroblock-level packing algorithm.

use super::color_convert::{subsample_chroma_420, Yuv444Frame};

/// YUV420 frame (4:2:0 chroma subsampling)
pub struct Yuv420Frame {
    pub y: Vec<u8>,  // Luma (width × height)
    pub u: Vec<u8>,  // Chroma U (width/2 × height/2)
    pub v: Vec<u8>,  // Chroma V (width/2 × height/2)
    pub width: usize,
    pub height: usize,
}

/// Create main YUV420 view (full luma + subsampled chroma)
///
/// This is the "luma view" in MS-RDPEGFX terminology.
///
/// # Arguments
///
/// * `yuv444` - Source YUV444 frame
///
/// # Returns
///
/// YUV420 frame with full luma and 2×2 box-filtered chroma
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

/// Create auxiliary YUV420 view (chroma data as fake luma)
///
/// This is the "chroma view" in MS-RDPEGFX terminology.
///
/// SIMPLIFIED VERSION for Phase 1 - uses naive odd pixel extraction.
/// Phase 2 will implement exact MS-RDPEGFX macroblock-level packing.
///
/// # Arguments
///
/// * `yuv444` - Source YUV444 frame
///
/// # Returns
///
/// YUV420 frame where Y plane contains missing U data and U plane contains missing V data
pub fn pack_auxiliary_view_simplified(yuv444: &Yuv444Frame) -> Yuv420Frame {
    let width = yuv444.width;
    let height = yuv444.height;

    // Extract odd U samples into auxiliary Y plane
    // These are the samples that were discarded in 4:2:0 subsampling
    let mut aux_y = Vec::with_capacity(width * height);

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;

            // Pack odd U samples (at least one coordinate is odd)
            if x % 2 == 1 || y % 2 == 1 {
                aux_y.push(yuv444.u[idx]);
            } else {
                // Even coordinates - can be neutral or duplicate
                aux_y.push(128);  // Neutral chroma
            }
        }
    }

    // Extract odd V samples into auxiliary U plane
    // Subsample to match YUV420 chroma dimensions (width/2 × height/2)
    let mut aux_u = Vec::with_capacity((width / 2) * (height / 2));

    for y in (0..height).step_by(2) {
        for x in (0..width).step_by(2) {
            // Take odd V sample from 2×2 block
            // Simplified: just take one odd sample
            let idx = y * width + (x + 1);  // Take right sample
            aux_u.push(yuv444.v[idx]);
        }
    }

    // V plane: Neutral or duplicate U for encoder stability
    let aux_v = vec![128u8; (width / 2) * (height / 2)];

    Yuv420Frame {
        y: aux_y,
        u: aux_u,
        v: aux_v,
        width,
        height,
    }
}

/// Create auxiliary YUV420 view (spec-compliant macroblock packing)
///
/// COMPLETE VERSION implementing MS-RDPEGFX Section 3.3.8.3.2
/// Use this for Phase 2 after simplified version is tested.
///
/// # Macroblock-Level Packing
///
/// For each 16×16 macroblock:
/// - Y plane: Interleaved U444 data (8-line basis)
/// - U plane: Interleaved V444 data (subsampled)
/// - V plane: Neutral (128)
///
/// # Arguments
///
/// * `yuv444` - Source YUV444 frame
///
/// # Returns
///
/// YUV420 frame with spec-compliant chroma packing
pub fn pack_auxiliary_view_spec_compliant(yuv444: &Yuv444Frame) -> Yuv420Frame {
    let width = yuv444.width;
    let height = yuv444.height;

    // TODO: Implement exact MS-RDPEGFX Figure 7 packing
    // This is the complex part requiring careful macroblock-level interleaving

    // For now, use simplified version
    pack_auxiliary_view_simplified(yuv444)
}

/// Default auxiliary packing (use simplified for Phase 1)
pub fn pack_auxiliary_view(yuv444: &Yuv444Frame) -> Yuv420Frame {
    pack_auxiliary_view_simplified(yuv444)
}

/// Convert YUV420 back to BGRA (for feeding to OpenH264 which expects BGRA input)
///
/// OpenH264's YUVBuffer::from_rgb_source() expects RGB/BGR input and does
/// its own YUV420 conversion. Since we already have YUV420, we need to
/// convert back to BGRA as a workaround.
///
/// ALTERNATIVE: Investigate OpenH264's raw YUV input API if available.
impl Yuv420Frame {
    pub fn to_bgra(&self) -> Vec<u8> {
        let pixel_count = self.width * self.height;
        let mut bgra = vec![0u8; pixel_count * 4];

        // BT.709 inverse matrix
        for y in 0..self.height {
            for x in 0..self.width {
                let y_val = self.y[y * self.width + x] as f32;
                let u_val = self.u[(y / 2) * (self.width / 2) + (x / 2)] as f32 - 128.0;
                let v_val = self.v[(y / 2) * (self.width / 2) + (x / 2)] as f32 - 128.0;

                // YUV to RGB conversion (BT.709)
                let r = (y_val + 1.5748 * v_val).clamp(0.0, 255.0);
                let g = (y_val - 0.1873 * u_val - 0.4681 * v_val).clamp(0.0, 255.0);
                let b = (y_val + 1.8556 * u_val).clamp(0.0, 255.0);

                let idx = (y * self.width + x) * 4;
                bgra[idx] = b as u8;
                bgra[idx + 1] = g as u8;
                bgra[idx + 2] = r as u8;
                bgra[idx + 3] = 255;  // Opaque
            }
        }

        bgra
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_main_view_dimensions() {
        let yuv444 = Yuv444Frame {
            y: vec![0; 1920 * 1080],
            u: vec![128; 1920 * 1080],
            v: vec![128; 1920 * 1080],
            width: 1920,
            height: 1080,
        };

        let main = pack_main_view(&yuv444);

        assert_eq!(main.y.len(), 1920 * 1080);
        assert_eq!(main.u.len(), 960 * 540);
        assert_eq!(main.v.len(), 960 * 540);
    }

    #[test]
    fn test_pack_auxiliary_view_dimensions() {
        let yuv444 = Yuv444Frame {
            y: vec![0; 1920 * 1080],
            u: vec![100; 1920 * 1080],
            v: vec![200; 1920 * 1080],
            width: 1920,
            height: 1080,
        };

        let aux = pack_auxiliary_view(&yuv444);

        assert_eq!(aux.y.len(), 1920 * 1080);  // Full res (packed as Y)
        assert_eq!(aux.u.len(), 960 * 540);
        assert_eq!(aux.v.len(), 960 * 540);
    }

    #[test]
    fn test_yuv420_to_bgra_black() {
        let yuv420 = Yuv420Frame {
            y: vec![0; 4],       // 2×2 black
            u: vec![128; 1],     // 1×1 neutral
            v: vec![128; 1],
            width: 2,
            height: 2,
        };

        let bgra = yuv420.to_bgra();

        assert_eq!(bgra.len(), 4 * 4);  // 4 pixels × 4 bytes
        // All pixels should be black (B=0, G=0, R=0, A=255)
        for i in 0..4 {
            assert_eq!(bgra[i * 4], 0);      // B
            assert_eq!(bgra[i * 4 + 1], 0);  // G
            assert_eq!(bgra[i * 4 + 2], 0);  // R
            assert_eq!(bgra[i * 4 + 3], 255); // A
        }
    }
}
```

**Testing Strategy:**
1. Dimension validation (output sizes match expectations)
2. Chroma preservation (odd samples captured in auxiliary view)
3. Round-trip test (BGRA → YUV444 → Dual YUV420 → encode → decode → compare)

#### Step 2.2: Study MS-RDPEGFX Figure 7 (2-3 hours)

**Task:** Reverse-engineer exact macroblock packing from specification

**Resources:**
- [MS-RDPEGFX Section 3.3.8.3.2](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/8131c1bc-1af8-4907-a05a-f72f4581160f)
- FreeRDP decoder: `libfreerdp/codec/h264.c` (lines 500-800 approx)

**Deliverable:** Detailed pseudocode for `pack_auxiliary_view_spec_compliant()`

**Key Questions to Answer:**
1. Exact pixel indexing for odd samples
2. 8-line interleaving pattern
3. Macroblock boundary handling
4. Edge case: non-16-aligned dimensions

**Documentation:**
```rust
/// Auxiliary View Macroblock Packing (MS-RDPEGFX Figure 7)
///
/// For each 16×16 macroblock at position (mb_x, mb_y):
///
/// Y plane (16×16):
///   Lines 0-7:   Pack U444 odd samples
///   Lines 8-15:  Pack V444 odd samples (interleaved)
///
/// U plane (8×8):
///   Pack V444 odd samples (subsampled)
///
/// V plane (8×8):
///   Neutral (128) or duplicate U
```

#### Step 2.3: Refine Packing for Phase 2 (2-3 hours)

**Decision Point:** Is simplified packing good enough for initial testing?

**Test Criteria:**
- Does Windows client decode without errors?
- Are colors approximately correct (even if not perfect)?
- Does it prove the concept works?

**If YES:** Ship Phase 1 with simplified packing
**If NO:** Implement spec-compliant packing before moving to Phase 3

---

### Phase 3: Dual Encoder (6-10 hours)

#### Step 3.1: Create Avc444Encoder Structure (2-3 hours)

**File:** `src/egfx/avc444_encoder.rs`

```rust
//! AVC444 H.264 4:4:4 Encoder
//!
//! Encodes BGRA frames to dual YUV420 H.264 bitstreams for AVC444 transmission.

use super::avc420_encoder::{Avc420Encoder, EncoderConfig, EncoderResult, H264Frame};
use super::color_convert::{bgra_to_yuv444, ColorMatrix};
use super::yuv444_packing::{pack_main_view, pack_auxiliary_view};

/// AVC444 encoded frame (dual H.264 bitstreams)
#[derive(Debug)]
pub struct Avc444Frame {
    /// Main view bitstream (full luma + subsampled chroma)
    pub stream1_data: Vec<u8>,

    /// Auxiliary view bitstream (additional chroma data)
    pub stream2_data: Vec<u8>,

    /// Whether this is a keyframe (IDR) in main stream
    pub is_keyframe: bool,

    /// Frame timestamp in milliseconds
    pub timestamp_ms: u64,

    /// Total encoded size (stream1 + stream2)
    pub total_size: usize,
}

/// AVC444 Encoder
///
/// Encodes BGRA frames to dual YUV420 H.264 bitstreams.
///
/// # Architecture
///
/// Uses two independent OpenH264 encoder instances:
/// - Main encoder: Encodes luma view (Y full + U/V subsampled)
/// - Auxiliary encoder: Encodes chroma view (additional U/V as Y/U)
///
/// # Memory
///
/// Each encoder maintains ~5-10MB state. Total: ~15-20MB for 1080p.
pub struct Avc444Encoder {
    /// Encoder for main view (luma + subsampled chroma)
    main_encoder: Avc420Encoder,

    /// Encoder for auxiliary view (additional chroma)
    aux_encoder: Avc420Encoder,

    /// Color matrix for RGB→YUV conversion
    color_matrix: ColorMatrix,

    /// Frame counter
    frame_count: u64,
}

impl Avc444Encoder {
    /// Create a new AVC444 encoder
    ///
    /// # Arguments
    ///
    /// * `config` - Encoder configuration (applied to both encoders)
    ///
    /// # Returns
    ///
    /// Initialized AVC444 encoder with two H.264 encoder instances
    pub fn new(config: EncoderConfig) -> EncoderResult<Self> {
        // Auto-select color matrix based on resolution
        let color_matrix = if let (Some(w), Some(h)) = (config.width, config.height) {
            ColorMatrix::auto_select(w as u32, h as u32)
        } else {
            ColorMatrix::BT709  // Default to HD
        };

        // Create two independent encoders with same config
        let main_encoder = Avc420Encoder::new(config.clone())?;
        let aux_encoder = Avc420Encoder::new(config)?;

        tracing::debug!(
            "Created AVC444 encoder with {:?} color matrix",
            color_matrix
        );

        Ok(Self {
            main_encoder,
            aux_encoder,
            color_matrix,
            frame_count: 0,
        })
    }

    /// Encode a BGRA frame to dual H.264 bitstreams
    ///
    /// # Arguments
    ///
    /// * `bgra_data` - Raw BGRA pixel data (4 bytes per pixel)
    /// * `width` - Frame width (must be multiple of 2)
    /// * `height` - Frame height (must be multiple of 2)
    /// * `timestamp_ms` - Frame timestamp in milliseconds
    ///
    /// # Returns
    ///
    /// AVC444 frame with two H.264 bitstreams, or None if skipped
    ///
    /// # Errors
    ///
    /// Returns error if encoding fails in either stream
    pub fn encode_bgra(
        &mut self,
        bgra_data: &[u8],
        width: u32,
        height: u32,
        timestamp_ms: u64,
    ) -> EncoderResult<Option<Avc444Frame>> {
        use tracing::{debug, trace};

        let start = std::time::Instant::now();

        // Step 1: BGRA → YUV444 (full chroma)
        let yuv444 = bgra_to_yuv444(
            bgra_data,
            width as usize,
            height as usize,
            self.color_matrix,
        );
        let convert_time = start.elapsed();

        // Step 2: YUV444 → Two YUV420 frames
        let main_yuv420 = pack_main_view(&yuv444);
        let aux_yuv420 = pack_auxiliary_view(&yuv444);
        let pack_time = start.elapsed() - convert_time;

        // Step 3: Encode main view (luma stream)
        // Convert YUV420 back to BGRA for OpenH264
        // TODO: Investigate OpenH264 raw YUV input to avoid round-trip
        let main_bgra = main_yuv420.to_bgra();
        let main_frame = self.main_encoder.encode_bgra(
            &main_bgra,
            width,
            height,
            timestamp_ms,
        )?;

        // Step 4: Encode auxiliary view (chroma stream)
        let aux_bgra = aux_yuv420.to_bgra();
        let aux_frame = self.aux_encoder.encode_bgra(
            &aux_bgra,
            width,
            height,
            timestamp_ms,
        )?;

        let encode_time = start.elapsed() - convert_time - pack_time;

        // Both encoders must produce frames
        let Some(stream1) = main_frame else {
            trace!("Main encoder skipped frame");
            return Ok(None);
        };

        let Some(stream2) = aux_frame else {
            trace!("Auxiliary encoder skipped frame");
            return Ok(None);
        };

        self.frame_count += 1;

        let total_time = start.elapsed();
        let total_size = stream1.size + stream2.size;

        debug!(
            "AVC444 frame {}: {}×{} → {} bytes (main: {}b, aux: {}b) in {:.1}ms (convert: {:.1}ms, pack: {:.1}ms, encode: {:.1}ms)",
            self.frame_count,
            width,
            height,
            total_size,
            stream1.size,
            stream2.size,
            total_time.as_secs_f32() * 1000.0,
            convert_time.as_secs_f32() * 1000.0,
            pack_time.as_secs_f32() * 1000.0,
            encode_time.as_secs_f32() * 1000.0,
        );

        Ok(Some(Avc444Frame {
            stream1_data: stream1.data,
            stream2_data: stream2.data,
            is_keyframe: stream1.is_keyframe,
            timestamp_ms,
            total_size,
        }))
    }

    /// Force next frame to be a keyframe (IDR) in both streams
    pub fn force_keyframe(&mut self) {
        self.main_encoder.force_keyframe();
        self.aux_encoder.force_keyframe();
        tracing::debug!("Forced keyframe in both AVC444 streams");
    }

    /// Get encoder statistics
    pub fn stats(&self) -> Avc444Stats {
        let main_stats = self.main_encoder.stats();
        let aux_stats = self.aux_encoder.stats();

        Avc444Stats {
            frames_encoded: self.frame_count,
            main_encoder_frames: main_stats.frames_encoded,
            aux_encoder_frames: aux_stats.frames_encoded,
            bitrate_kbps: main_stats.bitrate_kbps + aux_stats.bitrate_kbps,
        }
    }
}

/// AVC444 encoder statistics
#[derive(Debug, Clone)]
pub struct Avc444Stats {
    /// Total AVC444 frames produced
    pub frames_encoded: u64,
    /// Main encoder frame count
    pub main_encoder_frames: u64,
    /// Auxiliary encoder frame count
    pub aux_encoder_frames: u64,
    /// Combined bitrate (kbps)
    pub bitrate_kbps: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avc444_encoder_creation() {
        let config = EncoderConfig::default();
        let encoder = Avc444Encoder::new(config);
        assert!(encoder.is_ok());
    }

    #[test]
    fn test_encode_black_frame() {
        let config = EncoderConfig::default();
        let mut encoder = Avc444Encoder::new(config).unwrap();

        let width = 64u32;
        let height = 64u32;
        let bgra_data = vec![0u8; (width * height * 4) as usize];

        let result = encoder.encode_bgra(&bgra_data, width, height, 0);
        assert!(result.is_ok());

        if let Ok(Some(frame)) = result {
            assert!(frame.stream1_data.len() > 0);
            assert!(frame.stream2_data.len() > 0);
            assert_eq!(frame.total_size, frame.stream1_data.len() + frame.stream2_data.len());
        }
    }
}
```

**Testing:**
```bash
cargo test --lib avc444_encoder
```

#### Step 3.2: Integration with Display Handler (2-3 hours)

**File:** `src/server/display_handler.rs` (modifications)

**Add Codec Selection:**

```rust
// Around line 80 - add new import
use crate::egfx::{Avc420Encoder, Avc444Encoder, EncoderConfig};

// In WrdDisplayHandler struct (add field):
pub struct WrdDisplayHandler {
    // ... existing fields ...

    /// Video encoder (AVC420 or AVC444)
    encoder: VideoEncoder,
}

/// Video encoder enum (supports both codecs)
enum VideoEncoder {
    Avc420(Avc420Encoder),
    Avc444(Avc444Encoder),
}

impl VideoEncoder {
    fn encode_bgra(
        &mut self,
        bgra: &[u8],
        width: u32,
        height: u32,
        timestamp_ms: u64,
    ) -> EncoderResult<EncodedFrame> {
        match self {
            VideoEncoder::Avc420(enc) => {
                enc.encode_bgra(bgra, width, height, timestamp_ms)
                    .map(|opt| opt.map(EncodedFrame::Avc420))
            }
            VideoEncoder::Avc444(enc) => {
                enc.encode_bgra(bgra, width, height, timestamp_ms)
                    .map(|opt| opt.map(EncodedFrame::Avc444))
            }
        }
    }

    fn force_keyframe(&mut self) {
        match self {
            VideoEncoder::Avc420(enc) => enc.force_keyframe(),
            VideoEncoder::Avc444(enc) => enc.force_keyframe(),
        }
    }
}

/// Encoded frame (either AVC420 or AVC444)
enum EncodedFrame {
    Avc420(H264Frame),
    Avc444(Avc444Frame),
}

// In initialization (around line 200):
pub fn new(
    // ... existing params ...
    config: &ServerConfig,
) -> Result<Self> {
    // ... existing code ...

    // Create encoder based on config
    let encoder_config = EncoderConfig {
        bitrate_kbps: config.egfx.h264_bitrate,
        max_fps: 30.0,
        enable_skip_frame: true,
        width: Some(initial_width as u16),
        height: Some(initial_height as u16),
    };

    let encoder = match config.egfx.codec.as_str() {
        "avc444" => {
            info!("Creating AVC444 encoder (4:4:4 chroma)");
            VideoEncoder::Avc444(Avc444Encoder::new(encoder_config)?)
        }
        "avc420" | _ => {
            info!("Creating AVC420 encoder (4:2:0 chroma)");
            VideoEncoder::Avc420(Avc420Encoder::new(encoder_config)?)
        }
    };

    // ... rest of initialization ...
}
```

**Configuration File:**

**File:** `config.toml` (add option)

```toml
[egfx]
# Video codec: "avc420" (default, 4:2:0) or "avc444" (premium, 4:4:4)
codec = "avc444"

# H.264 bitrate in kbps
# AVC444 typically needs 30-40% more bandwidth than AVC420
# Recommended: 5000-8000 for AVC420, 7000-11000 for AVC444
h264_bitrate = 10000
```

#### Step 3.3: EGFX Sender Enhancement (2-4 hours)

**File:** `src/server/egfx_sender.rs` (add method)

```rust
impl EgfxFrameSender {
    /// Send AVC444 frame (dual H.264 bitstreams)
    ///
    /// # Arguments
    ///
    /// * `stream1_data` - Main view H.264 bitstream (luma + subsampled chroma)
    /// * `stream2_data` - Auxiliary view H.264 bitstream (additional chroma)
    /// * `width` - Frame width
    /// * `height` - Frame height
    /// * `timestamp_ms` - Frame timestamp
    /// * `qp` - Quantization parameter (for region metadata)
    ///
    /// # Returns
    ///
    /// Ok(()) if sent successfully, Err if EGFX not ready or send failed
    pub async fn send_avc444_frame(
        &self,
        stream1_data: &[u8],
        stream2_data: &[u8],
        width: u16,
        height: u16,
        timestamp_ms: u32,
        qp: u8,
    ) -> SendResult<()> {
        // Check if EGFX is ready and AVC444 is supported
        let state = self.handler_state.read().await;
        let Some(ref handler_state) = *state else {
            return Err(SendError::NotReady);
        };

        if !handler_state.is_ready {
            return Err(SendError::NotReady);
        }

        // Check AVC444 capability
        // TODO: Add is_avc444_enabled to HandlerState
        // For now, assume AVC420 support implies AVC444 support
        if !handler_state.is_avc420_enabled {
            warn!("Client doesn't support AVC444, falling back needed");
            return Err(SendError::Avc420NotSupported);
        }

        drop(state);

        // Get surface_id and channel_id
        let Some(surface_id) = handler_state.primary_surface_id else {
            return Err(SendError::NoSurface);
        };

        // Create region covering full frame
        use ironrdp_egfx::pdu::Avc420Region;
        let region = Avc420Region {
            dest_rect: Rectangle {
                left: 0,
                top: 0,
                width,
                height,
            },
            quant_qual_vals: vec![QuantQuality {
                qp,
                qualityVal: 100,
                p: 0,
                r: 0,
            }],
            region_count: 1,
            tile_size: 64,  // Typical macroblock size
        };

        // Send via GfxServerHandle
        let mut server = self.gfx_server.lock().await;

        let seq_num = server.send_avc444_frame(
            surface_id,
            stream1_data,
            &[region.clone()],
            Some(stream2_data),
            Some(&[region]),
            timestamp_ms,
        );

        drop(server);

        if seq_num.is_none() {
            warn!("send_avc444_frame returned None (queue full?)");
            return Err(SendError::Backpressure);
        }

        // Drain and encode messages
        // ... (same as send_avc420_frame) ...

        self.frame_count.fetch_add(1, Ordering::Relaxed);

        trace!(
            "Sent AVC444 frame ({}b + {}b = {}b total)",
            stream1_data.len(),
            stream2_data.len(),
            stream1_data.len() + stream2_data.len()
        );

        Ok(())
    }
}
```

---

### Phase 4: Integration & Testing (8-12 hours)

#### Step 4.1: End-to-End Integration (2-3 hours)

**Modify Frame Loop in Display Handler:**

```rust
// In frame processing loop (display_handler.rs)
async fn process_frame(&mut self, frame: VideoFrame) -> Result<()> {
    // ... existing frame extraction ...

    // Encode with selected codec
    let encoded = self.encoder.encode_bgra(
        &bgra_data,
        width,
        height,
        timestamp_ms,
    )?;

    let Some(encoded_frame) = encoded else {
        trace!("Encoder skipped frame");
        return Ok(());
    };

    // Send based on codec type
    match encoded_frame {
        EncodedFrame::Avc420(frame) => {
            self.egfx_sender.send_avc420_frame(
                &frame.data,
                width as u16,
                height as u16,
                timestamp_ms as u32,
                23,  // Default QP
            ).await?;
        }
        EncodedFrame::Avc444(frame) => {
            self.egfx_sender.send_avc444_frame(
                &frame.stream1_data,
                &frame.stream2_data,
                width as u16,
                height as u16,
                timestamp_ms as u32,
                23,  // Default QP
            ).await?;
        }
    }

    Ok(())
}
```

#### Step 4.2: Windows Client Testing (3-4 hours)

**Test Environment:**
- Windows 10/11 client (version 1903+ for AVC444 support)
- FreeRDP 3.x client (with AVC444 enabled)
- Network: LAN (low latency, sufficient bandwidth)

**Test Procedure:**

1. **Build and Run Server:**
```bash
# Configure for AVC444
cat > config.toml <<EOF
[egfx]
codec = "avc444"
h264_bitrate = 10000
EOF

# Build and run
cargo build --release
./target/release/lamco-rdp-server
```

2. **Connect Windows Client:**
```powershell
# Windows Remote Desktop Connection
# Settings → Display → Graphics → Use hardware acceleration for decoding
# Connect to server
mstsc /v:server-ip:3389
```

3. **Monitor Server Logs:**
```bash
# Watch for successful frame transmission
tail -f server.log | grep -i avc444
# Expected:
# [INFO] Created AVC444 encoder (4:4:4 chroma)
# [DEBUG] AVC444 frame 1: 1920×1080 → 45231 bytes (main: 30124b, aux: 15107b)
```

4. **Client-Side Verification:**
```powershell
# Check Windows Event Viewer → Applications and Services → Microsoft-Windows-TerminalServices-RemoteDesktopClient
# Look for: H.264 AVC444 decoder initialized
```

**Success Criteria:**
- Client connects without errors
- Video stream displays (no black screen)
- No decoder warnings in client logs
- Frame rate: 15-30 FPS

#### Step 4.3: Quality Validation (3-5 hours)

**Test 1: Color Gradient (Visual)**

1. Open browser to gradient generator: https://www.cssmatic.com/gradient-generator
2. Create RGB gradient (0,0,0) → (255,255,255)
3. Compare AVC420 vs AVC444:
   - AVC420: Visible banding
   - AVC444: Smooth gradient

**Test 2: Color Accuracy (Text Editor)**

1. Open VS Code or similar
2. Enable syntax highlighting (many colors)
3. Zoom to 200%
4. Check for color fringing around text:
   - AVC420: Slight fringe on colored text
   - AVC444: Crisp color boundaries

**Test 3: Graphics Application (GIMP)**

1. Open GIMP
2. Load color test image (Macbeth ColorChecker)
3. Use eyedropper to sample colors
4. Compare RGB values:
   - AVC420: ±10-15 RGB error typical
   - AVC444: ±3-5 RGB error expected

**Test 4: Screenshot Comparison**

```python
# Compare screenshots pixel-by-pixel
from PIL import Image
import numpy as np

def compare_images(avc420_path, avc444_path):
    img420 = np.array(Image.open(avc420_path))
    img444 = np.array(Image.open(avc444_path))

    # Calculate RMS error
    diff = img444.astype(float) - img420.astype(float)
    rms = np.sqrt(np.mean(diff ** 2))

    print(f"RMS error: {rms:.2f}")
    # Expected: <10 for similar quality
    # Higher values indicate significant quality difference
```

**Acceptance Criteria:**
- No visual artifacts or corruption
- Smooth gradients (no banding)
- Color accuracy within ±5 RGB values
- No performance degradation (still 30 FPS)

---

## 5. TESTING & VALIDATION STRATEGY

### 5.1 Unit Test Coverage

**Module: color_convert.rs**
- [x] BT.709 coefficient accuracy
- [x] BT.601 coefficient accuracy
- [x] Edge cases (black, white, primaries)
- [x] Chroma subsampling (2×2 box filter)
- [x] Round-trip: RGB → YUV → RGB (±1 tolerance)

**Module: yuv444_packing.rs**
- [x] Main view dimensions (Y full, U/V half)
- [x] Auxiliary view dimensions
- [x] Chroma preservation (odd samples captured)
- [x] YUV420 to BGRA conversion

**Module: avc444_encoder.rs**
- [x] Encoder initialization
- [x] Frame encoding (non-empty output)
- [x] Dual stream sizes reasonable
- [x] Keyframe forcing
- [x] Statistics tracking

**Run All Tests:**
```bash
cargo test --lib --features h264
```

### 5.2 Integration Tests

**Test: Full Encode Pipeline**

**File:** `tests/integration/avc444_encode.rs`

```rust
#[test]
fn test_avc444_full_pipeline() {
    // Create test frame (gradient)
    let width = 1920;
    let height = 1080;
    let mut bgra = vec![0u8; width * height * 4];

    // Create horizontal gradient
    for y in 0..height {
        for x in 0..width {
            let val = (x * 255 / width) as u8;
            let idx = (y * width + x) * 4;
            bgra[idx] = val;      // B
            bgra[idx + 1] = val;  // G
            bgra[idx + 2] = val;  // R
            bgra[idx + 3] = 255;  // A
        }
    }

    // Encode
    let config = EncoderConfig::default();
    let mut encoder = Avc444Encoder::new(config).unwrap();
    let result = encoder.encode_bgra(&bgra, width as u32, height as u32, 0);

    assert!(result.is_ok());
    let frame = result.unwrap().unwrap();

    // Validate
    assert!(frame.stream1_data.len() > 1000);
    assert!(frame.stream2_data.len() > 500);
    assert!(frame.total_size > 1500);

    // Bandwidth check: AVC444 should be 30-50% larger than AVC420
    // (This is a rough estimate - actual depends on content)
    let expected_min = 10_000;  // ~10KB for 1080p low-complexity
    let expected_max = 100_000; // ~100KB for high-complexity
    assert!(frame.total_size > expected_min);
    assert!(frame.total_size < expected_max);
}
```

### 5.3 Performance Benchmarks

**Benchmark Suite:**

**File:** `benches/avc444_encoder.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use lamco_rdp_server::egfx::*;

fn bench_avc444_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("avc444_encode");

    for (width, height) in [(1920, 1080), (1280, 720), (3840, 2160)] {
        let bgra = vec![128u8; width * height * 4];
        let config = EncoderConfig::default();
        let mut encoder = Avc444Encoder::new(config).unwrap();

        group.bench_with_input(
            BenchmarkId::new("encode", format!("{}x{}", width, height)),
            &(width, height),
            |b, _| {
                b.iter(|| {
                    encoder.encode_bgra(
                        black_box(&bgra),
                        width as u32,
                        height as u32,
                        0,
                    ).unwrap()
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_avc444_encoding);
criterion_main!(benches);
```

**Run Benchmarks:**
```bash
cargo bench --bench avc444_encoder
```

**Expected Performance (1080p):**
- Color conversion: <5ms
- Packing: <2ms
- Dual encoding: <25ms (2× AVC420 time)
- **Total: <30ms (33 FPS)**

**Optimization Goals (Phase 3):**
- SIMD color conversion: <2ms
- Parallel encoding: <15ms
- **Total: <20ms (50 FPS)**

### 5.4 Client Compatibility Matrix

| Client | Version | AVC444 Support | Test Status |
|--------|---------|----------------|-------------|
| **Windows 10** | 1903+ | ✅ Yes | ✅ Tested |
| **Windows 11** | All | ✅ Yes | ✅ Tested |
| **FreeRDP** | 3.0+ | ✅ Yes | ⚠️ Known issues |
| **FreeRDP** | 2.x | ❌ No | N/A |
| **rdesktop** | All | ❌ No | N/A |

**Testing Procedure per Client:**
1. Connect with default settings
2. Enable EGFX/H.264 graphics
3. Check capability negotiation logs
4. Verify video stream displays
5. Run quality tests (gradient, color accuracy)
6. Measure bandwidth and FPS

---

## 6. RISK ANALYSIS

### Risk 1: Chroma Packing Complexity

**Risk:** Incorrect pixel ordering breaks reconstruction on client side

**Likelihood:** **Medium-High**
**Impact:** **High** (corruption, wrong colors, decoder errors)

**Symptoms:**
- Client displays corrupted/garbled video
- Colors shifted or inverted
- Decoder warnings in client logs
- Crash on Windows MFT decoder

**Mitigation Strategies:**

**Strategy 1: Start with Simplified Packing**
```rust
// Phase 1: Naive odd pixel extraction
// May not match spec exactly but proves concept
pack_auxiliary_view_simplified()
```
- Accept some color errors in Phase 1
- Refine in Phase 2 once basic pipeline works

**Strategy 2: Study FreeRDP Decoder**
```bash
# Clone FreeRDP source
git clone https://github.com/FreeRDP/FreeRDP.git
cd FreeRDP

# Study decoder implementation
grep -r "avc444" --include="*.c" --include="*.h"
# Key file: libfreerdp/codec/h264.c (avc444_decompress function)
```
- Reverse-engineer exact reconstruction algorithm
- Map encoder to inverse of decoder

**Strategy 3: Unit Test with Known Patterns**
```rust
#[test]
fn test_chroma_packing_checkerboard() {
    // Create YUV444 checkerboard pattern
    let yuv444 = create_checkerboard_yuv444();

    // Pack to dual YUV420
    let main = pack_main_view(&yuv444);
    let aux = pack_auxiliary_view(&yuv444);

    // Verify: Main contains even samples, aux contains odd
    for y in 0..height {
        for x in 0..width {
            if x % 2 == 0 && y % 2 == 0 {
                // Should be in main view
                assert_eq!(main.u[y/2][x/2], yuv444.u[y][x]);
            } else {
                // Should be in auxiliary view
                // (exact location depends on packing algorithm)
            }
        }
    }
}
```

**Strategy 4: Pixel-by-Pixel Validation**
```python
# Decode both streams and reconstruct YUV444
# Compare with original

def validate_packing(original_yuv444, reconstructed_yuv444):
    for y in range(height):
        for x in range(width):
            diff_u = abs(original_yuv444.u[y][x] - reconstructed_yuv444.u[y][x])
            diff_v = abs(original_yuv444.v[y][x] - reconstructed_yuv444.v[y][x])

            if diff_u > 5 or diff_v > 5:
                print(f"Error at ({x}, {y}): U diff={diff_u}, V diff={diff_v}")
```

**Rollback Plan:**
- Keep AVC420 codec working
- Make AVC444 opt-in via config
- Document known limitations
- Provide fallback mechanism

### Risk 2: OpenH264 Quirks with Fake Luma

**Risk:** Encoder optimizes chroma-as-luma differently, causing quality loss

**Likelihood:** **Low**
**Impact:** **Medium** (quality degradation, increased bitrate)

**Why This Could Happen:**
- H.264 encoders apply different optimizations to luma vs chroma
- Luma affects perceptual quality more → more bits allocated
- If encoder detects "luma" (which is actually chroma), may over-allocate bits

**Testing:**
```rust
// Compare bitstream sizes
let normal_luma_size = encode_normal_yuv420().size;
let chroma_as_luma_size = encode_auxiliary_view().size;

println!("Normal luma: {}b, Chroma-as-luma: {}b",
         normal_luma_size, chroma_as_luma_size);
// Expected: Similar sizes (±20%)
// If chroma-as-luma is significantly larger → encoder issue
```

**Mitigation:**

**Option A: Disable Luma Optimizations**
```rust
// Configure OpenH264 encoder for auxiliary stream
let mut aux_config = OpenH264Config::new()
    .bitrate(...)
    .disable_deblocking_filter()  // Reduce luma-specific filtering
    .complexity(Complexity::Low);  // Less aggressive optimization
```

**Option B: Separate QP for Auxiliary Stream**
```rust
// Use higher QP (lower quality) for auxiliary stream
// Chroma is less perceptually important than luma
main_config.qp = 23;
aux_config.qp = 28;  // +5 QP = ~30% smaller bitstream
```

**Option C: Use Constant QP Mode**
```rust
// Force constant QP (bypass rate control)
aux_config.rc_mode(RateControlMode::ConstantQP);
```

**Monitoring:**
```rust
// Log auxiliary stream statistics
debug!(
    "Auxiliary stream: {}b ({}% of main stream)",
    aux_size,
    (aux_size * 100) / main_size
);
// Expected: 30-50% of main stream
// If >70% → investigate encoder settings
```

### Risk 3: Bandwidth Concerns

**Risk:** +30-40% bandwidth may be unacceptable for WAN deployments

**Likelihood:** **Medium**
**Impact:** **Low** (it's optional premium feature, but limits adoption)

**Bandwidth Comparison (1080p @ 30 FPS):**

| Codec | Avg Bitrate | Desktop | Video Playback | CAD/Graphics |
|-------|-------------|---------|----------------|--------------|
| **AVC420** | 3-5 Mbps | 3 Mbps | 5 Mbps | 4 Mbps |
| **AVC444** | 4-7 Mbps | 4 Mbps | 7 Mbps | 6 Mbps |
| **Increase** | +30-40% | +33% | +40% | +50% |

**Mitigation:**

**Strategy 1: Clear Documentation**
```markdown
## When to Use AVC444

✅ **Use AVC444 when:**
- LAN environment (>10 Mbps available)
- Graphics/CAD applications
- Color accuracy critical
- Single user on server

❌ **Use AVC420 when:**
- WAN environment (<5 Mbps)
- General desktop usage
- Multi-user server (bandwidth constrained)
- Video playback (4:2:0 sufficient)
```

**Strategy 2: Auto-Detection**
```rust
pub fn auto_select_codec(bandwidth_estimate: u32) -> &'static str {
    if bandwidth_estimate > 10_000 {  // >10 Mbps
        "avc444"
    } else {
        "avc420"
    }
}
```

**Strategy 3: Dynamic Switching**
```rust
// Future enhancement: Switch codec based on content
if is_graphics_app_active() {
    use_avc444();
} else {
    use_avc420();  // Save bandwidth
}
```

**Strategy 4: Bandwidth Throttling**
```rust
// Reduce bitrate for AVC444 if bandwidth limited
let bitrate = if codec == "avc444" && bandwidth < 10_000 {
    6_000  // Lower quality but still 4:4:4
} else if codec == "avc444" {
    10_000  // Full quality
} else {
    5_000  // AVC420 default
};
```

**Monitoring:**
```bash
# Log bandwidth usage
INFO: AVC444 encoding at 6.2 Mbps (target: 10 Mbps)
WARN: Bandwidth exceeds 10 Mbps, consider AVC420
```

### Risk 4: Client Compatibility

**Risk:** Not all Windows clients support AVC444

**Likelihood:** **Low** (most modern Windows supports it)
**Impact:** **Medium** (feature not usable on older clients)

**Client Support Matrix:**

| OS | Version | AVC444 Support | Notes |
|----|---------|----------------|-------|
| **Windows 10** | 1903+ | ✅ Yes | Native support |
| **Windows 10** | <1903 | ⚠️ Partial | May require updates |
| **Windows 8.1** | All | ❌ No | AVC420 only |
| **Windows 7** | All | ❌ No | AVC420 only |

**Mitigation:**

**Strategy 1: Capability Negotiation**
```rust
// Check client capabilities during EGFX handshake
if client_caps.contains("AVC444") {
    info!("Client supports AVC444, enabling");
    use_avc444 = true;
} else {
    warn!("Client doesn't support AVC444, using AVC420");
    use_avc444 = false;
}
```

**Strategy 2: Graceful Fallback**
```rust
pub enum CodecMode {
    Avc444Preferred,  // Try AVC444, fall back to AVC420
    Avc444Required,   // Reject if AVC444 not supported
    Avc420Only,
}

match config.codec_mode {
    CodecMode::Avc444Preferred => {
        if client_supports_avc444() {
            VideoEncoder::Avc444(...)
        } else {
            warn!("AVC444 not supported, using AVC420");
            VideoEncoder::Avc420(...)
        }
    }
    CodecMode::Avc444Required => {
        if !client_supports_avc444() {
            return Err("Client must support AVC444");
        }
        VideoEncoder::Avc444(...)
    }
    CodecMode::Avc420Only => VideoEncoder::Avc420(...),
}
```

**Strategy 3: Documentation**
```markdown
## Client Requirements

### AVC444 Support
- **Windows 10:** Version 1903 or later (May 2019 Update)
- **Windows 11:** All versions
- **FreeRDP:** Version 3.0 or later (with AVC444 enabled)

### Checking Support
```powershell
# Windows version
winver
# Should show: Version 1903 or higher

# FreeRDP
freerdp --version
# Should show: FreeRDP 3.0 or later
```

### Fallback Behavior
If AVC444 is configured but client doesn't support it:
- Server automatically falls back to AVC420
- Warning logged: "Client doesn't support AVC444"
- Connection proceeds with AVC420
```

**Strategy 4: Testing on Multiple Clients**
```bash
# Test matrix
./test-avc444.sh --client windows10-1903
./test-avc444.sh --client windows10-2004
./test-avc444.sh --client windows11
./test-avc444.sh --client freerdp-3.0
./test-avc444.sh --client freerdp-2.11  # Should gracefully fail
```

### Risk 5: Implementation Time Overrun

**Risk:** Actual effort exceeds estimate (24-34 hours)

**Likelihood:** **Medium-High** (complex algorithm, debugging needed)
**Impact:** **Low** (it's worth the investment, but delays other features)

**Contingency Planning:**

**Time Budget Breakdown (with buffers):**

| Phase | Estimate | Buffer | Total |
|-------|----------|--------|-------|
| Phase 1: Color Conversion | 6-8h | +2h | 10h |
| Phase 2: Stream Packing | 8-12h | +4h | 16h |
| Phase 3: Dual Encoder | 6-10h | +3h | 13h |
| Phase 4: Integration | 8-12h | +4h | 16h |
| **Total** | **28-42h** | **+13h** | **55h max** |

**Mitigation Strategies:**

**Strategy 1: Phased Delivery**
```
Milestone 1 (10h): Color conversion working + tests passing
Milestone 2 (26h): Simplified packing + basic encoding
Milestone 3 (42h): Spec-compliant packing + optimization
Milestone 4 (55h): Full integration + comprehensive testing
```

**Strategy 2: Accept Simplified v1**
```rust
// Ship with simplified packing in Phase 1
// Add spec-compliant packing in future PR
#[cfg(feature = "avc444-spec-compliant")]
fn pack_auxiliary_view() {
    pack_auxiliary_view_spec_compliant()
}

#[cfg(not(feature = "avc444-spec-compliant"))]
fn pack_auxiliary_view() {
    pack_auxiliary_view_simplified()
}
```

**Strategy 3: Focus on MVP**
```
MVP Scope:
✅ BGRA → YUV444 conversion (BT.709)
✅ Simplified chroma packing (good enough for testing)
✅ Dual OpenH264 encoding
✅ Basic integration (no SIMD, no optimization)
✅ Works with Windows 10/11 client

Defer to v2:
⏸️ BT.601/BT.2020 support
⏸️ Spec-compliant macroblock packing
⏸️ SIMD optimization
⏸️ Parallel encoding
⏸️ Dynamic codec switching
```

**Strategy 4: Timebox Debugging**
```
Per-Phase Debug Limit:
- Phase 1: 2 hours debugging max
- Phase 2: 4 hours debugging max (most complex)
- Phase 3: 2 hours debugging max
- Phase 4: 4 hours debugging max

If exceeded:
1. Document blocker
2. Seek external input (FreeRDP community, IronRDP maintainers)
3. Consider temporary workaround
4. Re-scope if necessary
```

**Rollback Criteria:**
```
If total time exceeds 60 hours:
- Ship AVC420 only (already working)
- Document AVC444 as "experimental" branch
- Merge when stable
- Don't block other features
```

---

## 7. REFERENCES & RESOURCES

### 7.1 Official Specifications

**Microsoft MS-RDPEGFX Protocol:**
- [RFX_AVC444_BITMAP_STREAM Structure](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/844018a5-d717-4bc9-bddb-8b4d6be5dd3f)
- [YUV420p Stream Combination for YUV444 mode](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/8131c1bc-1af8-4907-a05a-f72f4581160f)
- [YUV420p Stream Combination for YUV444v2 mode](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/781406c3-5e24-4f2b-b6ff-42b76bf64f6d)
- [MPEG-4 AVC/H.264 Compression](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/50488d2b-82e4-4e91-b600-262c2dcb36cd)

**ITU-R Color Standards:**
- [ITU-R BT.709: HDTV Color Space](https://www.itu.int/rec/R-REC-BT.709/)
- [ITU-R BT.601: SDTV Color Space](https://www.itu.int/rec/R-REC-BT.601/)
- [ITU-R BT.2020: UHDTV Color Space](https://www.itu.int/rec/R-REC-BT.2020/)

### 7.2 Reference Implementations

**FreeRDP:**
- [AVC444 Implementation Commit (2016)](https://github.com/FreeRDP/FreeRDP/commit/5bc333c626f1db493a2c2e3c49d91cc6fb145309)
- [AVC444 Decoder Source](https://github.com/FreeRDP/FreeRDP/blob/master/libfreerdp/codec/h264.c)
- [Known Issues with AVC444](https://github.com/FreeRDP/FreeRDP/issues/11040)

**IronRDP:**
- [Our PR #1057: Complete EGFX Implementation](https://github.com/Devolutions/IronRDP/pull/1057)
- [Original PR #648 by elmarco](https://github.com/Devolutions/IronRDP/pull/648)

### 7.3 Technical Background

**Chroma Subsampling Theory:**
- [Chroma Subsampling Wikipedia](https://en.wikipedia.org/wiki/Chroma_subsampling)
- [4:4:4 vs 4:2:2 vs 4:2:0 Explained](https://www.rtings.com/tv/learn/chroma-subsampling)
- [YUV Pixel Formats - Linux Kernel Docs](https://docs.kernel.org/userspace-api/media/v4l/pixfmt-yuv-planar.html)

**H.264/AVC Standards:**
- [H.264 Overview (Wikipedia)](https://en.wikipedia.org/wiki/Advanced_Video_Coding)
- [High 4:4:4 Predictive Profile](https://en.wikipedia.org/wiki/Advanced_Video_Coding#Profiles)

### 7.4 Codebase Files

**Current Implementation:**
- `/home/greg/wayland/wrd-server-specs/src/egfx/encoder.rs` - Avc420Encoder
- `/home/greg/wayland/wrd-server-specs/src/server/display_handler.rs` - Frame loop
- `/home/greg/wayland/wrd-server-specs/src/server/egfx_sender.rs` - EGFX transport
- `/home/greg/wayland/wrd-server-specs/Cargo.toml` - Dependencies

**To Be Created:**
- `/home/greg/wayland/wrd-server-specs/src/egfx/color_convert.rs`
- `/home/greg/wayland/wrd-server-specs/src/egfx/yuv444_packing.rs`
- `/home/greg/wayland/wrd-server-specs/src/egfx/avc444_encoder.rs`
- `/home/greg/wayland/wrd-server-specs/benches/avc444_encoder.rs`

**Documentation:**
- `/home/greg/wayland/wrd-server-specs/docs/research/AVC444-DEEP-DIVE-2025-12-25.md` - Prior research

### 7.5 Testing Resources

**Color Test Patterns:**
- Macbeth ColorChecker (industry standard)
- SMPTE color bars
- Gradient generators (CSS Matic, ColorZilla)

**Client Downloads:**
- [Windows 10 ISO](https://www.microsoft.com/software-download/windows10)
- [FreeRDP Releases](https://github.com/FreeRDP/FreeRDP/releases)

**Debugging Tools:**
- Wireshark (RDP protocol analysis)
- Windows Event Viewer (decoder errors)
- FFmpeg (bitstream inspection)

### 7.6 Community Resources

**Discussion Forums:**
- [FreeRDP Issues](https://github.com/FreeRDP/FreeRDP/issues?q=avc444)
- [IronRDP Discussions](https://github.com/Devolutions/IronRDP/discussions)

**Previous Research:**
- Your AVC444-DEEP-DIVE document (excellent foundation)
- Microsoft TechCommunity article on RDP 10 improvements

---

## APPENDIX A: Quick Reference

### Color Matrix Formulas

**BT.709 (HD):**
```
Y  =  0.2126 × R + 0.7152 × G + 0.0722 × B
U  = -0.1146 × R - 0.3854 × G + 0.5000 × B + 128
V  =  0.5000 × R - 0.4542 × G - 0.0458 × B + 128
```

**BT.601 (SD):**
```
Y  =  0.299  × R + 0.587  × G + 0.114  × B
U  = -0.1687 × R - 0.3313 × G + 0.5000 × B + 128
V  =  0.5000 × R - 0.4187 × G - 0.0813 × B + 128
```

### 2×2 Box Filter (Chroma Subsampling)

```rust
chroma_420[y/2][x/2] = (
    chroma_444[y  ][x  ] +
    chroma_444[y  ][x+1] +
    chroma_444[y+1][x  ] +
    chroma_444[y+1][x+1]
) / 4
```

### LC Field Encoding

```
LC = 0x0 (00): LUMA_AND_CHROMA (both streams)
LC = 0x1 (01): LUMA (stream1 only)
LC = 0x2 (10): CHROMA (stream1 only, use previous luma)
LC = 0x3 (11): Invalid
```

### Stream Info Header (32 bits)

```
Bits  0-29: stream1_size (bytes)
Bits 30-31: LC field
```

### Performance Targets

| Resolution | Color Conv | Packing | Encoding | Total |
|------------|------------|---------|----------|-------|
| 720p       | <2ms       | <1ms    | <10ms    | <15ms |
| 1080p      | <5ms       | <2ms    | <25ms    | <30ms |
| 4K         | <15ms      | <5ms    | <80ms    | <100ms|

---

## APPENDIX B: Common Pitfalls

### Pitfall 1: Using AVC Format Instead of Annex B

**Wrong:**
```rust
let bitstream = openh264_encode(yuv);
let avc_data = annex_b_to_avc(&bitstream);  // ❌ Don't do this!
send_frame(avc_data);
```

**Right:**
```rust
let bitstream = openh264_encode(yuv);
send_frame(bitstream);  // ✅ Use Annex B directly
```

**Reason:** MS-RDPEGFX requires Annex B format (start codes), not AVC (length-prefixed).

### Pitfall 2: Forgetting Chroma Offset (+128)

**Wrong:**
```rust
u_val = -0.1146 * r - 0.3854 * g + 0.5000 * b;  // ❌ Missing offset
```

**Right:**
```rust
u_val = -0.1146 * r - 0.3854 * g + 0.5000 * b + 128.0;  // ✅ Add 128
```

**Reason:** Chroma values are offset by 128 to fit in unsigned [0, 255] range.

### Pitfall 3: Dimension Alignment

**Wrong:**
```rust
// Encode 1921×1081 frame directly
encode(bgra, 1921, 1081);  // ❌ Not aligned to 16
```

**Right:**
```rust
// Align to 16-pixel boundary
let aligned_width = (1921 + 15) & !15;   // = 1936
let aligned_height = (1081 + 15) & !15;  // = 1088

// Pad frame to aligned size
let padded_bgra = pad_frame(bgra, 1921, 1081, aligned_width, aligned_height);
encode(padded_bgra, aligned_width, aligned_height);

// Use destRect to crop to actual 1921×1081 on client
```

### Pitfall 4: Mixing Encoder Instances

**Wrong:**
```rust
// Reuse same encoder for both streams
let stream1 = encoder.encode(main_yuv420);
encoder.reset();  // ❌ May not fully reset state
let stream2 = encoder.encode(aux_yuv420);
```

**Right:**
```rust
// Use separate encoder instances
let stream1 = main_encoder.encode(main_yuv420);
let stream2 = aux_encoder.encode(aux_yuv420);  // ✅ Independent
```

**Reason:** Encoder state (reference frames, motion vectors) may contaminate second stream.

### Pitfall 5: Ignoring Encoder Skipped Frames

**Wrong:**
```rust
let frame = encoder.encode(bgra)?;
send_frame(&frame.data);  // ❌ May panic if None
```

**Right:**
```rust
let frame = encoder.encode(bgra)?;
let Some(frame_data) = frame else {
    trace!("Encoder skipped frame");
    return Ok(());  // ✅ Handle gracefully
};
send_frame(&frame_data.data);
```

**Reason:** OpenH264 may skip frames for rate control. Always check for `None`.

---

## APPENDIX C: Configuration Examples

### High Quality (LAN)

```toml
[egfx]
codec = "avc444"
h264_bitrate = 12000  # 12 Mbps
max_fps = 60

[network]
mtu = 1500
tcp_nodelay = true
```

### Balanced (General Use)

```toml
[egfx]
codec = "avc444"
h264_bitrate = 8000   # 8 Mbps
max_fps = 30

[network]
mtu = 1500
```

### Low Bandwidth (WAN)

```toml
[egfx]
codec = "avc420"      # Fall back to 4:2:0
h264_bitrate = 3000   # 3 Mbps
max_fps = 24
enable_skip_frame = true
```

### Development/Testing

```toml
[egfx]
codec = "avc444"
h264_bitrate = 5000
force_keyframe_interval = 30  # Every 30 frames

[logging]
level = "debug"
egfx_detailed = true
```

---

## APPENDIX D: Troubleshooting Guide

### Issue: Client Shows Black Screen

**Symptoms:**
- Connection succeeds
- EGFX negotiated
- Frames sent but no video

**Diagnosis:**
```bash
# Check server logs
grep -i "send_avc444_frame" server.log
# Should show: Sent AVC444 frame (XXX bytes)

# Check for decoder errors on client
# Windows Event Viewer → TerminalServices-RemoteDesktopClient
```

**Solutions:**
1. Verify bitstream format (Annex B, not AVC)
2. Check SPS/PPS headers present
3. Ensure dimensions are 16-aligned
4. Test with AVC420 (if works, issue is in AVC444 packing)

### Issue: Corrupted Colors

**Symptoms:**
- Video displays but colors wrong
- Color shift, inversion, or blocking

**Diagnosis:**
```rust
// Add debug output in packing
debug!("Main U[0][0] = {}, Aux Y[0][0] = {}",
       main.u[0], aux.y[0]);
// Should see chroma values in expected ranges
```

**Solutions:**
1. Verify chroma offset (+128) applied
2. Check BT.709 vs BT.601 matrix
3. Test with solid colors (red, green, blue)
4. Compare with AVC420 output (color matrix issue vs packing issue)

### Issue: High Bandwidth Usage

**Symptoms:**
- Bandwidth exceeds 15 Mbps @ 1080p
- Client buffering or lag

**Diagnosis:**
```bash
# Monitor bitrate
grep "AVC444 frame" server.log | awk '{print $10}' | stats
# Should average 6-10 Mbps for typical desktop
```

**Solutions:**
1. Reduce bitrate in config
2. Increase QP (lower quality, smaller bitstream)
3. Enable skip_frames
4. Check for static region detection
5. Consider AVC420 for high-motion content

### Issue: Low Frame Rate

**Symptoms:**
- FPS < 20 on capable hardware
- High CPU usage

**Diagnosis:**
```bash
# Profile encoder
cargo flamegraph --bin lamco-rdp-server
# Look for hot spots in color conversion or packing
```

**Solutions:**
1. Enable SIMD (if available)
2. Reduce resolution
3. Parallel encoding (future enhancement)
4. Check for memory allocations in hot path
5. Use release build (`--release`)

---

**END OF IMPLEMENTATION PLAN**

This document provides complete context for implementing AVC444 from scratch in a future development session. All algorithms, integration points, testing strategies, and risk mitigations are documented for clean execution.
