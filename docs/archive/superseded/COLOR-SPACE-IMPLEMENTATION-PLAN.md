# Color Space Implementation Plan

**Created:** 2025-12-27
**Status:** Research & Planning Phase
**Priority:** Critical - Commercial Product Quality

## Executive Summary

The lamco-rdp-server has a color space mismatch issue causing visual artifacts (pink/magenta corruption). This document captures the research findings and implementation plan for a robust, standards-compliant solution across all encoder backends.

---

## Problem Statement

### Root Cause
AVC420 and AVC444 encoding paths handle color conversion differently:

| Path | Color Conversion | Range | Result |
|------|------------------|-------|--------|
| **AVC420** | OpenH264 internal | BT.601 limited (16-235) | ✅ Works |
| **AVC444** | Our `color_convert.rs` | BT.709 full (0-255) | ❌ Mismatch |

The AVC444 "optimization" (using `YUVSlices` directly) bypassed OpenH264's internal conversion, creating inconsistent color output.

### Symptoms
- Pink/magenta artifacts in RDP sessions
- Color distortion on decoded frames
- Inconsistent appearance between AVC420 and AVC444 modes

---

## Standards Reference

### H.264 VUI (Video Usability Information)

VUI parameters are embedded in the SPS (Sequence Parameter Set) NAL unit and tell decoders how to interpret color data.

**Key Fields:**
```
video_signal_type_present_flag  → Enables color signaling
video_full_range_flag           → 0=limited (16-235), 1=full (0-255)
colour_description_present_flag → Enables color space fields
colour_primaries                → Which RGB primaries (BT.709, BT.601, etc.)
transfer_characteristics        → Gamma/transfer function
matrix_coefficients             → RGB↔YUV conversion matrix
```

### Standard Values

| Color Space | colour_primaries | transfer_char | matrix_coeff | Use Case |
|-------------|------------------|---------------|--------------|----------|
| **BT.709** | 1 | 1 | 1 | HD content (≥1080p) |
| **BT.601 NTSC** | 6 | 6 | 6 | SD NTSC |
| **BT.601 PAL** | 5 | 6 | 5 | SD PAL |
| **sRGB** | 1 | 13 | 1 | Computer graphics |
| **BT.2020** | 9 | 14/15 | 9 | UHD/HDR |

### MS-RDPEGFX Requirements

The RDP Graphics Pipeline Extension (MS-RDPEGFX) specification:
- Does NOT mandate a specific color matrix
- Requires client/server to agree on interpretation
- Uses standard H.264 NAL format (Annex B)
- Relies on VUI for color space signaling

---

## Current Implementation Analysis

### OpenH264 (Software Encoder)

**Source:** `openh264` crate v0.9.0 wrapping OpenH264 v2.4.1

**VUI Support:**
- ✅ OpenH264 C++ API: Full VUI support in `SWelsSPS` structure
- ✅ openh264-sys2 bindings: All VUI fields exposed in `SSpatialLayerConfig`
- ❌ openh264 Rust crate: `EncoderConfig` does NOT expose VUI fields
- ⚠️ Workaround: Raw API access via `Encoder::raw_api()` possible

**VUI Fields in openh264-sys2:**
```rust
pub struct SSpatialLayerConfig {
    // VUI Color Parameters
    pub bVideoSignalTypePresent: bool,
    pub uiVideoFormat: c_uchar,
    pub bFullRange: bool,
    pub bColorDescriptionPresent: bool,
    pub uiColorPrimaries: c_uchar,
    pub uiTransferCharacteristics: c_uchar,
    pub uiColorMatrix: c_uchar,
    // ...
}
```

**Internal Conversion (BT.601 Limited):**
```c
// From openh264/formats/rgb2yuv.rs
*y = (((66 * R + 129 * G + 25 * B) >> 8) + 16) as u8;  // Y: 16-235
*u = (((-38 * R - 74 * G + 112 * B) >> 8) + 128) as u8; // U: 16-240
*v = (((112 * R - 94 * G - 18 * B) >> 8) + 128) as u8;  // V: 16-240
```

### VA-API (Intel/AMD Hardware)

**Source:** `cros-libva` crate v0.0.13

**Current State:**
- ❌ Color matrix hardcoded to BT.709 in `bgra_to_nv12()`
- ❌ VUI color fields NOT set in sequence parameters
- ❌ Ignores `EgfxConfig::color_matrix` setting
- ✅ VUI structure exists (`H264VuiFields`) but incomplete

**bgra_to_nv12() Coefficients (Hardcoded BT.709):**
```rust
// Line 754-757 in vaapi/mod.rs
let y = (0.2126 * r + 0.7152 * g + 0.0722 * b) as u8;
// Uses limited range clamping (16-235)
```

### NVENC (NVIDIA Hardware)

**Source:** `nvidia-video-codec-sdk` crate v0.4

**Current State:**
- ❌ Color conversion handled internally by GPU (opaque)
- ❌ No VUI parameter configuration exposed in Rust bindings
- ❌ Unknown what color matrix NVENC uses
- ⚠️ May require raw NVENC API or NAL post-processing

---

## Research Tasks

### Research Complete: OpenH264-rs Upstream Contribution

**Repository:** https://github.com/ralfbiedert/openh264-rs

**Findings:**

1. **No existing VUI support** - EncoderConfig does NOT expose VUI fields
2. **No forks have added VUI** - Checked 29 forks, none have this
3. **Upstream would welcome this** - Maintainer explicitly welcomes PRs
4. **Project is active** - Last commit December 25, 2025 (2 days ago)
5. **README lists this as needed** - "BT.601 / BT.709 YUV <-> RGB Conversion" is specifically mentioned

**Related Issues:**
- Issue #42: "Washed out colors" - Fixed limited range handling (PR #79)
- Issue #58: "Overread VUI by 8 bits" - Related to container muxing

**VUI Fields Available in openh264-sys2:**
```rust
// Already exposed in SSpatialLayerConfig:
bVideoSignalTypePresent: bool,
bFullRange: bool,
bColorDescriptionPresent: bool,
uiColorPrimaries: u8,        // CP_BT709=1, CP_SMPTE170M=6
uiTransferCharacteristics: u8,
uiColorMatrix: u8,           // CM_BT709=1, CM_SMPTE170M=6
```

**Recommendation:** Submit upstream PR - maintainer is receptive, change is small and backward compatible.

---

### Research Complete: NVENC Color Space (CRITICAL DISCOVERY)

**Updated 2025-12-27:** Additional research confirms the path forward.

#### The Key Finding

**NVENC's internal RGB→YUV conversion uses BT.601 - VUI parameters only set metadata, NOT the actual conversion matrix.**

The NVIDIA Control Panel color adjustments live in the **display pipeline** - they don't affect the encoder. For video encoding, you must treat color as two separate jobs:

1. **Do the pixel math yourself** (on GPU if you want)
2. **Signal the chosen colorimetry correctly** in VUI and/or container metadata

#### Evidence

1. **NVIDIA Developer Forums** - Multiple unanswered questions about internal conversion
2. **Community Testing** - BT.709 vs BT.601 VUI tags produce identical pixel values
3. **FFmpeg Source** - Explicitly sets BT.601 for RGB input because that's what NVENC uses:
   ```c
   // From FFmpeg nvenc.c
   if ((pixdesc->flags & AV_PIX_FMT_FLAG_RGB)) {
       vui->colourMatrix = AVCOL_SPC_BT470BG;  // BT.601!
   }
   ```

#### NVENC VUI API (DOES EXIST - CONFIRMED)

`NV_ENC_CONFIG_H264_VUI_PARAMETERS` structure provides all standard fields:

| Field | Purpose |
|-------|---------|
| `colourPrimaries` | Which RGB primaries (BT.709=1, BT.601=6) |
| `transferCharacteristics` | Gamma/OETF (BT.709=1, sRGB=13) |
| `colourMatrix` | RGB↔YCbCr matrix coefficients |
| `videoFullRangeFlag` | Limited (0) vs Full (1) |
| Chroma sample location flags | For decoder positioning |

**CRITICAL:** These VUI fields are **metadata only** - they tell the decoder how to interpret the data, but NVENC's internal RGB→YUV conversion always uses BT.601.

#### The Solution: Dual-Path Implementation

**Path A: CPU Color Conversion (Simpler)**
```
BGRA → Our bgra_to_nv12() (with correct matrix) → NV12 → NVENC → H.264 with matching VUI
```

**Path B: Full-GPU Pipeline (Higher Performance)**
```
BGRA → CUDA Kernel (BT.709 matrix) → NV12 (device memory) → NVENC → H.264
```

NPP (NVIDIA Performance Primitives) provides:
- Explicit Rec.709 HDTV conversion routines
- BUT: No direct RGB→NV12 helper (requires RGB→planar YUV→pack to NV12)

**Recommended approach:**
- NPP conversion + small custom CUDA kernel for packing, OR
- Single custom kernel: RGB→NV12 directly (100% deterministic)

#### CUDA Color Conversion Kernel (BT.709)

```cuda
// BT.709 RGB→YCbCr coefficients
#define Y_R  0.2126f
#define Y_G  0.7152f
#define Y_B  0.0722f
#define CB_R -0.1146f
#define CB_G -0.3854f
#define CB_B  0.5f
#define CR_R  0.5f
#define CR_G -0.4542f
#define CR_B -0.0458f

extern "C" __global__ void bgra_to_nv12_bt709(
    const unsigned char* __restrict__ bgra,
    unsigned char* __restrict__ y_plane,
    unsigned char* __restrict__ uv_plane,
    int width, int height,
    int bgra_pitch, int y_pitch, int uv_pitch,
    bool limited_range)
{
    const int x = (blockIdx.x * blockDim.x + threadIdx.x) * 2;
    const int y = (blockIdx.y * blockDim.y + threadIdx.y) * 2;

    if (x >= width || y >= height) return;

    float cb_sum = 0.0f, cr_sum = 0.0f;

    // Process 2x2 block for NV12 chroma subsampling
    for (int dy = 0; dy < 2; dy++) {
        for (int dx = 0; dx < 2; dx++) {
            int px = x + dx, py = y + dy;
            if (px < width && py < height) {
                int idx = py * bgra_pitch + px * 4;
                float b = bgra[idx] / 255.0f;
                float g = bgra[idx + 1] / 255.0f;
                float r = bgra[idx + 2] / 255.0f;

                float Y = Y_R * r + Y_G * g + Y_B * b;
                float cb = CB_R * r + CB_G * g + CB_B * b + 0.5f;
                float cr = CR_R * r + CR_G * g + CR_B * b + 0.5f;

                if (limited_range) {
                    Y = Y * 219.0f / 255.0f + 16.0f / 255.0f;
                    cb = cb * 224.0f / 255.0f + 16.0f / 255.0f;
                    cr = cr * 224.0f / 255.0f + 16.0f / 255.0f;
                }

                y_plane[py * y_pitch + px] = (unsigned char)(Y * 255.0f);
                cb_sum += cb;
                cr_sum += cr;
            }
        }
    }

    // Write averaged chroma (NV12 interleaved UV)
    int uv_idx = (y / 2) * uv_pitch + x;
    uv_plane[uv_idx] = (unsigned char)(cb_sum / 4.0f * 255.0f);
    uv_plane[uv_idx + 1] = (unsigned char)(cr_sum / 4.0f * 255.0f);
}
```

#### Rust Integration via cudarc

The `cudarc` crate (already in Cargo.toml for NVENC) provides:
- `CudaDevice::load_ptx()` - Load compiled kernel
- Zero-copy device memory allocation
- Direct handoff to NVENC without CPU round-trip

#### Summary: NVENC Implementation Strategy

| Phase | Approach | Performance |
|-------|----------|-------------|
| **Phase A** | CPU `bgra_to_nv12()` + NVENC NV12 input | Good |
| **Phase B** | Custom CUDA kernel + NVENC | Optimal |
| **Phase C** | NPP + custom packing kernel | Alternative |

**For AVC444:** We MUST do BGRA→YUV444 ourselves anyway (for dual-stream decomposition), so we have full control over the color matrix.

---

### Research Complete: VA-API Color Space Enhancement

**Current Implementation Status:**
- ✅ VUI structure exists (`H264VuiFields`)
- ❌ VUI color fields NOT populated
- ❌ `bgra_to_nv12()` hardcoded to BT.709
- ❌ No configuration support for color matrix

**Implementation Path:**
1. Add `ColorMatrix` parameter to `VaapiEncoder::new()`
2. Implement BT.601 coefficients alongside BT.709 in `bgra_to_nv12()`
3. Set VA-API VUI color fields in sequence parameters
4. Verify with FFprobe output analysis

**VA-API VUI is straightforward** - unlike NVENC, we control the color conversion ourselves, so VUI metadata will match actual pixel values.

---

## Proposed Architecture

### Unified Color Configuration Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    EgfxConfig (TOML)                        │
│  color_matrix: "auto" | "bt709" | "bt601" | "srgb"         │
│  color_range: "auto" | "full" | "limited"                   │
└─────────────────────────────────┬───────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────┐
│                    ColorSpaceConfig                         │
│  - matrix: ColorMatrix (BT709, BT601, SRGB)                │
│  - range: ColorRange (Full, Limited)                        │
│  - primaries: u8 (H.264 VUI value)                         │
│  - transfer: u8 (H.264 VUI value)                          │
│  - matrix_coeff: u8 (H.264 VUI value)                      │
└─────────────────────────────────┬───────────────────────────┘
                                  │
          ┌───────────────────────┼───────────────────────┐
          │                       │                       │
          ▼                       ▼                       ▼
   ┌─────────────┐        ┌─────────────┐        ┌─────────────┐
   │  OpenH264   │        │   VA-API    │        │   NVENC     │
   │             │        │             │        │             │
   │ VUI via raw │        │ VUI in seq  │        │ TBD based   │
   │ API access  │        │ params +    │        │ on research │
   │             │        │ SW color cv │        │             │
   └─────────────┘        └─────────────┘        └─────────────┘
```

### ColorSpaceConfig Struct

```rust
/// Complete color space configuration for H.264 encoding
#[derive(Debug, Clone, Copy)]
pub struct ColorSpaceConfig {
    /// Color matrix for RGB↔YUV conversion
    pub matrix: ColorMatrix,

    /// Value range (full 0-255 or limited 16-235)
    pub range: ColorRange,

    /// H.264 VUI colour_primaries value
    pub primaries: u8,

    /// H.264 VUI transfer_characteristics value
    pub transfer: u8,

    /// H.264 VUI matrix_coefficients value
    pub matrix_coeff: u8,
}

impl ColorSpaceConfig {
    /// BT.709 for HD content (default for ≥1080p)
    pub const BT709_LIMITED: Self = Self {
        matrix: ColorMatrix::BT709,
        range: ColorRange::Limited,
        primaries: 1,
        transfer: 1,
        matrix_coeff: 1,
    };

    /// BT.601 for SD content (≤720p)
    pub const BT601_LIMITED: Self = Self {
        matrix: ColorMatrix::BT601,
        range: ColorRange::Limited,
        primaries: 6,  // SMPTE 170M
        transfer: 6,
        matrix_coeff: 6,
    };

    /// sRGB for computer graphics (full range)
    pub const SRGB_FULL: Self = Self {
        matrix: ColorMatrix::BT709,  // sRGB uses BT.709 primaries
        range: ColorRange::Full,
        primaries: 1,
        transfer: 13,  // IEC 61966-2-1 (sRGB)
        matrix_coeff: 1,
    };

    /// Auto-select based on resolution
    pub fn auto_select(width: u32, height: u32) -> Self {
        if width >= 1280 && height >= 720 {
            Self::BT709_LIMITED
        } else {
            Self::BT601_LIMITED
        }
    }
}
```

---

## Implementation Phases (Updated Post-Research)

### Phase 1: OpenH264 VUI Support
**Goal:** Add VUI configuration to openh264-rs and our encoders

**Option A: Fork + Contribute Upstream (Recommended)**
1. Fork openh264-rs
2. Add VUI fields to `EncoderConfig`:
   ```rust
   pub struct EncoderConfig {
       // ... existing ...
       full_range: Option<bool>,
       color_primaries: Option<ColorPrimaries>,
       transfer_characteristics: Option<TransferCharacteristics>,
       color_matrix: Option<ColorMatrix>,
   }
   ```
3. Apply in `reinit()` to `sSpatialLayers[0]`
4. Submit PR upstream while using fork immediately
5. Switch to upstream once merged

**Option B: Raw API Wrapper**
- Use `Encoder::raw_api()` to access parameters directly
- Less clean but no fork required

**Estimated Effort:** 1-2 days

### Phase 2: Unified ColorSpaceConfig
**Goal:** Single color configuration flows through all paths

1. Create `ColorSpaceConfig` struct (see architecture section)
2. Update `EgfxConfig` with proper color options
3. Pass through to all encoder backends
4. Auto-select based on resolution (HD→BT.709, SD→BT.601)

**Estimated Effort:** 1 day

### Phase 3: VA-API Color Enhancement
**Goal:** Hardware encoder with configurable color

1. Pass `ColorSpaceConfig` to `VaapiEncoder::new()`
2. Update `bgra_to_nv12()` to support configurable matrix:
   ```rust
   fn bgra_to_nv12(data: &[u8], config: &ColorSpaceConfig) -> Vec<u8> {
       let (kr, kg, kb) = match config.matrix {
           ColorMatrix::BT709 => (0.2126, 0.7152, 0.0722),
           ColorMatrix::BT601 => (0.299, 0.587, 0.114),
       };
       // Apply conversion with selected matrix
   }
   ```
3. Set VA-API VUI fields in sequence parameters
4. Test with FFprobe to verify VUI correctness

**Estimated Effort:** 1-2 days

### Phase 4: NVENC Color Fix (Critical Change Required)
**Goal:** Accurate color encoding with NVIDIA GPUs

**The Problem:** NVENC uses BT.601 internally for RGB input - we cannot change this.

**The Solution:** Pre-convert BGRA→NV12 ourselves, feed NVENC YUV data:

```rust
// Current (WRONG for BT.709):
pub fn encode(&mut self, bgra: &[u8]) {
    self.nvenc.encode_argb(bgra);  // Internal BT.601 conversion
}

// Fixed (CORRECT):
pub fn encode(&mut self, bgra: &[u8], config: &ColorSpaceConfig) {
    let nv12 = bgra_to_nv12_with_matrix(bgra, config);  // Our conversion
    self.nvenc.encode_nv12(nv12);  // No internal conversion
    self.set_vui(config);  // VUI matches our conversion
}
```

**Implementation Steps:**
1. Add `bgra_to_nv12()` function for NVENC path (can reuse VA-API code)
2. Change NVENC input format from ARGB to NV12
3. Set VUI parameters to match our conversion
4. Benchmark performance impact (CPU conversion vs GPU)

**Estimated Effort:** 2-3 days

### Phase 5: Upstream Contributions
**Goal:** Give back to community

1. **openh264-rs PR:** VUI configuration (likely accepted quickly)
2. **Documentation:** Publish NVENC color findings (valuable to community)
3. **cros-libva:** Consider contributing VUI improvements if relevant

**Estimated Effort:** Ongoing

---

## Revised Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    EgfxConfig (TOML)                            │
│  color_space: "auto" | "bt709" | "bt601" | "srgb"              │
│  color_range: "auto" | "full" | "limited"                       │
└─────────────────────────────────┬───────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                    ColorSpaceConfig                             │
│  Computed once, passed to all encoders                          │
└─────────────────────────────────┬───────────────────────────────┘
                                  │
        ┌─────────────────────────┼─────────────────────────┐
        │                         │                         │
        ▼                         ▼                         ▼
┌───────────────┐         ┌───────────────┐         ┌───────────────┐
│   OpenH264    │         │    VA-API     │         │    NVENC      │
│               │         │               │         │               │
│ VUI via       │         │ Our bgra→nv12 │         │ Our bgra→nv12 │
│ EncoderConfig │         │ VA-API VUI    │         │ NVENC VUI     │
│ (fork/PR)     │         │               │         │ (NV12 input!) │
│               │         │               │         │               │
│ ✓ Conversion  │         │ ✓ Conversion  │         │ ✓ Conversion  │
│   matches VUI │         │   matches VUI │         │   matches VUI │
└───────────────┘         └───────────────┘         └───────────────┘
```

**Key Insight:** All three paths now do BGRA→YUV conversion ourselves (or via VUI-aware OpenH264), ensuring VUI metadata always matches actual pixel data.

---

## Testing Strategy

### FFprobe Verification
```bash
ffprobe -show_streams -select_streams v output.h264 2>&1 | grep -E "(color|range)"
# Expected output:
# color_range=tv (limited) or pc (full)
# color_space=bt709 or bt601
# color_primaries=bt709 or smpte170m
# color_transfer=bt709 or smpte170m
```

### Visual Comparison
1. Capture reference image with known colors
2. Encode with each encoder backend
3. Decode and compare color values
4. Measure ΔE (color difference) values

### RDP Client Testing
1. Windows Remote Desktop (mstsc.exe)
2. FreeRDP client
3. Remmina
4. Verify consistent rendering across clients

---

## Open Questions (Resolved)

1. **Full Range vs Limited:** ✅ RESOLVED
   - Default: Limited range for broadcast/TV compatibility
   - Option: Full range for screen content (make configurable)
   - Signal correctly via `videoFullRangeFlag` in VUI

2. **sRGB vs BT.709:** ✅ RESOLVED
   - sRGB for computer graphics is best default
   - Treat desktop capture as sRGB-ish / Rec.709-ish
   - Signal sRGB transfer (13) or BT.709 (1) based on config
   - User preference: Support ALL options

3. **NVENC Priority:** ✅ RESOLVED
   - NVENC VUI IS exposed via `NV_ENC_CONFIG_H264_VUI_PARAMETERS`
   - Solution: Do our own RGB→NV12 conversion, feed NVENC YUV
   - Full-GPU pipeline possible with custom CUDA kernel

4. **Upstream Strategy:** ✅ RESOLVED
   - Fork openh264-rs, prepare VUI additions
   - Submit upstream PR (maintainer is active and receptive)
   - Use fork immediately while PR is pending

---

## References

- [MS-RDPEGFX Specification](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/)
- [ITU-T H.264 Annex E (VUI)](https://www.itu.int/rec/T-REC-H.264)
- [OpenH264 GitHub](https://github.com/cisco/openh264)
- [openh264-rs GitHub](https://github.com/ralfbiedert/openh264-rs)
- [NVIDIA Video Codec SDK](https://developer.nvidia.com/nvidia-video-codec-sdk)
- [FFmpeg NVENC Implementation](https://github.com/FFmpeg/FFmpeg/blob/master/libavcodec/nvenc_h264.c)

---

## Changelog

- **2025-12-27:** Initial document created from research session
