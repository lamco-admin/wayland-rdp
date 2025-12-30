# REFOCUS: Commercial AVC444 Solution Required - No Workarounds

**Date**: 2025-12-29 05:35 UTC
**Reality Check**: All-I is NOT acceptable for production
**Goal**: FULLY WORKING AVC444 with P-frames (commercial quality)
**Mindset**: Solve it completely, not accept partial solutions

---

## THE ACTUAL GOAL

**NOT**: "Stable workaround with acceptable bandwidth"
**YES**: "Full-featured, efficient, commercial-quality AVC444"

**Requirements**:
- ✅ Full 4:4:4 chroma (not 4:2:0)
- ✅ P-frame compression (not all-I wasteful encoding)
- ✅ <2 MB/s bandwidth (not 4.3 MB/s)
- ✅ Perfect quality
- ✅ Rock-solid reliability
- ✅ Production-ready

**Current all-I**: Fails bandwidth requirement, not acceptable

---

## THE BLOCKER WE MUST SOLVE

### Aux Always Produces IDR (Not P-Frames)

**Every single test**, regardless of configuration:
- Scene change ON → Aux produces IDR
- Scene change OFF → Aux produces IDR
- Temporal layers=2 → Aux produces IDR
- **Nothing makes Aux produce P-frames!**

**Why this blocks us**:
- IDR has nal_ref_idc=3 (must be reference per spec)
- IDR enters DPB
- Main P-frames can reference Aux IDR
- **Cross-stream reference → corruption**

### THIS is the Problem to Solve

**Not**: "How to work around Aux-IDR"
**But**: "How to make Aux produce P-frames" OR "How to handle Aux-IDR correctly"

---

## ULTRATHINKING: What Are We Missing?

### Question 1: Is Aux SUPPOSED to be IDR?

**Maybe AVC444 design is**:
- Main: Uses P-frames (compresses well)
- Aux: Always IDR (provides full chroma detail each frame)

**Check**:
- Microsoft's implementation
- FreeRDP server (if exists)
- Academic papers (Wu et al.)
- Spec implications

**If true**: We need different solution (not temporal layers)

---

### Question 2: Are We Encoding in Wrong Order/Pattern?

**Current**: Main(t), Aux(t), Main(t+1), Aux(t+1)...

**Maybe should be**:
- Main(0) IDR, Aux(0) IDR
- Main(1) P, Aux(1) P
- Main(2) P, Aux(2) P
- But with EXPLICIT reference control

**Or**:
- Encode Main GOP (Group of Pictures)
- Then encode Aux GOP
- Not interleaved

**Need**: Research proper encoding pattern from working implementations

---

### Question 3: Do We Need LTR After All?

**Maybe the solution IS**:
- Use LTR properly (not as I misunderstood before)
- Pin Main frames to one LTR slot
- Pin Aux frames to different LTR slot (or make non-LTR)
- Control reference selection explicitly

**Need**: Deep dive into OpenH264 LTR API
- How to mark frame for specific LTR slot
- How to force reference to specific slot
- Example code from OpenH264 tests

---

### Question 4: Is There an OpenH264 Mode We're Missing?

**Simulcast AVC**:
```c
bool bSimulcastAVC;  // Use Simulcast AVC for multiple layers
```

**Could this be relevant**?
- Treat Main and Aux as different "spatial layers"
- OpenH264 handles them as separate streams within one encoder
- Might solve reference issues

**Need**: Research OpenH264 simulcast mode

---

### Question 5: Should We Force Aux to Reference Main?

**Wild idea**: Maybe Aux SHOULD predict from Main
- Aux derived from same source as Main
- Aux predicting from Main might actually work
- Less efficient but might be correct

**Test**: Don't strip anything, let Aux reference whatever encoder picks, see what happens

---

## CRITICAL RESEARCH TASKS

### Priority 1: Find Working AVC444 Server Implementation

**Must find**:
- ANY working AVC444 server encoder
- Open source preferred (FreeRDP, others)
- Or documented example from Microsoft

**What to learn**:
1. Do they use one encoder or two?
2. How do they handle Aux frame types?
3. What's their reference strategy?
4. How do they configure OpenH264?

**This would give us the ANSWER**, not guesses!

---

### Priority 2: OpenH264 Deep Source Analysis

**Specifically research**:
1. Why does encoder force IDR for Aux?
2. What triggers IDR insertion (beyond scene change)?
3. Is there a parameter to prevent it?
4. How do temporal layers ACTUALLY work with frame types?

**Files to examine**:
- `encoder_ext.cpp` - Main encoding logic
- `slice decision logic` - When P vs I vs IDR
- `reference management` - DPB handling
- Temporal layer assignment code

---

### Priority 3: Microsoft/Expert Consultation

**If we can't find working code**:

**Contact**:
1. Microsoft RDP team (official)
2. FreeRDP developers (open source)
3. OpenH264 maintainers (encoder experts)
4. Wu et al. (original researchers)

**Ask specific questions**:
- How is AVC444 server encoding SUPPOSED to work?
- Should Aux use P-frames or always IDR?
- How to prevent cross-stream references?
- Working example code?

---

## WHAT TO TRY NEXT

### Experiment 1: Simulcast Mode

**Test if OpenH264 simulcast helps**:
```rust
params.bSimulcastAVC = true;
params.iSpatialLayerNum = 2;  // Treat Main/Aux as spatial layers?
```

**Maybe**: This handles dual streams properly

---

### Experiment 2: Don't Strip Anything

**Theory**: Maybe our stripping is wrong

**Test**: Send exactly what encoder produces
- Don't strip SPS/PPS
- Don't strip IDR
- Let client handle it

**See**: Does it work? Different corruption? Clues?

---

### Experiment 3: Force Aux Content to be More Similar

**Theory**: Extreme content difference forces IDR

**Test**: Use simpler Aux packing
- Instead of full chroma detail
- Use averaged or interpolated values
- Make Aux "look more like" a video frame

**See**: Does Aux then produce P-frames?

---

### Experiment 4: Two Encoders with Reset Synchronization

**Theory**: Use two encoders but sync them

**Test**:
- Use two encoders
- But reset Aux encoder after each Main encode
- Or force Aux to have same DPB state as Main somehow

**Challenge**: OpenH264 doesn't expose DPB cloning

---

## THE REAL PRIORITY

**STOP accepting workarounds**
**START finding THE solution**

**Immediate action**: Research working implementations exhaustively
- Spend 2-3 hours finding ANY working AVC444 server code
- Learn from it
- Implement properly

**If no working examples exist**:
- This might indicate AVC444 server-side is rarely/never done
- Or always uses all-I
- Or there's a reason it's hard

**But**: Don't give up until we KNOW for certain

---

## COMMITMENT

**Goal**: Commercial-quality AVC444 with P-frames
**Bandwidth target**: <2 MB/s
**Quality**: Perfect
**Reliability**: 100%

**Not acceptable**: 4.3 MB/s all-I workaround

**Next**: Deep research on working implementations and OpenH264 source

**Mindset shift**: This MUST work, find how.
