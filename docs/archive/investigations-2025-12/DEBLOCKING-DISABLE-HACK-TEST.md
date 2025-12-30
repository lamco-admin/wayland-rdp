# Deblocking Disable Hack Test - THE DEFINITIVE TEST

**Date**: 2025-12-29 01:30 UTC
**Binary MD5**: `d78ff865d371e32bb871a185d9b1ff29`
**Status**: CRITICAL TEST - Will definitively answer if deblocking is the cause

---

## What This Test Does

### The Hack

**Modified**: `/home/greg/openh264-rs/openh264/src/encoder.rs:1044`

```rust
// BEFORE InitializeExt():
params.iLoopFilterDisableIdc = 1;  // ← HARDCODED for ALL encoders
```

**Effect**: Deblocking filter is DISABLED for:
- ✅ Main encoder (luma + subsampled chroma)
- ✅ Auxiliary encoder (chroma-as-luma)

**Note**: This affects BOTH streams, not just auxiliary. Not ideal for production but perfect for diagnosis.

---

## What We'll Learn

### Scenario A: NO Corruption

**Observation**: Text readable, no lavender/brown artifacts, smooth

**Conclusion**: Deblocking filter IS the root cause

**Next Steps**:
1. Extend openh264-rs API properly (only disable for auxiliary)
2. Test main deblocking ON, aux deblocking OFF
3. Production solution found ✅

---

### Scenario B: STILL Have Corruption

**Observation**: Lavender/brown artifacts still present

**Conclusion**: Deblocking is NOT the root cause (or not the ONLY cause)

**Next Steps (from other session's advice)**:

**Test deterministic padding**:
```rust
// Clear ALL padding every frame
for row in height..padded_height {
    aux_y[row * width..(row + 1) * width].fill(128);
}
```

**Check reference frame handling**:
- Are auxiliary frames marked as non-reference? (nal_ref_idc)
- POC/frame_num management
- DPB (decoded picture buffer) state

**Investigate other H.264 processing**:
- Transform/quantization artifacts
- Motion compensation issues
- Chroma-specific encoding problems

---

### Scenario C: Different Artifacts

**Observation**: No lavender, but see blocking artifacts or different pattern

**Conclusion**: Deblocking was SPREADING the issue, but something else is causing it

**Next Steps**: Hybrid approach (fix root cause + tune deblocking)

---

## Technical Context (from Other Session)

### Why Deblocking Matters in AVC444

**In-Loop Filter**: Deblocking runs on reconstructed frames BEFORE they become reference frames for P-frames.

**For Luma**: Designed to smooth block boundaries (works well)

**For Chroma-as-Luma**:
- Chroma has different statistics than luma
- Filter thresholds (α, β) derived from QP might be wrong
- Could smooth incorrectly → color shifts

### How Deblocking Can Amplify Other Bugs

1. **Edge Bleed from Padding**:
   - If padding bytes are unstable
   - Deblocking touches boundary pixels
   - Bad values bleed inward → visible corruption

2. **Reference Contamination**:
   - Filtered frame becomes reference
   - If frame N slightly wrong
   - Frame N+1 predicts from wrong reference
   - Error propagates and compounds

3. **4:4:4 Makes It More Visible**:
   - Full-resolution chroma
   - Small errors → strong color blocks
   - In 4:2:0, chroma lower res → artifacts blur away

---

## Our Specific Situation

**Resolution**: 1280x800
- Height 800 = 50 × 16 (perfect alignment!)
- No padding rows needed (padded_height == height)
- Width 1280 = 80 × 16 (perfect alignment!)

**aux_u/aux_v**: 640x400
- Width 640 = 80 × 8 (perfect alignment after our stride fix)
- Height 400 = 50 × 8 (perfect alignment)

**Stride**: Fixed (no padding, uses actual dimensions)

**So padding might NOT be the issue** for 1280x800.

---

## Test Instructions

**Run**: Same rigorous test as before
- Scroll terminal text (fast)
- Move windows
- Right-click menus
- General interaction

**Look For**:

**Primary**: **Lavender/brown corruption**
- YES → Deblocking not the cause (or not only cause)
- NO → Deblocking IS the cause ✅

**Secondary**: **Blocking artifacts**
- Might see slight "grid" pattern from disabled deblocking
- Should be subtle (8x8 or 16x16 blocks)
- Much less visible than lavender corruption

**Tertiary**: **Main stream quality**
- Main encoder also has deblocking disabled (not ideal)
- Might see slight blocking in luma (text, edges)
- Acceptable for this diagnostic test

---

## After Test

Tell me **one of these**:

1. **"No corruption at all"** → Deblocking confirmed as cause
2. **"Still have lavender"** → Deblocking not the cause
3. **"Different artifacts"** → Deblocking was spreading something else

Then I'll know exactly what to investigate next!

---

## Why This Test is Valid

✅ Deblocking actually disabled (hardcoded at init)
✅ AVC444 dual-stream encoding active
✅ P-frames enabled
✅ All other variables same

This is the DEFINITIVE test the other session recommended.
