# AVC444 P-Frame Corruption Investigation
## 2025-12-27 Evening Session

**Status**: Root cause identified but not fully resolved
**Symptom**: Lavender/purple corruption in P-frames only, keyframes perfect
**Progress**: Eliminated encoding-side causes, pointing to encoder config or client reconstruction

---

## 🎯 The Breakthrough Discovery

**All-keyframes mode works perfectly!**
- No corruption when `force_all_keyframes: true`
- Perfect color quality in keyframe-only mode
- Proves: Auxiliary packing algorithm is **CORRECT**
- Proves: Main view encoding is **CORRECT**
- Proves: Color conversion is **CORRECT**

**Conclusion**: The issue is **P-frame temporal prediction**, not spatial packing.

---

## 🔬 Systematic Elimination (What We Tested)

### ✅ Test 1: U/V Channel Swap
- **Hypothesis**: U and V channels backwards throughout pipeline
- **Result**: Made it WORSE (keyframes also corrupted)
- **Conclusion**: Channels are correct

### ✅ Test 2: Minimal Auxiliary (All 128s)
- **Hypothesis**: Auxiliary packing is wrong
- **Result**: Total color corruption everywhere
- **Conclusion**: Auxiliary IS being used and IS critical

### ✅ Test 3: Simplified Packing Algorithm
- **Hypothesis**: Row-level macroblock structure is wrong
- **Result**: Same lavender corruption
- **Conclusion**: Packing structure not the issue

### ✅ Test 4: All-Keyframes Mode
- **Hypothesis**: P-frame temporal prediction is the problem
- **Result**: **PERFECT! No corruption!**
- **Conclusion**: Auxiliary packing is correct, P-frames are the issue

### ✅ Test 5: Deterministic Padding
- **Hypothesis**: Non-deterministic padding bytes cause phantom changes
- **Result**: IMPROVED (corruption more localized) but not fixed
- **Conclusion**: Padding was part of the problem

### ✅ Test 6: Disable SIMD (Scalar Only)
- **Hypothesis**: AVX2 vs scalar rounding differences
- **Result**: Still corrupted
- **Conclusion**: SIMD is not the root cause

---

## 💡 Current Understanding

### What Works
1. **Keyframe encoding**: Both main and auxiliary encode correctly
2. **Spatial packing**: Row-level macroblock structure is correct
3. **Color conversion**: BGRA → YUV444 is correct (BT.709)
4. **Main view chroma**: 2×2 box filter subsampling works

### What Breaks
1. **P-frame encoding**: Auxiliary stream P-frames cause lavender artifacts
2. **Temporal consistency**: Something changes frame-to-frame even for static content

---

## 🔍 The Real Problem: Identified But Not Fixed

Based on guidance from another debugging session, the issue is likely:

### Most Probable Cause: DPB Contamination

**Theory**: Auxiliary pictures are contaminating the reference picture buffer

**Evidence**:
- All-I frames work (no references)
- P-frames break (use references)
- Corruption appears in changed areas only

**What to check**:
1. Are auxiliary NAL units marked as **non-reference** (`nal_ref_idc = 0`)?
2. Are auxiliary pictures being stored in the DPB?
3. Is `frame_num` / POC advancing correctly for both streams?
4. Are reference lists (L0) constructed correctly?

### Secondary Cause: Stride/Padding Still Unstable

**Partial fix applied**:
- Added 8×8 macroblock padding to chroma planes
- Changed `.to_vec()` to `.truncate()` for Y plane
- Pre-initialized padding with deterministic values (128)

**Why it only partially worked**:
- May need padding on Y plane too (currently 16-aligned so no padding added)
- Stride calculations might be wrong when passing to OpenH264
- Buffer reuse between frames might introduce non-determinism

---

## 📋 Next Steps (Priority Order)

### 1. **Verify Encoder Settings** (Highest Priority)
Check that auxiliary encoder is configured correctly:

```rust
// In avc444_encoder.rs, check if we need:
encoder_config = encoder_config
    .nal_ref_idc(0)  // Mark auxiliary as non-reference?
    .max_ref_frames(1)  // Limit reference frames?
```

### 2. **Dump NAL Unit Headers**
Add logging to check:
- NAL unit types (IDR, I, P)
- `nal_ref_idc` values
- `frame_num` sequences
- POC (picture order count) values

### 3. **Compare FreeRDP Source Code**
Look at how FreeRDP's AVC444 encoder configures OpenH264:
- Repository: https://github.com/FreeRDP/FreeRDP
- File: `libfreerdp/codec/h264_ffmpeg.c` or similar
- Search for: AVC444, auxiliary, dual stream

### 4. **Test Separate Encoder Instances**
Verify our main and auxiliary encoders are truly independent:
- Check if they share any state
- Ensure SPS/PPS are separate
- Verify frame_num doesn't clash

### 5. **Force Deterministic Memory**
Even more aggressive padding:
- Zero-initialize ALL buffers before filling
- Use `vec![0u8; size]` instead of `Vec::with_capacity()`
- Clear buffers between frames

### 6. **Client-Side Investigation**
If all else fails, the issue might be client-side:
- Windows RDP client reconstruction logic
- FreeRDP client reconstruction logic
- How client combines main + auxiliary streams

---

## 🔧 Code Changes Made This Session

### File: `src/egfx/yuv444_packing.rs`

1. **Added temporal stability logging** (lines 436-458):
   - Computes hash of auxiliary frames
   - Logs when frames are identical vs different
   - Helps diagnose phantom changes

2. **Fixed padding in auxiliary view** (lines 419-445):
   - Pre-allocate chroma with 8×8 padding
   - Fill with deterministic 128 values
   - Use `truncate()` instead of `.to_vec()`

3. **Fixed padding in main view** (lines 238-251):
   - Added same 8×8 macroblock padding
   - Ensures deterministic buffer sizes

### File: `src/egfx/color_convert.rs`

1. **Disabled SIMD for testing** (lines 209, 676):
   - Force scalar code paths
   - Eliminate AVX2 rounding differences
   - Confirmed SIMD is not the cause

### File: `src/egfx/avc444_encoder.rs`

1. **force_all_keyframes flag** (line 248):
   - Temporarily set to `false` (re-enabled P-frames)
   - Proven that keyframes work perfectly

---

## 📊 Test Results Summary

| Test | Result | Latency | Corruption | Conclusion |
|------|--------|---------|------------|------------|
| Original | ❌ | Normal | Heavy lavender in P-frames | Baseline |
| U/V Swap | ❌ | Normal | Worse (keyframes too) | Channels correct |
| All 128s Aux | ❌ | Normal | Total color failure | Aux is critical |
| Simplified Packing | ❌ | Normal | Same lavender | Structure OK |
| **All-Keyframes** | ✅ | **EXTREME** | **NONE!** | **Packing correct!** |
| Padding Fix | 🟡 | Normal | Improved, localized | Partial fix |
| No SIMD | ❌ | Slower | Still corrupted | SIMD not root cause |

---

## 🎓 Key Learnings

1. **All-keyframes test is definitive**: If it works with all-I, packing is correct
2. **Padding matters**: Even minor memory non-determinism causes P-frame issues
3. **SIMD is not the culprit**: Scalar code has same issue
4. **The bug is subtle**: Affects only P-frames, only auxiliary stream
5. **Client reconstruction is complex**: May involve reference frame handling

---

## 🚀 Recommended Next Action

**When you wake up, start here**:

1. **Check encoder NAL settings**:
   ```rust
   // Add to avc444_encoder.rs after encoder creation:
   debug!("Main encoder config: {:?}", main_encoder.config());
   debug!("Aux encoder config: {:?}", aux_encoder.config());
   ```

2. **Add NAL unit logging**:
   ```rust
   // In encode_bgra(), after encoding:
   debug!("Main NAL: type={:?}, ref_idc={}, frame_num=?",
          main_bitstream.nal_type(), main_bitstream.nal_ref_idc());
   debug!("Aux NAL: type={:?}, ref_idc={}, frame_num=?",
          aux_bitstream.nal_type(), aux_bitstream.nal_ref_idc());
   ```

3. **Research FreeRDP implementation**:
   - Clone FreeRDP repo
   - Find their AVC444 encoder
   - Compare encoder settings
   - Look for auxiliary-specific config

4. **Consider asking for help**:
   - FreeRDP mailing list / GitHub issues
   - RDP/H.264 experts
   - Microsoft RDP team (if accessible)

---

## 📚 References

- **MS-RDPEGFX**: Section 3.3.8.3.2 (AVC444 specification)
- **FreeRDP**: https://github.com/FreeRDP/FreeRDP
- **OpenH264**: https://github.com/cisco/openh264
- **This session's docs**:
  - `docs/ANALYSIS-2025-12-27-EVENING.md`
  - `docs/READY-FOR-TOMORROW.md`
  - `docs/AVC444-COMPREHENSIVE-RESEARCH-AND-FIX-2025-12-27.md`

---

## 🌙 Closing Notes

We've made **massive progress** tonight:
- Definitively proven the auxiliary packing algorithm is correct
- Identified P-frame temporal prediction as the root cause
- Eliminated: SIMD, padding (mostly), packing structure, color conversion
- Narrowed down to: encoder reference frame handling or client reconstruction

The fix is close. The next session should focus on **encoder configuration** and **reference frame management**, not on packing algorithms.

**Rest well!** Tomorrow's debugging will be focused and targeted. 💤
