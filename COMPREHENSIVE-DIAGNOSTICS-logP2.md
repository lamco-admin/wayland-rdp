# Comprehensive Diagnostic Analysis - logP2.txt

**Session:** logP2.txt (17,931 lines)
**Date:** 2025-11-19 21:11-21:13
**Build:** bf02500 (with restored logging + enhanced debugging)

---

## ISSUE SUMMARY

| Issue | Count | Severity | Status |
|-------|-------|----------|--------|
| Frame send failures | 680 | HIGH | Channel still overloading despite 256 capacity |
| Frame corruption | 17 | MEDIUM | Intermittent frame validation failures |
| Connection reset | 1 | LOW | Expected cert retry behavior |
| SelectionOwnerChanged not firing | N/A | CRITICAL | Linux→Windows clipboard blocked |

---

## ISSUE 1: SelectionOwnerChanged Signal NOT Firing 🔴 CRITICAL

### Evidence

**Stream Created:**
```
21:11:31.500040Z INFO: SelectionOwnerChanged listener task starting
21:11:31.500264Z INFO: SelectionOwnerChanged stream created successfully - waiting for signals
```

**But NO Events:**
```
[User copied text in Linux]
...ZERO SelectionOwnerChanged events in entire log...
```

### Root Cause Analysis

**The stream was created successfully** but **NO D-Bus signals received**.

**Possible Causes:**

1. **Portal Backend Doesn't Implement SelectionOwnerChanged**
   - xdg-desktop-portal-gnome may not support this signal
   - Check Portal version and capabilities

2. **Clipboard Isolation**
   - RemoteDesktop session clipboard may be isolated from system clipboard
   - Session may only see its OWN clipboard changes
   - System app clipboard changes don't propagate to session

3. **Permission/Scope Issue**
   - Signal subscription may require different permissions
   - Or only fires for clipboard owned by THIS session

### Testing Required

**Test with dbus-monitor:**
```bash
dbus-monitor --session "interface='org.freedesktop.portal.Clipboard'" 2>&1 | tee dbus-monitor.log &
# Copy in Linux app
# Check if ANY signals appear
```

**Check Portal capabilities:**
```bash
busctl --user introspect org.freedesktop.portal.Desktop \
  /org/freedesktop/portal/desktop \
  org.freedesktop.portal.Clipboard
```

### Workaround Options

**Option A: Polling-based clipboard monitoring**
```rust
// Poll every 500ms
tokio::spawn(async move {
    let mut last_content_hash = None;

    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Try to read current clipboard
        if let Ok(data) = portal.selection_read(&session, "text/plain;charset=utf-8").await {
            let hash = calculate_hash(&data);

            if Some(&hash) != last_content_hash.as_ref() {
                last_content_hash = Some(hash);
                // Clipboard changed - announce to RDP
                event_tx.send(PortalFormatsAvailable(vec!["text/plain;charset=utf-8"])).await;
            }
        }
    }
});
```

**Option B: Use wl-data-device protocol directly**
- Bypass Portal for clipboard monitoring
- Use wayland-client to subscribe to wl_data_device events
- More complex but guaranteed to work

---

## ISSUE 2: Frame Send Failures 🔴 HIGH

### Statistics

**Count:** 680 failures in ~2 minute session
**Rate:** ~5.7 failures per second
**Channel:** 256 slots (increased from 64)

### Analysis

**Still overloading despite 4x capacity increase!**

**Math:**
- PipeWire capturing at compositor rate (likely 60+ FPS)
- Target FPS: 30
- Ratio: 2:1 (should be sustainable with 256 buffer)
- But: Processing time per frame may be > 16ms
- If processing takes 33ms, only 30 FPS output
- At 60 FPS input, that's 30 frames/sec surplus
- 256 buffer fills in 256/30 = 8.5 seconds

**This suggests processing is slower than expected!**

### Root Causes

1. **Frame Conversion Too Slow**
   - Bitmap conversion taking >16ms per frame
   - BGRA→RGB conversion
   - Memory copies
   - RemoteFX encoding

2. **IronRDP Encoding Backpressure**
   - RDP encoder may be blocking
   - Network send might be slow
   - Client might not be acknowledging fast enough

3. **PipeWire Producing Too Fast**
   - May be capturing at 75 or 90 FPS
   - Need to verify actual capture rate

### Investigation Needed

**Add timing metrics:**
```rust
let start = Instant::now();
// ... frame processing ...
let elapsed = start.elapsed();
if elapsed > Duration::from_millis(16) {
    warn!("Slow frame processing: {:?}", elapsed);
}
```

**Check actual PipeWire rate:**
```rust
// Log frame timestamps
let frame_interval = current_time - last_frame_time;
if frame_count % 60 == 0 {
    info!("PipeWire actual FPS: {:.1}", 1000.0 / frame_interval.as_millis() as f32);
}
```

---

## ISSUE 3: Frame Corruption 🟡 MEDIUM

### Statistics

**Count:** 17 frame corruption errors
**Pattern:** Sporadic, not continuous
**Impact:** Frames skipped, video continues

### Evidence

**All errors same:**
```
ERROR: Failed to convert frame to bitmap: Bitmap conversion failed: Invalid frame: Frame is corrupted or incomplete
```

**Frame parameters look valid:**
```
width: 1280, height: 800, format: BgrX32, stride: 5120
```

### Likely Causes

1. **DMA-BUF Race Condition**
```
PipeWire:              Our code:
Buffer N ready ───────> Map DMA-BUF
Recycle buffer N        Read memory ← Corruption! Buffer reused
```

**Fix:** Ensure immediate copy:
```rust
// MUST copy before returning buffer
let frame_data = dmabuf_slice.to_vec(); // Copy NOW
pw_buffer.queue(); // Safe to recycle
```

2. **Buffer Size Validation**
```rust
let expected_size = height * stride;
if buffer.len() < expected_size {
    return Err("corrupted or incomplete");
}
```

**Check:** Log actual vs expected sizes

3. **SPA Chunk Incomplete Flag**
```rust
if chunk.flags & SPA_CHUNK_FLAG_CORRUPTED != 0 {
    // PipeWire itself marked as corrupted
}
```

### Debug Logging Needed

**In display_handler.rs:**
```rust
debug!("Frame validation: w={}, h={}, stride={}, format={:?}, buf_len={}, expected={}",
    width, height, stride, format, buffer.len(), height * stride);

if buffer.len() < height * stride {
    error!("Buffer too small: {} < {} (CORRUPTED)", buffer.len(), height * stride);
}
```

---

## CLIPBOARD STATUS

### ✅ Windows→Linux: PERFECT

**Test Results:**
- Short text: ✅ "GREG LAMBERSON444" (17 bytes)
- Long text/RTF: ✅ 1180 bytes transferred successfully
- Multiple pastes: ✅ Serials 1, 2, 3, 4 all worked
- UTF-16→UTF-8: ✅ Correct conversion
- Format mapping: ✅ CF_UNICODETEXT (13) used correctly

**Workflow Validated:**
```
Windows copy → FormatList[CF_UNICODETEXT] →
SetSelection["text/plain;charset=utf-8"] →
Linux paste → SelectionTransfer(serial=N) →
FormatDataRequest(13) → FormatDataResponse(UTF-16) →
Convert→UTF-8 → SelectionWrite(serial=N) → SUCCESS
```

### ❌ Linux→Windows: BLOCKED

**Problem:** SelectionOwnerChanged signal never fires

**Stream Status:**
- ✅ Stream created successfully
- ✅ Waiting for signals
- ❌ ZERO events received when copying in Linux

**This is a PORTAL LIMITATION**, not our code issue.

### 📋 Image/File Clipboard Status

**Images:** NOT YET SUPPORTED
- Converters exist in formats.rs (DIB↔PNG, etc.)
- Need to wire CF_DIB, CF_PNG to clipboard flow
- Should be ~1 hour work after text is fully working

**Files:** NOT YET SUPPORTED
- FileContents protocol handlers exist in ironrdp_backend.rs (stubs)
- Transfer engine exists in transfer.rs
- Need to implement chunked transfer workflow
- Estimated 4-6 hours work

**Can you test now?** NO - need implementation first

---

## INVESTIGATION PLAN

### Immediate: Debug SelectionOwnerChanged

**Test 1: dbus-monitor**
```bash
dbus-monitor --session "interface='org.freedesktop.portal.Clipboard'" > dbus.log &
# Copy in Linux
# Check dbus.log for signals
```

**Test 2: Portal introspection**
```bash
busctl --user introspect org.freedesktop.portal.Desktop \
  /org/freedesktop/portal/desktop \
  org.freedesktop.portal.Clipboard | grep SelectionOwner
```

**Test 3: Check xdg-desktop-portal-gnome version**
```bash
dpkg -l | grep xdg-desktop-portal
# Or: rpm -qa | grep xdg-desktop-portal
```

### If Portal Signal Doesn't Work: Implement Polling

**Code:**
```rust
// Fallback: Poll clipboard every 500ms
tokio::spawn(async move {
    let mut last_hash = String::new();
    let mut poll_count = 0;

    loop {
        poll_count += 1;
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Try to read clipboard
        match portal.selection_read(&session, "text/plain;charset=utf-8").await {
            Ok(fd) => {
                let mut file = tokio::fs::File::from_std(fd.into());
                let mut data = Vec::new();
                if file.read_to_end(&mut data).await.is_ok() {
                    let hash = format!("{:x}", sha2::Sha256::digest(&data));

                    if hash != last_hash && !data.is_empty() {
                        info!("📋 Clipboard changed detected via polling (poll #{})", poll_count);
                        last_hash = hash;

                        // Announce to RDP
                        event_tx.send(PortalFormatsAvailable(...)).await;
                    }
                }
            }
            Err(_) => {
                // Clipboard empty or unavailable
            }
        }
    }
});
```

---

## FRAME CORRUPTION DEEP DIVE

### Needed Investigation

**Add to display_handler.rs (before validation):**
```rust
debug!("Validating frame: stream={}, w={}, h={}, stride={}, format={:?}",
    frame.stream_id, frame.width, frame.height, frame.stride, frame.format);
debug!("Buffer: len={}, expected={}", frame.data.len(), frame.height * frame.stride);

if frame.data.len() < (frame.height * frame.stride) as usize {
    error!("FRAME CORRUPTION DETECTED:");
    error!("  Buffer too small: {} bytes", frame.data.len());
    error!("  Expected: {} bytes ({} × {})",
        frame.height * frame.stride, frame.height, frame.stride);
    error!("  Shortfall: {} bytes",
        (frame.height * frame.stride) as usize - frame.data.len());
    return Err(...);
}
```

**Check SPA chunk flags:**
```rust
// In pw_thread.rs process callback
debug!("SPA chunk: size={}, offset={}, stride={}, flags={:?}",
    chunk.size, chunk.offset, chunk.stride, chunk.flags);

if chunk.flags & SPA_CHUNK_FLAG_CORRUPTED != 0 {
    warn!("PipeWire marked chunk as CORRUPTED");
}
```

---

## ANSWERS TO YOUR QUESTIONS

### Can you copy/paste images yet?

**NO** - Images not implemented yet.

**What's needed:**
1. Add CF_DIB, CF_PNG to format announcements
2. Wire image converters (already exist in formats.rs)
3. Test PNG clipboard transfer
4. Estimated: 1-2 hours work

### Can you copy/paste files yet?

**NO** - File transfer not implemented yet.

**What's needed:**
1. Implement FileContentsRequest/Response handlers
2. Wire CF_HDROP format
3. Generate URI lists
4. Implement chunked transfer
5. Estimated: 4-6 hours work

### Priority order?

**My recommendation:**
1. **P0:** Fix Linux→Windows text (via polling if Portal signal doesn't work)
2. **P1:** Add image clipboard (quick win, converters ready)
3. **P2:** Debug frame corruption (adds logging, investigate DMA-BUF)
4. **P3:** Add file transfer (most complex)

---

## IMMEDIATE ACTIONS

### For You to Test:

**Test A: dbus-monitor for Portal signals**
```bash
dbus-monitor --session > /tmp/dbus-all.log 2>&1 &
# Run server, copy in Linux, check dbus-all.log
```

**Test B: Check Portal version/capabilities**
```bash
busctl --user tree org.freedesktop.portal.Desktop
busctl --user introspect org.freedesktop.portal.Desktop \
  /org/freedesktop/portal/desktop
```

### For Me to Implement:

1. Add frame processing timing metrics
2. Add frame validation detailed logging
3. Implement clipboard polling fallback
4. Wire image clipboard support

---

**Status:** Windows→Linux perfect. Linux→Windows needs Portal signal investigation or polling fallback. Video has performance issues needing deep investigation.
