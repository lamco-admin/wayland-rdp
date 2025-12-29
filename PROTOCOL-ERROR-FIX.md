# Protocol Error Fix - Frame 0 Must Keep IDR

**Date**: 2025-12-29 05:10 UTC
**Issue**: Stripped IDR from frame 0 Aux → decoder couldn't initialize
**Fix**: Frame-aware stripping (keep frame 0 IDR, strip frame 1+)
**Binary MD5**: `a2cda0800a077c7900dbb0cad09eed65`

---

## WHAT WENT WRONG

**Previous implementation**:
```rust
// Stripped IDR from ALL Aux frames including frame 0
stream2_data = Self::strip_sps_pps_and_idr(stream2_data);
```

**Problem**:
- Frame 0 Aux had NO slice data (all IDR stripped)
- Client decoder needs IDR to initialize
- **Protocol error**: Invalid bitstream

---

## THE FIX

**Frame-aware stripping**:
```rust
stream2_data = if self.frame_count == 0 {
    // Frame 0: Keep IDR (decoder initialization)
    stream2_data
} else {
    // Frame 1+: Strip IDR, keep only P-slices
    Self::strip_sps_pps_and_idr(stream2_data)
};
```

**Result**:
- Frame 0 Aux: `[SPS][PPS][IDR]` - decoder can initialize ✓
- Frame 1+ Aux: `[P-slice ref_idc=0]` - non-reference only ✓

---

## EXPECTED BEHAVIOR

**Frame 0 (Both IDR)**:
- Main: `[SPS][PPS][IDR ref_idc=3]`
- Aux: `[SPS][PPS][IDR ref_idc=3]`  ← Kept!
- Both initialize decoder
- Both enter DPB as reference (OK for first frame)

**Frame 1+ (Main P, Aux non-ref P)**:
- Main: `[SPS prepended][P-slice ref_idc=2]`
- Aux: `[P-slice ref_idc=0]`  ← IDR stripped!
- Main uses DPB (contains Main_0)
- Aux doesn't enter DPB (ref_idc=0)

**DPB progression**:
```
Frame 0: DPB = [Main_0, Aux_0]  (both reference for init)
Frame 1: DPB = [Main_0, Main_1]  (Aux_1 not added - ref_idc=0!)
Frame 2: DPB = [Main_1, Main_2]  (Aux_2 not added)
...
```

**Main P-frames**: Can only reference Main frames (Aux not in DPB after frame 0)

---

## WHY THIS SHOULD WORK

**Frame 0**: Normal dual-IDR initialization (like all-I, which works)
**Frame 1+**: Main refs Main only (Aux non-reference)

**Aux_0 in DPB**: Might allow Main_1 to ref it
**But**: After frame 1, Aux stops entering DPB
**So**: Corruption might happen frame 1 only, then clear

**Or**: With NUM_REF=2 and eviction, Aux_0 gets evicted quickly

---

## TEST EXPECTATIONS

**Best case**: NO corruption at all
**Good case**: Brief corruption at start, then clean
**Problem case**: Still corrupted throughout

**After test, logs will show**:
```
[Frame #0 AUX-FINAL] Full bitstream (with IDR)
[Frame #1 AUX-FINAL] Only P-slices (IDR stripped)
[Frame #2 AUX-FINAL] Only P-slices (IDR stripped)
```

**Binary**: `a2cda0800a077c7900dbb0cad09eed65`

Test and report!
