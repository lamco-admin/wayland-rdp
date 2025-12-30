# AVC444 P-Frame Corruption Fix - Deployed

**Date**: 2025-12-27  
**Binary MD5**: `d1884c62d485d94a31ae88205f9f6495`  
**Status**: ✅ Deployed to greg@192.168.10.205:~/lamco-rdp-server  
**Ready for Testing**: Yes  

---

## Changes Implemented

### 1. Rewrote Auxiliary View Packing (B4/B5 Blocks)

**File**: `src/egfx/yuv444_packing.rs:289-367`

**Before (BROKEN)**:
- Pixel-level odd/even filtering
- Interpolation at even positions
- Frame-dependent values → P-frame corruption

**After (FIXED)**:
- Row-level macroblock packing
- Direct row copying from U444/V444 odd rows
- 16-row macroblock structure (rows 0-7: U444, rows 8-15: V444)
- No interpolation → temporal consistency

### 2. Updated Auxiliary U/V Packing (B6/B7 Blocks)

**File**: `src/egfx/yuv444_packing.rs:369-394`

**Changed to**:
- Direct sampling from U444/V444 at (odd_col, even_row)
- Matches FreeRDP implementation exactly
- No averaging, no interpolation

### 3. Removed Dead Code

**File**: `src/egfx/yuv444_packing.rs:405-413`

**Deleted**: `interpolate_even_position()` function (32 lines)
**Added**: Historical comment explaining why it was removed

### 4. Re-enabled P-Frames

**File**: `src/egfx/avc444_encoder.rs:247`

**Changed**: `force_all_keyframes: false` (was `true` for diagnostic)

---

## Technical Rationale

### The Bug
Interpolation created frame-dependent values at even positions in auxiliary Y plane:
- Frame N: `interpolate(neighbors at N)` → value A
- Frame N+1: `interpolate(neighbors at N+1)` → value B (different!)
- H.264 P-frame residual: `B - A` → artificial change
- Decoder applied residual to wrong baseline → lavender corruption

### The Fix
Row-level macroblock packing copies entire source rows:
- Auxiliary row 0 = U444 row 1 (complete row)
- Auxiliary row 1 = U444 row 3 (complete row)
- ...
- Auxiliary row 8 = V444 row 1 (complete row)
- Pattern repeats every 16 rows

**Result**: Direct source data (no computation) → perfect temporal consistency

### Why This Works
1. ✅ **Spec-compliant**: Matches MS-RDPEGFX Section 3.3.8.3.2 macroblock structure
2. ✅ **Client-compatible**: FreeRDP decoder expects row-based layout
3. ✅ **Temporally consistent**: Static content → identical auxiliary frames → zero P-frame residuals
4. ✅ **Efficient**: Simple memcpy operations, no interpolation overhead

---

## Expected Results

### Performance
- Encoding time: **20-25ms** per frame (vs 33ms with all-keyframes)
- Frame sizes: **20-40KB** typical (vs 110KB all-keyframes)
- Throughput: **30 fps** sustained (vs ~0.7 fps all-keyframes)

### Quality
- ✅ **No lavender corruption** in P-frames
- ✅ **Correct colors** throughout session
- ✅ **Smooth playback** without artifacts

---

## Testing Instructions

1. On VM (greg@192.168.10.205):
   ```bash
   ~/run-server.sh
   ```

2. From Windows client:
   - Connect via `mstsc.exe` to 192.168.10.205
   - Interact with desktop (move windows, type, scroll)
   - Observe for **60+ seconds**
   - Look for: lavender artifacts, color corruption

3. After test:
   - Copy log file back to dev machine for analysis
   - Check encoding times, frame sizes, error counts

---

## Success Criteria

✅ **No visual corruption** in changed areas
✅ **Performance**: Encoding time 20-30ms per frame  
✅ **Stability**: No encoder errors in logs
✅ **Efficiency**: P-frames smaller than I-frames

---

## If Issues Occur

**Corruption still present**:
- Check log for encoder errors
- Verify auxiliary U/V (B6/B7) implementation
- Consider macroblock row mapping formula

**Performance issues**:
- Check encoding times in log
- Verify SIMD color conversion is active
- Check frame drop rate

**Encoder errors**:
- Check YUV420 frame validation
- Verify source row bounds checking
- Look for stride/alignment issues

---

## Research Documentation

See `docs/AVC444-COMPREHENSIVE-RESEARCH-AND-FIX-2025-12-27.md` for complete analysis.

---

*Fix implemented: 2025-12-27*  
*Binary deployed: 2025-12-27*  
*Ready for testing*
