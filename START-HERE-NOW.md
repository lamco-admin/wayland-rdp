# START HERE - 2025-12-27 Evening

**Binary MD5**: `5d4da6b23c98cef0efbe1e61dbdebc1e`
**Status**: Hash moved after truncate + frame numbering added

---

## 🎯 Key Findings from Last Test

### GOOD NEWS: All-I Mode is PERFECT! ✅
- No corruption
- No lavender
- No color issues
- Quality is absolutely perfect

### THE PROBLEM: Auxiliary Buffer is Nondeterministic

**Evidence**:
```
Input (static wallpaper):  BGRA=(125,35,240) → IDENTICAL every frame ✅
Auxiliary hash: DIFFERENT every frame ❌
  Frame 1: 0x6baef4dda507a104
  Frame 2: 0xfedd5d5e0220c3a0
  Frame 3: 0x1f6f654f2b312036
```

**Why this matters**: P-frames compare current to previous. If buffers change (even padding), P-frames detect "phantom changes" → encode garbage → lavender corruption.

---

## 🔧 What I Just Changed

### 1. Moved Hash Computation
**Before**: Hashed aux_y BEFORE truncate (included padding rows)
**After**: Hash AFTER truncate (only hash what OpenH264 sees)

**Why**: Previous hash included padding rows that get removed. Now we only hash the actual data sent to encoder.

### 2. Added Frame Numbers
Logs now show:
```
[Frame #0] 🔵 FIRST FRAME
[Frame #1] ⚠️  TEMPORAL CHANGE
[Frame #2] ✅ TEMPORAL STABLE
```

**Why**: Easy to track which specific frames are stable vs changing.

---

## 🚀 Next Test

```bash
ssh greg@192.168.10.205
./run-server.sh
```

**Check logs for**:
```
[Frame #1] ✅ TEMPORAL STABLE: Auxiliary IDENTICAL
[Frame #2] ✅ TEMPORAL STABLE: Auxiliary IDENTICAL
```

**If STABLE**: Padding is deterministic! Can re-enable P-frames!
**If CHANGE**: Still nondeterministic, need deeper investigation.

---

## Current State

**Both encoders**: All-I frames (no P-frames)
**Config**: damage_tracking=false
**Result**: Perfect quality, no corruption
**Problem**: High latency/bitrate (not sustainable)

**Goal**: Make padding deterministic → re-enable P-frames → keep perfect quality

---

## Two Scenarios

### Scenario A: Logs Show STABLE
✅ Padding is deterministic
→ Re-enable P-frames for both encoders
→ Test if corruption returns
→ If no corruption: **PROBLEM SOLVED!**

### Scenario B: Logs Still Show CHANGE
❌ Padding still nondeterministic
→ Investigate aux_u / aux_v padding
→ Check if vec![128] is actually deterministic
→ May need explicit memset or different approach

---

**Action**: Run test, check for "[Frame #X] TEMPORAL STABLE" in logs
