# Single Encoder Architecture - REVISED Plan (Based on Expert Analysis)

**Date**: 2025-12-29 02:00 UTC
**Status**: Plan revision based on correct understanding
**Credit**: Incorporating insights from experienced session

---

## KEY INSIGHT: LTR is NOT the Solution

### Why My LTR Approach Was Wrong

**LTR Purpose** (actual):
- Error recovery from packet loss
- Feedback-driven mechanism (LTR_RECOVERY_REQUEST, LTR_MARKING_FEEDBACK)
- Designed for resilience, not deterministic reference control

**My Misunderstanding**:
- Thought LTR could be used as "always reference slot X"
- Not how the API is designed
- Would be fighting the intended usage

**Correct Insight from Other Session**:
> "LTR may end up being useful as a robustness feature (recover cleanly after loss), but I wouldn't make it your primary mechanism for 'main refs main; aux refs aux.'"

---

## THE CORRECT APPROACH: Natural Multi-Reference

### The Key Realization

**With encoding order**: Main(t), Aux(t), Main(t+1), Aux(t+1), ...

**And DPB containing ≥2 frames**:
- Main(t+1) wants to reference frame 2 positions back = Main(t) ✓
- Aux(t+1) wants to reference frame 2 positions back = Aux(t) ✓

**OpenH264's motion search will naturally prefer**:
- Main refs Main (similar content - actual luma)
- Aux refs Aux (similar content - chroma patterns)

**We don't need to control it - just let it happen!**

---

## REVISED PHASED PLAN

### Phase 1: Single Encoder All-I (Validation) - 2 hours

**Goal**: Prove single encoder structure works, no regression

**Changes**:
```rust
// BEFORE:
pub struct Avc444Encoder {
    main_encoder: Encoder,
    aux_encoder: Encoder,
    // dual SPS/PPS caches
}

// AFTER:
pub struct Avc444Encoder {
    encoder: Encoder,  // ONE encoder
    cached_sps_pps: Option<Vec<u8>>,  // ONE cache
}
```

**Encoding**:
```rust
fn encode_bgra(...) -> Result<Avc444Frame> {
    let (main, aux) = pack_dual_views(&yuv444);

    // Force all-I (same as current workaround)
    self.encoder.force_intra_frame();
    let main_bs = self.encoder.encode(&main)?;

    self.encoder.force_intra_frame();
    let aux_bs = self.encoder.encode(&aux)?;

    // Package as AVC444Frame
}
```

**Success Criteria**:
- ✅ Compiles and runs
- ✅ Same perfect quality as current
- ✅ No crashes or errors
- ✅ Bandwidth similar (~4.3 MB/s)

**If successful**: Structure is sound, proceed to Phase 2
**If fails**: Debug before continuing

---

### Phase 2: Enable P-Frames with Multi-Ref + Instrumentation - 3 hours

**Goal**: Test if natural reference selection "just works"

#### Step 2.1: Configure Multi-Reference

```rust
pub fn new(config: EncoderConfig) -> Result<Self> {
    let mut encoder = Encoder::with_api_config(api, encoder_config)?;

    // CRITICAL: Enable multiple reference frames
    unsafe {
        Self::set_num_ref_frames(&mut encoder, 2)?;  // Keep last 2 frames
        // Or try 4 for more safety margin
    }

    Ok(Self { encoder, ... })
}

unsafe fn set_num_ref_frames(encoder: &mut Encoder, num: i32) -> Result<()> {
    let raw_api = encoder.raw_api();

    // ENCODER_OPTION_NUMBER_REF = 13 (from codec_app_def.h)
    const ENCODER_OPTION_NUMBER_REF: i32 = 13;

    let result = raw_api.set_option(
        ENCODER_OPTION_NUMBER_REF,
        &num as *const i32 as *mut c_void
    );

    if result != 0 {
        return Err(...);
    }

    debug!("✅ Configured encoder to keep {} reference frames in DPB", num);
    Ok(())
}
```

#### Step 2.2: Enable P-Frames

```rust
fn encode_bgra(...) -> Result<Avc444Frame> {
    let (main, aux) = pack_dual_views(&yuv444);

    // NO force_intra - let encoder use P-frames
    let main_bs = self.encoder.encode(&main)?;
    let aux_bs = self.encoder.encode(&aux)?;

    // Package as AVC444Frame
}
```

#### Step 2.3: Add NAL Instrumentation (CRITICAL!)

**From other session**:
> "Add debug instrumentation that parses each encoded access unit and logs: frame_num/POC, for P-slices: which reference indices/POCs are used in list0, whether a picture is marked as reference"

```rust
fn analyze_nal_structure(bitstream: &[u8], label: &str) {
    // Parse NAL units
    let nals = parse_annex_b(bitstream);

    for nal in nals {
        let nal_type = nal.header & 0x1F;

        match nal_type {
            1 => {  // P-slice
                let slice_header = parse_slice_header(&nal.data);
                debug!("  [{}] P-slice: frame_num={}, POC={:?}, ref_list[0]={:?}",
                       label,
                       slice_header.frame_num,
                       slice_header.poc,
                       slice_header.ref_pic_list0);
            },
            5 => {  // IDR
                debug!("  [{}] IDR slice", label);
            },
            // ...
        }

        // Check nal_ref_idc
        let nal_ref_idc = (nal.header >> 5) & 0x03;
        debug!("  [{}] nal_ref_idc = {} ({})",
               label,
               nal_ref_idc,
               if nal_ref_idc > 0 { "REFERENCE" } else { "NON-REFERENCE" });
    }
}

fn encode_bgra(...) {
    let main_bs = self.encoder.encode(&main)?;
    analyze_nal_structure(&main_bs.to_vec(), "MAIN");

    let aux_bs = self.encoder.encode(&aux)?;
    analyze_nal_structure(&aux_bs.to_vec(), "AUX");
}
```

**This logging will show**:
- Is Main(t+1) referencing Main(t) or Aux(t)?
- Is Aux marked as reference or non-reference?
- Are frame_num/POC values sane?

**Success Criteria**:
- ✅ Main refs Main (frame 2 positions back)
- ✅ Aux refs Aux (frame 2 positions back)
- ✅ No corruption

**Failure Modes**:
- ❌ Main refs Aux sometimes → Corruption
- ❌ Wrong reference pattern → Different corruption

**This phase is DIAGNOSTIC** - tells us exactly what's happening

---

### Phase 3: Fix Reference Issues (If Phase 2 Shows Problems) - 2-4 hours

**Only if Phase 2 logs show wrong references**

#### Option 3A: Make Aux Non-Reference (Preferred)

**From other session**:
> "Prefer 'Aux is non-reference' if you can do it inside the encoder"

**How**: Need to find if OpenH264 exposes this
- Might be in SEncParamExt somewhere
- Or might need per-frame hint
- Research needed

**Conceptually**:
- Aux frames provide chroma detail
- Don't need to be used for prediction
- Marking as non-reference prevents Main from ref'ing Aux

#### Option 3B: Increase Ref Frame Count

```rust
set_num_ref_frames(&mut encoder, 4)?;  // More DPB slots
```

**Theory**: More refs = better chance of finding correct match

#### Option 3C: Periodic IDR in Main Only

```rust
if self.frame_count % 30 == 0 {
    // Force IDR in main to "reset" and prevent drift
    self.encoder.force_intra_frame();
    let main_bs = self.encoder.encode(&main)?;

    // Aux still uses P-frame
    let aux_bs = self.encoder.encode(&aux)?;
}
```

**Theory**: Regular resets prevent accumulation of wrong references

---

### Phase 4: Production Fallback (If All Else Fails) - 1 hour

**Single encoder, Main P, Aux I**:

```rust
fn encode_bgra(...) {
    // Main: Normal P-frame
    let main_bs = self.encoder.encode(&main)?;

    // Aux: Force I-frame
    self.encoder.force_intra_frame();
    let aux_bs = self.encoder.encode(&aux)?;
}
```

**Bandwidth**: ~2.8 MB/s (better than full all-I's 4.3 MB/s)
**Quality**: Perfect (aux I-frames work)
**Complexity**: Low

**From other session**: "Most robust production fallback"

---

## CRITICAL: NAL INSTRUMENTATION

### Why This is Essential

**From other session**:
> "For 'best/robust,' don't fly blind. Add debug instrumentation that tells you what the encoder is actually doing"

**Without this**:
- We're guessing if references are correct
- Can't diagnose subtle issues
- Flying blind

**With this**:
- See exactly which frames reference which
- Validate assumptions
- Quick diagnosis if something's wrong

### NAL Parser Implementation

```rust
// Parse Annex B stream into NAL units
fn parse_annex_b(data: &[u8]) -> Vec<NalUnit> {
    let mut nals = Vec::new();
    let mut i = 0;

    while i < data.len() {
        // Find start code (0x000001 or 0x00000001)
        let start_code_len = if i + 4 <= data.len() && &data[i..i+4] == &[0,0,0,1] {
            4
        } else if i + 3 <= data.len() && &data[i..i+3] == &[0,0,1] {
            3
        } else {
            i += 1;
            continue;
        };

        i += start_code_len;

        // Find next start code (end of this NAL)
        let mut end = i;
        while end < data.len() {
            if end + 3 <= data.len() && &data[end..end+3] == &[0,0,1] {
                break;
            }
            if end + 4 <= data.len() && &data[end..end+4] == &[0,0,0,1] {
                break;
            }
            end += 1;
        }

        if end > i {
            nals.push(NalUnit {
                header: data[i],
                data: &data[i..end],
            });
        }

        i = end;
    }

    nals
}

struct NalUnit<'a> {
    header: u8,
    data: &'a [u8],
}

impl NalUnit<'_> {
    fn nal_type(&self) -> u8 {
        self.header & 0x1F
    }

    fn nal_ref_idc(&self) -> u8 {
        (self.header >> 5) & 0x03
    }

    fn is_reference(&self) -> bool {
        self.nal_ref_idc > 0
    }
}

// Simplified slice header parsing (complex in reality)
fn parse_slice_header_simple(nal_data: &[u8]) -> SliceInfo {
    // This is complex - need to parse RBSP, exp-golomb codes, etc.
    // For now, extract what we can easily
    SliceInfo {
        // ... would need proper H.264 parser
    }
}
```

**For Phase 2**: Even simple logging helps:
```rust
fn log_nal_types(bitstream: &[u8], label: &str) {
    let nals = parse_annex_b(bitstream);

    for nal in nals {
        debug!("  [{}] NAL type={}, ref_idc={} ({})",
               label,
               nal.nal_type(),
               nal.nal_ref_idc(),
               if nal.is_reference() { "REF" } else { "NON-REF" });
    }
}
```

This alone tells us if Aux is being marked as reference or not!

---

## REVISED PHASED IMPLEMENTATION

### Phase 1: Single Encoder Structure (All-I) - 2 hours

**Goal**: Structural refactor without behavior change

**Implementation**:
1. Change Avc444Encoder to use one Encoder
2. Merge SPS/PPS caching (one cache not two)
3. Keep all-I workaround active
4. Test for no regression

**Deliverable**: Working single encoder all-I (same quality as current)

---

### Phase 2: Multi-Ref P-Frames with Instrumentation - 3 hours

**Goal**: Enable P-frames with proper reference count + see what happens

**Implementation**:

**2.1: Set NUM_REF to ≥2**
```rust
unsafe fn configure_multi_ref(encoder: &mut Encoder, num_refs: i32) -> Result<()> {
    const ENCODER_OPTION_NUMBER_REF: i32 = 13;
    raw_api.set_option(ENCODER_OPTION_NUMBER_REF, &num_refs as *const _ as *mut c_void)?;
    debug!("✅ Encoder configured to keep {} reference frames", num_refs);
    Ok(())
}
```

**2.2: Add NAL logging**
```rust
fn log_reference_structure(bitstream: &[u8], label: &str, frame_num: u64) {
    let nals = parse_annex_b(bitstream);

    for nal in nals {
        let nal_type = nal.nal_type();
        let is_ref = nal.is_reference();

        debug!("[Frame #{} {}] NAL type={} ({}), nal_ref_idc={} ({})",
               frame_num,
               label,
               nal_type,
               nal_type_name(nal_type),
               nal.nal_ref_idc(),
               if is_ref { "REFERENCE" } else { "NON-REF" });
    }
}
```

**2.3: Enable P-frames**
```rust
fn encode_bgra(...) {
    let (main, aux) = pack_dual_views(&yuv444);

    // No force_intra - let encoder decide
    let main_bs = self.encoder.encode(&main)?;
    log_reference_structure(&main_bs.to_vec(), "MAIN", self.frame_count);

    let aux_bs = self.encoder.encode(&aux)?;
    log_reference_structure(&aux_bs.to_vec(), "AUX", self.frame_count);

    self.frame_count += 1;
}
```

**Test and Observe**:
- Is there corruption?
- What do logs show about reference usage?
- Is Aux marked as reference or non-reference?

**Possible Outcomes**:

**A) No corruption + logs show sane refs** ✅
- **Solution found!** Just needed single encoder + multi-ref
- Proceed to optimization

**B) Still corruption + logs show Main refs Aux sometimes** ⚠️
- Need Phase 3 (make Aux non-reference)

**C) Still corruption + logs show correct refs** ❓
- Something else wrong, deeper investigation

---

### Phase 3: Make Aux Non-Reference (If Needed) - 2-4 hours

**Goal**: Prevent Aux frames from polluting Main's reference chain

**From other session**:
> "If OpenH264 doesn't give you a supported, encoder-internal way to mark 'this picture is disposable / non-reference,' then do not do a 'post-encode nal_ref_idc = 0' hack by itself"

**Research Needed**:
- Is there an encoder option to mark frames non-reference?
- Can we do this per-frame at encode time?
- Or must we use SEncParamExt settings?

**Approach TBD** based on OpenH264 capabilities

---

### Phase 4: Optimization (After It Works) - 2-3 hours

**Once corruption eliminated**:
- Fine-tune num_ref_frames (2 vs 4)
- Optimize bitrate allocation
- Performance benchmarking
- Remove all-I fallback code

---

## IMPLEMENTATION DETAILS

### Single Encoder Structure

```rust
pub struct Avc444Encoder {
    /// Single encoder for both main and auxiliary subframes
    encoder: Encoder,

    /// Configuration
    config: EncoderConfig,

    /// Color matrix for BGRA → YUV444
    color_matrix: ColorMatrix,

    /// Frame counter (counts logical frames, not subframes)
    frame_count: u64,

    /// Total bytes encoded (both subframes)
    bytes_encoded: u64,

    /// Cumulative encoding time
    total_encode_time_ms: f64,

    /// Unified SPS/PPS cache (shared by both subframes)
    cached_sps_pps: Option<Vec<u8>>,

    /// Current H.264 level
    current_level: Option<super::h264_level::H264Level>,

    /// Number of reference frames configured
    num_ref_frames: i32,
}
```

### Key Architectural Changes

**Removed**:
- ❌ `main_encoder: Encoder`
- ❌ `aux_encoder: Encoder`
- ❌ `main_cached_sps_pps: Option<Vec<u8>>`
- ❌ `aux_cached_sps_pps: Option<Vec<u8>>`
- ❌ `handle_sps_pps_main()` function
- ❌ `handle_sps_pps_aux()` function

**Added**:
- ✅ `num_ref_frames: i32` (tracking)
- ✅ `configure_multi_ref()` function
- ✅ `log_reference_structure()` function (instrumentation)
- ✅ `parse_annex_b()` helper (NAL parsing)

**Modified**:
- ⚠️ `encode_bgra()` - sequential encoding through one encoder
- ⚠️ `handle_sps_pps()` - unified version
- ⚠️ SPS/PPS extraction - once per logical frame, not per subframe

---

## CRITICAL SUCCESS FACTORS

### Must Verify in Phase 2

1. **Reference Pattern**: Main(t+1) refs Main(t), not Aux(t)
2. **Aux Marking**: Is Aux marked as reference? (Should it be?)
3. **DPB Size**: Are 2 refs enough or need 4?
4. **Frame Numbering**: Is frame_num incrementing correctly?

### NAL Logging Must Show

```
[Frame #0 MAIN] IDR slice, nal_ref_idc=3 (REFERENCE)
[Frame #0 AUX] IDR slice, nal_ref_idc=3 (REFERENCE)
[Frame #1 MAIN] P-slice: frame_num=1, refs frame_num=0 (should be previous MAIN)
[Frame #1 AUX] P-slice: frame_num=2, refs frame_num=1 (should be previous AUX)
```

Or possibly:
```
[Frame #1 AUX] IDR slice, nal_ref_idc=0 (NON-REFERENCE)  ← Aux might be non-ref
```

**The logs will tell us what's actually happening**

---

## ALTERNATIVE IF NATURAL REFS DON'T WORK

### Fallback: Single Encoder, Main P + Aux I

**From other session**: "Most robust production fallback"

```rust
fn encode_bgra(...) {
    let (main, aux) = pack_dual_views(&yuv444);

    // Main: P-frame
    let main_bs = self.encoder.encode(&main)?;

    // Aux: I-frame (never used for prediction)
    self.encoder.force_intra_frame();
    let aux_bs = self.encoder.encode(&aux)?;
}
```

**Benefits**:
- ✅ Single encoder (matches spec)
- ✅ No corruption (aux I-frames work)
- ✅ Better than full all-I (~2.8 MB/s vs 4.3 MB/s)
- ✅ Simple and robust

**Tradeoff**: Aux bandwidth 3x higher than ideal, but acceptable

---

## WHY THIS APPROACH IS SUPERIOR

### Compared to My LTR Plan

**LTR Plan Issues**:
- ❌ Misuses LTR API (designed for error recovery, not ref control)
- ❌ Complex and fighting the design
- ❌ Uncertain if it would even work
- ❌ Hard to debug

**Multi-Ref Plan Advantages**:
- ✅ Uses OpenH264 as designed (multi-ref is standard)
- ✅ Simpler (let motion search find best match)
- ✅ Easier to debug (NAL instrumentation shows what's happening)
- ✅ Natural reference selection
- ✅ Multiple fallback options if needed

### Key Insight from Other Session

**The problem isn't "how to control references"**

**The problem is "using one encoder with correct ref count"**

Motion search will naturally prefer:
- Main refs Main (luma patterns match)
- Aux refs Aux (chroma patterns match)

**We just need to let it work naturally!**

---

## ESTIMATED TIMELINE

### Optimistic (Natural refs work)
- Phase 1: 2 hours
- Phase 2: 3 hours (including NAL parsing)
- **Total: 5 hours** ✅

### Realistic (Need some tuning)
- Phase 1: 2 hours
- Phase 2: 3 hours
- Phase 3: 2 hours (adjust ref count or aux marking)
- **Total: 7 hours**

### Pessimistic (Need fallback)
- Phase 1: 2 hours
- Phase 2: 3 hours
- Phase 3: 3 hours (try various fixes)
- Phase 4: 1 hour (implement fallback)
- **Total: 9 hours**

**All cases lead to working solution**

---

## DECISION POINTS

### Decision 1: Start with Phase 1 Now?

**Yes** (Recommended):
- Low risk structural change
- Validates approach
- Foundation for Phase 2

**No**: More research first?

### Decision 2: How Much NAL Instrumentation?

**Minimal** (just NAL type and ref_idc):
- Quick to implement
- Tells us enough

**Full** (slice headers, POC, frame_num):
- More complex
- Complete picture
- Better for debugging

**Recommended**: Start minimal, add detail if needed

### Decision 3: Target for Phase 2

**Conservative**: num_ref_frames = 4 (more safety)
**Aggressive**: num_ref_frames = 2 (minimal)

**Recommended**: Start with 2, increase to 4 if issues

---

## READY FOR IMPLEMENTATION

**Current State**: ✅ Stable, clean, documented
**Plan**: ✅ Phased, low-risk, well-researched
**Fallbacks**: ✅ Multiple options if issues arise
**Instrumentation**: ✅ NAL logging for visibility

**Awaiting your approval to proceed with Phase 1.**

**Key principle from other session**:
> "Don't fly blind. Add instrumentation. Validate assumptions."

This plan embodies that principle.
