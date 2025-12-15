# 16-Copy Bug - Root Cause Found
**Issue:** 1 Ctrl+V = 16 copies in Writer
**Date:** 2025-12-11

---

## THE BUG

**My Error:** I removed the logic that cancels unfulfilled SelectionTransfer requests!

**What happens:**
1. User presses Ctrl+V in LibreOffice
2. LibreOffice generates **16+ SelectionTransfer signals** for ONE paste (different MIME types: text/plain, UTF8_STRING, text/html, etc.)
3. Our code queues all 16 requests in FIFO
4. We get ONE FormatDataResponse from Windows
5. We fulfill serial 15 and write to Portal
6. **My broken code:** Left other 15 requests in queue
7. LibreOffice generates 15 more responses somehow OR we have stale queue
8. Result: 16 pastes

**Old working code:**
```rust
// After successful write:
pending.clear();  // Clear ALL pending
for unfulfilled_serial in others {
    portal.selection_write_done(serial, false).await;  // Cancel each
}
```

**My broken code:**
```rust
// Note: Other pending requests remain in queue
// They will be fulfilled by subsequent FormatDataResponse events
```

**I thought:** We want to support multiple Ctrl+V (queue them)
**Reality:** LibreOffice sends 16-45 requests for ONE Ctrl+V, we must cancel 15-44 of them!

---

## THE FIX

**Restored cancellation logic:**
```rust
// After successful write to Portal:
let unfulfilled: Vec<u32> = pending.iter()
    .filter(|(s, _, _)| *s != serial)
    .collect();

pending.clear();  // Clear ALL

// Cancel each unfulfilled request
for unfulfilled_serial in unfulfilled {
    portal.selection_write_done(unfulfilled_serial, false).await;
}
```

**This ensures:**
- First SelectionTransfer of 16: Fulfilled ✅
- Other 15 SelectionTransfer: Canceled ✅
- Result: Single paste (correct)

---

## WHY THIS WAS CONFUSING

**Our logs only showed 2 SelectionTransfer:**
- At INFO level, we only log when we START processing
- LibreOffice's other 14-44 requests arrive but aren't logged (they go to queue)
- FIFO queue had 16 requests, but logs only showed first of each batch

**The 16 requests were queued but invisible in INFO logs!**

---

## EXPECTED BEHAVIOR AFTER FIX

**User presses Ctrl+V once:**
1. LibreOffice sends 16 SelectionTransfer signals (different MIME types)
2. We add all 16 to FIFO queue
3. We send ONE FormatDataRequest to Windows (for first serial)
4. We get ONE FormatDataResponse
5. We pop first request from queue, write to Portal
6. **We cancel the other 15 requests** ✅
7. Result: Single paste

**Test:** Copy in Windows, Ctrl+V in Writer → Should get 1 copy only
