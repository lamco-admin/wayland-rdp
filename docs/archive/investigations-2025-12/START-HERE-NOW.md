# AVC444 Troubleshooting - Session Summary 2025-12-27

**Time**: 2025-12-27 Evening
**Commit**: `f6be355` (pushed to GitHub ✅)
**Status**: All-I workaround works perfectly, root cause identified

---

## ✅ CONFIRMED: All-I Workaround is PERFECT

**Test Result**: "absolutely no quality or color or anything wrong with this. no lavender whatsoever"

**Binary MD5**: `b1e79780138f6bd8069dbe506f0b48a9` (both encoders all-I)
**Quality**: Perfect colors, no corruption, works with screen changes

---

## 🎯 ROOT CAUSE IDENTIFIED

### The Evidence

**Input is 100% stable** (static wallpaper):
```
Frame #1:   center BGRA=(125,35,240) → YUV=(85,150,226)
Frame #10:  center BGRA=(125,35,240) → YUV=(85,150,226)
Frame #20:  center BGRA=(125,35,240) → YUV=(85,150,226)
Frame #100: center BGRA=(125,35,240) → YUV=(85,150,226)
```

**Auxiliary buffer changes every frame**:
```
Frame #1:  hash 0xd8a756e4f8c9559d
Frame #2:  hash 0x87de1792f232834a
Frame #3:  hash 0x27cb16d11170a197
Frame #8:  hash 0xd5aa60343cfceb02  ← Repeats later
Frame #10: hash 0xd5aa60343cfceb02  ← Same as #8
```

**Conclusion**: Auxiliary packing code produces nondeterministic output despite deterministic input.

### Why This Causes P-Frame Corruption

1. **Frame 1**: Aux buffer = state A → Encoded as I-frame
2. **Frame 2**: Aux buffer = state B (but input identical!)
3. **P-frame encoder**: Sees "change" from A to B
4. **Encodes**: Difference between A and B (garbage)
5. **Result**: Lavender/brown macroblock corruption

With all-I: Every frame encoded independently → no corruption!

---

## 🔬 The Cycling Pattern

Hash `0xd5aa60343cfceb02` appears in frames: 8, 10, 13, 22, 27

**This isn't random** - buffer cycles through ~3-5 states.

**Likely causes**:
- Vec reallocation patterns
- Uninitialized memory reuse
- Alignment/padding variations
- Stride mismatch reading beyond bounds

---

## 📂 What's Committed (f6be355)

### Code Changes
- Force both main+aux to all-I (`src/egfx/avc444_encoder.rs:325-326`)
- Multi-position sampling (`color_convert.rs`, `yuv444_packing.rs`)
- Frame numbering for tracking
- Temporal hash check (moved after truncate)

### Documentation (15+ files)
- **BREAKTHROUGH-2025-12-27.md**: Root cause findings
- **ROOT-CAUSE-ANALYSIS.md**: Technical deep dive
- **SESSION-COMPLETE-2025-12-27.md**: Full session summary
- Investigation and handover docs

---

## 🚀 Next Session: Fix the Nondeterminism

### Option 1: Explicit Fill (Quick Test)

Replace `vec![128u8; size]` with explicit fill:

```rust
let mut aux_u = vec![0u8; padded_chroma_width * padded_chroma_height];
let mut aux_v = vec![0u8; padded_chroma_width * padded_chroma_height];
aux_u.fill(128);
aux_v.fill(128);
```

### Option 2: Log Buffer Diffs (Find Where It Changes)

When hash changes, log first different byte:

```rust
if prev_hash != curr_hash {
    // Log where buffers differ
}
```

Shows if changes are in padding or data regions.

### Option 3: Remove Truncate

Keep full padded buffer:

```rust
// aux_y.truncate(height * width);  // Don't truncate
```

Pass full dimensions to encoder.

### Option 4: Test with Larger Explicit Clear

Manually zero padding regions:

```rust
// Clear Y padding rows
for row in height..padded_height {
    for col in 0..width {
        aux_y[row * width + col] = 128;
    }
}

// Clear U/V padding (right + bottom edges)
// ...
```

---

## ⚙️ Current Configuration

**Test Server**: `greg@192.168.10.205`

**Binary**:
- MD5: `b1e79780138f6bd8069dbe506f0b48a9`
- Both encoders: all-I frames
- Logging: Multi-position, frame-numbered, temporal hash check

**Config** (`~/config.toml`):
- `video.damage_tracking = false`
- `egfx.codec = "avc444"`

**Latest Log**: `/tmp/latest-test.log` (76MB, copied locally)

---

## 📊 The Two Issues (Recap)

### Issue #1: P-Frame Corruption
**Status**: WORKAROUND WORKS (all-I), root cause identified
- Symptom: Lavender/brown in changed areas
- Cause: Auxiliary buffer nondeterminism
- Workaround: Force all-I (current binary)
- Proper fix: Make padding deterministic (next session)

### Issue #2: Color Quality
**Status**: NOT AN ISSUE with all-I workaround
- All-I mode has perfect colors
- No investigation needed currently

---

## 🎓 What We Learned

1. **Multi-position sampling was crucial**: Found that (0,0) was gray, needed to sample colorful areas
2. **Damage tracking interfered**: Had to disable to get consistent frames
3. **Frame numbering helps**: Easy to track temporal patterns
4. **Hash cycling reveals clue**: Not random, suggests memory reuse pattern
5. **All-I validates algorithm**: Proves packing logic is correct

---

## 📝 How to Resume

### Next Session Checklist

1. ✅ Read this file (you're doing it!)
2. ⬜ Try Option 1: Explicit `.fill(128)` instead of `vec![128; size]`
3. ⬜ Build, deploy, test
4. ⬜ Check logs for `✅ TEMPORAL STABLE`
5. ⬜ If stable: Re-enable P-frames
6. ⬜ If unstable: Try Option 2 (log diffs)

### Expected Outcome

With deterministic padding:
- Static screen → `✅ TEMPORAL STABLE`
- P-frames re-enabled → No corruption
- Perfect quality with normal compression
- **Problem solved!**

---

## 📂 Key Files

**Status docs**:
- **This file** - Complete session summary
- `SESSION-COMPLETE-2025-12-27.md` - Detailed findings
- `BREAKTHROUGH-2025-12-27.md` - Root cause analysis
- `ROOT-CAUSE-ANALYSIS.md` - Technical deep dive

**Code** (all in commit f6be355):
- `src/egfx/yuv444_packing.rs:468-487` - Auxiliary packing (needs determinism fix)
- `src/egfx/avc444_encoder.rs:325-326` - All-I workaround
- `src/egfx/color_convert.rs:218-249` - Multi-position sampling

**Logs**:
- `/tmp/latest-test.log` (76MB, local copy)
- Shows input stable, auxiliary changing

---

**Status**: Session complete, all work committed (f6be355), clear path to fix.
**Next**: Make auxiliary buffer deterministic → re-enable P-frames → done!
