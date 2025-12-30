# Color Architecture Executive Summary

**Document:** Executive Overview
**Last Updated:** 2025-12-30
**Status:** VUI Complete (OpenH264, NVENC), Color Conversion Only (VAAPI)

---

## Overview

This RDP server implements a unified color management system that ensures H.264 video streams are correctly encoded and signaled across all encoder backends. The architecture solves a fundamental problem: **color conversion (pixel math) and color signaling (VUI metadata) must match**, or clients render colors incorrectly.

---

## The Problem We Solve

```
Without proper color management:

  Desktop Capture     →    Encoder (unknown conversion)    →    Client
      (sRGB)                                                   (wrong colors!)
                              ↓
                      No VUI metadata = decoder guesses
```

**Symptoms of broken color:**
- Pink/magenta artifacts (lavender tint)
- Washed-out or oversaturated colors
- Inconsistent appearance between codec modes

---

## Our Solution: Unified ColorSpaceConfig

```
┌─────────────────────────────────────────────────────────────────────┐
│                      ColorSpaceConfig                                │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │
│  │  Color Matrix   │  │   Color Range   │  │ VUI Parameters  │     │
│  │  (pixel math)   │  │  (value range)  │  │ (H.264 metadata)│     │
│  │                 │  │                 │  │                 │     │
│  │  BT.709 (HD)    │  │  Full (0-255)   │  │  primaries: 1   │     │
│  │  BT.601 (SD)    │  │  Limited(16-235)│  │  transfer: 1    │     │
│  └─────────────────┘  └─────────────────┘  │  matrix: 1      │     │
│                                            └─────────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
                              │
         ┌────────────────────┼────────────────────┐
         │                    │                    │
         ▼                    ▼                    ▼
   ┌───────────┐        ┌───────────┐        ┌───────────┐
   │ OpenH264  │        │  VA-API   │        │   NVENC   │
   │ (Software)│        │ (Intel)   │        │ (NVIDIA)  │
   │           │        │           │        │           │
   │ VuiConfig │        │ CPU color │        │ VUI via   │
   │ presets   │        │ convert   │        │ h264VUI   │
   │ ✅ VUI    │        │ ⚠️ No VUI │        │ ✅ VUI    │
   └───────────┘        └───────────┘        └───────────┘
```

**Key Principle:** One configuration object drives both the pixel conversion AND the VUI signaling, guaranteeing they always match.

---

## Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| **ColorSpaceConfig struct** | ✅ Complete | `src/egfx/color_space.rs` |
| **Color conversion (SIMD)** | ✅ Complete | AVX2, NEON, scalar fallbacks |
| **OpenH264 VUI integration** | ✅ Complete | Uses forked openh264-rs with VuiConfig |
| **EncoderConfig color field** | ✅ Complete | `color_space: Option<ColorSpaceConfig>` |
| **Auto-selection by resolution** | ✅ Complete | HD→BT.709, SD→BT.601 |
| **VAAPI color conversion** | ✅ Complete | CPU BGRA→NV12 with correct matrix |
| **VAAPI VUI integration** | ⛔ Not Possible | VA-API H.264 lacks color VUI fields |
| **NVENC VUI integration** | ✅ Complete | h264VUIParameters configured |
| **Runtime configuration** | 🔄 Planned | config.toml integration |

---

## Color Standards We Support

| Standard | Resolution | Matrix | Range | Use Case |
|----------|------------|--------|-------|----------|
| **BT.709 Full** | ≥720p | BT.709 | 0-255 | Desktop capture (default) |
| **BT.709 Limited** | ≥720p | BT.709 | 16-235 | HD video content |
| **BT.601 Limited** | <720p | BT.601 | 16-235 | SD content |
| **sRGB** | Any | BT.709 | 0-255 | Web/graphics content |

**Auto-selection logic:**
```rust
// From avc444_encoder.rs
let color_space = config.color_space.unwrap_or_else(|| {
    match (config.width, config.height) {
        (Some(w), Some(h)) if w >= 1280 && h >= 720 => ColorSpaceConfig::BT709_FULL,
        (Some(_), Some(_)) => ColorSpaceConfig::BT601_LIMITED,
        _ => ColorSpaceConfig::BT709_FULL,  // Safe default
    }
});
```

---

## Data Flow

```
PipeWire Capture (BGRA, sRGB-ish)
         │
         ▼
┌─────────────────────────────────┐
│   ColorSpaceConfig Selection    │  ← Auto or explicit configuration
│   (based on resolution/config)  │
└─────────────────────────────────┘
         │
         ├──────────────────────────────────────────────┐
         │                                              │
         ▼                                              ▼
┌─────────────────────────┐                 ┌─────────────────────────┐
│   AVC420 Path           │                 │   AVC444 Path           │
│                         │                 │                         │
│   BGRA → color_convert  │                 │   BGRA → color_convert  │
│        (BT.709 matrix)  │                 │        (BT.709 matrix)  │
│            │            │                 │            │            │
│            ▼            │                 │            ▼            │
│   YUV420 → OpenH264     │                 │   YUV444 → Dual-stream  │
│   (VuiConfig::bt709())  │                 │   split → 2x OpenH264   │
│            │            │                 │   (VuiConfig::bt709())  │
│            ▼            │                 │            │            │
│   H.264 + VUI metadata  │                 │            ▼            │
└─────────────────────────┘                 │   H.264 + VUI metadata  │
         │                                  └─────────────────────────┘
         │                                              │
         └───────────────────┬──────────────────────────┘
                             │
                             ▼
                    EGFX WireToSurface PDU
                             │
                             ▼
                    RDP Client (decodes with correct color)
```

---

## Key Design Decisions

### 1. Full Range for Desktop Content

**Decision:** Default to full range (0-255) for desktop capture.

**Rationale:**
- Desktop content (UI, text, icons) benefits from full tonal range
- Limited range (16-235) loses detail in shadows and highlights
- Most desktop applications output sRGB, which is full range
- RDP clients on PCs expect PC-range content

### 2. VUI Signaling via openh264-rs Fork

**Decision:** Use forked openh264-rs with VuiConfig support.

**Rationale:**
- OpenH264 internally uses BT.601 limited range
- Without VUI, decoders assume wrong color space
- Our fork adds VuiConfig presets: `bt709()`, `bt709_full()`, `bt601()`, `srgb()`
- PR #86 submitted upstream; using fork until merged

### 3. Separate Conversion and Signaling

**Decision:** ColorSpaceConfig holds both conversion matrix AND VUI parameters.

**Rationale:**
- These MUST match or colors are wrong
- Single source of truth prevents mismatches
- Easy to add new color spaces (just add a preset)

### 4. SIMD Color Conversion

**Decision:** Implement AVX2/NEON optimized BGRA→YUV conversion.

**Rationale:**
- Color conversion runs on every frame
- ~4× speedup over scalar code
- Critical for 60fps HD streams
- Same matrix used regardless of SIMD path

---

## Future Work

### 1. NVENC Color Conversion Optimization

**Status:** VUI signaling complete. Color conversion uses CPU path.

**Optimization opportunity:** Implement CUDA kernel for BGRA→NV12 with correct BT.709 matrix to offload color conversion to GPU.

```
Current:  BGRA → CPU (BT.709) → NV12 → NVENC → H.264 + VUI  ✅ Correct
Optimized: BGRA → CUDA kernel (BT.709) → NV12 → NVENC → H.264 + VUI  (faster)
```

### 2. VAAPI VUI Limitation

**Problem:** VA-API H.264 encoding interface does not expose color VUI fields.

**Investigation result:** Checked `/usr/include/va/va_enc_h264.h` - the `VAEncSequenceParameterBufferH264` structure only includes timing/aspect ratio VUI fields, not color parameters (`colour_primaries`, `transfer_characteristics`, `matrix_coefficients`).

**Workaround:** VAAPI path uses correct color conversion on CPU (BT.709 matrix). Decoders that don't receive VUI will likely assume BT.709 for HD content, which matches our conversion. This is not ideal but acceptable for most use cases.

**Note:** Interestingly, VA-API MPEG2 encoding (`va_enc_mpeg2.h`) DOES have color VUI fields. This is a legacy limitation of the H.264 VA-API interface design.

### 3. Runtime Configuration

**Planned:** Add color settings to config.toml:
```toml
[egfx]
color_space = "auto"  # or "bt709", "bt601", "srgb"
color_range = "full"  # or "limited", "auto"
```

---

## Files Reference

| File | Purpose |
|------|---------|
| `src/egfx/color_space.rs` | ColorSpaceConfig, presets, VUI parameters |
| `src/egfx/color_convert.rs` | BGRA→YUV conversion (AVX2, NEON, scalar) |
| `src/egfx/encoder.rs` | EncoderConfig with color_space field |
| `src/egfx/avc444_encoder.rs` | AVC444 encoder with VUI integration |
| `src/egfx/hardware/vaapi/mod.rs` | VAAPI encoder (color conversion ✅, VUI not available) |
| `src/egfx/hardware/nvenc/mod.rs` | NVENC encoder with VUI integration ✅ |

---

## Dependencies

```toml
# Cargo.toml - openh264-rs fork with VUI support
openh264 = { git = "https://github.com/glamberson/openh264-rs.git", branch = "feature/vui-support" }
```

**Upstream Status:** PR #86 pending review. Switch to upstream once merged.

---

## Summary

The color architecture ensures that:

1. **Colors are converted correctly** using the appropriate matrix (BT.709 for HD, BT.601 for SD)
2. **Decoders know how to interpret the data** via VUI metadata in the H.264 stream
3. **Conversion and signaling always match** through the unified ColorSpaceConfig
4. **Performance is maintained** with SIMD-optimized conversion paths

### VUI Implementation Status

| Encoder | Color Conversion | VUI Signaling | Notes |
|---------|-----------------|---------------|-------|
| **OpenH264** | ✅ BT.709 | ✅ VuiConfig | Full control via fork |
| **NVENC** | ✅ BT.709 | ✅ h264VUIParameters | Complete implementation |
| **VAAPI** | ✅ BT.709 | ⚠️ Not available | VA-API H.264 limitation |

This architecture delivers correct colors for OpenH264 and NVENC paths. VAAPI relies on decoder assumptions (typically correct for HD content).
