# AVC444 Corruption Diagnostic Plan

**Date**: 2025-12-27  
**Status**: Deep Diagnosis Phase  
**Binary MD5**: `936ed57c9453676c4c52c3df5435085a`  

---

## Critical Finding: AVC420 Works, AVC444 Corrupts

### Test Results

| Mode | Result | Conclusion |
|------|--------|------------|
| AVC444 + all-keyframes | ✅ No corruption | P-frame issue confirmed |
| AVC444 + row-level fix | ❌ Still corrupts | Row-level fix insufficient |
| AVC420 only | ✅ Works well | Isolates to AVC444 code |

**This proves**:
- ✅ Color conversion (BGRA→YUV) is CORRECT (AVC420 uses it)
- ✅ OpenH264 encoder is CORRECT  
- ❌ Problem is IN AVC444-specific packing code

---

## Code Path Comparison

### AVC420 (Working)
```
BGRA → OpenH264.encode(BGRA) → H.264 stream
        ↑
        OpenH264 does color conversion internally
```

### AVC444 (Corrupted)
```
BGRA → bgra_to_yuv444() → YUV444
                 ↓
          pack_main_view() → YUV420 (stream 1)
                 ↓
       pack_auxiliary_view() → YUV420 (stream 2)
                 ↓
       OpenH264.encode(YUV420) × 2 → Two H.264 streams
```

**Suspects**:
1. `pack_main_view()` - Main view chroma subsampling
2. `pack_auxiliary_view()` - Auxiliary Y/U/V packing
3. Dual encoder coordination

---

## Diagnostic Logging Added

### Current Build (MD5: 936ed57c9453676c4c52c3df5435085a)

**Added TRACE logging to**:
1. `bgra_to_yuv444()` - First pixel conversion values
2. `pack_main_view()` - Main view chroma samples
3. `pack_auxiliary_view()` - Auxiliary Y/U/V samples with source comparison

**To see logs**: Run server with `RUST_LOG=trace` or check for 🔍 markers in log

---

## Analysis Questions

### Q1: Is YUV444 conversion correct?
**Check**: Do BGRA→YUV444 values look reasonable?
- Y should be in range [16-235] for limited, [0-255] for full
- U/V should be near 128 for gray colors
- Compare against OpenH264's internal conversion (AVC420 path)

### Q2: Is main view subsampling correct?
**Check**: Does 2×2 box filter produce expected values?
- Main U[0] should be average of U444[0][0], U444[0][1], U444[1][0], U444[1][1]
- Verify with manual calculation from logged values

### Q3: Is auxiliary Y packing correct?
**Check**: Does aux_Y[0] match U444[1]?
- aux_Y row 0 should equal U444 row 1 exactly
- Compare logged values directly
- If mismatch → row mapping bug

### Q4: Is auxiliary U/V packing correct?
**Check**: Does aux_U[0] match U444[0][1]?
- aux_U should sample from odd columns, even rows
- Verify with logged values

---

## Test Scenarios

### Test 1: Diagnostic Build with TRACE Logging

**Run**:
```bash
# On VM
RUST_LOG=lamco_rdp_server=trace ~/run-server.sh
```

**Analyze**:
- Look for 🔍 markers in log
- Compare source values to packed values
- Verify row mappings are correct

### Test 2: Swap U/V in Main View

**Purpose**: Test if main view chroma is swapped

**Change**: In `pack_main_view()`:
```rust
let u = subsample_chroma_420(&yuv444.v, width, height);  // SWAP
let v = subsample_chroma_420(&yuv444.u, width, height);  // SWAP
```

**If this fixes it**: Main view U/V are backwards

### Test 3: Disable Auxiliary Stream Entirely

**Purpose**: Test if auxiliary stream is the problem

**Change**: Return dummy auxiliary view with all 128s

**If this fixes it**: Problem is in auxiliary view packing

### Test 4: Simplified Auxiliary Y (No Row-Level)

**Purpose**: Test simpler packing

**Change**: Use constant 128 at all auxiliary Y positions (just send main view data)

**If this fixes it**: Row-level packing has bug

---

## Hypotheses to Test

### Hypothesis 1: Main View Chroma is Swapped
**Symptom**: Colors wrong in both I-frames and P-frames
**Test**: Swap U/V in pack_main_view()
**If correct**: Would see consistent color shift (not lavender specifically)

### Hypothesis 2: Auxiliary Y Row Mapping is Off-By-One
**Symptom**: Systematic color shift
**Test**: Check if aux_Y[0] actually equals U444[1] in logs
**If wrong**: Adjust row calculation formula

### Hypothesis 3: Auxiliary U/V Sampling is Wrong  
**Symptom**: Chroma artifacts in specific areas
**Test**: Verify aux_U/V sample positions in logs
**If wrong**: Fix sampling positions

### Hypothesis 4: Dual Encoder Synchronization Issue
**Symptom**: Frames don't align properly
**Test**: Check if both encoders produce same frame types (I vs P)
**Likelihood**: Low (both use same OpenH264 config)

---

## Data Collection

### From Next Test Run

**Capture**:
1. First 30 seconds of TRACE log (contains initial frames)
2. Frame encoding times
3. Frame sizes (main vs aux)
4. Any ERROR or WARN messages

**Extract diagnostic lines**:
```bash
grep "🔍" logfile.log > diagnostic_data.txt
```

---

## Mathematical Verification

### Expected Values for Test

For a pixel at (x=0, y=0) with BGRA = [B, G, R, 255]:

**YUV444 (OpenH264 limited range)**:
```
Y  = (66*R + 129*G + 25*B) / 256 + 16
U  = (-38*R - 74*G + 112*B) / 256 + 128  
V  = (112*R - 94*G - 18*B) / 256 + 128
```

**Main view U**: Average of U444[0][0], U444[0][1], U444[1][0], U444[1][1]

**Auxiliary Y row 0**: Should equal U444 row 1 exactly

**Auxiliary U[0]**: Should equal U444[0][1]

---

## Next Steps After Data Collection

1. **Verify row mappings** from logs
2. **Check U/V ordering** (might be swapped somewhere)
3. **Test simplified variants** to isolate
4. **Consider**: Maybe client expects different data than we think

---

*Diagnostic build deployed: 2025-12-27 20:XX*  
*Run with RUST_LOG=trace for full data*
