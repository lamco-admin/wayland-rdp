# Phase 2 PROPER - AVC444 with Multi-Ref DPB

**Date**: 2025-12-29 03:10 UTC
**Binary MD5**: `f592bd98e203527a03da776e5947ffb7`
**Status**: Phase 2 properly deployed with AVC444 (not AVC420 fallback)

---

## WHAT'S DIFFERENT FROM PREVIOUS PHASE 2 ATTEMPT

### Previous Attempt (Failed)

**Binary**: `17da98ec8c23fe05d38f377cbd4aee05`
**Issue**: Tried to configure NUM_REF via unsafe `set_option()` after creation
**Result**: Error code 4 → Fell back to AVC420
**Test**: AVC420 worked perfectly (but not what we wanted)

### This Deployment (Correct)

**Binary**: `f592bd98e203527a03da776e5947ffb7`
**Implementation**: Extended openh264-rs with `.num_ref_frames()` fluent API
**Configuration**: NUM_REF=2 set DURING encoder creation
**Result**: Should create AVC444 encoder successfully

---

## OPENH264-RS EXTENSION

### Changes Made to openh264-rs

**File**: `/home/greg/openh264-rs/openh264/src/encoder.rs`

**1. Added field to EncoderConfig**:
```rust
pub struct EncoderConfig {
    // ... existing fields ...
    num_ref_frames: Option<i32>,  // NEW
}
```

**2. Initialize in new()**:
```rust
impl EncoderConfig {
    pub const fn new() -> Self {
        // ...
        num_ref_frames: None,  // NEW
    }
}
```

**3. Added fluent API method**:
```rust
pub const fn num_ref_frames(mut self, num: i32) -> Self {
    let clamped = if num < 1 { 1 } else if num > 16 { 16 } else { num };
    self.num_ref_frames = Some(clamped);
    self
}
```

**4. Applied during initialization**:
```rust
// In with_api_config(), before InitializeExt():
if let Some(num) = self.config.num_ref_frames {
    params.iNumRefFrame = num;  // Applied to SEncParamExt
}
```

**Total Lines Added**: ~40 lines
**Pattern**: Identical to VUI support (same PR)
**Tested**: openh264-rs compiles cleanly

---

## USAGE IN AVC444 ENCODER

**Before** (unsafe set_option attempt):
```rust
let encoder = Encoder::with_api_config(api, config)?;
unsafe {
    configure_num_ref_frames(&mut encoder, 2)?;  // FAILED
}
```

**After** (clean fluent API):
```rust
let config = OpenH264Config::new()
    .bitrate(...)
    .num_ref_frames(2);  // ✅ Clean, configured at creation

let encoder = Encoder::with_api_config(api, config)?;
```

---

## WHAT THIS TEST WILL SHOW

### The Definitive AVC444 P-Frame Test

**Previous tests**:
- AVC420 P-frames: ✅ Perfect (but not full 4:4:4)
- AVC444 All-I: ✅ Perfect (but high bandwidth)
- AVC444 P-frames (dual encoder): ❌ Lavender corruption

**This test**:
- **AVC444** (full 4:4:4 chroma) ✓
- **P-frames** enabled ✓
- **Single encoder** (MS-RDPEGFX compliant) ✓
- **NUM_REF=2** (multi-reference DPB) ✓
- **NAL instrumentation** (reference tracking) ✓

**This is the complete, proper implementation!**

---

## EXPECTED OUTCOMES

### Success: No Corruption ✅

**Observation**: No lavender, perfect quality, P-frames compressing

**Means**:
- Single encoder + multi-ref DPB WAS the solution
- Natural reference selection works (Main refs Main, Aux refs Aux)
- **PROBLEM SOLVED!**

**Next**:
- Document solution
- Commit code
- Prepare PR amendment for openh264-rs
- Optimize if needed

---

### Partial: Reduced Corruption ⚠️

**Observation**: Less lavender than before, but still some

**Means**:
- Single encoder helps
- Might need NUM_REF=4 instead of 2
- Or need to make Aux non-reference

**Next**:
- Analyze NAL logs
- Try NUM_REF=4
- Implement Phase 3 if needed

---

### Failure: Same Corruption ❌

**Observation**: Extensive lavender still present

**Means**:
- Single encoder + multi-ref not sufficient
- Need deeper investigation

**Next**:
- Analyze NAL logs carefully
- Check reference behavior
- Implement aux non-reference strategy
- Or explore other solutions

---

## NAL INSTRUMENTATION

### What the Logs Will Show

```
[Frame #0 MAIN NAL#0] type= 7 (SPS) ref_idc=3 (REFERENCE(3))
[Frame #0 MAIN NAL#1] type= 8 (PPS) ref_idc=3 (REFERENCE(3))
[Frame #0 MAIN NAL#2] type= 5 (IDR) ref_idc=3 (REFERENCE(3))
[Frame #0 AUX NAL#0] type= 5 (IDR) ref_idc=3 (REFERENCE(3))
[Frame #1 MAIN NAL#0] type= 1 (P-slice) ref_idc=2 (REFERENCE(2))
[Frame #1 AUX NAL#0] type= 1 (P-slice) ref_idc=? (???)
```

**Key Questions**:
1. Is Aux marked as REFERENCE or NON-REF?
2. Are P-slices being used for both?
3. Frame sizes reasonable?

---

## TEST INSTRUCTIONS

**Run rigorous test**:
1. Connect via RDP
2. Scroll terminal text (fast)
3. Move windows around
4. Right-click menus
5. Type and interact

**Watch for**:
- Lavender/brown corruption
- Color accuracy
- Performance/lag
- Any artifacts

**After test, tell me**:
1. Corruption status: none / less / same
2. Performance: good / laggy / stuttering
3. Any other observations

**Then I'll grab log and analyze NAL structure exhaustively**

---

## PROGRESS SUMMARY

**Total time**: ~3 hours
**Phases complete**:
- ✅ Phase 1: Single encoder all-I (validated)
- 🚀 Phase 2: Single encoder P-frames with NUM_REF=2 (testing now)

**OpenH264-RS Extended**:
- ✅ VUI support (existing)
- ✅ NUM_REF support (just added)
- Ready to amend PR later

**System state**: Clean, well-documented, properly architected

**This is the moment of truth for AVC444 P-frames!**
