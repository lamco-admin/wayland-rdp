# Architecture and Status Review - December 21, 2025

**Project:** lamco-rdp-server (Portal mode RDP server)
**Current State:** VIDEO WORKING, CLIPBOARD NOT WORKING
**Test Environment:** GNOME Wayland @ 192.168.10.205

---

## CURRENT STATUS SUMMARY

### ✅ What's Working

1. **RDP Connection** - TLS handshake, authentication
2. **Video Streaming** - RemoteFX codec, 30 FPS, 1280x800
3. **Input Injection** - Keyboard and mouse (not tested yet, but infrastructure present)
4. **Portal Integration** - Session creation, screen sharing permissions

### ❌ What's Not Working

1. **Clipboard** - Neither direction (Linux → Windows, Windows → Linux)

### 🔧 Critical Bug Fixed Today

**FD Ownership Bug:**
- Portal returned OwnedFd
- PortalSessionHandle stored it
- session_handle dropped after extracting FD
- FD closed before PipeWire could use it
- Result: Stream stuck in Connecting state forever

**Fix:**
- Changed to RawFd with std::mem::forget()
- Prevents premature FD closure
- Video now works!

---

## ARCHITECTURE OVERVIEW

### Data Flow Diagram

```
┌─────────────┐
│ RDP Client  │
└──────┬──────┘
       │ TLS 1.3
       ▼
┌─────────────────────┐
│  IronRDP Server     │
│  - Protocol         │
│  - Codecs           │
│  - Channels         │
└──────┬──────────────┘
       │
       ├──────────────────────┐
       │                      │
       ▼                      ▼
┌──────────────┐    ┌──────────────────┐
│ Display      │    │ Input Handler    │
│ Handler      │    │                  │
└──────┬───────┘    └─────────┬────────┘
       │                      │
       ▼                      ▼
┌──────────────┐    ┌──────────────────┐
│ PipeWire     │    │ Portal Remote    │
│ Capture      │    │ Desktop API      │
└──────┬───────┘    └─────────┬────────┘
       │                      │
       └──────────┬───────────┘
                  ▼
       ┌─────────────────────┐
       │  XDG Desktop Portal  │
       │  - ScreenCast        │
       │  - RemoteDesktop     │
       │  - Clipboard         │
       └──────────┬───────────┘
                  ▼
       ┌─────────────────────┐
       │  Wayland Compositor │
       │  (GNOME/KDE/etc)    │
       └─────────────────────┘
```

---

## IMPLEMENTED FEATURES

### 1. Video Streaming ✅ WORKING

**Module:** `src/server/display_handler.rs` (680 lines)

**Flow:**
```
Portal.create_session()
  → PipeWire FD
  → PipeWireThreadManager (dedicated thread, handles !Send types)
  → Stream.connect(node_id, Direction::Input)
  → process() callback fires
  → VideoFrame extracted (MemFd/MemPtr/DmaBuf support)
  → BitmapConverter (lamco-video)
  → IronRDP DisplayUpdate
  → RemoteFX encoding
  → RDP Client
```

**Codec:** RemoteFX (IronRDP built-in)
- Lossless wavelet compression
- Good for desktop (text, UI)
- CPU-based encoding

**Buffer Types Supported:**
- MemPtr (type=1) - Direct memory
- MemFd (type=2) - Memory-mapped FD ✅ Currently using
- DmaBuf (type=3) - GPU buffers

**Performance:**
- Frame processing: 0.9-6ms
- Target: 30 FPS
- Resolution: Dynamic (current 1280x800)

### 2. H.264/EGFX Codec 🟡 IMPLEMENTED BUT NOT ENABLED

**Module:** `src/egfx/` (1,801 lines)

**Files:**
- `mod.rs` (444 lines) - EGFX channel protocol
- `encoder.rs` (665 lines) - H.264 encoder (OpenH264)
- `video_handler.rs` (452 lines) - Frame encoding pipeline
- `surface.rs` (240 lines) - Surface management

**Status:** Code complete, feature flag exists

**To Enable:**
```toml
# In Cargo.toml
[features]
default = ["pam-auth", "h264"]  # Add h264

# Or build with:
cargo build --release --features h264
```

**What H.264/EGFX Provides:**
- Hardware-accelerated encoding (if VAAPI available)
- Better compression than RemoteFX (lower bandwidth)
- Optimized for video/movement
- Uses Dynamic Virtual Channel (DVC)

**Implementation:**
- AVC420 codec (H.264 YUV420)
- Surface management for multi-monitor
- Frame acknowledgment flow control
- OpenH264 encoder integration

**Why Not Enabled:**
- Requires IronRDP PR #1057 (EGFX support) - still using git patches
- Needs testing
- RemoteFX works fine for now

### 3. Input Handling ✅ IMPLEMENTED (NOT TESTED)

**Module:** `src/server/input_handler.rs` (660 lines)

**Capabilities:**
- Keyboard: 200+ scancode mappings (RDP → evdev)
- Mouse: Absolute/relative motion, buttons, wheel
- Coordinate transformation for multi-monitor
- Event batching (10ms intervals to reduce Portal API spam)

**Flow:**
```
RDP Client input
  → IronRDP RdpServerInputHandler trait
  → WrdInputHandler::keyboard() / mouse()
  → Input queue (priority 1, size 32)
  → Batching task (10ms flush)
  → KeyboardHandler / MouseHandler (lamco-rdp-input)
  → CoordinateTransformer
  → Portal.notify_keyboard_keycode() / notify_pointer_motion_absolute()
  → Wayland Compositor
```

**Status:** Infrastructure complete, not manually tested yet

### 4. Clipboard Sync ❌ NOT WORKING

**Module:** `src/clipboard/` (116KB, 5 files)

**Architecture:** TWO PATHS

#### Path A: Portal API (Standard - for KDE, Sway, wlroots)

**Linux → Windows (Copy from Linux):**
```
User copies in Linux
  → Portal SelectionOwnerChanged signal
  → ClipboardManager receives PortalFormatsAvailable
  → Read clipboard via Portal.get_selection()
  → Send FormatList to RDP client
  → Client requests data
  → Send FormatDataResponse
```

**Windows → Linux (Paste to Linux):**
```
RDP client sends FormatList
  → ClipboardManager announces via Portal.set_selection()
  → User pastes in Linux app
  → Portal sends SelectionTransfer signal
  → ClipboardManager requests data from RDP
  → Write data via Portal.write_selection()
```

**Status:** ❌ SelectionOwnerChanged doesn't fire on GNOME (Portal API limitation)

#### Path B: D-Bus Extension (GNOME Workaround)

**Purpose:** GNOME Portal doesn't emit SelectionOwnerChanged signals

**GNOME Extension:** `wayland-rdp-clipboard@wayland-rdp.io`
- Monitors GNOME's internal clipboard (`St.Clipboard`)
- Polls every 500ms
- Emits `ClipboardChanged` signal on D-Bus
- **ONLY for Linux → Windows direction**

**Linux → Windows (via Extension):**
```
User copies in Linux
  → GNOME extension polls St.Clipboard
  → Detects change, emits D-Bus ClipboardChanged signal
  → ClipboardManager receives signal
  → Read clipboard via Portal.get_selection()
  → Send FormatList to RDP client
```

**Windows → Linux (via Portal):**
```
Same as Path A - uses Portal SelectionTransfer
(This part works on GNOME)
```

**Current Status:**
- Extension: ACTIVE and running ✅
- D-Bus bridge: Connected ✅
- Events detected: Yes (saw clipboard change #1) ✅
- **But:** Clipboard still doesn't work ❌

**Issue:** Need to debug why FormatList isn't reaching RDP client

### 5. Multi-Monitor 🟡 IMPLEMENTED (NOT TESTED)

**Module:** `src/multimon/` (2 files)

**Capabilities:**
- Layout negotiation (DisplayControl channel)
- Monitor enumeration from Portal streams
- Coordinate transformation
- Surface management

**Status:** Code exists, not tested with multiple monitors

---

## CODEC COMPARISON

| Feature | RemoteFX | H.264/EGFX |
|---------|----------|------------|
| **Type** | Wavelet | Video codec |
| **Encoding** | CPU | CPU or GPU (VAAPI) |
| **Quality** | Lossless | Lossy (configurable) |
| **Best For** | Desktop, text, UI | Video, movement |
| **Bandwidth** | Medium-High | Low-Medium |
| **Latency** | Low | Low |
| **Status** | ✅ Working | 🟡 Implemented, not enabled |
| **Implementation** | IronRDP built-in | src/egfx/ (1,801 lines) |
| **Feature Flag** | Always on | `h264` (optional) |

**To Switch to H.264:**
```bash
cargo build --release --features h264
```

**Requirements:**
- OpenH264 library (BSD license)
- IronRDP DVC support (using git patches already)
- Client must support EGFX

---

## CODE SIZE

**Total:** ~10,831 lines (server orchestration only)

| Module | Lines | Purpose |
|--------|-------|---------|
| clipboard/ | ~3,145 | Clipboard state machine, bridging |
| egfx/ | 1,801 | H.264/EGFX implementation |
| server/ | ~680 | Display handler, input handler, multiplexer |
| config/ | ~200 | Configuration management |
| security/ | ~400 | TLS, authentication |
| multimon/ | ~300 | Multi-monitor layout |
| utils/ | ~500 | Diagnostics, helpers |

**Published Crates (open source infrastructure):**
- lamco-portal (600 lines) - XDG Portal integration
- lamco-pipewire (3,392 lines) - PipeWire capture
- lamco-video (1,735 lines) - Frame processing
- lamco-rdp-input (3,727 lines) - Input translation
- lamco-clipboard-core (1,500 lines) - Clipboard primitives
- lamco-rdp-clipboard (1,645 lines) - RDP clipboard backend

**Total codebase:** ~30,000 lines

---

## TESTING STATUS

### Video ✅
- [x] Portal session creation
- [x] PipeWire stream connection
- [x] Frame capture (MemFd buffers)
- [x] RemoteFX encoding
- [x] RDP client display
- [x] 30 FPS streaming
- [x] Good performance (<6ms/frame)

### Input ⏳ NOT TESTED
- [x] Code implemented
- [ ] Manual testing (keyboard)
- [ ] Manual testing (mouse)
- [ ] Multi-monitor coordinates

### Clipboard ❌ NOT WORKING
- [x] Infrastructure complete
- [x] GNOME extension active
- [x] D-Bus bridge connected
- [x] Extension detects changes
- [ ] Linux → Windows (not sending FormatList to RDP)
- [ ] Windows → Linux (not tested)

### H.264/EGFX ⏳ NOT TESTED
- [x] Code implemented (1,801 lines)
- [x] Feature flag exists
- [ ] Build with h264 feature
- [ ] Test encoding
- [ ] Test client compatibility

---

## GNOME EXTENSION DETAILS

**Name:** `wayland-rdp-clipboard@wayland-rdp.io`
**Purpose:** **ONLY Linux → Windows clipboard** (copy from Linux, paste to Windows)
**Why:** GNOME Portal doesn't emit SelectionOwnerChanged signals

**What it does:**
1. Polls `St.Clipboard` every 500ms
2. Detects changes via content hash
3. Emits `ClipboardChanged(mime_types, hash)` signal on D-Bus
4. Server receives signal and announces to RDP client

**What it DOESN'T do:**
- Windows → Linux (that uses Portal SelectionTransfer - built into Portal)

**Direction Summary:**
- **Linux → Windows:** Needs extension (Portal API limitation on GNOME)
- **Windows → Linux:** Uses Portal (works without extension)

---

## CLIPBOARD ARCHITECTURE PATHS

### Portal Path (Standard)

**Works on:** KDE, Sway, wlroots, Hyprland (NOT GNOME)

**Linux → Windows:**
```
Linux clipboard change
  → Portal emits SelectionOwnerChanged signal
  → We receive signal with MIME types
  → Read data via Portal.get_selection()
  → Send FormatList to RDP client
```

**Windows → Linux:**
```
RDP client sends FormatList
  → We announce via Portal.set_selection()
  → User pastes
  → Portal emits SelectionTransfer signal
  → We request data from RDP via ServerEvent
  → Write data via Portal.write_selection()
```

### D-Bus Extension Path (GNOME Workaround)

**Works on:** GNOME only (requires extension installed)

**Linux → Windows:**
```
Linux clipboard change
  → Extension polls St.Clipboard (500ms)
  → Extension emits ClipboardChanged on D-Bus
  → We receive signal
  → Read data via Portal.get_selection()
  → Send FormatList to RDP client
```

**Windows → Linux:**
```
Same as Portal path (SelectionTransfer works on GNOME)
```

---

## CURRENT CLIPBOARD ISSUE

**On GNOME VM (192.168.10.205):**

**What's happening:**
1. ✅ Extension active and running
2. ✅ D-Bus bridge connected
3. ✅ Extension detects clipboard change
4. ✅ Server logs: "📋 D-Bus clipboard change #1: 4 MIME types"
5. ✅ Server logs: "Event loop should now call cliprdr.initiate_copy()"
6. ❌ **FormatList NOT sent to RDP client**

**Hypothesis:**
- Event is received
- cliprdr.initiate_copy() is NOT being called
- Or: It's being called but RDP channel isn't ready
- Or: Event is being suppressed/filtered

**Need to debug:** Clipboard event flow from D-Bus → IronRDP channel

---

## H.264/EGFX STATUS

### Implementation: ✅ COMPLETE (1,801 lines)

**Files:**
- `src/egfx/mod.rs` - EGFX channel protocol (MS-RDPEGFX spec)
- `src/egfx/encoder.rs` - H.264 encoder (OpenH264 integration)
- `src/egfx/video_handler.rs` - Encoding pipeline
- `src/egfx/surface.rs` - Surface management

**Features:**
- AVC420 codec (H.264 YUV420)
- Capability negotiation
- Surface creation and mapping
- Frame acknowledgment flow control
- Backpressure when client falls behind

**Codec Support:**
- OpenH264 (software encoding)
- VAAPI (hardware encoding) - optional
- Configurable bitrate, quality

**To Enable:**
```bash
# Build with H.264 support
cargo build --release --features h264

# System dependency required:
# OpenH264 will be compiled from source (BSD license)
```

**Why Not Default:**
- Adds OpenH264 dependency (~2MB compiled)
- RemoteFX works fine for desktop
- H.264 better for video/gaming scenarios

**When to Use:**
- Low bandwidth networks
- Video playback heavy
- Hardware encoding available (VAAPI)

---

## DEPENDENCIES STATUS

### Published Crates (Open Source) ✅

All using crates.io published versions:
- lamco-portal v0.2.0
- lamco-clipboard-core v0.2.0
- lamco-rdp-clipboard v0.2.0
- lamco-rdp-input v0.1.0

**EXCEPT:**
- lamco-pipewire - Using local path (FD ownership fix not published yet)
- lamco-video - Using local path (depends on lamco-pipewire)

### IronRDP (Git Dependencies) ⚠️

Using git patches from master branch:
```toml
[patch.crates-io]
ironrdp = { git = "https://github.com/Devolutions/IronRDP", branch = "master" }
# ... 9 more ironrdp crates
```

**Reason:** Waiting for PR #1057 (EGFX support) to be published

**Impact:** Can't publish to crates.io until IronRDP publishes or we remove git deps

---

## WHAT NEEDS WORK

### Priority 1: Fix Clipboard (This Session)

**Issue:** D-Bus events received but FormatList not sent to RDP

**Debug Steps:**
1. Add logging to see if cliprdr.initiate_copy() is called
2. Check if RDP clipboard channel is initialized
3. Verify event routing from ClipboardManager → IronRDP backend
4. Test both directions

### Priority 2: Test Input (Quick Test)

**Current:** Infrastructure complete, not manually tested

**Test:** Type in RDP session, click mouse, verify it works

### Priority 3: Test H.264/EGFX (Optional)

**Current:** Implemented but not enabled

**Test:**
1. Build with `--features h264`
2. Deploy to VM
3. Test encoding performance
4. Compare bandwidth vs RemoteFX

### Priority 4: Clean Up and Publish

**After everything works:**
1. Fix lamco-pipewire FD ownership in published crate
2. Publish lamco-pipewire v0.1.3
3. Remove local path dependencies
4. Wait for IronRDP to publish or accept using git deps
5. Create public repository
6. Publish to crates.io

---

## CLIPBOARD DETAILED ARCHITECTURE

### State Machine (src/clipboard/sync.rs)

```
Idle ─────────────────┐
  │                   │
  │ RDP FormatList    │ D-Bus ClipboardChanged
  │                   │
  ▼                   ▼
RdpToPortal      PortalToRdp
  │                   │
  │ Data received     │ Data sent
  │                   │
  └─────────────────→ Idle
```

### Components

**ClipboardManager** (src/clipboard/manager.rs - 850 lines)
- Event router
- State machine orchestration
- Loop detection
- Format conversion

**SyncManager** (src/clipboard/sync.rs)
- State transitions
- Echo protection
- Duplicate suppression

**RdpCliprdrBackend** (lamco-rdp-clipboard)
- IronRDP cliprdr channel integration
- Protocol encoding/decoding

**PortalClipboardManager** (lamco-portal)
- Portal Clipboard API
- SelectionTransfer signals
- SelectionOwnerChanged signals

**FormatConverter** (lamco-clipboard-core)
- MIME type ↔ RDP format mapping
- Text conversion (UTF-8 ↔ UTF-16)
- Image conversion (PNG, JPEG, BMP, DIB)

---

## IMMEDIATE NEXT STEPS

1. **Debug clipboard** - Add logging to see where events are being dropped
2. **Test input** - Quick manual test (should work)
3. **Decide on H.264** - Enable and test if desired
4. **Test on actual KDE VM** - If you have one (192.168.10.205 is GNOME)

---

## QUESTIONS FOR YOU

1. **Clipboard:** Want me to debug why FormatList isn't being sent?
2. **H.264:** Want to enable and test it now?
3. **KDE VM:** Do you have another VM that's actually KDE? (This one is GNOME)
4. **Input:** Should I test it or focus on clipboard first?

---

**Summary:**
- Video: ✅ WORKS (RemoteFX, great performance)
- Input: ⏳ Implemented, not tested
- Clipboard: ❌ Broken (events received but not propagated)
- H.264: 🟡 Implemented (1,801 lines), not enabled
- Code: ~30K lines total (~10K server, ~20K published crates)
