# ULTRA-DEEP CODE TRACE: What EXACTLY Happens vs What SHOULD Happen

**Purpose**: Trace EVERY step of our AVC444 encoding and compare with spec requirements
**Method**: Line-by-line code execution analysis
**Goal**: Find the EXACT mismatch causing corruption

---

## PART 1: WHAT OUR CODE ACTUALLY DOES

### Step-by-Step Execution Trace

#### Step 1: encode_bgra() Called

**Input**: BGRA frame from PipeWire
**Location**: `src/egfx/avc444_encoder.rs:270`

```rust
pub fn encode_bgra(bgra: &[u8], width: u32, height: u32, timestamp_ms: u64)
```

#### Step 2: BGRA → YUV444 Conversion

**Location**: Line ~297
```rust
let yuv444 = bgra_to_yuv444(bgra, width, height, self.color_matrix);
```

**Output**: YUV444Frame with FULL chroma resolution
- Y plane: 1280×800
- U plane: 1280×800 (full resolution!)
- V plane: 1280×800 (full resolution!)

#### Step 3: YUV444 → Dual YUV420 Packing

**Location**: Line ~303
```rust
let (main_yuv420, aux_yuv420) = pack_dual_views(&yuv444);
```

**Output**:
- `main_yuv420`: Standard YUV420 (luma + subsampled chroma)
  - Y: 1280×800
  - U: 640×400 (subsampled)
  - V: 640×400 (subsampled)

- `aux_yuv420`: Chroma-as-fake-luma YUV420
  - Y: 1280×800 (contains U444/V444 odd rows)
  - U: 640×400 (U444 odd columns, even rows)
  - V: 640×400 (V444 odd columns, even rows)

#### Step 4: Encode Main Subframe

**Location**: Line ~332-335
```rust
let main_yuv_slices = YUVSlices::new(...);
let main_bitstream = self.encoder.encode(&main_yuv_slices)?;
let stream1_data = main_bitstream.to_vec();
```

**What encoder does**:
1. Takes YUV420 data
2. Performs motion estimation (if P-frame)
3. Transform + quantize
4. Entropy coding
5. Produces Annex B bitstream

**Output for Frame 0**: `[SPS][PPS][IDR slice]`
**Output for Frame N**: `[P slice]` (if scene change doesn't trigger)

**Stored in DPB**: Reconstructed Main frame

#### Step 5: Encode Aux Subframe

**Location**: Line ~354-360
```rust
let aux_yuv_slices = YUVSlices::new(...);
let aux_bitstream = self.encoder.encode(&aux_yuv_slices)?;
let stream2_data = aux_bitstream.to_vec();
```

**What encoder does**:
1. Takes YUV420 data (aux subframe)
2. **DPB STATE**: Contains reconstructed Main frame from previous encode
3. Performs motion estimation → searches DPB
4. **Finds Main frame** (only thing in DPB for frame 0)
5. Tries to predict Aux from Main?? (THIS COULD BE THE PROBLEM!)
6. Produces bitstream

**Critical Issue**: Aux encoder state has Main in DPB, not previous Aux!

#### Step 6: SPS/PPS Handling

**Location**: Line ~388-389
```rust
stream1_data = self.handle_sps_pps(stream1_data, main_is_keyframe);
stream2_data = self.handle_sps_pps(stream2_data, aux_is_keyframe);
```

**Logic**:
- If IDR: Extract SPS/PPS, cache it
- If P-frame: Prepend cached SPS/PPS

**Current cache state**:
- Frame 0 Main IDR: Extracts SPS/PPS → cached
- Frame 0 Aux IDR: Extracts SPS/PPS → **OVERWRITES cache!**
- Frame 1 Main P: Gets **Aux's SPS/PPS** prepended? (BUG?)
- Frame 1 Aux: Gets Aux's SPS/PPS

**POTENTIAL BUG**: Main might be using Aux's SPS/PPS!

#### Step 7: Package as Avc444Frame

**Location**: Line ~406-420
```rust
Ok(Some(Avc444Frame {
    stream1_data,  // Main bitstream (possibly with wrong SPS/PPS?)
    stream2_data,  // Aux bitstream
    is_keyframe: main_is_keyframe,
    // ...
}))
```

#### Step 8: Send to Client

**Location**: `src/server/egfx_sender.rs:475-482`
```rust
server.send_avc444_frame(
    surface_id,
    stream1_data,      // Main H.264 bitstream
    &luma_regions,     // Full frame region
    Some(stream2_data), // Aux H.264 bitstream
    Some(&chroma_regions), // Full frame region
    timestamp_ms,
)
```

#### Step 9: IronRDP-EGFX Packaging

**Location**: `IronRDP/crates/ironrdp-egfx/src/server.rs:1179-1186`
```rust
let avc444_stream = Avc444BitmapStream {
    encoding: LUMA_AND_CHROMA,  // LC = 0x0
    stream1,  // Luma stream (main)
    stream2,  // Chroma stream (aux)
};

let bitmap_data = encode_avc444_bitmap_stream(&avc444_stream);
```

---

## PART 2: WHAT THE SPEC REQUIRES

### MS-RDPEGFX RFX_AVC444_BITMAP_STREAM Structure

**From spec**: [MS-RDPEGFX §3.2.9.1.2](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/844018a5-d717-4bc9-bddb-8b4d6be5dd3f)

```
LC (2 bits): Encoding mode
  0x0 = YUV420 + Chroma420
  0x1 = YUV420 only
  0x2 = Chroma420 only

avc420EncodedBitstream1: First subframe (luma/main)
avc420EncodedBitstream2: Second subframe (chroma/aux) (optional based on LC)
```

**Key Requirement**:
> "These bitstreams MUST be encoded using the same MPEG-4 AVC/H.264 encoder"

**Decoding Requirement**:
> "These bitstreams MUST be decoded by a single MPEG-4 AVC/H.264 decoder as one stream"

### What "Decoded as One Stream" Means

**This is CRITICAL and I haven't fully understood it!**

**Possibility 1**: Client concatenates both bitstreams and decodes as ONE continuous H.264 stream
```
Client receives:
  stream1 = [SPS][PPS][IDR_main]
  stream2 = [P_aux]

Client does:
  combined = stream1 + stream2 = [SPS][PPS][IDR_main][P_aux]
  decoder.decode(combined)  // ONE decoder call

  DPB after: [Main_reconstructed, Aux_reconstructed]
```

**Possibility 2**: Client decodes separately but shares DPB state somehow
```
decoder.decode(stream1)  // DPB: [Main]
decoder.decode(stream2)  // DPB: [Main, Aux]
```

**I NEED TO UNDERSTAND WHICH!**

---

## PART 3: THE CRITICAL MISUNDERSTANDING

### What I Think Might Be Wrong

**Our approach**:
```rust
// Frame 0:
main_bs = encoder.encode(&main)  // Produces: [SPS][PPS][IDR]
aux_bs = encoder.encode(&aux)    // Produces: [SPS][PPS][IDR]

// We send BOTH bitstreams separately to client
stream1 = [SPS][PPS][IDR_main]
stream2 = [SPS][PPS][IDR_aux]
```

**Client might expect** (if "one stream" means concatenated):
```
stream1 = [SPS][PPS][IDR_main]
stream2 = [P_aux]  // NO SPS/PPS! Uses same as Main!

// Or even:
combined_stream = [SPS][PPS][IDR_main][IDR_aux]
// And we should send this as ONE bitstream split into two "regions"?
```

### Key Question: SPS/PPS in Aux Subframes

**What we do**: Aux IDR has its own [SPS][PPS][IDR]
**What might be needed**: Aux shares Main's SPS/PPS

**If client concatenates**:
```
combined = Main_[SPS][PPS][IDR] + Aux_[SPS][PPS][IDR]
```

This gives TWO sets of SPS/PPS! Decoder might interpret this as:
- First SPS/PPS → Main stream
- Second SPS/PPS → NEW stream (reset)?
- Could cause decoder confusion

---

## PART 4: WHAT TO RESEARCH/TEST NEXT

### Critical Test 1: Single SPS/PPS for Both Subframes

**Hypothesis**: Aux should NOT have separate SPS/PPS

**Test**:
```rust
// After encoding Main (has SPS/PPS):
let main_data = main_bitstream.to_vec();
let sps_pps = extract_sps_pps(&main_data);

// After encoding Aux:
let mut aux_data = aux_bitstream.to_vec();

// Remove SPS/PPS from Aux if present
aux_data = strip_sps_pps(aux_data);

// DON'T prepend anything - Aux uses Main's SPS/PPS
```

**Client would then see**:
```
stream1 = [SPS][PPS][IDR_main or P_main]
stream2 = [IDR_aux or P_aux]  // No SPS/PPS!
```

**Rationale**: "Same encoder, one stream" might mean shared SPS/PPS

---

### Critical Test 2: Understand Encoder DPB State

**After these calls**:
```rust
let main_bs = encoder.encode(&main_yuv420);  // DPB = [Main]
let aux_bs = encoder.encode(&aux_yuv420);    // DPB = [Main, Aux]? Or [Aux]?
```

**What's ACTUALLY in DPB?**

**If DPB = [Aux] (Main evicted)**:
- Next Main encode can't reference previous Main!
- Would explain corruption

**If DPB = [Main, Aux]**:
- Should work (with NUM_REF=2)
- But maybe encoder picks wrong reference?

**Need to**: Understand OpenH264's DPB management with sequential encodes

---

### Critical Test 3: Reference Frame Usage

**With NUM_REF=2 and alternating encodes**:

**Frame sequence**:
```
0: Main IDR → DPB[0] = Main_0
1: Aux IDR → DPB[0,1] = [Main_0, Aux_0] or DPB[0] = Aux_0?
2: Main P → Should ref Main_0 (DPB slot 0 or 1?)
3: Aux P → Should ref Aux_0 (DPB slot 1 or 0?)
```

**If DPB is FIFO**:
- DPB[0] = oldest, DPB[1] = newest
- After Main_0, Aux_0 encodes: DPB = [Main_0, Aux_0]
- Main_1 encodes: Motion search looks at DPB
  - **Might find Aux_0 as "best match"** if search is unrestricted!
  - This would be WRONG reference → corruption!

**This could be it!**

---

## PART 5: THE SMOKING GUN HYPOTHESIS

### Why Corruption Persists with Single Encoder

**The Problem**: Motion search is UNRESTRICTED

**When encoding Main_1 (P-frame)**:
```
DPB contains: [Main_0, Aux_0]
Motion search: Looks at BOTH frames
Best match: Should be Main_0 (similar luma content)
But: Motion search might pick Aux_0 for some blocks!
  - If Aux happens to have similar pixel patterns
  - Or search algorithm artifact
Result: Some blocks predict from Aux_0 instead of Main_0
Decoded: Wrong prediction base → color corruption!
```

**Same for Aux_1**:
```
DPB contains: [Aux_0, Main_0] or [Main_0, Aux_0]
Motion search: Might pick Main_0 for some blocks
Result: Aux predicts from Main → chroma corruption!
```

### Why This Explains EVERYTHING

**All-I works**: No motion search, no references
**AVC420 works**: Only one content type, all references are valid
**AVC444 Main-P + Aux-IDR corrupts**: Main might ref Aux_0!
**AVC444 Both-P corrupts**: Cross-stream references happening!

### The Solution: PREVENT CROSS-STREAM REFERENCES

**Need to**:
1. **Force Main to only reference Main frames**
2. **Force Aux to only reference Aux frames**
3. **Or make Aux non-reference entirely**

**How?**:
- Can't control with NUM_REF alone (just sets DPB size)
- Need reference list control or frame marking
- LTR might actually be needed (but used correctly)
- Or make Aux non-reference (nal_ref_idc=0)

---

## PART 6: REVISED UNDERSTANDING OF LTR

### Why LTR Might Be Needed After All

**Not for "two chains" as I thought before**

**But for**: Preventing wrong reference selection

**LTR Usage**:
1. Mark Main_0 as LTR slot 0
2. Mark Aux_0 as LTR slot 1 (or non-reference)
3. When encoding Main_1: **Force reference to LTR slot 0**
4. When encoding Aux_1: Either force to slot 1 or make non-ref

**This CONSTRAINS motion search** to only look at correct frames!

### Or: Make Aux Non-Reference

**Simpler approach**:
1. Main frames: Normal (reference)
2. Aux frames: Mark as non-reference (nal_ref_idc=0)
3. **Main can't accidentally reference Aux!**
4. Aux predicts from... Main? Or nothing (I-frame)?

**If Aux is non-reference**: What does Aux P-frame predict from?
- If from Main: Might actually work (Aux derived from same content)
- If forced to I: Back to hybrid workaround

---

## PART 7: WHAT TO TEST IMMEDIATELY

### Test A: Strip Aux SPS/PPS

**Change handle_sps_pps logic**:
```rust
stream1_data = self.handle_sps_pps(stream1_data, main_is_keyframe);
// DON'T call for stream2 - let it use Main's SPS/PPS
// stream2_data = self.handle_sps_pps(stream2_data, aux_is_keyframe);

// Or strip SPS/PPS from aux_data explicitly
stream2_data = strip_sps_pps(stream2_data);
```

**Hypothesis**: Dual SPS/PPS confuses decoder

---

### Test B: Increase NUM_REF to 4-8

**Maybe DPB eviction is the issue**

```rust
.num_ref_frames(8)  // Large DPB, less eviction
```

---

### Test C: Add Detailed Motion Vector Logging

**If OpenH264 exposes it**: Log which DPB slots are being used

**Or**: Parse slice headers to extract ref_pic_list_modification

---

## PART 8: COMPARISON WITH WORKING IMPLEMENTATIONS

### Need to Find

1. **Working AVC444 server** (any language):
   - How do they encode subframes?
   - Do they use one encoder or two?
   - How do they handle SPS/PPS?
   - What's their reference strategy?

2. **Microsoft's implementation** (if docs exist):
   - Official guidance on encoder setup
   - Reference frame requirements
   - SPS/PPS handling

3. **Any research code**:
   - Wu et al. paper - might have reference implementation
   - Academic projects
   - Open source projects

---

## PART 9: CRITICAL QUESTIONS TO ANSWER

### Q1: What's in DPB After Sequential Encodes?

**After**:
```rust
encoder.encode(&main);  // DPB = [Main]
encoder.encode(&aux);   // DPB = ???
```

**Possibilities**:
- [Aux] (Main evicted) → Main_1 can't ref Main_0!
- [Main, Aux] → Good!
- [Aux, Main] → Order matters?

**How to find out**:
- OpenH264 source code analysis
- Or empirical testing with logging
- Or ask OpenH264 community

---

### Q2: Can We Control Reference Selection?

**Options in OpenH264**:
- LTR (long term reference)
- Reference list modification
- Frame marking (reference vs non-reference)
- Explicit ref index selection?

**What's accessible from openh264-rs?**

---

### Q3: What Does Client Actually Do?

**When client receives**:
```
stream1_data: [SPS1][PPS1][NAL_main]
stream2_data: [SPS2][PPS2][NAL_aux]
```

**Does client**:
- Concatenate and decode as one stream?
- Decode separately with shared DPB?
- Something else?

**How to find out**:
- MS-RDPEGFX spec deeper reading
- FreeRDP client code analysis
- Test with packet capture and analysis

---

## PART 10: MY THEORY (REVISED)

### The Actual Problem

**With single encoder + sequential encodes**:

**Frame 0**:
```
Encode Main → DPB = [Main_0]
Encode Aux → Aux searches DPB for references
  → Finds Main_0 (only thing there)
  → Aux_0 is predicted FROM Main_0!
  → DPB = [Main_0, Aux_0] or just [Aux_0]
```

**Frame 1**:
```
Encode Main → Main searches DPB
  → DPB might have [Aux_0, Main_0] or just [Aux_0]
  → If Main finds Aux_0, predicts from it → CORRUPTION!
  → If Main finds Main_0, OK

Encode Aux → Aux searches DPB
  → Might find Main_1 (just encoded) instead of Aux_0
  → Predicts from wrong stream → CORRUPTION!
```

**The core issue**: **Motion search can select ANY frame in DPB**

**Solution needed**: **Constrain motion search to only select correct stream**

### How to Fix

**Option 1**: Make Aux non-reference
- Main searches DPB, but Aux frames not there
- Main can only find previous Main frames
- Aux... doesn't have refs? Becomes I-frame? Or refs Main?

**Option 2**: Use LTR to constrain search
- Mark Main_0 as LTR slot 0 (pinnned)
- Mark Aux_0 as LTR slot 1 (pinned)
- **Force Main_1 to search only slot 0**
- **Force Aux_1 to search only slot 1**
- Requires API we don't have easy access to

**Option 3**: Separate encoders with synchronized DPB
- Not possible with OpenH264 API

---

## RECOMMENDED IMMEDIATE ACTION

### Test: Disable SPS/PPS Prepending for Aux

**Quick code change** to test SPS/PPS theory:

```rust
// Only prepend to Main, not Aux
stream1_data = self.handle_sps_pps(stream1_data, main_is_keyframe);
// stream2_data = self.handle_sps_pps(stream2_data, aux_is_keyframe);  // SKIP
```

**If this fixes it**: SPS/PPS handling was the issue
**If still corrupts**: Reference selection is the issue

---

## SUMMARY FOR NEXT SESSION

**We have**:
- ✅ Correct architecture (single encoder)
- ✅ Proper configuration (NUM_REF=2, scene change off)
- ✅ Clean API (extended openh264-rs)

**We're missing**:
- ❌ How to prevent cross-stream reference selection
- ❌ Correct SPS/PPS handling for dual subframes
- ❌ Understanding of client decoder expectations

**Next**: Test SPS/PPS theory, then research reference constraint mechanisms

**This analysis should guide the next session's work.**
