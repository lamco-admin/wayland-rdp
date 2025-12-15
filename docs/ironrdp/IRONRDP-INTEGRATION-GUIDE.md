# IRONRDP INTEGRATION GUIDE - AUTHORITATIVE
**Document:** IRONRDP-INTEGRATION-GUIDE.md
**Version:** 2.0
**Date:** 2025-01-18
**Based On:** Actual IronRDP v0.9.0 source code analysis

---

## CRITICAL DISCOVERY

After analyzing the **actual IronRDP server source code**, the integration approach is MUCH simpler than initially specified. IronRDP provides a complete RDP server framework that requires only implementing 2 traits:

1. **`RdpServerInputHandler`** - Receive input events from client
2. **`RdpServerDisplay`** - Provide display updates to send to client

---

## IRONRDP SERVER API (ACTUAL)

### Key Facts from Source Code Analysis

**Version:** 0.9.0 (latest on crates.io)
**Crate:** `ironrdp-server`
**Architecture:** Trait-based callback system
**Runtime:** Requires Tokio

### What IronRDP Server Provides OUT OF THE BOX

✅ **Complete RDP Protocol Handling**
- X.224 connection
- MCS (Multi-Channel Service)
- Capability negotiation
- Channel multiplexing
- FastPath input/output
- TLS security (TLS 1.2 and 1.3)
- NLA/CredSSP authentication support

✅ **Built-in Channel Support**
- Static virtual channels
- Dynamic virtual channels (DVC)
- Clipboard (CLIPRDR) - via `CliprdrServer`
- Sound (RDPSND) - via `RdpsndServer`
- Display Control - for multi-monitor
- Advanced Input (AINPUT)

✅ **Codecs**
- RDP 6.0 Bitmap Compression
- RemoteFX (RFX)
- QOI (optional feature)
- QOIZ (optional feature)

---

## HOW IRONRDP SERVER WORKS

### The Builder Pattern

```rust
use ironrdp_server::{RdpServer, RdpServerInputHandler, RdpServerDisplay};
use tokio_rustls::TlsAcceptor;

let server = RdpServer::builder()
    .with_addr(([0, 0, 0, 0], 3389))
    .with_tls(tls_acceptor)
    .with_input_handler(my_input_handler)
    .with_display_handler(my_display_handler)
    .build();

server.run().await?;
```

**That's it!** IronRDP handles all the protocol complexity.

---

## THE TWO REQUIRED TRAITS

### 1. RdpServerInputHandler Trait

```rust
pub trait RdpServerInputHandler: Send {
    fn keyboard(&mut self, event: KeyboardEvent);
    fn mouse(&mut self, event: MouseEvent);
}

// Keyboard events
pub enum KeyboardEvent {
    Pressed { code: u8, extended: bool },
    Released { code: u8, extended: bool },
    UnicodePressed(u16),
    UnicodeReleased(u16),
    Synchronize(SynchronizeFlags),
}

// Mouse events
pub enum MouseEvent {
    Move { x: u16, y: u16 },
    RightPressed,
    RightReleased,
    LeftPressed,
    LeftReleased,
    MiddlePressed,
    MiddleReleased,
    Button4Pressed,
    Button4Released,
    Button5Pressed,
    Button5Released,
    VerticalScroll { value: i16 },
    Scroll { x: i32, y: i32 },
    RelMove { x: i32, y: i32 },
}
```

**Your job:** Forward these events to the Portal's RemoteDesktop API for injection.

---

### 2. RdpServerDisplay Trait

```rust
#[async_trait]
pub trait RdpServerDisplay: Send {
    /// Return current desktop size
    async fn size(&mut self) -> DesktopSize;

    /// Return a display updates receiver
    async fn updates(&mut self) -> Result<Box<dyn RdpServerDisplayUpdates>>;

    /// Optional: handle layout requests from client
    fn request_layout(&mut self, layout: DisplayControlMonitorLayout) {
        // Default: ignore
    }
}

#[async_trait]
pub trait RdpServerDisplayUpdates {
    /// MUST be cancellation-safe (used in tokio::select!)
    async fn next_update(&mut self) -> Result<Option<DisplayUpdate>>;
}

// Display updates you send TO IronRDP
pub enum DisplayUpdate {
    Resize(DesktopSize),
    Bitmap(BitmapUpdate),
    PointerPosition(PointerPositionAttribute),
    ColorPointer(ColorPointer),
    RGBAPointer(RGBAPointer),
    HidePointer,
    DefaultPointer,
}

pub struct BitmapUpdate {
    pub x: u16,
    pub y: u16,
    pub width: NonZeroU16,
    pub height: NonZeroU16,
    pub format: PixelFormat,  // ABgr32, XRgb32, etc.
    pub data: Bytes,
    pub stride: NonZeroUsize,
}
```

**Your job:** Convert PipeWire frames to `BitmapUpdate` and send via channel.

---

## CRITICALLY IMPORTANT: NO H.264 SUPPORT YET

**MAJOR LIMITATION DISCOVERED:**

IronRDP server currently supports:
- ✅ RDP 6.0 Bitmap Compression
- ✅ RemoteFX codec
- ✅ QOI/QOIZ codecs
- ❌ **H.264 Graphics Pipeline Extension NOT YET IMPLEMENTED**

**This means:** We CANNOT use H.264 encoding as originally planned with current IronRDP server!

### Options:

**Option A:** Use RemoteFX instead of H.264
- RemoteFX is hardware-accelerated
- Supported by Windows clients
- IronRDP already has it
- Likely good enough performance

**Option B:** Use RDP 6.0 Bitmap Compression
- Basic compression
- Universally supported
- Lower performance
- Good for MVP

**Option C:** Wait for/contribute H.264 support to IronRDP
- Would need to implement Graphics Pipeline Extension
- Complex (weeks of work)
- Not viable for timeline

**RECOMMENDATION:** Use **RemoteFX** for Phase 1, add H.264 in future version.

---

## CORRECT DEPENDENCY LIST

Based on IronRDP Cargo.toml analysis:

```toml
[dependencies]
# IronRDP Server (v0.9.0)
ironrdp-server = { version = "0.9", features = ["helper"] }

# IronRDP will pull in these automatically:
# - ironrdp-pdu (PDU encoding/decoding)
# - ironrdp-acceptor (connection acceptance)
# - ironrdp-async (async utilities)
# - ironrdp-tokio (Tokio integration)
# - ironrdp-cliprdr (clipboard)
# - ironrdp-rdpsnd (sound)
# - ironrdp-displaycontrol (multi-monitor)
# - ironrdp-dvc/svc (virtual channels)

# Security (provide your own TLS acceptor)
tokio-rustls = "0.26"
rustls = "0.23"
```

---

## CORRECT ARCHITECTURE

### How Data Actually Flows

```
Portal (PipeWire frames)
    ↓
Your VideoProcessor
    ↓
Convert to BitmapUpdate
    ↓
Send via mpsc::channel
    ↓
RdpServerDisplayUpdates::next_update() ← IronRDP calls this
    ↓
IronRDP encodes with RemoteFX/RDP6
    ↓
IronRDP sends to RDP client
    ↓
Windows client displays


RDP Client (input events)
    ↓
IronRDP receives and parses
    ↓
RdpServerInputHandler::keyboard/mouse ← IronRDP calls this
    ↓
Your InputForwarder
    ↓
Portal RemoteDesktop API injection
    ↓
Compositor receives input
```

---

## CORRECT IMPLEMENTATION STRUCTURE

### Your Implementation Needs Only 3 Components:

#### 1. Input Forwarder (implements RdpServerInputHandler)
```rust
struct WaylandInputForwarder {
    remote_desktop: Arc<RemoteDesktopManager>,
}

impl RdpServerInputHandler for WaylandInputForwarder {
    fn keyboard(&mut self, event: KeyboardEvent) {
        // Forward to Portal RemoteDesktop API
        // Call notify_keyboard_keycode
    }

    fn mouse(&mut self, event: MouseEvent) {
        // Forward to Portal RemoteDesktop API
        // Call notify_pointer_motion, notify_pointer_button, etc.
    }
}
```

#### 2. Display Provider (implements RdpServerDisplay)
```rust
struct WaylandDisplayProvider {
    portal_session: PortalSessionHandle,
    update_rx: mpsc::Receiver<DisplayUpdate>,
    width: u16,
    height: u16,
}

#[async_trait]
impl RdpServerDisplay for WaylandDisplayProvider {
    async fn size(&mut self) -> DesktopSize {
        DesktopSize {
            width: self.width,
            height: self.height,
        }
    }

    async fn updates(&mut self) -> Result<Box<dyn RdpServerDisplayUpdates>> {
        Ok(Box::new(DisplayUpdatesReceiver {
            rx: self.update_rx,
        }))
    }
}

struct DisplayUpdatesReceiver {
    rx: mpsc::Receiver<DisplayUpdate>,
}

#[async_trait]
impl RdpServerDisplayUpdates for DisplayUpdatesReceiver {
    async fn next_update(&mut self) -> Result<Option<DisplayUpdate>> {
        Ok(self.rx.recv().await)
    }
}
```

#### 3. Video Processor (PipeWire → BitmapUpdate converter)
```rust
struct VideoProcessor {
    pipewire_stream: PipeWireStream,
    update_tx: mpsc::Sender<DisplayUpdate>,
}

impl VideoProcessor {
    async fn run(&mut self) {
        loop {
            // Receive frame from PipeWire
            let frame = self.pipewire_stream.receive_frame().await?;

            // Convert to BitmapUpdate (no encoding needed - IronRDP does it!)
            let bitmap = BitmapUpdate {
                x: 0,
                y: 0,
                width: NonZeroU16::new(frame.width).unwrap(),
                height: NonZeroU16::new(frame.height).unwrap(),
                format: PixelFormat::ABgr32,  // or XRgb32
                data: Bytes::from(frame.data),
                stride: NonZeroUsize::new(frame.width as usize * 4).unwrap(),
            };

            // Send to IronRDP
            self.update_tx.send(DisplayUpdate::Bitmap(bitmap)).await?;
        }
    }
}
```

---

## REVISED CARGO.TOML (CORRECT)

```toml
[dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }
tokio-util = "0.7.10"
futures = "0.3.30"
async-trait = "0.1.77"

# IronRDP Server - THIS IS ALL YOU NEED FOR RDP
ironrdp-server = { version = "0.9", features = ["helper"] }

# Portal integration
ashpd = { version = "0.12.0", features = ["tokio"] }
zbus = "4.0.1"

# PipeWire
pipewire = { version = "0.9.2", features = ["v0_3_77"] }
libspa = "0.9.2"

# TLS (for IronRDP)
tokio-rustls = "0.26"
rustls = "0.23.4"
rustls-pemfile = "2.1.0"

# Utilities
bytes = "1.5.0"
anyhow = "1.0.79"
thiserror = "1.0.56"
tracing = "0.1.40"
tracing-subscriber = { version = "0.3.18", features = ["env-filter", "json"] }

# Config and CLI
serde = { version = "1.0", features = ["derive"] }
toml = "0.8.10"
clap = { version = "4.5", features = ["derive", "env"] }

# NO NEED FOR:
# - Video encoding crates (IronRDP does encoding!)
# - OpenH264, VA-API (IronRDP handles it!)
# - Complex RDP PDU handling (IronRDP provides it!)
```

---

## CORRECT TASK SEQUENCE (REVISED)

### Phase 1 Tasks - CORRECTED ORDER

**COMPLETED:**
- ✅ P1-01: Foundation
- ✅ P1-02: Security

**NEXT TASKS:**

**P1-03: Portal Integration** (5-7 days)
- Implement PortalManager
- Get PipeWire FD and streams
- Setup input injection via RemoteDesktop portal
- **Output:** Working portal integration, can get frames and inject input

**P1-04: PipeWire Integration** (5-7 days)
- Connect to PipeWire using FD from portal
- Receive frames
- Convert formats
- **Output:** VideoFrame structs

**P1-05: IronRDP Server Integration** (7-10 days) ← THE BIG ONE
- Implement RdpServerInputHandler (forward to portal)
- Implement RdpServerDisplay (provide bitmap updates)
- Convert PipeWire frames → BitmapUpdate
- Wire everything together
- **Output:** WORKING END-TO-END RDP SERVER!

**P1-06: Clipboard** (3-5 days)
- Implement CliprdrServerFactory
- Wire to Portal clipboard

**P1-07: Multi-Monitor** (3-5 days)
- Handle multiple PipeWire streams
- DisplayControl for layout

**P1-08: Testing & Polish** (7-10 days)
- Integration tests
- Performance optimization
- Bug fixes

**Total:** ~8-9 weeks (faster than original 12 weeks!)

---

## WHAT YOU DON'T NEED TO BUILD

❌ RDP protocol state machines (IronRDP has it)
❌ PDU encoding/decoding (IronRDP has it)
❌ Channel management (IronRDP has it)
❌ Capability negotiation (IronRDP has it)
❌ Video encoding (IronRDP does it internally with RemoteFX!)
❌ Graphics pipeline (IronRDP handles it)
❌ Input PDU parsing (IronRDP has it)

**You only need:** Glue code between Portal/PipeWire and IronRDP's traits!

---

## EXAMPLE: MINIMAL WORKING RDP SERVER

```rust
use ironrdp_server::*;
use tokio_rustls::TlsAcceptor;

struct MyInputHandler {
    portal: Arc<RemoteDesktopManager>,
}

impl RdpServerInputHandler for MyInputHandler {
    fn keyboard(&mut self, event: KeyboardEvent) {
        // Forward to portal
        match event {
            KeyboardEvent::Pressed { code, .. } => {
                tokio::spawn(async move {
                    portal.notify_keyboard_keycode(code as i32, true).await;
                });
            }
            // ... handle other events
        }
    }

    fn mouse(&mut self, event: MouseEvent) {
        // Forward to portal
    }
}

struct MyDisplay {
    update_rx: mpsc::Receiver<DisplayUpdate>,
}

#[async_trait]
impl RdpServerDisplay for MyDisplay {
    async fn size(&mut self) -> DesktopSize {
        DesktopSize { width: 1920, height: 1080 }
    }

    async fn updates(&mut self) -> Result<Box<dyn RdpServerDisplayUpdates>> {
        Ok(Box::new(MyUpdates { rx: self.update_rx }))
    }
}

struct MyUpdates {
    rx: mpsc::Receiver<DisplayUpdate>,
}

#[async_trait]
impl RdpServerDisplayUpdates for MyUpdates {
    async fn next_update(&mut self) -> Result<Option<DisplayUpdate>> {
        Ok(self.rx.recv().await)
    }
}

#[tokio::main]
async fn main() {
    let tls_acceptor = create_tls_acceptor();
    let input_handler = MyInputHandler { ... };
    let display = MyDisplay { ... };

    let server = RdpServer::builder()
        .with_addr(([0, 0, 0, 0], 3389))
        .with_tls(tls_acceptor)
        .with_input_handler(input_handler)
        .with_display_handler(display)
        .build();

    server.run().await.unwrap();
}
```

That's **~100 lines** for a working RDP server!

---

## CRITICAL INSIGHTS

### 1. No Custom Encoding Needed
IronRDP's `UpdateEncoder` handles all encoding internally:
- You give it `BitmapUpdate` with raw BGRA/XRGB data
- It compresses with RemoteFX or RDP 6.0
- It fragments and sends to client

### 2. No PDU Handling Needed
IronRDP's `Acceptor` handles entire connection sequence:
- X.224 negotiation
- MCS connection
- Capability exchange
- Channel setup

### 3. No Channel Management Needed
IronRDP's `StaticChannelSet` manages channels:
- Automatically handles clipboard
- Automatically handles sound
- Automatically handles display control

---

## CODEC COMPARISON

Since H.264 isn't available, here's what we have:

| Codec | Performance | Compatibility | IronRDP Support |
|-------|-------------|---------------|-----------------|
| H.264 AVC | Excellent | Windows 10+ | ❌ Not implemented |
| RemoteFX | Very Good | Windows 7+ | ✅ Full support |
| RDP 6.0 | Good | All Windows | ✅ Full support |
| QOI | Good | Custom | ✅ Optional feature |

**Recommendation:** Use **RemoteFX** - nearly as good as H.264, fully supported.

---

## UPDATED TECHNOLOGY STACK

### What Changes

**REMOVE:**
- ~~openh264~~ (not needed!)
- ~~va-api bindings~~ (not needed!)
- ~~H.264 encoder~~ (not available in IronRDP server yet)
- ~~Complex RDP crates~~ (ironrdp-server bundles everything)

**KEEP:**
- ironrdp-server (THE key dependency)
- ashpd (portal integration)
- pipewire (video source)
- tokio-rustls (TLS for IronRDP)

**ADD:**
- Nothing! We're good.

---

## REVISED SYSTEM ARCHITECTURE

```
Windows RDP Client
        ↓
   (RDP Protocol)
        ↓
┌────────────────────┐
│  IronRDP Server    │ ← Handles ALL protocol complexity
│  .run() method     │
└──┬─────────────┬───┘
   │             │
   │ (traits)    │ (traits)
   ↓             ↓
┌──────────┐ ┌──────────┐
│  Input   │ │ Display  │ ← Your implementations
│ Handler  │ │ Provider │
└───┬──────┘ └────┬─────┘
    │             │
    ↓             ↓
Portal API    PipeWire
    ↓             ↓
Compositor   Screen Content
```

**That's it!** Much simpler than originally specified.

---

## NEXT STEPS - CORRECTED

### Task P1-03 (NEXT): Portal Integration
Implement portal access - this is still needed and correct.

### Task P1-04 (THEN): PipeWire Integration
Receive frames from PipeWire.

### Task P1-05 (THEN): IronRDP Integration
**This is where it all comes together:**
1. Implement `RdpServerInputHandler` trait (forward to portal)
2. Implement `RdpServerDisplay` trait (provide PipeWire frames as BitmapUpdate)
3. Create IronRDP server with builder
4. **Done!** You have a working RDP server.

---

## IMPLICATIONS FOR SPECIFICATIONS

### Specifications That Are CORRECT
- ✅ Portal integration (still needed)
- ✅ PipeWire integration (still needed)
- ✅ Security/TLS setup (needed for IronRDP)
- ✅ Configuration system (still needed)
- ✅ Input handling concept (implementation simpler)

### Specifications That Need REVISION
- ❌ Video encoding tasks (don't need them!)
- ❌ Graphics channel tasks (IronRDP does it!)
- ❌ RDP protocol tasks (IronRDP does it!)
- ❌ H.264 codec tasks (not available!)

### Specifications That Are OUTDATED
- All references to H.264 encoding
- All references to VA-API
- All references to OpenH264
- All references to implementing RDP protocol
- All references to channel management

---

## CONCLUSION

**IronRDP is MORE capable than initially researched.**

It provides a **complete RDP server framework** that handles 90% of the complexity. We only need to:

1. Get video frames from Portal/PipeWire
2. Convert to BitmapUpdate format
3. Implement 2 simple traits
4. Call the builder

This is **MUCH SIMPLER** than the original specification and will result in a **working RDP server in ~8 weeks** instead of 12-18 weeks.

---

**Status:** READY TO CREATE CORRECTED SPECIFICATIONS

**Next:** Generate corrected task specifications based on this real understanding.
