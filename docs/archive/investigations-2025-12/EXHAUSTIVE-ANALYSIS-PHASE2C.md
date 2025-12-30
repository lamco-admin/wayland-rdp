# Exhaustive Analysis: Phase 2c - Still Has Corruption

**Date**: 2025-12-29 03:55 UTC
**Binary MD5**: `b08a81e9baad23f13f899c670088eec4`
**Result**: Extensive lavender corruption persists
**Status**: Single encoder + NUM_REF=2 + Scene change OFF still not sufficient

---

## CONFIGURATION SUMMARY

### What Was Deployed

```rust
let config = OpenH264Config::new()
    .bitrate(5000 kbps)
    .num_ref_frames(2)              // Multi-ref DPB
    .scene_change_detect(false);    // Disable auto-IDR

pub struct Avc444Encoder {
    encoder: Encoder,  // SINGLE encoder for both subframes
}

// Encoding (no force_intra calls):
main_bs = encoder.encode(&main_yuv420);
aux_bs = encoder.encode(&aux_yuv420);
```

**Goal**: Both Main and Aux using P-frames with natural reference selection

---

## NAL STRUCTURE ANALYSIS

### Frame Type Summary

```
Frame #0:  Main: IDR, Aux: IDR
Frame #1:  Main: IDR, Aux: IDR
Frame #7:  Main: P,   Aux: IDR
Frame #8:  Main: P,   Aux: IDR
...
Frame #38: Main: P,   Aux: IDR
```

**Pattern**: Aux NEVER uses P-frames, always IDR (even with scene change disabled!)

### Detailed NAL Analysis (Frame #1 Aux)

```
[Frame #1 AUX NAL#0] type= 1 (P-slice) ref_idc=3 (REFERENCE(3))  ← P-slice present!
[Frame #1 AUX NAL#0] type= 1 (P-slice) ref_idc=3 (REFERENCE(3))  ← Multiple P-slices?
...
[Frame #1 AUX NAL#0] type= 7 (SPS) ref_idc=3 (REFERENCE(3))     ← Then SPS
[Frame #1 AUX NAL#1] type= 8 (PPS) ref_idc=3 (REFERENCE(3))     ← Then PPS
[Frame #1 AUX NAL#2] type= 5 (IDR) ref_idc=3 (REFERENCE(3))     ← Then IDR!
```

**Observation**: Bitstream contains BOTH P-slices AND IDR slice!

### Contradiction

**EncodedBitStream.frame_type()**: Returns IDR
**Actual NAL content**: Contains both P-slices and IDR slice

**This suggests**: Encoder produces P-frames but ALSO inserts IDR (for some reason)

---

## HYPOTHESES FOR WHY AUX STILL GETS IDR

### H1: Encoder Internal Logic Forcing IDR

**Possible triggers**:
- Content completely different from reference (even with scene change off)
- QP threshold exceeded
- Some other internal heuristic
- Complexity mode decision

**Evidence**: NAL logs show IDR slices being produced

---

### H2: Our Encoding Pattern Triggers Special Behavior

**Our pattern**: Main(t), Aux(t), Main(t+1), Aux(t+1)...

**OpenH264 might**:
- Detect alternating frame dimensions? (no - both 1280x800)
- Detect alternating content patterns?
- Have some internal state that resets?

---

### H3: SPS/PPS Reinsertion Forcing IDR

**When we call** `handle_sps_pps()`:
- Aux subframes get SPS/PPS prepended (from cache)
- Maybe this signals to decoder "new sequence"?
- Causes issues?

**But**: Main also gets SPS/PPS prepended and works fine (uses P)

---

### H4: Frame Skip Logic

**OpenH264 has frame skip** for rate control

**Maybe**:
- Encoder tries to skip Aux frames
- When frame is "recovered", inserts IDR
- Our skip_frames setting?

---

## CRITICAL ISSUE: EVEN HYBRID (MAIN-P + AUX-IDR) HAD CORRUPTION

### This is the Most Puzzling Finding

**Phase 2b configuration**:
- Main: P-frames ✓
- Aux: IDR (forced by scene change)
- **Result**: Extensive lavender corruption

**Expected**: This should have been SAFER than both P!
- Aux-IDR has no prediction issues
- Only Main using references
- Should be similar to all-I workaround (which works perfectly)

**But it corrupted!**

**This strongly suggests**: The corruption is NOT solely about Aux P-frame prediction!

---

## POSSIBLE ROOT CAUSES (Revised)

### Cause A: SPS/PPS Handling is Wrong

**Our current logic**:
1. Cache SPS/PPS from first IDR
2. Prepend to all subsequent P-frames (both Main and Aux)

**With single encoder**:
- Main IDR (frame 0) produces SPS/PPS → cached
- Aux IDR (frame 0) produces SPS/PPS → overwrites cache
- Main P-frame (frame 1) gets Aux's SPS/PPS prepended?

**This could cause mismatch!**

**Test**: Don't prepend SPS/PPS, let encoder handle it

---

### Cause B: Subframe Packaging/Ordering Wrong

**MS-RDPEGFX might expect specific packaging**:
- Specific NAL unit ordering
- Specific timing/sequence
- Specific region rectangles

**Our packaging might violate expectations**

---

### Cause C: Client Decoder Bug/Incompatibility

**Maybe our bitstreams are CORRECT** but:
- Windows RDP client has specific expectations
- Our subframe interleaving doesn't match
- Client decoder gets confused

**Test**: Try different client (FreeRDP)?

---

### Cause D: Reference Frame Indices Wrong

**Even with NUM_REF=2**:
- Maybe Main refs wrong frame
- Maybe Aux refs Main instead of Aux
- Need deeper NAL parsing (slice headers, ref lists)

---

## IMMEDIATE NEXT STEPS

### Step 1: Test WITHOUT SPS/PPS Prepending

**Hypothesis**: Our SPS/PPS handling is causing issues

**Change**:
```rust
fn handle_sps_pps(&mut self, data: Vec<u8>, is_keyframe: bool) -> Vec<u8> {
    // Don't cache, don't prepend - let encoder handle it
    data  // Return unchanged
}
```

**Or even simpler**: Just don't call handle_sps_pps at all

**Test**: See if corruption changes or disappears

---

### Step 2: Add More Detailed NAL Logging

**Parse slice headers** to see:
- Which reference frame index is being used
- POC values
- frame_num values

**This requires H.264 bitstream parser** (complex)

---

### Step 3: Try NUM_REF = 4 or Even Higher

**Maybe 2 refs isn't enough** with interleaving

**Try**:
```rust
.num_ref_frames(4)  // Or even 8
```

---

### Step 4: Force Aux to be NON-REFERENCE

**Make Aux frames not used for prediction**

**Need to research**: How to mark frames as non-reference in OpenH264

---

## SESSION TIME LIMIT

We're approaching the context/session limits. I should document current state comprehensively for next session.

---

## WHAT WE'VE LEARNED THIS SESSION

✅ **Root Cause Identified**: Two encoders violate MS-RDPEGFX spec
✅ **Single Encoder Implemented**: Phase 1 successful
✅ **NUM_REF API Extended**: Clean openh264-rs extension
✅ **Scene Change Issue Found**: Was forcing Aux to IDR
✅ **Still Not Solved**: Even with proper config, corruption persists

❌ **Main-P + Aux-IDR corrupted** (unexpected!)
❌ **Both-P configuration** (with scene change off) still corrupts
❌ **Single encoder + multi-ref not sufficient alone**

---

## FOR NEXT SESSION

### State

**Code**: All changes committed and documented
**Binary**: Stable all-I workaround available (MD5: f415eec59d996114...)
**OpenH264-RS**: Extended with num_ref_frames (local fork)

### Documents

START-HERE-2025-12-29.md - Entry point
COMPREHENSIVE-SOLUTION-RESEARCH-2025-12-28.md - All solutions explored
REVISED-SINGLE-ENCODER-PLAN.md - Single encoder approach
PHASE1-IMPLEMENTATION-DETAILS.md - Phase 1 spec
PHASE2-PROPER-DEPLOYED.md - Phase 2 deployment
CRITICAL-FINDING-AUX-ALWAYS-IDR.md - Scene change issue
This document - Exhaustive analysis

### Next Investigations

1. **SPS/PPS handling** - Could be the issue
2. **NUM_REF higher values** - Try 4 or 8
3. **Aux non-reference** - Research how to implement
4. **Client compatibility** - Test with different clients
5. **Deep NAL parsing** - Understand exact references being used

---

## MY ASSESSMENT

**We're close but missing something fundamental**

Single encoder is NECESSARY but not SUFFICIENT.

There's something else about how subframes need to be:
- Packaged
- Ordered
- Referenced
- Or signaled to the client

That we haven't figured out yet.

**Time to consult experts or Microsoft directly?**
