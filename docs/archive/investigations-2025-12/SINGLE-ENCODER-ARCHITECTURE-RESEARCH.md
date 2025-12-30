# Single Encoder Architecture - Comprehensive Research

**Date**: 2025-12-29
**Status**: Deep research in progress
**Goal**: Design robust single encoder architecture for AVC444 P-frame support

---

## TABLE OF CONTENTS

1. [MS-RDPEGFX Spec Requirements](#spec-requirements)
2. [Single Encoder Interpretation](#single-encoder-interpretation)
3. [OpenH264 Capabilities](#openh264-capabilities)
4. [FreeRDP Implementation Analysis](#freerdp-analysis)
5. [Architecture Design Options](#design-options)
6. [Critical Implementation Questions](#critical-questions)
7. [Risk Assessment](#risk-assessment)
8. [Recommended Approach](#recommended-approach)

---

## MS-RDPEGFX Spec Requirements

### Primary Requirement

> "The two subframe bitstreams MUST be encoded using the same H.264 encoder and decoded by a single H.264 decoder as one stream."

**Source**: [RFX_AVC444_BITMAP_STREAM](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/844018a5-d717-4bc9-bddb-8b4d6be5dd3f)

### Subframe Types (LC Field)

| LC | Stream 1 | Stream 2 | Description |
|----|----------|----------|-------------|
| 0x0 | YUV420 (main/luma) | Chroma420 (aux) | Both present |
| 0x1 | YUV420 only | (none) | Chroma deferred |
| 0x2 | Chroma420 only | (none) | Combines with previous |
| 0x3 | INVALID | INVALID | Must not occur |

### Rectangle Synchronization

> "For macroblocks in chroma subframes, color conversion MUST use Y, U, V components from the **last corresponding rectangle in a luma subframe**."

**Implications**:
- Chroma subframes depend on luma subframes
- Decoder maintains state between luma and chroma processing
- "Last corresponding" implies temporal ordering

### What the Spec DOESN'T Say

❓ How to structure encoder calls
❓ Access unit organization
❓ POC/frame_num management
❓ Reference frame list management
❓ Whether subframes share reference frames or maintain separate lists

---

## Single Encoder Interpretation

### Possible Interpretations

#### Interpretation A: Sequential Subframes in Same Access Unit

**Concept**: Both subframes are part of ONE frame's encoding

```
Frame N:
  Encode main_yuv420 → Bitstream A (slice 0)
  Encode aux_yuv420 → Bitstream B (slice 1)
  Both part of same access unit, same POC, same frame_num
```

**DPB Impact**:
- One DPB entry per logical frame
- Both subframes share DPB slot
- P-frames reference the combined frame

**Questions**:
- Can OpenH264 encode two different YUV420 buffers as one access unit?
- How to structure slices?

---

#### Interpretation B: Interleaved Frames in Stream

**Concept**: Subframes are separate frames in one encoder's stream

```
Timeline:
  Frame 0: Encode main_yuv420 → POC=0, frame_num=0
  Frame 1: Encode aux_yuv420 → POC=1, frame_num=1
  Frame 2: Encode main_yuv420 → POC=2, frame_num=2 (P-frame refs POC=0)
  Frame 3: Encode aux_yuv420 → POC=3, frame_num=3 (P-frame refs POC=1)
```

**DPB Impact**:
- Two DPB entries (main and aux)
- Main P-frames reference main history
- Aux P-frames reference aux history
- ONE encoder manages BOTH histories

**Questions**:
- Does client expect this interleaving?
- How are POCs mapped to display order?
- Reference frame list management?

---

#### Interpretation C: Logical Stream, Physical Separation

**Concept**: "Same encoder" means same configuration/state, not same instance

**This would mean our current approach is OK**

**Evidence Against**:
- Spec says "same encoder" not "same configuration"
- Our approach fails with P-frames
- This interpretation doesn't match reality

**Verdict**: Unlikely correct interpretation

---

## OpenH264 Capabilities Research

### Question 1: Can One Encoder Handle Sequential Different Frames?

**From OpenH264 API**:
```c
int EncodeFrame(ISVCEncoder* encoder, SSourcePicture* pic, SFrameBSInfo* info);
```

**Each call**:
- Takes one SSourcePicture (one YUV420 frame)
- Produces one or more NAL units
- Updates DPB with reconstructed frame

**For our use**:
```rust
// Call 1:
let info1 = encoder.encode_frame(&main_yuv420)?;
// DPB now contains reconstructed main frame

// Call 2:
let info2 = encoder.encode_frame(&aux_yuv420)?;
// DPB now contains reconstructed aux frame
// But what happened to main frame in DPB?
```

**Critical Question**: After second encode, does DPB contain:
- Only aux frame (main replaced)?
- Both frames (DPB has 2 slots)?
- Main frame marked as reference, aux not?

---

### Question 2: DPB Management Between Calls

**H.264 DPB Basics**:
- Max DPB size determined by level (Level 4.0 = 12,288 macroblocks)
- For 1280x800: 50×80 = 4,000 MB → can hold ~3 frames in DPB
- Reference frames stored until marked for eviction

**OpenH264 DPB**:
- Managed automatically
- Reference list reordering possible
- Can we query/control DPB state?

**What We Need to Know**:
- After encoding main (POC=0), then aux (POC=1), what's in DPB?
- When encoding main P-frame (POC=2), does it reference POC=0 or POC=1?
- Is POC even the right concept here?

---

### Question 3: Bitstream Structure

**Standard H.264 Stream**:
```
Access Unit 0: [SPS] [PPS] [IDR Slice]
Access Unit 1: [P Slice]
Access Unit 2: [P Slice]
...
```

**For AVC444 with Interleaved Subframes**:

**Option A - Separate Access Units**:
```
Access Unit 0: [SPS] [PPS] [IDR Slice - Main]
Access Unit 1: [IDR Slice - Aux]
Access Unit 2: [P Slice - Main]
Access Unit 3: [P Slice - Aux]
```

**Option B - Combined Access Units** (unlikely):
```
Access Unit 0: [SPS] [PPS] [IDR Slice - Main] [IDR Slice - Aux]
Access Unit 2: [P Slice - Main] [P Slice - Aux]
```

**Option C - Something Else**?

**What RDP PDU Structure Shows**:
```rust
struct RFX_AVC444_BITMAP_STREAM {
    avc420EncodedBitstream1: Vec<u8>,  // Separate complete bitstream
    avc420EncodedBitstream2: Vec<u8>,  // Separate complete bitstream
}
```

This suggests **separate complete bitstreams**, not interleaved NAL units.

---

## Critical Implementation Questions

### Q1: What Does "Same Encoder" Actually Mean?

**Possible Meanings**:

**A) Same OpenH264 instance, sequential encode() calls**:
```rust
let encoder = Encoder::new();
let main_nal = encoder.encode(&main_yuv420)?;  // Call 1
let aux_nal = encoder.encode(&aux_yuv420)?;    // Call 2 (same instance)
```

**B) Same encoder configuration, separate instances OK**:
```rust
let config = EncoderConfig::new();
let main = Encoder::with_config(config.clone());
let aux = Encoder::with_config(config.clone());
```
(This is what we do - and it fails!)

**C) Some special encoder mode we don't know about**

**Verdict**: Most likely (A) - same instance

---

### Q2: How Should We Structure encode() Calls?

**Option 1: Double encode() per logical frame**:
```rust
fn encode_bgra(...) {
    let (main, aux) = pack_dual_views(&yuv444);

    // Encode main
    encoder.encode(&main)?;  // Sets DPB[N]

    // Encode aux (what happens to DPB?)
    encoder.encode(&aux)?;   // Sets DPB[N+1]? Or replaces DPB[N]?
}
```

**Option 2: Encode main, skip aux encoding, use main bitstream twice?**
(Doesn't make sense)

**Option 3: Encode both as part of larger structure?**
(OpenH264 doesn't expose this)

---

### Q3: Reference Frame Strategy

**For Main P-frames**:
- Should reference previous main subframe ✓

**For Aux P-frames**:
- Should reference previous aux subframe? ✓
- OR should reference main subframe? ❓
- OR should be non-reference (nal_ref_idc=0)? ❓

**From spec**:
> "Last corresponding rectangle in luma subframe"

This suggests aux DOES depend on main, but at DECODER side for color conversion, not necessarily at ENCODER side for prediction.

---

### Q4: POC and Frame Numbering

**If interleaved**:
```
POC sequence: 0(main), 1(aux), 2(main), 3(aux), ...
Frame_num: 0, 0, 1, 1, 2, 2, ...? Or 0,1,2,3,...?
```

**Standard H.264**:
- POC determines display order
- frame_num increments for each reference frame
- Non-reference frames can share frame_num with previous

**For AVC444**:
- Display order: Both subframes belong to same display time
- How to represent this in POC?
- Should POC be: 0,0,2,2,4,4,... (pairs)?

---

## FreeRDP Implementation Analysis

### Finding: FreeRDP Server H.264 Encoder

**File**: `server/shadow/shadow_encoder.c`

**Relevant code**:
```c
static int shadow_encoder_init_h264(rdpShadowEncoder* encoder)
{
    encoder->h264 = h264_context_new(TRUE);  // ONE context
    // ...
    encoder->codecs |= FREERDP_CODEC_AVC420 | FREERDP_CODEC_AVC444;
}
```

**FreeRDP wraps OpenH264** in `H264_CONTEXT`.

**Key Question**: How does FreeRDP's h264_context handle AVC444 dual subframes?

**Looking for**:
- `avc444_compress()` or similar function
- How they call OpenH264 for main vs aux

### Finding: FreeRDP Only Implements CLIENT Side?

**Searching GitHub**: Most AVC444 code is in decoder (client) path.

**Server code**: Mostly shows capability negotiation, not actual encoding.

**Implication**: FreeRDP server might NOT implement AVC444 encoding, OR they haven't open-sourced it, OR it's done differently.

---

## Architecture Design Options

### Design A: True Interleaved Encoding

**Structure**:
```rust
pub struct Avc444Encoder {
    encoder: Encoder,
    subframe_counter: u64,  // Tracks main vs aux
}

impl Avc444Encoder {
    pub fn encode_bgra(...) {
        let (main, aux) = pack_dual_views(&yuv444);

        // Encode main subframe (POC = counter * 2)
        let main_bs = self.encoder.encode(&main)?;

        // Encode aux subframe (POC = counter * 2 + 1)
        let aux_bs = self.encoder.encode(&aux)?;

        self.subframe_counter += 1;
    }
}
```

**Pros**:
- ✅ One encoder, one DPB
- ✅ Maintains unified temporal history
- ✅ Matches spec literally

**Cons**:
- ⚠️ DPB contains BOTH main and aux frames (uses 2x slots)
- ⚠️ Not sure if this is correct interpretation
- ⚠️ Reference list might be confusing (which frame does P reference?)

**Questions**:
- Does aux P-frame reference previous aux or previous main?
- How does DPB eviction work with doubled frames?

---

### Design B: Single Encoder, Selective Encoding

**Structure**:
```rust
pub struct Avc444Encoder {
    encoder: Encoder,
}

impl Avc444Encoder {
    pub fn encode_bgra(...) {
        let (main, aux) = pack_dual_views(&yuv444);

        // Encode ONLY main through encoder (maintains normal DPB)
        let main_bs = self.encoder.encode(&main)?;

        // For aux: Don't use encoder for prediction
        // Encode as I-frame manually? Or force intra?
        self.encoder.force_intra_frame();
        let aux_bs = self.encoder.encode(&aux)?;
    }
}
```

**Pros**:
- ✅ Simple DPB (only main frames)
- ✅ Aux always I-frame (no corruption)

**Cons**:
- ❌ Aux uses all-I (not P-frames)
- ❌ Defeats purpose of fix

**Verdict**: This is basically our hybrid workaround, not a real solution

---

### Design C: Encoder Pool/Swap

**Structure**:
```rust
pub struct Avc444Encoder {
    main_encoder: Encoder,
    aux_encoder: Encoder,
    // But manage their DPBs carefully
}
```

**Approach**:
- Encode main normally
- Before encoding aux, somehow sync DPB from main to aux?
- Or reset aux encoder state to match main?

**Pros**:
- Uses existing structure

**Cons**:
- ❌ Can't sync DPB between encoders (not exposed in OpenH264 API)
- ❌ Doesn't solve the spec violation

**Verdict**: Not viable without deep OpenH264 internals access

---

## OpenH264 Frame Encoding Research

### How encode() Affects DPB

**From OpenH264 documentation and code inspection**:

```
1. Call encode() with YUV frame
2. Encoder performs motion estimation (if P-frame)
   - Searches in DPB for matching blocks
3. Generates prediction
4. Computes residuals
5. Transform + quantize
6. Entropy code
7. Inverse transform + dequantize
8. Add to prediction
9. Store reconstructed frame in DPB  ← Key step
10. Output bitstream
```

**After encode() returns**:
- DPB updated with reconstructed frame
- Frame marked as reference (usually) or non-reference
- DPB eviction may occur (FIFO or based on reference marking)

### Sequential encode() Calls

**Scenario**:
```rust
encoder.encode(&frame_0)?;  // DPB: [frame_0]
encoder.encode(&frame_1)?;  // DPB: [frame_0, frame_1] or [frame_1]?
encoder.encode(&frame_2)?;  // DPB: [frame_0, frame_1, frame_2]? Or sliding window?
```

**DPB Sliding Window** (default for many profiles):
- Keep N most recent reference frames
- Evict oldest when DPB full
- For BaselineProfile/MainProfile: typically 1-4 frames

**Key Insight**:
If we encode main then aux then main then aux:
- DPB might contain: [main_N-1, aux_N-1, main_N]
- Main P-frame references main_N-1 ✓
- Aux P-frame references aux_N-1 ✓
- **This could work!**

---

### POC Assignment Strategy

**Picture Order Count (POC)**:
- Determines display order
- Used for reference frame ordering
- Must be unique and increasing

**For AVC444 Interleaving**:

**Strategy 1: Paired POCs**:
```
Main frame 0: POC=0
Aux frame 0: POC=0  (same as main)
Main frame 1: POC=2
Aux frame 1: POC=2  (same as main)
```

**Strategy 2: Sequential POCs**:
```
Main frame 0: POC=0
Aux frame 0: POC=1
Main frame 1: POC=2
Aux frame 1: POC=3
```

**Strategy 3: Separate POC Spaces** (if possible):
```
Main frames: POC=0,1,2,3...
Aux frames: POC=0,1,2,3... (separate numbering)
```

**Question**: Which does OpenH264 use? Can we control it?

---

## Critical Implementation Questions (Detailed)

### Q1: How to Maintain Separate Reference Histories with One Encoder?

**The Challenge**:
- Main P-frames should reference previous MAIN frames
- Aux P-frames should reference previous AUX frames
- But there's only ONE DPB

**Possible Solutions**:

**S1a: Reference List Reordering**:
- OpenH264 supports reference list reordering
- Before encoding aux P-frame, reorder list to put previous aux at index 0
- **API Question**: Can we control this from openh264-rs?

**S1b: Long-Term Reference (LTR)**:
- Mark main frames as LTR slot 0
- Mark aux frames as LTR slot 1
- Each P-frame references its own slot
- **API Question**: Does openh264-rs expose LTR?

**S1c: Accept Cross-References**:
- Let aux P-frames sometimes reference main frames
- Might not be optimal but could work?
- **Testing needed**

---

### Q2: Frame_num Management

**H.264 Spec**: frame_num increments for each reference frame

**For AVC444**:

**Option A**: Increment for each subframe:
```
Main 0: frame_num=0
Aux 0: frame_num=1
Main 1 P: frame_num=2 (references frame_num=0)
Aux 1 P: frame_num=3 (references frame_num=1)
```

**Option B**: Share frame_num per logical frame:
```
Main 0: frame_num=0
Aux 0: frame_num=0 (same)
Main 1 P: frame_num=1 (references frame_num=0 - which one?)
Aux 1 P: frame_num=1 (references frame_num=0 - which one?)
```

**OpenH264 behavior**: Automatically manages frame_num

**Question**: Does sequential encode() increment frame_num automatically?

---

### Q3: SPS/PPS for Dual Subframes

**Current** (two encoders):
- Each encoder has own SPS/PPS
- We cache and prepend separately

**Single encoder**:
- ONE set of SPS/PPS
- Do both subframes use same SPS/PPS? Likely yes
- Prepend SPS/PPS to both IDR subframes? Or just first?

**Client expectation**:
- Probably expects same SPS/PPS for both subframes
- Makes sense since they're from "one stream"

---

## Experimental Approach

### Experiment A: Minimal Restructure Test

**Goal**: Validate that single encoder instance works at all

```rust
pub struct Avc444Encoder {
    encoder: Encoder,  // Remove main_encoder, aux_encoder
}

fn encode_bgra(...) {
    let (main, aux) = pack_dual_views(&yuv444);

    // Sequential encoding
    let main_bs = self.encoder.encode(&main)?;
    let aux_bs = self.encoder.encode(&aux)?;

    // Package and send
}
```

**Test**:
- Does it compile?
- Does it encode?
- What's the corruption like? (worse? better? different?)

**This is lowest-risk first step**

---

### Experiment B: Force Aux to I-Frames with Single Encoder

**Goal**: Test if single encoder with aux-I works differently than two encoders with aux-I

```rust
fn encode_bgra(...) {
    let (main, aux) = pack_dual_views(&yuv444);

    // Main as normal
    let main_bs = self.encoder.encode(&main)?;

    // Aux as I-frame
    self.encoder.force_intra_frame();
    let aux_bs = self.encoder.encode(&aux)?;
}
```

**Comparison**:
- Current: Two encoders, both all-I → perfect
- This: One encoder, aux all-I, main P → ?

---

## Next Research Tasks

1. **Study OpenH264 DPB behavior**:
   - Trace through encode() to see DPB updates
   - Understand reference marking
   - Figure out eviction policy

2. **Analyze NAL structure**:
   - Parse our current bitstreams
   - Check POC values
   - Check frame_num values
   - See how they differ between main and aux

3. **Contact experts**:
   - Open issue on FreeRDP (ask if they do AVC444 server encoding)
   - Reach out to openh264 community
   - Contact Microsoft RDP team if possible

4. **Study Wu et al. paper more carefully**:
   - Original research might have implementation details
   - Could reveal encoder architecture

---

## Research Status: In Progress

**Completed**:
- ✅ MS-RDPEGFX spec primary requirements
- ✅ Subframe types and LC field
- ✅ Rectangle synchronization requirements
- ✅ Initial OpenH264 API understanding

**In Progress**:
- 🔄 DPB management details
- 🔄 POC/frame_num strategy
- 🔄 FreeRDP implementation search

**Pending**:
- ⬜ Design decision matrix
- ⬜ Implementation plan
- ⬜ Risk assessment
- ⬜ Testing strategy

**Continuing research...**
