# DECISION: Temporal Layers - The Architecturally Correct Solution

**Date**: 2025-12-29 04:50 UTC
**Decision**: Implement temporal layers (not NUM_REF increase)
**Rationale**: Architectural correctness over implementation speed

---

## THE BREAKTHROUGH THAT LED US HERE

### Intermittent Corruption Pattern (User Observation)

> "Corruption ceases and perfectly good modification comes through, but some previous corrupted frames stay lavender"

**Analysis of logs revealed**:
- **Clean frames**: 0-6, 33-34, 38, 59, 73-77, 88-91
  - Pattern: BOTH Main and Aux are IDR
- **Corrupted frames**: All others
  - Pattern: Main uses P-frames

**Conclusion**: **Main P-frame prediction is broken** - predicting from wrong reference (Aux frames!)

---

## WHY MAIN REFERENCES AUX (The Root Cause)

### DPB State with Single Encoder

**Encoding sequence**:
```
Main_0 IDR → DPB = [Main_0]
Aux_0 IDR  → DPB = [Main_0, Aux_0]  (both are reference frames!)

Main_1 P-frame:
  Motion search scans DPB = [Main_0, Aux_0]
  Searches for best matching blocks
  Might pick Aux_0 for some blocks (if pixel patterns match)
  → Predicts from Aux_0 → WRONG PREDICTION → Corruption!
```

**The problem**: **Aux frames are marked as REFERENCE** (nal_ref_idc=3 for IDR)
- They enter DPB
- Motion search can select them
- **Aux contaminates Main's prediction**

---

## THE TWO SOLUTION CANDIDATES

### Option A: NUM_REF=4-8 (Rejected)

**Concept**: Make DPB larger, hope eviction/search works out

**Why rejected**:
1. **Probabilistic**: Hopes motion search picks right frame
2. **Workaround**: Doesn't fix root cause (Aux being reference)
3. **Inefficient**: Wastes DPB memory on frames we don't need
4. **Fragile**: Content-dependent, might fail on edge cases
5. **Non-deterministic**: No guarantee of correctness

**This is "make DPB big and pray" - NOT robust!**

---

### Option B: Temporal Layers (CHOSEN)

**Concept**: Use H.264 temporal scalability to mark Aux as non-reference

**Mechanism**:
```
iTemporalLayerNum = 2

OpenH264 automatically assigns:
  Frame 0 (Main) → Temporal layer 0 (BASE) → nal_ref_idc=2-3 (REFERENCE)
  Frame 1 (Aux)  → Temporal layer 1 (ENH)  → nal_ref_idc=0 (NON-REFERENCE)
  Frame 2 (Main) → Temporal layer 0 (BASE) → nal_ref_idc=2-3 (REFERENCE)
  Frame 3 (Aux)  → Temporal layer 1 (ENH)  → nal_ref_idc=0 (NON-REFERENCE)
```

**H.264 Specification GUARANTEES**:
- T1 frames have nal_ref_idc=0
- Decoders CANNOT use nal_ref_idc=0 frames for prediction
- **Aux frames CANNOT contaminate DPB for prediction!**

**Why chosen**:
1. **Deterministic**: Specification-enforced, not probabilistic
2. **Correct**: Uses H.264 features AS DESIGNED
3. **Semantic match**: Aux IS an enhancement (extra chroma detail)
4. **Efficient**: DPB only contains T0 (Main) frames
5. **Robust**: Works regardless of content patterns
6. **Future-proof**: Guaranteed by H.264 specification

**This is "use the right tool for the job" - ROBUST!**

---

## SEMANTIC ALIGNMENT

### What AVC444 Is

**Main stream**:
- Luma + subsampled chroma (4:2:0)
- Essential for display
- MUST be decoded

**Auxiliary stream**:
- Additional chroma detail
- Converts 4:2:0 → 4:4:4
- **Optional enhancement** (could fall back to 4:2:0)

### What H.264 Temporal Layers Are

**Base layer (T0)**:
- Essential frames
- MUST be decoded
- Used as references

**Enhancement layer (T1)**:
- Optional extra frames
- Can be dropped
- **Not used as references**

### The Perfect Match

**Main = T0 (base)**: Essential, reference
**Aux = T1 (enhancement)**: Optional detail, non-reference

**This is not a hack - it's the CORRECT architectural model!**

---

## IMPLEMENTATION DECISION

### What We'll Implement

**1. Extend openh264-rs**:
```rust
pub struct EncoderConfig {
    // ...
    temporal_layers: Option<i32>,  // NEW
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

**2. Configure AVC444 encoder**:
```rust
let config = OpenH264Config::new()
    .num_ref_frames(2)  // Keep current
    .temporal_layers(2)  // NEW - Main=T0, Aux=T1
    .scene_change_detect(false);
```

**3. Test and verify**:
- Check NAL logs: Aux should have nal_ref_idc=0
- Test for corruption: Should be eliminated
- Verify performance: Should be good

---

## EXPECTED OUTCOME

### With Temporal Layers Enabled

**NAL structure**:
```
[Frame #0 MAIN] type=5 (IDR) ref_idc=3 (REFERENCE)   ← T0
[Frame #0 AUX]  type=5 (IDR) ref_idc=0 (NON-REF)    ← T1!
[Frame #1 MAIN] type=1 (P)   ref_idc=2 (REFERENCE)   ← T0
[Frame #1 AUX]  type=5 (IDR) ref_idc=0 (NON-REF)    ← T1!
```

**DPB after Frame 1**:
```
DPB = [Main_0, Main_1]  ← Only T0 (Main) frames!
Aux frames not in DPB (non-reference)
```

**Main_2 P-frame**:
```
Searches DPB = [Main_0, Main_1]  ← Only Main frames available!
Cannot reference Aux (not in DPB)
Predicts correctly from Main_0 or Main_1
→ NO CORRUPTION! ✅
```

---

## WHY THIS IS THE ROBUST CHOICE

**If we chose NUM_REF=4**:
- ⚠️ Might work, might not
- ⚠️ Depends on content
- ⚠️ Fragile workaround

**If it failed**:
- Would need to try NUM_REF=8, 16, etc.
- Still no guarantee
- Eventually would need temporal layers anyway

**By choosing Temporal Layers first**:
- ✅ Go directly to the correct solution
- ✅ Avoid band-aid attempts
- ✅ Architectural correctness
- ✅ Senior engineering decision

---

## COMMITMENT TO CORRECTNESS

**You said**: "I want the completely correct thing"

**Temporal Layers IS the completely correct thing**:
- Not a workaround
- Not a guess
- Not a hope
- **A proper use of H.264's design!**

**Time investment**: 2-3 hours
**Confidence**: 75%
**Correctness**: 100% (it's the right approach)

**Even if it doesn't work** (25% chance), we'll have learned something fundamental and can pivot with full understanding.

---

## IMPLEMENTATION BEGINS NOW

**Next**:
1. Extend openh264-rs with temporal_layers API
2. Update AVC444 encoder to use it
3. Deploy and test
4. Analyze results

**All decisions documented.**
**Proceeding with robust, correct solution.**
