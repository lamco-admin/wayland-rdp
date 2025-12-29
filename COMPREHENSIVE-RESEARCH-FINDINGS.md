# Comprehensive Research Findings - Reference Marking Solutions

**Date**: 2025-12-29 04:40 UTC
**Status**: Exhaustive research complete
**Confidence**: Multiple viable paths identified

---

## EXECUTIVE SUMMARY

### The Core Problem (Proven by Intermittent Corruption)

**When**: Main uses P-frames + Aux in DPB
**What**: Main's motion search references Aux frames
**Result**: Wrong prediction base → lavender corruption

**When**: Main uses IDR (frames 33-34, 38, 59, 73-77, 88-91)
**What**: No prediction needed
**Result**: Perfect display (user confirmed)

### Solutions Found (In Order of Viability)

1. **Increase NUM_REF** (85% confidence, 15 min)
2. **Temporal layers** (75% confidence, 2-3 hrs)
3. **Accept all-I workaround** (100% confidence, 0 min - already working)

---

## SOLUTION 1: Increase NUM_REF to 4-8 (RECOMMENDED FIRST TEST)

### Theory

**Current**: NUM_REF=2 (DPB holds 2 frames)
**Problem**: With Main-Aux alternating, eviction pattern might be wrong

**Encoding sequence with NUM_REF=2**:
```
Main_0 → DPB = [Main_0]          (slot 0)
Aux_0  → DPB = [Main_0, Aux_0]   (slots 0, 1)
Main_1 → DPB = [Aux_0, Main_1]   (Main_0 EVICTED! Can't ref it!)
```

**With NUM_REF=4**:
```
Main_0 → DPB = [Main_0]
Aux_0  → DPB = [Main_0, Aux_0]
Main_1 → DPB = [Main_0, Aux_0, Main_1]  (Main_0 KEPT!)
Aux_1  → DPB = [Main_0, Aux_0, Main_1, Aux_1]
Main_2 → DPB has Main_0 and Main_1 available ✓
```

**Or with NUM_REF=8**: Even safer, keeps more history

### Implementation

**One line change**:
```rust
.num_ref_frames(4)  // Or 8
```

**Test**: 15 minutes
**Risk**: None (just more memory)
**Confidence**: 85% - DPB eviction might be the issue

**Why this is strong**:
- Simple
- Low risk
- Addresses a real possibility
- OpenH264's default might be 1, we set 2, but need more

---

## SOLUTION 2: Temporal Layers (If NUM_REF doesn't work)

### How Temporal Scalability Works

**H.264 temporal scalability**:
- Multiple temporal layers for frame rate scalability
- Base layer (T0): Always reference, cannot be dropped
- Enhancement layers (T1+): Non-reference, droppable

**Frame assignment** (with 2 layers):
```
Frame#: 0   1   2   3   4   5   6   7
Layer:  T0  T1  T0  T1  T0  T1  T0  T1
Ref:    YES NO  YES NO  YES NO  YES NO

T0 (even): Reference frames
T1 (odd): Non-reference frames
```

**For our Main-Aux pattern**:
```
Encode: Main Aux Main Aux Main Aux
Frame#:  0    1    2    3    4    5
Layer:   T0   T1   T0   T1   T0   T1
Ref:     YES  NO   YES  NO   YES  NO

Main (even): Reference ✓
Aux (odd): NON-REFERENCE ✓✓✓
```

**PERFECT MATCH!**

### Implementation

**Extend openh264-rs** (currently hardcoded to 1 layer):

**File**: `openh264/src/encoder.rs:1061`

**Current**:
```rust
params.iTemporalLayerNum = 1;  // Hardcoded
```

**Add to EncoderConfig**:
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
params.iTemporalLayerNum = self.config.temporal_layers.unwrap_or(1);
```

**Usage**:
```rust
let config = OpenH264Config::new()
    .num_ref_frames(2)  // Keep both T0 and T1 in DPB
    .temporal_layers(2)  // Enable T0/T1 pattern
    .scene_change_detect(false);
```

**OpenH264 will automatically**:
- Assign even frames (Main) to T0 (reference)
- Assign odd frames (Aux) to T1 (non-reference)
- Set nal_ref_idc=0 for T1 frames

**Result**: Aux won't contaminate Main's DPB!

**Confidence**: 75%
**Time**: 2-3 hours (extend openh264-rs, test)

---

## SOLUTION 3: Production Fallback (All-I)

**Current working**: Both Main and Aux use IDR
**Quality**: Perfect (user verified extensively)
**Bandwidth**: ~4.3 MB/s

**Optimizations possible**:
- Adaptive quality
- Periodic P-frames (every Nth frame)
- Lower frame rate for high quality modes

**This is always available** if other solutions don't work

---

## DECISION TREE

```
Start
  │
  ├─> Test NUM_REF=4
  │     │
  │     ├─> Works? ✅
  │     │     └─> DONE! Document and commit
  │     │
  │     └─> Fails? ❌
  │           └─> Test NUM_REF=8
  │                 │
  │                 ├─> Works? ✅
  │                 │     └─> DONE!
  │                 │
  │                 └─> Fails? ❌
  │                       └─> Implement Temporal Layers
  │                             │
  │                             ├─> Works? ✅
  │                             │     └─> DONE!
  │                             │
  │                             └─> Fails? ❌
  │                                   └─> Accept All-I or Consult Experts
```

---

## WHY START WITH NUM_REF

### Arguments For

**1. Simplest**: One line change
**2. Fastest**: 15 minutes total
**3. Likely**: DPB eviction is a real possibility
**4. Safe**: No downside to trying
**5. Informative**: Rules out or confirms a major hypothesis

### Arguments Against

**None really** - it's a 15-minute test with high information value

---

## WHY TEMPORAL LAYERS IS STRONG BACKUP

### Perfect Semantic Match

**Main-Aux-Main-Aux pattern** naturally maps to **T0-T1-T0-T1**

**T1 frames are non-reference by design** - exactly what we need!

**OpenH264 handles it automatically** - no manual control needed

**Implementation**: Clean API extension (like NUM_REF)

### Why Not First?

**More complex** (requires extending openh264-rs)
**Takes longer** (2-3 hours vs 15 min)
**Might not be needed** if NUM_REF works

**But**: High confidence if needed (75%)

---

## RECOMMENDATION

**Phase 1**: Test NUM_REF=4 (15 min)
- Quick, safe, informative
- If works: DONE!

**Phase 2**: If fails, try NUM_REF=8 (5 min)
- Rule out DPB size completely

**Phase 3**: If still fails, implement Temporal Layers (2-3 hrs)
- Clean solution
- High confidence
- Matches our pattern perfectly

**Phase 4**: If all fail, either:
- Accept all-I (proven working)
- Or consult Microsoft/FreeRDP experts

---

## READY TO PROCEED

**Ultra-research complete**: All options analyzed

**Recommendation**: Start with NUM_REF=4 test

**Confidence levels**:
- NUM_REF=4-8: 85% might fix it
- Temporal layers: 75% will fix it
- All-I fallback: 100% works

**Should I proceed with NUM_REF=4 test?**

**Sources**:
- [RFC 6184 H.264 RTP Payload](https://datatracker.ietf.org/doc/html/rfc6184)
- [H.264 Picture Management](https://www.vcodex.com/h264avc-picture-management/)
- [H.264 nal_ref_idc Explanation](https://yumichan.net/video-processing/video-compression/breif-description-of-nal_ref_idc-value-in-h-246-nalu/)
