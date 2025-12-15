# Clipboard Deadlock Bug - Critical Fix

**Date:** 2025-11-19
**Severity:** CRITICAL - Server crash on connection
**Status:** FIXED
**Commits:** 92cc475, b6bd36e, ccc0d26

---

## Problem Summary

After implementing clipboard functionality (commits c6faa70 and 9dcef91), the server crashed immediately when RDP clients attempted to connect. The crash occurred after certificate acceptance retry, preventing any connections from succeeding.

---

## Symptoms

**Before clipboard implementation (commit 74696bf):**
- ✅ Server handles connection retries correctly
- ✅ First attempt fails (cert untrusted)
- ✅ Second attempt succeeds
- ✅ Server continues running
- ✅ Mouse and keyboard work

**After clipboard implementation (commits c6faa70, 9dcef91):**
- ❌ First connection attempt fails (expected - cert)
- ❌ Second connection attempt initiated
- ❌ **Server crashes with "channel closed" error**
- ❌ No mouse/keyboard functionality
- ❌ Server exits immediately

**Log Evidence:**
```
ERROR ironrdp_server::server: Connection error error=failed to accept client during finalize
DEBUG wrd_server::clipboard::ironrdp_backend: Building clipboard backend for new connection
ERROR wrd_server::server::display_handler: Failed to send display update: channel closed
DEBUG wrd_server::server: WrdServer dropped - cleaning up resources
```

---

## Root Cause Analysis

### The Deadlock

**IronRDP Architecture:**
- `CliprdrBackend` trait methods are **synchronous** (not async)
- Called from IronRDP's internal event loop (async runtime)
- Methods must be non-blocking and fast

**Our Implementation:**
- Clipboard callbacks used `blocking_lock()` on Arc<Mutex<>>
- Called `blocking_send()` on tokio channels
- These are **blocking operations in an async context**

**What Happened:**
1. Client connects → IronRDP calls `build_cliprdr_backend()`
2. Connection proceeds → IronRDP calls `on_remote_copy()`
3. `on_remote_copy()` calls `manager.blocking_lock()`
4. **`blocking_lock()` blocks the tokio runtime thread**
5. IronRDP's event loop hangs waiting for the lock
6. Connection times out → client resets connection
7. IronRDP's `run()` returns with error
8. Display update channel closes → server exits

**Code Location:**
```rust
// src/clipboard/ironrdp_backend.rs:206-210 (BEFORE FIX)
fn on_remote_copy(&mut self, available_formats: &[ClipboardFormat]) {
    // ...
    let event_tx = {
        let manager = self.clipboard_manager.blocking_lock(); // ← DEADLOCK!
        manager.event_sender()
    };

    if let Err(e) = event_tx.blocking_send(ClipboardEvent::RdpFormatList(wrd_formats)) {
        // ← BLOCKS IRONRDP EVENT LOOP!
        error!("Failed to send RDP format list to manager: {:?}", e);
    }
}
```

### Why This Caused Server Exit

The blocking operation caused IronRDP's connection finalization to timeout:
1. Client sends clipboard format announcement
2. Server's `on_remote_copy()` blocks on mutex
3. IronRDP waits for callback to return
4. Timeout exceeded → "failed to accept client during finalize"
5. IronRDP's `server.run()` returns error
6. Server cleanup initiated → channels closed → exit

### Why First Connection Worked Before

**Previous behavior (no clipboard):**
- First connection fails on cert (expected)
- IronRDP's `run()` continues listening
- Second connection succeeds

**With blocking clipboard:**
- First connection fails on cert
- Second connection starts
- Clipboard callbacks block IronRDP
- **IronRDP's `run()` exits instead of continuing**

---

## Investigation Timeline

**09:09-09:16:** Initial debugging
- Verified old version (74696bf) works
- Confirmed new version crashes
- Tested with clipboard disabled in config - still crashed
- Ruled out clipboard config setting as cause

**09:16-09:20:** Identified channel lifecycle bug
- Found `build_cliprdr_backend()` creating channels on each call
- Suspected channel dropping on retry
- Fixed by removing channel creation from build method
- **Did not resolve the crash**

**09:20-09:23:** Found the real bug
- Analyzed working vs failing log differences
- Working: 2 connection attempts, stays running
- Failing: 2 attempts start, crashes on second
- Found `blocking_lock()` and `blocking_send()` in callbacks
- **Removed blocking operations → FIXED**

---

## The Fix

### Temporary Solution (Current)

**Removed all blocking operations from clipboard callbacks:**

```rust
// src/clipboard/ironrdp_backend.rs (AFTER FIX)

fn on_remote_copy(&mut self, available_formats: &[ClipboardFormat]) {
    info!("Remote copy announced with {} formats", available_formats.len());

    // Log available formats
    for (idx, format) in available_formats.iter().enumerate() {
        let name = format.name.as_ref().map(|n| n.value()).unwrap_or("");
        debug!("  Format {}: ID={:?}, Name={}", idx, format.id, name);
    }

    // Note: This callback is called from IronRDP's sync context.
    // Blocking operations here can deadlock the IronRDP runtime.
    // For now, just log the formats. Full clipboard integration will
    // be implemented after resolving the async/sync boundary issues.
}

fn on_format_data_request(&mut self, request: FormatDataRequest) {
    debug!("Format data requested for format ID: {}", request.format.0);
    // Logging only - no blocking operations
}

fn on_format_data_response(&mut self, response: FormatDataResponse<'_>) {
    debug!("Format data response received: {} bytes", response.data().len());
    // Logging only - no blocking operations
}
```

**Result:**
- ✅ Server connects successfully
- ✅ Handles connection retries properly
- ✅ Mouse and keyboard work perfectly
- ⚠️ Clipboard functionality disabled (no data transfer)

**Test Results (logold3.txt):**
- 24,564 log lines - extended session
- Mouse and keyboard events confirmed working
- No crashes or channel errors
- Server stable

---

## Proper Solution Design

### Challenge

**IronRDP constraints:**
- Trait methods are synchronous (fn, not async fn)
- Called from tokio async runtime
- Must not block the runtime

**Our needs:**
- Access clipboard manager (async)
- Send events to manager (async channel)
- Process clipboard data (async operations)

### Proposed Architecture

**Option 1: Thread-Safe Queue**
```rust
pub struct WrdCliprdrBackend {
    clipboard_manager: Arc<Mutex<ClipboardManager>>,
    // Add non-blocking event queue
    event_queue: Arc<RwLock<VecDeque<ClipboardEvent>>>,
}

fn on_remote_copy(&mut self, available_formats: &[ClipboardFormat]) {
    // Non-blocking: just push to queue
    let event = ClipboardEvent::RdpFormatList(convert_formats(available_formats));
    if let Ok(mut queue) = self.event_queue.try_write() {
        queue.push_back(event);
    }
}

// Separate async task drains queue
async fn process_clipboard_events(queue: Arc<RwLock<VecDeque<ClipboardEvent>>>) {
    loop {
        if let Ok(mut q) = queue.try_write() {
            while let Some(event) = q.pop_front() {
                // Process with async operations
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
```

**Option 2: mpsc::UnboundedChannel (Non-Blocking)**
```rust
pub struct WrdCliprdrBackend {
    clipboard_manager: Arc<Mutex<ClipboardManager>>,
    // Use unbounded channel for non-blocking sends
    event_tx: mpsc::UnboundedSender<ClipboardEvent>,
}

fn on_remote_copy(&mut self, available_formats: &[ClipboardFormat]) {
    let event = ClipboardEvent::RdpFormatList(convert_formats(available_formats));
    // unbounded send never blocks
    let _ = self.event_tx.send(event);
}

// Receiver in async task
async fn process_clipboard_events(mut rx: mpsc::UnboundedReceiver<ClipboardEvent>) {
    while let Some(event) = rx.recv().await {
        // Process with async operations
    }
}
```

**Option 3: Atomic State + Polling**
```rust
pub struct WrdCliprdrBackend {
    // Store formats in atomic structure
    current_formats: Arc<RwLock<Vec<ClipboardFormat>>>,
}

fn on_remote_copy(&mut self, available_formats: &[ClipboardFormat]) {
    // Non-blocking write
    if let Ok(mut formats) = self.current_formats.try_write() {
        *formats = available_formats.to_vec();
    }
}

// Separate task polls for changes
async fn clipboard_sync_task(formats: Arc<RwLock<Vec<ClipboardFormat>>>) {
    let mut last_hash = 0u64;
    loop {
        if let Ok(formats) = formats.try_read() {
            let hash = calculate_hash(&formats);
            if hash != last_hash {
                // Formats changed - process them
                process_formats(&formats).await;
                last_hash = hash;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
```

**Recommendation: Option 2 (UnboundedChannel)**
- Simplest implementation
- Non-blocking by design
- Handles backpressure naturally
- Fits existing architecture

---

## Implementation Plan

### Phase 1: Restore Basic Clipboard (1-2 days)

1. **Create non-blocking event queue** in clipboard backend
2. **Spawn async task** to process clipboard events
3. **Implement format announcement** (RDP → Portal)
4. **Test text clipboard** (copy/paste both directions)

### Phase 2: Full Clipboard (2-3 days)

5. **Implement format data exchange**
6. **Add image clipboard** support
7. **Add file transfer** support
8. **Test all clipboard operations**

### Phase 3: Testing & Validation (1-2 days)

9. **Comprehensive clipboard testing**
10. **Performance validation**
11. **Stability testing**

**Total Estimate:** 4-7 days for complete clipboard restoration

---

## Lessons Learned

### Don't Block Async Runtimes

**Problem:**
- Calling `blocking_lock()` from async context
- Calling `blocking_send()` from async context
- Blocking trait method implementations

**Solution:**
- Use non-blocking alternatives (try_lock, try_send)
- Use unbounded channels (never block)
- Spawn separate tasks for async work
- Keep trait methods minimal and fast

### IronRDP Integration Patterns

**IronRDP traits are sync, our code is async:**
- Trait methods run in IronRDP's runtime
- Must not block or hold locks
- Use message passing for async communication
- Spawn tasks for complex operations

**Pattern:**
```rust
impl IronRdpTrait for Handler {
    fn sync_method(&mut self, data: Data) {
        // NEVER: blocking_lock(), blocking_send(), sleep(), etc.
        // ALWAYS: try_lock(), unbounded send, spawn tasks

        let tx = self.event_channel.clone();
        let _ = tx.send(Event::from(data)); // Non-blocking
    }
}
```

---

## Current Status

**Working:**
- ✅ RDP connection (TLS 1.3)
- ✅ Video streaming (RemoteFX)
- ✅ Mouse input (motion + clicks)
- ✅ Keyboard input (all keys)
- ✅ Connection retry handling
- ✅ Server stability

**Not Working:**
- ❌ Clipboard text copy/paste
- ❌ Clipboard image transfer
- ❌ Clipboard file transfer

**Code Quality:**
- ✅ Zero TODOs in clipboard code
- ✅ No blocking operations in callbacks
- ✅ Server stable and functional
- ⚠️ Clipboard integration incomplete (temporarily disabled)

---

## Testing Evidence

**Log Files:**
- `logold.txt` - Working with old version (74696bf), 30K+ lines
- `logclicks1.txt` - Working session from Nov 18, validated
- `logclips.txt` - Failed with blocking operations
- `lognoclips.txt` - Failed with clipboard disabled in config
- `logold1.txt` - Failed with channel lifecycle fix only
- `logold2.txt` - Failed with optional wl-clipboard
- **`logold3.txt`** - **SUCCESS with blocking operations removed, 24K+ lines**

**Validation:**
- Mouse: "Mouse move: RDP(648, 246) -> Stream(648.00, 246.00)"
- Keyboard: "Key up: scancode=0x000F, keycode=15"
- Portal injection: "Injecting keyboard: keycode=15, pressed=false"
- Extended session: 24,564 lines of stable operation

---

## Next Actions

**Immediate (Today):**
1. ✅ Server working again
2. ✅ Basic functionality validated
3. ✅ Bug documented

**Short-term (This Week):**
1. Redesign clipboard using non-blocking event queue
2. Implement text clipboard first
3. Test thoroughly
4. Add images and files incrementally

**Medium-term:**
1. Complete clipboard testing
2. Performance baseline measurements
3. Multi-monitor testing
4. Stability testing (24-hour runs)

---

**Status:** Server restored to working state, clipboard redesign needed.
