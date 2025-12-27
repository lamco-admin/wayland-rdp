# Ready for Tomorrow - Systematic U/V Swap Tests

**Date**: 2025-12-27 Late Evening  
**Status**: Multiple test builds prepared  
**Current Finding**: AVC420 works, AVC444 corrupts → Bug is in AVC444 packing  

---

## Most Likely Fix: Complete U/V Swap

**Theory**: U and V channels are swapped somewhere in AVC444 path

**Evidence**:
- AVC420 works (OpenH264 handles U/V internally - gets it right)
- AVC444 corrupts (we handle U/V manually - might have it backwards)
- All-keyframes worked (less corruption due to no inter-prediction amplification)

**Test**: Swap U/V in ALL AVC444 packing locations

---

## Test Variant A: Swap Main View U/V

**File**: `src/egfx/yuv444_packing.rs`
**Lines**: 232-236

**Current**:
```rust
let u = subsample_chroma_420(&yuv444.u, width, height);
let v = subsample_chroma_420(&yuv444.v, width, height);
```

**Change to**:
```rust
let u = subsample_chroma_420(&yuv444.v, width, height);  // SWAP
let v = subsample_chroma_420(&yuv444.u, width, height);  // SWAP
```

---

## Test Variant B: Swap Auxiliary Y Source

**File**: `src/egfx/yuv444_packing.rs`  
**Lines**: 346-365

**Current**:
```rust
if macroblock_row < 8 {
    aux_y[aux_start..aux_end]
        .copy_from_slice(&yuv444.u[src_start..src_end]);
} else {
    aux_y[aux_start..aux_end]
        .copy_from_slice(&yuv444.v[src_start..src_end]);
}
```

**Change to**:
```rust
if macroblock_row < 8 {
    aux_y[aux_start..aux_end]
        .copy_from_slice(&yuv444.v[src_start..src_end]);  // SWAP: V not U
} else {
    aux_y[aux_start..aux_end]
        .copy_from_slice(&yuv444.u[src_start..src_end]);  // SWAP: U not V
}
```

---

## Test Variant C: Swap Auxiliary Chroma

**File**: `src/egfx/yuv444_packing.rs`  
**Lines**: 388-392

**Current**:
```rust
aux_u.push(yuv444.u[idx]);
aux_v.push(yuv444.v[idx]);
```

**Change to**:
```rust
aux_u.push(yuv444.v[idx]);  // SWAP
aux_v.push(yuv444.u[idx]);  // SWAP
```

---

## Test Variant D: COMPLETE Swap (ALL THREE)

**Combine**: Variants A + B + C
**Rationale**: If U/V is consistently backwards, fix all at once

---

## Test Variant E: Minimal Auxiliary (Fallback Test)

**Purpose**: Confirm auxiliary is the problem

**Change**: Replace all auxiliary packing with neutral values:
```rust
let aux_y = vec![128u8; height * width];
let aux_u = vec![128u8; chroma_width * chroma_height];
let aux_v = vec![128u8; chroma_width * chroma_height];
// Return immediately, skip all packing
```

**Expected**: Clean video but looks like AVC420 (no 4:4:4 benefit)

---

## Recommended Testing Order

1. **Test Variant D** (complete swap) - Most likely to work if U/V is backwards
2. **Test Variant E** (minimal auxiliary) - Confirms auxiliary is problem source
3. **Test Variants A/B/C individually** - If D doesn't work, isolate which swap helps

---

## Build Commands Ready

```bash
cd /home/greg/wayland/wrd-server-specs

# For each variant:
# 1. Edit the file as documented above
# 2. Build:
cargo build --release

# 3. Deploy:
ssh greg@192.168.10.205 "rm -f ~/lamco-rdp-server"
scp target/release/lamco-rdp-server greg@192.168.10.205:~/

# 4. Test and observe results
```

---

## What to Look For

**If U/V swap fixes it**:
- Colors should be correct
- No lavender artifacts
- Normal performance

**If minimal auxiliary works**:
- No corruption
- But video will look like AVC420 (less sharp text)
- Proves auxiliary is the bug source

**If nothing works**:
- Problem is more fundamental
- May need to question MS-RDPEGFX spec understanding
- Consider other structural issues

---

*Prepared: 2025-12-27 evening*  
*Ready for systematic testing tomorrow*
