# CLAUDE CODE WEB - CONTINUE DEVELOPMENT SESSION
**Repository:** https://github.com/lamco-admin/wayland-rdp
**Current Status:** Foundation, Security, and Portal modules complete
**Next Task:** PipeWire Integration
**Estimated Duration:** 3-5 days

---

## 📋 PROJECT CONTEXT

You're continuing development of **wrd-server** (Wayland Remote Desktop Server), a Rust-based RDP server that allows Windows clients to remotely control Wayland Linux desktops.

**Key Architecture Decision (IMPORTANT):**
We're using IronRDP library which provides complete RDP protocol handling. We only need to:
1. Get video frames from Wayland compositor (via Portal + PipeWire)
2. Get input events from RDP client (IronRDP gives us callbacks)
3. Implement 2 traits to connect them

**Read:** The IRONRDP-INTEGRATION-GUIDE.md in the repo explains this simplified approach.

---

## ✅ WHAT'S ALREADY IMPLEMENTED

### Completed Modules
1. **Configuration** (src/config/)
   - Config loading from TOML
   - CLI argument parsing
   - Validation

2. **Security** (src/security/)
   - TLS 1.3 configuration
   - Certificate management
   - PAM authentication
   - Self-signed cert generation

3. **Portal Integration** (src/portal/)
   - PortalManager using ashpd
   - ScreenCast portal (screen capture)
   - RemoteDesktop portal (input injection)
   - Clipboard portal
   - Session management with PipeWire FD

### Available APIs
```rust
// You can use these:
use wrd_server::portal::{PortalManager, PortalSessionHandle};
use wrd_server::security::SecurityManager;
use wrd_server::config::Config;

// Portal provides:
let portal = PortalManager::new(&config).await?;
let session = portal.create_session().await?; // Triggers permission dialog
let fd = session.pipewire_fd(); // Raw file descriptor for PipeWire
let streams = session.streams(); // Stream metadata (resolution, position, etc.)
```

---

## 🎯 NEXT TASK: PIPEWIRE INTEGRATION

### Objective
Connect to PipeWire using the file descriptor provided by the Portal, receive video frames, and make them available for encoding/transmission.

### What to Implement

**Module:** `src/pipewire/`

**Files to create:**
1. `src/pipewire/mod.rs` - Module coordinator
2. `src/pipewire/stream.rs` - PipeWire stream connection
3. `src/pipewire/receiver.rs` - Frame reception
4. `src/pipewire/format.rs` - Format negotiation

### Key Requirements

**1. Connect to PipeWire**
```rust
// Use FD from portal:
let fd = portal_session.pipewire_fd();

// Connect PipeWire using this FD
// Use pipewire crate (already in Cargo.toml)
```

**2. Receive Frames**
```rust
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,  // BGRA or XRGB format
    pub format: String,  // "BGRx", "RGBx", etc.
    pub pts: u64,  // Presentation timestamp
}
```

**3. Handle Multiple Streams (Multi-Monitor)**
- Portal may provide multiple streams
- One per monitor
- Handle all of them

### Dependencies Available
Already in Cargo.toml:
```toml
pipewire = { version = "0.9.2", features = ["v0_3_77"] }
libspa = "0.9.2"
```

### Integration Points

**Input:** PortalSessionHandle from portal module
```rust
let session = portal_manager.create_session().await?;
let fd = session.pipewire_fd();
let streams = session.streams(); // Get metadata
```

**Output:** VideoFrame structs
```rust
// Send frames via channel to next module:
frame_tx.send(VideoFrame { ... }).await?;
```

---

## 📖 REFERENCE DOCUMENTATION

### IronRDP Simplified Approach
We're NOT implementing video encoding ourselves. IronRDP handles that.

**What you need:** Raw bitmap data in BGRA/XRGB format
**IronRDP wants:** `BitmapUpdate` struct with raw pixels
**Your job:** Convert PipeWire frames to the format IronRDP expects

### PipeWire Resources
- Official docs: https://docs.pipewire.org/
- pipewire-rs docs: https://docs.rs/pipewire/
- Tutorial: https://docs.pipewire.org/page_tutorial5.html

### Example Pattern
```rust
// PipeWire frame reception pattern:
let stream = pw::stream::Stream::new(&core, "video-src", props)?;
stream.add_listener()
    .process(|stream, buf| {
        // Extract frame data
        // Create VideoFrame
        // Send to channel
    })
    .register()?;
stream.connect(...)?;
```

---

## ✅ SUCCESS CRITERIA

Task complete when:
- [ ] PipeWire connects using FD from portal
- [ ] Frames received at ~30 FPS
- [ ] Frame data accessible (width, height, pixels)
- [ ] Multiple streams handled (multi-monitor)
- [ ] Frames sent to channel/callback for next module
- [ ] cargo build succeeds
- [ ] Example program demonstrates frame reception
- [ ] Integration test verifies flow

---

## 🧪 TESTING

### Integration Test
Create `tests/integration/pipewire_test.rs`:
- Create portal session
- Get PipeWire FD
- Connect PipeWire
- Receive at least one frame
- Verify frame data

**Mark as #[ignore]** (requires Wayland)

### Example Program
Create `examples/pipewire_frames.rs`:
- Shows how to receive frames
- Prints frame metadata
- Optionally saves first frame to file

---

## 🔍 IMPORTANT NOTES

### About PipeWire API
The `pipewire` crate API may have quirks:
- Uses callback-based event model
- Requires main loop integration
- May need unsafe blocks for FFI

**If you encounter issues:**
- Check pipewire-rs examples in the crate repo
- Use async-friendly patterns with tokio
- Document any unsafe blocks with SAFETY comments

### About Frame Format
PipeWire typically provides:
- Format: "BGRx", "RGBx", "BGRA", "RGBA"
- Size: Matches monitor resolution
- Stride: May differ from width * bytes_per_pixel
- DMA-BUF: Possible for zero-copy (advanced, optional)

**For MVP:** Accept any format, convert to BGRA if needed

---

## 📁 FILE STRUCTURE TO CREATE

```
src/pipewire/
├── mod.rs          # Main module, PipeWire connection coordinator
├── stream.rs       # Stream management
├── receiver.rs     # Frame receiver implementation
└── format.rs       # Format negotiation and conversion

examples/
└── pipewire_frames.rs  # Demonstration program

tests/integration/
└── pipewire_test.rs    # Integration test
```

---

## 🎯 SUGGESTED IMPLEMENTATION APPROACH

### Step 1: Basic Connection
- Create PipeWire context using FD
- Connect to PipeWire daemon
- Log successful connection

### Step 2: Stream Setup
- Use stream metadata from portal
- Create PipeWire stream
- Setup event listeners

### Step 3: Frame Reception
- Implement frame callback
- Extract frame data from buffer
- Create VideoFrame struct

### Step 4: Multi-Stream
- Handle multiple streams (multi-monitor)
- Track which stream is which monitor

### Step 5: Testing
- Integration test
- Example program
- Verify frames flow

---

## 💡 TIPS FOR SUCCESS

1. **Check pipewire-rs examples first** - The crate has examples
2. **Use tokio integration** - PipeWire needs to work with async
3. **Handle errors gracefully** - PipeWire connection can fail
4. **Log everything** - Use tracing crate for debug info
5. **Test incrementally** - Get connection working first, then frames

---

## 🚀 START IMPLEMENTATION

**Your task:**
Implement PipeWire integration in src/pipewire/ module following the structure above.

**Key deliverables:**
- PipeWire connection using portal FD
- Frame reception working
- VideoFrame structs created
- Multi-stream support
- Tests and examples

**Reference:**
- Existing portal code in src/portal/
- IronRDP guide in IRONRDP-INTEGRATION-GUIDE.md
- PipeWire docs at docs.pipewire.org

**Estimated completion:** 3-5 days

Begin implementation!
