# AVC444 P-Frame Corruption Solution: Hybrid Frame Strategy

**Date**: 2025-12-28
**Status**: Proposed Solution
**Confidence**: High (addresses root cause without requiring FFI)

---

## Analysis Summary

### What We've Confirmed

✅ **Streams Are Synchronized**:
- Frame #0: Main IDR + Aux IDR
- Frames #1-235: Main P + Aux P
- Perfect synchronization, desynchronization ruled out

✅ **Frame Sizes Are Reasonable**:
- Main IDR: ~74KB
- Aux IDR: ~70KB
- Main P avg: ~22KB
- Aux P avg: ~23KB

✅ **P-Frames Compress Well**:
- 3:1 compression ratio vs IDR
- P-frame encoding is working

❌ **P-Frames Cause Lavender Corruption**:
- User confirmed visual artifacts
- Only in changed areas (scrolling text, moving windows)
- Static areas remain correct

### Root Cause

**Deblocking filter corrupts chroma-as-luma in auxiliary P-frames**.

H.264's in-loop deblocking filter:
- Designed for luma statistics (edges, text, high-frequency content)
- Applied to auxiliary Y plane (which contains chroma data)
- Chroma has different statistics → wrong filtering → color corruption

---

## Proposed Solution: Hybrid Frame Strategy

### Concept

**Main Stream**: Use P-frames (efficient, works well)
**Auxiliary Stream**: Use all-I frames (avoid deblocking corruption)

### Why This Works

1. **Main stream** (luma + subsampled chroma):
   - Contains actual luma → deblocking filter works correctly
   - P-frames compress well
   - No corruption

2. **Auxiliary stream** (chroma-as-fake-luma):
   - All-I frames → no P-frame prediction
   - No reference frame → minimal deblocking
   - No corruption (we've proven this works)

### Bandwidth Impact

**Current (All-I for both)**:
- Main IDR: ~74KB
- Aux IDR: ~70KB
- Total per frame: ~144KB
- At 30 FPS: ~4.3 MB/s

**Proposed (P for main, I for aux)**:
- Main P: ~22KB (after first IDR)
- Aux IDR: ~70KB
- Total per frame: ~92KB
- At 30 FPS: ~2.8 MB/s

**Savings vs current all-I**: 36% bandwidth reduction
**Cost vs ideal P+P**: 2x auxiliary bandwidth

### Comparison Table

| Strategy | Main | Aux | Total/Frame | 30 FPS | Quality |
|----------|------|-----|-------------|--------|---------|
| All-I (current) | 74KB | 70KB | 144KB | 4.3 MB/s | Perfect ✅ |
| P+P (broken) | 22KB | 23KB | 45KB | 1.4 MB/s | Corrupted ❌ |
| **P+I (proposed)** | **22KB** | **70KB** | **92KB** | **2.8 MB/s** | **Perfect?** ✅ |

### Benefits

✅ Main stream efficiency (P-frames work well for luma)
✅ Auxiliary corruption avoided (no P-frames on chroma-as-luma)
✅ 36% bandwidth savings vs current all-I
✅ No FFI required (uses existing force_intra_frame API)
✅ Simple to implement (3 lines of code)

### Tradeoffs

⚠️ Auxiliary bandwidth 3x higher than ideal P-frames
⚠️ Not as efficient as full P+P (if we could make it work)

---

## Implementation

### Code Change

**File**: `src/egfx/avc444_encoder.rs:322-328`

**Replace**:
```rust
// === TEST: P-frames re-enabled after stride fix ===
// self.main_encoder.force_intra_frame();
// self.aux_encoder.force_intra_frame();
```

**With**:
```rust
// === SOLUTION: Hybrid frame strategy ===
// Main: P-frames work well (luma + subsampled chroma)
// Aux: All-I frames (avoid P-frame deblocking corruption on chroma-as-luma)
// Trade: Higher aux bandwidth, but no corruption
self.aux_encoder.force_intra_frame();  // Force aux to all-I
// Main encoder uses P-frames naturally (no force_intra_frame)
```

### Expected Outcome

**If deblocking was the issue**:
- ✅ No lavender corruption
- ✅ Text readable during scrolling
- ✅ Windows move smoothly
- ✅ 36% bandwidth savings vs current all-I
- **Problem solved with acceptable tradeoff!**

**If deblocking wasn't the issue**:
- Still see corruption
- Need deeper investigation (unlikely at this point)

---

## Alternative Solutions (If Hybrid Doesn't Work)

### Plan B: Selective I-Frames for Auxiliary

Force auxiliary I-frame every N frames:
```rust
if self.frame_count % 10 == 0 {
    self.aux_encoder.force_intra_frame();
}
```

Reduces corruption frequency, lowers bandwidth vs all-I.

### Plan C: Access Raw OpenH264 API

Use openh264-sys2 FFI to disable deblocking:
```rust
unsafe {
    use openh264_sys2::*;
    let encoder_ptr = /* get raw encoder pointer */;

    let mut params = SEncParamExt::default();
    encoder_ptr.GetDefaultParams(&mut params);
    params.iLoopFilterDisableIdc = 1;  // Disable deblocking
    encoder_ptr.InitializeExt(&params);
}
```

Complex, requires unsafe FFI, but would allow P-frames in auxiliary.

### Plan D: Keep Current All-I Workaround

- Perfect quality (user confirmed)
- 4.3 MB/s bandwidth
- Simple, no risks
- If bandwidth is acceptable, this is safest

---

## Recommendation

**Try hybrid strategy first** (P for main, I for auxiliary):

**Pros**:
- Simple to implement (3 lines)
- 36% bandwidth savings
- High confidence it will work
- No unsafe code

**Cons**:
- Not as efficient as full P+P would be
- If it doesn't work, we've wasted time

**If bandwidth is critical**: Skip to Plan C (raw API deblocking disable)
**If quality is critical**: Stick with Plan D (current all-I)

---

## Next Steps

1. Implement hybrid strategy
2. Build and deploy
3. Test for corruption
4. If successful: Document and close
5. If unsuccessful: Investigate Plan C (raw API)

---

## Research Sources

- [MS-RDPEGFX YUV420p Combination](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/8131c1bc-1af8-4907-a05a-f72f4581160f)
- [FreeRDP AVC444 Issue #11040](https://github.com/FreeRDP/FreeRDP/issues/11040)
- [OpenH264 Deblocking Parameters](https://github.com/cisco/openh264/blob/master/codec/api/wels/codec_app_def.h)
- [Microsoft Research: Tunneling Color Content](https://www.microsoft.com/en-us/research/publication/tunneling-high-resolution-color-content-through-420-hevc-and-avc-video-coding-systems-2/)
