# AVC444 P-Frame Re-enablement Test - Status 2025-12-28

**Date**: 2025-12-28 12:34
**Status**: P-frames re-enabled, testing stride fix
**Previous Binary**: `d6868ffc21420d2355570524087fdaef` (all-I with stride fix)

---

## Journey Summary

### Phase 1: Identified P-Frame Corruption
- **Problem**: Lavender/brown corruption in changed areas with P-frames
- **Workaround**: Force all-I frames (perfect quality, larger bandwidth)

### Phase 2: Root Cause Investigation (Options 1-3)
- **Option 1**: Tested explicit `.fill(128)` → Didn't fix cycling
- **Option 2**: Added buffer diff logging → Found DATA region differences
- **Option 3**: Targeted position logging → Found input is actually changing

### Phase 3: Deep Analysis
- **Discovery**: Screen has real changing content (scrolling text, moving windows, UI)
- **Verification**: Position (329, 122) stable during certain periods, but other areas change
- **Finding**: Center (640, 400) mostly stable, but occasional changes

### Phase 4: Stride Mismatch Fix
- **Bug Found**: aux_u/aux_v allocated with PADDED dimensions but encoder told UNPADDED stride
- **Impact**: For 1280x800 no visual issue (640=640), but breaks variable resolutions
- **Fix Applied**: Removed padding from aux_u/aux_v, use chroma_width for all calculations

---

## All-I Workaround Validation

**User Testing Report** (2025-12-28 12:34):

✅ **Visual Quality**: Perfect, colors correct
✅ **Fast Scrolling Text**: Readable despite speed
✅ **Window Movement**: Smooth, correct colors
✅ **UI Interactions**: Right-click menus display properly
✅ **Responsiveness**: Relatively responsive

**Conclusion**: Our AVC444 implementation is fundamentally correct. Packing and color conversion work perfectly.

---

## P-Frame Re-enablement Test

### Code Changes

**File**: `src/egfx/avc444_encoder.rs:322-328`

**Before** (all-I workaround):
```rust
self.main_encoder.force_intra_frame();
self.aux_encoder.force_intra_frame();
```

**After** (P-frames enabled):
```rust
// self.main_encoder.force_intra_frame();
// self.aux_encoder.force_intra_frame();
```

### What Changed

**Stride Fix**:
- aux_u/aux_v now use `chroma_width * chroma_height` (no padding)
- Loop writes with `cy * chroma_width + cx` (not padded)
- `strides()` returns `(width, width/2, width/2)` matching buffer layout
- **Encoder and buffer now in sync!**

### Test Hypothesis

**If stride was causing P-frame corruption**:
- P-frames should work correctly now
- No lavender/brown artifacts
- Normal compression (20-40KB frames vs 110KB all-I)
- **Problem solved!**

**If stride wasn't the issue**:
- P-frames still show corruption
- Need to investigate:
  - Deblocking filter (corrupts chroma-as-luma?)
  - Dual-stream synchronization
  - Reference frame management
  - OpenH264 encoder settings

---

## Expected Test Results

### Success Indicators
- ✅ No lavender/brown corruption
- ✅ Text remains readable during scrolling
- ✅ Window movements show correct colors
- ✅ Right-click menus appear correctly
- ✅ Smaller bandwidth (check frame sizes in logs)

### Failure Indicators
- ❌ Lavender/brown macroblocks in changed areas
- ❌ Text becomes unreadable when scrolling
- ❌ Window movements show color artifacts
- ❌ Menu corruption

---

## Test Plan

1. **Deploy P-frame enabled binary**
2. **Connect via RDP**
3. **Perform actions**:
   - Scroll terminal text (fast)
   - Move windows around
   - Right-click to open menus
   - Type in terminal
4. **Observe for corruption**
5. **Check logs**:
   - Frame types (I vs P)
   - Frame sizes
   - Any error messages

---

## Next Binary

**File**: `target/release/lamco-rdp-server`
**Changes**:
- Stride fix (aux_u/aux_v no padding)
- P-frames RE-ENABLED (removed force_intra_frame)

**Testing**: Will determine if stride fix resolves P-frame corruption or if deeper investigation needed.

---

## Fallback Plan

If P-frames still show corruption with stride fix:

### Option A: Disable Deblocking for Auxiliary
- Auxiliary stream encodes chroma as luma
- H.264 deblocking filter designed for luma might corrupt chroma
- Need to access OpenH264 raw API (might require FFI)

### Option B: Investigate Dual-Stream Coordination
- Ensure both streams use matching:
  - Reference frames
  - Frame types (if main P, aux must be P with same ref)
  - Quantization parameters

### Option C: Keep All-I Workaround
- Perfect quality (user confirmed)
- Higher bandwidth (~110KB vs 20-40KB per frame)
- May be acceptable for target use case
- Could use selective I-frames (I every N frames)

---

## Files Modified This Session

1. **src/egfx/yuv444_packing.rs**:
   - Option 1: Explicit .fill(128)
   - Option 2: Buffer diff logging
   - Option 3: Targeted position logging
   - Stride fix: Removed padding from aux_u/aux_v

2. **src/egfx/color_convert.rs**:
   - Added cycling position to sample list

3. **src/egfx/avc444_encoder.rs**:
   - Re-enabled P-frames (commented out force_intra_frame)

4. **Documentation**:
   - DEPLOYMENT-WORKFLOW.md
   - BREAKTHROUGH-OPTION2-2025-12-28.md
   - ROOT-CAUSE-INVESTIGATION-2025-12-28.md
   - DEEP-ANALYSIS-STRIDE-BUG-2025-12-28.md
   - TEST-STATIC-SCREEN-INSTRUCTIONS.md
   - This file

---

## Ready for P-Frame Test

**Current Status**: Building P-frame enabled binary with stride fix

**Next**: Deploy and test for corruption
