# Phase 1 Deployment Guide - Auxiliary Stream Omission

**Binary MD5**: `f8a3f098252ebe5004a96c2d8ffbd8ae`
**Build Date**: 2025-12-29 17:10 UTC
**Status**: ✅ READY TO DEPLOY
**Implementation**: Complete, tested compilation

---

## QUICK START

###  Enable Aux Omission (Simple Method)

**Edit**: `src/egfx/avc444_encoder.rs` line 315

**Change**:
```rust
enable_aux_omission: false,   // Current (disabled)
```

**To**:
```rust
enable_aux_omission: true,    // Enabled!
```

**Then rebuild**:
```bash
cargo build --release --features h264
```

**New MD5 will be different** - verify after rebuild

---

## DEPLOYMENT OPTIONS

### Option A: Test with Defaults (RECOMMENDED FIRST)

**Current state**: Aux omission implemented but **DISABLED**

**What this gives you**:
- ✅ All Phase 1 code present and validated
- ✅ Behavior identical to previous (both streams always sent)
- ✅ No risk - just confirming implementation doesn't break anything
- ✅ Can enable via single line change

**Deploy**: Current binary (`f8a3f098252ebe5004a96c2d8ffbd8ae`)

**Test**: Verify quality is perfect, no corruption (should match previous)

**Then**: Enable aux omission (Option B)

### Option B: Enable Aux Omission

**Change**: `src/egfx/avc444_encoder.rs` line 315 to `true`

**Rebuild and deploy**

**What this gives you**:
- Aux omission active
- Expected bandwidth: Still ~4.3 MB/s (all-I mode active)
- Logs will show "Aux: OMITTED" messages
- **Purpose**: Verify omission logic works correctly

### Option C: Full Phase 1 (Aux Omission + P-Frames)

**Changes**:
1. Line 315: `enable_aux_omission: true`
2. Line 307: `force_all_keyframes: false`  (remove all-I workaround)

**Rebuild and deploy**

**What this gives you**:
- Full Phase 1 implementation
- Main uses P-frames (compression)
- Aux omitted when unchanged
- **Expected**: 0.7-1.5 MB/s bandwidth
- **Risk**: P-frame corruption might return (needs testing!)

---

## TESTING PROGRESSION

### Test 1: Disabled (Baseline)

**Binary**: `f8a3f098252ebe5004a96c2d8ffbd8ae`
**Config**: `enable_aux_omission = false` (current)

**Verify**:
- ✅ Quality perfect
- ✅ No corruption
- ✅ Bandwidth ~4.3 MB/s (unchanged)
- ✅ Logs show no omission messages

**Duration**: 10 minutes

### Test 2: Enabled (All-I Mode)

**Change**: Line 315 → `true`
**Rebuild**: New MD5

**Verify**:
- ✅ Logs show "Aux: OMITTED (LC=1)" messages
- ✅ Some frames show "BOTH SENT"
- ✅ Quality still perfect
- ✅ No corruption
- ⚠️ Bandwidth still ~4.3 MB/s (all-I no compression yet)

**Purpose**: Verify omission logic without P-frame risk

**Duration**: 15 minutes

### Test 3: Full Phase 1 (P-Frames ON)

**Changes**: Lines 315 + 307
**Rebuild**: New MD5

**Verify**:
- ✅ Logs show Main P-frames
- ✅ Logs show aux omission
- ✅ Bandwidth significantly reduced
- ⚠️ **CRITICAL**: Check for any lavender corruption!

**If corruption appears**:
- P-frame issue remains unsolved
- Revert to Test 2 configuration (all-I + omission)
- Research further

**If NO corruption**:
- 🎉 **PHASE 1 COMPLETE SUCCESS!**
- Measure actual bandwidth
- Validate <2 MB/s achievement

**Duration**: 30 minutes + extended session test

---

## LOG ANALYSIS

### What to Look For

**Startup**:
```
Created AVC444 encoder: BT709 matrix, 5000kbps, level=Some(L4_0)
```

**If omission enabled** (Test 2+):
```
🎬 Phase 1 AUX OMISSION ENABLED: max_interval=30frames, force_idr_on_return=true
```

**Per-frame logging** (Test 2+):
```
[AVC444 Frame #0000] Main: IDR (74KB), Aux: IDR (73KB) [BOTH SENT]
[AVC444 Frame #0001] Main: IDR (74KB), Aux: OMITTED (LC=1) [BANDWIDTH SAVE]
[AVC444 Frame #0002] Main: IDR (74KB), Aux: OMITTED (LC=1) [BANDWIDTH SAVE]
...
[AVC444 Frame #0030] Main: IDR (75KB), Aux: IDR (72KB) [BOTH SENT]  (forced refresh)
```

**With P-frames** (Test 3):
```
[AVC444 Frame #0008] Main: P (21KB), Aux: OMITTED (LC=1) [BANDWIDTH SAVE]
[AVC444 Frame #0009] Main: P (19KB), Aux: OMITTED (LC=1) [BANDWIDTH SAVE]
```

**Periodic stats** (every 30 frames):
```
AVC444 frame 30: 1280×800 → 95123b (main: 20456b, aux: 74667b [sent]) in 33.2ms
AVC444 frame 60: 1280×800 → 19872b (main: 19872b, aux: 0b [omitted]) in 12.1ms
```

---

## BANDWIDTH MEASUREMENT

### Calculate from Logs

```bash
# Get all frame sizes
rg "\[AVC444 Frame" colorful-test-*.log > frames.txt

# Count omissions
grep "OMITTED" frames.txt | wc -l

# Count sent
grep "BOTH SENT" frames.txt | wc -l

# Calculate average size and bandwidth
python3 -c "
import re
with open('frames.txt') as f:
    sizes = [int(m.group(1)) for line in f for m in [re.search(r'Main:.*?\((\d+)B\)', line)] if m]
    print(f'Frames: {len(sizes)}')
    print(f'Average: {sum(sizes)/len(sizes)/1024:.1f} KB/frame')
    print(f'Bandwidth @ 30fps: {sum(sizes)*30/(1024*1024):.2f} MB/s')
"
```

---

## SUCCESS CRITERIA

### Phase 1A (Disabled)

- ✅ Compiles successfully
- ✅ No behavior change
- ✅ Quality perfect

### Phase 1B (Enabled, All-I)

- ✅ Omission logic works
- ✅ Logs show statistics
- ✅ No corruption
- ✅ Quality perfect

### Phase 1C (Enabled, P-Frames)

- ✅ Bandwidth < 2 MB/s
- ✅ No corruption
- ✅ Quality excellent
- ✅ Stable over 30+ minutes

---

## ROLLBACK

**If any issues**:

```bash
# Quick: Disable in code
# Line 315: enable_aux_omission: true → false
# Rebuild

# Full: Revert commits
git checkout HEAD~1 src/egfx/avc444_encoder.rs src/server/egfx_sender.rs src/server/display_handler.rs src/config/types.rs config.toml
cargo build --release --features h264
```

---

## NEXT PHASE PREPARATION

**After Phase 1 validated**:

### Phase 2 Features (Planned)

1. Threshold-based pixel diffing
2. Dual bitrate control
3. Encoder telemetry
4. Advanced monitoring

**Timeline**: Next sprint (12-16 hours)

---

**Status**: Ready to test
**Risk**: Very low (conservative defaults)
**Expected**: <2 MB/s with P-frames enabled
**Documentation**: Complete

**Deploy and test Option A first, then progress to B and C!**
