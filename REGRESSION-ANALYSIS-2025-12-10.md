# Regression Analysis - Full Multiplexer Build
## Date: 2025-12-10 14:15 UTC
## Log: multiplexer-test-20251210-141240.log (107,192 lines)

---

## REPORTED ISSUES

1. **Clipboard 2x paste** - Regression from previous fix
2. **Sluggish performance** - Previous optimizations not working

---

## ISSUE 1: CLIPBOARD 2X PASTE

### Evidence from Logs

```
12:13:41.113674 - SelectionTransfer signal: text/plain;charset=utf-8 (serial 52)
12:13:41.113690 - ✅ First SelectionTransfer for paste operation - will fulfill serial 52
12:13:41.170276 - ✅ Wrote 64 bytes to Portal clipboard (serial 52)

12:13:46.105798 - SelectionTransfer signal: text/plain;charset=utf-8 (serial 53)
12:13:46.105808 - ✅ First SelectionTransfer for paste operation - will fulfill serial 53
12:13:46.117456 - ✅ Wrote 64 bytes to Portal clipboard (serial 53)
```

**Time Gap:** 4.99 seconds between pastes

### Analysis

**Deduplication Code Status:**
- ✅ Present in manager.rs lines 280-305
- ✅ Checks if pending_requests is empty
- ✅ Cancels duplicate requests within same paste operation

**Why Both Marked "First":**
- Serial 52 completes at 12:13:41.170
- pending_requests cleared after completion (line 1105)
- Serial 53 arrives 5 seconds later
- pending_requests is empty → marked as "First"

**Possible Causes:**
1. User pasted twice (4.99 seconds apart)
2. Application made two separate paste requests
3. Different issue than the 45x LibreOffice duplication (that was fixed)

**Previous Fix vs Current:**
- Previous: Fixed 45+ rapid signals (milliseconds apart) for ONE paste
- Current: Two signals 5 seconds apart (looks like TWO pastes)
- Code is working correctly for its design

### Recommendation

Add deduplication window to prevent pastes within N seconds:

```rust
// Track last paste time globally
static LAST_PASTE_TIME: AtomicU64 = AtomicU64::new(0);
const MIN_PASTE_INTERVAL_MS: u64 = 2000; // 2 seconds

// In SelectionTransfer handler:
let now_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
let last_paste_ms = LAST_PASTE_TIME.load(Ordering::Relaxed);

if now_ms - last_paste_ms < MIN_PASTE_INTERVAL_MS {
    info!("Ignoring duplicate paste within {}ms window", MIN_PASTE_INTERVAL_MS);
    // Cancel this request
    continue;
}

LAST_PASTE_TIME.store(now_ms, Ordering::Relaxed);
```

---

## ISSUE 2: PERFORMANCE REGRESSION

### What Was Lost

**Previous Implementation (Working):**
```rust
// In WrdInputHandler::new():
tokio::spawn(async move {
    let mut keyboard_batch = Vec::with_capacity(16);
    let mut mouse_batch = Vec::with_capacity(16);
    let mut last_flush = Instant::now();

    loop {
        tokio::select! {
            Some(event) = input_rx.recv() => {
                // Batch events
            }
            _ = sleep(batch_interval) => {
                // Process batches every 10ms
            }
        }
    }
});
```

**Current Implementation (Broken):**
```rust
// In input_handler.rs:
// Input events now routed through multiplexer (batching happens there)
info!("Input batching task started (10ms flush interval)");  // LIE!

// NO ACTUAL BATCHING TASK EXISTS!
```

### Root Cause

When I moved input to multiplexer, I:
1. Removed the dedicated batching task
2. Assumed multiplexer would handle batching
3. Left fake log message saying task started
4. Multiplexer IS batching, but may have different timing

### Evidence from Logs

**Multiplexer Created:**
```
📊 Full multiplexer created:
🚀 Full multiplexer drain loop started (all priorities active)
```

**Input Processing:**
- NO "Processed input batch" messages in logs
- NO "Multiplexer input: X events processed" messages
- Suggests multiplexer NOT receiving/processing input events

**Why?**
Events are being sent via `try_send()` from input handler, but multiplexer loop might not be polling fast enough or events aren't arriving.

### Diagnosis Needed

Check multiplexer_loop.rs timing:
- 100μs sleep might be too long
- 10ms batch interval might not align with event arrival
- `try_recv()` might be missing events

---

## COMPREHENSIVE FIX PLAN

### Fix 1: Restore Input Batching Task (CRITICAL)

The multiplexer approach was too complex. Restore the proven working batching task:

**In input_handler.rs constructor:**
```rust
// Create bounded input queue for multiplexer awareness
let (input_tx, mut input_rx) = mpsc::channel(32);

// Start ACTUAL batching task (this was working before!)
let portal_clone = Arc::clone(&portal);
let keyboard_clone = Arc::clone(&keyboard_handler);
let mouse_clone = Arc::clone(&mouse_handler);
let coord_clone = Arc::clone(&coordinate_transformer);
let session_clone = Arc::clone(&session);

tokio::spawn(async move {
    let mut keyboard_batch = Vec::with_capacity(16);
    let mut mouse_batch = Vec::with_capacity(16);
    let mut last_flush = Instant::now();
    let batch_interval = tokio::time::Duration::from_millis(10);

    loop {
        tokio::select! {
            Some(event) = input_rx.recv() => {
                match event {
                    InputEvent::Keyboard(kbd) => keyboard_batch.push(kbd),
                    InputEvent::Mouse(mouse) => mouse_batch.push(mouse),
                }
            }

            _ = tokio::time::sleep_until(tokio::time::Instant::from_std(last_flush + batch_interval)) => {
                // Process batches (use existing static methods)
                for kbd_event in keyboard_batch.drain(..) {
                    // Process keyboard
                }
                for mouse_event in mouse_batch.drain(..) {
                    // Process mouse
                }
                last_flush = Instant::now();
            }
        }
    }
});
```

### Fix 2: Clipboard Time-Based Deduplication

Add global paste time tracking:

```rust
use std::sync::atomic::{AtomicU64, Ordering};

static LAST_PASTE_TIME_MS: AtomicU64 = AtomicU64::new(0);

// In SelectionTransfer handler, BEFORE the pending check:
let now_ms = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis() as u64;
let last_paste = LAST_PASTE_TIME_MS.load(Ordering::Relaxed);

if now_ms > 0 && now_ms - last_paste < 3000 {  // 3 second window
    info!("🔄 Ignoring duplicate paste within 3-second window (serial {})", transfer_event.serial);
    // Cancel this request
    if let (Some(portal), Some(session)) = (...) {
        portal.selection_write_done(&session, transfer_event.serial, false).await;
    }
    continue;
}

LAST_PASTE_TIME_MS.store(now_ms, Ordering::Relaxed);
```

### Fix 3: Simplify Multiplexer (PRAGMATIC)

Current approach is too complex. Simpler design:

**Keep:**
- Graphics queue (working)
- Graphics drain task (working)

**Restore:**
- Input batching task in input_handler.rs (proven working)
- Direct clipboard handling (working before)

**Result:**
- Graphics isolated (multiplexed)
- Input batched (optimized)
- Clipboard direct (simple, working)
- All previous optimizations restored

---

## ROOT CAUSE SUMMARY

### Performance Regression
**Cause:** Removed working input batching task when implementing multiplexer
**Symptom:** Events go to multiplexer queue but aren't processed efficiently
**Fix:** Restore dedicated input batching task

### Clipboard Regression
**Cause:** Two pastes 5 seconds apart, both treated as "First" (pending cleared between them)
**Symptom:** 2 copies instead of 1
**Fix:** Add time-based deduplication (3-second window)

---

## IMPLEMENTATION PRIORITY

1. **Restore input batching task** (30 min) - Fixes performance
2. **Add time-based clipboard deduplication** (15 min) - Fixes 2x paste
3. **Test thoroughly** (30 min) - Verify both fixes
4. **Simplify architecture** (optional) - Keep what works

---

## LESSONS LEARNED

1. **Don't remove working code** during refactoring
2. **Test immediately** after changes
3. **Complexity has costs** - simpler is often better
4. **Preserve what works** - graphics queue good, input task was good too

---

## END OF ANALYSIS
Ready to implement fixes.
