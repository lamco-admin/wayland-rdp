# SPS/PPS Theory Test - Critical Hypothesis

**Date**: 2025-12-29 04:00 UTC
**Binary MD5**: `99a2ec31c6e26689da2aca0ab1bafcd9`
**Hypothesis**: Dual SPS/PPS sets confuse decoder

---

## THE THEORY

### What We Were Doing (Potentially Wrong)

```
Main stream:  [SPS_main][PPS_main][IDR_main or P_main]
Aux stream:   [SPS_aux][PPS_aux][IDR_aux or P_aux]
```

**Both streams have SPS/PPS!**

**If client concatenates** (as "one stream" requirement suggests):
```
combined = [SPS_main][PPS_main][IDR_main][SPS_aux][PPS_aux][IDR_aux]
```

**Decoder sees**: TWO SPS/PPS sets → interprets as TWO separate streams? → confusion?

### What We're Testing Now

```
Main stream:  [SPS][PPS][IDR_main or P_main]
Aux stream:   [IDR_aux or P_aux]  ← NO SPS/PPS!
```

**Aux shares Main's SPS/PPS**

**If client concatenates**:
```
combined = [SPS][PPS][IDR_main][IDR_aux]  ← ONE SPS/PPS set!
```

**Decoder sees**: ONE unified stream with shared parameters

---

## CODE CHANGES

### Change 1: Don't Prepend to Aux

```rust
// BEFORE:
stream1_data = self.handle_sps_pps(stream1_data, main_is_keyframe);
stream2_data = self.handle_sps_pps(stream2_data, aux_is_keyframe);

// AFTER:
stream1_data = self.handle_sps_pps(stream1_data, main_is_keyframe);
// stream2_data NOT processed - no prepending
```

### Change 2: Strip SPS/PPS from Aux

**New function**: `strip_sps_pps()`
- Parses Annex B bitstream
- Removes NAL type 7 (SPS) and type 8 (PPS)
- Keeps IDR slices, P-slices, etc.

```rust
stream2_data = Self::strip_sps_pps(stream2_data);
```

**Result**: Aux bitstream has ONLY the slice data, no parameter sets

---

## EXPECTED OUTCOMES

### If This Fixes Corruption ✅

**Means**: Dual SPS/PPS WAS the problem!
- Client expects shared SPS/PPS
- Our code was violating this
- Simple fix!

**Next**:
- Document solution
- Verify bandwidth and quality
- Commit and push

---

### If Still Corrupts ❌

**Means**: SPS/PPS not the issue
**Next**: Test Theory 2 (cross-stream reference selection)
- Motion search picking wrong stream frames
- Need to constrain references
- Likely need aux non-reference or LTR

---

### If Decoder Errors/Crashes ⚠️

**Means**: Aux DOES need SPS/PPS (or Main's explicitly)
**Adjust**: Try prepending Main's SPS/PPS to Aux instead of Aux's own

---

## WHY THIS THEORY IS STRONG

### Evidence

1. **"One stream" requirement**: Both subframes decoded together
2. **Our all-I works**: When both have SPS/PPS as IDR, maybe OK
3. **Our P-frames fail**: When mixing SPS/PPS with P-frames, issues arise
4. **Main-P + Aux-IDR corrupted**: Both had separate SPS/PPS

### Precedent

**Standard H.264**: ONE SPS/PPS per stream
**AVC444 as "one stream"**: Should have ONE SPS/PPS total
**We had**: TWO SPS/PPS sets → violation?

---

## TEST NOW

**Rigorous test**:
1. Connect RDP
2. Scroll terminal text
3. Move windows
4. Watch for lavender corruption

**Critical observations**:
- Corruption: present/absent/different?
- Decoder errors: any crashes/freezes?
- Quality: colors correct?

**This could be THE solution or rule out a major theory.**

Report back what you see!
