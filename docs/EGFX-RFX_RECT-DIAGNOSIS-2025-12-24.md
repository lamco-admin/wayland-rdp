# EGFX H.264 RFX_RECT Encoding Bug - Root Cause Analysis

**Date:** 2025-12-24
**Issue:** Windows RDP client crashes with error 0x1108 when receiving H.264 frames
**Status:** FIXED - Root cause identified and corrected

---

## Executive Summary

The Windows RDP client was crashing immediately upon receiving the first H.264 encoded frame through the EGFX channel. Through systematic analysis comparing our implementation with FreeRDP, the Microsoft OPN specification, and detailed packet inspection, we identified a critical bug in the RFX_RECT encoding within the RFX_AVC420_BITMAP_STREAM structure.

**The Bug:** IronRDP was encoding RFX_RECT regions as `(x, y, width, height)` when the specification requires `(left, top, right, bottom)` bounds format.

**Impact:** The Windows client received invalid rectangle bounds (e.g., right=1280 instead of right=1279), causing graphics pipeline validation failures and immediate disconnection.

---

## Timeline of Investigation

### Initial Symptoms
- Windows client connects successfully
- EGFX capability negotiation completes (V10.6)
- Surface setup PDUs (ResetGraphics, CreateSurface, MapSurfaceToOutput) sent successfully
- Client disconnects 15-35ms after receiving first H.264 frame with error 0x1108
- Zero FRAME_ACKNOWLEDGE PDUs received from client
- Backpressure stuck at 3 frames in flight

### Diagnostic Approach

1. **Added comprehensive logging** to trace exact PDU structures
2. **Inspected hex dumps** of RFX_AVC420_BITMAP_STREAM
3. **Compared with FreeRDP** server implementation
4. **Consulted Microsoft OPN specification** (official protocol notation)
5. **Verified with multiple evidence sources**

---

## Technical Analysis

### RFX_AVC420_BITMAP_STREAM Structure

Per MS-RDPEGFX section 2.2.4.4, the structure is:

```c
typedef struct {
    UINT32 numRegionRects;               // Number of regions (LE)
    RDPGFX_RECT16 regionRects[...];      // Region rectangles
    RDPGFX_AVC420_QUANT_QUALITY quantQualityVals[...];  // QP values
    BYTE avc420EncodedBitstream[...];    // H.264 NAL units (AVC format)
} RFX_AVC420_BITMAP_STREAM;
```

### RDPGFX_RECT16 Structure

**Microsoft OPN Specification:**
```opn
type RDPGFX_RECT16
{
    ushort left;      // Left bound (inclusive)
    ushort top;       // Top bound (inclusive)
    ushort right;     // Right bound (inclusive)
    ushort bottom;    // Bottom bound (inclusive)
}
```

**FreeRDP Implementation:**
```c
// rdpgfx_common.c:144
UINT rdpgfx_write_rect16(wStream* s, const RECTANGLE_16* rect16)
{
    Stream_Write_UINT16(s, rect16->left);   /* left (2 bytes) */
    Stream_Write_UINT16(s, rect16->top);    /* top (2 bytes) */
    Stream_Write_UINT16(s, rect16->right);  /* right (2 bytes) */
    Stream_Write_UINT16(s, rect16->bottom); /* bottom (2 bytes) */
    return CHANNEL_RC_OK;
}

// Client decode (rdpgfx_codec.c:82)
Stream_Read_UINT16(s, rect16->left);
Stream_Read_UINT16(s, rect16->top);
Stream_Read_UINT16(s, rect16->right);
Stream_Read_UINT16(s, rect16->bottom);
```

### Incorrect IronRDP Implementation (Before Fix)

**File:** `/home/greg/wayland/IronRDP/crates/ironrdp-egfx/src/pdu/avc.rs:90-98`

```rust
// WRONG: Converted to x, y, width, height
dst.write_u16(rectangle.left);                         // x = 0
dst.write_u16(rectangle.top);                          // y = 0
dst.write_u16(rectangle.right - rectangle.left + 1);   // width = 1280
dst.write_u16(rectangle.bottom - rectangle.top + 1);   // height = 800
```

**Hex dump of incorrect encoding:**
```
01 00 00 00  - numRegions = 1
00 00        - field1 = 0
00 00        - field2 = 0
00 05        - field3 = 0x0500 = 1280  ← WRONG (should be 1279)
20 03        - field4 = 0x0320 = 800   ← WRONG (should be 799)
```

### Correct Implementation (After Fix)

**File:** `/home/greg/wayland/IronRDP/crates/ironrdp-egfx/src/pdu/avc.rs:94-97`

```rust
// CORRECT: Direct bounds encoding
dst.write_u16(rectangle.left);    // left = 0
dst.write_u16(rectangle.top);     // top = 0
dst.write_u16(rectangle.right);   // right = 1279
dst.write_u16(rectangle.bottom);  // bottom = 799
```

**Expected hex dump after fix:**
```
01 00 00 00  - numRegions = 1
00 00        - left = 0
00 00        - top = 0
FF 04        - right = 0x04FF = 1279  ✓ CORRECT
1F 03        - bottom = 0x031F = 799  ✓ CORRECT
```

---

## Evidence Summary

| Source | Evidence | Format |
|--------|----------|--------|
| **Microsoft OPN Spec** | `type RFX_AVC420_METABLOCK { array<RDPGFX_RECT16> regionRects; }` | left, top, right, bottom |
| **Microsoft OPN Spec** | `type RDPGFX_RECT16 { ushort left; ushort top; ushort right; ushort bottom; }` | left, top, right, bottom |
| **FreeRDP Server** | `rdpgfx_write_rect16()` writes left, top, right, bottom | left, top, right, bottom |
| **FreeRDP Client** | `rdpgfx_read_rect16()` reads left, top, right, bottom | left, top, right, bottom |
| **IronRDP Original** | Used `InclusiveRectangle::encode()` → left, top, right, bottom | left, top, right, bottom ✓ |
| **IronRDP "Fixed"** | Changed to x, y, width, height calculation | x, y, width, height ✗ |
| **IronRDP Current** | Reverted to left, top, right, bottom | left, top, right, bottom ✓ |

---

## Why the Previous "Fix" Was Wrong

### Confusion Source

The confusion likely came from MS-RDPEGFX section 2.2.4.4 which documents the **RemoteFX** RFX_RECT structure (used in progressive codec), which DOES use x, y, width, height format.

However, for H.264/AVC420, the specification uses RDPGFX_RECT16 (section 2.2.4.2), NOT the RemoteFX RFX_RECT.

### Different Contexts

1. **TS_RFX_RECT** (RemoteFX Progressive) - Uses x, y, width, height
2. **RDPGFX_RECT16** (H.264 AVC420/AVC444) - Uses left, top, right, bottom

The RFX_AVC420_METABLOCK explicitly uses RDPGFX_RECT16, not TS_RFX_RECT.

---

## Verification

### Test Case: 1280x800 Frame

**Input:** `InclusiveRectangle { left: 0, top: 0, right: 1279, bottom: 799 }`

**Incorrect Encoding (Before Fix):**
```
Field 1: 0      (left → x)
Field 2: 0      (top → y)
Field 3: 1280   (right - left + 1 → width)  ← INVALID BOUND
Field 4: 800    (bottom - top + 1 → height) ← INVALID BOUND
```

**Correct Encoding (After Fix):**
```
Field 1: 0      (left)
Field 2: 0      (top)
Field 3: 1279   (right)   ← Valid inclusive bound
Field 4: 799    (bottom)  ← Valid inclusive bound
```

### SPS/PPS Parameters (from diagnostics)

```
SPS: 67 42 c0 20 8c 68 05 00 65 a0 1e 11 08 d4
     - Profile: 66 (Baseline)
     - Level: 3.2
     - Constraints: Constrained Baseline (c0)

PPS: 68 ce 3c 80
     - 4 bytes
```

These H.264 parameters are valid and compatible with Windows Media Foundation decoder.

---

## Code Changes

### File: `/home/greg/wayland/IronRDP/crates/ironrdp-egfx/src/pdu/avc.rs`

**Encode Function (lines 88-98):**
```rust
// OLD (WRONG):
dst.write_u16(rectangle.right - rectangle.left + 1);  // width
dst.write_u16(rectangle.bottom - rectangle.top + 1);  // height

// NEW (CORRECT):
dst.write_u16(rectangle.right);   // right bound
dst.write_u16(rectangle.bottom);  // bottom bound
```

**Decode Function (lines 126-141):**
```rust
// OLD (WRONG):
let x = src.read_u16();
let y = src.read_u16();
let width = src.read_u16();
let height = src.read_u16();
// ... convert to InclusiveRectangle

// NEW (CORRECT):
let left = src.read_u16();
let top = src.read_u16();
let right = src.read_u16();
let bottom = src.read_u16();
// ... direct assignment to InclusiveRectangle
```

---

## Test Results

### After Fix - Test 1 (2025-12-24 18:24)

**Hex dump verification:**
```
01 00 00 00  - numRegions = 1
00 00        - left = 0
00 00        - top = 0
ff 04        - right = 0x04FF = 1279  ✅ CORRECT!
1f 03        - bottom = 0x031F = 799  ✅ CORRECT!
```

**Connection behavior:**
1. ✅ Rectangle bounds are now valid (0,0,1279,799)
2. ✅ Connection doesn't crash immediately on first H.264 frame
3. ✅ Connection stayed alive 4+ seconds (vs 15-35ms before)
4. ✅ Successfully sent 142 H.264 frames
5. ❌ No FRAME_ACKNOWLEDGE PDUs received from client
6. ❌ Backpressure stuck at 3 frames in flight
7. ❌ Client eventually disconnects

### New Issue Identified: H.264 Level Constraint Violation

**SPS Parameters Decoded:**
```
Profile: 66 (Baseline)
Level: 3.2 (0x20 = 32)
Constraints: Constrained Baseline (c0)
```

**Level 3.2 Limits:**
- Max macroblocks/second: 108,000 (for frames > 1620 MBs)
- Max frame size: 5,120 macroblocks

**Our Configuration:**
- Resolution: 1280×800 = 4,000 macroblocks
- Framerate: 30 fps
- Total: 4,000 × 30 = **120,000 MB/s** ← **EXCEEDS 108,000 limit by 11%!**

**Hypothesis:**
Windows Media Foundation decoder may be rejecting the H.264 stream because it violates Level 3.2 constraints, preventing frame acknowledgements.

**Potential Solutions:**
1. Configure OpenH264 to use Level 4.0 (if possible via C API)
2. Reduce framerate to 27 fps (4000 × 27 = 108,000 MB/s exactly)
3. Test with 1280×720 resolution (3600 MBs @ 30fps = 108,000 MB/s)
4. Investigate if level validation can be disabled or overridden

---

## Testing Plan

1. **Deploy fixed binary** to test VM
2. **Connect with Windows mstsc**
3. **Monitor server logs** for:
   - Hex dump showing `FF 04 1F 03` (1279, 799) instead of `00 05 20 03` (1280, 800)
   - Frame ACK messages from client
   - Sustained streaming without disconnection
4. **Verify Windows client** stays connected and displays video
5. **Monitor backpressure** - should fluctuate rather than staying stuck at 3

---

## Related Issues Fixed in This Session

1. ✅ **Double annex_b_to_avc conversion** - Encoder already outputs AVC format
2. ✅ **Missing ResetGraphics PDU** - Now sent before CreateSurface
3. ✅ **RFX_RECT encoding bug** - This document's primary issue

---

## References

- **MS-RDPEGFX v18.1** - Remote Desktop Protocol: Graphics Pipeline Extension
- **Microsoft OPN Specification** - `riverar/messageanalyzer-archive/RDPEGFX.opn`
- **FreeRDP Implementation** - `FreeRDP/FreeRDP/channels/rdpgfx/`
  - Server: `rdpgfx_main.c:587 rdpgfx_write_h264_metablock()`
  - Client: `rdpgfx_codec.c:40 rdpgfx_read_h264_metablock()`
  - Common: `rdpgfx_common.c:144 rdpgfx_write_rect16()`

---

## Conclusion

The bug was caused by misinterpreting the RFX_RECT format. The code comments referenced MS-RDPEGFX 2.2.4.4 which documents the RemoteFX progressive codec's TS_RFX_RECT (x,y,width,height), but the H.264 AVC420 codec uses RDPGFX_RECT16 from section 2.2.4.2 (left,top,right,bottom).

Multiple authoritative sources (Microsoft OPN, FreeRDP encode/decode symmetry) confirm the bounds-based format is correct for AVC420.

**Fix applied:** Reverted RFX_RECT encoding to use direct bounds (left,top,right,bottom) instead of calculated dimensions (x,y,width,height).
