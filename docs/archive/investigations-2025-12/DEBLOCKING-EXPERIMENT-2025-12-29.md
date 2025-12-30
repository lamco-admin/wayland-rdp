# Deblocking Filter Disable Experiment - 2025-12-29

**Binary MD5**: `415e6511868b9923e0ff857a6b474bd0`
**Status**: CRITICAL EXPERIMENT - Pinpointing Root Cause
**Goal**: Determine if H.264 deblocking filter is causing P-frame lavender corruption

---

## What Changed

### Code Modification

**File**: `src/egfx/avc444_encoder.rs`

**Added**:
1. Import `SEncParamExt` from `openh264_sys2`
2. New function: `configure_auxiliary_deblocking()`
3. Called after creating auxiliary encoder

**The Critical Change**:
```rust
// Via OpenH264 raw FFI API:
params.iLoopFilterDisableIdc = 1;  // Disable deblocking COMPLETELY
```

**Applied To**: Auxiliary encoder ONLY
**Not Changed**: Main encoder (deblocking still enabled for luma)

---

## What This Tests

### Hypothesis

**H.264's deblocking filter corrupts chroma when encoded as luma.**

The auxiliary stream's Y plane contains chroma values (U444/V444 odd rows) but H.264 treats it as luma. The deblocking filter:
- Is tuned for luma statistics (edges, text, high-frequency content)
- Uses luma-optimized thresholds
- Smooths block boundaries inappropriately for chroma
- **Result**: Color corruption (lavender/brown artifacts)

### Test Setup

**Main Encoder**:
- Deblocking: ENABLED (default, appropriate for real luma)
- Frame type: P-frames
- Contains: Actual luma + subsampled chroma

**Auxiliary Encoder**:
- Deblocking: **DISABLED** (experiment)
- Frame type: P-frames
- Contains: Chroma-as-fake-luma

---

## Expected Outcomes

### Scenario A: Hypothesis Correct ✅

**Observations**:
- ✅ NO lavender/brown corruption in changed areas
- ✅ Scrolling text remains readable and correct colors
- ✅ Window movements smooth with no artifacts
- ✅ Right-click menus display correctly
- ⚠️ POSSIBLE slight blocking artifacts in auxiliary (chroma)
  - Might appear as very subtle "blockiness" in color gradients
  - Should be much less visible than lavender corruption

**Conclusion**: Deblocking filter IS the root cause
**Next Steps**: Tune deblocking (don't fully disable, just reduce strength)

---

### Scenario B: Hypothesis Wrong ❌

**Observations**:
- ❌ Lavender corruption STILL present
- Same artifacts as before

**Conclusion**: Deblocking is NOT the root cause
**Next Steps**: Investigate other hypotheses:
1. Quantization matrix issues
2. Motion compensation artifacts
3. Transform/DCT issues
4. Something else we haven't considered

---

### Scenario C: Partial Success ⚠️

**Observations**:
- ⚠️ Corruption REDUCED but not eliminated
- ⚠️ Different artifact pattern

**Conclusion**: Deblocking is PART of the problem
**Next Steps**: Combine deblocking disable with other fixes

---

## Test Instructions

### What to Do

1. **Connect via RDP** to the test server
2. **Perform same actions** as previous test:
   - Scroll terminal text (fast)
   - Move windows around
   - Right-click for menus
   - Type in terminal
   - General UI interaction

3. **Compare with previous test**:
   - Previous: Lavender corruption in all changed areas
   - This test: Corruption present/absent?

### What to Look For

**Primary**: **Lavender/brown corruption**
- Changed areas when scrolling
- Window edges during movement
- Menu backgrounds
- Text areas

**Secondary**: **Blocking artifacts** (from disabled deblocking)
- Subtle "grid" pattern in color gradients
- Slight blockiness in smooth color transitions
- Should be MUCH less visible than corruption

### Quality Assessment

Rate on scale of 1-5:
- **Corruption**: 1 (severe) to 5 (none)
- **Blocking**: 1 (severe) to 5 (none)
- **Overall**: 1 (unusable) to 5 (perfect)

---

## Technical Details

### What Deblocking Filter Does

H.264's in-loop deblocking filter runs on **reconstructed frames** (after decoding):

**Purpose**:
- Smooth block boundaries (8x8 or 4x4 blocks)
- Reduce "blocking" visual artifacts
- Improve subjective quality

**How It Works**:
1. Analyzes pixels on both sides of block boundary
2. Calculates boundary strength (BS) based on:
   - Coding mode (intra/inter)
   - Motion vectors
   - Coded block patterns
   - QP values
3. Applies smoothing filter if BS exceeds threshold
4. Uses Alpha/Beta thresholds tuned for luma statistics

**Why It Might Corrupt Chroma**:
- Thresholds designed for luma (brightness) statistics
- Chroma (color) has different frequency content
- Filter might smooth where it shouldn't
- Or not smooth where it should
- **Result**: Color shifts and artifacts

### Deblocking in P-Frames

**I-Frames**: Minimal deblocking (intra prediction, less aggressive)
**P-Frames**: Aggressive deblocking (inter prediction, motion compensation residuals)

This explains why:
- All-I frames work perfectly (minimal deblocking)
- P-frames show corruption (aggressive deblocking on chroma-as-luma)

---

## Log Analysis Checklist

After test, check logs for:

```bash
# Find latest log
ssh greg@192.168.10.205 "ls -lt ~/colorful-test-*.log | head -1"

# Copy locally
scp greg@192.168.10.205:~/colorful-test-TIMESTAMP.log ./deblocking-exp.log

# Check experiment messages
rg "EXPERIMENT.*Disabling deblocking" deblocking-exp.log
rg "Auxiliary encoder reconfigured" deblocking-exp.log

# Verify frame types still P-frames
rg "\[AVC444 Frame" deblocking-exp.log | head -20

# Check for any errors
rg "error|ERROR|failed" deblocking-exp.log -i | grep -v "error=false"
```

---

## Decision Tree

```
Test Result
    │
    ├─ NO Corruption, NO Blocking
    │   └─> ✅ SUCCESS! Deblocking was the issue
    │       └─> Solution: Keep deblocking disabled for aux
    │       └─> Document and close
    │
    ├─ NO Corruption, YES Blocking
    │   └─> ⚠️ SUCCESS with tradeoff
    │       └─> Next: Tune deblocking (offsets -6 to -3)
    │       └─> Goal: Find balance (no corruption, minimal blocking)
    │
    ├─ YES Corruption, NO Blocking
    │   └─> ❌ Deblocking NOT the issue
    │       └─> Next: Investigate quantization/motion compensation
    │       └─> Explore innovative solutions (precompensation)
    │
    └─ YES Corruption, YES Blocking
        └─> ❌ Deblocking NOT the issue
            └─> Revert experiment
            └─> Explore other hypotheses
```

---

## Why This Experiment is Critical

This single test will definitively answer:
- **Is deblocking the root cause?** (Yes/No)
- **Can we use P-frames in auxiliary?** (With tuning?)
- **Do we need more complex solutions?** (Precompensation, etc.)

Pinpointing the exact cause enables targeted, efficient solutions rather than workarounds.

---

## Ready for Test

**Binary deployed**: `greg@192.168.10.205:~/lamco-rdp-server`
**MD5 verified**: `415e6511868b9923e0ff857a6b474bd0`
**Experiment active**: Auxiliary deblocking filter DISABLED
**Main encoder**: Unchanged (deblocking enabled for luma)

Run `./run-server.sh` and test!
