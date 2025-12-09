# Session Handover: Clipboard Fix Progress - Next Steps

**Date:** 2025-12-09
**Branch:** `feature/gnome-clipboard-extension`
**Commit:** 74b9eab (wrd-server) + 2d0ed673 (IronRDP fork)
**Status:** IronRDP Fix Applied & Running, But PDU Encoding Issue Discovered

---

## What We Accomplished Today

### 1. ✅ Root Cause Identified via KDE Testing
- Set up KDE test VM (Debian + KDE Plasma on Wayland)
- Proved Portal SelectionOwnerChanged detects clipboard changes on KDE
- Discovered Portal path was blocked by aggressive echo protection
- Confirmed SAME wrong PDUs on both GNOME (D-Bus) and KDE (Portal)
- **Conclusion: Issue is NOT D-Bus-specific, it's IronRDP**

### 2. ✅ Fixed Portal Echo Detection
- Changed Portal path from `force=false` to `force=true`
- Portal's `session_is_owner` flag already filters echoes
- Now uses same 2-second timing protection as D-Bus
- Portal clipboard changes reach `handle_portal_formats()` correctly

### 3. ✅ Deep Protocol & Architecture Research
- **MS-RDPECLIP Spec:** Servers CAN send CB_FORMAT_LIST anytime after init
- **IronRDP Analysis:** Cliprdr crate is heavily client-biased
- **Found Bug:** `initiate_copy()` blocks servers in Initialization state
- **Proper Fix:** Servers bypass client state machine per MS-RDPECLIP

### 4. ✅ Clean Fork Architecture Established
- Created branch in glamberson/IronRDP fork
- Branch: `update-sspi-with-clipboard-fix`
- Base: allan2's update-sspi (for sspi/picky compatibility)
- Plus: Server clipboard initiate_copy() fix (commit 2d0ed673)
- wrd-server Cargo.toml now points to clean fork dependency

### 5. ⚠️ Fix Runs But PDU Encoding Still Wrong
- Our fix IS running (see log: "SERVER initiate_copy: sending FormatList")
- Fix claims to push `ClipboardPdu::FormatList`
- But PDUs sent are still CB_LOCK_CLIPDATA (0x000A) + CB_FORMAT_LIST_RESPONSE (0x0003)
- **NOT the expected CB_FORMAT_LIST (0x0002)**

---

## The Remaining Mystery

### What We Expect:

```rust
// Our fix in initiate_copy():
if R::is_server() {
    info!("SERVER initiate_copy: sending FormatList");
    pdus.push(ClipboardPdu::FormatList(...));  // Should generate 0x0002
}
return Ok(pdus.into_iter().map(into_cliprdr_message).collect());
```

### What We See in Logs:

```
✅ SERVER initiate_copy: sending FormatList (state=Ready, 3 formats)  ← Fix is running!
❌ McsMessage { ... user_data: [..., 10, 0, ...] }  ← 0x000A = CB_LOCK_CLIPDATA
❌ McsMessage { ... user_data: [..., 3, 0, ...] }   ← 0x0003 = CB_FORMAT_LIST_RESPONSE
```

### Theories:

**Theory 1: Encoding Bug**
- `ClipboardPdu::FormatList` is being encoded to wrong message type
- `into_cliprdr_message()` or `SvcMessage::from(pdu)` has bug
- Check PDU encoding in ironrdp-cliprdr/src/pdu/ modules

**Theory 2: Wrong Backend Instance**
- Multiple backends from connection retries (original theory from handover)
- `get_svc_processor()` returns wrong instance
- Wrong instance has different state/behavior
- Check: Are there TRACE logs showing which backend processes the event?

**Theory 3: PDUs Being Overridden**
- FormatList IS generated but then replaced
- Something else generates Lock+FormatListResponse after
- Check: Is there code that sends Lock when FormatList is sent?
- MS-RDPECLIP: Lock is optional for file transfers, why sent for text?

**Theory 4: Return Value Lost**
- initiate_copy() returns FormatList but it's ignored
- Server event dispatch has bug
- Check server.rs line 552: Does it properly encode/send the Vec?

---

## Evidence from Test Logs

### Test Log: clean-fork-test.log (KDE, 2025-12-09 19:00-19:04)

**Clipboard Changes Detected:**
```
19:00:40 - SERVER initiate_copy (0 formats) → 0x000B (Unlock) sent
19:01:54 - SERVER initiate_copy (3 formats) → 0x000A (Lock) + 0x0003 (FormatListResponse) sent
19:04:09 - Remote copy announced (client→server) → 0x0002 (FormatList) sent ✅
```

**Key Observation:** When CLIENT copies (19:04:09), CB_FORMAT_LIST (0x0002) IS generated correctly. When SERVER tries to announce (19:01:54), wrong PDUs are sent.

### Successful vs Failed PDU Comparison

**Successful (Windows→Linux at 19:04:09):**
```
[32m INFO[0m Remote copy announced with 4 formats
[34mDEBUG[0m McsMessage::SendDataRequest { channel_id: 1006,
  user_data: [32, 0, 0, 0, 19, 0, 0, 0, 2, 0, ...] }
                                      ^^ 0x02 = CB_FORMAT_LIST ✅
```

**Failed (Linux→Windows at 19:01:54):**
```
[32m INFO[0m SERVER initiate_copy: sending FormatList (state=Ready, 3 formats)
[34mDEBUG[0m McsMessage::SendDataRequest { channel_id: 1006,
  user_data: [12, 0, 0, 0, 19, 0, 0, 0, 10, 0, ...] }
                                      ^^ 0x0A = CB_LOCK_CLIPDATA ❌
[34mDEBUG[0m McsMessage::SendDataRequest { channel_id: 1006,
  user_data: [8, 0, 0, 0, 19, 0, 0, 0, 3, 0, ...] }
                                     ^^ 0x03 = CB_FORMAT_LIST_RESPONSE ❌
```

**Different code paths?** Client→server uses process() to handle incoming FormatList. Server→client uses initiate_copy() via ServerEvent. These may encode differently!

---

## Repository Status

### wrd-server-specs (lamco-admin/wayland-rdp)

**Branch:** `feature/gnome-clipboard-extension`
**Latest Commit:** 74b9eab

**Key Changes:**
- `src/clipboard/manager.rs`: Portal echo detection fix (force=true)
- `Cargo.toml`: Points to glamberson/IronRDP fork
- `KDE-TESTING-GUIDE.md`: Comprehensive testing documentation
- `SESSION-SUMMARY-CLIPBOARD-FIX-2025-12-09.md`: Research findings

### IronRDP Fork (glamberson/IronRDP)

**Repository:** https://github.com/glamberson/IronRDP
**Branch:** `update-sspi-with-clipboard-fix`
**Commit:** 2d0ed673

**Changes:**
- `crates/ironrdp-cliprdr/src/lib.rs`: Server initiate_copy() fix
- Servers bypass client state machine
- Documented with MS-RDPECLIP spec references
- Ready for upstream PR (after verification)

### Local IronRDP Modifications

**Dev Machine:**
- `~/.cargo/git/.../c580de5/crates/ironrdp-cliprdr/src/lib.rs` - Has patch + backup
- `~/.cargo/git/.../d69e6f9/crates/ironrdp-cliprdr/src/lib.rs` - Has patch (old local test)

**Status:** These local patches are now superseded by clean fork. Can be cleaned up.

---

## Next Debugging Steps

### Step 1: Add Detailed PDU Logging

Add logging in IronRDP to see what PDUs are actually being generated:

**File:** `glamberson/IronRDP` branch `update-sspi-with-clipboard-fix`
**Location:** `crates/ironrdp-cliprdr/src/lib.rs` line 246

```rust
if R::is_server() {
    info!("SERVER initiate_copy: sending FormatList (state={:?}, {} formats)",
          self.state, available_formats.len());

    let format_list = self.build_format_list(available_formats).map_err(|e| encode_err!(e))?;

    // NEW: Log what we're actually adding
    info!("📦 Pushing ClipboardPdu::FormatList to pdus vec");
    pdus.push(ClipboardPdu::FormatList(format_list));

    // NEW: Log what we're returning
    info!("📤 Returning {} PDUs from initiate_copy()", pdus.len());
}
```

Then in `server.rs` at the ServerEvent::Clipboard handler (line 552):

```rust
ServerEvent::Clipboard(c) => {
    let msgs = match c {
        ClipboardMessage::SendInitiateCopy(formats) => {
            let result = cliprdr.initiate_copy(&formats)?;
            // NEW: Log what was returned
            info!("📬 initiate_copy() returned {} messages", result.len());
            result
        },
        //...
    };
    // Encode and send...
}
```

This will show us if FormatList is being generated but lost somewhere.

### Step 2: Check PDU Encoding

Look at how `ClipboardPdu::FormatList` is encoded to bytes:

**File:** `crates/ironrdp-cliprdr/src/pdu/mod.rs`

Search for `impl PduEncode for FormatList` or similar. Verify it sets msgType to 0x0002.

### Step 3: Compare Client vs Server Encoding

When client sends FormatList (which works), trace the code path:
- Client copies → FormatList received → handle_format_list() → FormatListResponse sent

When server tries to send FormatList (which fails):
- Linux copies → SendInitiateCopy → initiate_copy() → ??? → Wrong PDUs

Compare these paths to find where they diverge.

### Step 4: Check for Lock/Unlock Auto-Generation

Search IronRDP for any code that automatically generates Lock/Unlock PDUs:
- File transfer code might add Lock automatically
- Capabilities negotiation (CAN_LOCK_CLIPDATA flag) might trigger behavior
- Check if FileContentsRequest/Response handlers add Lock

---

## Clean Architecture Now in Place

### Dependency Chain:

```
wrd-server (lamco-admin/wayland-rdp)
    ↓ Cargo.toml
glamberson/IronRDP (branch: update-sspi-with-clipboard-fix)
    ├─ Base: allan2/update-sspi (sspi/picky fixes)
    └─ Plus: Server clipboard fix (commit 2d0ed673)
```

### Benefits:

- ✅ No more local ~/.cargo patches
- ✅ Version controlled fix
- ✅ Reproducible across machines
- ✅ Ready for upstream contribution
- ✅ Clean separation of concerns

### Files to Clean Up (Optional):

- `~/.cargo/git/.../c580de5/` old checkout with patch
- `~/.cargo/git/.../d69e6f9/` old checkout (now superseded)
- `DONOTUSE/clipboard-debugging/` move to `docs/research/`

---

## Test Boxes Status

### KDE (192.168.10.3 - debway)
- **OS:** Debian 14 (forky) + KDE Plasma 6.5.3
- **Session:** Wayland (kwin_wayland)
- **Status:** Rebooted after freeze, healthy
- **Portal:** xdg-desktop-portal-kde active
- **Binary:** wrd-server-clean-fork deployed
- **Ready:** Can test from desktop (not SSH - needs WAYLAND_DISPLAY)

### GNOME (192.168.10.205)
- **OS:** Ubuntu/Debian + GNOME 47
- **Session:** Wayland
- **Status:** Available
- **Extension:** wayland-rdp-clipboard D-Bus extension
- **Binary:** Needs fresh deployment
- **Ready:** Run from desktop session

---

## Immediate Test Procedure (When Resuming)

**On KDE VM console (192.168.10.3):**

```bash
cd ~/wayland/wrd-server-specs
./target/release/wrd-server-clean-fork -c config.toml 2>&1 | tee final-test.log

# Wait for Portal permission dialog, approve it
# Connect from Windows RDP
# Copy text on KDE
# Check logs:

# Should see our fix running:
grep "SERVER initiate_copy" final-test.log

# Check PDUs (the mystery):
grep "McsMessage.*1006" final-test.log | grep "19:XX:XX"

# Look for 0x0002 in the PDU bytes (position 8-9):
# If present = SUCCESS!
# If still seeing 0x000A/0x0003 = Need deeper PDU encoding investigation
```

---

## Questions for Next Session

1. **Why does our fix log "sending FormatList" but Lock+FormatListResponse are sent?**
   - Is `ClipboardPdu::FormatList` being encoded to wrong msgType?
   - Is there post-processing changing the PDU type?
   - Is our fix actually being compiled in? (Check cargo checkout hash)

2. **Why does client→server FormatList work (0x0002) but server→client doesn't?**
   - Different code paths for sending vs receiving?
   - Does `process()` handle incoming PDUs differently than `initiate_copy()` generates them?

3. **Where are Lock/FormatListResponse coming from?**
   - Are they remnants from handle_format_list() (client sending to us)?
   - Timing coincidence?
   - Or actually generated by our code path?

---

## Recommended Investigation Path

### Option A: Add Extensive Logging (Recommended)

Modify glamberson/IronRDP fork to add debug logging:

1. **In initiate_copy()** - Log each PDU added to vec
2. **In into_cliprdr_message()** - Log PDU type before encoding
3. **In SvcMessage encoding** - Log msgType being set
4. **In server.rs dispatch** - Log messages being encoded/sent

This will trace the complete path from "push(FormatList)" to actual bytes sent.

### Option B: Compare Working Path

Instrument the handle_format_list() path (client→server) that DOES generate 0x0002:
- See how it encodes FormatListResponse
- Compare with how initiate_copy() encodes FormatList
- Find the difference

### Option C: Direct PDU Construction (Bypass)

If encoding is fundamentally broken for server→client, bypass it:

```rust
use ironrdp_cliprdr::pdu::{FormatList, ClipboardPduHeader};
use ironrdp_pdu::{encode_vec, PduEncode};

// Build PDU manually
let format_list = FormatList::new(formats);
let mut bytes = Vec::new();

// Encode header with correct msgType
let header = ClipboardPduHeader {
    msg_type: 0x0002,  // CB_FORMAT_LIST
    msg_flags: 0,
};
header.encode(&mut bytes)?;

// Encode format list data
format_list.encode_body(&mut bytes)?;

// Send raw bytes
sender.send(ServerEvent::RawClipboardPdu(bytes))?;
```

---

## Files for Next Session

### Logs to Analyze:
- `/tmp/clean-fork-test.log` (on dev machine)
- `~/wayland/wrd-server-specs/clean-fork-test.log` (on KDE VM)

### Code to Review:
- `glamberson/IronRDP`: `crates/ironrdp-cliprdr/src/pdu/mod.rs` - PDU encoding
- `glamberson/IronRDP`: `crates/ironrdp-cliprdr/src/pdu/format_list.rs` - FormatList encoding
- `glamberson/IronRDP`: `crates/ironrdp-server/src/server.rs` line 546-566 - ServerEvent dispatch

### Research Documents:
- `SESSION-SUMMARY-CLIPBOARD-FIX-2025-12-09.md` - Comprehensive research findings
- `KDE-TESTING-GUIDE.md` - Testing procedure and PDU reference
- `SESSION-HANDOVER-CLIPBOARD-BIDIRECTIONAL-2025-12-09.md` - Original problem analysis

---

## Current State Summary

**Windows → Linux:** ✅ WORKING (always has been)

**Linux → Windows:**
- Detection: ✅ Portal (KDE) and D-Bus (GNOME) both detect changes
- Echo Prevention: ✅ Fixed (Portal uses session_is_owner, D-Bus uses timing)
- Format Conversion: ✅ MIME → RDP formats working
- IronRDP Fix: ✅ Running and logging correctly
- PDU Generation: ❌ **STILL BROKEN** - Wrong PDUs despite fix

**The last mile:** PDU encoding/sending is still generating wrong message types.

---

## Success Criteria

**Minimum Goal:**
- See `user_data: [..., 2, 0, ...]` (0x0002 = CB_FORMAT_LIST) in logs
- Windows receives FormatList
- Paste in Windows succeeds

**Complete Goal:**
- Both directions work on KDE (Portal path)
- Both directions work on GNOME (D-Bus path)
- No echo loops
- Clean fork-based architecture ✅
- Upstream PR to IronRDP (after verification)

---

## Repository Cleanup Tasks (Parallel Work)

While debugging PDU encoding, these can be done in parallel:

### 1. Branch Consolidation

**Current branches:**
```
feature/gnome-clipboard-extension     ← Active work
feature/clipboard-monitoring-solution
feature/wlr-clipboard-backend
feature/lamco-compositor-clipboard
main
```

**Proposed:**
- Keep current branch until clipboard fully works
- Merge to main when complete
- Archive experimental branches to `archive/YYYY-MM-DD-branch-name`
- Document why each was archived

### 2. Documentation Structure

**Current:** Files scattered in root
**Proposed:**
```
docs/
├── architecture/
│   └── clipboard-system.md
├── research/
│   ├── ironrdp-investigation/
│   ├── protocol-analysis/
│   └── kde-testing/
├── decisions/
│   └── why-fork-ironrdp.md
└── testing/
    └── clipboard-test-guide.md
```

### 3. DONOTUSE Cleanup

Move valuable content:
- `clipboard-debugging/` → `docs/research/clipboard-investigation-2025-12/`
- Preserve patches as reference
- Keep investigation documents

---

## Commands Reference

### Dev Machine Build

```bash
cd /home/greg/wayland/wrd-server-specs
git pull origin feature/gnome-clipboard-extension
cargo clean
cargo build --release
# Binary: target/release/wrd-server
```

### Deploy to KDE VM

```bash
scp target/release/wrd-server greg@192.168.10.3:~/wayland/wrd-server-specs/target/release/

# Then ON KDE DESKTOP (not SSH):
cd ~/wayland/wrd-server-specs
./target/release/wrd-server -c config.toml 2>&1 | tee test.log
```

### IronRDP Fork Updates

```bash
cd ~/repos/ironrdp-work/IronRDP
git checkout update-sspi-with-clipboard-fix
# Make changes
git add .
git commit -m "..."
git push origin update-sspi-with-clipboard-fix

# Then in wrd-server-specs:
cargo update  # Fetch latest from fork
cargo build --release
```

---

## Resources

**Repositories:**
- wrd-server: https://github.com/lamco-admin/wayland-rdp
- IronRDP fork: https://github.com/glamberson/IronRDP
- IronRDP upstream: https://github.com/Devolutions/IronRDP

**Protocol Specs:**
- MS-RDPECLIP: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpeclip/
- Section 2.2.3.1: Format List PDU
- Section 3.3.5.2.2: Server Sending Format List

**Previous Sessions:**
- SESSION-HANDOVER-CLIPBOARD-BIDIRECTIONAL-2025-12-09.md
- SESSION-SUMMARY-CLIPBOARD-FIX-2025-12-09.md

---

## Final Note: We're Close!

The architecture is now clean and correct:
- ✅ Detection works (Portal + D-Bus)
- ✅ Echo prevention works
- ✅ IronRDP fix is in place
- ✅ Fork-based dependency (no local hacks)

One remaining issue: PDU encoding/sending. The fix says it's sending FormatList, but wrong PDUs appear on the wire.

**This is solvable** - it's now a focused encoding bug, not an architectural problem.

Next session: Add extensive logging to trace PDU encoding path, or implement direct PDU construction bypass if needed.
