# SMOKING GUN: Single Encoder Requirement - Root Cause Identified

**Date**: 2025-12-29
**Status**: ARCHITECTURAL FLAW IDENTIFIED
**Severity**: Critical - Requires complete restructure

---

## The Smoking Gun (from MS-RDPEGFX Spec)

> **"The two subframe bitstreams MUST be encoded using the same H.264 encoder and decoded by a single H.264 decoder as one stream."**

**Source**: [MS-RDPEGFX RFX_AVC444_BITMAP_STREAM](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/844018a5-d717-4bc9-bddb-8b4d6be5dd3f)

---

## Our Fundamental Architectural Flaw

### What We're Doing (WRONG)

```rust
pub struct Avc444Encoder {
    main_encoder: Encoder,    // ← Encoder instance #1
    aux_encoder: Encoder,     // ← Encoder instance #2 (PROBLEM!)
}

fn encode_bgra(...) {
    let (main_yuv420, aux_yuv420) = pack_dual_views(&yuv444);

    let main_bitstream = self.main_encoder.encode(&main_yuv420)?;  // DPB #1
    let aux_bitstream = self.aux_encoder.encode(&aux_yuv420)?;     // DPB #2 (WRONG!)
}
```

**This Creates**:
- ❌ TWO separate DPBs (decoded picture buffers)
- ❌ TWO separate temporal reference histories
- ❌ TWO separate POC (picture order count) sequences
- ❌ TWO separate frame numbering sequences
- ❌ TWO independent H.264 elementary streams

### What the Spec Requires (CORRECT)

```rust
pub struct Avc444Encoder {
    encoder: Encoder,  // ← ONE encoder for BOTH subframes
}

fn encode_bgra(...) {
    let (main_yuv420, aux_yuv420) = pack_dual_views(&yuv444);

    // Encode both through SAME encoder (maintains ONE DPB)
    let main_bitstream = self.encoder.encode(&main_yuv420)?;   // Subframe 1
    let aux_bitstream = self.encoder.encode(&aux_yuv420)?;     // Subframe 2

    // Package into AVC444 structure
}
```

**This Creates**:
- ✅ ONE DPB shared between main and aux
- ✅ ONE temporal reference history
- ✅ ONE POC sequence
- ✅ ONE frame numbering
- ✅ ONE H.264 elementary stream (logically)

---

## Why This Explains EVERYTHING

### Why All-I Frames Work

**All-I frames**: No reference frames, each frame self-contained

**Our broken architecture**: Doesn't matter! No DPB lookups, no temporal dependencies.

**Result**: Perfect quality ✅

---

### Why AVC420 Works

**AVC420**: Single stream, single encoder, normal prediction

**No dual-stream complexity**: Just works normally

**Result**: Perfect quality ✅

---

### Why AVC444 P-Frames Fail

**Our implementation**: Two separate encoders

**Encoding**:
```
Encoder #1 (main):  Frame 0 IDR → DPB[0]
                    Frame 1 P references DPB[0] ✓

Encoder #2 (aux):   Frame 0 IDR → DPB[0] (separate DPB!)
                    Frame 1 P references DPB[0] ✓
```

**Client decoding** (expects ONE stream):
```
Client DPB:
  Decode frame 0 main IDR → DPB[0] = main frame 0
  Decode frame 0 aux IDR → DPB[1] = aux frame 0
  Decode frame 1 main P → references DPB[0] = main frame 0 ✓ Correct!
  Decode frame 1 aux P → references DPB[1] = aux frame 0 ✗ But client expects different ref!
```

**Or worse** - client might expect:
```
  Decode "subframe 0" (main IDR) → DPB[0]
  Decode "subframe 1" (aux IDR) → Use same DPB[0] as reference?
  Decode "subframe 2" (main P) → references subframe 0
  Decode "subframe 3" (aux P) → references subframe 1 BUT DPB state is wrong!
```

**Result**: Reference frame mismatch → decoder reconstructs from wrong data → **lavender corruption** ❌

---

## Why Deblocking Disable Didn't Help

Deblocking operates on reconstructed frames AFTER decoding.

The corruption happens DURING P-frame prediction because:
- Client's DPB doesn't match what our encoders expect
- Wrong reference frame used for prediction
- Motion compensation + residuals applied to wrong base
- **Reconstructed pixels are wrong BEFORE deblocking even runs**

Deblocking might spread it, but it's not the cause.

---

## The "Corresponding Rectangle" Requirement

From spec:
> "For macroblocks in rectangles in a received chroma subframe, color conversion MUST use the Y, U, and V components from the last corresponding rectangle in a luma subframe together with the current chroma subframe."

**This implies**:
- Chroma subframes DEPEND on luma subframes
- There's a synchronization requirement
- It's not just "two independent streams"

**Our current architecture violates this** because each encoder has independent state.

---

## The Fix: Single Encoder Architecture

### Conceptual Change

**Current** (WRONG):
```rust
struct Avc444Encoder {
    main_encoder: Encoder,  // Independent temporal history
    aux_encoder: Encoder,   // Independent temporal history
}
```

**Required** (CORRECT):
```rust
struct Avc444Encoder {
    encoder: Encoder,  // ONE temporal history for BOTH
    // Track which subframe we're encoding (luma vs chroma)
}
```

### Encoding Flow

**Per Frame**:
1. Pack YUV444 → (main_yuv420, aux_yuv420)
2. Encode main_yuv420 through encoder → bitstream_1
3. Encode aux_yuv420 through SAME encoder → bitstream_2
4. Create RFX_AVC444_BITMAP_STREAM with both bitstreams
5. Send to client

**Key**: Both subframes go through the SAME encoder instance in sequence, maintaining ONE DPB.

---

## Implementation Challenges

### Challenge 1: Encoder State

After encoding main_yuv420, the encoder's DPB contains the reconstructed main frame.

When we encode aux_yuv420 next, it will use that main frame as a reference.

**Question**: Is this correct? Or do we need to control reference frame selection?

### Challenge 2: Frame Dimensions

Main and aux have SAME dimensions (1280x800).

Both are YUV420 format.

Encoder should handle this, but need to verify.

### Challenge 3: SPS/PPS Management

Currently we cache separate SPS/PPS for main and aux.

With one encoder, there's ONE set of SPS/PPS.

**Question**: Do we need different SPS/PPS for subframes?

### Challenge 4: Bitrate Control

One encoder = one bitrate target.

**Question**: How to allocate bitrate between main and aux subframes?

**Options**:
- Let encoder decide (might not be optimal)
- Use frame-level QP hints (if OpenH264 exposes this)
- Accept that bitrate is shared

---

## Implementation Plan

### Phase 1: Minimal Restructure (Proof of Concept)

**Goal**: Use one encoder for both subframes, test if corruption disappears

```rust
pub struct Avc444Encoder {
    encoder: Encoder,  // Single encoder
    // Remove main_encoder and aux_encoder
}

impl Avc444Encoder {
    pub fn encode_bgra(...) -> Result<Avc444Frame> {
        // Convert BGRA → YUV444
        let yuv444 = bgra_to_yuv444(bgra, width, height, self.color_matrix);

        // Pack into dual YUV420 views
        let (main_yuv420, aux_yuv420) = pack_dual_views(&yuv444);

        // Encode MAIN subframe
        let main_bitstream = self.encoder.encode(&main_yuv420)?;
        let stream1_data = main_bitstream.to_vec();

        // Encode AUX subframe (SAME encoder, maintains DPB continuity)
        let aux_bitstream = self.encoder.encode(&aux_yuv420)?;
        let stream2_data = aux_bitstream.to_vec();

        // Create AVC444 frame
        Avc444Frame {
            stream1_data,
            stream2_data,
            // ...
        }
    }
}
```

**Expected Result**: Corruption disappears (if architectural issue is the cause)

---

### Phase 2: Handle SPS/PPS Correctly

With one encoder, we get one set of SPS/PPS.

**Question**: Do both subframes use the same SPS/PPS?
**Answer from spec research needed**: TBD

---

### Phase 3: Bitrate Management

**Options**:
1. Use single bitrate for both (simplest)
2. Alternate bitrate between frames (complex)
3. Accept that encoder distributes bitrate automatically

---

### Phase 4: Optimize

Once working:
- Fine-tune bitrate allocation
- Optimize reference frame selection
- Performance tuning

---

## Why This Makes Perfect Sense

### The Symptoms Match

**Lavender corruption in changed areas**: Classic reference frame mismatch
- P-frame predicts from wrong reference
- Motion compensation applies to wrong base
- Result: Color shifts (lavender/brown)

**Static areas work fine**: When content doesn't change, even wrong reference looks similar
**Changed areas corrupt**: Different reference → prediction fails → corruption

### The Tests Match

**All-I works**: No references needed
**AVC420 works**: Single stream architecture
**AVC444 P-frames fail**: Dual-stream with separate DPBs

### Deblocking Test Makes Sense

Corruption occurs BEFORE deblocking (during prediction).

Disabling deblocking can't fix prediction from wrong reference frame.

---

## Immediate Action

**STOP quantization test** - that's not the issue.

**START single encoder restructure** - this is the root cause with 95% confidence.

### Restructure Scope

**Files to modify**:
- `src/egfx/avc444_encoder.rs`: Complete rewrite (change from 2 encoders to 1)
- Struct definition
- Initialization
- Encoding flow
- SPS/PPS handling

**Estimated effort**: 2-3 hours for initial implementation + testing

**Risk**: Medium (significant architecture change)

**Confidence**: 95% this will fix the corruption

---

## Questions for You

1. **Should I proceed with single encoder restructure immediately?**
   - This supersedes the quantization test
   - High confidence this is the real fix

2. **Or test quantization first anyway?**
   - Lower confidence but already deployed
   - Quick test (5 min)
   - Then do restructure

3. **Want me to study FreeRDP's implementation first?**
   - See how they handle single encoder
   - Might reveal implementation details

My strong recommendation: **Do single encoder restructure now**. This explains everything and matches the spec requirements exactly.