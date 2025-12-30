# AVC444 P-Frame Corruption - Deep Technical Analysis

**Date**: 2025-12-28
**Binary MD5**: `ebeb203fe150ffd2575427c6a6b9cfd9`
**Status**: Investigating P-frame corruption with frame type logging

---

## Executive Summary

After extensive investigation and multiple test cycles:
- ✅ All-I frames work PERFECTLY (user confirmed: good colors, readable scrolling text, responsive)
- ✅ Packing algorithm is correct (matches FreeRDP implementation)
- ✅ Color conversion is correct
- ✅ Stride mismatch fixed (aux_u/aux_v no longer padded)
- ❌ P-frames still show lavender corruption in changed areas

**Root Cause Hypothesis**: H.264 P-frame encoding treats auxiliary stream Y plane (which contains chroma data) as if it were luma, applying luma-specific optimizations that corrupt chroma values.

---

## The AVC444 Architecture Problem

### What AVC444 Does

Encodes YUV444 (4:4:4 full chroma) using two H.264 YUV420 (4:2:0 subsampled) streams:

**Main Stream (Standard YUV420)**:
- Y plane: Full luma
- U plane: Subsampled chroma U (2×2 box filter)
- V plane: Subsampled chroma V (2×2 box filter)

**Auxiliary Stream (Chroma-as-Fake-Luma)**:
- Y plane: Missing U444 (odd rows 1,3,5...) and V444 (odd rows 1,3,5...) packed
- U plane: U444 at odd columns, even rows
- V plane: V444 at odd columns, even rows

### The Core Problem

The auxiliary stream's **Y plane contains CHROMA values** but is **encoded AS IF it were LUMA**.

H.264 has luma-specific optimizations:
1. **Deblocking Filter**: Smooths block edges, tuned for luma statistics
2. **Quantization**: Different matrices/parameters for luma vs chroma
3. **Motion Compensation**: Search algorithms tuned for luma statistics

When these are applied to chroma data, they can produce artifacts.

---

## Evidence from Testing

### All-I Frames (Working Perfectly)

**User Report**:
- ✅ Good colors
- ✅ Readable fast scrolling text
- ✅ Window movement smooth and correct
- ✅ Right-click menus display properly
- ✅ Relatively responsive

**Why All-I Works**:
Each frame is independently encoded without reference to previous frames. No inter-frame prediction, no motion compensation, minimal deblocking impact.

### P-Frames (Lavender Corruption)

**Symptoms**:
- Lavender/brown macroblocks in changed areas
- Corruption appears where content changed (scrolling text, moving windows)
- Static areas remain correct

**Why P-Frames Fail**:
P-frames encode differences from previous frames using:
- Motion vectors (block matching)
- Residuals (difference encoding)
- Deblocking filter on reconstructed frame

When applied to chroma-as-luma in auxiliary stream → corruption.

---

## FreeRDP Implementation Comparison

Analyzed FreeRDP's `general_YUV444SplitToYUV420()` function:

### Auxiliary Stream Packing (B6/B7)

**FreeRDP**:
```c
for (size_t y = 0; y < halfHeight; y++) {
    const BYTE* pSrcU = pSrc[1] + 2 * y * srcStep[1];  // Even rows
    const BYTE* pSrcV = pSrc[2] + 2 * y * srcStep[2];
    BYTE* pU = pAuxDst[1] + y * dstAuxStep[1];
    BYTE* pV = pAuxDst[2] + y * dstAuxStep[2];

    for (size_t x = 0; x < halfWidth; x++) {
        pU[x] = pSrcU[2 * x + 1];  // Odd columns
        pV[x] = pSrcV[2 * x + 1];
    }
}
```

**Our Implementation**:
```rust
for cy in 0..chroma_height {
    let y = cy * 2;  // Even row
    for cx in 0..chroma_width {
        let x = cx * 2 + 1;  // Odd column
        let idx = y * width + x;
        let out_idx = cy * chroma_width + cx;

        aux_u[out_idx] = yuv444.u[idx];  // Odd column, even row
        aux_v[out_idx] = yuv444.v[idx];
    }
}
```

**Conclusion**: Our implementation matches FreeRDP's approach exactly. Packing is correct.

---

## H.264 P-Frame Processing Pipeline

### Encoder Side

```
Input Frame (YUV420)
    ↓
Motion Estimation (find matching blocks in reference frame)
    ↓
Motion Compensation (predict from reference + motion vectors)
    ↓
Transform & Quantization (DCT + quantize residuals)
    ↓
Entropy Coding (CAVLC/CABAC)
    ↓
Inverse Quantization & Transform
    ↓
Add to Prediction
    ↓
Deblocking Filter ← SUSPECT #1: Corrupts chroma-as-luma
    ↓
Reconstructed Frame (used as reference for next P-frame)
    ↓
Bitstream Output
```

### Decoder Side (Client)

```
Bitstream Input
    ↓
Entropy Decoding
    ↓
Inverse Quantization & Transform
    ↓
Motion Compensation (apply motion vectors to reference)
    ↓
Add Residuals
    ↓
Deblocking Filter ← SUSPECT #2: Corrupts again on client side
    ↓
Reconstructed Frame
    ↓
YUV420 → YUV444 Combination (FreeRDP's prim_YUV.c)
    ↓
Display
```

---

## Root Cause Hypotheses (Ranked by Likelihood)

### #1: Deblocking Filter Corruption (90% confidence)

**The Problem**:
H.264's in-loop deblocking filter smooths block boundaries to reduce blocking artifacts. It's designed for luma (brightness) which has different statistical properties than chroma (color).

**Why It Corrupts Chroma**:
- Luma: High-frequency detail (edges, text)
- Chroma: Smoother, less detail
- Filter thresholds tuned for luma statistics
- Chroma values filtered as luma → wrong smoothing → color corruption

**Evidence**:
- Corruption appears as "lavender/brown" - color shifts typical of chroma errors
- Appears in P-frames only (deblocking more aggressive for P-frames)
- All-I frames work (minimal deblocking)

**Solution**:
Disable deblocking filter for auxiliary encoder. Requires accessing OpenH264's raw API:
```c
// Via FFI to openh264-sys
SEncParamExt params;
encoder->GetDefaultParams(&params);
params.iLoopFilterDisableIdc = 1;  // Disable deblocking
encoder->InitializeExt(&params);
```

### #2: Quantization Matrix Mismatch (60% confidence)

**The Problem**:
H.264 uses different quantization for luma vs chroma. Auxiliary stream uses luma quantization on chroma data.

**Why It Corrupts**:
- Luma QP (quantization parameter) might be too aggressive for chroma
- Chroma more sensitive to quantization errors (color shifts visible)
- Wrong QP → excessive quantization → color corruption

**Solution**:
- Match auxiliary encoder QP to main encoder's chroma QP (not luma QP)
- Or use lower QP for auxiliary (higher quality)

### #3: Motion Compensation Artifacts (40% confidence)

**The Problem**:
Motion search algorithms tuned for luma patterns, not chroma. Poor motion vectors for chroma data.

**Why It Corrupts**:
- Bad motion vectors → large residuals
- Large residuals + quantization → corruption
- Chroma patterns don't match well between frames (different statistics)

**Solution**:
- Adjust motion search parameters for auxiliary encoder
- Use smaller search range
- Or force more frequent I-frames in auxiliary

### #4: Dual-Stream Desynchronization (30% confidence)

**The Problem**:
Main and auxiliary streams use different reference frames or timing.

**Why It Corrupts**:
- Decoder combines frames from mismatched references
- Temporal mismatch → color shifts

**Evidence Against**:
- Both encoders get same input simultaneously
- Both use same encode() call
- Should be synchronized

**Solution**:
- Log and verify frame numbers match
- Ensure SPS/PPS prepending is correct

---

## Test Plan for New Binary

**Binary MD5**: `ebeb203fe150ffd2575427c6a6b9cfd9`

**New Logging**: Frame types and sizes for both streams

### What to Look For:

1. **Frame Type Patterns**:
   ```
   [AVC444 Frame #0] Main: IDR (45000B), Aux: IDR (38000B)
   [AVC444 Frame #1] Main: P (12000B), Aux: P (8000B)
   [AVC444 Frame #2] Main: P (11000B), Aux: P (7500B)
   ```

2. **Size Patterns**:
   - IDR: 40-50KB per stream
   - P: 5-15KB per stream (should be much smaller)

3. **Synchronization**:
   - Do both streams produce same frame type?
   - Are sizes reasonable?

### Expected Findings:

**If both streams synchronized**:
- Main: IDR, Aux: IDR → Good
- Main: P, Aux: P → Good
- Corruption still present → Deblocking/quantization issue

**If streams desynchronized**:
- Main: IDR, Aux: P → Bad!
- Main: P, Aux: IDR → Bad!
- This would explain corruption

---

## Next Steps Based on Results

### Scenario A: Streams Synchronized, Still Corrupted

**Action**: Access OpenH264 raw API to disable deblocking for auxiliary encoder

**Implementation**:
1. Add openh264-sys dependency for FFI access
2. After creating aux_encoder, configure via raw API:
   ```rust
   unsafe {
       let raw_encoder = aux_encoder.raw_api();
       // Set iLoopFilterDisableIdc = 1
   }
   ```

### Scenario B: Streams Desynchronized

**Action**: Force frame type matching between encoders

**Implementation**:
```rust
// After encoding main stream
if main_is_keyframe {
    self.aux_encoder.force_intra_frame();
}
// This ensures aux uses same frame type as main
```

### Scenario C: P-Frame Sizes Abnormal

**Action**: Investigate quantization parameters

**Implementation**:
- Log QP values
- Adjust bitrate ratio between main/aux
- Test with different QP settings

---

## References and Research

**Microsoft Spec**:
- [MS-RDPEGFX YUV420p Stream Combination for YUV444 mode](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/8131c1bc-1af8-4907-a05a-f72f4581160f)

**FreeRDP Implementation**:
- [FreeRDP AVC444 Issue #11040](https://github.com/FreeRDP/FreeRDP/issues/11040)
- [FreeRDP prim_YUV.c](https://github.com/FreeRDP/FreeRDP/blob/master/libfreerdp/primitives/prim_YUV.c)

**Microsoft Research**:
- [Tunneling High-Resolution Color Content (Wu et al., 2013)](https://www.microsoft.com/en-us/research/publication/tunneling-high-resolution-color-content-through-420-hevc-and-avc-video-coding-systems-2/)

**OpenH264 Documentation**:
- [OpenH264 Usage Example](https://github.com/cisco/openh264/wiki/UsageExampleForEncoder)
- [OpenH264 Encoder Parameters](https://github.com/cisco/openh264/blob/master/codec/api/wels/codec_app_def.h)

---

## Current Status

**Testing**: P-frames with frame type logging
**Awaiting**: Test results to determine if streams are synchronized
**Next**: Based on synchronization, either fix deblocking or frame type matching

**Fallback**: All-I workaround provides perfect quality (user confirmed)
