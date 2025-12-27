# Deep Analysis - Evening Session 2025-12-27

**Session**: Late evening after user rest  
**Status**: Systematic diagnosis in progress  
**Key Finding**: AVC420 works, AVC444 corrupts even after row-level fix  

---

## Critical Empirical Results

### Test Matrix

| Test | Corruption? | Performance | Diagnostic Value |
|------|-------------|-------------|------------------|
| AVC444 + all-keyframes | ❌ NO | Terrible | Proves P-frame involvement |
| AVC444 + row-level fix | ✅ YES | Good | Row-level fix insufficient |
| **AVC420 only** | **❌ NO** | **Good** | **ISOLATES TO AVC444** |
| AVC444 + TRACE logging | ✅ YES | Good | Shows neutral gray frames only |

### Definitive Conclusion

**AVC420 working proves**:
- ✅ `bgra_to_yuv444()` color conversion is CORRECT
- ✅ OpenH264 encoder is CORRECT  
- ✅ Core infrastructure is CORRECT

**AVC444 corrupting proves**:
- ❌ Bug is ONLY in AVC444 dual-stream code
- Suspects: `pack_main_view()`, `pack_auxiliary_view()`, or dual encoder coordination

---

## Diagnostic Logging Analysis

### Data Observed

**Session**: 24 seconds, 12 frames encoded
**Content**: Dark gray desktop (BGRA[19,19,19])
**YUV444**: Y=32, U=128, V=128 (neutral - correct for gray)
**Packing**: aux_Y[0] = [128,128,...] matches U444[1] = [128,128,...]

**Status**: ✅ Packing is mathematically correct for neutral gray content

### What We Didn't Capture

**Problem**: All logged data shows neutral gray (U=V=128)
**Lavender corruption** likely appears on:
- Colored UI elements
- Text with anti-aliasing
- Window borders
- Icons

**Limitation**: Diagnostic logging only samples first 8 pixels of first row
- Doesn't capture colored regions
- Doesn't show data during corruption

---

## Hypotheses Still Under Test

### Hypothesis 1: U/V Swap in Main View (B2/B3)
**Theory**: Main view might have U/V backwards
**Test**: Swap in `pack_main_view()` lines 232-236
**Status**: READY TO BUILD

### Hypothesis 2: U/V Swap in Auxiliary Y (B4/B5)
**Theory**: Packing V444 where should pack U444 and vice versa
**Test**: Swap in `pack_auxiliary_view()` lines 336-366
**Status**: READY TO BUILD

### Hypothesis 3: Auxiliary U/V (B6/B7) Wrong
**Theory**: aux_U/aux_V might be swapped or sampling wrong positions
**Test**: Check B6/B7 block implementation
**Status**: READY TO INVESTIGATE

### Hypothesis 4: Something Structural
**Theory**: Misunderstanding of spec or client expectations
**Test**: Compare byte-for-byte with working implementation
**Status**: REQUIRES MORE RESEARCH

---

## Next Diagnostic Steps

### Immediate (Next Test)

**Test Variant**: Swap U/V in BOTH main and auxiliary views

**Rationale**: If it's a consistent U/V swap throughout, this would fix it

**Change locations**:
1. `src/egfx/yuv444_packing.rs:232-236` (main view)
2. `src/egfx/yuv444_packing.rs:336-366` (auxiliary Y)
3. `src/egfx/yuv444_packing.rs:388-392` (auxiliary chroma)

### Alternative Approach

**Create visual color test**:
- Generate synthetic frame with known RGB values
- Red, green, blue blocks in known positions
- Log what YUV values they produce
- Verify end-to-end correctness

---

## Code Inspection Needed

### Check Main View Subsampling

**Function**: `subsample_chroma_420()` at `src/egfx/color_convert.rs:651`

**Algorithm**: 2×2 box filter
```rust
// For output position (x, y):
// Sample from input at (2x, 2y), (2x+1, 2y), (2x, 2y+1), (2x+1, 2y+1)
// Average with rounding
```

**Question**: Is this sampling the right positions?

### Check Auxiliary Chroma (B6/B7)

**Current implementation**: Lines 381-394
```rust
for cy in 0..chroma_height {
    let y = cy * 2;  // Even row: 0, 2, 4, ...
    for cx in 0..chroma_width {
        let x = cx * 2 + 1;  // Odd column: 1, 3, 5, ...
        aux_u.push(yuv444.u[y * width + x]);
        aux_v.push(yuv444.v[y * width + x]);
    }
}
```

**Question**: Should this be:
- Sampling from even rows? Or odd rows?
- Sampling from odd columns? Or even columns?
- U444 and V444? Or swapped?

---

## Observations from Log

### Build Info
- Binary timestamp shows old date (build timestamp macro not updating)
- But MD5 confirms correct binary is running
- Diagnostic logging IS present and working

### Frame Data
- Only captured neutral gray content
- All U/V values = 128 (expected for gray)
- No colored content in first pixel samples

### Performance
- 27-37ms per frame (reasonable for AVC444)
- 12 frames in 24 seconds (~0.5fps throughput)
- High backpressure (initial connection issues)

---

## Proposed Test Plan for Tomorrow

### Test 1: Complete U/V Swap Test
Build with ALL U/V swaps:
```
Main view: swap U/V
Auxiliary Y: swap U444/V444 source
Auxiliary chroma: swap U/V
```
**If this fixes it**: We have U/V backwards throughout

### Test 2: Dump Raw YUV444 to File
Add code to save first encoded frame as raw YUV444
- Inspect with external viewer (YUView, ffplay)
- Verify color conversion is actually correct

### Test 3: Compare Bitstreams
- Capture AVC420 H.264 stream
- Capture AVC444 main view H.264 stream
- Compare with hex viewer
- Look for differences in SPS/PPS/VUI

### Test 4: Minimal Auxiliary
Create absolutely minimal auxiliary view:
- aux_Y: all 128
- aux_U: all 128
- aux_V: all 128
**If clean**: Confirms auxiliary has bug
**Visual result**: Should look like AVC420

---

## Questions to Answer

1. **Why did all-keyframes work?** If packing is wrong, shouldn't all-keyframes also corrupt?
2. **Why is corruption "lavender" specifically?** What does that tell us about U/V values?
3. **Where is colored content?** Why don't we see non-neutral chroma in logs?
4. **Is main view identical to AVC420?** If so, problem must be in auxiliary only

---

## Status for Continuation

**Deployed**: Diagnostic build with extensive logging (MD5: 936ed57c9453676c4c52c3df5435085a)

**Ready to build**:
- Complete U/V swap test
- Neutral auxiliary test
- Additional logging variants

**Awaiting**: Decision on next test variant to deploy

---

*Analysis completed: 2025-12-27 ~20:45*  
*Ready for next diagnostic when user is rested*
