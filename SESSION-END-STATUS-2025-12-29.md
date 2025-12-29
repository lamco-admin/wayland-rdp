# Session End Status - 2025-12-29

**Time**: ~5 hours of intensive investigation and implementation
**Status**: Significant progress, P-frame corruption not yet solved
**Current**: Stable all-I workaround active, can revert anytime

---

## MAJOR ACCOMPLISHMENTS

### 1. Root Cause Identified ✅

**MS-RDPEGFX Specification Requirement**:
> "The two subframe bitstreams MUST be encoded using the **same H.264 encoder**"

**Our Original Implementation**: TWO separate encoder instances (spec violation)
**Impact**: Separate DPBs → P-frame reference mismatches → corruption

### 2. Single Encoder Architecture Implemented ✅

**Phase 1**: Structural refactor
- Changed from dual encoders to single encoder
- Merged SPS/PPS caching
- Validated with all-I frames (perfect quality)
- **~150 lines changed**

### 3. OpenH264-RS Extended ✅

**Added**: `.num_ref_frames(num)` fluent API
- Exposes `iNumRefFrame` parameter
- Configured at encoder creation (not runtime)
- Follows same pattern as VUI support
- **~40 lines added**
- Ready to add to existing VUI PR

### 4. Extensive Testing Conducted ✅

**Tests performed**:
- ✅ Deblocking filter disable (ruled out)
- ✅ Quantization increase 3x (ruled out)
- ✅ Single encoder all-I (validated)
- ✅ Single encoder with NUM_REF (Aux forced to IDR)
- ✅ Scene change detection disabled (Aux still IDR)

**What we've ruled out**:
- Deblocking filter
- Quantization damage
- Padding/stride issues
- Packing algorithm bugs
- Color conversion bugs
- Input nondeterminism
- Dual encoder architecture (necessary but not sufficient)

---

## CURRENT PUZZLE

### What DOESN'T Make Sense

**Observation**: Even Main-P + Aux-IDR has corruption

**Expected**: This should be safe!
- Aux using IDR (no prediction, like all-I)
- Only Main using P-frames
- Should be similar to hybrid workaround

**But**: Had same extensive lavender corruption

**This suggests**: Something else is fundamentally wrong beyond just "which frames use P"

### Possible Explanations

**A) SPS/PPS Handling Bug**:
- Single encoder produces one set of SPS/PPS
- Maybe caching/prepending logic is wrong for interleaved subframes
- Could be mixing Main and Aux SPS/PPS incorrectly

**B) Subframe Packaging Wrong**:
- How we package into RFX_AVC444_BITMAP_STREAM
- Region rectangles
- Timing/ordering

**C) Client Decoder Expectations**:
- Windows RDP client might have specific requirements
- Our bitstream structure doesn't match
- Even if "spec compliant", client is pickier

**D) Reference Management Still Wrong**:
- NUM_REF=2 might not be enough
- Or references still crossing streams incorrectly

---

## MYSTERIOUS NAL OBSERVATIONS

### Aux Bitstreams Contain Both P-slices and IDR

**From NAL logs (Frame #1 Aux)**:
```
NAL#0: type=1 (P-slice)
NAL#0: type=1 (P-slice)  ← Multiple?
...
NAL#0: type=7 (SPS)
NAL#1: type=8 (PPS)
NAL#2: type=5 (IDR)
```

**Questions**:
1. Why multiple NAL#0 entries? (logging bug or actual multiple NALs?)
2. Why both P-slice AND IDR in same bitstream?
3. Is encoder producing both or is our code doubling something?

**Need**: Better understanding of what encoder actually produces

---

## NEXT INVESTIGATIVE STEPS

### Priority 1: Test SPS/PPS Handling

**Hypothesis**: Our SPS/PPS prepending is causing issues

**Test A**: Disable SPS/PPS prepending entirely
```rust
fn handle_sps_pps(&mut self, data: Vec<u8>, is_keyframe: bool) -> Vec<u8> {
    data  // Just return unchanged
}
```

**Test B**: Only prepend to Main, not Aux
**Test C**: Extract fresh SPS/PPS from each subframe separately

---

### Priority 2: Increase NUM_REF

**Try NUM_REF = 4 or 8**

Maybe DPB size 2 isn't sufficient with interleaving

---

### Priority 3: Deep NAL Parsing

**Parse slice headers** to see:
- Actual reference frame indices
- POC values
- frame_num sequencing
- Which frames are in ref list 0

**Requires**: H.264 bitstream parser (complex but necessary)

---

### Priority 4: Research Similar Implementations

**Find working AVC444 server implementations**:
- Microsoft's (proprietary but might have docs)
- Any open source projects
- Academic papers with code

---

### Priority 5: Expert Consultation

**Reach out to**:
- FreeRDP developers
- OpenH264 community
- Microsoft RDP team
- Academic researchers (Wu et al.)

**Ask specific questions about**:
- Single encoder with dual subframes
- Reference frame management
- Client expectations

---

## STABLE FALLBACK

### Current Working Configuration

**Binary MD5**: `f415eec59d996114a97923a11a2ba087`

**Configuration**: All-I frames (both Main and Aux)
**Quality**: Perfect (user verified extensively)
**Bandwidth**: ~4.3 MB/s

**This can be used in production** if needed while we solve P-frames

---

## OPENH264-RS FORK STATUS

### Changes Made

**File**: `/home/greg/openh264-rs/openh264/src/encoder.rs`

**Added**:
1. `num_ref_frames: Option<i32>` field to EncoderConfig
2. `.num_ref_frames(num)` fluent API method
3. Application of `params.iNumRefFrame` during init

**Status**: Compiles, tested locally
**Ready**: To commit and add to VUI PR later

**Temporarily**: Using local path in Cargo.toml
**Eventually**: Will push to feature/vui-support branch

---

## DOCUMENTS CREATED THIS SESSION

### Investigation

1. SMOKING-GUN-SINGLE-ENCODER-REQUIREMENT.md
2. CRITICAL-FINDING-DEBLOCKING-RULED-OUT.md
3. CRITICAL-FINDING-AUX-ALWAYS-IDR.md
4. IMPORTANT-PHASE2-WAS-AVC420-AGAIN.md
5. EXHAUSTIVE-ANALYSIS-PHASE2C.md (this document)

### Implementation

6. SINGLE-ENCODER-ARCHITECTURE-RESEARCH.md
7. REVISED-SINGLE-ENCODER-PLAN.md
8. PHASE1-IMPLEMENTATION-DETAILS.md
9. PHASE2-PROPER-DEPLOYED.md
10. STATUS-SCENE-CHANGE-DISABLED.md

### Analysis

11. COMPREHENSIVE-SOLUTION-RESEARCH-2025-12-28.md (15+ solutions)
12. OPENH264-RS-NUMREF-EXTENSION-ANALYSIS.md

**All committed to git and pushed**

---

## RECOMMENDED NEXT SESSION ACTIONS

### 1. Test SPS/PPS Theory (30 min)

Disable SPS/PPS prepending, test if corruption changes

### 2. Increase NUM_REF (15 min)

Try NUM_REF=4, then 8, see if helps

### 3. Deep NAL Parsing (2-3 hours)

Implement proper H.264 slice header parser
Understand exact reference behavior

### 4. Research Working Implementations (2-3 hours)

Find ANY working AVC444 server encoding example
Understand what we're missing

### 5. Consider Consulting Experts

Might be time to ask for help from:
- FreeRDP team
- Microsoft
- OpenH264 developers

---

## KEY INSIGHTS FOR NEXT SESSION

**Critical Unknowns**:
1. Why does Main-P + Aux-IDR corrupt? (Should be safe!)
2. Why does Aux get IDR even with scene change off?
3. Is our SPS/PPS handling causing issues?
4. Are we missing a critical MS-RDPEGFX requirement?

**Next session should**:
- Read this document first
- Try SPS/PPS theory (quick test)
- Consider expert consultation if still stuck

**We've made MASSIVE progress** on architecture, but there's a missing piece.
