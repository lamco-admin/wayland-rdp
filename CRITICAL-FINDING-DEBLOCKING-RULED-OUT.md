# CRITICAL FINDING: Deblocking Filter is NOT the Root Cause

**Date**: 2025-12-29 01:47 UTC
**Binary MD5**: `d78ff865d371e32bb871a185d9b1ff29`
**Test Result**: EXTENSIVE lavender corruption even with deblocking DISABLED
**Status**: Deblocking hypothesis DEFINITIVELY RULED OUT

---

## What We Tested

### Verification from Log

```
✅ AVC444 encoder initialized for 1280×800 (4:4:4 chroma)
🔬 HACK TEST: openh264-rs modified to disable deblocking for ALL encoders
[AVC444 Frame #0] Main: IDR, Aux: IDR
[AVC444 Frame #1] Main: P, Aux: P
[AVC444 Frame #2] Main: P, Aux: P
...
```

**Confirmed**:
- ✅ AVC444 dual-stream encoding active
- ✅ P-frames in both streams
- ✅ Deblocking filter ACTUALLY disabled (hardcoded at init)
- ❌ EXTENSIVE lavender corruption still present

---

## What This Means

### Deblocking is NOT the Root Cause

The corruption occurs **even without deblocking filter**. This rules out deblocking as:
- Primary cause
- Contributing factor
- Relevant to the problem

### What We Know FOR CERTAIN Now

✅ **Packing algorithm**: Correct (matches FreeRDP, verified extensively)
✅ **Color conversion**: Correct (deterministic, verified)
✅ **Stride alignment**: Correct (fixed, no padding mismatch)
✅ **Stream synchronization**: Perfect (both IDR, both P)
✅ **Deblocking filter**: NOT the cause (corruption persists without it)

❌ **Something else in P-frame processing** causes lavender corruption

---

## Remaining Hypotheses

### Hypothesis 1: Dual-Stream P-Frame Architecture Issue

**MS-RDPEGFX Spec Quote**:
> "For macroblocks in rectangles in a received chroma subframe, color conversion MUST use the Y, U, and V components from the last corresponding rectangle in a luma subframe together with the current chroma subframe."

**Question**: Are auxiliary P-frames supposed to be INDEPENDENT, or do they reference the main stream somehow?

**Current Implementation**: We encode main and auxiliary as two completely independent H.264 streams. Each has its own reference frames.

**Potential Issue**: Maybe auxiliary P-frames need to coordinate with main stream P-frames in a way we don't understand?

---

### Hypothesis 2: Quantization/QP Differences

**The Issue**: Auxiliary stream encodes chroma as luma, using luma QP values.

**Chroma vs Luma Quantization**:
- Standard H.264: Chroma QP offset from luma QP (usually -2 to -6)
- Our auxiliary: Using luma QP on chroma data
- Result: Chroma over-quantized → precision loss → color shifts?

**Test**: Try auxiliary encoder with LOWER QP (higher quality):
```rust
// Auxiliary bitrate 2x or 3x higher than main
// Results in lower QP, less quantization damage
```

---

### Hypothesis 3: Transform/DCT Artifacts on Chroma

**The Issue**: H.264's DCT transform optimized for luma frequency content.

**Chroma Characteristics**:
- Lower spatial frequency than luma
- Different energy distribution in frequency domain
- Transform might not be optimal for chroma

**Evidence**: Corruption appears as macroblock-level artifacts (8x8 transform blocks)

---

### Hypothesis 4: Motion Compensation/Prediction Issues

**The Issue**: P-frame motion estimation finds matching blocks. Search tuned for luma patterns.

**For Chroma-as-Luma**:
- Motion search might find wrong matches (chroma has different patterns)
- Poor motion vectors → large residuals
- Large residuals + quantization → corruption

**Test**: Force smaller motion search range or disable motion search for auxiliary?

---

### Hypothesis 5: Reference Frame Management

**The Issue**: How auxiliary frames are stored/referenced in DPB (decoded picture buffer).

**From Other Session**:
> "Check that aux frames are not entering the reference picture buffer (DPB). Make sure those pictures are non-reference (nal_ref_idc=0) and don't affect POC/frame_num."

**Current**: We don't control nal_ref_idc directly. OpenH264 manages this.

**Question**: Should auxiliary frames be marked as non-reference frames?

---

### Hypothesis 6: Incorrect Auxiliary Stream Usage by Client

**The Issue**: Maybe our ENCODING is correct, but the Windows RDP CLIENT has trouble decoding our specific auxiliary P-frames?

**Evidence**:
- All-I frames work perfectly (client can decode those)
- P-frames corrupt (maybe client decoder bug with our P-frames?)

**Test**: Capture bitstreams, analyze NAL structure, compare with working implementation.

---

## Immediate Next Steps

### Step 1: Research Dual-Stream P-Frame Requirements

**Questions to answer**:
1. How do main and auxiliary P-frames coordinate?
2. Should they reference each other or be independent?
3. Are there timing/ordering requirements?
4. What does "last corresponding rectangle in luma subframe" mean for P-frames?

**Sources**:
- MS-RDPEGFX detailed spec
- FreeRDP encoder implementation (if they have server-side AVC444)
- Microsoft's original Wu et al. paper

---

### Step 2: Test Quantization Theory

**Quick Test**: Increase auxiliary bitrate significantly:

```rust
// Current: Same bitrate as main (5000 kbps)
// Test: 2-3x higher for auxiliary (10000-15000 kbps)
// Result: Lower QP, less quantization damage

let aux_config = OpenH264Config::new()
    .bitrate(BitRate::from_bps(15_000_000))  // 3x higher
    .max_frame_rate(FrameRate::from_hz(30));
```

**If corruption disappears**: Quantization was the issue
**If corruption persists**: Quantization not the cause

---

### Step 3: Analyze NAL Structure

**Extract and compare NAL units**:

```rust
// Parse P-frame bitstreams
fn analyze_nal_structure(bitstream: &[u8]) {
    // Find NAL units
    // Check: slice type, QP, nal_ref_idc
    // Log motion vectors, reference indices
    // Compare main vs aux
}
```

Look for:
- Different nal_ref_idc values
- Different slice types
- Different reference management

---

### Step 4: Test with Forced Low QP

Via raw API, force QP to very low value (high quality):

```rust
params.iTargetQP = 10;  // Very low (high quality)
params.iMinQP = 10;
params.iMaxQP = 15;
```

---

### Step 5: Investigate FreeRDP Server Implementation

**Question**: Does FreeRDP's server-side AVC444 work with P-frames?

If yes: What do they do differently?
If no: They might have the same issue or only support all-I

---

## Implications

### What We Can Rule Out
- ❌ Deblocking filter (tested with it disabled)
- ❌ Vec initialization (tested)
- ❌ Padding in aux_u/aux_v (tested)
- ❌ Stride mismatch (fixed)
- ❌ Input nondeterminism (verified stable)
- ❌ Packing algorithm bugs (verified correct)

### What Remains
- ⚠️ Quantization issues
- ⚠️ Dual-stream P-frame architecture
- ⚠️ Motion compensation artifacts
- ⚠️ Reference frame management
- ⚠️ Transform/DCT issues
- ⚠️ Client decoder issues
- ⚠️ Something we haven't thought of

---

## My Analysis

Given that:
1. All-I frames work perfectly
2. AVC420 P-frames work perfectly (confirmed in fallback test)
3. AVC444 P-frames fail (even without deblocking)

**The issue is specific to**:
- **P-frame prediction** in the **auxiliary stream** specifically
- OR **dual-stream coordination** in P-frames
- OR **client-side decoding** of auxiliary P-frames

**Not related to**:
- Deblocking
- Padding
- Stride
- Basic encoding quality

---

## Next Investigation Priorities

1. **HIGHEST**: Quantization test (easy, high impact if true)
2. **HIGH**: Understand dual-stream P-frame architecture from spec
3. **MEDIUM**: Analyze NAL structure differences
4. **LOW**: Reference frame marking

---

## Sources

- [MS-RDPEGFX YUV420p Combination](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/8131c1bc-1af8-4907-a05a-f72f4581160f)
- [MS-RDPEGFX RFX_AVC444_BITMAP_STREAM](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/844018a5-d717-4bc9-bddb-8b4d6be5dd3f)
- [FreeRDP AVC444 Issue #11040](https://github.com/FreeRDP/FreeRDP/issues/11040)
- [FreeRDP AVC444 Quality Issue #4030](https://github.com/FreeRDP/FreeRDP/issues/4030)
