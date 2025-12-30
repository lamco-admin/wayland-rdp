# EXHAUSTIVE ANALYSIS: Temporal Layers Test Results

**Date**: 2025-12-29 05:00 UTC
**Binary MD5**: `984436f565ca6eaaa59f35d0ca3306a6`
**Finding**: Temporal layers ARE working (nal_ref_idc=0 seen) but still have intermittent corruption

---

## CRITICAL DISCOVERY #1: Temporal Layers ARE Working!

### Evidence from NAL Logs

**Aux frames contain P-slices with ref_idc=0**:
```
[Frame #2 AUX NAL#0] type= 1 (P-slice) ref_idc=0 (NON-REF) ✅
[Frame #4 AUX NAL#0] type= 1 (P-slice) ref_idc=0 (NON-REF) ✅
[Frame #7 AUX NAL#0] type= 1 (P-slice) ref_idc=0 (NON-REF) ✅
...
[Frame #31 AUX NAL#0] type= 1 (P-slice) ref_idc=0 (NON-REF) ✅
```

**This proves**: Temporal layer assignment IS working!
- Aux frames (odd) → T1 layer
- T1 layer → nal_ref_idc=0 (non-reference)
- **Exactly as designed!**

---

## CRITICAL DISCOVERY #2: Aux Contains BOTH P-slices and IDR

### Confusing NAL Structure

**Frame #2 Aux** (example):
```
NAL#0: type=1 (P-slice) ref_idc=0 (NON-REF)     ← From temporal layers
NAL#0: type=7 (SPS) ref_idc=3                    ← Then SPS
NAL#1: type=8 (PPS) ref_idc=3                    ← Then PPS
NAL#2: type=5 (IDR) ref_idc=3 (REFERENCE)        ← Then IDR!
```

**Questions**:
1. Why BOTH P-slice and IDR in same bitstream?
2. Why multiple NAL#0 entries? (logging calling twice?)
3. Which one does client use?

**Hypothesis**:
- Our logging is being called multiple times per frame (once before SPS/PPS handling, once after?)
- Or encoder produces both and we're seeing both states

---

## CRITICAL DISCOVERY #3: Intermittent Corruption Pattern Persists

### Good Frames (Both Main-IDR + Aux-IDR)

**Frames**: 0, 1, 2, 13, 14, 20, 26, 27, 34, 40, 51, 56, 57, 63, 69, 70, 72-75, 81, 82, 84, 122, 131, 143

**Pattern**: When Main sends IDR + Aux sends IDR → CLEAN

### Corrupted Frames (Main-P + Aux-anything)

**All other frames** where Main uses P

**Pattern**: Main-P → CORRUPTION (regardless of Aux type)

**SAME pattern as SPS/PPS test!**

---

## WHAT THIS MEANS

### Temporal Layers Works Technically...

✅ Aux P-slices have nal_ref_idc=0
✅ Aux is being marked as non-reference (when P)
✅ H.264 temporal scalability is functioning

### ...But Corruption Still Happens

❌ Intermittent corruption persists
❌ Same pattern: Clean when Main-IDR, corrupt when Main-P
❌ Temporal layers didn't solve the problem

**Why?**

**Possibility 1**: The logging shows BOTH our bitstream (before SPS/PPS strip) and AFTER
- We're seeing the pre-stripped bitstream with P-slice ref_idc=0
- But we then prepend IDR (with SPS/PPS) which has ref_idc=3
- **Client sees the IDR, not the P-slice!**

**Possibility 2**: Encoder produces BOTH P-slices and IDR
- Some internal OpenH264 behavior
- Mixed bitstream confuses client

**Possibility 3**: Even with nal_ref_idc=0, something else is wrong
- Client decoder behavior
- Our packaging
- Something fundamental we're missing

---

## CRITICAL ISSUE: Our Logging is Misleading

### When is log_nal_structure() Called?

**In our code** (avc444_encoder.rs:~349, ~370):
```rust
// After encoding, before SPS/PPS handling:
let stream1_data = main_bitstream.to_vec();
Self::log_nal_structure(&stream1_data, "MAIN", self.frame_count);

// Then later:
stream1_data = self.handle_sps_pps(stream1_data, main_is_keyframe);
stream2_data = Self::strip_sps_pps(stream2_data);
```

**We log BEFORE handle_sps_pps and strip_sps_pps!**

**This means**:
- Logs show what encoder produced
- NOT what we actually send to client
- **After strip_sps_pps, Aux should NOT have SPS/PPS/IDR!**

**Need to log AFTER processing to see what client receives!**

---

## CRITICAL QUESTION: What Does Client Actually Receive?

### For Frame #2 (Should be Clean)

**What encoder produced** (from logs BEFORE our processing):
```
Aux: [P-slice ref_idc=0][SPS ref_idc=3][PPS ref_idc=3][IDR ref_idc=3]
```

**What strip_sps_pps() should produce**:
```
Aux: [P-slice ref_idc=0]  ← Only the P-slice, SPS/PPS/IDR stripped
```

**But**: Does our strip function work correctly?
- Does it remove ALL SPS/PPS/IDR?
- Or does it only remove SPS/PPS, leaving IDR?

**Need to verify**: Log what actually gets sent!

---

## DEBUGGING HYPOTHESIS

### Is strip_sps_pps() Working Correctly?

**Our implementation** (avc444_encoder.rs:~477-527):
```rust
fn strip_sps_pps(data: Vec<u8>) -> Vec<u8> {
    // ... parse NALs ...
    if nal_type != 7 && nal_type != 8 {  // Keep if NOT SPS/PPS
        result.extend_from_slice(&data[i..nal_end]);
    }
}
```

**This keeps**:
- NAL type 1 (P-slice) ✓
- NAL type 5 (IDR) ✓  ← PROBLEM!
- Everything except SPS(7) and PPS(8)

**This means**: IDR slices are NOT stripped, only SPS/PPS!

**So client receives for Aux**:
```
[P-slice ref_idc=0][IDR ref_idc=3]  ← Both present!
```

**Client might**:
- Use the IDR (ref_idc=3) as reference!
- Ignore the P-slice
- Get confused

**THE BUG**: We're stripping SPS/PPS but NOT stripping IDR slices!

---

## THE ACTUAL PROBLEM

### Aux Bitstream Still Contains IDR

**What we wanted**: Aux with only P-slice (ref_idc=0)
**What we're sending**: Aux with P-slice + IDR

**frame_type()** returns IDR (because IDR NAL present)
**But bitstream also has** P-slice (with ref_idc=0)

**Client decoder**:
- Sees IDR slice (ref_idc=3) → stores in DPB as reference!
- **Aux is STILL entering DPB!**
- Main can still reference it!

**This explains why temporal layers didn't fix it!**

---

## ROOT CAUSE IDENTIFIED (For Real This Time)

### The Issue is Our Bitstream Manipulation

**Encoder produces** (with temporal layers):
- Multiple NAL units including P-slices (ref_idc=0) and possibly IDR

**Our strip_sps_pps()**:
- Removes SPS and PPS
- **Keeps IDR slices!**

**Result**:
- Aux still has IDR (ref_idc=3)
- **Aux still enters DPB as reference!**
- Temporal layers are working but we're breaking them!

---

## THE FIX

### Option 1: Strip IDR from Aux Too

**Not just SPS/PPS, but also IDR**:
```rust
fn strip_sps_pps_and_idr(data: Vec<u8>) -> Vec<u8> {
    // ...
    // Skip NAL types: 5 (IDR), 7 (SPS), 8 (PPS)
    if nal_type != 5 && nal_type != 7 && nal_type != 8 {
        result.extend_from_slice(&data[i..nal_end]);
    }
}
```

**Keep only**: P-slices, SEI, etc.

**This would make Aux**:
```
[P-slice ref_idc=0]  ← ONLY non-reference P-slice!
```

**Then**: Aux truly won't enter DPB!

---

### Option 2: Understand Why Encoder Produces Both

**Question**: Why does encoder produce BOTH P-slice and IDR?

**Possible reasons**:
- Scene change detection still triggering? (we disabled it)
- Intra refresh?
- Some other internal logic?

**Need to**: Understand OpenH264's behavior better

---

### Option 3: Log After Processing

**First**: Verify what we're actually sending

**Add logging AFTER strip_sps_pps**:
```rust
stream2_data = Self::strip_sps_pps(stream2_data);
Self::log_nal_structure(&stream2_data, "AUX-AFTER-STRIP", self.frame_count);
```

**This shows**: What client actually receives

---

## IMMEDIATE NEXT STEP

### Fix strip_sps_pps to Also Remove IDR

**Change**:
```rust
// Strip SPS (7), PPS (8), AND IDR (5) from Aux
if nal_type != 5 && nal_type != 7 && nal_type != 8 {
    result.extend_from_slice(&data[i..nal_end]);
}
```

**Test**: See if this eliminates corruption

**Theory**: With only P-slices (ref_idc=0) in Aux, no reference contamination!

---

## SUMMARY

**Temporal layers**: ✅ WORKING (ref_idc=0 seen for Aux P-slices)
**Our bug**: ❌ Not stripping IDR from Aux, so Aux still enters DPB
**The fix**: Strip IDR slices too, leave only non-reference P-slices

**Shall I implement this fix now?** (5 minutes)

**Sources**:
- [RFC 6184 - H.264 NAL Reference](https://datatracker.ietf.org/doc/html/rfc6184)
- [H.264 nal_ref_idc Specification](https://yumichan.net/video-processing/video-compression/breif-description-of-nal_ref_idc-value-in-h-246-nalu/)
