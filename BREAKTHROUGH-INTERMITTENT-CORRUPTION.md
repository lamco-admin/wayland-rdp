# BREAKTHROUGH: Intermittent Corruption Pattern Reveals the Truth

**Date**: 2025-12-29 04:20 UTC
**Finding**: Corruption is NOT constant - it comes and goes!
**This is MASSIVE progress** - we've changed the corruption pattern!

---

## THE PATTERN

### Frames That Display CORRECTLY (No Corruption)

**All frames where BOTH Main and Aux are IDR**:
- Frames 0-6 (initial)
- Frames 33-34
- Frame 38
- Frame 59
- Frames 73-77
- Frames 88-91

**These frames**: User sees "perfectly good modification comes through"

### Frames That Display CORRUPTED (Lavender)

**Frames where Main is P-frame**:
- Frames 7-32
- Frames 35-37, 39-58
- Frames 60-72, 76, 78-87
- Frames 92-99

**These frames**: User sees lavender corruption

---

## CRITICAL INSIGHT: THE PROBLEM IS MAIN P-FRAMES

### The Smoking Gun

**Configuration**: Main-P + Aux-IDR (since Aux is always IDR despite scene change off)

**Result**:
- ✅ Both-IDR frames: **PERFECT** (no corruption)
- ❌ Main-P + Aux-IDR frames: **CORRUPTED**

**Conclusion**: **Main P-frame prediction is the problem!**

Not Aux P-frames (Aux never used P anyway)!

### Why This is Shocking

**We expected**: Aux P-frames to be the problem (chroma-as-luma)

**Reality**: **Main P-frames corrupt** even with Aux-IDR!

**This means**: Something about having Aux in the DPB or interleaved encoding breaks Main's P-frame prediction!

---

## WHAT THIS TELLS US

### The Issue is Cross-Contamination

**Theory**: When we encode Aux (even as IDR), it goes into the DPB

**Then**: When Main tries to encode P-frame, it searches DPB

**DPB contains**: Previous Main + Aux frames

**Problem**: Main's motion search might:
1. Reference Aux frames (wrong stream!)
2. Or Aux presence corrupts DPB state somehow
3. Or reference frame indices get confused

**When Main sends IDR**: Doesn't need references → displays correctly!

---

## WHY AUX IS STILL IDR (Despite Scene Change Off)

**Observation**: Aux ALWAYS IDR in all our tests

**Possible reasons**:
1. First encode after Main automatically triggers IDR?
2. Some other OpenH264 heuristic forcing IDR?
3. Our code logic somehow forcing it?

**But this is actually IRRELEVANT now** because the corruption happens with Main P-frames, regardless of Aux type!

---

## THE REAL ROOT CAUSE (Revised Theory)

### Main P-Frames Search Wrong References

**Encoding sequence with single encoder**:
```
Frame 0:
  Encode Main IDR → DPB = [Main_0]
  Encode Aux IDR → DPB = [Main_0, Aux_0]

Frame 1:
  Encode Main P-frame:
    - Motion search looks in DPB
    - DPB contains: [Main_0, Aux_0]
    - Search might find Aux_0 as "best match" for some blocks!
    - Or even if finds Main_0, indices might be wrong
    - Predicts from wrong data
    → CORRUPTION!

  Encode Aux IDR:
    - Doesn't use references
    - Clean
```

**Result**: Main corrupted, Aux clean, combined = lavender artifacts

**When Main sends IDR**: No search, no references → clean!

---

## SOLUTION PATHS

### Solution 1: Make Aux NON-REFERENCE

**Prevent Aux from entering reference pool**:
- Aux frames marked as nal_ref_idc=0 (non-reference)
- DPB doesn't include them
- Main can ONLY reference other Main frames
- **Aux can't contaminate Main's prediction!**

**This is THE solution** (90% confidence)

---

### Solution 2: Use LTR Properly

**Pin Main frames only**:
- Mark Main_0 as LTR slot 0
- Don't mark Aux as LTR (or mark as non-ref)
- Main P-frames forced to reference LTR slot 0
- Aux uses IDR (as it's already doing)

---

### Solution 3: Separate Encoders After All?

**Maybe the spec is flexible**:
- Use two encoders but coordinate them somehow
- Or accept all-I for Aux (proven safe)

---

## IMMEDIATE NEXT STEP

**Implement**: Make Aux non-reference (nal_ref_idc=0)

**How**:
1. Research if OpenH264 exposes "mark as non-reference"
2. Or post-process NAL headers to set nal_ref_idc=0
3. Test if Main P-frames work correctly when Aux not in DPB

**This should eliminate corruption!**

---

## WHY THIS EXPLAINS EVERYTHING

**All our tests**:
- Dual encoder all-I: Both IDR → Clean ✅
- Dual encoder P+P: Cross-ref issue → Corrupt ❌
- Single encoder all-I: Both IDR → Clean ✅
- Single encoder Main-P: Main refs wrong → Corrupt ❌
- **This test**: Confirms Main-P is the issue!

**The pattern fits perfectly!**

---

## USER OBSERVATION VALIDATES THIS

> "Corruption ceases and perfectly good modification comes through"

**This happened when**: Main sent IDR (frames 33-34, 38, 59, 73-77, 88-91)

**Corruption persisted**: When Main sent P-frames

**This is diagnostic gold!** Tells us EXACTLY where the problem is.

---

## CONFIDENCE LEVEL

**That Aux contaminating Main's DPB is the issue**: 95%

**That making Aux non-reference fixes it**: 90%

**Next**: Implement aux non-reference mechanism

**This is THE breakthrough we needed!**
