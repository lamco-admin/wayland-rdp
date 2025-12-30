# Status Update: Scene Change Detection Disabled

**Date**: 2025-12-29 03:35 UTC
**Binary MD5**: `b08a81e9baad23f13f899c670088eec4`
**Change**: Disabled scene change detection to allow Aux P-frames

---

## PROGRESS SUMMARY

### Investigation Journey

**Session Start** → **Root Cause** → **Phase 1** → **Phase 2 Attempts** → **Now**

**Hours Invested**: ~4 hours
**Tests Conducted**: 15+ systematic tests
**Ruled Out**: Deblocking, quantization, padding, stride, dual caches
**Identified**: Two-encoder architecture violates MS-RDPEGFX spec

---

## PHASE PROGRESSION

### Phase 1: ✅ COMPLETE (Validated)

**Change**: Two encoders → One encoder
**Behavior**: All-I frames (safe mode)
**Result**: Perfect quality, architecture validated
**Time**: 1.5 hours

### Phase 2a: ❌ FAILED (AVC420 Fallback)

**Attempt**: Configure NUM_REF via set_option() after creation
**Result**: Error code 4 → Fell back to AVC420
**Learning**: Can't configure after init, need at creation

### Phase 2b: ⚠️ PARTIAL (Main-P, Aux-IDR)

**Extended**: openh264-rs with .num_ref_frames() API
**Result**: AVC444 created successfully
**Finding**: Aux forced to IDR by scene change detection
**Corruption**: Same as dual-encoder (extensive lavender)
**Time**: 1 hour

### Phase 2c: 🚀 NOW DEPLOYED (Both P-Frames)

**Extended**: Using existing .scene_change_detect(false) API
**Configuration**:
```rust
OpenH264Config::new()
    .num_ref_frames(2)
    .scene_change_detect(false)  // NEW - Critical!
```

**Expected**: Aux will now use P-frames like Main
**Binary**: `b08a81e9baad23f13f899c670088eec4`

---

## WHAT THIS TEST WILL SHOW

### The Complete Configuration

✅ **Single encoder** (MS-RDPEGFX compliant)
✅ **NUM_REF = 2** (multi-reference DPB)
✅ **Scene change detection OFF** (allows Aux P-frames)
✅ **NAL instrumentation** (reference tracking)

**This is the FULL proper implementation of single encoder + P-frames**

### Expected NAL Pattern

```
Frame #0:  Main: IDR, Aux: IDR
Frame #1:  Main: P,   Aux: P    ← Both should be P now!
Frame #2:  Main: P,   Aux: P    ← Both using P-frames
```

**If we see this pattern**: Scene change detection was the blocker

---

## POSSIBLE OUTCOMES

### Outcome A: NO CORRUPTION ✅

**Means**:
- Scene change detection WAS blocking Aux P-frames
- With both using P-frames + single encoder + multi-ref → SOLVED!
- **PROBLEM COMPLETELY SOLVED!**

**Next**: Document, commit, optimize

---

### Outcome B: SAME CORRUPTION ❌

**Means**:
- Even with both using P-frames, still corrupts
- Single encoder + multi-ref not sufficient
- Need deeper solution

**Next**:
- Analyze NAL logs (which refs are being used?)
- Make Aux non-reference
- Or explore other approaches

---

### Outcome C: DIFFERENT CORRUPTION ⚠️

**Means**:
- Pattern changed
- Partial progress
- Need refinement

**Next**: Analyze logs, adjust approach

---

## CRITICAL OBSERVATION

**Previous test** (Main-P + Aux-IDR): Had corruption

**This is UNEXPECTED** because:
- Aux-IDR should be safe (no prediction, no ref issues)
- Only Main using P-frames
- Should be similar to hybrid workaround

**But had same extensive corruption!**

**This suggests**: The corruption might not be about Aux P-frames specifically, but about **HOW subframes are packaged/sent** or **client-side decoding expectations**.

**Or**: There's still something wrong with our single encoder implementation.

---

## WHAT TO WATCH FOR

### Primary: Corruption Pattern

- None → Solved!
- Same → Deeper issue
- Different → Progress, need tuning

### Secondary: NAL Logs

After test, I'll check:
```
rg "Frame #.*AUX.*P-slice" phase2c-test.log
```

**Should see**: Aux using P-slices now (not just IDR)

### Tertiary: Performance

- Bandwidth should be lowest yet (~1-2 MB/s)
- Both streams compressing with P-frames

---

## TEST NOW

Same rigorous test:
- Scroll terminal (watch for lavender)
- Move windows
- Right-click menus

**Report**:
1. Corruption: none / same / different
2. NAL logs will show: Are both using P-frames now?

**This is test #2c with BOTH subframes using P-frames via single encoder.**

Binary deployed and ready!
