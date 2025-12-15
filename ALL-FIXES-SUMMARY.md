# All Fixes Needed - Complete List
**Date:** 2025-12-11

---

## FIXES TO APPLY

### ✅ Fix 1: Frame Rate Regulation (DONE)
**File:** `src/server/display_handler.rs:121`
**Status:** ✅ Already fixed
**Change:** Move `self.last_frame_time = now` outside if statement

---

### ⏸️ Fix 2: DMA-BUF Remapping (ARCHITECTURE QUESTION)

**Current behavior:**
```rust
// Every frame (5,096 times per session):
mmap(fd, size)     // Syscall
copy 4MB to Vec    // Memory copy
munmap()           // Syscall
```

**Comment in code says:**
> "We immediately copy data and unmap"
> "Safety: We immediately copy and unmap (no lifetime issues)"

**This suggests intentional design for safety.**

**Options:**

**A) Keep current (safe but slow):**
- Pro: No pointer lifetime issues
- Pro: Safe across thread boundaries
- Con: 3 operations × 5,096 frames = 15,000+ syscalls/copies

**B) Cache mappings (fast but complex):**
```rust
// Map once per FD:
mmap_cache: HashMap<RawFd, (*mut u8, usize)>

// Every frame:
if !cache.contains(fd) {
    cache.insert(fd, mmap(fd, size))
}
let ptr = cache[fd];
// Use ptr directly (no copy, no munmap)
```
- Pro: Eliminate 10,000+ syscalls, 5,000+ copies
- Con: Pointer lifetime management
- Con: Must track FD reuse
- Con: Must unmap when stream closes

**QUESTION:** Was the copy+unmap design intentional for safety? Or should we implement caching?

**I need your decision before changing this.**

---

### ✅ Fix 3: Empty Frame Logging

**Current:** No log output when empty frames skipped
**Issue:** Can't verify optimization is working

**Fix:** Add logging (but limit spam)
```rust
// Line 365-368
if bitmap_update.rectangles.is_empty() {
    static EMPTY_COUNT: AtomicU64 = AtomicU64::new(0);
    let count = EMPTY_COUNT.fetch_add(1, Ordering::Relaxed);
    if count % 100 == 0 {
        debug!("Empty frame detection: {} unchanged frames skipped", count);
    }
    continue;
}
```

---

### ✅ Fix 4: Keyboard Mapper Scancode 29

**File:** `src/input/keyboard.rs` or `src/input/mapper.rs`
**Symptom:** `handle_key_down returned KeyUp for code 29`
**Scancode 29:** Left Ctrl
**Issue:** Returning wrong event type

**Need to find and fix the mapper logic for scancode 29.**

---

## WHICH FIXES TO APPLY NOW?

**Definitely apply:**
1. ✅ Frame rate regulation (already done)
3. ✅ Empty frame logging (simple addition)
4. ✅ Keyboard mapper (if we can find it)

**Architecture decision needed:**
2. ⏸️ DMA-BUF caching - Was copy+unmap intentional? Or should we cache?

---

**YOUR DECISION NEEDED:**

Should I:
- A) Apply fixes 1, 3, 4 and SKIP DMA-BUF (leave as-is, carefully architected)
- B) Apply fixes 1, 3, 4 and ADD DMA-BUF caching (despite safety concerns)
- C) Review earlier docs about DMA-BUF design before deciding

**What's your call on the DMA-BUF architecture?**
