# NVENC Hardware Encoding and Color Infrastructure

This document provides comprehensive technical documentation for the NVENC hardware encoding implementation and the color space infrastructure used in this RDP server.

## Table of Contents

1. [NVENC Move-Invalidation Problem and Box Workaround](#nvenc-move-invalidation-problem-and-box-workaround)
2. [Color Space Infrastructure](#color-space-infrastructure)
3. [Color Conversion Pipeline](#color-conversion-pipeline)
4. [Integration with RDP/EGFX](#integration-with-rdpegfx)

---

## NVENC Move-Invalidation Problem and Box Workaround

### The Problem

The `nvidia-video-codec-sdk` Rust crate provides safe wrappers around NVIDIA's Video Codec SDK (NVENC). However, these types contain **internal raw pointers** that become invalid when the struct is moved in memory.

**Affected types:**
- `Session` - Contains a pointer to the NVENC encoder
- `Buffer` - Contains a pointer to the input buffer
- `Bitstream` - Contains a pointer to the output bitstream

**When does this happen?**

```rust
// This works fine - everything stays in scope
fn encode_in_place() {
    let session = encoder.start_session(...);
    let buffer = session.create_input_buffer();  // buffer has pointer into session
    // ... use buffer and session ...
} // all dropped here - OK

// This FAILS - struct is moved when returned
fn create_encoder() -> NvencEncoder {
    let session = encoder.start_session(...);
    let buffer = session.create_input_buffer();
    NvencEncoder { session, buffer }  // Move invalidates internal pointers!
}
```

When `NvencEncoder` is returned from a function, the struct is **moved** to a new memory location. The `Buffer` and `Bitstream` types contain pointers to memory managed by the `Session`, but these pointers still point to the **old** memory location.

**Symptoms:**
- `InvalidEncoderDevice` error during `encode_picture`
- `DeviceNotExist` error during `Buffer::lock()` or `Bitstream::lock()`
- Segmentation faults during cleanup/drop

### The Solution: Boxing

By wrapping the NVENC types in `Box`, we heap-allocate them immediately after creation. This ensures they **never move** - only the Box pointer moves, while the actual data stays at a stable heap address.

```rust
pub struct NvencEncoder {
    // Option<Box> for explicit drop control
    input_buffers: Vec<Option<Box<Buffer<'static>>>>,
    output_bitstreams: Vec<Option<Box<Bitstream<'static>>>>,
    cuda_ctx: Arc<CudaContext>,
    // ... other fields ...
    // MUST BE LAST - buffers reference the session
    session: Box<Session>,
}
```

**Key implementation details:**

1. **Box immediately after creation:**
   ```rust
   let session = Box::new(encoder.start_session(...)?);
   let input = session.create_input_buffer()?;
   let input: Buffer<'static> = unsafe { std::mem::transmute(input) };
   input_buffers.push(Some(Box::new(input)));
   ```

2. **Use `Option<Box>` for explicit drop control:**
   ```rust
   // In encode_bgra:
   let input_buffer = self.input_buffers[buf_idx]
       .as_mut()
       .ok_or_else(|| HardwareEncoderError::EncodeFailed("Input buffer was dropped".to_string()))?;

   // Use double-deref to get &mut Buffer
   self.session.encode_picture(&mut **input_buffer, &mut **output_bitstream, params)?;
   ```

3. **Explicit drop order in Drop implementation:**
   ```rust
   impl Drop for NvencEncoder {
       fn drop(&mut self) {
           // CRITICAL: Rebind CUDA context first
           if let Err(e) = self.cuda_ctx.bind_to_thread() {
               warn!("Failed to bind CUDA context during drop");
               return;
           }

           // Drop buffers BEFORE session by setting to None
           for buf in &mut self.input_buffers {
               *buf = None;
           }
           for buf in &mut self.output_bitstreams {
               *buf = None;
           }
           // session drops automatically after this
       }
   }
   ```

### CUDA Context Binding

A related issue: CUDA contexts are **thread-local**. Between the last `encode_bgra` call and the `Drop`, the context may have been unbound (e.g., thread switch, or another CUDA operation).

The `Buffer` and `Bitstream` destructors need a valid CUDA context to release GPU resources. Without it, you get `DeviceNotExist` errors.

**Solution:** Always bind the CUDA context in Drop:
```rust
if let Err(e) = self.cuda_ctx.bind_to_thread() {
    warn!("Failed to bind CUDA context during drop - GPU resources may leak");
    return;
}
```

### Field Ordering

Rust drops struct fields **in declaration order**. Since buffers reference the session's encoder, they must be dropped first.

```rust
pub struct NvencEncoder {
    input_buffers: Vec<...>,      // Dropped first
    output_bitstreams: Vec<...>,  // Dropped second
    cuda_ctx: Arc<...>,           // Dropped third
    // ... other fields ...
    session: Box<Session>,         // DROPPED LAST
}
```

### Lifetime Transmutation

The `Buffer<'a>` and `Bitstream<'a>` types have a lifetime parameter tied to the `Session`. Since we're managing lifetimes manually (buffers in Options, dropped before session), we transmute to `'static`:

```rust
let input = session.create_input_buffer()?;
let input: Buffer<'static> = unsafe { std::mem::transmute(input) };
```

**Safety:** This is safe because:
1. The session outlives the buffers (field ordering + explicit drop)
2. The Box prevents any memory moves
3. We never create dangling references

---

## Color Space Infrastructure

### Overview

The color space system provides unified handling of:
- **H.264 VUI parameters** - Embedded in SPS, tells decoders how to interpret color
- **RGB to YUV conversion matrices** - For software color conversion
- **Full vs limited range** - PC (0-255) vs TV (16-235) value ranges

### H.264 VUI (Video Usability Information)

VUI parameters are embedded in the SPS NAL unit and signal to decoders:

| Parameter | Description | Location |
|-----------|-------------|----------|
| `colour_primaries` | RGB chromaticity coordinates | SPS VUI |
| `transfer_characteristics` | Gamma/OETF curve | SPS VUI |
| `matrix_coefficients` | RGB↔YCbCr conversion matrix | SPS VUI |
| `video_full_range_flag` | Limited (0) or Full (1) range | SPS VUI |

### Color Space Standards

| Standard | Use Case | Primaries | Transfer | Matrix |
|----------|----------|-----------|----------|--------|
| **BT.709** | HD (1080p, 720p) | 1 | 1 | 1 |
| **BT.601** | SD (480p, 576p) | 6 (SMPTE 170M) | 6 | 6 |
| **sRGB** | PC/Web content | 1 (BT.709) | 13 (sRGB) | 1 |
| **BT.2020** | UHD/4K/HDR | 9 | 14/15 | 9 |

### ColorSpaceConfig API

```rust
use lamco_rdp_server::egfx::color_space::{ColorSpaceConfig, ColorSpacePreset};

// Auto-detect based on resolution
let config = ColorSpaceConfig::from_resolution(1920, 1080);  // → BT.709 limited

// Use a preset
let config = ColorSpaceConfig::from_preset(ColorSpacePreset::SRGB);  // → sRGB full

// Preset with range override
let config = ColorSpaceConfig::from_preset_with_range(
    ColorSpacePreset::BT709,
    ColorRange::Full
);

// Full customization
let config = ColorSpaceConfig::bt709_full();

// Parse from config file
let config = ColorSpaceConfig::from_config(
    Some("auto"),    // preset
    Some("full"),    // range override
    1920, 1080       // resolution for auto
);
```

### Accessing VUI Parameters

```rust
let config = ColorSpaceConfig::bt709_limited();

// Get all VUI params as tuple
let (primaries, transfer, matrix, full_range) = config.vui_params();
// → (1, 1, 1, false)

// Get individual values
config.vui_colour_primaries();       // → 1
config.vui_transfer_characteristics(); // → 1
config.vui_matrix_coefficients();     // → 1
config.vui_full_range_flag();         // → false
```

### Luma Coefficients for RGB→Y

```rust
// Floating-point (Kr, Kg, Kb)
let (kr, kg, kb) = config.luma_coefficients();
// BT.709: (0.2126, 0.7152, 0.0722)
// BT.601: (0.299, 0.587, 0.114)

// Fixed-point for SIMD (scaled by 65536)
let (kr_fx, kg_fx, kb_fx) = config.luma_coefficients_fixed();
// BT.709: (13933, 46871, 4732)
```

### Full vs Limited Range

| Range | Y Min | Y Max | UV Min | UV Max | Use Case |
|-------|-------|-------|--------|--------|----------|
| **Limited** | 16 | 235 | 16 | 240 | Broadcast/TV |
| **Full** | 0 | 255 | 0 | 255 | PC/Graphics |

Limited range reserves headroom/footroom for signal processing. Full range uses all 256 values.

---

## Color Conversion Pipeline

### BGRA → YUV444 Conversion

The `color_convert` module provides high-performance color space conversion:

```
BGRA (from PipeWire)
       │
       ▼
color_convert::bgra_to_yuv444()
       │
       ├── AVX2 (x86_64, 8 pixels/iteration)
       ├── NEON (AArch64, 8 pixels/iteration)
       └── Scalar fallback
       │
       ▼
Yuv444Frame { y, u, v, width, height }
```

### ColorMatrix Options

```rust
pub enum ColorMatrix {
    /// BT.601 - SD content, FULL RANGE
    BT601,

    /// BT.709 - HD content, FULL RANGE
    BT709,

    /// OpenH264-compatible - LIMITED RANGE
    /// Y: 16-235, UV: 16-240
    OpenH264,
}

// Auto-select based on resolution
let matrix = ColorMatrix::auto_select(1920, 1080);  // → BT709
let matrix = ColorMatrix::auto_select(640, 480);    // → BT601
```

### Conversion Formulas

**BT.709 (full range):**
```
Y  =  0.2126 R + 0.7152 G + 0.0722 B
Cb = -0.1146 R - 0.3854 G + 0.5    B + 128
Cr =  0.5    R - 0.4542 G - 0.0458 B + 128
```

**BT.601 (full range):**
```
Y  =  0.299  R + 0.587  G + 0.114  B
Cb = -0.1687 R - 0.3313 G + 0.5    B + 128
Cr =  0.5    R - 0.4187 G - 0.0813 B + 128
```

**OpenH264 (limited range):**
```
Y  = (66  R + 129 G + 25  B) / 256 + 16
Cb = (-38 R - 74  G + 112 B) / 256 + 128
Cr = (112 R - 94  G - 18  B) / 256 + 128
```

### Fixed-Point Arithmetic

For SIMD performance, coefficients are scaled by 65536 (16.16 fixed-point):

```rust
// BT.709 Y coefficients
const Y_KR: i32 = 13933;  // 0.2126 × 65536
const Y_KG: i32 = 46871;  // 0.7152 × 65536
const Y_KB: i32 = 4732;   // 0.0722 × 65536

// Compute Y with rounding
let y = (Y_KR * r + Y_KG * g + Y_KB * b + 32768) >> 16;
```

### Chroma Subsampling (4:4:4 → 4:2:0)

For standard H.264 (not AVC444), chroma is subsampled:

```rust
let chroma_420 = subsample_chroma_420(&chroma_444, width, height);
```

Uses a 2×2 box filter with proper rounding:
```
┌──┬──┐
│A │B │  output = (A + B + C + D + 2) / 4
├──┼──┤
│C │D │
└──┴──┘
```

### SIMD Implementations

**AVX2 (x86_64):**
- Processes 8 BGRA pixels per iteration
- Uses 256-bit registers for parallel computation
- Shuffle+unpack for BGRA deinterleaving
- ~4× speedup over scalar

**NEON (AArch64):**
- Processes 8 pixels per iteration
- Uses `vld4_u8` for automatic BGRA deinterleaving
- `vmla` (multiply-accumulate) for coefficient application
- ~4× speedup over scalar

---

## Integration with RDP/EGFX

### Encoding Pipeline

```
PipeWire Capture (BGRA)
         │
         ▼
┌─────────────────────────────────────────┐
│ Software Path (OpenH264)                │
│                                         │
│   BGRA → bgra_to_yuv444(OpenH264)       │
│        → encode_slices() → H.264        │
└─────────────────────────────────────────┘
         │
         or
         │
┌─────────────────────────────────────────┐
│ Hardware Path (NVENC)                   │
│                                         │
│   BGRA → NvencEncoder::encode_bgra()    │
│        → NVENC internal conversion      │
│        → H.264                          │
└─────────────────────────────────────────┘
         │
         ▼
RDP EGFX H264_AVC420 / H264_AVC444
```

### SPS/PPS and Color Signaling

For correct color rendering on the client, VUI (Video Usability Information) parameters
must be embedded in the H.264 SPS NAL unit. This tells decoders how to interpret the
YUV color data.

#### OpenH264 with VuiConfig

We use a [forked openh264-rs](https://github.com/glamberson/openh264-rs/tree/feature/vui-support)
with VUI support ([PR #86](https://github.com/ralfbiedert/openh264-rs/pull/86)):

```rust
use openh264::encoder::{EncoderConfig, VuiConfig};

// For desktop/screen content: sRGB with full range (0-255)
let config = EncoderConfig::new()
    .vui(VuiConfig::srgb());  // BT.709 primaries + sRGB transfer + full range

// For video content: BT.709 limited range (16-235)
let config = EncoderConfig::new()
    .vui(VuiConfig::bt709());
```

**VuiConfig Presets:**

| Preset | Primaries | Transfer | Matrix | Range | Use Case |
|--------|-----------|----------|--------|-------|----------|
| `srgb()` | BT.709 | sRGB | BT.709 | Full | Desktop, web, screen capture |
| `bt709()` | BT.709 | BT.709 | BT.709 | Limited | HD video content |
| `bt709_full()` | BT.709 | BT.709 | BT.709 | Full | HD with PC range |
| `bt601()` | SMPTE170M | SMPTE170M | SMPTE170M | Limited | SD video content |
| `bt2020()` | BT.2020 | BT.2020 | BT.2020 | Limited | UHD/HDR content |

**Critical:** The color conversion matrix must match the VUI signaling:
- `VuiConfig::srgb()` → use `ColorMatrix::BT709` (full range 0-255)
- `VuiConfig::bt709()` → use `ColorMatrix::OpenH264` (limited range 16-235)

#### NVENC

VUI is embedded in generated SPS (configure during init)

### AVC444 Dual-Stream Format

MS-RDPEGFX AVC444 uses two YUV420 streams:

```
YUV444 Source
     │
     ├── Stream 1: Y₀ + subsampled_U + subsampled_V
     │             (standard YUV420)
     │
     └── Stream 2: Y₁ + high-frequency chroma
                   (chroma residual data)

Client reconstructs: Y₀ + Y₁ → luma, merge chroma → full 4:4:4
```

The `yuv444_packing` module handles this split:
```rust
let (stream1, stream2) = split_yuv444_to_dual_420(&yuv444_frame);
```

---

## Summary

### Key Takeaways

1. **NVENC types are NOT move-safe** - Always Box immediately after creation
2. **CUDA contexts are thread-local** - Rebind before any NVENC operation
3. **Drop order matters** - Buffers before session
4. **Color space affects quality** - Match source content (sRGB for desktop, BT.709 for video)
5. **Full vs limited range** - Use full for PC content to preserve all color values
6. **SIMD is critical for performance** - 4× speedup with AVX2/NEON

### Quick Reference

| Task | Solution |
|------|----------|
| NVENC crashes after function return | Box all NVENC types |
| NVENC crashes during Drop | Bind CUDA context, drop buffers before session |
| Colors look wrong | Check VUI parameters match conversion matrix |
| Banding/clipping | Verify full vs limited range consistency |
| Slow encoding | Enable SIMD (compile with target-cpu=native) |

### Files

| File | Purpose |
|------|---------|
| `src/egfx/avc444_encoder.rs` | AVC444 encoder with VuiConfig for full-range sRGB |
| `src/egfx/hardware/nvenc/mod.rs` | NVENC encoder with Box workaround |
| `src/egfx/color_space.rs` | VUI parameters and color space presets |
| `src/egfx/color_convert.rs` | BGRA→YUV444 with SIMD (full/limited range) |
| `src/egfx/yuv444_packing.rs` | AVC444 dual-stream packing |

### Dependencies

The project uses a forked `openh264-rs` for VUI support:

```toml
# Cargo.toml
openh264 = { git = "https://github.com/glamberson/openh264-rs.git", branch = "feature/vui-support" }
```

This fork adds `VuiConfig` for proper color space signaling in H.264 streams.
Once [PR #86](https://github.com/ralfbiedert/openh264-rs/pull/86) is merged upstream,
we can switch back to the official crate.
