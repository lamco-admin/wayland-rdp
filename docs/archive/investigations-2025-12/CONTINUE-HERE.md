# 👉 START HERE WHEN YOU CONTINUE

**Last Updated**: 2025-12-27 Late Evening  
**Status**: AVC444 corruption isolated to packing code, systematic tests prepared  

---

## 🎯 Where We Are

**✅ Confirmed Working**:
- AVC420 encoding (proves color conversion & OpenH264 are correct)
- All-keyframes mode (proves data is OK when no P-frames)

**❌ Still Broken**:
- AVC444 with P-frames (lavender corruption in changed areas)
- Row-level macroblock fix didn't solve it

**🔍 Currently Deployed**:
- Diagnostic build with TRACE logging (MD5: 936ed57c9453676c4c52c3df5435085a)
- Located at: greg@192.168.10.205:~/lamco-rdp-server

---

## 💡 Most Likely Issue: U/V Swap

Based on evidence, the most probable cause is **U and V channels are swapped** somewhere in the AVC444 packing code.

**Why this makes sense**:
- AVC420 works (OpenH264 gets U/V right internally)
- AVC444 corrupts (we manually pack U/V - might be backwards)
- Lavender is purple-ish (specific U/V color signature)

---

## 🚀 Next Test: Complete U/V Swap

**Recommendation**: Test swapping U/V in ALL three AVC444 packing locations at once

### Changes Needed

**1. Main View** (`src/egfx/yuv444_packing.rs:232-236`):
```rust
let u = subsample_chroma_420(&yuv444.v, width, height);  // SWAP
let v = subsample_chroma_420(&yuv444.u, width, height);  // SWAP
```

**2. Auxiliary Y** (`src/egfx/yuv444_packing.rs:346-365`):
```rust
if macroblock_row < 8 {
    .copy_from_slice(&yuv444.v[...]);  // SWAP: was .u
} else {
    .copy_from_slice(&yuv444.u[...]);  // SWAP: was .v
}
```

**3. Auxiliary Chroma** (`src/egfx/yuv444_packing.rs:388-392`):
```rust
aux_u.push(yuv444.v[idx]);  // SWAP
aux_v.push(yuv444.u[idx]);  // SWAP
```

### Build & Deploy
```bash
cargo build --release
ssh greg@192.168.10.205 "rm -f ~/lamco-rdp-server"
scp target/release/lamco-rdp-server greg@192.168.10.205:~/
```

**Expected**: If this fixes it → we had U/V backwards the whole time

---

## 📚 Documentation Created

**Read these for full context**:
1. `docs/ANALYSIS-2025-12-27-EVENING.md` - Evening analysis summary
2. `docs/READY-FOR-TOMORROW.md` - Test variants prepared
3. `docs/TEST-VARIANTS-READY.md` - Complete test scenarios
4. `docs/AVC444-COMPREHENSIVE-RESEARCH-AND-FIX-2025-12-27.md` - Full research (3+ hrs)

---

## 🔄 Alternative: If U/V Swap Doesn't Work

**Test**: Minimal auxiliary (all 128s)
- Proves whether auxiliary is the problem
- Will look like AVC420 but should be corruption-free

---

**Your next action**: Make the U/V swap changes above and test, or let me know which variant to build.

Rest well! 💤
