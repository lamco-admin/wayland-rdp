# Test #1: Quantization Theory - High Quality Auxiliary Encoder

**Date**: 2025-12-29
**Binary MD5**: `4836d38a00ab88d7068942e3327f814f`
**Status**: Testing if over-quantization causes P-frame corruption

---

## Hypothesis

**Chroma values are over-quantized when encoded as luma, causing precision loss and color corruption.**

### The Theory

**Normal H.264 Quantization**:
- Luma QP: Determined by bitrate/quality settings
- Chroma QP: Offset from luma QP (typically -2 to -6)
- **Chroma gets LESS quantization** (higher quality) than luma

**Our AVC444 Auxiliary Stream**:
- Contains: Chroma values (U444/V444)
- Encoded as: Luma (Y plane)
- Quantization: **LUMA QP** (not chroma QP with offset)
- **Result**: Chroma gets MORE quantization than it should → precision loss → color shifts

### Why This Could Cause Lavender Corruption

**Quantization in P-Frames**:
1. Motion compensation creates prediction
2. Residual = Actual - Prediction
3. Residual transformed (DCT)
4. Residual quantized (QP applied) ← **Precision loss here**
5. Encoded and sent
6. Decoder: Dequantize + inverse DCT + add to prediction
7. **If QP too high**: Residual loses precision → reconstructed values wrong → color corruption

**For Chroma**:
- Small color differences (U/V values close together)
- Over-quantization loses fine distinctions
- Decoder reconstructs wrong colors → lavender/brown artifacts

---

## Test Configuration

### Main Encoder (Unchanged)
- Bitrate: 5000 kbps
- QP: Determined by bitrate/content (probably ~23-28)
- Encoding: Real luma + subsampled chroma

### Auxiliary Encoder (BOOSTED)
- Bitrate: **15000 kbps** (3x higher!)
- QP: Will be much lower (~15-20 estimated)
- Encoding: Chroma-as-luma

### Expected QP Impact

**QP and Quality Relationship**:
- QP 0-17: Very high quality (near lossless)
- QP 18-28: High quality (default range)
- QP 29-40: Medium quality
- QP 41-51: Low quality

**Normal config** (5000 kbps): QP ~25
**Test config** (15000 kbps): QP ~17 (estimated)

**QP 17 vs QP 25**:
- ~4x less quantization damage
- Much finer chroma precision
- If quantization was the issue, should be fixed or significantly improved

---

## Expected Outcomes

### Scenario A: Corruption ELIMINATED or SIGNIFICANTLY REDUCED ✅

**Observation**: No lavender, or much less lavender than before

**Conclusion**: Quantization IS the root cause (or a major contributing factor)

**Next Steps**:
1. Find optimal auxiliary bitrate (test 2x, 2.5x, etc.)
2. Make configurable (not hardcoded 3x)
3. Document optimal ratio for different resolutions
4. **Production solution found!**

---

### Scenario B: Corruption UNCHANGED ❌

**Observation**: Same extensive lavender corruption

**Conclusion**: Quantization is NOT the cause

**Next Steps**:
- Move to Test #2: Dual-stream P-frame coordination
- Investigate how main and auxiliary streams interact
- Research MS-RDPEGFX "corresponding luma subframe" requirement

---

### Scenario C: Corruption DIFFERENT PATTERN ⚠️

**Observation**: Different artifacts (less lavender, more of something else)

**Conclusion**: Quantization is PART of the problem

**Next Steps**:
- Keep higher bitrate
- Investigate additional factors
- Hybrid solution

---

## Bandwidth Impact

**Previous** (both streams 5000 kbps):
- Main P-frame: ~22KB
- Aux P-frame: ~23KB
- Total: ~45KB per frame
- At 30 FPS: ~1.4 MB/s

**This Test** (main 5000, aux 15000 kbps):
- Main P-frame: ~22KB (same)
- Aux P-frame: ~35KB (estimated, 1.5x size for 3x bitrate)
- Total: ~57KB per frame
- At 30 FPS: ~1.7 MB/s

**Cost**: +21% bandwidth for test
**Benefit**: If this fixes corruption, we found the solution

---

## Technical Background

### Quantization Parameter (QP) in H.264

**What QP Controls**:
- DCT coefficient quantization step size
- Higher QP = larger steps = more precision loss
- Lower QP = smaller steps = better quality

**Bitrate → QP Relationship**:
- Encoder adjusts QP to hit target bitrate
- More bitrate budget → can use lower QP
- Less bitrate budget → must use higher QP

**Chroma Sensitivity**:
- Human vision more sensitive to luma than chroma
- Standard H.264: Chroma QP offset compensates for this
- Our auxiliary: No offset applied (treating as luma)

### Why 3x Bitrate?

**Conservative estimate** for significant QP reduction:
- 2x bitrate → ~6 QP units lower
- 3x bitrate → ~8-10 QP units lower
- Should be enough to see clear difference if QP is the issue

---

## After Test

**Report**:
1. Corruption level: None / Reduced / Same / Worse
2. Visual quality: Any other artifacts?
3. Performance: Lag or stuttering?

Then I'll analyze the log and we'll proceed to next test if needed.

---

## Why This Test Comes First

**Easiest to implement**: 1-line change
**Quick to test**: Same test procedure
**High impact if true**: Simple production fix
**Low risk**: Just uses more bandwidth temporarily

If this doesn't work, we'll systematically test each remaining hypothesis.
