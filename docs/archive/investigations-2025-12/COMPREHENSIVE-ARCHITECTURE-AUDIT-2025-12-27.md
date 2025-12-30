# Comprehensive Architecture Audit - December 27, 2025

This document provides an exhaustive empirical analysis of the `lamco-rdp-server` architecture, based on direct code inspection. It serves as the authoritative reference for understanding codebase boundaries, color infrastructure, premium features, and their exceptions.

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Multi-Repository Architecture](#multi-repository-architecture)
3. [Codebase Boundaries and Dependencies](#codebase-boundaries-and-dependencies)
4. [Premium Features and Boundary Exceptions](#premium-features-and-boundary-exceptions)
5. [Color Infrastructure (Complete Analysis)](#color-infrastructure-complete-analysis)
6. [Hardware Encoding Architecture](#hardware-encoding-architecture)
7. [AVC444 Dual-Stream Implementation](#avc444-dual-stream-implementation)
8. [Implementation vs Documentation Comparison](#implementation-vs-documentation-comparison)
9. [Issues and Recommendations](#issues-and-recommendations)
10. [File Reference Matrix](#file-reference-matrix)

---

## Executive Summary

The `lamco-rdp-server` project is a Wayland RDP server with:

- **6 published lamco crates** on crates.io
- **2 local path dependencies** (with documented reasons)
- **11 patched IronRDP crates** (pending upstream PRs)
- **3 premium feature flags** (vaapi, nvenc, hardware-encoding)
- **Complete color infrastructure** with SIMD-optimized conversions
- **MS-RDPEGFX compliant** AVC444 dual-stream packing

### Key Findings

1. **Color Infrastructure**: Fully implemented with BT.601/BT.709/sRGB support, VUI parameters, and SIMD (AVX2/NEON)
2. **NVENC Integration**: Working with Box workaround for move-invalidation bug
3. **VA-API Integration**: Uses `cros-libva` with ColorSpaceConfig integration
4. **Boundary Violations**: Two justified local path dependencies (lamco-pipewire, lamco-rdp-clipboard)
5. **Documentation Gap**: `01-ARCHITECTURE.md` predates EGFX/H.264 implementation; needs update

---

## Multi-Repository Architecture

### Repository Layout

```
wayland/
├── wrd-server-specs/           # This repo - lamco-rdp-server
│   ├── src/                    # Main server code
│   ├── docs/                   # Documentation
│   └── Cargo.toml              # Dependencies and features
│
├── lamco-rdp-workspace/        # Published lamco crates workspace
│   └── crates/
│       ├── lamco-clipboard-core/   # Published
│       ├── lamco-pipewire/         # Local dep (unpublished fix)
│       ├── lamco-rdp-clipboard/    # Local dep (IronRDP coupling)
│       └── lamco-rdp-input/        # Published
│
└── IronRDP/                    # Local fork of Devolutions IronRDP
    └── crates/
        ├── ironrdp/
        ├── ironrdp-server/
        ├── ironrdp-egfx/
        ├── ironrdp-cliprdr/     # File transfer methods added
        └── ... (11 crates total)
```

### Data Flow Between Repositories

```
[PipeWire] ──→ lamco-pipewire ──→ lamco-rdp-server ──→ IronRDP ──→ [RDP Client]
                    │                    │
                    │                    ├──→ lamco-portal (D-Bus)
                    │                    ├──→ lamco-video (frames)
                    │                    ├──→ lamco-rdp-input (keyboard/mouse)
                    │                    └──→ lamco-rdp-clipboard (cliprdr)
                    │
                    └──→ VideoFrame { bgra, width, height, timestamp }
```

---

## Codebase Boundaries and Dependencies

### Published Dependencies (crates.io)

| Crate | Version | Purpose | Notes |
|-------|---------|---------|-------|
| `lamco-wayland` | 0.2.2 | Wayland client bindings | Stable |
| `lamco-rdp` | 0.4.0 | RDP protocol utilities | Stable |
| `lamco-portal` | 0.2.2 | XDG Desktop Portal integration | Includes EAGAIN fix |
| `lamco-video` | 0.1.2 | Video frame processing | Stable |
| `lamco-rdp-input` | 0.1.1 | RDP input event translation | Stable |
| `lamco-clipboard-core` | 0.4.0 | Clipboard primitives | FileDescriptor support |

### Local Path Dependencies (Exceptions)

#### 1. `lamco-pipewire` (path dependency)

**Location:** `../lamco-rdp-workspace/crates/lamco-pipewire`

**Reason:** Zero-size buffer validation fix not yet published.

```toml
# Cargo.toml lines 43-44:
# Local fork with fix for zero-size buffer passthrough bug
# See docs/QUALITY-ISSUE-ANALYSIS-2025-12-27.md for details
# TODO: Publish fix to crates.io and revert to version dependency
lamco-pipewire = { path = "../lamco-rdp-workspace/crates/lamco-pipewire" }
```

**Resolution Path:** Publish fix to crates.io, update to version dependency.

#### 2. `lamco-rdp-clipboard` (path dependency)

**Location:** `../lamco-rdp-workspace/crates/lamco-rdp-clipboard`

**Reason:** Tightly coupled to patched `ironrdp-cliprdr` for file transfer methods.

```toml
# Cargo.toml lines 48-49:
# Local path required - tightly coupled to patched ironrdp-cliprdr
lamco-rdp-clipboard = { path = "../lamco-rdp-workspace/crates/lamco-rdp-clipboard" }
```

**Why This Cannot Be Published:**
1. `lamco-rdp-clipboard` implements `CliprdrBackend` trait from `ironrdp-cliprdr`
2. We patch `ironrdp-cliprdr` with file transfer methods (PRs #1064-1066)
3. If published, it would depend on upstream `ironrdp-cliprdr` without these methods
4. Trait implementations would fail to compile

**Resolution Path:** Wait for IronRDP PRs to merge upstream, then publish.

### IronRDP Fork (11 Patched Crates)

**Fork Location:** `/home/greg/wayland/IronRDP`
**Branch:** `pr3-file-contents-response-v2`

**Patched Crates:**
1. `ironrdp`
2. `ironrdp-pdu`
3. `ironrdp-server`
4. `ironrdp-graphics`
5. `ironrdp-cliprdr`
6. `ironrdp-svc`
7. `ironrdp-dvc`
8. `ironrdp-core`
9. `ironrdp-tokio`
10. `ironrdp-displaycontrol`
11. `ironrdp-egfx`

**Pending PRs:**
- PR #1057: EGFX support
- PR #1064: `lock_clipboard()` / `unlock_clipboard()`
- PR #1065: `request_file_contents()`
- PR #1066: `SendFileContentsResponse`

**Why All Crates Patched:**
```toml
# Cargo.toml line 296:
# All crates patched to same source to avoid trait conflicts
```

Rust's coherence rules require that if you patch one crate, all crates using its types must be patched to the same source.

---

## Premium Features and Boundary Exceptions

### Feature Flags Overview

```toml
[features]
default = ["pam-auth", "h264"]

# Software H.264 (included by default)
h264 = ["openh264", "openh264-sys2"]

# Premium: Hardware-accelerated encoding
vaapi = ["cros-libva"]                  # Intel/AMD GPU
nvenc = ["nvidia-video-codec-sdk", "cudarc"]  # NVIDIA GPU
hardware-encoding = ["vaapi", "nvenc"]  # Both backends
```

### Premium Feature: Hardware Encoding

#### VA-API Backend (`vaapi` feature)

**Compile Requirements:**
- `libva-dev` >= 1.20.0
- Intel iHD driver (modern Intel) or i965 (older Intel)
- AMD radeonsi driver

**Runtime Requirements:**
- Appropriate GPU drivers loaded
- DRM device access (`/dev/dri/renderD*`)

**Code Path:**
```
src/egfx/hardware/vaapi/mod.rs (926 lines)
├── VaapiEncoder::new()         # Initialize VA-API context
├── VaapiEncoder::encode_bgra() # BGRA → NV12 → H.264
└── bgra_to_nv12()              # Color conversion with ColorSpaceConfig
```

**ColorSpaceConfig Integration:**
```rust
// vaapi/mod.rs:
color_space: ColorSpaceConfig,

// In new():
let color_space = ColorSpaceConfig::from_resolution(width, height);
```

#### NVENC Backend (`nvenc` feature)

**Compile Requirements:**
- `nvidia-video-codec-sdk` crate (links to NVENC API)
- `cudarc` crate (CUDA runtime)
- CUDA toolkit headers

**Runtime Requirements:**
- NVIDIA driver with `libnvidia-encode.so`
- CUDA toolkit libraries
- NVENC-capable GPU (GTX 6xx+, any RTX)

**Code Path:**
```
src/egfx/hardware/nvenc/mod.rs (802 lines)
├── NvencEncoder::new()         # CUDA context + NVENC session
├── NvencEncoder::encode_bgra() # Upload to GPU, encode
└── Drop::drop()                # CUDA context rebind, explicit drop order
```

**Critical Implementation Detail - Box Workaround:**
```rust
pub struct NvencEncoder {
    // Option<Box> for explicit drop control
    input_buffers: Vec<Option<Box<Buffer<'static>>>>,
    output_bitstreams: Vec<Option<Box<Bitstream<'static>>>>,
    cuda_ctx: Arc<CudaContext>,
    // ... other fields ...
    session: Box<Session>,  // MUST BE LAST - buffers reference session
}
```

This prevents move-invalidation of internal pointers. See `docs/architecture/NVENC-AND-COLOR-INFRASTRUCTURE.md` for details.

### Feature Matrix

| Feature | License | Code Location | Dependencies |
|---------|---------|---------------|--------------|
| `h264` (default) | BSD (OpenH264) | `src/egfx/encoder.rs` | openh264, openh264-sys2 |
| `vaapi` | BUSL-1.1 | `src/egfx/hardware/vaapi/` | cros-libva |
| `nvenc` | BUSL-1.1 | `src/egfx/hardware/nvenc/` | nvidia-video-codec-sdk, cudarc |

---

## Color Infrastructure (Complete Analysis)

### Module Structure

```
src/egfx/
├── color_space.rs      # VUI parameters, color space presets
├── color_convert.rs    # BGRA→YUV444 with SIMD
├── yuv444_packing.rs   # AVC444 dual-stream packing
└── mod.rs              # Public API exports
```

### color_space.rs - VUI Parameters and Presets

**Purpose:** Unified handling of H.264 VUI (Video Usability Information) parameters.

**Key Types:**

```rust
/// Color primaries (chromaticity coordinates)
pub enum ColorPrimaries {
    BT709 = 1,      // HD, sRGB
    BT601_625 = 5,  // PAL
    BT601_525 = 6,  // NTSC (SMPTE 170M)
    BT2020 = 9,     // UHD
}

/// Transfer characteristics (gamma/OETF)
pub enum TransferCharacteristics {
    BT709 = 1,      // Standard gamma
    SRGB = 13,      // sRGB non-linear
    BT2020_10 = 14, // HDR10
    BT2020_12 = 15, // HDR12
}

/// Matrix coefficients (RGB↔YCbCr conversion)
pub enum MatrixCoefficients {
    BT709 = 1,      // HD
    BT601 = 6,      // SD
    BT2020_NCL = 9, // UHD non-constant luminance
}

/// Color range
pub enum ColorRange {
    Limited,  // Y: 16-235, UV: 16-240 (TV)
    Full,     // Y: 0-255, UV: 0-255 (PC)
}

/// Preset configurations
pub enum ColorSpacePreset {
    Auto,     // Resolution-based selection
    BT709,    // HD content
    BT601,    // SD content
    SRGB,     // PC/web content (full range)
    BT2020,   // UHD content
    Custom,   // Manual configuration
}
```

**ColorSpaceConfig API:**

```rust
pub struct ColorSpaceConfig {
    preset: ColorSpacePreset,
    matrix: MatrixCoefficients,
    range: ColorRange,
    primaries: ColorPrimaries,
    transfer: TransferCharacteristics,
}

impl ColorSpaceConfig {
    // Factory methods
    pub fn from_resolution(width: u32, height: u32) -> Self;
    pub fn from_preset(preset: ColorSpacePreset) -> Self;
    pub fn from_preset_with_range(preset: ColorSpacePreset, range: ColorRange) -> Self;
    pub fn from_config(preset: Option<&str>, range: Option<&str>, width: u32, height: u32) -> Self;

    // Common presets
    pub fn bt709_limited() -> Self;
    pub fn bt709_full() -> Self;
    pub fn srgb_full() -> Self;
    pub fn bt601_full() -> Self;

    // VUI parameter accessors
    pub fn vui_params(&self) -> (u8, u8, u8, bool);
    pub fn vui_colour_primaries(&self) -> u8;
    pub fn vui_transfer_characteristics(&self) -> u8;
    pub fn vui_matrix_coefficients(&self) -> u8;
    pub fn vui_full_range_flag(&self) -> bool;

    // Luma coefficients for RGB→Y
    pub fn luma_coefficients(&self) -> (f32, f32, f32);      // (Kr, Kg, Kb)
    pub fn luma_coefficients_fixed(&self) -> (i32, i32, i32); // Scaled by 65536
}
```

### color_convert.rs - BGRA→YUV444 Conversion

**Purpose:** High-performance color space conversion with SIMD acceleration.

**Key Types:**

```rust
/// Color matrix selection
pub enum ColorMatrix {
    BT601,    // SD content, full range
    BT709,    // HD content, full range
    OpenH264, // Limited range (Y: 16-235)
}

/// YUV444 frame (4:4:4 chroma)
pub struct Yuv444Frame {
    pub y: Vec<u8>,     // width × height
    pub u: Vec<u8>,     // width × height
    pub v: Vec<u8>,     // width × height
    pub width: usize,
    pub height: usize,
}
```

**Main Functions:**

```rust
/// Convert BGRA to YUV444 with auto-SIMD dispatch
pub fn bgra_to_yuv444(bgra: &[u8], width: usize, height: usize, matrix: ColorMatrix) -> Yuv444Frame;

/// Subsample chroma from 444 to 420 (2×2 box filter)
pub fn subsample_chroma_420(chroma: &[u8], width: usize, height: usize) -> Vec<u8>;
```

**SIMD Implementations:**

| Architecture | Implementation | Throughput | Speedup |
|--------------|----------------|------------|---------|
| x86_64 + AVX2 | `bgra_to_yuv444_avx2()` | 8 pixels/iter | ~4× |
| AArch64 + NEON | `bgra_to_yuv444_neon()` | 8 pixels/iter | ~4× |
| Fallback | `bgra_to_yuv444_scalar()` | 1 pixel/iter | 1× |

**Conversion Formulas:**

BT.709 (full range):
```
Y  =  0.2126 R + 0.7152 G + 0.0722 B
Cb = -0.1146 R - 0.3854 G + 0.5000 B + 128
Cr =  0.5000 R - 0.4542 G - 0.0458 B + 128
```

BT.601 (full range):
```
Y  =  0.299  R + 0.587  G + 0.114  B
Cb = -0.1687 R - 0.3313 G + 0.5000 B + 128
Cr =  0.5000 R - 0.4187 G - 0.0813 B + 128
```

OpenH264 (limited range, Y: 16-235):
```
Y  = (66  R + 129 G + 25  B) / 256 + 16
Cb = (-38 R - 74  G + 112 B) / 256 + 128
Cr = (112 R - 94  G - 18  B) / 256 + 128
```

### yuv444_packing.rs - AVC444 Dual-Stream Packing

**Purpose:** Implements MS-RDPEGFX Section 3.3.8.3.2 for YUV444→dual YUV420 encoding.

**Key Types:**

```rust
/// YUV420 frame (4:2:0 chroma subsampling)
pub struct Yuv420Frame {
    pub y: Vec<u8>,     // width × height
    pub u: Vec<u8>,     // width/2 × height/2
    pub v: Vec<u8>,     // width/2 × height/2
    pub width: usize,
    pub height: usize,
}
```

**Main Functions:**

```rust
/// Pack YUV444 into main YUV420 view (standard encoding)
pub fn pack_main_view(yuv444: &Yuv444Frame) -> Yuv420Frame;

/// Pack YUV444 into auxiliary YUV420 view (residual chroma)
pub fn pack_auxiliary_view(yuv444: &Yuv444Frame) -> Yuv420Frame;

/// Pack both views at once
pub fn pack_dual_views(yuv444: &Yuv444Frame) -> (Yuv420Frame, Yuv420Frame);
```

**AVC444 Algorithm:**

```
YUV444 Source (full chroma)
     │
     ├── Main View (Stream 1):
     │   • Y: Full luma (unchanged)
     │   • U: 2×2 box filter subsampled
     │   • V: 2×2 box filter subsampled
     │
     └── Auxiliary View (Stream 2):
         • Y: U444 samples at odd positions
         • U: V444 samples at odd positions (subsampled)
         • V: Neutral (128)

Client Reconstruction:
     • Even positions: Use main view chroma
     • Odd positions: Use auxiliary "luma" as chroma
```

---

## Hardware Encoding Architecture

### Unified Interface

```rust
// src/egfx/hardware/mod.rs

/// Unified hardware encoder interface
pub trait HardwareEncoder {
    /// Encode BGRA frame to H.264
    fn encode_bgra(
        &mut self,
        bgra_data: &[u8],
        width: u32,
        height: u32,
        timestamp_ms: u64,
    ) -> HardwareEncoderResult<Option<H264Frame>>;

    /// Force next frame to be keyframe (IDR)
    fn force_keyframe(&mut self);

    /// Get encoder statistics
    fn stats(&self) -> HardwareEncoderStats;

    /// Get backend name for logging
    fn backend_name(&self) -> &'static str;
}
```

### Encoder Factory

```rust
/// Create hardware encoder with automatic backend selection
pub fn create_hardware_encoder(
    config: &HardwareEncodingConfig,
    width: u32,
    height: u32,
) -> HardwareEncoderResult<Box<dyn HardwareEncoder>>;
```

### Quality Presets

```rust
pub enum QualityPreset {
    Speed,      // P2, UltraLowLatency, 3 Mbps, GOP 60
    Balanced,   // P4, LowLatency, 5 Mbps, GOP 30 (default)
    Quality,    // P6, Default, 10 Mbps, GOP 15
}
```

---

## AVC444 Dual-Stream Implementation

### avc444_encoder.rs Overview

**Purpose:** Dual OpenH264 encoders for AVC444 (4:4:4 chroma).

```rust
pub struct Avc444Encoder {
    main_encoder: Encoder,      // Standard YUV420 encoder
    aux_encoder: Encoder,       // Auxiliary stream encoder
    color_matrix: ColorMatrix,  // Auto-selected based on resolution
    // ... timing and stats fields
}
```

### Encoding Pipeline

```
BGRA Frame (from PipeWire)
     │
     ▼
bgra_to_yuv444(matrix)
     │
     ▼
YUV444Frame { y, u, v }
     │
     ├──────────────────────────────────────┐
     │                                      │
     ▼                                      ▼
pack_main_view()                    pack_auxiliary_view()
     │                                      │
     ▼                                      ▼
main_encoder.encode()               aux_encoder.encode()
     │                                      │
     ▼                                      ▼
Avc444Frame {                              │
    main_stream: H264Frame,     ◄──────────┘
    aux_stream: H264Frame,
}
```

---

## Implementation vs Documentation Comparison

### docs/specs/01-ARCHITECTURE.md Analysis

**Status:** Partially outdated

**What's Correct:**
- High-level architecture diagram
- Portal integration layer
- Input/clipboard handlers
- Threading model

**What's Missing:**
- EGFX/H.264 video pipeline
- Color infrastructure
- Hardware encoding backends
- AVC444 dual-stream encoding
- Damage tracking

**What's Outdated:**
- "Bitmap Updates" terminology (now H.264 frames)
- DisplayUpdateHandler interface (now uses EgfxVideoHandler)
- No mention of color space handling

### docs/architecture/NVENC-AND-COLOR-INFRASTRUCTURE.md Analysis

**Status:** Current and accurate

**Coverage:**
- ✅ NVENC move-invalidation problem
- ✅ Box workaround with Option
- ✅ CUDA context binding
- ✅ Drop order requirements
- ✅ Color space standards
- ✅ ColorSpaceConfig API
- ✅ VUI parameters
- ✅ SIMD conversion details
- ✅ AVC444 dual-stream format

---

## Issues and Recommendations

### Critical Issues

1. **None identified** - The implementation is solid

### Moderate Issues

1. **Documentation Gap:** `01-ARCHITECTURE.md` needs EGFX/H.264 section
2. **Local Dependencies:** Two crates blocked on upstream (acceptable)
3. **IronRDP Fork:** 11 crates patched (required for file transfer)

### Recommendations

1. **Short-term:**
   - Publish `lamco-pipewire` fix to crates.io
   - Update `01-ARCHITECTURE.md` with EGFX section

2. **Medium-term:**
   - Follow up on IronRDP PRs #1064-1066
   - When merged, publish `lamco-rdp-clipboard`
   - Remove all IronRDP patches

3. **Long-term:**
   - Consider adding VPU/VK_VIDEO backend for future GPUs
   - Add HDR10 support (BT.2020 + PQ transfer)

---

## File Reference Matrix

### Core Color Infrastructure

| File | Lines | Purpose |
|------|-------|---------|
| `src/egfx/color_space.rs` | ~700 | VUI parameters, presets, ColorSpaceConfig |
| `src/egfx/color_convert.rs` | ~1254 | BGRA→YUV444 with SIMD |
| `src/egfx/yuv444_packing.rs` | ~916 | AVC444 dual-stream packing |
| `src/egfx/mod.rs` | 103 | Public API, module re-exports |

### Hardware Encoding

| File | Lines | Purpose |
|------|-------|---------|
| `src/egfx/hardware/mod.rs` | 344 | HardwareEncoder trait, QualityPreset |
| `src/egfx/hardware/nvenc/mod.rs` | 802 | NVIDIA NVENC implementation |
| `src/egfx/hardware/vaapi/mod.rs` | 926 | Intel/AMD VA-API implementation |

### Software Encoding

| File | Lines | Purpose |
|------|-------|---------|
| `src/egfx/encoder.rs` | ~800 | OpenH264 AVC420 encoder |
| `src/egfx/avc444_encoder.rs` | 1022 | Dual-encoder AVC444 |

### Key Configuration

| File | Lines | Purpose |
|------|-------|---------|
| `Cargo.toml` | 314 | Dependencies, features, patches |
| `src/lib.rs` | 159 | Module structure, re-exports |

---

## Appendix: Build Commands

### Default Build (Software H.264)
```bash
cargo build --release
```

### With VA-API (Intel/AMD GPU)
```bash
cargo build --release --features vaapi
```

### With NVENC (NVIDIA GPU)
```bash
cargo build --release --features nvenc
```

### All Hardware Backends
```bash
cargo build --release --features hardware-encoding
```

### Verify Features Enabled
```bash
cargo tree -f "{p} {f}" | grep lamco-rdp-server
```

---

**Document Version:** 1.0
**Last Updated:** 2025-12-27
**Author:** Architecture Audit (Automated)
