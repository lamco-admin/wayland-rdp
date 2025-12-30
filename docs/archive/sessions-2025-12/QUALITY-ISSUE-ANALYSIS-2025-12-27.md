# RDP Quality Issue Analysis - 2025-12-27

## Summary

The RDP session exhibited **severe visual corruption** with pink/magenta artifacts and horizontal banding. This analysis identifies multiple contributing factors and their root causes.

## Visual Symptoms

From the screenshots:
- **Pink/magenta noise** covering large portions of the display
- **Horizontal banding/lines** across the image
- **Color distortion** - incorrect color rendering

## Log Analysis (kde-test-20251227-023535.log)

**Session Duration:** ~63 seconds (00:35:35 to 00:36:38)

### Frame Statistics

| Metric | Count | Notes |
|--------|-------|-------|
| Total MemFd buffer copies | 3010 | All buffers processed by PipeWire |
| Zero-size buffers from PipeWire | 82 | 2.7% of frames had no data |
| "copying 0 bytes" events | 82 | Matches zero-size count |
| Frames skipped as invalid | 24 | Validation caught only 29% of bad frames |
| AVC444 frames queued | 1138 | Successfully encoded frames |
| Encoder skipped frames | 68 | H.264 encoder internal skips |
| Frames dropped (backpressure) | 1818 | High drop rate |
| Frames sent | 1530 | Successfully transmitted |

### Critical Issues Identified

---

## Issue #1: Zero-Size Buffer Passthrough (CRITICAL)

**Location:** `lamco-pipewire/src/pw_thread.rs` lines 905-927

**Problem:** When PipeWire provides a buffer with `size=0`:
1. An empty `Vec<u8>` is created (`mapped_data[0..0].to_vec()`)
2. The stride mismatch warning is logged
3. The frame is **still created and sent** with empty data
4. Only 24 of 82 (29%) are caught by display_handler validation

**Root Cause:** The stride mismatch detection at line 905-909 **only warns** but does not skip the frame:

```rust
if actual_stride != calculated_stride {
    warn!("⚠️  Stride mismatch detected:");
    warn!("   Calculated: {} bytes/row", calculated_stride);
    warn!("   Actual: {} bytes/row (from buffer size)", actual_stride);
    warn!("   This may cause horizontal line artifacts!");
}
// BUG: No `return` or `continue` - frame still gets sent!

let frame = VideoFrame {
    data: StdArc::new(pixel_data),  // Empty data!
    ...
};
frame_tx_for_process.try_send(frame)  // Sent anyway!
```

**Impact:** 58 corrupted frames may have reached the encoder with empty or invalid data.

**Fix Required:** Add early return when stride mismatch is detected for zero-size buffers.

---

## Issue #2: Stride Calculation Division by Zero Risk

**Location:** `lamco-pipewire/src/pw_thread.rs` line 872

**Problem:** When `size = 0`:
```rust
let actual_stride = if expected_size as usize == size {
    calculated_stride
} else {
    (size / config.height as usize) as u32  // 0 / 800 = 0
};
```

**Impact:** Frame created with `stride: 0`, causing undefined behavior during color conversion.

---

## Issue #3: High Frame Drop Rate

**Statistics:**
- 1818 frames dropped vs 1530 sent (54% drop rate)
- Indicates backpressure in the processing pipeline

**Possible Causes:**
1. Encoder taking too long (~30ms for AVC444)
2. Network transmission delays
3. Queue depth configuration issues

---

## Issue #4: Initial Client Connection Failure

**Log Entry:**
```
2025-12-27T00:35:46.312078Z ERROR ironrdp_server::server: Connection error error=failed to accept client during finalize
    0: [read frame by hint] custom error
    1: Connection reset by peer (os error 104)
```

**Impact:** First connection attempt failed; client had to reconnect.

---

## Issue #5: Format Parameter Warning

**Log Entry:**
```
WARN lamco_pipewire::pw_thread: ⚠️  Format parameter building not working - using auto-negotiation
WARN lamco_pipewire::pw_thread: ⚠️  This may cause stream to not start on some compositors
```

**Impact:** PipeWire format negotiation may not be optimal.

---

## Configuration at Time of Test

```toml
[egfx]
enabled = true
codec = "avc420"         # Config says avc420
avc444_enabled = true    # But AVC444 is also enabled
h264_bitrate = 5000
qp_default = 23

[damage_tracking]
enabled = false          # Damage tracking disabled
```

**Note:** Despite `codec = "avc420"`, the logs show AVC444 encoding was active:
```
INFO lamco_rdp_server::egfx::handler: EGFX: AVC420 + AVC444v2 encoding enabled (V10+ capabilities)
```

---

## Recommended Fixes

### Immediate (Critical)

1. **Fix Zero-Size Buffer Handling** - Add early return in `pw_thread.rs`:
```rust
if actual_stride != calculated_stride {
    warn!("⚠️  Stride mismatch detected");
    // CRITICAL: Skip frame with corrupt stride
    if actual_stride == 0 {
        warn!("Skipping frame with zero stride (corrupted buffer)");
        return;  // Or continue in the callback
    }
}
```

2. **Add Size Validation Before Frame Creation** - Validate before creating VideoFrame:
```rust
let expected_min_size = (config.width * config.height * 4) as usize;
if pixel_data.len() < expected_min_size {
    warn!("Buffer too small: {} < {}, skipping", pixel_data.len(), expected_min_size);
    return;
}
```

### Short-Term

3. **Enable Damage Tracking** - May reduce processing load:
```toml
[damage_tracking]
enabled = true
```

4. **Investigate PipeWire Format Negotiation** - Fix the format parameter building warning

### Medium-Term

5. **Add Frame Validation Metrics** - Track and alert on validation failures
6. **Investigate High Drop Rate** - Profile pipeline to identify bottlenecks

---

## Test Environment

- **Server:** KDE Plasma on 192.168.10.205
- **Resolution:** 1280×800
- **Encoder:** OpenH264 (software) with AVC444
- **Color Matrix:** BT.709
- **Frame Rate Target:** ~27 fps (Level 3.2 constraint)

---

## Conclusion

The primary cause of visual corruption is **zero-size buffers from PipeWire being passed through to the encoder**. The `lamco-pipewire` crate correctly detects the stride mismatch but fails to prevent the corrupted frame from being processed.

Secondary factors include high frame drop rate and format negotiation issues, but these are less critical than the buffer validation bug.

**Priority:** Fix Issue #1 (zero-size buffer passthrough) before next deployment test.

---

## Appendix A: AVC444 Codec Versions (Reference)

The MS-RDPEGFX specification defines **two distinct versions** of the AVC444 codec, each with different reconstruction algorithms. Understanding this is critical for debugging color issues.

### AVC444 (Codec ID 0x0E) - Original

**Specification:** MS-RDPEGFX Section 3.3.8.3.2

**Algorithm Summary:**
- Two H.264 streams: Main View + Auxiliary View
- Both encoded as standard YUV420 (4:2:0)
- Client uses "basic YUV420p combination" to reconstruct YUV444

**Stream Structure:**
```
Stream 1 (Main View):
├── Y plane: Full luma at 1920×1080 (unchanged from source)
├── U plane: Subsampled chroma at 960×540 (2×2 box filter from U444)
└── V plane: Subsampled chroma at 960×540 (2×2 box filter from V444)

Stream 2 (Auxiliary View):
├── Y plane: Missing U444 samples (the 75% discarded by 4:2:0) packed as "fake luma"
├── U plane: Missing V444 samples packed as chroma
└── V plane: Neutral (128) or unused
```

**Client Reconstruction (Basic):**
```
For even pixels (where x%2==0 AND y%2==0):
    U = U_main (from subsampled chroma)
    V = V_main (from subsampled chroma)

For odd pixels (where x%2==1 OR y%2==1):
    U = aux_Y (auxiliary Y plane contains U444 residuals)
    V = aux_U (auxiliary U plane contains V444 residuals)
```

### AVC444v2 (Codec ID 0x0F) - Enhanced

**Specification:** MS-RDPEGFX Section 3.3.8.3.3

**Algorithm Summary:**
- Same dual-stream structure as AVC444
- **Different reconstruction method** with additional filtering
- "More complex reconstruction with improved quality"
- Better for smooth gradients and color transitions

**Key Differences from AVC444:**
- Adds filtering steps during reconstruction
- Different pixel combination weights
- Better quality for gradient content
- Known to have bugs in FreeRDP (Issue #11040)

### Current Implementation Status

| Aspect | Status | Notes |
|--------|--------|-------|
| IronRDP Protocol | ✅ Both supported | Codec types 0x0E and 0x0F |
| lamco-rdp-server Packing | ✅ Implemented | Using AVC444 (original) algorithm |
| Codec Type Sent | AVC444 (0x0E) | Set in ironrdp-egfx/server.rs:1197 |
| AVC444v2 Packing | ❌ Not implemented | Would require different algorithm |

### Potential Mismatch Issue

**CRITICAL:** If the packing algorithm doesn't match the codec type sent to the client, the reconstruction will produce wrong colors.

- **Symptom:** Colors correct on first frame, corrupted on subsequent changes
- **Cause:** Client interprets auxiliary stream data using wrong reconstruction algorithm
- **Diagnosis:** Compare our packing algorithm against MS-RDPEGFX Section 3.3.8.3.2

### Debugging Steps

1. **Verify Codec Type:** Check IronRDP is sending `Codec1Type::Avc444` (0x0E), not `Avc444v2` (0x0F)
2. **Verify Packing Algorithm:** Ensure our `pack_auxiliary_view()` matches Section 3.3.8.3.2
3. **Test P-frames:** Force all-keyframes to isolate P-frame issues
4. **Compare with FreeRDP:** FreeRDP's working implementation is the reference

### References

- MS-RDPEGFX: https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx
- Section 3.3.8.3.2: RDPGFX_AVC444_BITMAP_STREAM (original)
- Section 3.3.8.3.3: RDPGFX_AVC444V2_BITMAP_STREAM (enhanced)
- FreeRDP Issue #11040: AVC444v2 implementation bugs
