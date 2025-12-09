# Session Handover: Bidirectional Clipboard Implementation

**Date:** 2025-12-09
**Branch:** `feature/gnome-clipboard-extension`
**Current Commit:** `14b0f76` (Move debugging artifacts to DONOTUSE)
**Status:** Windows→Linux ✅ | Linux→Windows ❌

---

## Test Environment

### Test Box Access
```bash
ssh greg@192.168.10.205
Password: Bibi4189
```

**Working Directory:** `~/wayland/wrd-server-specs`
**Desktop:** GNOME 47 on Wayland
**RDP Client:** Windows 11 (192.168.10.x)

### Build Commands
```bash
# On test box (192.168.10.205):
cd ~/wayland/wrd-server-specs
git pull                      # Pull from dev machine
cargo build --release         # Build
./target/release/wrd-server -c config.toml  # Run
```

**IMPORTANT:** Test box directory is NOT a git repo. All git operations happen on dev machine at `/home/greg/wayland/wrd-server-specs`, then code is deployed to test box.

---

## Current Status

### What Works ✅

**Windows → Linux Clipboard:**
- Windows user copies text (Ctrl+C)
- RDP client sends FormatList PDU to server
- Server receives via `on_remote_copy()` callback
- Server announces to Portal via `SetSelection`
- Linux user pastes (Ctrl+V)
- Portal sends `SelectionTransfer` signal
- Server requests data from RDP via `ServerEvent::Clipboard(SendInitiatePaste)`
- RDP sends FormatDataResponse
- Server writes to Portal via `SelectionWrite`
- ✅ **Paste succeeds in Linux**

**D-Bus Echo Loop Prevention:**
- D-Bus signals within 2 seconds of RDP ownership are blocked (identified as echoes)
- D-Bus signals after 2 seconds allowed (real user copies)
- Uses timestamp-based protection in `ClipboardState::RdpOwned`
- Fixed in commits: e1001c6, 161a63e, df060b5

### What Doesn't Work ❌

**Linux → Windows Clipboard:**
- Linux user copies text (Ctrl+C)
- D-Bus extension detects clipboard change
- Server calls `handle_portal_formats()`
- Server sends `ServerEvent::Clipboard(SendInitiateCopy(formats))`
- **BUG:** `initiate_copy()` generates wrong PDUs
  - Expected: `CB_FORMAT_LIST (0x0002)` to announce formats
  - Actually sent: `CB_LOCK_CLIPDATA (0x000A)` + `CB_FORMAT_LIST_RESPONSE (0x0003)`
- Windows never receives format list
- ❌ **Paste fails in Windows**

---

## Technical Investigation Summary

### Root Cause Identified

From detailed log analysis (irondbg1.log):

**Timeline of Failed Linux Copy:**
```
00:37:26.100240 - SendInitiateCopy dispatched with CF_TEXT + CF_UNICODETEXT
00:37:26.104792 - CB_LOCK_CLIPDATA (0x000A) sent ← WRONG
00:37:26.109831 - CB_FORMAT_LIST_RESPONSE (0x0003) sent ← WRONG
00:37:26.xxx - NO CB_FORMAT_LIST (0x0002) ← MISSING!
```

**Why Wrong PDUs Are Sent:**

IronRDP creates multiple `CliprdrServer` backend instances when connections fail/retry:
```
00:37:20.804396 - Backend #1 created
00:37:20.851909 - ERROR: Connection reset by peer
00:37:20.872072 - Backend #2 created
00:37:21.043242 - ONE cliprdr transitions to Ready state
```

When `ServerEvent::Clipboard(SendInitiateCopy)` is processed:
- Event loop calls `get_svc_processor()` to get cliprdr instance
- May return wrong backend (from failed connection)
- Wrong backend is in `Initialization` state
- `initiate_copy()` with `state=Initialization` generates wrong PDUs

**IronRDP State Machine:**
```rust
match (self.state, R::is_server()) {
    (CliprdrState::Ready, _) => {
        // Send CB_FORMAT_LIST ← Need this
    }
    _ => {
        error!("incorrect state");
        // Returns empty Vec ← Getting this
    }
}
```

The state machine is designed for CLIENTS (need Init→Ready transition). But SERVERS should be able to announce clipboard anytime after channel negotiation.

---

## Architecture: Two Clipboard Monitoring Paths

### Path 1: Portal SelectionOwnerChanged (Non-GNOME)

**Used on:** KDE, wlroots, Sway, etc.
**Mechanism:** Portal emits `SelectionOwnerChanged` signal when clipboard changes
**Status:** UNTESTED (all testing done on GNOME where this doesn't work)

```rust
// In manager.rs::start_owner_changed_listener()
portal.start_owner_changed_listener(owner_tx).await
// Signal handler sends: PortalFormatsAvailable(mime_types, false)
// force=false because might be echo of our SetSelection
```

### Path 2: D-Bus Extension (GNOME Only)

**Used on:** GNOME Shell
**Mechanism:** Custom GNOME extension polls clipboard, emits D-Bus signals
**Extension:** `wayland-rdp-clipboard` (separate repo)
**Status:** Partially working (detects copies, but announcement to Windows fails)

```rust
// In manager.rs::start_dbus_clipboard_listener()
bridge.start_signal_listener(dbus_tx).await
// Signal handler sends: PortalFormatsAvailable(mime_types, true)
// force=true because D-Bus is authoritative source
```

**D-Bus Extension Details:**
- Location: Separate GNOME Shell extension repo
- Polls `St.Clipboard` every 200ms
- Emits `ClipboardChanged(mime_types, content_hash, is_primary)` on D-Bus
- Our server subscribes via `zbus` library
- Hash-based loop suppression (2-second window)

---

## Code Structure

### Key Files

**manager.rs** - Main clipboard coordination
- `/home/greg/wayland/wrd-server-specs/src/clipboard/manager.rs`
- `handle_rdp_format_list()` - Windows→Linux copy flow
- `handle_portal_formats()` - Linux→Windows copy flow (BROKEN)
- `start_selection_transfer_listener()` - Windows→Linux paste flow
- `start_owner_changed_listener()` - Portal monitoring (non-GNOME)
- `start_dbus_clipboard_listener()` - D-Bus monitoring (GNOME)

**sync.rs** - State management and loop detection
- `/home/greg/wayland/wrd-server-specs/src/clipboard/sync.rs`
- `ClipboardState` enum: `Idle | RdpOwned(formats, timestamp) | PortalOwned(mimes)`
- Timestamp added in df060b5 for D-Bus echo protection
- `handle_rdp_formats()` - Sets RdpOwned state
- `handle_portal_formats(mime_types, force)` - Sets PortalOwned state (with timing check)

**ironrdp_backend.rs** - IronRDP integration
- `/home/greg/wayland/wrd-server-specs/src/clipboard/ironrdp_backend.rs`
- `WrdCliprdrFactory` - Creates backends per connection
- `WrdCliprdrBackend` - Implements `CliprdrBackend` trait
- `on_remote_copy()` - Receives Windows FormatList
- `on_format_data_request()` - Handles Windows paste requests
- `on_format_data_response()` - Receives clipboard data from Windows

---

## Commits History (Since Last Working State)

```
14b0f76 - Move debugging artifacts to DONOTUSE
a4c76f5 - Patch IronRDP cliprdr: servers always send FormatList
e851c48 - Document Linux→Windows clipboard investigation
9756102 - Patch IronRDP server for detailed clipboard event tracing
6caf26f - Enable TRACE logging for IronRDP server and cliprdr
7f2ba50 - Enable IronRDP cliprdr and server logging for debugging
55aff69 - Test ServerEvent::Clipboard in on_ready() callback
54683c2 - Add detailed ServerEvent send logging for Linux→Windows clipboard
b9cb29f - Add debug logging for FormatList contents
df060b5 - Fix D-Bus echo loop with time-based state protection ← LAST STABLE
1f7e16d - Check clipboard state before SelectionTransfer requests
161a63e - Skip loop detector for authoritative D-Bus signals
e1001c6 - Fix bidirectional clipboard with force flag for D-Bus signals
38c37bb - Fix clipboard echo loop when Windows copies
1bd4d70 - Fix Linux→Windows clipboard: use ServerEvent instead of callback
```

**Key Commit:** df060b5 is last known stable state where Windows→Linux works and D-Bus echo loop is fixed.

---

## Dependencies

### IronRDP (Git Dependencies)

```toml
# From Cargo.toml
ironrdp = { git = "https://github.com/allan2/IronRDP", branch = "update-sspi" }
ironrdp-server = { git = "https://github.com/allan2/IronRDP", branch = "update-sspi" }
ironrdp-cliprdr = { git = "https://github.com/allan2/IronRDP", branch = "update-sspi" }
```

**Git Checkout Location:** `~/.cargo/git/checkouts/ironrdp-4ef039df412dfe33/c580de5/`

**Modified Source (locally only, not on test box):**
- `~/.cargo/git/.../ironrdp-cliprdr/src/lib.rs` - Patched `initiate_copy()` method
- `~/.cargo/registry/.../ironrdp-server-0.9.0/src/server.rs` - Added debug logging

---

## Debugging Artifacts (in DONOTUSE/)

All comprehensive investigation files moved to `DONOTUSE/clipboard-debugging/`:

1. **CLIPBOARD-LINUX-TO-WINDOWS-INVESTIGATION.md** - Full technical analysis
2. **ironrdp-cliprdr-server-initiate-copy.patch** - IronRDP fix patch
3. **apply-patches.sh** - Script to apply patches (didn't work across machines)
4. **ironrdp_server_patch.diff** - Alternative patch format

**Log Files (on test box at /home/greg/wayland-rdp/):**
- `dectest1-4.log` - Initial testing, echo loop discovery
- `dectest5.log` - D-Bus echo loop confirmed
- `dectest6-9.log` - Echo loop fixes and state checking
- `irondbg.log` - IronRDP cliprdr logging enabled
- `irondbg1.log` - **PRIMARY EVIDENCE** - Full TRACE logs showing wrong PDUs
- `irondbg2.log` - After attempted IronRDP patch (didn't work)

---

## Next Steps: KDE Testing Plan

### Why KDE Testing Is Critical

GNOME's SelectionOwnerChanged doesn't work, so we use D-Bus extension. But:
1. SelectionOwnerChanged SHOULD work on KDE/wlroots/Sway
2. If Linux→Windows works on KDE via SelectionOwnerChanged, the issue is D-Bus-specific
3. If it ALSO fails on KDE, the issue is in `SendInitiateCopy` architecture

### KDE Test Box Setup

**Create new VM/box with:**
- KDE Plasma on Wayland (NOT X11!)
- Debian 13 or Ubuntu 24.04
- Same network (192.168.10.x)
- SSH access
- wrd-server build dependencies

**Test Procedure:**
1. Build and run wrd-server on KDE box
2. Connect from Windows RDP client
3. Test Windows→Linux paste (should work)
4. **CRITICAL:** Test Linux→Windows copy
   - Copy text in Linux (Ctrl+C)
   - Check server logs for "SelectionOwnerChanged event"
   - Check if `SendInitiateCopy` is called
   - Paste in Windows (Ctrl+V)
5. Compare logs with GNOME logs

**Expected Outcomes:**

If Linux→Windows **WORKS on KDE:**
- Issue is D-Bus extension integration
- Fix: Improve D-Bus signal handling or timing
- Path forward: Make D-Bus work like SelectionOwnerChanged

If Linux→Windows **FAILS on KDE too:**
- Issue is fundamental `SendInitiateCopy` problem
- Affects both Portal and D-Bus paths
- Fix: Bypass or patch IronRDP's `initiate_copy()` method
- Confirms my IronRDP patch is correct approach

---

## Technical Deep Dive

### SendInitiateCopy vs SendInitiatePaste

**SendInitiatePaste (Windows→Linux) - WORKS:**
```
Flow: Portal signal → ServerEvent(SendInitiatePaste(format_id))
      → cliprdr.initiate_paste() → CB_FORMAT_DATA_REQUEST → Windows responds
```

**SendInitiateCopy (Linux→Windows) - BROKEN:**
```
Flow: D-Bus signal → ServerEvent(SendInitiateCopy(formats))
      → cliprdr.initiate_copy() → ??? WRONG PDUs → Windows ignores
```

### PDU Analysis from irondbg1.log

**After SendInitiateCopy dispatch:**
```python
# Bytes sent (line 2825):
PDU 1: [12, 0, 0, 0, 19, 0, 0, 0, 10, 0, ...]  # msgType=0x000A = CB_LOCK_CLIPDATA
PDU 2: [8, 0, 0, 0, 19, 0, 0, 0, 3, 0, ...]    # msgType=0x0003 = CB_FORMAT_LIST_RESPONSE

# Missing:
PDU: [?, ?, ?, ?, 19, 0, 0, 0, 2, 0, ...]      # msgType=0x0002 = CB_FORMAT_LIST
```

**Why Wrong PDUs:**
- Lock + FormatListResponse are what SERVER sends when RESPONDING to client's FormatList
- But we're trying to ANNOUNCE our own FormatList
- The cliprdr state machine is confused about direction

### IronRDP Source Locations

**Git Source (on dev machine):**
```
~/.cargo/git/checkouts/ironrdp-4ef039df412dfe33/c580de5/crates/ironrdp-cliprdr/src/lib.rs
```

**Registry Cache (on dev machine):**
```
~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ironrdp-cliprdr-0.4.0/src/lib.rs
~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ironrdp-server-0.9.0/src/server.rs
```

**Modified Methods:**
- `Cliprdr::initiate_copy()` at line 238 - State machine logic
- `RdpServer::dispatch_server_events()` at line 490 - Event processing

---

## Attempted Fixes (All in DONOTUSE/)

### Fix 1: D-Bus Echo Loop Protection ✅
**Commits:** e1001c6, 161a63e, df060b5
**Problem:** D-Bus signals after Portal writes were changing state RdpOwned→PortalOwned
**Solution:** Added timestamp to RdpOwned, block D-Bus signals within 2 seconds
**Result:** Windows→Linux now works reliably

### Fix 2: SelectionTransfer State Checking ✅
**Commit:** 1f7e16d
**Problem:** Asking RDP for data when Portal owns clipboard (causes errors)
**Solution:** Check state before sending FormatDataRequest
**Result:** Prevents "Format data response received with error flag" spam

### Fix 3: IronRDP Cliprdr State Machine Patch ❌
**Commits:** 55aff69, 7f2ba50, 9756102, a4c76f5, 244e192
**Problem:** `initiate_copy()` won't send FormatList when state=Initialization
**Attempted Solution:** Patch cliprdr to bypass state check for servers
**Result:** Didn't work (cargo caching, can't deploy patches across machines)
**Files:** See DONOTUSE/clipboard-debugging/

---

## Key Source Code Sections

### D-Bus Signal Flow (GNOME)

```rust
// manager.rs:461
async fn start_dbus_clipboard_listener(&self) {
    let mut bridge = DbusBridge::new();
    bridge.start_signal_listener(dbus_tx).await;

    // Forward D-Bus events as PortalFormatsAvailable
    event_tx.send(ClipboardEvent::PortalFormatsAvailable(
        dbus_event.mime_types,
        true  // force=true: authoritative D-Bus signal
    ))
}
```

### Portal Signal Flow (KDE/Others)

```rust
// manager.rs:392
async fn start_owner_changed_listener(&self) {
    portal.start_owner_changed_listener(owner_tx).await;

    // Forward Portal events as PortalFormatsAvailable
    event_tx.send(ClipboardEvent::PortalFormatsAvailable(
        mime_types,
        false  // force=false: might be echo
    ))
}
```

### Format Announcement (BROKEN)

```rust
// manager.rs:1073
async fn handle_portal_formats(mime_types, force, ...) {
    // Convert MIME → RDP formats
    let rdp_formats = converter.mime_to_rdp_formats(&mime_types)?;

    // Convert to IronRDP types
    let ironrdp_formats: Vec<ClipboardFormat> = ...;

    // Send to RDP client
    sender.send(ServerEvent::Clipboard(
        ClipboardMessage::SendInitiateCopy(ironrdp_formats)  // ← THIS FAILS
    ))
}
```

---

## Evidence: Complete Event Sequences

### Windows→Linux (WORKING)

From any log file:
```
1. Windows copies → RDP sends FormatList
2. Server: on_remote_copy() → handle_rdp_format_list()
3. Server: SetSelection to Portal (announce formats)
4. Linux pastes → Portal sends SelectionTransfer
5. Server: SendInitiatePaste → RDP sends data
6. Server: SelectionWrite to Portal
✅ Paste succeeds
```

### Linux→Windows (BROKEN)

From irondbg1.log lines 2805-2830:
```
1. Linux copies → D-Bus detects change
2. Server: handle_portal_formats(force=true)
3. Server: SendInitiateCopy(CF_TEXT, CF_UNICODETEXT) dispatched
4. IronRDP: initiate_copy() called
5. Wrong PDUs sent: Lock + FormatListResponse
6. Windows never sees FormatList
❌ Windows doesn't know Linux has clipboard
```

---

## Configuration Files

### config.toml
```toml
[clipboard]
enabled = true
max_data_size = 16777216  # 16MB
enable_images = true
```

### GNOME Extension D-Bus Interface
```
Bus: Session bus
Name: io.github.lamco.WaylandRdp
Path: /io/github/lamco/WaylandRdp/Clipboard
Interface: io.github.lamco.WaylandRdp.Clipboard
Signal: ClipboardChanged(as mime_types, s content_hash, b is_primary)
```

---

## Tomorrow's Action Plan

### Step 1: Set Up KDE Test Environment

**Option A: New VM**
```bash
# Download KDE neon or Kubuntu 24.04 ISO
# Create VM with:
- 2GB RAM minimum
- Wayland session (NOT X11!)
- Network bridge to 192.168.10.x
- SSH server enabled
```

**Option B: Dual Boot / Container**
- Install KDE Plasma on existing test box
- Use different session (GNOME vs KDE)
- Share same wrd-server build

### Step 2: Test on KDE

```bash
# On KDE box:
cd ~/wayland/wrd-server-specs
cargo build --release
./target/release/wrd-server -c config.toml 2>&1 | tee kde-test.log

# Test both directions:
1. Windows→Linux (should work)
2. Linux→Windows (CRITICAL TEST)
```

**Look for in logs:**
```
"SelectionOwnerChanged event" - Confirms Portal monitoring works
"📤 Sending ServerEvent::Clipboard(SendInitiateCopy)" - Confirms copy detected
"McsMessage::SendDataRequest.*0002" - Confirms CB_FORMAT_LIST sent
```

### Step 3A: If KDE Works

**Conclusion:** D-Bus integration has timing/architectural issues

**Fix Options:**
1. Improve D-Bus signal timing/filtering
2. Make D-Bus mimic SelectionOwnerChanged exactly
3. Add retry logic for SendInitiateCopy
4. Investigate why D-Bus path triggers wrong state

### Step 3B: If KDE Also Fails

**Conclusion:** `SendInitiateCopy` fundamentally broken in current architecture

**Fix Options:**
1. Fork IronRDP and fix cliprdr state machine properly
2. Bypass `initiate_copy()` entirely - construct FormatList PDU manually
3. Use callback-based approach instead of ServerEvent
4. File IronRDP bug report with evidence

---

## Alternative Architectures to Consider

### Option 1: Direct PDU Construction

Instead of `SendInitiateCopy`, manually build and send FormatList PDU:

```rust
use ironrdp_cliprdr::pdu::*;
use ironrdp_pdu::encode_vec;

// Build FormatList PDU directly
let format_list = FormatList::new(vec![
    ClipboardFormat { id: ClipboardFormatId(1), name: None },
    ClipboardFormat { id: ClipboardFormatId(13), name: None },
]);

// Encode to bytes
let pdu_bytes = encode_vec(&format_list)?;

// Send via lower-level channel (bypass cliprdr state machine)
```

### Option 2: Shared Backend Instance

Store Arc<Mutex<CliprdrServer>> in ClipboardManager:
```rust
pub struct ClipboardManager {
    // ...
    active_cliprdr: Arc<Mutex<Option<Arc<Mutex<CliprdrServer>>>>>,
}
```

Set it when backend becomes ready, call `initiate_copy()` directly instead of via ServerEvent.

### Option 3: Polling Clipboard State

Since Portal polling was disabled (causes session lock contention), use different approach:
- Poll clipboard hash every 500ms
- On change, read content directly and send to RDP
- Bypass delayed rendering entirely
- Trade latency for reliability

---

## Known Issues

### Issue 1: Connection Retries Create Multiple Backends

**Evidence:** irondbg1.log shows 2 backends built
**Impact:** ServerEvent may go to wrong backend instance
**Mitigation:** Ensure only active backend processes events

### Issue 2: IronRDP State Machine Client-Centric

**Evidence:** `initiate_copy()` requires Ready state, but multiple backends leave some in Init
**Impact:** Servers can't reliably announce clipboard
**Solution:** Patch IronRDP or bypass state machine

### Issue 3: Ctrl+C Timing

**Observation:** When user presses Ctrl+C in Linux via RDP, Windows also processes it
**Result:** Both Linux and Windows try to copy simultaneously
**Impact:** Race condition in clipboard announcement
**Unknown:** Does this cause state confusion?

---

## Questions to Answer Tomorrow

1. **Does SelectionOwnerChanged work on KDE for Linux→Windows?**
   - If YES: D-Bus is the problem
   - If NO: IronRDP/architecture is the problem

2. **Are connection retries causing the multiple backend issue consistently?**
   - Test with stable connection (no retries)
   - Check if wrong PDUs still sent

3. **Can we eliminate connection errors entirely?**
   - Investigate "Connection reset by peer" cause
   - Fix TLS/finalize issue
   - Single backend = no state confusion

4. **Is there a simpler clipboard announcement method?**
   - Check IronRDP examples/tests
   - Look for alternative APIs
   - Consider RDP protocol documentation directly

---

## Commands Reference

### Development Machine (192.168.10.x)
```bash
cd /home/greg/wayland/wrd-server-specs
git status
git log --oneline -10
git diff df060b5 HEAD -- src/clipboard/
```

### Test Box (192.168.10.205)
```bash
ssh greg@192.168.10.205
cd ~/wayland/wrd-server-specs
# Pull is done from dev machine, build here
cargo build --release
./target/release/wrd-server -c config.toml 2>&1 | tee test.log
```

### Log Analysis
```bash
# Copy logs from test box
scp greg@192.168.10.205:/home/greg/wayland-rdp/*.log /tmp/

# Search for key events
grep "SendInitiateCopy" /tmp/test.log
grep "CB_FORMAT_LIST" /tmp/test.log
grep "McsMessage::SendDataRequest" /tmp/test.log | python3 decode_pdus.py
```

---

## Success Criteria

### Minimum Viable Product
- ✅ Windows→Linux clipboard (paste) - **DONE**
- ❌ Linux→Windows clipboard (copy+paste) - **IN PROGRESS**

### Full Success
- Both directions work on GNOME (via D-Bus extension)
- Both directions work on KDE/others (via SelectionOwnerChanged)
- No echo loops or state corruption
- Handles connection retries gracefully
- Logging can be disabled for production

---

## Contact & Resources

**Repository:** https://github.com/lamco-admin/wayland-rdp
**Branch:** `feature/gnome-clipboard-extension`
**IronRDP Source:** https://github.com/allan2/IronRDP (branch: update-sspi)
**MS-RDPECLIP Spec:** https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpeclip/

**Related Documentation:**
- `SESSION-HANDOVER-CLIPBOARD-2025-11-19.md` - Original clipboard work
- `SESSION-HANDOVER-CLIPBOARD-COMPLETE-2025-11-19.md` - Completion handover
- `DONOTUSE/clipboard-debugging/` - Investigation artifacts

---

## Final Notes

The Linux→Windows clipboard issue is **NOT a regression** - it was never fully working. Commit 1bd4d70 "Fix Linux→Windows clipboard" actually only fixed the PASTE direction (sending data to Windows), not the COPY announcement.

We're implementing Linux→Windows COPY for the first time with the D-Bus extension, and discovering that `SendInitiateCopy` has fundamental issues with IronRDP's cliprdr state machine when multiple backend instances exist.

The proper fix requires either:
1. Patching IronRDP (done locally, needs proper deployment)
2. Testing on KDE to isolate if it's D-Bus-specific
3. Bypassing `initiate_copy()` with manual PDU construction

Windows→Linux works perfectly and is production-ready. Linux→Windows needs architectural decision on fix approach.
