# AVC444 Lavender Color Corruption - Debug Handover

**Date:** 2025-12-27
**Status:** Active debugging, P-frame hypothesis being tested
**Priority:** Critical - blocking AVC444 release

---

## Problem Statement

AVC444 encoding produces **lavender/purple color corruption**. The user reports:
- **"Original colors of the desktop are OK"** - First frame appears correct
- **"It's the change that is wrong"** - When content changes, changed areas turn lavender

This suggests the issue is specific to P-frames (delta encoding) or damage tracking.

---

## Test Environment

- **Server:** Remote VM (not local machine)
- **Deployment path:** `/home/greg/lamco-rdp-server` (single binary, not a directory)
- **Build command:** `cargo build --release` from `/home/greg/wayland/wrd-server-specs`
- **Deploy command:**
  ```bash
  rm -f /home/greg/lamco-rdp-server
  cp target/release/lamco-rdp-server /home/greg/lamco-rdp-server
  ```
- **DO NOT start the server** - user handles that on the remote VM
- **Client:** Windows mstsc.exe with AVC444 codec

---

## What Has Been Tested (All Produced Same Lavender)

### 1. Color Conversion Tests
| Test | Change | Result |
|------|--------|--------|
| B/R swap in BGRA→YUV | Swapped `bgra[0]` and `bgra[2]` | Same lavender |
| Scalar vs SIMD | Forced `use_simd = false` | Same lavender |
| BT.709 full range | Used BT.709 coefficients | Same lavender |
| OpenH264 limited range | Used limited range (Y:16-235) | Same lavender |

**Conclusion:** B/R swap producing identical lavender proves the issue is NOT in color conversion.

### 2. Packing Tests
| Test | Change | Result |
|------|--------|--------|
| UV swap in aux stream | Swapped U and V in auxiliary view | Same lavender |

**Conclusion:** UV swap producing identical lavender proves the issue is NOT in U/V channel assignment in packing.

### 3. VUI Signaling Tests
| Test | Change | Result |
|------|--------|--------|
| VUI enabled (full range) | Signaled full range in SPS | Same lavender |
| VUI disabled | No VUI signaling | Same lavender |

**Conclusion:** VUI signaling is not the cause.

---

## Current Diagnostic Build

**MD5:** `322bd8d195bf99b48a082816c50ce602`

**Changes in this build:**
1. `force_all_keyframes = true` in `src/egfx/avc444_encoder.rs:247`
2. Forces both encoders to produce IDR frames on every encode
3. Log message: `🔧 DIAGNOSTIC: force_all_keyframes=true - All frames will be IDR`

**Purpose:** If colors are correct with all-keyframes, the issue is P-frame specific. If still lavender, the issue is in the base algorithm.

**User reported:** "it's exactly the same" - meaning still lavender with all-keyframes.

**If confirmed:** This would prove the issue is NOT P-frame specific, and is in the base encoding/packing/protocol.

---

## Current Code State

### Key Files Modified

1. **`src/egfx/avc444_encoder.rs`**
   - Line 185: `ColorMatrix::OpenH264` (limited range)
   - Line 247: `force_all_keyframes: true` (DIAGNOSTIC)
   - Lines 312-320: Force IDR logic before encoding

2. **`src/egfx/color_convert.rs`**
   - Line 185: `let use_simd = false;` (FORCED SCALAR)
   - BGRA→YUV444 conversion using OpenH264 matrix

3. **`src/egfx/yuv444_packing.rs`**
   - `pack_auxiliary_view_spec_compliant()` - Current algorithm
   - aux.Y = U444 at odd positions
   - aux.U = V444 samples (subsampled)
   - aux.V = neutral (128)

### IronRDP Codec Type
- File: `/home/greg/wayland/IronRDP/crates/ironrdp-egfx/src/server.rs:1197`
- Uses: `Codec1Type::Avc444` (0x0E) - NOT AVC444v2

---

## Two AVC444 Versions (Documented)

See `docs/QUALITY-ISSUE-ANALYSIS-2025-12-27.md` Appendix A for full details.

| Version | Codec ID | Spec Section | Notes |
|---------|----------|--------------|-------|
| AVC444 | 0x0E | 3.3.8.3.2 | Original, simpler reconstruction |
| AVC444v2 | 0x0F | 3.3.8.3.3 | Enhanced, different combination method |

**Current implementation:** AVC444 (0x0E) packing with AVC444 codec type.

---

## Remaining Hypotheses

### If all-keyframes still produces lavender:

1. **OpenH264 encoding issue** - Something in how OpenH264 encodes the aux stream
2. **Packing algorithm mismatch** - Our algorithm doesn't match MS-RDPEGFX Section 3.3.8.3.2
3. **Wire format issue** - IronRDP's Avc444BitmapStream encoding is wrong
4. **Windows client bug** - Client's AVC444 reconstruction is broken

### Next Steps to Try

1. **Verify diagnostic is running** - Check log for `force_all_keyframes=true` message
2. **Compare with FreeRDP** - Look at FreeRDP's AVC444 packing implementation
3. **Test AVC420** - Does simple AVC420 (not AVC444) produce correct colors?
4. **Dump raw YUV** - Save YUV444 to file and inspect visually
5. **Binary stream comparison** - Compare H.264 NAL units with working implementation

---

## File Locations

| Purpose | Path |
|---------|------|
| Main project | `/home/greg/wayland/wrd-server-specs` |
| Built binary | `target/release/lamco-rdp-server` |
| Deploy target | `/home/greg/lamco-rdp-server` |
| IronRDP fork | `/home/greg/wayland/IronRDP` |
| AVC444 encoder | `src/egfx/avc444_encoder.rs` |
| Color conversion | `src/egfx/color_convert.rs` |
| YUV444 packing | `src/egfx/yuv444_packing.rs` |
| Quality analysis doc | `docs/QUALITY-ISSUE-ANALYSIS-2025-12-27.md` |

---

## Build and Deploy Workflow

```bash
# Build
cd /home/greg/wayland/wrd-server-specs
cargo build --release

# Deploy (user restarts server on remote VM)
rm -f /home/greg/lamco-rdp-server
cp target/release/lamco-rdp-server /home/greg/lamco-rdp-server
md5sum /home/greg/lamco-rdp-server

# DO NOT start server - user does that on remote VM
```

---

## Key Insight

**B/R swap and UV swap both producing identical lavender** is the critical clue. This proves:
- The issue is NOT in our BGRA→YUV color conversion
- The issue is NOT in our U/V channel packing

The lavender must be coming from:
1. OpenH264's internal processing
2. The IronRDP protocol encoding
3. The Windows client decoder/reconstruction

---

## Questions for Next Session

1. Did the log show `force_all_keyframes=true`? (Confirms new code is running)
2. With all-keyframes, are colors still wrong? (Isolates P-frame vs base issue)
3. Does AVC420 produce correct colors? (Isolates AVC444-specific issue)
4. Can we capture the H.264 NAL units for analysis?

---

## Todo List State

- [x] Diagnose AVC444 corruption issue
- [x] Fix color matrix in AVC444 encoder
- [x] Fix SIMD dispatch for limited range
- [x] Switch to openh264 fork with VUI support
- [x] Update AVC444 encoder to use VuiConfig
- [x] Rebuild and redeploy
- [x] Update documentation for VUI support
- [x] Verify fix with testing (FAILED - still lavender)
- [x] Try UV swap in auxiliary stream packing (same result)
- [x] Try B/R swap in color conversion (same result)
- [x] Document AVC444 vs AVC444v2
- [ ] **Test with all-keyframes to isolate P-frame issue** (IN PROGRESS)
- [ ] If still lavender: Compare with FreeRDP implementation
- [ ] If still lavender: Test AVC420 to isolate AVC444-specific issue
