# CRITICAL FINDING: Auxiliary Subframes Always Encoded as IDR

**Date**: 2025-12-29 03:30 UTC
**Binary MD5**: `f592bd98e203527a03da776e5947ffb7`
**Finding**: Aux never uses P-frames, always IDR!

---

## ANALYSIS FROM NAL LOGS

### Frame Type Pattern

```
Frame #0:  Main: IDR (80KB), Aux: IDR (91KB)
Frame #1:  Main: IDR (80KB), Aux: IDR (74KB)
Frame #2:  Main: IDR (84KB), Aux: IDR (97KB)
...
Frame #7:  Main: IDR (82KB), Aux: IDR (94KB)
Frame #8:  Main: P (35KB),   Aux: IDR (96KB)  ← Main switches to P
Frame #9:  Main: P (20KB),   Aux: IDR (100KB) ← Aux STILL IDR
Frame #10: Main: P (28KB),   Aux: IDR (64KB)  ← Aux STILL IDR
...
Frame #39: Main: P (21KB),   Aux: IDR (91KB)  ← Aux NEVER becomes P!
```

**Pattern**:
- Main: IDR for 0-7, then P-frames from frame 8 onward ✓
- Aux: **ALWAYS IDR**, never P-frames ❌

### NAL Reference Marking

**Aux subframes**:
```
[Frame #1 AUX NAL#0] type= 1 (P-slice) ref_idc=3 (REFERENCE(3))
```

Wait - this says "P-slice" but frame type log says "IDR". Let me check more carefully...

**Actually from frame type log**:
```
[AVC444 Frame #1] Main: IDR, Aux: IDR
```

**But NAL log shows**:
```
[Frame #1 AUX NAL#0] type= 1 (P-slice)
```

**Contradiction!** NAL type=1 IS a P-slice, but frame_type() returns IDR?

---

## HYPOTHESIS: Encoder State Issue

### Possible Causes

**H1: Scene Change Detection**:
- After encoding Main, encoder detects Aux as "completely different scene"
- Automatically inserts IDR for Aux
- Even though we didn't call force_intra_frame()

**H2: Encoder Reset Between Calls**:
- Something resets encoder state after Main encode
- Causes next encode (Aux) to be IDR
- Might be in openh264-rs or OpenH264 itself

**H3: Dimension or Format Change Detection**:
- Even though both are 1280x800 YUV420
- Maybe encoder thinks format changed?
- Forces IDR on "format change"

**H4: Auto-Keyframe Insertion**:
- OpenH264 has automatic keyframe insertion logic
- Might trigger for some reason on Aux

---

## OPENH264 PARAMETERS TO CHECK

### Scene Change Detection

```rust
params.bEnableSceneChangeDetect = true;  // Default in openh264-rs
```

**If this is enabled**:
- Encoder compares current frame to reference
- If difference > threshold → inserts IDR automatically
- **Aux is completely different from Main** → could trigger!

**Test**: Disable scene change detection for aux

### Intra Period

```rust
params.uiIntraPeriod = 0;  // Default (0 = no periodic IDR, only first frame)
```

**Should be OK** - 0 means only first frame is IDR

### Adaptive Quantization / Background Detection

```rust
params.bEnableAdaptiveQuant = true;
params.bEnableBackgroundDetection = true;
```

**Could these affect frame type decisions?**

---

## THE SMOKING GUN OBSERVATION

Looking at Main:
- Frames 0-7: All IDR
- Frame 8+: P-frames

**This is 8 frames** = likely OpenH264's internal warmup/stabilization period

**But Aux NEVER transitions to P!**

**This strongly suggests**: Something about Aux content triggers auto-IDR insertion every single time.

**Most Likely**: **Scene change detection** sees Aux as "completely different" from Main (which is true - different data entirely) and forces IDR.

---

## SOLUTION: Disable Scene Change Detection

### For Single Encoder Approach

**Problem**: Can't disable scene change for just Aux (one encoder, shared config)

**Options**:

**A) Disable for both**:
```rust
let config = OpenH264Config::new()
    .num_ref_frames(2)
    .scene_change_detect(false);  // Disable auto-IDR
```

**B) Increase scene change threshold** (if exposed)

**C) Use two encoders again but with coordinated DPB** (complex, probably wrong)

---

## NEXT TEST

**Disable scene change detection**:

1. Check if openh264-rs exposes scene_change_detect setter
2. If not, extend it (like we did for NUM_REF)
3. Test with scene change disabled
4. See if Aux now uses P-frames

---

## WHY THIS EXPLAINS THE CORRUPTION

**Aux as IDR every frame**:
- Aux doesn't compress (always full frame)
- Bandwidth: ~70-100KB per aux frame (no compression)
- This is actually SIMILAR to our all-I workaround!

**But why corruption?**:
- Main uses P-frames (compresses well)
- Aux uses IDR (no compression but no corruption risk)
- **Maybe we DIDN'T have corruption with this combination?**

**User said**: "perhaps not immediately"

**Possibility**: Corruption might have been LESS than before, or different pattern?

---

## CLARIFICATION NEEDED

**Question for user**:
1. Was corruption SAME as dual-encoder P+P test?
2. Or was it DIFFERENT/LESS than before?
3. When did you notice it - right away or after some time?

This matters because:
- If **same as before**: Scene change detection not helping
- If **less than before**: Main-P + Aux-IDR is partial solution
- If **delayed onset**: Reference drift issue

---

## RECOMMENDATION

**Next test**: Disable scene change detection

Let's see if that allows Aux to use P-frames like Main does.

Then we'll truly test "both subframes using P-frames with single encoder + multi-ref DPB".
