# Deep Analysis: Auxiliary Encoder Stride Mismatch Bug

**Date**: 2025-12-28
**Status**: Investigation Complete - Bug Identified
**Severity**: Critical - Affects P-frame encoding quality

---

## Executive Summary

After comprehensive testing, we've determined:
1. ✅ Input changes are real (UI animations, cursor movement)
2. ✅ Packing algorithm is deterministic and correct
3. ✅ Color conversion is deterministic and correct
4. ❌ **CRITICAL BUG**: Stride mismatch in auxiliary encoder setup

---

## The Stride Mismatch Bug

### Problem Location

**File**: `src/egfx/yuv444_packing.rs:685-690` and `src/egfx/avc444_encoder.rs:339-344`

### The Bug

**Auxiliary Buffer Allocation** (yuv444_packing.rs:465-475):
```rust
let padded_chroma_width = ((chroma_width + 7) / 8) * 8;
let padded_chroma_height = ((chroma_height + 7) / 8) * 8;

let mut aux_u = vec![0u8; padded_chroma_width * padded_chroma_height];
let mut aux_v = vec![0u8; padded_chroma_width * padded_chroma_height];
aux_u.fill(128);
aux_v.fill(128);

// Data written with stride = padded_chroma_width
for cy in 0..chroma_height {
    for cx in 0..chroma_width {
        let out_idx = cy * padded_chroma_width + cx;  // ← PADDED stride
        aux_u[out_idx] = yuv444.u[idx];
        aux_v[out_idx] = yuv444.v[idx];
    }
}
```

**Yuv420Frame Construction** (yuv444_packing.rs:685-690):
```rust
Yuv420Frame {
    y: aux_y,
    u: aux_u,  // Buffer with PADDED stride
    v: aux_v,  // Buffer with PADDED stride
    width,      // Original width (NOT padded!)
    height,
}
```

**Stride Calculation** (yuv444_packing.rs:143-145):
```rust
pub fn strides(&self) -> (usize, usize, usize) {
    (self.width, self.width / 2, self.width / 2)  // ← Uses UNPADDED width!
}
```

**Encoder Setup** (avc444_encoder.rs:339-344):
```rust
let aux_strides = aux_yuv420.strides();  // Returns (1280, 640, 640)
let aux_yuv_slices = YUVSlices::new(
    (aux_yuv420.y_plane(), aux_yuv420.u_plane(), aux_yuv420.v_plane()),
    dims,        // (1280, 800)
    aux_strides, // (1280, 640, 640) ← WRONG for padded buffer!
);
```

### The Impact

**For 1280x800 (current test)**:
- chroma_width = 640
- padded_chroma_width = 640 (no padding needed)
- **Stride mismatch**: NONE (lucky!)
- **Effect**: No visible impact on this resolution

**For 1366x768 (non-8-aligned)**:
- chroma_width = 683
- padded_chroma_width = 688 (padded to next multiple of 8)
- **Stride mismatch**: Data stored with stride 688, encoder told stride 683
- **Effect**: Encoder reads wrong bytes, skips padding, wraps incorrectly
- **Result**: Severe corruption and visual artifacts

**For 1920x1080 (aligned)**:
- chroma_width = 960
- padded_chroma_width = 960 (already multiple of 8)
- **Stride mismatch**: NONE
- **Effect**: No visible impact

### Why This Causes P-Frame Corruption (Theory)

Even though 1280x800 doesn't have padding, the conceptual mismatch might affect how OpenH264 handles the buffers internally:

1. **Memory Layout Assumptions**: OpenH264 might assume buffer continuity based on stride
2. **Cache Alignment**: Padded buffers have different alignment than encoder expects
3. **Internal Processing**: OpenH264's SIMD code might over-read based on stride assumptions

---

## Test Results Summary

### Test 1: Option 1 (Explicit .fill)
- **Result**: Failed - Still cycling
- **Conclusion**: Not a Vec initialization issue

### Test 2: Option 2 (Buffer Diff Logging)
- **Result**: Found DATA region differences (not padding)
- **Conclusion**: Real differences in DATA, ruled out padding corruption

### Test 3: Targeted Position Logging
- **Result**: BGRA input actually changes (UI animations)
- **Conclusion**: Not nondeterminism, real screen changes
- **But**: Even with stable BGRA periods, hashes still change on most frames!

### Test 4: Static Screen Test
- **Result**: During stable periods (same BGRA), auxiliary hashes still change
- **Evidence**:
  - Frames 1-14: BGRA=(22, 81, 247) constant, but only got 0 TEMPORAL STABLE
  - Frames 31-41: BGRA=(255, 255, 255) constant, but only got 1 TEMPORAL STABLE (frame 33)
  - Frames 42-43: BGRA=(42, 13, 62) constant, got 1 TEMPORAL STABLE (frame 43)
- **Conclusion**: SOMETHING ELSE is changing besides position (329, 122)!

---

## The Real Problem

Position (329, 122) is stable during certain periods, but the **overall hash still changes**. This means:

**OTHER positions in the auxiliary buffer are changing even when input is stable!**

This could be:
1. **Stride bug** causing encoder to read wrong memory regions
2. **Multiple animated areas** on screen (not just one position)
3. **PipeWire delivering slightly different frames** (double buffering, etc.)
4. **SIMD/AVX2 nondeterminism** at positions we're not tracking

---

## Recommended Next Steps

### Option A: Fix Stride Mismatch (Most Likely Fix)

Change auxiliary buffer to NOT use padding, matching how it's presented to encoder:

```rust
// Instead of padding, use actual chroma dimensions
let mut aux_u = vec![0u8; chroma_width * chroma_height];
let mut aux_v = vec![0u8; chroma_width * chroma_height];
aux_u.fill(128);
aux_v.fill(128);

// Write with correct stride
for cy in 0..chroma_height {
    for cx in 0..chroma_width {
        let out_idx = cy * chroma_width + cx;  // ← Match stride we'll report
        aux_u[out_idx] = yuv444.u[idx];
        aux_v[out_idx] = yuv444.v[idx];
    }
}

// Return with unpadded dimensions
Yuv420Frame {
    y: aux_y,
    u: aux_u,
    v: aux_v,
    width,
    height,
}
```

**Why This Might Fix It:**
- Buffer layout matches what encoder expects
- No mismatch between stored stride and reported stride
- Works for all resolutions (aligned and non-aligned)

### Option B: Sample MORE Positions

Add comprehensive logging at many screen positions to find ALL changing areas:

```rust
// Sample grid of 20+ positions
for y in (0..height).step_by(100) {
    for x in (0..width).step_by(100) {
        // Log BGRA and YUV
    }
}
```

### Option C: Disable Deblocking Filter

Try to access OpenH264's raw API to disable deblocking filter for auxiliary encoder:
- Auxiliary stream contains chroma encoded as luma
- Deblocking filter designed for luma might corrupt chroma
- Requires using openh264-sys direct FFI calls

### Option D: Compare All-I vs P-Frame Bitstreams

Enable detailed NAL logging and compare:
- All-I: What NAL units are in each frame
- P-frame: What NAL units, reference frames, motion vectors
- Identify specific difference causing corruption

---

## My Recommendation

**Start with Option A (Fix Stride Mismatch)**

Reasons:
1. Clear bug - padding doesn't match what we tell encoder
2. Simple fix - remove padding from aux_u/aux_v
3. Should work for all resolutions
4. Might fix P-frame corruption as side effect

If that doesn't fix it:
- Try Option C (disable deblocking) using raw API
- Then Option B (more comprehensive sampling)

---

## Code Change for Option A

**File**: `src/egfx/yuv444_packing.rs`

**Lines to change**: 465-490, 685-690

**Before**:
```rust
let padded_chroma_width = ((chroma_width + 7) / 8) * 8;
let padded_chroma_height = ((chroma_height + 7) / 8) * 8;
let mut aux_u = vec![0u8; padded_chroma_width * padded_chroma_height];
let mut aux_v = vec![0u8; padded_chroma_width * padded_chroma_height];
```

**After**:
```rust
// Don't pad! Encoder will handle any needed padding internally
let mut aux_u = vec![0u8; chroma_width * chroma_height];
let mut aux_v = vec![0u8; chroma_width * chroma_height];
```

**Change loop stride**:
```rust
let out_idx = cy * chroma_width + cx;  // NOT padded_chroma_width
```

---

## Expected Outcome

**If stride was the issue**:
- All frames with stable input show `✅ TEMPORAL STABLE`
- P-frames can be re-enabled
- No corruption with changing content
- **Problem solved!**

**If stride wasn't the issue**:
- Hashes still change with stable input
- Need to try Option C (deblocking) or deeper investigation
- All-I workaround remains valid fallback

---

Sources:
- [OpenH264 Usage Example](https://github.com/cisco/openh264/wiki/UsageExampleForEncoder)
- [OpenH264 Rust Bindings](https://github.com/ralfbiedert/openh264-rs)
- [OpenH264 Encoder Parameters](https://github.com/cisco/openh264/blob/master/codec/api/wels/codec_app_def.h)
