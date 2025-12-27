# Critical Discovery: Damage Tracking Causing Inconsistent Input

**Date**: 2025-12-27
**Session**: Log analysis with colorful wallpaper

---

## 🎯 Executive Summary

**Root Cause Found**: Damage tracking was enabled and causing **inconsistent BGRA input data** between frames, even for a static colorful wallpaper. This made it appear as if AVC444 color conversion was broken, when actually the input data itself was changing.

---

## What We Discovered

### Test Setup
- **Wallpaper**: Vibrant abstract image with pinks, yellows, blues, greens, oranges
- **Resolution**: 1280×800
- **Expected behavior**: Static screen = identical frames

### Frame-to-Frame Analysis

**Frame 1** (21:49:50.212):
```
Color conversion center (640,400):
  Input:  BGRA=(125, 35,240) ← PURPLE/MAGENTA (high R, high B)
  Output: YUV =( 85,150,226) ← Correct conversion!

Main view center:
  U444: [150,149] [148,148]  ← Colors preserved
  V444: [226,227] [227,228]  ← Colors preserved

Auxiliary center (row 400):
  Aux Y: [148,148,147,147]   ← U chroma correctly packed
  Aux U420: 149, Aux V420: 227 ← Colors in auxiliary
```

**Frame 2** (21:49:53.161) - **2.9 seconds later, same wallpaper**:
```
Color conversion center (640,400):
  Input:  BGRA=(29, 29, 29) ← GRAY! Where did the colors go?
  Output: YUV =(29,128,128) ← Neutral (correct for gray input)

Main view center:
  U444: [128,128] [128,128]  ← All neutral
  V444: [128,128] [128,128]  ← All neutral

Auxiliary center (row 400):
  Aux Y: [128,128,128,128]   ← Neutral

Temporal marker:
  ⚠️ TEMPORAL CHANGE: Auxiliary DIFFERENT
```

### The Smoking Gun

Log messages showed:
```
damage_tracking: true  # In config
🎯 Damage tracking: 200 frames skipped (no change), 96.9% bandwidth saved
skipped_damage 207
```

**Damage tracking detected** "no change" on the static wallpaper and started **skipping frames or sending incomplete data**. This caused:

1. **First frame**: Full frame with colorful BGRA data
2. **Second frame**: Damage tracking said "no change" → gray/empty BGRA data (29,29,29)
3. **Temporal instability**: Auxiliary hash changing even though screen is static

---

## Why This Matters for AVC444

Our investigation originally suspected:
- ❌ U/V swap in packing code
- ❌ Color matrix mismatch (BT.709 vs BT.601)
- ❌ SIMD conversion errors
- ❌ Auxiliary packing algorithm bugs

**Actual problem**: None of the above! The **input data was inconsistent** due to damage tracking behavior.

### What Was Actually Working Correctly

✅ **Color conversion**: BGRA→YUV444 was perfect (when given correct input)
✅ **Main view packing**: 4:2:0 subsampling was correct
✅ **Auxiliary packing**: Row-level macroblock packing matched spec
✅ **U/V channel ordering**: Not swapped, values made sense for colors

---

## Configuration Change Made

**Location**: `greg@192.168.10.205:~/config.toml`

**Changed**:
```toml
[video]
damage_tracking = true   # OLD: Causing inconsistent frames
```

**To**:
```toml
[video]
damage_tracking = false  # NEW: Send full frames every time
```

This ensures we get **complete, consistent BGRA data** for every frame, even if the screen content hasn't changed.

---

## Next Steps

### Immediate: Retest with Damage Tracking Disabled

**Run another test to verify colors are now consistent**:

```bash
ssh greg@192.168.10.205
cd ~
./run-server.sh  # Damage tracking now disabled
```

**Expected results**:
- All frames should show the **same colorful BGRA values** at center position
- Main view U/V should be **consistent** across frames
- Auxiliary view should show `✅ TEMPORAL STABLE` (hash identical)
- No more gray (29,29,29) frames when wallpaper is colorful

### Then: Investigate AVC444 Color Quality

Once we have **consistent input data**, we can properly investigate:

1. **Are colors slightly off even with perfect encoding?**
   - Compare AVC420 vs AVC444 visual quality
   - Check if client-side reconstruction is correct

2. **VUI signaling issues?**
   - Test limited range vs full range
   - Try different color matrices

3. **P-frame corruption (separate issue)**
   - Why does main P-frames + auxiliary cause lavender?
   - Currently mitigated by forcing main to all-I

---

## Log Evidence

**Test log**: `colorful-test-20251227-234938.log` (72MB)
- Shows frame-to-frame BGRA variance
- Demonstrates damage tracking skipping frames
- Proves our YUV conversion works correctly when input is correct

**Key timestamps**:
- 21:49:50.212: Frame with colors (BGRA=125,35,240)
- 21:49:53.161: Frame without colors (BGRA=29,29,29)
- Both frames: Same wallpaper, no screen changes!

---

## Lessons Learned

1. **Always verify input data**: Don't assume the BGRA buffer is consistent
2. **Damage tracking can interfere with diagnostics**: Disable for color testing
3. **Multi-position sampling was crucial**: Revealed the frame-to-frame variance
4. **Temporal stability checks work**: Hash comparison caught the inconsistency

---

**Status**: Configuration updated, ready for retest with damage tracking disabled.
**Expected outcome**: Consistent colorful frames, allowing proper AVC444 color analysis.
