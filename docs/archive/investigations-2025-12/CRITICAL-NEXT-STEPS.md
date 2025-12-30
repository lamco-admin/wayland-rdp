# 🚨 CRITICAL NEXT STEPS - Start Here Tomorrow

**Date**: 2025-12-27 Night → 2025-12-28 Morning
**Status**: P-frame corruption isolated but not fixed
**Key Finding**: **All-keyframes works perfectly → Packing is correct, P-frames are broken**

---

## ✅ What We Know FOR SURE

1. **Auxiliary packing algorithm is CORRECT** (proven by all-keyframes test)
2. **Color conversion is CORRECT** (works in all-keyframes)
3. **Main view is CORRECT** (works in all-keyframes)
4. **The client is NOT the problem** (Windows RDP client works with other servers)
5. **The issue is in OUR P-frame encoder configuration**

---

## 🎯 The Smoking Gun: Reference Frame Configuration

**Theory from expert session**:
> "Auxiliary pictures are contaminating reference state (wrong DPB / POC / frame_num behavior)"

### What This Means

In AVC444, we encode TWO H.264 streams:
- **Main stream**: Normal video (luma + chroma)
- **Auxiliary stream**: Extra chroma data

**The Problem**: If the auxiliary stream pictures are marked as "reference pictures", they contaminate the decoder's reference picture buffer (DPB). When the decoder tries to use them for motion compensation, it causes corruption.

**The Solution**: Auxiliary pictures must be marked as **NON-REFERENCE**.

---

## 🔧 Action Items (Do These First)

### 1. Check OpenH264 Reference Frame Settings

**File**: `src/egfx/avc444_encoder.rs`

**Current config** (lines 198-202):
```rust
let mut encoder_config = OpenH264Config::new()
    .bitrate(BitRate::from_bps(config.bitrate_kbps * 1000))
    .max_frame_rate(FrameRate::from_hz(config.max_fps))
    .skip_frames(config.enable_skip_frame)
    .usage_type(UsageType::ScreenContentRealTime);
```

**What to add**:
```rust
// Try adding these configurations to the AUXILIARY encoder only:
encoder_config = encoder_config
    .max_num_ref_frames(0)  // Don't use auxiliary as reference?
    .enable_long_term_reference(false)  // No long-term refs?
```

### 2. Research OpenH264 Rust Bindings

Check what methods are available:
```bash
# In your project:
grep -r "impl.*EncoderConfig" ~/.cargo/registry/src/
# Or check docs:
cargo doc --open --package openh264
```

Look for methods related to:
- `num_ref_frames`
- `nal_ref_idc`
- `long_term_reference`
- `reference_pictures`

### 3. Compare with FreeRDP's AVC444 Encoder

**Clone FreeRDP**:
```bash
cd ~/wayland
git clone https://github.com/FreeRDP/FreeRDP.git
cd FreeRDP
```

**Find AVC444 encoder**:
```bash
grep -r "AVC444\|avc444" libfreerdp/codec/
grep -r "auxiliary" libfreerdp/codec/
```

**Look for**:
- How they configure OpenH264 for auxiliary stream
- Any special flags or settings
- Reference frame configuration

### 4. Test: Disable Reference Frames in Auxiliary

**Quick test**: Try making auxiliary encoder use ONLY I-frames (different from main):

```rust
// In avc444_encoder.rs, modify encode_bgra():

// Force auxiliary to always be I-frame (not IDR, just I)
self.aux_encoder.force_intra_frame();

// Keep main encoder normal (mix of I/P frames)
// Don't force main encoder
```

This will increase bandwidth but might eliminate corruption.

---

## 🔍 Diagnostic Logging to Add

Add to `encode_bgra()` after encoding:

```rust
// After line 358 (aux_bitstream creation):
if self.frame_count % 30 == 0 {
    debug!("Frame {}: Main={:?}, Aux={:?}",
           self.frame_count,
           main_bitstream.frame_type(),
           aux_bitstream.frame_type());
}
```

This will show if main and aux are producing different frame types.

---

## 📖 Background Reading

### The Reference Picture Problem

In H.264:
- **Reference pictures**: Stored in DPB, used for motion compensation in P/B frames
- **Non-reference pictures**: Not stored, can't be used for prediction

For AVC444:
- **Main stream**: Normal reference behavior (I-frames are refs, P-frames use refs)
- **Auxiliary stream**: Should be **NON-REFERENCE** (just extra data, don't use for prediction)

If auxiliary pictures get into DPB, decoder might try to use them for motion compensation → corruption.

### NAL Unit Structure

Every NAL unit has:
```
nal_header = (forbidden_zero_bit << 7) | (nal_ref_idc << 5) | nal_unit_type
```

- `nal_ref_idc = 0`: Non-reference (discard after decoding)
- `nal_ref_idc > 0`: Reference (store in DPB)

**We need**: Auxiliary NAL units with `nal_ref_idc = 0`

---

## 🎲 Quick Tests to Try

### Test A: All-I Auxiliary (Keep Main Normal)
```rust
// In encode_bgra(), before aux encoding:
self.aux_encoder.force_intra_frame();  // ALWAYS

// Keep main encoder normal
```
**Expected**: Should fix corruption (but high bandwidth)

### Test B: Separate Frame Numbers
Check if main and aux are using same frame_num sequence (they shouldn't interfere if truly separate encoders, but worth verifying).

### Test C: Different SPS/PPS
Verify main and aux have truly separate SPS/PPS (check the cached values are different).

---

## 📊 Current Binary State

**Last deployed**: MD5 `d75455a9c4d4ffa700525d1313ee1af1`
**Config**: SIMD disabled, padding added, P-frames enabled
**Result**: Still corrupted (but slightly better localized)

**To revert to baseline**:
1. Re-enable SIMD (remove `&& false` in color_convert.rs lines 209, 676)
2. Keep padding fixes (those help)
3. Focus on encoder reference config

---

## 🌅 Morning Checklist

- [ ] Read this document
- [ ] Check OpenH264-rs docs for reference frame config
- [ ] Try forcing auxiliary to all-I frames
- [ ] Compare with FreeRDP implementation
- [ ] Add diagnostic logging for frame types
- [ ] Test with max_num_ref_frames(0) on auxiliary

---

**The fix is close. We know EXACTLY where the problem is now.**
Focus on **encoder configuration**, not packing algorithms.

Good luck! 🚀
