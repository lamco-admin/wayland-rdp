# Linux→Windows Clipboard Investigation

**Date:** 2025-12-09
**Status:** In Progress - Root cause identified, fix pending
**Issue:** Linux→Windows clipboard doesn't work (Windows→Linux works fine)

## Summary

Windows→Linux clipboard works perfectly. Linux→Windows fails because `SendInitiateCopy` doesn't generate CB_FORMAT_LIST PDU.

## Evidence from irondbg1.log

### Timeline of Linux Copy Event (00:37:26)

```
00:37:26.100125 - D-Bus detects Linux clipboard change
00:37:26.100240 - SendInitiateCopy dispatched with CF_TEXT + CF_UNICODETEXT
00:37:26.104792 - CB_LOCK_CLIPDATA (0x000A) sent ← WRONG PDU
00:37:26.109831 - CB_FORMAT_LIST_RESPONSE (0x0003) sent ← WRONG PDU
00:37:26.xxx - NO CB_FORMAT_LIST (0x0002) sent ← MISSING!
```

### What SHOULD Happen

```
SendInitiateCopy → cliprdr.initiate_copy() → CB_FORMAT_LIST PDU → Windows receives it
```

### What ACTUALLY Happens

```
SendInitiateCopy → cliprdr.initiate_copy() → CB_LOCK + CB_FORMAT_LIST_RESPONSE → Windows ignores (wrong PDUs)
```

## Root Cause

**Multiple Cliprdr Backend Instances**

From irondbg1.log lines 479-515:
```
00:37:20.804396 - Building clipboard backend for new connection (#1)
00:37:20.851909 - ERROR: Connection reset by peer
00:37:20.872072 - Building clipboard backend for new connection (#2)
00:37:21.007893 - Client accepted
00:37:21.043242 - ONE cliprdr initialized to Ready state
```

Two backends created due to connection retry. When `ServerEvent::Clipboard(SendInitiateCopy)` is processed:
- `get_svc_processor()` might return wrong backend instance
- Wrong backend is in Initialization state
- `initiate_copy()` with state=Initialization generates wrong PDUs

## Why Windows→Linux Works

Windows→Linux uses `SendInitiatePaste`, which:
- Is called AFTER clipboard is definitely in Ready state
- Processes through the correct active backend
- No state machine dependency issues

## Why on_ready() Test Worked

The empty FormatList sent from `on_ready()` at 00:37:21 DID work:
- It sent CB_FORMAT_LIST (0x0002) successfully
- Because it was called IMMEDIATELY after state transition to Ready
- No timing issues or wrong backend

## Attempted Fixes

1. ✅ Fixed D-Bus echo loop with time-based state protection (commit df060b5)
2. ✅ Added SelectionTransfer state checking (commit 1f7e16d)
3. ❌ SendInitiateCopy still doesn't work due to backend instance issues

## Technical Details

### PDU Types (from MS-RDPECLIP)
- 0x0002 = CB_FORMAT_LIST (announce available formats)
- 0x0003 = CB_FORMAT_LIST_RESPONSE (ACK a FormatList)
- 0x000A = CB_LOCK_CLIPDATA (lock clipboard during transfer)

### IronRDP Cliprdr State Machine

```rust
match (self.state, R::is_server()) {
    (CliprdrState::Ready, _) => {
        // Send CB_FORMAT_LIST ← This is what we need
    }
    _ => {
        error!("Attempted to initiate copy in incorrect state");
        // Returns empty Vec ← This is what's happening
    }
}
```

## Potential Fixes

### Option 1: Ensure Single Backend Instance
- Modify IronRDP server to properly cleanup failed connection backends
- Ensure get_svc_processor always returns active backend

### Option 2: Bypass State Machine
- Directly construct CB_FORMAT_LIST PDU bytes
- Send via lower-level API that doesn't use cliprdr.initiate_copy()
- Similar to how SendInitiatePaste works

### Option 3: Delay Retry
- Add delay between SendInitiateCopy attempts
- Retry if CB_FORMAT_LIST wasn't sent
- Monitor outgoing PDUs and retry until 0x0002 appears

### Option 4: Direct Backend Reference
- Store Arc<Cliprdr> in our ClipboardManager
- Call initiate_copy() directly instead of via ServerEvent
- Bypass get_svc_processor lookup

## Next Steps

1. Implement Option 2 (bypass state machine) - most reliable
2. Test on actual hardware
3. If still fails, implement Option 4 (direct reference)

## Logs Referenced

- dectest5.log - First discovery of echo loop
- dectest6-8.log - D-Bus echo loop investigation
- irondbg.log - IronRDP logging enabled
- irondbg1.log - Full trace with PDU decoding ← PRIMARY EVIDENCE
