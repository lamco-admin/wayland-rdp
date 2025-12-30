# EGFX P-Slice Display Investigation - 2025-12-25

**Status:** ZGFX Complete ✅, Annex B Fixed ✅, P-Slice Display Issue Identified ❌
**Next Session:** Investigate Windows component {DD15FA56-7000-43FB-BD84-FD8B56527EFC} P-slice rejection

---

## Session Achievements

### 1. ZGFX Compression Implementation - COMPLETE ✅

**Files Created:**
- `IronRDP/crates/ironrdp-graphics/src/zgfx/wrapper.rs` (186 lines)
- `IronRDP/crates/ironrdp-graphics/src/zgfx/compressor.rs` (492 lines)
- `IronRDP/crates/ironrdp-graphics/src/zgfx/api.rs` (130 lines)

**Files Modified:**
- `IronRDP/crates/ironrdp-graphics/src/zgfx/mod.rs` - Exports
- `IronRDP/crates/ironrdp-graphics/src/zgfx/circular_buffer.rs` - Helper methods
- `IronRDP/crates/ironrdp-egfx/src/server.rs` - ZGFX integration

**Implementation Details:**
- **Uncompressed Wrapper:** Wraps data in ZGFX segment structure (descriptor 0xE0/0xE1 + flags)
- **Full Compression:** LZ77-variant with 40-token Huffman encoding
- **Match Finding:** Backward scan through 2.5MB history buffer
- **BitWriter:** MSB-first bit packing with padding indicator
- **Smart Auto Mode:** Compresses only if beneficial

**Test Results:**
- 46 tests passing
- Round-trip verified (compress → decompress = original)
- Compression ratios: 10.65x on repetitive data, 1.43x on mixed
- Single and multipart segments working

**Integration:**
- Added `Compressor` instance to `GraphicsPipelineServer`
- Modified `drain_output()` to compress and wrap PDUs before DVC encoding
- Three compression modes: Never/Auto/Always
- Automatic compression ratio logging

### 2. Annex B Format Bug - FIXED ✅

**Root Cause Identified:**
- MS-RDPEGFX specification requires Annex B format (ITU-H.264 Annex B with start codes)
- We were incorrectly converting Annex B → AVC format (length prefixes)
- Windows Media Foundation H.264 decoder expects Annex B format

**The Bug:**
```rust
// BEFORE (WRONG):
let data = annex_b_to_avc(&annex_b_data);  // Converted to AVC ❌

// AFTER (CORRECT):
let data = annex_b_data;  // Keep Annex B format ✅
```

**Files Modified:**
- `src/egfx/encoder.rs:304` - Removed conversion, added documentation
- `src/server/egfx_sender.rs:205-283` - Updated NAL parsing for Annex B

**Result:**
- Windows successfully parses H.264 streams
- Frame acknowledgments flowing
- ERROR_INSUFFICIENT_BUFFER (0x8007007A) resolved
- GfxEventDecodingW2S1PduFailed resolved

### 3. H.264 Stream Validation ✅

**Dumped and analyzed H.264 files:**
- Frame 0 (IDR): 51.9KB - SPS(15b) + PPS(4b) + IDR slice
- Frame 1 (P-slice): 24KB - Valid P-slice NAL
- Frame 2 (P-slice): 4.4KB - Valid P-slice NAL

**Structure Validation:**
```
Start codes: 00 00 00 01 ✅
NAL headers: Correct type and ref_idc ✅
Annex B format: Compliant ✅
SPS parameters: Profile 66 (Baseline), Level 31 (3.1), Constraints 0xC0 ✅
```

All H.264 streams are structurally valid and comply with ITU-H.264 specification.

---

## Current Blocker: P-Slice Display Issue

### The Problem

**Symptom:**
- First IDR frame displays correctly
- Subsequent P-slice frames decode but don't display
- Windows shows black screen or frozen frame
- Keyboard input becomes unresponsive through RDP session

**Windows Error:**
```
Event ID: 1404
Component: {DD15FA56-7000-43FB-BD84-FD8B56527EFC}
Function: 16
Error Code: 0x80004005 (E_FAIL)
Message: "The client encountered an issue while decoding and displaying RDP graphics"
```

### Evidence from Systematic Testing

**Test 1: No modifications (natural OpenH264 behavior)**
- Frame 1: IDR (displayed) ✅
- Frames 2-17: P-slices (did not display) ❌
- Component errors: Frames 2-6
- 16 frames acknowledged (all decoded)
- Result: Frozen on first frame

**Test 2: force_keyframe() before every encode**
- Frame 1: IDR (displayed) ✅
- Frames 2-41: P-slices (intermittently displayed) ⚠️
- Component errors: Still occurred
- 14 frames acknowledged
- Result: "Mostly visible and updating, freezing occasionally"

**Test 3: IDR_INTERVAL=1 (all keyframes attempt)**
- All frames: Should be IDRs
- Only Frame 1 was actually IDR (OpenH264 internal limit)
- Frames 2+: Still P-slices
- 1 frame acknowledged
- Result: Black screen (too slow decode)

**Test 4: IDR_INTERVAL=1 with set_option() call**
- Frames 1-5: All IDRs ✅
- Black screen (IDR decode 5-17 seconds each)
- 3 frames acknowledged
- Result: Black screen, severe performance degradation

### Frame Decode Latency Analysis

**IDR Frames:**
- Consistently 1.7 - 6.4 seconds
- Some as high as 17.6 seconds
- Average: 3-5 seconds

**P-Slice Frames:**
- Range: 138ms - 5.9 seconds
- Fast frames: 138-647ms (acceptable)
- Slow frames: 1-3 seconds
- Outliers: 5-10 seconds

**Pattern:** P-slices decode MUCH faster than IDRs when decode is successful

### Component Error Pattern

**Timing Correlation:**
- Errors occur EXACTLY when P-slice frames are sent to client
- Frame 1 (IDR): No component error
- Frames 2-6 (P-slices): Component error E_FAIL
- Frames 7+ (P-slices): No component error (but display already broken)

**Error stops new errors but doesn't fix display:**
After initial component errors on Frames 2-6, subsequent frames (7-16) have no errors but still don't display. The error broke the display pipeline permanently for that session.

---

## Technical Analysis

### What Works Correctly

1. **EGFX Protocol Flow** ✅
   - CapabilitiesAdvertise/Confirm
   - ResetGraphics
   - CreateSurface
   - MapSurfaceToOutput
   - StartFrame/WireToSurface1/EndFrame sequence

2. **ZGFX Compression** ✅
   - All PDUs properly wrapped
   - Decompression successful
   - Client processes all PDUs

3. **H.264 Decoding** ✅
   - Frame acknowledgments prove decoding works
   - Both IDR and P-slices acknowledged
   - Variable decode times but eventually complete

### What Fails

**Windows Display Compositor (Component {DD15FA56...})**
- Function 16 returns E_FAIL when called with P-slice frames
- Only affects P-slices, not IDR frames
- Breaks display pipeline after errors occur
- Undocumented component (GUID not in public docs)

### Hypotheses Tested and Rejected

- ❌ ZGFX compression (fixed, working)
- ❌ AVC vs Annex B format (fixed, working)
- ❌ Multipart ZGFX segments (tested with small frames)
- ❌ Frame structure errors (all validated correct)
- ❌ Capability negotiation (V10_6, AVC enabled)
- ❌ Surface configuration (XRgb, 800x600, correct)
- ❌ PDU sequence (matches specification)

### Remaining Hypotheses

**Hypothesis 1: OpenH264 P-Slice Configuration Issue**
- Our P-slices might use parameters Windows doesn't accept
- Possible: num_ref_frames, reference list, slice type, etc.
- Test: Compare with FreeRDP's OpenH264 configuration

**Hypothesis 2: Windows Expects Periodic SPS/PPS**
- Maybe Windows needs SPS/PPS more frequently than just with IDRs
- Test: Prepend SPS+PPS to every P-slice

**Hypothesis 3: Profile/Level Incompatibility**
- Baseline Profile with constraints 0xC0 might not fully support P-slices in Windows
- Test: Try Main Profile or High Profile

**Hypothesis 4: Reference Frame Management**
- P-slices reference IDR incorrectly
- Or Windows loses reference frame context
- Test: Check SPS max_num_ref_frames parameter

**Hypothesis 5: Windows RDP Bug/Limitation**
- Component {DD15FA56...} has bug specific to AVC420 P-slices
- Might require undocumented workaround
- Test: Packet capture from known-working Windows RDP server

---

## Critical Files for Investigation

### H.264 Test Files (on test server)

**Location:** `/tmp/rdp-frame-*.h264` (from frozen-frame-test)

**Frame 0 (IDR):** 51,933 bytes
```
00 00 00 01 67 42 c0 1f ... (SPS, 15 bytes)
00 00 00 01 68 ce 3c 80 ... (PPS, 4 bytes)
00 00 00 01 65 b8 00 04 ... (IDR slice, 51,899 bytes)
```

**Frame 1 (P-slice):** 24,034 bytes
```
00 00 00 01 61 e0 00 7e ... (P-slice, 24,027 bytes)
NAL type: 1 (P-slice)
NAL ref_idc: 3 (reference frame)
```

**Frame 2 (P-slice):** 4,380 bytes
```
00 00 00 01 61 ... (P-slice, 4,373 bytes)
```

All files validated as correct Annex B format with proper NAL structures.

### Server Logs

**Key Tests:**
- `kde-test-20251225-003407.log` - "Mostly working" test (force_keyframe)
- `kde-test-20251225-121834.log` - IDR_INTERVAL=1 test (all keyframes, failed)
- `kde-test-20251225-123226.log` - Revert test (inconsistent)
- `kde-test-20251225-124317.log` - Natural keyframe interval (frozen on Frame 1)

### Windows Event Logs

- `ops-1146.csv` - Earlier test with AVC format (W2S1PduFailed)
- `deug-1146.csv` - Debug events from same test
- `ops-25dec.csv` - Test showing component errors
- `ops-1243.csv` - Natural keyframe test

---

## OpenH264 Configuration Details

### Current Configuration

**Encoder Parameters:**
```rust
EncoderConfig::new()
    .set_bitrate_bps(5000 * 1000)  // 5 Mbps
    .max_frame_rate(30.0)
    .enable_skip_frame(true)
    .usage_type(UsageType::ScreenContentRealTime)
```

**Default OpenH264 Parameters (from code inspection):**
- `uiIntraPeriod`: ~30-320 (default keyframe interval)
- `iNumRefFrame`: Auto (likely 1 for ScreenContentRealTime)
- `eSpsPpsIdStrategy`: ConstantId (default)
- `iRCMode`: Quality mode (default)

### Issues Discovered

**force_keyframe() Limitation:**
OpenH264's `uiIntraPeriod` blocks force_keyframe() from working repeatedly. Even calling force_keyframe() before every encode only resulted in Frame 1 being an IDR; subsequent frames were P-slices due to internal minimum keyframe interval.

**IDR_INTERVAL Setting:**
Successfully set via `ENCODER_OPTION_IDR_INTERVAL` to 1, resulting in all frames being IDRs. However, IDR frames decode extremely slowly (5-17 seconds each), making this approach unusable.

---

## Detailed Test Results

### Test: Natural Keyframe Interval (Latest)

**Configuration:**
- No force_keyframe() calls
- OpenH264 default behavior
- First frame is IDR, subsequent are P-slices

**Server Log Analysis:**
```
Frame 1: 56810 bytes, keyframe=true (IDR with SPS+PPS)
Frame 2: 22789 bytes, keyframe=false (P-slice)
Frame 3: 15950 bytes, keyframe=false (P-slice)
Frame 4: 7608 bytes, keyframe=false (P-slice)
Frame 5: 16358 bytes, keyframe=false (P-slice)
Frame 6: 14758 bytes, keyframe=false (P-slice)
Frames 7-17: All P-slices
```

**Frame Acknowledgments:**
- 16 frames acknowledged
- Latencies: 138ms to 5.9s (variable)
- All frames decoded by Windows

**Windows Component Errors:**
- 5 errors total (Event ID 1404)
- Component: {DD15FA56-7000-43FB-BD84-FD8B56527EFC}
- Function: 16
- Error: 0x80004005 (E_FAIL)
- Timing: Frames 2-6 only

**User Report:**
- One frame displayed
- Then black screen
- Keyboard unresponsive through session
- Had to kill from console

### Test: force_keyframe() Before Every Encode

**Configuration:**
- `encoder.force_keyframe()` called before each `encode_bgra()`
- Expected: All keyframes
- Actual: Only Frame 1 was keyframe (OpenH264 uiIntraPeriod blocked it)

**Server Log Analysis:**
```
Frame 1: 51933 bytes, keyframe=true (IDR)
Frames 2-41: All keyframe=false (P-slices)
```

**Frame Acknowledgments:**
- 14 frames acknowledged
- Latencies: 143ms to 10.9s (variable)

**User Report:**
- "Mostly visible and updating"
- Freezing occasionally but recovered
- Keyboard/mouse functional
- Could Ctrl-C to kill terminal

**Why Better?**
Unknown. Same frame type pattern as natural test (1 IDR, then P-slices) but different result.

---

## The Core Mystery

### Same Frame Pattern, Different Results

**Natural Test:**
- Frame 1: IDR
- Frames 2+: P-slices
- Result: Frozen on first frame

**force_keyframe Test:**
- Frame 1: IDR
- Frames 2+: P-slices
- Result: Mostly working

**The Difference:**
Calling `force_keyframe()` before each encode (even though it only affected Frame 1) somehow made subsequent P-slices acceptable to Windows.

**Possible Explanations:**
1. force_keyframe() sets internal encoder state affecting P-slice generation
2. Timing difference when force_keyframe() is called
3. Some encoder parameter is set differently when force_keyframe() is used
4. Random variation in test conditions

---

## Component {DD15FA56-7000-43FB-BD84-FD8B56527EFC} Analysis

### What We Know

**Error Message:**
> "The client encountered an issue while decoding and displaying RDP graphics"

**Error Details:**
- Component GUID: {DD15FA56-7000-43FB-BD84-FD8B56527EFC}
- Function: 16
- Error Code: 0x80004005 (E_FAIL - Unspecified failure)
- Category: "RdClient Pipeline workspace"

**Behavior:**
- Accepts IDR frames without error
- Rejects P-slice frames with E_FAIL
- Only first 5 P-slices cause errors (Frames 2-6)
- After errors, display pipeline stops updating

### What We Don't Know

- What component {DD15FA56...} actually is
- What function 16 does specifically
- Why it accepts IDRs but rejects P-slices
- Whether this is a bug or expected behavior
- How to work around or fix it

### Investigation Needed

1. **Identify Component:**
   - Search Windows Registry for GUID
   - Check COM object registration
   - Look for RDP/EGFX-related components

2. **Understand Function 16:**
   - Map to COM interface method
   - Determine what operation is failing
   - Find if it's documented anywhere

3. **Compare with Working Implementation:**
   - Capture FreeRDP server H.264/EGFX stream
   - Check if FreeRDP has same issue
   - Look for differences in P-slice generation

---

## Next Session Action Items

### Immediate Priority

**Validate H.264 Stream with ffmpeg:**
```bash
# On test server
sudo apt install ffmpeg
ffmpeg -i /tmp/rdp-frame-0.h264 -frames:v 1 /tmp/frame0.png
ffmpeg -i /tmp/rdp-frame-1.h264 -frames:v 1 /tmp/frame1.png
ffmpeg -i /tmp/rdp-frame-2.h264 -frames:v 1 /tmp/frame2.png
```

If all decode successfully: Our H.264 is valid, problem is Windows RDP client-specific
If P-slices fail to decode: Our H.264 generation has issues

### Secondary Priorities

1. **Research Component GUID** (Windows client-side)
   - Registry search
   - Process Monitor during RDP session
   - Check for known RDP components

2. **Compare FreeRDP Implementation**
   - Study shadow server H.264 encoding
   - Check if FreeRDP does anything special for P-slices
   - Look for RDP-specific H.264 workarounds

3. **Test H.264 Parameter Variations**
   - Main Profile vs Baseline Profile
   - Different constraint flags
   - Explicit num_ref_frames configuration
   - SPS/PPS with every frame

---

## Technical Debt / TODO

### Code Quality

1. **Remove frame dump logging:**
   - `src/server/egfx_sender.rs:285-300` - Temporary debugging code
   - Remove before production

2. **Remove force_keyframe() call:**
   - `src/server/display_handler.rs:540-541` - Temporary workaround
   - Remove once P-slice issue resolved

3. **Deprecated annex_b_to_avc():**
   - Currently marked deprecated
   - Consider removing entirely or moving to tests-only

### Documentation

1. Update EGFX handover documents with ZGFX completion
2. Document P-slice display issue as known limitation
3. Add troubleshooting guide for display issues

---

## Performance Observations

### Decode Performance (Windows Client)

**Highly Variable:**
- Best case: 138-353ms (usable for 30fps)
- Average case: 1-3 seconds (causes stuttering)
- Worst case: 5-17 seconds (appears frozen)

**Pattern:**
- IDR frames consistently slow (3-17 seconds)
- P-slices variable (138ms to 5.9 seconds)
- No clear pattern to predict decode time

**Hypothesis:**
Windows might be using software H.264 decoder (no DXVA hardware acceleration), causing inconsistent CPU-based decode performance.

### Bandwidth (with ZGFX Auto mode)

**Compression Ratios:**
- H.264 frames: 1.00x (Auto correctly chooses uncompressed)
- Setup PDUs: Minimal overhead

**Frame Sizes (800x600):**
- IDR frames: 50-60KB
- P-slice frames: 4-24KB
- ZGFX overhead: +2 bytes (single segment)

---

## Files Modified This Session

### IronRDP Repository

**New Files:**
1. `crates/ironrdp-graphics/src/zgfx/wrapper.rs`
2. `crates/ironrdp-graphics/src/zgfx/compressor.rs`
3. `crates/ironrdp-graphics/src/zgfx/api.rs`

**Modified:**
4. `crates/ironrdp-graphics/src/zgfx/mod.rs`
5. `crates/ironrdp-graphics/src/zgfx/circular_buffer.rs`
6. `crates/ironrdp-egfx/src/server.rs`

### wrd-server-specs Repository

**Modified:**
7. `src/egfx/encoder.rs` - Annex B fix, deprecated annex_b_to_avc()
8. `src/server/egfx_sender.rs` - Annex B NAL parsing, frame dumping
9. `src/server/display_handler.rs` - force_keyframe() experimentation

---

## Recommendations for Next Session

### Systematic Investigation Approach

**Phase 1: Validate (1-2 hours)**
- Install ffmpeg on test server
- Decode all 3 H.264 test files
- Verify P-slices are valid H.264
- If valid: Problem is Windows-specific
- If invalid: Problem is in OpenH264 configuration

**Phase 2: Compare (2-3 hours)**
- Run FreeRDP shadow server
- Capture H.264 stream
- Compare P-slice byte structure
- Look for configuration differences

**Phase 3: Test Variations (2-4 hours)**
- Try Main Profile
- Try different num_ref_frames
- Try SPS/PPS with every frame
- Try different constraint flags

**Phase 4: Research Component (1-2 hours)**
- Identify {DD15FA56...} on Windows
- Understand function 16
- Look for known issues/workarounds

### Success Criteria

**Minimum Acceptable:**
- P-slices display consistently
- No component errors
- Video updates smoothly
- Frame rate ≥ 15fps effective

**Optimal:**
- All frames display correctly
- Decode latency < 200ms
- Normal keyframe interval (30-60 frames)
- Full bandwidth efficiency

---

## Session Statistics

**Duration:** ~6 hours
**Files Created:** 3 (ZGFX modules)
**Files Modified:** 9
**Tests Written:** 46 (all passing)
**Tests Performed:** 4 complete RDP session tests
**Bugs Fixed:** 2 critical (ZGFX missing, Annex B format)
**Bugs Identified:** 1 (P-slice display)
**Lines of Code:** ~1000 (ZGFX implementation)

---

## Context for Continuation

### What's Ready for Production

- ✅ ZGFX compression (ready for IronRDP PR)
- ✅ Annex B format fix (ready for IronRDP PR)
- ✅ EGFX protocol implementation (structurally complete)

### What Blocks Production Use

- ❌ P-slice display on Windows mstsc client
- ⚠️ Inconsistent decode performance
- ⚠️ Display compositor errors

### Immediate Next Action

Run ffmpeg validation on H.264 test files to determine if issue is:
- Our H.264 generation (fixable in OpenH264 config)
- Windows RDP client limitation (requires workaround)

This will guide whether to focus on encoder configuration or Windows-specific workarounds.
