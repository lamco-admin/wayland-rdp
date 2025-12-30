# Test Variants Ready for Systematic Diagnosis

**Prepared**: 2025-12-27  
**Current Deployed**: Diagnostic build with extensive logging (MD5: 936ed57c9453676c4c52c3df5435085a)

---

## Test Variant 1: TRACE Logging (CURRENTLY DEPLOYED)

**Binary**: Already on VM  
**MD5**: `936ed57c9453676c4c52c3df5435085a`  
**Run**: `RUST_LOG=lamco_rdp_server=trace ~/run-server.sh`

**What it does**:
- AVC444 enabled with row-level packing
- Extensive logging of YUV444, main view, auxiliary view values
- Look for 🔍 markers in log

**Purpose**: See actual data being packed to find mismatch

---

## Test Variant 2: Swap U/V in Main View (READY TO BUILD)

**Purpose**: Test if main view chroma channels are backwards

**Change needed**: `src/egfx/yuv444_packing.rs:232-236`
```rust
// Swap U and V
let u = subsample_chroma_420(&yuv444.v, width, height);  // Was yuv444.u
let v = subsample_chroma_420(&yuv444.u, width, height);  // Was yuv444.v
```

**Expected**: If colors improve → main view U/V are backwards

---

## Test Variant 3: Swap U/V in Auxiliary Y (READY TO BUILD)

**Purpose**: Test if we're packing V444 when should pack U444

**Change needed**: `src/egfx/yuv444_packing.rs:336-366`
```rust
if macroblock_row < 8 {
    // SWAP: Pack V444 instead of U444
    aux_y[aux_start..aux_end]
        .copy_from_slice(&yuv444.v[src_start..src_end]);  // Was yuv444.u
} else {
    // SWAP: Pack U444 instead of V444
    aux_y[aux_start..aux_end]
        .copy_from_slice(&yuv444.u[src_start..src_end]);  // Was yuv444.v
}
```

**Expected**: If colors improve → auxiliary Y packing backwards

---

## Test Variant 4: Neutral Auxiliary View (READY TO BUILD)

**Purpose**: Disable auxiliary stream to test if problem is there

**Change needed**: `src/egfx/yuv444_packing.rs` - replace auxiliary view content:
```rust
let mut aux_y = vec![128u8; height * width];  // All neutral
let aux_u = vec![128u8; chroma_width * chroma_height];
let aux_v = vec![128u8; chroma_width * chroma_height];
// Skip all packing logic
```

**Expected**: If clean → auxiliary view is the problem  
**Visual**: Will look like AVC420 (no 4:4:4 chroma)

---

## Test Variant 5: Check BGRA→YUV Color Matrix

**Purpose**: Verify we're using correct color matrix

**Currently using**: `ColorMatrix::OpenH264` (BT.601 limited range)

**Alternatives to test**:
- `ColorMatrix::BT709` (full range)
- `ColorMatrix::BT601` (full range)

**Change**: `src/egfx/avc444_encoder.rs:190`

---

## Systematic Isolation Plan

### Phase 1: Verify Data (TRACE logging)
→ Check if values match expectations mathematically

### Phase 2: Test Swaps
→ U/V swap in main view
→ U/V swap in auxiliary Y
→ U/V swap in auxiliary chroma

### Phase 3: Simplify
→ Neutral auxiliary (effectively AVC420 quality)
→ If that works, gradually add back complexity

### Phase 4: Alternative Approaches
→ Different color matrices
→ Different row mapping formulas

---

## Commands to Build Test Variants

```bash
cd /home/greg/wayland/wrd-server-specs

# Test Variant 2: Swap main U/V
# (edit src/egfx/yuv444_packing.rs:232-236)
cargo build --release
# Deploy: ssh greg@192.168.10.205 "rm -f ~/lamco-rdp-server"
# scp target/release/lamco-rdp-server greg@192.168.10.205:~/
# Test and observe

# Test Variant 3: Swap auxiliary Y  
# (edit src/egfx/yuv444_packing.rs:336-366)
cargo build --release
# Deploy and test

# etc.
```

---

## Priority Order for Testing

1. **TRACE logging first** - See actual data
2. **U/V swaps** - Quick to test, high info value
3. **Neutral auxiliary** - Confirms auxiliary is problem
4. **Alternative formulas** - If above don't work

---

*All test variants planned*  
*Ready for systematic diagnosis*  
*Continue when rested*
