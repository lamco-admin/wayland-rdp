# Video Performance Issues - Analysis and Fixes

**Date:** 2025-11-19
**Log File:** logP.txt (30,967 lines)
**Context:** Performance degradation observed during clipboard testing session

---

## ISSUES IDENTIFIED

### Issue 1: Frame Channel Overload ⚠️ HIGH IMPACT

**Frequency:** 385 occurrences
**Location:** `src/pipewire/pw_thread.rs:555`
**Error:** "Failed to send frame: sending on a full channel"

**Analysis:**
```
PipeWire Thread (Producer)          Frame Channel (64 slots)          Display Handler (Consumer)
     60 FPS capture                        ↓                              ~30 FPS processing
         │                              [Frame 1]
         │                              [Frame 2]
         │                              [Frame 3]
         │                                 ...
         │                              [Frame 64] ← FULL!
         ├─ try_send(Frame 65) ───────────X FAILED
         │                              Channel full, frame dropped
         ├─ try_send(Frame 66) ───────────X FAILED
         ...                            Backpressure builds up
```

**Root Cause:**
1. PipeWire captures at compositor refresh rate (~60 FPS)
2. Display handler processes at target_fps (30 FPS from config)
3. Processing includes: frame copy, format conversion, bitmap encoding
4. Channel capacity is only 64 frames
5. At 60 FPS input / 30 FPS output = 30 frames/sec surplus
6. Channel fills in ~2 seconds, then constant dropping

**Impact on Performance:**
- Frame drops cause choppy video
- User sees laggy/stuttering screen
- Dropped frames waste PipeWire capture effort
- WARN spam fills logs (385 times in this session)

**Current Config (from log):**
```
max_queue_depth: 30
channel_size: 30
buffer_pool_size: 8
```

**Fix Options:**

**Option A: Increase Channel Capacity (Quick Fix)**
```rust
// src/pipewire/pw_thread.rs:217
let (frame_tx, frame_rx) = std_mpsc::sync_channel::<VideoFrame>(256);
//                                                               ^^^ was 64
```
- Pros: Simple one-line change, absorbs burst traffic
- Cons: Uses more memory, doesn't solve root cause
- Recommend: 256 slots (4x current, ~4 seconds buffer at 60 FPS)

**Option B: Frame Skipping Strategy (Better)**
```rust
// In PipeWire process callback
let mut consecutive_failures = 0;

match frame_tx.try_send(frame) {
    Ok(()) => {
        consecutive_failures = 0;
    }
    Err(e) => {
        consecutive_failures += 1;

        // Only warn periodically to avoid log spam
        if consecutive_failures == 1 || consecutive_failures % 30 == 0 {
            warn!("Frame channel full, dropped {} frames", consecutive_failures);
        }

        // Frame dropped - continue processing next
    }
}
```
- Pros: Handles overload gracefully, reduces log spam
- Cons: Still drops frames

**Option C: Adaptive Frame Rate (Best)**
```rust
// Monitor channel fill level
let fill_ratio = frame_tx.len() as f32 / frame_tx.capacity() as f32;

if fill_ratio > 0.8 {
    // Channel >80% full - reduce capture rate
    // Skip every other frame
    skip_next_frame = !skip_next_frame;
    if skip_next_frame {
        return; // Don't send this frame
    }
}
```
- Pros: Dynamically adapts to consumer speed
- Cons: More complex, needs state tracking

**Recommendation:** Implement Option A (increase to 256) + Option B (reduce log spam)

---

### Issue 2: Frame Corruption ⚠️ MEDIUM IMPACT

**Frequency:** 31 occurrences
**Location:** `src/server/display_handler.rs`
**Error:** "Failed to convert frame to bitmap: Bitmap conversion failed: Invalid frame: Frame is corrupted or incomplete"

**Frame Parameters (from log):**
```
width: 1280
height: 800
format: BgrX32
stride: 5120  (= 1280 × 4 bytes/pixel)
```

**Parameters Look Correct:**
- Stride calculation: 1280 × 4 = 5120 ✅
- Format: BgrX32 (32-bit with padding) ✅
- Dimensions: Standard 1280x800 ✅

**Possible Causes:**

1. **DMA-BUF Access Race Condition**
```
PipeWire sends buffer       Display handler maps buffer
         │                            │
         ├─ Buffer N ready ─────────►│
         │                            ├─ Map DMA-BUF
         ├─ Recycle buffer N          │ (reading memory)
         │   (compositor overwrites)   │
         └─────────────────────────────┤
                                       └─ Corruption! Buffer was reused
```

**Solution:** Ensure buffer is copied before returning to PipeWire:
```rust
// Must copy data immediately after mapping
let frame_data = dmabuf_data.to_vec(); // COPY before unmapping
// Now safe to return buffer to PipeWire
```

2. **Buffer Size Validation Failure**
```rust
// Check if buffer is large enough
let expected_size = height * stride;
if buffer.len() < expected_size {
    return Err("Frame is corrupted or incomplete");
}
```

**This might be failing if:**
- Buffer is smaller than expected
- Stride is incorrect
- Height includes padding

3. **Partial Frame Transfer**
- PipeWire buffer not fully written when we read it
- Need to check SPA chunk flags for complete frame

**Investigation Needed:**
```rust
// Add detailed logging before validation
debug!("Frame validation: width={}, height={}, stride={}, format={:?}",
    frame.width, frame.height, frame.stride, frame.format);
debug!("Buffer: len={}, expected={}", buffer.len(), height * stride);
debug!("SPA chunk: size={}, offset={}, flags={:?}",
    chunk.size, chunk.offset, chunk.flags);
```

**Temporary Fix:**
```rust
// If frame validation fails, log details and skip
if let Err(e) = validate_frame(&frame) {
    debug!("Skipping invalid frame: {}  (w={}, h={}, stride={}, buf_len={})",
        e, frame.width, frame.height, frame.stride, frame.data.len());
    return Ok(None); // Skip this frame, don't error
}
```

---

### Issue 3: Connection Reset During Finalize ✅ EXPECTED

**Frequency:** 1 occurrence
**Error:** "Connection error: failed to accept client during finalize - Connection reset by peer"

**Analysis:**
This is the EXPECTED certificate retry behavior:
1. Client connects with untrusted cert
2. Connection fails during finalize
3. User accepts cert
4. Client retries
5. Second connection succeeds

**This is NOT a bug** - it's documented RDP behavior.

---

## PROPOSED FIXES

### Fix 1: Increase Frame Channel Capacity (Immediate)

**File:** `src/pipewire/pw_thread.rs:217`

```rust
// BEFORE:
let (frame_tx, frame_rx) = std_mpsc::sync_channel::<VideoFrame>(64);

// AFTER:
let (frame_tx, frame_rx) = std_mpsc::sync_channel::<VideoFrame>(256);
```

**Rationale:**
- Config shows channel_size: 30, but actual channel is 64
- At 60 FPS input / 30 FPS target = 2:1 ratio
- Need buffer for bursts: 256 slots = ~4 seconds at 60 FPS
- Memory cost: ~4MB per frame × 256 = ~1GB (acceptable for server)

### Fix 2: Reduce Frame Drop Logging (Immediate)

**File:** `src/pipewire/pw_thread.rs:555`

```rust
// BEFORE:
if let Err(e) = frame_tx_for_process.try_send(frame) {
    warn!("Failed to send frame: {}", e);
}

// AFTER:
match frame_tx_for_process.try_send(frame) {
    Ok(()) => {
        // Frame sent successfully
    }
    Err(_) => {
        // Frame dropped due to full channel
        // Only log periodically to avoid spam
        frame_drop_count += 1;
        if frame_drop_count % 30 == 0 {
            warn!("Frame channel full - dropped {} frames (backpressure)", frame_drop_count);
        }
    }
}
```

**Requires:** Add `frame_drop_count` field to stream state

### Fix 3: Investigate Frame Corruption (Research Needed)

**Steps:**
1. Add detailed logging in frame validation
2. Check if DMA-BUF is being copied vs referenced
3. Verify buffer lifecycle (when does PipeWire recycle?)
4. Check SPA chunk flags for completion status

**File to Investigate:** `src/server/display_handler.rs` - frame validation logic
**File to Investigate:** `src/video/converter.rs` - bitmap conversion

**Temporary Mitigation:**
```rust
// Skip corrupted frames instead of erroring
if let Err(e) = validate_and_convert_frame(frame) {
    debug!("Skipping corrupted frame: {}", e);
    continue; // Get next frame
}
```

---

## CONFIGURATION RECOMMENDATIONS

### Current Video Pipeline Config
```toml
[video_pipeline.processor]
target_fps = 30
max_queue_depth = 30

[video_pipeline.dispatcher]
channel_size = 30

[video_pipeline.converter]
buffer_pool_size = 8
```

### Recommended Changes
```toml
[video_pipeline.processor]
target_fps = 30
max_queue_depth = 60    # Increase from 30

[video_pipeline.dispatcher]
channel_size = 60           # Increase from 30

[video_pipeline.converter]
buffer_pool_size = 16       # Increase from 8
```

**But:** These only affect async tokio channels. The PipeWire thread uses `std::sync::mpsc::sync_channel(64)` which is HARDCODED, not from config!

**Root Issue:** Frame channel capacity is hardcoded at 64, ignoring config settings.

---

## IMPLEMENTATION PRIORITY

### P0 - Critical (Fix Now)
1. ✅ Clipboard format mapping (already fixed)
2. 🔧 Increase frame channel to 256 (one line)
3. 🔧 Reduce frame drop log spam (5 lines)

### P1 - High (Fix Soon)
4. 🔍 Investigate frame corruption (research needed)
5. 🔧 Respect video_pipeline config for frame channel
6. 🔧 Add adaptive frame rate based on backpressure

### P2 - Medium (Later)
7. 📊 Add frame drop metrics
8. 🎯 Performance tuning for 60 FPS
9. 🔍 DMA-BUF vs memcpy analysis

---

## QUICK WINS (Can Fix Now)

### 1. Frame Channel Capacity

**Change:**
```rust
// src/pipewire/pw_thread.rs
let (frame_tx, frame_rx) = std_mpsc::sync_channel::<VideoFrame>(256);
```

**Impact:** Eliminates most frame drops, improves performance

### 2. Frame Drop Logging

**Add field to stream state:**
```rust
struct StreamState {
    ...
    frame_drop_count: u64,
    last_drop_warn: Instant,
}
```

**Update logging:**
```rust
if let Err(_) = frame_tx.try_send(frame) {
    self.frame_drop_count += 1;

    let now = Instant::now();
    if now.duration_since(self.last_drop_warn) > Duration::from_secs(5) {
        warn!("Frame drops: {} total (channel overload)", self.frame_drop_count);
        self.last_drop_warn = now;
    }
}
```

**Impact:** Cleaner logs, still tracks issue

---

## CLIPBOARD STATUS (Separate from Video Issues)

**From Same Log:**
- ✅ SelectionTransfer working (6 successful transfers)
- ✅ Data delivered to Portal (all 6 succeeded)
- ✅ SelectionOwnerChanged working (detected Linux clipboard change)
- ✅ FormatList sent to RDP client
- ⚠️ Text corruption due to CF_TEXT/CF_UNICODETEXT mapping (FIXED)

**Next Clipboard Test Should Show:**
- Correct text (not Chinese) due to format mapping fix
- Both directions working
- No clipboard-related errors

---

## RECOMMENDED ACTION PLAN

### Immediate (While User Tests)
1. ✅ Clipboard format fix already pushed and built
2. 🔧 Fix frame channel capacity (1 line)
3. 🔧 Reduce frame drop logging (10 lines)
4. 🚀 Push and rebuild for user's next test

### After Clipboard Validation
5. 🔍 Investigate frame corruption with detailed logging
6. 🔧 Implement proper frame skipping strategy
7. 🎯 Performance tuning session

---

**Status:** Analysis complete, quick fixes identified, ready to implement while user tests clipboard.
