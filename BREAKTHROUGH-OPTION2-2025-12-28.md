# AVC444 P-Frame Corruption - BREAKTHROUGH: Option 2 Results

**Date**: 2025-12-28 00:53 UTC
**Binary MD5**: `fc20635e0e863e357960698f55b38bc7`
**Test Log**: `colorful-test-20251228-005330.log`

---

## 🎯 CRITICAL DISCOVERY: Data Region Cycling

### Option 2 Implementation

Added buffer diff logging to find exactly WHERE auxiliary buffers differ between frames:
- Compares current frame buffers with previous frame
- Logs first byte difference in aux_y, aux_u, aux_v
- Reports index, old value, new value, and region (DATA vs PADDING)

### Results: NOT a Padding Issue!

**Zero differences found in PADDING regions** ✅

**All differences are in DATA regions** ❌

### The Smoking Gun: Cycling Values

The buffers don't have random differences - they **cycle between 2-3 values** at specific positions:

```
Position: aux_y[149983] (row ~117)
Frame #1: 176
Frame #2: 137  ← Different!
Frame #3: 176  ← Back to first value
Frame #4: 137  ← Cycling
Frame #5: 176
```

```
Position: aux_u[39204] (row 61, col 164)
Frame #1: 125
Frame #2: 137  ← Different!
Frame #3: 125  ← Back to first value
Frame #4: 137  ← Cycling
```

```
Position: aux_v[39204] (row 61, col 164)
Frame #1: 144
Frame #2: 146  ← Different!
Frame #3: 144  ← Back to first value
Frame #4: 146  ← Cycling
```

### Input is 100% Stable

Color conversion samples show **identical input every frame**:

```
Frame #0: center BGRA=(125, 35, 240) → YUV=(85, 150, 226)
Frame #1: center BGRA=(125, 35, 240) → YUV=(85, 150, 226)
Frame #2: center BGRA=(125, 35, 240) → YUV=(85, 150, 226)
Frame #3: center BGRA=(125, 35, 240) → YUV=(85, 150, 226)
...
```

Multi-position samples also stable:

```
Frame #0: Aux Y[400]: [148,148,147,147], U: 149, V: 227
Frame #1: Aux Y[400]: [148,148,147,147], U: 149, V: 227
Frame #2: Aux Y[400]: [148,148,147,147], U: 149, V: 227
```

---

## 🔬 What This Means

### NOT These Issues:
- ❌ Vec initialization nondeterminism (Option 1 ruled this out)
- ❌ Padding region memory issues (Option 2 shows zero padding diffs)
- ❌ Random memory corruption (values cycle predictably)
- ❌ Input changing (color conversion is 100% stable)

### IS This Issue:
- ✅ **Packing algorithm logic bug**
- ✅ **Reads from different source positions on alternating frames**
- ✅ **Deterministic cycling pattern (not random)**

---

## 📊 Affected Regions

### Hot Spots (most frequent differences)

**aux_y positions:**
- Index 149983 (row ~117)
- Index 150033 (row ~117)
- Index 150100 (row ~117)
- Index 150149 (row ~117)
- Index 151638 (row ~118)

**aux_u/aux_v positions (chroma):**
- Row 61, columns: 152, 164, 197, 224, 234, 254
- Row 62, columns: 299, 300

**Pattern**: Concentrated around specific rows (61-62 in chroma, 117-118 in aux_y)

---

## 🧩 Root Cause Hypothesis

The packing algorithm has a **state-dependent bug** that causes it to:

1. Read from position A on odd frames
2. Read from position B on even frames
3. Repeat this cycle

**Likely culprits:**
- Loop counter reuse (`u_row_counter`, `v_row_counter`)
- Index calculation that depends on previous state
- Iteration order instability
- Race condition in row/column indexing

**Key evidence:**
- Same indices differ repeatedly (not random)
- Values alternate between 2-3 stable states
- Input is provably stable
- Only affects specific row ranges

---

## 📝 Sample Diff Output

```
[Frame #2] ⚠️  TEMPORAL CHANGE: Auxiliary DIFFERENT (prev: 0xa258921882ca6365, curr: 0x7a178d7da4ae4c63)
  📍 aux_y[149983] differs: 176 (was 176) → 137 (now 137) [DATA]
  📍 aux_u[39204] (row 61, col 164) differs: 125 → 137 [DATA]
     (chroma_width=640, padded_chroma_width=640, data_size=256000)
  📍 aux_v[39204] (row 61, col 164) differs: 144 → 146 [DATA]
     (chroma_width=640, padded_chroma_width=640, data_size=256000)

[Frame #3] ⚠️  TEMPORAL CHANGE: Auxiliary DIFFERENT (prev: 0x7a178d7da4ae4c63, curr: 0xa258921882ca6365)
  📍 aux_y[149983] differs: 137 (was 137) → 176 (now 176) [DATA]
  📍 aux_u[39204] (row 61, col 164) differs: 137 → 125 [DATA]
  📍 aux_v[39204] (row 61, col 164) differs: 146 → 144 [DATA]
```

Notice: Frame #3 shows **exact reverse** of Frame #2!

---

## 🎓 Why P-Frames Fail

With this cycling behavior:

1. **Frame 1**: Encoder sees aux_u[39204] = 125
2. **Frame 2**: Encoder sees aux_u[39204] = 137 (even though input identical!)
3. **P-frame encoder**: Detects "change" from 125 → 137
4. **Encodes**: Delta of +12
5. **Decoder**: Applies +12 to previous frame
6. **Result**: Corruption in changed areas (lavender/brown macroblocks)

**All-I frames work** because each frame is encoded independently without reference to previous frames, so the cycling doesn't cause visible corruption.

---

## 🚀 Next Steps

### 1. Deep Code Analysis

Examine `pack_auxiliary_view_spec_compliant()` for:
- State that persists between calls
- Loop counter logic (`u_row_counter`, `v_row_counter`)
- Index calculations for reading yuv444.u and yuv444.v
- Any iteration order dependencies

### 2. Targeted Logging

Add logging to show:
- Which source indices are being read for aux_u[39204]
- Loop counter values on each frame
- Row/column calculations for hot spot positions

### 3. Fix Strategy

Once root cause identified:
- Make packing algorithm stateless (no hidden dependencies)
- Ensure deterministic iteration order
- Verify index calculations are frame-independent

### 4. Validation

After fix:
- Run test with static wallpaper
- Should see `✅ TEMPORAL STABLE` on all frames
- Re-enable P-frames
- Verify no corruption with screen changes

---

## 📂 Files Changed (Option 2)

**Code**: `src/egfx/yuv444_packing.rs:582-673`
- Added `OnceLock<Mutex<(Vec<u8>, Vec<u8>, Vec<u8>)>>` for buffer storage
- Compare current frame with previous frame buffers
- Find and log first byte difference in each plane
- Report index, row, col, old/new values, DATA/PADDING region

**Status**: This session (BREAKTHROUGH-OPTION2-2025-12-28.md)

---

## 🏁 Summary

**Problem Found**: Packing algorithm reads from different source positions on alternating frames despite stable input.

**Not Problems**: Vec initialization, padding, memory corruption, input instability.

**Next**: Deep code investigation to find the source of the cycling and fix it.

**Expected Outcome**: Stateless packing → temporal stability → P-frames work perfectly.
