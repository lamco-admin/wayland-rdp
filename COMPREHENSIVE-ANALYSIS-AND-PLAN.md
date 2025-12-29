# AVC444 P-Frame Corruption - Comprehensive Analysis and Implementation Plan

**Date**: 2025-12-29 02:00 UTC
**Status**: Research complete, ready for implementation planning review
**Current State**: Stable all-I workaround active

---

## EXECUTIVE SUMMARY

### Root Cause Identified

**MS-RDPEGFX Specification Violation**:
> "The two subframe bitstreams MUST be encoded using the **same H.264 encoder**"

**Our Implementation**: Uses TWO separate encoder instances
**Result**: Separate DPBs → P-frame reference mismatches → lavender corruption

### Confirmed via Systematic Testing

✅ Deblocking filter ruled out (disabled, still corrupted)
✅ Quantization ruled out (3x bitrate, still corrupted)
✅ Padding ruled out (stride fix applied, no padding in aux_u/aux_v)
✅ Packing algorithm verified correct (matches FreeRDP)
✅ Color conversion verified correct
✅ Stream synchronization verified (both IDR, both P)

**Remaining cause**: Architectural spec violation (two encoders vs one)

---

## PROPOSED SOLUTION: Single Encoder with LTR

### Architecture Overview

```rust
pub struct Avc444Encoder {
    encoder: Encoder,  // ONE encoder for BOTH subframes

    // Long Term Reference (LTR) slot management:
    // - Slot 0: Last main subframe
    // - Slot 1: Last auxiliary subframe
    main_ltr_slot: i32,
    aux_ltr_slot: i32,
}
```

### Key Innovation: LTR Enables Separate Reference Chains

**OpenH264 LTR Feature**:
- Mark frames to stay in DPB as "long term reference"
- Assign to specific slots (up to 2 slots supported)
- P-frames can reference specific LTR slots

**For AVC444**:
- Main subframe → Mark as LTR slot 0
- Aux subframe → Mark as LTR slot 1
- Main P-frames → Reference slot 0 (previous main)
- Aux P-frames → Reference slot 1 (previous aux)
- **Result**: Separate reference chains in ONE DPB!

---

## PHASED IMPLEMENTATION PLAN

### Phase 1: Structural Refactor (Single Encoder, All-I) - 2 hours

**Goal**: Migrate from two encoders to one, maintain all-I workaround

**Changes**:
```rust
// Before:
pub struct Avc444Encoder {
    main_encoder: Encoder,
    aux_encoder: Encoder,
}

// After:
pub struct Avc444Encoder {
    encoder: Encoder,
}
```

**Encoding**:
```rust
fn encode_bgra(...) {
    let (main, aux) = pack_dual_views(&yuv444);

    self.encoder.force_intra_frame();
    let main_bs = self.encoder.encode(&main)?;

    self.encoder.force_intra_frame();
    let aux_bs = self.encoder.encode(&aux)?;
}
```

**Test**: Verify quality same as current all-I (should be identical)

**Success Criteria**: No regression, same perfect quality

---

### Phase 2: Test Basic P-Frames Without LTR - 1 hour

**Goal**: See what happens with naive single encoder P-frames

**Changes**:
```rust
fn encode_bgra(...) {
    let (main, aux) = pack_dual_views(&yuv444);

    // Let encoder use P-frames naturally
    let main_bs = self.encoder.encode(&main)?;
    let aux_bs = self.encoder.encode(&aux)?;  // What does this reference?
}
```

**Possible Outcomes**:

**A) Corruption eliminated** ✅
- Single encoder "just works"
- DPB management automatically correct
- **Solution found!**

**B) Different corruption pattern** ⚠️
- Better than two encoders
- But still issues
- Move to Phase 3 (LTR)

**C) Same/worse corruption** ❌
- Naive approach doesn't help
- Need LTR (Phase 3)
- Or something else is wrong

**Value**: This tells us if we need LTR complexity or not

---

### Phase 3: Implement LTR Reference Strategy - 4-6 hours

**Goal**: Use LTR to maintain separate main/aux reference chains

#### Step 3.1: Enable LTR

```rust
unsafe fn enable_ltr(encoder: &mut Encoder) -> Result<()> {
    let raw_api = encoder.raw_api();

    let num_ltr_frames: i32 = 2;  // Main + Aux
    const ENCODER_OPTION_LTR: i32 = 18;

    let result = raw_api.set_option(
        ENCODER_OPTION_LTR,
        &num_ltr_frames as *const i32 as *mut c_void
    );

    if result != 0 {
        return Err(...);
    }

    Ok(())
}
```

#### Step 3.2: Mark Frames as LTR

```rust
unsafe fn mark_as_ltr(encoder: &mut Encoder, slot: i32) -> Result<()> {
    use openh264_sys2::SLTRMarkingFeedback;

    let raw_api = encoder.raw_api();

    let mut feedback = SLTRMarkingFeedback {
        uiFeedbackType: 4,  // LTR_MARKING_SUCCESS
        uiIDRPicId: 0,
        iLTRFrameNum: slot,  // 0 for main, 1 for aux
        iLayerId: 0,
    };

    const ENCODER_LTR_MARKING_FEEDBACK: i32 = 16;

    let result = raw_api.set_option(
        ENCODER_LTR_MARKING_FEEDBACK,
        &mut feedback as *mut _ as *mut c_void
    );

    if result != 0 {
        return Err(...);
    }

    Ok(())
}
```

#### Step 3.3: Request LTR Recovery (Force Reference to Specific Slot)

```rust
unsafe fn use_ltr_reference(encoder: &mut Encoder, slot: i32) -> Result<()> {
    use openh264_sys2::SLTRRecoverRequest;

    let raw_api = encoder.raw_api();

    let mut request = SLTRRecoverRequest {
        uiFeedbackType: 1,  // LTR_RECOVERY_REQUEST
        uiIDRPicId: 0,
        iLastCorrectFrameNum: slot,
        iCurrentFrameNum: /* current */,
        iLayerId: 0,
    };

    const ENCODER_LTR_RECOVERY_REQUEST: i32 = 15;

    let result = raw_api.set_option(
        ENCODER_LTR_RECOVERY_REQUEST,
        &mut request as *mut _ as *mut c_void
    );

    // ...
}
```

#### Step 3.4: Full Encoding Flow with LTR

```rust
pub fn encode_bgra(...) -> Result<Avc444Frame> {
    let (main, aux) = pack_dual_views(&yuv444);

    // === ENCODE MAIN SUBFRAME ===
    let main_bitstream = if self.frame_count == 0 {
        // First frame: IDR
        self.encoder.encode(&main)?
    } else {
        // P-frame: Request reference to LTR slot 0 (previous main)
        unsafe {
            self.use_ltr_reference(self.main_ltr_slot)?;
        }
        self.encoder.encode(&main)?
    };

    // Mark main frame as LTR slot 0
    unsafe {
        self.mark_as_ltr(self.main_ltr_slot)?;
    }

    // === ENCODE AUX SUBFRAME ===
    let aux_bitstream = if self.frame_count == 0 {
        // First frame: IDR
        self.encoder.encode(&aux)?
    } else {
        // P-frame: Request reference to LTR slot 1 (previous aux)
        unsafe {
            self.use_ltr_reference(self.aux_ltr_slot)?;
        }
        self.encoder.encode(&aux)?
    };

    // Mark aux frame as LTR slot 1
    unsafe {
        self.mark_as_ltr(self.aux_ltr_slot)?;
    }

    // Package into AVC444Frame
    // ...
}
```

---

## ALTERNATIVE APPROACHES (If LTR Doesn't Work)

### Alternative A: Reference Frame Count Limitation

**Approach**: Configure encoder to keep exactly 2 reference frames

```rust
// Via SEncParamExt:
params.iNumRefFrame = 2;  // Keep last 2 frames in DPB

// Encoding order:
// Frame 0: Main IDR → DPB[0]
// Frame 1: Aux IDR → DPB[1]
// Frame 2: Main P → DPB[0,1] - might auto-reference DPB[0] (previous main)
// Frame 3: Aux P → DPB[1,2] - might auto-reference DPB[1] (previous aux, evicted DPB[0])
```

**Theory**: Natural FIFO eviction might create correct reference pattern

**Testing needed**: Verify DPB eviction order

---

### Alternative B: Selective Force-Intra

**Approach**: Main uses P-frames, Aux uses I-frames

```rust
fn encode_bgra(...) {
    // Main: Normal P-frame encoding
    let main_bs = self.encoder.encode(&main)?;

    // Aux: Always force I-frame
    self.encoder.force_intra_frame();
    let aux_bs = self.encoder.encode(&aux)?;
}
```

**This is single encoder version of hybrid workaround**

**Bandwidth**: Better than full all-I, worse than full P+P
**Quality**: Should be perfect (aux I-frames work)
**Complexity**: Low

---

### Alternative C: Non-Reference Marking

**Approach**: Mark aux frames as non-reference

```rust
// After encoding aux:
// Set nal_ref_idc = 0 in NAL header
// This tells decoder not to use aux for prediction
```

**Requires**: NAL unit parsing and modification

**Complexity**: Medium-High

---

## RISKS AND UNKNOWNS

### High Priority Unknowns

**U1: Does OpenH264 LTR actually work as we expect?**
- Can we mark to specific slots?
- Can we force reference to specific slots?
- Does it work with screen content mode?

**U2: Does client expect specific LTR usage?**
- Windows RDP might have assumptions
- Could reject our bitstream if wrong

**U3: DPB size with dual subframes**
- 2 LTR slots + normal ref frames
- Might exceed DPB capacity quickly
- Level 4.0 limits

### Medium Priority Unknowns

**U4: Performance impact**
- LTR management overhead
- Encoding time comparison

**U5: Bitrate distribution**
- Single target for both subframes
- How does encoder split it?

### Low Priority Unknowns

**U6: SPS/PPS changes**
- LTR might require different SPS
- Need to test

---

## RECOMMENDED NEXT STEPS

### Before Implementation

1. **Review this plan** with you
2. **Decide on approach**:
   - Phased (recommended) vs all-at-once
   - LTR vs simpler alternative
3. **Confirm risk tolerance**
4. **Allocate time** (7-12 hours estimated)

### Implementation Sequence

1. **Phase 1**: Single encoder all-I (2 hrs) - Validates structure
2. **Phase 2**: Test basic P-frames (1 hr) - Might "just work"
3. **Phase 3**: Implement LTR if needed (4-6 hrs) - Complex but proper
4. **Phase 4**: Optimization (2-3 hrs) - Fine-tuning

### Fallback Strategy

If all above fails:
- Keep current all-I workaround (known working)
- Document as "AVC444 with efficient P-frames not yet supported"
- Bandwidth cost acceptable for high-quality 4:4:4

---

## CONFIDENCE ASSESSMENT

**That single encoder fixes it**: 75%
- Matches spec requirement
- Explains all symptoms
- Logical architecture

**That LTR is needed**: 60%
- Might work without LTR (Phase 2 test)
- LTR provides control if needed

**That this is solvable**: 90%
- Multiple paths forward
- Fallback options exist
- Worst case: keep all-I

---

## READY FOR YOUR DECISION

**Questions for you**:

1. **Proceed with phased implementation?**
   - Start with Phase 1 (single encoder all-I)
   - Test each phase before proceeding

2. **Or want more research first?**
   - Study OpenH264 source code deeper
   - Find working AVC444 server example
   - Contact experts

3. **Or try simpler approach first?**
   - Alternative B (main P, aux I with single encoder)
   - Lower risk, partial solution

**My recommendation**: **Proceed with Phase 1** (single encoder all-I). It's low risk, validates the structural change, and sets foundation for Phase 2/3.

What's your preference?
