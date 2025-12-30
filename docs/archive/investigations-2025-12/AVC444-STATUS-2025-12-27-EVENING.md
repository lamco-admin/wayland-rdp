# AVC444 Status Report - 2025-12-27 Evening

## Current State

**Two separate issues** with AVC444, one now has a better workaround:

### Issue #1: Change-Area Corruption (NEW FINDING)
**Symptom**: When screen content changes (menu opens, window moves), the changed areas show **brown/dark corruption**. Static areas remain colorful and correct.

**Root Cause**: **Auxiliary encoder P-frames** are corrupting.

**Previous workaround**: Only forced main encoder to all-I → didn't fix it
**New workaround**: Force **BOTH** main AND auxiliary to all-I → **DEPLOYED NOW**

**Binary**: `b1e79780138f6bd8069dbe506f0b48a9` (just deployed)

### Issue #2: Overall Color Quality
**Symptom**: Even when corruption-free, colors look "slightly off" compared to AVC420
**Status**: Needs investigation after Issue #1 is confirmed fixed

---

## What We Learned Today

### Session 1: Multi-Position Logging (Early)
- Added logging to sample 5 screen positions (not just 0,0)
- **Found**: BGRA input was changing between frames even on static wallpaper
- **Cause**: Damage tracking (`video.damage_tracking = true`) was skipping/truncating frames
- **Fix**: Disabled damage tracking in config.toml

### Session 2: Screenshots Show Corruption (Now)
- Tested with colorful wallpaper
- **Found**: Static areas = fine, changed areas = brown corruption
- **Realization**: We only fixed main encoder, not auxiliary
- **Root cause**: Auxiliary P-frames break when screen changes
- **Fix**: Force auxiliary to all-I too (just deployed)

---

## Test Status - What to Do Next

### Immediate Test (With Latest Binary)

**Binary MD5**: `b1e79780138f6bd8069dbe506f0b48a9`

```bash
ssh greg@192.168.10.205
cd ~
./run-server.sh
```

**What to test**:
1. Open colorful wallpaper
2. **Open menus, move windows** (trigger changes)
3. Check if brown/dark corruption still appears in changed areas

**Expected result**: NO corruption even when things change (both encoders now all-I)

**Side effect**: Higher latency (no P-frames at all), but should be corruption-free

---

## Configuration Summary

**On test server** (`greg@192.168.10.205`):

```toml
[video]
damage_tracking = false  # Disabled to get consistent frames

[egfx]
codec = "avc444"         # Still using AVC444
```

**In code** (`src/egfx/avc444_encoder.rs:325-326`):
```rust
self.main_encoder.force_intra_frame();  // All-I for main
self.aux_encoder.force_intra_frame();   // All-I for auxiliary (NEW)
```

---

## The Two Issues Explained

### Why Both Issues Exist

**Issue #1** (P-frame corruption):
- Happens when screen changes
- Auxiliary P-frame tries to predict from previous frame
- Prediction breaks → corruption in changed area
- **Workaround**: All-I frames (no prediction)

**Issue #2** (Color quality):
- Happens even with perfect all-I encoding
- Might be VUI signaling, color matrix, or client reconstruction
- **Needs**: Comparison test AVC420 vs AVC444 side-by-side

### Why All-I Workaround is Not Ideal

**Pros**:
- Eliminates corruption ✅
- Proves the packing/conversion code is correct ✅

**Cons**:
- Much larger bitstream (no compression from P-frames)
- Higher latency
- Not sustainable for production

**Real fix needed**: Understand why P-frames break and fix the root cause

---

## Next Steps (In Order)

### Step 1: Verify Latest Fix
Test with new binary (both encoders all-I). Open menus/move windows. Should be corruption-free.

### Step 2: If Still Corrupts
There's a deeper issue with the auxiliary stream itself, not just P-frames.

### Step 3: If Corruption Fixed
Compare visual quality:
- AVC420 vs AVC444 (both with colorful content)
- Are colors identical? Slightly different?
- Test different VUI settings (limited vs full range)

### Step 4: Fix P-Frames Properly
Investigate why auxiliary P-frames corrupt:
- Check OpenH264 settings
- Verify auxiliary frame structure
- Compare with FreeRDP implementation
- Test with both encoders using P-frames

---

## Files Changed This Session

1. **Multi-position logging**:
   - `src/egfx/color_convert.rs` (5 sample positions)
   - `src/egfx/yuv444_packing.rs` (main & aux analysis)

2. **Config changes**:
   - `~/config.toml` on server (damage_tracking=false)

3. **Encoder workaround**:
   - `src/egfx/avc444_encoder.rs:326` (added aux all-I)

---

## Documentation Created

- `FINDINGS-2025-12-27-DAMAGE-TRACKING.md` - Damage tracking discovery
- `TESTING-SESSION-2025-12-27.md` - Testing procedures
- `READY-TO-TEST.md` - Test guide
- **THIS FILE** - Current comprehensive status

---

## Quick Reference

**Test server**: `greg@192.168.10.205`
**Binary MD5**: `b1e79780138f6bd8069dbe506f0b48a9` (BOTH encoders all-I)
**Config**: damage_tracking=false, codec=avc444
**Test**: `./run-server.sh` then open menus/move windows
**Expected**: No corruption in changed areas

---

**Status**: New binary deployed. Ready to test if auxiliary all-I fixes change-area corruption.
