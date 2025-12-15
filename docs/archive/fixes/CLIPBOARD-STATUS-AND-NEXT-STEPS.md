# Clipboard Status and Next Steps

**Date:** 2025-11-19
**Current Status:** Architecture research complete, implementation in progress
**Blocker:** Portal Clipboard API integration complexity

---

## Current Situation

**What's working:**
- ✅ Server connects and runs stably
- ✅ Mouse and keyboard fully functional
- ✅ Format announcement from RDP received
- ✅ Non-blocking event queue architecture
- ✅ No deadlocks or crashes

**What's not working:**
- ❌ Clipboard data transfer (no copy/paste functionality)
- ❌ Portal Clipboard API integration incomplete
- ❌ wl-clipboard-rs being removed but replacement not finished

---

## Research Completed

**Comprehensive analysis of:**
1. ✅ RDP CLIPRDR protocol (MS-RDPECLIP spec)
2. ✅ VNC, SPICE clipboard protocols
3. ✅ X11 clipboard architecture (selections, delayed rendering)
4. ✅ Wayland clipboard (wl_data_device protocol)
5. ✅ Portal Clipboard D-Bus API
6. ✅ FreeRDP, xrdp, wayvnc implementations
7. ✅ IronRDP cliprdr crate architecture

**Key finding:** Portal Clipboard API (`ashpd::desktop::clipboard::Clipboard`) provides delayed rendering support required for proper RDP integration.

---

## Correct Architecture (Validated)

**Portal Clipboard delayed rendering model:**

```
Windows copies → RDP FormatList
    ↓
on_remote_copy() callback
    ↓
Portal.SetSelection(mime_types)  ← Announce formats WITHOUT data
    ↓
[User pastes in Linux app]
    ↓
Portal SelectionTransfer signal (mime_type, serial)
    ↓
Request data from RDP client
    ↓
RDP FormatDataResponse
    ↓
on_format_data_response() callback
    ↓
Portal.SelectionWrite(serial, fd) with data
    ↓
Linux app receives data ✅
```

---

## Implementation Challenges Encountered

### 1. Async/Sync Boundary (SOLVED)
- **Problem:** IronRDP callbacks are sync, Portal is async, blocking causes deadlocks
- **Solution:** Non-blocking event queue + async task processing ✅

### 2. Channel Lifecycle (SOLVED)
- **Problem:** Creating channels per connection attempt caused crashes
- **Solution:** Shared queue and single processing task ✅

### 3. Message Proxy Availability (SOLVED)
- **Problem:** Proxy was None during on_remote_copy()
- **Solution:** Create proxy in factory constructor, share via Arc ✅

### 4. Portal API Integration (IN PROGRESS)
- **Problem:** Complex lifetime issues with Session, async closures
- **Challenge:** Migrating from wl-clipboard-rs to Portal API
- **Status:** portal/clipboard.rs rewritten but needs compilation fixes

### 5. RDP Request Mechanism (NOT YET ADDRESSED)
- **Problem:** How to send FormatDataRequest PDU to RDP client
- **Need:** Access to CliprdrServer object or alternative request method
- **Status:** Research needed on IronRDP server API

---

## Recommended Next Steps

### Option A: Complete Portal API Migration (2-3 days)

**Pros:**
- Proper delayed rendering support
- Clean architecture
- Matches industry standard

**Cons:**
- Complex async lifetime issues to resolve
- ashpd API learning curve
- Signal handling complexity

**Tasks:**
1. Fix compilation errors in new portal/clipboard.rs
2. Wire Portal signals to clipboard manager
3. Implement RDP data request mechanism
4. Test thoroughly

### Option B: Simplified Immediate Solution (4-8 hours)

**Use Portal API more simply:**
- Keep existing event queue architecture
- Add Portal SetSelection() for announcements
- Add SelectionTransfer listener for data requests
- Don't try to use full Portal lifecycle yet
- Get text clipboard working as POC

**Then iterate** to full Portal integration.

### Option C: Strategic Pause and Planning (Recommended)

**Given:**
- Server is stable and working (mouse/keyboard perfect)
- Clipboard is complex and requires careful design
- Multiple architectural iterations attempted
- Research phase complete

**Recommend:**
1. Document current progress ✅
2. Create detailed implementation spec for Portal integration
3. Allocate focused time block for clipboard (not fragmented)
4. Consider if clipboard is v1.0 blocker or v1.1 feature

---

## What We've Learned

### wl-clipboard-rs Limitations
- ❌ No format announcement support
- ❌ No delayed rendering
- ❌ No change monitoring
- ❌ Fork-based model (not suitable for daemons)
- ✅ Simple for CLI tools, wrong for RDP integration

### Portal Clipboard Advantages
- ✅ Delayed rendering (SetSelection announces formats)
- ✅ Signal-based (SelectionTransfer notifies data requests)
- ✅ FD-based transfer (efficient, large data)
- ✅ Session-scoped security
- ✅ Standard for Wayland remote desktop

### IronRDP Integration Points
- ✅ CliprdrBackend trait (callbacks for RDP events)
- ✅ CliprdrBackendFactory (creates backends per connection)
- ✅ ClipboardMessage enum (for backend → app communication)
- ⚠️ Server-side request mechanism unclear

---

## File Transfer and Device Sharing Impact

### File Copy/Paste (CF_HDROP + FileContents)

**Architecture:**
```
Windows: Copy files
    ↓
RDP FormatList [CF_HDROP]
    ↓
Portal SetSelection(["text/uri-list"])
    ↓
Linux: Paste
    ↓
Portal SelectionTransfer("text/uri-list", serial)
    ↓
RDP FileContentsRequest (chunked, stream_id, position, size)
    ↓
RDP FileContentsResponse (data chunks)
    ↓
Write to /tmp/wrd-clipboard/file_N
    ↓
Generate URI list: file:///tmp/wrd-clipboard/file_0\nfile:///...
    ↓
Portal SelectionWrite(serial) with URI list
    ↓
Linux file manager shows files ✅
```

**Impact:** ✅ **POSITIVE**
- Portal handles file URIs natively
- FD-based transfer efficient for large files
- Simpler than manual URI handling

**Timeline:** +1-2 days after text/images work

### USB Device Redirection (RDPUSB)

**Completely separate virtual channel:**
- Uses RDPUSB channel (not CLIPRDR)
- Requires USB/IP kernel support
- No clipboard interaction

**Impact:** ❌ **NONE** - Different code path entirely

**Timeline:** 6-9 months (independent feature)

### Drive Mapping (RDPDR)

**Separate device redirection channel:**
- RDPDR channel with SMB2-like protocol
- FUSE filesystem integration
- No clipboard interaction

**Impact:** ❌ **NONE** - Independent feature

**Timeline:** 4-6 months

---

## Production Quality Considerations

### What's Required for v1.0

**Minimum viable clipboard:**
- ✅ Text clipboard both directions
- ✅ Basic loop prevention
- ✅ Stable (no crashes)
- ⚠️ Images and files can be v1.1

**Current blockers:**
- Portal API integration complexity
- RDP request mechanism unclear
- Testing infrastructure needed

### What Can Wait for v1.1

**Advanced clipboard features:**
- Image clipboard (DIB/PNG conversion)
- File transfer (CF_HDROP + FileContents)
- HTML clipboard
- RTF clipboard
- Multiple concurrent format support

---

## Recommendations

### Immediate (Today):

**Document and commit:**
- ✅ Architecture research (done)
- ✅ Portal API design (done)
- ✅ Current status (this document)

**Stabilize current state:**
- ✅ Ensure server runs without crashes
- ✅ Mouse/keyboard working
- ⚠️ Clipboard disabled but not breaking

### Short-term (This Week):

**Focus on basics:**
- Get text clipboard working (one direction first)
- Use Portal API properly
- Thorough testing

**Or defer clipboard to v1.1:**
- Release v1.0 with video + input only
- Clipboard as next major feature

### Medium-term (Next Week):

**Complete clipboard:**
- Both directions working
- Images support
- File transfer

---

## Technical Debt Accumulated

**During clipboard implementation:**
1. Multiple architectural iterations (wl-clipboard-rs → Portal API)
2. Deadlock bugs discovered and fixed
3. Channel lifecycle bugs found and fixed
4. Understanding of server vs client role evolved

**Positive outcomes:**
- Deep understanding of RDP clipboard protocol
- Non-blocking architecture proven
- Server stability maintained through debugging
- Research completed and documented

**Cleanup needed:**
- Remove TODO comments added during debugging
- Consolidate clipboard documentation
- Finish Portal API integration or revert cleanly

---

## Decision Required

**Path forward:**

**A. Continue clipboard implementation**
- Finish Portal API integration
- Aim for working text clipboard this session
- Timeline: 4-8 hours more work

**B. Defer clipboard to focused session**
- Revert to stable state (no clipboard)
- Plan dedicated clipboard implementation session
- Ship v1.0 without clipboard (video + input only)

**C. Implement basic text clipboard with simpler approach**
- Forget Portal delayed rendering for now
- Direct read/write on paste events
- Get something working, optimize later

---

**Status:** Awaiting direction on clipboard implementation strategy.

**Server core functionality:** ✅ STABLE AND WORKING
**Clipboard functionality:** ⚠️ IN PROGRESS, COMPLEX
