# Performance Optimization Audit
## Date: 2025-12-10 18:45 UTC
## Comparing Current vs Previous Working Build

---

## PREVIOUS WORKING BUILD OPTIMIZATIONS

From SESSION-HANDOVER-2025-12-10-FINAL.md, these were the key performance fixes:

### 1. PipeWire Polling Optimization ✅ PRESENT
**Fix:** Non-blocking `iterate(0ms)` + 5ms sleep
**Location:** `src/pipewire/pw_thread.rs:443, 447`
**Status:** ✅ Verified present in current code
```rust
loop_ref.iterate(Duration::from_millis(0));  // Non-blocking
std::thread::sleep(Duration::from_millis(5));  // Brief sleep
```

### 2. Input Event Batching ✅ RESTORED
**Fix:** 10ms batching window, single task processes batches
**Location:** `src/server/input_handler.rs:170-213`
**Status:** ✅ Restored after regression
**Log Evidence:** "Input batching task started (REAL task, 10ms flush interval)"

### 3. Frame Rate Regulation ✅ PRESENT
**Fix:** Token bucket algorithm targeting 30 FPS
**Location:** `src/server/display_handler.rs:85-133, 303-343`
**Status:** ✅ Verified present
**Log Evidence:** "Frame rate regulation: dropped X frames, sent Y"

### 4. Clipboard Hash Cleanup ✅ PRESENT
**Fix:** Moved to 1-second background task (not per-event)
**Location:** `src/clipboard/manager.rs:639-670`
**Status:** ✅ Verified present
```rust
tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        // Cleanup old hashes
    }
});
```

### 5. Graphics Queue Isolation ✅ PRESENT
**Fix:** Bounded queue (4 frames) with drop/coalesce
**Location:** `src/server/graphics_drain.rs`
**Status:** ✅ Working

---

## CURRENT IMPLEMENTATION CHECK

### All Major Optimizations Present:
- ✅ PipeWire polling: Non-blocking iterate
- ✅ Input batching: 10ms windows
- ✅ Frame rate regulation: 30 FPS target
- ✅ Graphics isolation: Bounded queue with coalescing
- ✅ Hash cleanup: Background task

---

## POTENTIAL SLUGGISHNESS CAUSES

### 1. RemoteFX Slow Encoding
**Evidence:** 42+ frames took 11-69ms to encode (from previous test)
**Impact:** Some frames exceed 33ms budget (30 FPS = 33ms/frame)
**Frames affected:** 0.68% of total
**Not fixed by:** Any optimization (codec limitation)
**Solution:** H.264 migration

### 2. Graphics Drain Task Overhead
**New in current build:** Graphics drain task adds conversion overhead
**Check:** Are we doing double conversion (frame → GraphicsFrame → BitmapUpdate)?

### 3. Input Batching Queue Size
**Current:** Bounded 32
**Previous:** Unbounded
**Impact:** If queue fills, events drop
**Check:** Are we seeing queue full errors?

### 4. Multiplexer Loop Overhead
**New:** multiplexer_loop running (control + clipboard)
**Overhead:** 100μs sleep per cycle
**Check:** Is this adding latency?

---

## IMMEDIATE CHECKS NEEDED

### Check 1: Graphics Conversion Path

Current path:
```
PipeWire frame
  → Display handler convert_to_bitmap()
  → Display handler convert_to_iron_format()
  → Create GraphicsFrame
  → Graphics drain task
  → convert_to_iron_format() AGAIN?
```

**Possible double conversion!** Let me verify.

### Check 2: Input Queue Drops

Search logs for:
- "Failed to queue keyboard event for batching"
- "Failed to queue mouse event for batching"

If present: Queue too small or filling up

### Check 3: Encoding Performance

Compare encoding times:
- Previous build: 2 slow frames in 130 seconds
- Current build: 42 slow frames in similar duration

**5-10x worse!** Why?

---

## ACTION PLAN

1. Check graphics conversion path for double work
2. Review encoding time distribution
3. Verify no blocking calls in hot paths
4. Check for new mutex contention
5. Profile actual bottleneck
