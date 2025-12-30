# ULTRA-RESEARCH: Reference Frame Marking in H.264 and OpenH264

**Date**: 2025-12-29 04:30 UTC
**Purpose**: EXHAUSTIVE research on how to make Aux frames non-reference
**Method**: H.264 spec, OpenH264 source, working implementations, all options

---

## PART 1: H.264 SPECIFICATION - nal_ref_idc

### What nal_ref_idc Means

**From H.264 Spec (ITU-T H.264 §7.4.1)**:

`nal_ref_idc`: 2-bit field in NAL unit header

**Values**:
- `0`: NON-REFERENCE - Frame is NOT used for prediction of other frames
- `1-3`: REFERENCE - Frame MAY be used for prediction (priority levels)

**Requirements** ([RFC 6184](https://datatracker.ietf.org/doc/html/rfc6184)):
> "nal_ref_idc equal to 0 for a NAL unit containing a slice indicates that the slice is part of a non-reference picture."

> "When nal_ref_idc is equal to 0 for one slice NAL unit of a picture, it shall be equal to 0 for all slice NAL units of the picture."

**Key constraints**:
- `nal_ref_idc` MUST be >0 for IDR frames
- `nal_ref_idc` MUST be 0 for all slices if one slice has it
- Entire picture is either reference or non-reference (no mixing)

---

## PART 2: HOW ENCODERS SET nal_ref_idc

### Standard Encoder Logic

**From OpenH264 source** (`encoder_ext.cpp`):
```cpp
pNalHd->uiNalRefIdc = pCtx->eNalPriority;
```

**eNalPriority is determined by**:
1. **Frame type**:
   - IDR: Always NRI_PRI_HIGHEST (3)
   - I-frame: NRI_PRI_HIGH (2-3)
   - P-frame: NRI_PRI_HIGH or MEDIUM (1-2)
   - B-frame: NRI_PRI_LOWEST (0) for non-reference B

2. **Temporal layer**:
   - Base layer (temporal_id=0): Reference
   - Higher layers (temporal_id>0): Often non-reference

3. **Long-term reference settings**:
   - LTR frames: Reference
   - Non-LTR: Depends on config

**Default for P-frames**: Usually marked as reference (nal_ref_idc=2)

---

## PART 3: OPENH264 API RESEARCH

### Exposed Options (From codec_app_def.h)

**Reference-related options**:
```c
ENCODER_OPTION_NUMBER_REF     // iNumRefFrame - DPB size
ENCODER_OPTION_LTR            // Long-term reference enable
ENCODER_LTR_MARKING_PERIOD    // LTR marking period
```

**NOT exposed**:
- Direct nal_ref_idc control
- Per-frame reference/non-reference marking
- Frame disposability hints

**Conclusion**: OpenH264 does NOT expose API to mark arbitrary frames as non-reference

---

## PART 4: POSSIBLE APPROACHES

### Approach A: Post-Process NAL Headers

**Concept**: After encoding, modify NAL headers to set nal_ref_idc=0

```rust
fn make_non_reference(mut bitstream: Vec<u8>) -> Vec<u8> {
    // Find all NAL units in bitstream
    // For each NAL unit header byte:
    //   header = (nal_ref_idc << 5) | nal_type
    //   Set nal_ref_idc bits to 0
    //   header_new = (0 << 5) | nal_type

    // Modify in place
}
```

**Pros**:
- ✅ Direct control
- ✅ Works with any encoder
- ✅ Simple to implement

**Cons**:
- ⚠️ Encoder's DPB still contains the frame (encoder-side)
- ⚠️ Only affects decoder-side (client)
- ⚠️ **Encoder-decoder DPB mismatch!**
- ⚠️ Next encode() still uses this frame internally

**CRITICAL ISSUE**: This creates mismatch:
- Encoder thinks frame is reference → uses it for prediction
- Decoder gets nal_ref_idc=0 → doesn't store in DPB
- **Encoder and decoder DPBs diverge** → undefined behavior!

**Verdict**: **DANGEROUS - Do not use**

---

### Approach B: Temporal Layering

**Concept**: Use OpenH264's temporal layers, make Aux higher temporal layer

**H.264 Temporal Layers**:
- Layer 0 (base): Always reference
- Layer 1+: Can be non-reference, droppable

```rust
// Configure encoder for 2 temporal layers
params.iTemporalLayerNum = 2;

// Somehow hint that Aux is temporal layer 1 (non-reference)
```

**Challenges**:
- OpenH264's temporal layers are for scalability (frame rate)
- Not designed for "alternate content" like Main/Aux
- Might not work semantically
- API unclear

**Research needed**: Can we abuse temporal layers for this?

**Verdict**: **Uncertain - needs deeper investigation**

---

### Approach C: Exploit OpenH264 Internals via Raw API

**From OpenH264 source**: `eNalPriority` is set based on internal logic

**Could we**:
- Access encoder context directly (very unsafe)
- Modify eNalPriority before encode() call
- Set it to NRI_PRI_LOWEST (0) for Aux frames

**Challenges**:
- Requires deep FFI into OpenH264 internals
- Not exposed through ISVCEncoder interface
- Extremely fragile
- Would break with OpenH264 updates

**Verdict**: **Too dangerous and fragile**

---

### Approach D: Separate Encoders with Coordinated Frame Numbering

**Concept**: Use two encoders but manipulate their state to share DPB conceptually

**Not possible because**:
- Can't sync DPB state between encoder instances
- No API to clone/share DPB
- This is what we tried originally (failed)

**Verdict**: **Not feasible**

---

### Approach E: Accept Main-P + Aux-IDR Configuration

**Observation from test**: Main-P + Aux-IDR still corrupts!

**But wait**: Aux is IDR (nal_ref_idc must be 3 for IDR per spec)

**So Aux IS in DPB as reference** even though it's IDR!

**The corruption happens because**:
- Aux-IDR is in DPB as reference
- Main P-frame can reference it
- **IDR doesn't mean non-reference!**

**We still need**: Aux to be NON-REFERENCE

**But IDR can't be non-reference** (spec violation)

**Implication**: **We can't use Aux-IDR + Main-P if Aux enters DPB!**

---

### Approach F: Use B-Frames for Aux

**Concept**: B-frames can be non-reference

**H.264 B-frames**:
- Can be marked as non-reference (nal_ref_idc=0)
- Used for prediction but don't enter DPB permanently
- Droppable

**Could we**:
- Encode Aux as B-frames?
- B-frames can be non-reference
- Main uses P-frames

**Challenges**:
- B-frames require BOTH past and future references
- OpenH264 in screen content mode might not use B-frames
- Complexity

**Research needed**: Can OpenH264 produce non-reference B-frames?

**Verdict**: **Possible but complex**

---

### Approach G: Modify NAL Headers BUT Also Tell Encoder

**Two-step approach**:

**Step 1**: Before encoding Aux, tell encoder "this is disposable"
- Might be possible through complexity mode?
- Or some hidden parameter?

**Step 2**: Verify nal_ref_idc=0 in output, or force it

**This avoids encoder-decoder DPB mismatch**

**Research needed**: Is there ANY encoder hint for disposability?

---

## PART 5: EXAMINATION OF THE BREAKTHROUGH

### What the Intermittent Pattern Tells Us

**Clean frames** (Both Main-IDR + Aux-IDR):
- Both are reference frames (nal_ref_idc=3)
- Both in DPB
- But Main doesn't NEED references (IDR is self-contained)
- **Displays perfectly**

**Corrupt frames** (Main-P + Aux-IDR):
- Main is P-frame (needs reference)
- Aux is IDR (nal_ref_idc=3, IS reference)
- **Main searches DPB, finds Aux, uses it → CORRUPTION!**

**The key**: Main's motion search is picking Aux frames from DPB!

---

## PART 6: WHY MOTION SEARCH PICKS AUX

### DPB Contents After Interleaved Encoding

```
After Main_0 IDR encode: DPB = [Main_0]
After Aux_0 IDR encode:  DPB = [Main_0, Aux_0]

When encoding Main_1 P-frame:
  Motion search scans DPB = [Main_0, Aux_0]
  For each macroblock:
    - Searches Main_0 → Calculate SAD (sum of absolute differences)
    - Searches Aux_0 → Calculate SAD
    - Picks LOWEST SAD
```

**If Aux_0 happens to have pixel patterns similar to Main_1**:
- Aux_0 SAD might be lower than Main_0 SAD
- Motion search picks Aux_0 as reference
- **Predicts Main_1 from Aux_0 → color corruption!**

**Why this can happen**:
- Aux contains chroma values (U/V in 0-255 range)
- Main contains luma values (Y in 0-255 range)
- Some pixel patterns might coincidentally match
- Motion search doesn't "know" they're different semantic content

---

## PART 7: POTENTIAL SOLUTIONS THAT MIGHT ACTUALLY WORK

### Solution A: Force Encoder to Skip Certain References

**Research needed**: Does OpenH264 have ANY mechanism to exclude specific frames from motion search?

**Looked for**:
- Reference list modification options
- Search range per-frame hints
- Any "exclude from search" flags

**Status**: NOT FOUND in public API

---

### Solution B: Use Temporal Scalability Creatively

**OpenH264 temporal layers**:
- Can have up to 4 temporal layers
- Higher layers are non-reference by default
- Designed for frame rate scalability

**Creative abuse**:
```rust
// Configure 2 temporal layers
params.iTemporalLayerNum = 2;

// Encode Main as layer 0 (reference)
// Encode Aux as layer 1 (non-reference)
```

**Challenge**: How to specify which layer per encode() call?

**Research shows**: Temporal layer is usually determined by frame number pattern
- Even frames: Layer 0
- Odd frames: Layer 1
- etc.

**With our alternating pattern**: Main-Aux-Main-Aux...
- Frame 0 (Main): Layer 0 (reference) ✓
- Frame 1 (Aux): Layer 1 (non-reference) ✓
- Frame 2 (Main): Layer 0 (reference) ✓
- Frame 3 (Aux): Layer 1 (non-reference) ✓

**This could work!**

---

### Solution C: Post-Process NAL Headers (Carefully)

**Despite dangers**, if we ALSO configure encoder correctly:

**Idea**: Configure encoder for B-frames or temporal layers, THEN also post-process

**But**: We don't have B-frame or temporal layer control easily

---

### Solution D: Accept All-I Workaround as Production Solution

**Observation**: All-I works PERFECTLY

**Bandwidth**: ~4.3 MB/s at 1280x800@30fps

**Is this acceptable**?
- For local network: Probably yes
- For internet: Maybe too high
- For production: Depends on use case

**Could optimize**:
- Adaptive quality based on bandwidth
- Periodic all-I with some P-frames
- Lower frame rate for high quality

---

## PART 8: DEEP DIVE INTO TEMPORAL LAYERS

### How OpenH264 Temporal Scalability Works

**From OpenH264 wiki and source**:

**Temporal layers**:
- Layer 0: Base layer (cannot be dropped, always reference)
- Layer 1+: Enhancement layers (can be dropped, often non-reference)

**Frame pattern for 2 layers**:
```
0 1 2 3 4 5 6 7 ...  (encode order)
T T T T T T T T      (temporal layer)
0 1 0 1 0 1 0 1      (pattern)

Layer 0 (0,2,4,6...): Reference frames
Layer 1 (1,3,5,7...): Non-reference frames (display-only)
```

**For our case**:
```
Main Aux Main Aux Main Aux ...  (our encoding order)
 0    1    2    3    4    5     (frame number)
 0    1    0    1    0    1     (temporal layer if 2-layer config)

Main (even): Layer 0 (reference)
Aux (odd): Layer 1 (NON-REFERENCE!) ✓✓✓
```

**This is PERFECT!** Even-odd pattern naturally maps to our Main-Aux pattern!

### How to Enable Temporal Layers

**In SEncParamExt**:
```c
iTemporalLayerNum = 2;  // Enable 2 temporal layers
```

**OpenH264 automatically**:
- Assigns frames to layers based on encoding order
- Marks layer 1 frames as non-reference
- Layer 0 frames are reference

**This should**:
- Make Aux (odd frames) non-reference
- Keep Main (even frames) as reference
- **Solve our problem!**

---

## PART 9: RESEARCH ON IMPLEMENTING TEMPORAL LAYERS

### Check if openh264-rs Exposes Temporal Layers

**Search needed**: Does EncoderConfig have temporal layer support?

**If not**: Need to add it (like VUI and NUM_REF)

**Addition would be**:
```rust
pub struct EncoderConfig {
    // ...
    temporal_layers: Option<i32>,
}

impl EncoderConfig {
    pub const fn temporal_layers(mut self, num: i32) -> Self {
        self.temporal_layers = Some(num.clamp(1, 4));
        self
    }
}

// In with_api_config():
if let Some(num) = self.config.temporal_layers {
    params.iTemporalLayerNum = num;
}
```

---

## PART 10: ALTERNATIVE INTERPRETATION - REVIEW THE LOGS MORE CAREFULLY

### Wait - Let Me Re-examine the Frame Type Logs

**From logs**:
```
Frame #7:  Main: P (18KB), Aux: IDR (94KB)
Frame #8:  Main: P (20KB), Aux: IDR (95KB)
```

**Frame #33-34**: Both IDR (clean)
**Frame #35**: Main: P, Aux: IDR (corrupt)
**Frame #38**: Both IDR (clean)

**Pattern**: When Main resets to IDR, corruption clears!

**This confirms**: Main P-frames are the issue

**But**: Why? With NUM_REF=2, Main should reference previous Main!

### Possible Explanation: DPB Eviction Policy

**With NUM_REF=2**:

```
Encode Main_0 IDR → DPB = [Main_0] (slot 0)
Encode Aux_0 IDR  → DPB = [Main_0, Aux_0] (slots 0, 1)

Encode Main_1 P:
  DPB BEFORE: [Main_0, Aux_0]
  Search in DPB: Finds both
  If picks Aux_0: CORRUPTION!
  DPB AFTER: [Aux_0, Main_1] (Main_0 evicted??) or [Main_0, Aux_0, Main_1]?

Encode Aux_1 IDR:
  DPB: ???
```

**If DPB is FIFO with size 2**:
- Always evicts oldest
- After Main_1, DPB might be [Aux_0, Main_1]
- Main_0 is gone!
- Main_2 can't reference Main_0!

**If DPB is size 3+**:
- Might keep all
- But then which does Main_2 reference?

**Maybe NUM_REF=2 is actually the problem!**
- Too small for our interleaving pattern
- Need NUM_REF=4 or higher?

---

## PART 11: CONCRETE TESTS TO RUN

### Test 1: NUM_REF=4

**Quick test**: Increase DPB size

```rust
.num_ref_frames(4)  // Keep more frames
```

**Theory**: Larger DPB = less eviction = Main frames stay around longer

**If this fixes it**: DPB size was the issue, not ref marking

---

### Test 2: Temporal Layers (2 layers)

**If openh264-rs supports it** (or extend it):

```rust
.temporal_layers(2)  // Aux becomes layer 1 (non-ref)
```

**Theory**: Aux marked as non-reference automatically

**If this fixes it**: Temporal layers was the solution

---

### Test 3: Force Both IDR for N Frames, Then Try P

**Diagnostic test**:

```rust
if self.frame_count < 100 {
    // Force both IDR for first 100 frames
    self.encoder.force_intra_frame();  // Before Main
    self.encoder.force_intra_frame();  // Before Aux
} else {
    // After 100 frames, try P-frames
    // See if corruption appears immediately or gradually
}
```

**This tells us**: Does corruption accumulate or appear instantly?

---

## PART 12: EXAMINE IRONRDP-EGFX MORE CAREFULLY

### How Are Subframes Actually Sent?

**Our code** (`egfx_sender.rs:475-482`):
```rust
server.send_avc444_frame(
    surface_id,
    stream1_data,      // Main
    &luma_regions,
    Some(stream2_data), // Aux
    Some(&chroma_regions),
    timestamp_ms,
)
```

**IronRDP creates** (`ironrdp-egfx/server.rs:1179-1186`):
```rust
let avc444_stream = Avc444BitmapStream {
    encoding: LUMA_AND_CHROMA,  // LC = 0x0
    stream1: Avc420BitmapStream {
        rectangles: luma_rectangles,
        data: luma_data,  // stream1_data
    },
    stream2: Some(Avc420BitmapStream {
        rectangles: chroma_rectangles,
        data: chroma,  // stream2_data
    }),
};
```

**Then encoded** via `encode_avc444_bitmap_stream()`

**Question**: Does IronRDP concatenate the bitstreams or send them separately?

**Need to examine**: The actual wire format produced

---

## PART 13: EXAMINE MS-RDPEGFX WIRE FORMAT

### What Actually Goes Over the Wire

**From MS-RDPEGFX spec**:

**RFX_AVC444_BITMAP_STREAM structure**:
```
LC (2 bits)
avc420EncodedBitstream1 (variable)  // Length-prefixed
avc420EncodedBitstream2 (variable)  // Length-prefixed if present
```

**Each avc420EncodedBitstream is**:
```
RFX_AVC420_BITMAP_STREAM:
  - regionRects[]
  - quantVals[]
  - data (H.264 bitstream)
```

**Key insight**: They're sent as SEPARATE byte arrays with LENGTH prefixes!

**Client receives**: Two distinct byte arrays
**Client must**: Decode them as "one stream"

**How does client unify them?**
- Concatenate before decoding?
- Feed to decoder sequentially?
- Something else?

**THIS IS WHAT WE DON'T UNDERSTAND!**

---

## PART 14: CRITICAL REALIZATION

### The Spec Says "Decoded as One Stream"

**But doesn't specify HOW!**

**Possibility 1**: Client concatenates:
```
decoder.decode(stream1 + stream2)
```

**Possibility 2**: Client decodes sequentially with shared state:
```
decoder.decode(stream1)  // Updates DPB
decoder.decode(stream2)  // Uses same DPB
```

**Possibility 3**: Something else entirely

**We're guessing!** And maybe guessing wrong!

---

## PART 15: WHAT WE SHOULD RESEARCH FROM WORKING CODE

### FreeRDP Client Decoder

**Location**: FreeRDP client-side AVC444 decoding

**What to find**:
1. How does it receive the two bitstreams?
2. Does it concatenate them?
3. Does it decode separately?
4. How does it manage DPB?

**This would tell us EXACTLY what client expects!**

---

### Microsoft RDP Client Behavior

**If we could**:
- Packet capture a working AVC444 session
- Analyze bitstream structure
- See how Microsoft server encodes it
- Compare with our output

**This would show us the "gold standard"**

---

## PART 16: COMPREHENSIVE FINDINGS

### What We Know For Certain

✅ Single encoder is necessary (spec requirement)
✅ Intermittent corruption pattern: Clean when Main-IDR, corrupt when Main-P
✅ Aux type doesn't matter (always IDR anyway)
✅ Main P-frames are predicting from wrong source
✅ NUM_REF=2 configured but might not be enough

### What We Don't Know

❌ How client actually decodes "as one stream"
❌ How to prevent Main from referencing Aux in DPB
❌ Whether temporal layers would work
❌ What the "correct" reference pattern should be
❌ How Microsoft's encoder handles this

### What We Should Try Next (Prioritized)

**Priority 1**: NUM_REF=4 or higher
- Simplest test
- Might keep Main frames around longer
- 15 minutes to implement and test

**Priority 2**: Research FreeRDP client decoder
- Understand EXACTLY how it processes subframes
- See what it expects
- 1-2 hours of code reading

**Priority 3**: Temporal layers investigation
- Check if openh264-rs supports it
- Extend if needed
- Test if Aux becomes non-reference
- 2-3 hours

**Priority 4**: Consult experts
- FreeRDP developers
- Microsoft if possible
- OpenH264 community
- Could get authoritative answer

---

## MY RECOMMENDATION

### Immediate Action: Try NUM_REF=4

**Rationale**:
- Simplest test (one line change)
- Might solve it if DPB eviction was the issue
- 15 minutes total

**If successful**: Problem was DPB size, not ref marking

**If fails**: Then invest in temporal layers research or expert consultation

**Change**:
```rust
.num_ref_frames(4)  // Instead of 2
```

Shall I do this quick test, or do you want me to research temporal layers first?

**Sources**:
- [RFC 6184 - H.264 RTP Payload](https://datatracker.ietf.org/doc/html/rfc6184)
- [H.264 nal_ref_idc](https://yumichan.net/video-processing/video-compression/breif-description-of-nal_ref_idc-value-in-h-246-nalu/)
- [H.264 Picture Management](https://www.vcodex.com/h264avc-picture-management/)