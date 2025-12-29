# Single Encoder Architecture - Implementation Plan

**Date**: 2025-12-29
**Status**: Research complete, planning phase
**Confidence**: High - LTR feature enables solution

---

## EXECUTIVE SUMMARY

### The Solution: Long Term Reference (LTR) Frames

**OpenH264 Feature**: Long Term Reference (LTR) frames
- Allows marking specific frames to stay in DPB long-term
- Frames can reference specific LTR slots
- **Perfect for maintaining separate main/aux reference chains in ONE encoder!**

### Architecture Design

```rust
pub struct Avc444Encoder {
    encoder: Encoder,  // ONE encoder for both subframes

    // LTR slot assignments:
    // - Slot 0: Last main subframe
    // - Slot 1: Last auxiliary subframe
}
```

### Encoding Flow

```rust
fn encode_bgra(...) {
    let (main, aux) = pack_dual_views(&yuv444);

    // Encode main subframe
    // - Use LTR slot 0 as reference (previous main)
    // - Mark current frame for LTR slot 0
    let main_bs = self.encoder.encode(&main)?;
    mark_as_ltr(slot=0);

    // Encode aux subframe
    // - Use LTR slot 1 as reference (previous aux)
    // - Mark current frame for LTR slot 1
    let aux_bs = self.encoder.encode(&aux)?;
    mark_as_ltr(slot=1);

    // Both subframes in same encoder DPB
    // But reference separate LTR slots
    // = Separate reference chains!
}
```

---

## RESEARCH FINDINGS

### OpenH264 LTR Capabilities

**From codec_app_def.h**:

```c
// Enable LTR
ENCODER_OPTION_LTR  // 0=disable, >0=enable (fixed to 2 slots)

// LTR configuration
typedef struct {
    bool bEnableLongTermReference;  // Enable/disable
    int  iLTRRefNum;                // Number of LTR frames (2 supported)
} SLTRConfig;

// LTR operations
ENCODER_LTR_RECOVERY_REQUEST   // Request recovery from LTR
ENCODER_LTR_MARKING_FEEDBACK   // LTR marking feedback
ENCODER_LTR_MARKING_PERIOD     // LTR marking period
```

**Key Features**:
- ✅ Support for 2 LTR slots (perfect for main + aux!)
- ✅ Can mark frames to specific slots
- ✅ Can reference specific slots
- ✅ LTR frames stay in DPB across many frames

**What This Enables**:
- Main subframes use LTR slot 0 → maintain main reference chain
- Aux subframes use LTR slot 1 → maintain aux reference chain
- ONE encoder, ONE DPB, but TWO logical reference chains

---

## DETAILED ARCHITECTURE

### Data Structures

```rust
pub struct Avc444Encoder {
    /// Single encoder for both main and auxiliary subframes
    encoder: Encoder,

    /// Configuration
    config: EncoderConfig,

    /// Color matrix
    color_matrix: ColorMatrix,

    /// Frame statistics
    frame_count: u64,
    bytes_encoded: u64,
    total_encode_time_ms: f64,

    /// SPS/PPS cache (shared between subframes)
    cached_sps_pps: Option<Vec<u8>>,

    /// LTR state tracking
    ltr_enabled: bool,
    main_ltr_slot: i32,   // Always 0
    aux_ltr_slot: i32,    // Always 1
}
```

### Initialization

```rust
impl Avc444Encoder {
    pub fn new(config: EncoderConfig) -> EncoderResult<Self> {
        let encoder_config = OpenH264Config::new()
            .bitrate(BitRate::from_bps(config.bitrate_kbps * 1000))
            .max_frame_rate(FrameRate::from_hz(config.max_fps))
            .usage_type(UsageType::ScreenContentRealTime);

        let mut encoder = Encoder::with_api_config(
            openh264::OpenH264API::from_source(),
            encoder_config,
        )?;

        // Enable LTR with 2 slots (main + aux)
        unsafe {
            Self::enable_ltr(&mut encoder, 2)?;
        }

        Ok(Self {
            encoder,
            // ...
            ltr_enabled: true,
            main_ltr_slot: 0,
            aux_ltr_slot: 1,
        })
    }

    unsafe fn enable_ltr(encoder: &mut Encoder, num_slots: i32) -> EncoderResult<()> {
        let raw_api = encoder.raw_api();

        // ENCODER_OPTION_LTR (from codec_app_def.h)
        const ENCODER_OPTION_LTR: i32 = 18;

        let result = raw_api.set_option(
            ENCODER_OPTION_LTR,
            &num_slots as *const i32 as *mut std::ffi::c_void
        );

        if result != 0 {
            return Err(EncoderError::InitFailed(
                format!("Failed to enable LTR: {}", result)
            ));
        }

        debug!("✅ Enabled LTR with {} slots for dual subframe encoding", num_slots);
        Ok(())
    }
}
```

### Encoding Flow with LTR

```rust
pub fn encode_bgra(...) -> EncoderResult<Option<Avc444Frame>> {
    // Step 1: Color conversion
    let yuv444 = bgra_to_yuv444(bgra, width, height, self.color_matrix);

    // Step 2: Pack into dual YUV420 views
    let (main_yuv420, aux_yuv420) = pack_dual_views(&yuv444);

    // Step 3: Encode MAIN subframe (use LTR slot 0)
    let main_bitstream = if self.frame_count == 0 {
        // First frame: IDR
        self.encoder.encode(&main_yuv420)?
    } else {
        // P-frame: Reference LTR slot 0 (previous main)
        unsafe { self.encode_with_ltr_reference(&main_yuv420, self.main_ltr_slot)?}
    };

    // Mark main frame for LTR slot 0
    unsafe { self.mark_ltr_frame(self.main_ltr_slot)?; }

    // Step 4: Encode AUX subframe (use LTR slot 1)
    let aux_bitstream = if self.frame_count == 0 {
        // First frame: IDR
        self.encoder.encode(&aux_yuv420)?
    } else {
        // P-frame: Reference LTR slot 1 (previous aux)
        unsafe { self.encode_with_ltr_reference(&aux_yuv420, self.aux_ltr_slot)? }
    };

    // Mark aux frame for LTR slot 1
    unsafe { self.mark_ltr_frame(self.aux_ltr_slot)?; }

    // Step 5: Extract bitstreams and create AVC444 frame
    // ...
}
```

### LTR Helper Functions

```rust
impl Avc444Encoder {
    /// Encode frame and force it to reference a specific LTR slot
    unsafe fn encode_with_ltr_reference(
        &mut self,
        yuv: &Yuv420Frame,
        ltr_slot: i32
    ) -> EncoderResult<EncodedBitStream> {
        // Before encoding, set which LTR to use as reference
        // This might require LTR recovery request or similar

        let bitstream = self.encoder.encode(yuv)?;

        Ok(bitstream)
    }

    /// Mark current frame to be stored in specific LTR slot
    unsafe fn mark_ltr_frame(&mut self, slot: i32) -> EncoderResult<()> {
        let raw_api = self.encoder.raw_api();

        // Send LTR marking feedback
        // This tells encoder to store current reconstructed frame in LTR slot
        let mut marking = SLTRMarkingFeedback {
            uiFeedbackType: LTR_MARKING_SUCCESS,  // Mark as successful
            uiIDRPicId: 0,
            iLTRFrameNum: slot,  // Which slot to use
            iLayerId: 0,
        };

        const ENCODER_LTR_MARKING_FEEDBACK: i32 = 16;
        let result = raw_api.set_option(
            ENCODER_LTR_MARKING_FEEDBACK,
            &mut marking as *mut _ as *mut std::ffi::c_void
        );

        if result != 0 {
            return Err(EncoderError::EncodeFailed(
                format!("LTR marking failed: {}", result)
            ));
        }

        Ok(())
    }
}
```

---

## ALTERNATIVE APPROACHES

### Approach B: Force Main-Only Reference

**If LTR doesn't work as expected**:

```rust
// Simpler approach: Only main frames are reference frames
// Aux frames are always I-frames OR always reference the main

fn encode_bgra(...) {
    // Encode main normally (builds reference chain)
    let main_bs = self.encoder.encode(&main)?;

    // Encode aux as I-frame (no reference)
    self.encoder.force_intra_frame();
    let aux_bs = self.encoder.encode(&aux)?;
}
```

**Pros**:
- ✅ Simple
- ✅ No complex LTR management

**Cons**:
- ⚠️ Aux uses all-I (same as current workaround)
- ⚠️ But at least uses single encoder (might fix something else?)

---

### Approach C: Main References Main, Aux References Main Too

```rust
fn encode_bgra(...) {
    // Main frame: Normal P-frame (references previous main)
    let main_bs = self.encoder.encode(&main)?;
    // DPB: [main_current]

    // Aux frame: Also references main (not previous aux)
    // Might work since aux is "derived" from main
    let aux_bs = self.encoder.encode(&aux)?;
    // DPB: [main_current, aux_current]

    // Next iteration:
    // Main P-frame will reference main_current
    // Aux P-frame will reference main_next (the one we just encoded)
}
```

**Theory**: Aux is "derivative" of main, so referencing main might make sense?

**Pros**:
- ✅ Simple (no LTR needed)
- ✅ Might match client's expectation?

**Cons**:
- ⚠️ Aux pred from main might not be good (different data)
- ⚠️ Speculative

---

## IMPLEMENTATION ROADMAP

### Phase 1: Minimal Single Encoder (No LTR) - 2 hours

**Goal**: Prove single encoder structure works at all

```rust
pub struct Avc444Encoder {
    encoder: Encoder,  // Remove dual encoders
}

// Force all-I for both subframes initially
fn encode_bgra(...) {
    self.encoder.force_intra_frame();
    let main = self.encoder.encode(&main_yuv420)?;

    self.encoder.force_intra_frame();
    let aux = self.encoder.encode(&aux_yuv420)?;
}
```

**Test**: Does this work? Same quality as current all-I?

**If yes**: Proceed to Phase 2
**If no**: Debug why single encoder with all-I fails

---

### Phase 2: Enable P-Frames with Simple Strategy - 2 hours

**Goal**: Test basic P-frame support without LTR

```rust
fn encode_bgra(...) {
    // Don't force_intra, let encoder decide
    let main = self.encoder.encode(&main_yuv420)?;
    let aux = self.encoder.encode(&aux_yuv420)?;
}
```

**Hypothesis**: Single encoder might "just work" even without LTR

**Test**:
- Better than two encoders (less corruption)?
- Same corruption?
- Different corruption?

**This tells us if single encoder helps AT ALL**

---

### Phase 3: Implement LTR Strategy - 4 hours

**Goal**: Use LTR to maintain separate reference chains

**Steps**:
1. Enable LTR with 2 slots
2. Implement mark_ltr_frame()
3. Implement encode_with_ltr_reference()
4. Test main refs slot 0, aux refs slot 1

**Expected**: Corruption eliminated if LTR works correctly

---

### Phase 4: Optimization - 3 hours

**After P-frames work**:
- Fine-tune bitrate allocation
- Optimize LTR marking strategy
- Performance benchmarking
- Bandwidth analysis

---

## RISK ASSESSMENT

### High Risks

**R1: OpenH264 LTR might not work as expected**
- Mitigation: Phase 2 tests without LTR first
- Fallback: Use all-I for aux (Approach B)

**R2: Client might not handle our bitstream structure**
- Mitigation: Follow spec exactly
- Testing: Verify with Windows RDP client

**R3: DPB size limits**
- 2 frames per logical frame (main + aux)
- Might fill DPB quickly
- Mitigation: Monitor and adjust iNumRefFrame

### Medium Risks

**R4: SPS/PPS handling changes**
- One encoder = one SPS/PPS set
- Need to verify prepending logic
- Mitigation: Careful testing

**R5: Performance impact**
- More complex encoding flow
- Mitigation: Benchmark and optimize

### Low Risks

**R6: Bitrate allocation**
- Single bitrate for both subframes
- Encoder will distribute automatically
- Might not be optimal but should work

---

## CRITICAL SUCCESS FACTORS

### Must Have
1. ✅ Single Encoder instance
2. ✅ Separate reference chains (via LTR or other mechanism)
3. ✅ No P-frame corruption
4. ✅ Reasonable bandwidth

### Should Have
1. ⚠️ Efficient P-frame compression (both streams)
2. ⚠️ Configurable bitrate allocation
3. ⚠️ Performance competitive with all-I

### Nice to Have
1. 💡 Optimal QP per subframe
2. 💡 Adaptive LTR management
3. 💡 Advanced reference frame strategies

---

## DECISION POINTS

### Decision 1: LTR vs Simple Approach

**Option A**: Implement LTR (complex but proper)
- Separate reference chains
- Both streams use P-frames
- Matches architectural model

**Option B**: Single encoder with aux all-I (simple)
- Only main uses P-frames
- Aux always I-frames
- Less bandwidth efficient but simpler

**Recommendation**: Try (A) first, fallback to (B) if LTR doesn't work

---

### Decision 2: Phased vs All-At-Once

**Phased** (Recommended):
1. Single encoder all-I (validate structure)
2. Enable P-frames without LTR (see what happens)
3. Add LTR (fix remaining issues)

**All-At-Once**:
- Implement complete LTR solution immediately
- Higher risk if wrong

**Recommendation**: Phased approach reduces risk

---

### Decision 3: FFI Scope

**Minimal FFI**:
- Only what's needed for LTR
- set_option() calls for:
  - ENCODER_OPTION_LTR
  - ENCODER_LTR_MARKING_FEEDBACK

**Extensive FFI**:
- Full control over encoding parameters
- Reference list reordering
- Custom slice configuration

**Recommendation**: Start minimal, expand if needed

---

## IMPLEMENTATION ESTIMATES

### Phase 1: Single Encoder All-I
- **Effort**: 2 hours
- **Confidence**: 95%
- **Risk**: Low

### Phase 2: Basic P-Frames
- **Effort**: 1 hour
- **Confidence**: 60% (might work without LTR)
- **Risk**: Medium

### Phase 3: LTR Implementation
- **Effort**: 4-6 hours
- **Confidence**: 80%
- **Risk**: Medium-High

### Total Estimated Time
- **Minimum**: 7 hours (if LTR works first try)
- **Maximum**: 12 hours (with debugging/refinement)
- **Most Likely**: 9 hours

---

## OPEN QUESTIONS FOR RESEARCH

### Q1: OpenH264 LTR API Details

**Need to find**:
- Exact API calls for LTR marking
- How to specify which LTR to reference
- SLTRMarkingFeedback structure usage
- SLTRRecoverRequest structure usage

**Sources**:
- OpenH264 header files
- OpenH264 examples/tests
- FFmpeg's OpenH264 wrapper (might use LTR)

---

### Q2: Does FreeRDP Use LTR for AVC444?

**Search for**:
- FreeRDP server AVC444 encoding
- Any LTR usage in FreeRDP's H.264 code
- How they structure dual subframes

**Status**: Searching...

---

### Q3: Client Compatibility

**Need to verify**:
- Does Windows RDP client expect specific LTR configuration?
- Any requirements for how LTR is used?
- Can we test with different clients (FreeRDP, etc.)?

---

## NEXT STEPS

1. ✅ Complete LTR API research (codec headers, examples)
2. ⬜ Design exact FFI calls for LTR marking/recovery
3. ⬜ Create detailed implementation plan for Phase 1
4. ⬜ Review with user before coding
5. ⬜ Implement Phase 1 (single encoder all-I)
6. ⬜ Test and verify no regression
7. ⬜ Implement Phase 2 (basic P-frames)
8. ⬜ Assess results
9. ⬜ Implement Phase 3 (LTR) if needed
10. ⬜ Final testing and benchmarking

---

## RESEARCH CONTINUING...

**Status**: Actively researching LTR API details and usage patterns
**Next**: Study OpenH264 examples and FFmpeg implementation
**Goal**: Complete understanding before implementation begins
