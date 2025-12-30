# Critical Research Tasks for Next Session - MUST Solve AVC444 P-Frames

**Priority**: HIGHEST - Commercial solution required
**Goal**: <2 MB/s bandwidth with perfect quality (not 4.3 MB/s all-I)
**Mindset**: No workarounds, complete solution only

---

## THE UNSOLVED PROBLEM

**Aux always produces IDR** (never P-frames)
- Blocks efficient P-frame compression
- Forces either all-I (high bandwidth) or corruption
- **THIS MUST BE SOLVED**

---

## CRITICAL RESEARCH TASK 1: Find Working Implementation (4 hours)

### Search Strategy

**1. FreeRDP Server** (2 hours):
```bash
# Clone FreeRDP if not local
git clone https://github.com/FreeRDP/FreeRDP
cd FreeRDP

# Search for AVC444 encoding
rg "avc444.*compress|avc444.*encode" --type c
rg "YUV444.*split|dual.*stream.*encode" --type c
rg "h264_context.*avc444" --type c

# Examine server/shadow/shadow_encoder.c thoroughly
# Look for how it calls h264_context
# Trace to actual encoding implementation
```

**2. Microsoft Open Source** (1 hour):
- Search GitHub for "microsoft/rdp" or "microsoft/remotedeskop"
- Windows SDK samples
- Any official sample code

**3. Academic/Research Code** (1 hour):
- Wu et al. paper authors - contact for reference implementation
- Search academic repositories
- Conference presentation code

**Goal**: Find ANY code that successfully encodes AVC444 with P-frames

---

## CRITICAL RESEARCH TASK 2: OpenH264 Source Analysis (3 hours)

### If No Working Implementation Found

**Specific analysis needed**:

**A) IDR Insertion Logic** (1 hour):
```bash
cd ~/openh264-rs/openh264-sys2/upstream
# OpenH264 source is here

# Find why IDR is inserted
rg "ForceIntra|bForceIntra|scene.*change|force.*idr" --type cpp

# Examine:
# - encoder_ext.cpp
# - slice decision logic
# - Rate control logic
```

**Goal**: Find EXACT condition causing Aux to be IDR

**B) Temporal Layer Behavior** (1 hour):
```bash
rg "temporal.*layer|iTemporalLayerNum|TemporalId" --type cpp

# Understand:
# - How frames are assigned to layers
# - Whether T1 prevents P-frames or just marks non-ref
# - Any way to force P-frame in T1
```

**Goal**: Understand if temporal layers can give us P-frames

**C) Alternative Encoding Modes** (1 hour):
```bash
rg "simulcast|spatial.*layer|svc" --type cpp

# Check if:
# - Simulcast mode helps
# - Spatial layers could work
# - SVC features relevant
```

**Goal**: Find alternative OpenH264 features that might work

---

## CRITICAL RESEARCH TASK 3: Microsoft Spec Deep Dive (2 hours)

### Re-read MS-RDPEGFX with New Understanding

**Specific questions**:

1. **Does spec require Aux to use P-frames?**
   - Or is Aux-IDR acceptable/expected?
   - Look for frame type requirements

2. **What does "same encoder" REALLY mean?**
   - Same instance?
   - Same configuration?
   - Something else?

3. **Are there encoding examples in spec?**
   - Pseudocode?
   - Bitstream examples?
   - Frame sequence examples?

4. **Client decoder expectations**:
   - How does it handle reference frames?
   - DPB management requirements?
   - Any hints about encoding?

**Read sections**:
- §3.2.9.1.2 RFX_AVC444_BITMAP_STREAM
- §3.3.8.3.2 YUV420p Stream Combination
- Any encoding guidelines sections

---

## CRITICAL RESEARCH TASK 4: OpenH264 Community/Experts (If Needed)

### Structured Questions to Ask

**Post on OpenH264 GitHub Discussions**:

Title: "Encoding dual interleaved streams with single encoder - AVC444 use case"

Body:
```markdown
I'm implementing Microsoft RDP's AVC444 (MS-RDPEGFX spec) which requires:
- Two YUV420 subframes (Main and Auxiliary)
- Encoded using "same H.264 encoder"
- Main contains luma, Aux contains additional chroma

Currently using:
- One OpenH264 encoder instance
- Sequential encode() calls: Main, Aux, Main, Aux...
- NUM_REF=2, temporal_layers=2

Problem:
- Main produces P-frames correctly
- Aux ALWAYS produces IDR (never P-frames)
- Even with scene_change_detect=false, temporal_layers=2

Questions:
1. Is there a way to force P-frames for content very different from previous frame?
2. Do temporal layers prevent P-frames or just mark them non-reference?
3. Is there an encoder mode for dual-stream encoding?
4. Should Aux always be IDR in this use case?

Any guidance appreciated!
```

**Contact FreeRDP developers similarly**

**Contact Wu et al.** (original AVC444 researchers):
- Find their contact info from paper
- Ask for reference implementation or guidance

---

## DECISION TREE FOR NEXT SESSION

```
Start
  │
  ├─> Found working code?
  │     YES: Study it, implement same approach → DONE
  │     NO: Continue
  │
  ├─> OpenH264 source reveals solution?
  │     YES: Apply configuration/approach → TEST
  │     NO: Continue
  │
  ├─> Spec reveals we misunderstood?
  │     YES: Correct implementation → TEST
  │     NO: Continue
  │
  └─> Expert response with guidance?
        YES: Follow guidance → IMPLEMENT
        NO: Consider fundamental alternatives
```

---

## ALTERNATIVE APPROACHES (If Above Fails)

### If Aux-IDR is INTENDED

**Then we need**: Way to prevent Aux-IDR from contaminating Main DPB

**Options**:
1. **Two encoders with LTR synchronization**: Complex but might work
2. **Client-side handling**: Modify how client processes (not ideal)
3. **Different codec for Aux**: Not H.264?

### If No Solution Exists

**Then**:
1. **Accept all-I as state of art**: Maybe no one has solved this
2. **Optimize all-I**: Make it as efficient as possible
3. **Alternative to AVC444**: Different approach to 4:4:4

**But**: Don't conclude this until exhaustive research done!

---

## TIME BUDGET FOR NEXT SESSION

**Total**: 8-10 hours dedicated research and implementation

**Breakdown**:
- Working code search: 4 hours
- OpenH264 source: 3 hours
- Spec deep dive: 2 hours
- Expert consultation: 1 hour (post questions, wait for response)

**Goal**: Either SOLVE it or definitively understand why it's unsolved

---

## SUCCESS CRITERIA

**NOT**: "Stable workaround working"
**YES**: "AVC444 P-frames at <2 MB/s with perfect quality"

**Acceptable**: "Exhaustive research proves it's unsolvable, here's why"
**Not acceptable**: "Gave up, using workaround"

---

## READY FOR NEXT SESSION

**Current state**: Stable all-I verified
**Next priority**: SOLVE Aux-IDR → Aux-P problem
**Approach**: Research-first, implement with certainty
**Mindset**: Commercial quality, no compromises

**Start here next session**: This document

**Sources for initial research**:
- [FreeRDP AVC444 Issue](https://github.com/FreeRDP/FreeRDP/issues/11040)
- [MS-RDPEGFX Spec](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx)
- [OpenH264 Repository](https://github.com/cisco/openh264)

**The goal is CLEAR: Full working AVC444 with P-frames.**
