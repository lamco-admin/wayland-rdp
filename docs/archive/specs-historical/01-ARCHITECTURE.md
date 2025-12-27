# SYSTEM ARCHITECTURE SPECIFICATION
**Document:** 01-ARCHITECTURE.md
**Version:** 1.0
**Date:** 2025-01-18
**Parent:** 00-MASTER-SPECIFICATION.md

---

## DOCUMENT PURPOSE

This document provides the complete system architecture for the Wayland Remote Desktop Server. It defines all components, their interactions, data flows, threading models, and architectural patterns that MUST be followed in implementation.

---

## TABLE OF CONTENTS

1. [High-Level Architecture](#high-level-architecture)
2. [Component Architecture](#component-architecture)
3. [Data Flow Architecture](#data-flow-architecture)
4. [Threading and Concurrency Model](#threading-and-concurrency-model)
5. [Protocol Stack](#protocol-stack)
6. [State Machines](#state-machines)
7. [Error Handling Architecture](#error-handling-architecture)
8. [Security Architecture](#security-architecture)

---

## HIGH-LEVEL ARCHITECTURE

### System Context Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    WINDOWS RDP CLIENT                        │
│                     (mstsc.exe)                              │
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  Video   │  │  Input   │  │Clipboard │  │  Audio   │   │
│  │ Display  │  │  KB/MS   │  │          │  │ (Phase2) │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└───────┬──────────────┬────────────┬──────────────┬─────────┘
        │              │            │              │
        │              │            │              │
     Bitmap         Input      Clipboard        Audio
     Updates        Events      Data            Stream
        │              │            │              │
        │              │            │              │
        └──────────────┴────────────┴──────────────┘
                       │
                  TCP Port 3389
                  TLS 1.3 Encrypted
                  RDP 10.x Protocol
                       │
┌──────────────────────▼────────────────────────────────────────┐
│                  WRD-SERVER (Rust Binary)                     │
│                                                               │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              IRONRDP SERVER LAYER                        │ │
│  │  • TLS Termination & NLA Authentication                 │ │
│  │  • Protocol State Machine                                │ │
│  │  • Capability Negotiation                                │ │
│  │  • Channel Management (Graphics, Input, Clipboard)       │ │
│  │  • Bitmap Encoding & Compression                         │ │
│  └────────┬──────────────────────────────────┬──────────────┘ │
│           │                                  │                │
│  ┌────────▼──────────────────────────────────▼──────────────┐ │
│  │            TRAIT IMPLEMENTATION LAYER                    │ │
│  │  • DisplayUpdateHandler (frame processing)               │ │
│  │  • InputHandler (keyboard/mouse events)                  │ │
│  │  • ClipboardHandler (cliprdr protocol)                   │ │
│  │  • SessionHandler (connection lifecycle)                 │ │
│  └─┬──────────┬─────────────┬──────────┬────────────────────┘ │
│    │          │             │          │                     │
│ ┌──▼────────┐ ┌▼──────────┐ ┌─▼──────────┐ ┌──────────────┐ │
│ │  Bitmap   │ │  Input    │ │ Clipboard  │ │Multi-Monitor │ │
│ │ Converter │ │ Forwarder │ │  Manager   │ │  Manager     │ │
│ └──┬────────┘ └──┬────────┘ └──────┬─────┘ └──────┬───────┘ │
│    │            │                  │               │         │
│ ┌──▼──────────┐ ┌▼──────────┐ ┌───▼──────┐ ┌─────▼────────┐ │
│ │ PipeWire    │ │Portal     │ │Clipboard │ │ Monitor      │ │
│ │ Receiver    │ │Interface  │ │ Portal   │ │ Discovery    │ │
│ └──┬──────────┘ └──┬────────┘ └────┬─────┘ └──────────────┘ │
│    │               │               │                        │
└────┼───────────────┼───────────────┼────────────────────────┘
     │               │               │
┌────▼───────────────▼───────────────▼─────────────────────────┐
│              XDG-DESKTOP-PORTAL (D-Bus)                       │
│                                                               │
│  ┌────────────────┐  ┌──────────────────┐  ┌──────────────┐ │
│  │  ScreenCast    │  │  RemoteDesktop   │  │  Clipboard   │ │
│  │  Portal        │  │  Portal          │  │  Portal      │ │
│  └────────┬───────┘  └────────┬─────────┘  └──────┬───────┘ │
└───────────┼──────────────────┼────────────────────┼─────────┘
            │                  │                    │
       ┌────▼────┐        ┌────▼─────┐        ┌────▼────┐
       │PipeWire │        │libei/eis │        │Clipboard│
       │(Video)  │        │ (Input)  │        │ Access  │
       └────┬────┘        └────┬─────┘        └────┬────┘
            │                  │                   │
┌───────────▼──────────────────▼───────────────────▼───────────┐
│               WAYLAND COMPOSITOR                              │
│         (GNOME / KDE Plasma / Sway)                           │
│                                                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │ Monitor 1│  │ Monitor 2│  │  Input   │  │Clipboard │    │
│  │ Output   │  │ Output   │  │  Stack   │  │ Manager  │    │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘    │
└───────────────────────────────────────────────────────────────┘
```

### Key Architectural Layers

1. **Network Layer:** TLS-encrypted TCP connections
2. **IronRDP Server Layer:** Complete RDP protocol implementation
3. **Trait Implementation Layer:** Bridge between IronRDP and system
4. **Portal Integration Layer:** Access to compositor resources
5. **System Layer:** Wayland compositor, PipeWire, libei

---

## COMPONENT ARCHITECTURE

### Core Components

#### 1. IronRDP Server Component
**Location:** `src/server/`

**Responsibilities:**
- Accept TCP connections on port 3389
- Manage IronRDP server lifecycle
- Handle protocol state machine
- Coordinate trait implementations

**Sub-components:**
- **IronRdpServer:** Main RDP server (from ironrdp-server crate)
- **SessionManager:** Tracks active RDP sessions
- **CapabilityManager:** Negotiate client capabilities

**Interfaces:**
```rust
pub struct WrdServer {
    iron_server: ironrdp_server::Server,
    display_handler: Arc<DisplayUpdateHandler>,
    input_handler: Arc<InputHandler>,
    clipboard_handler: Arc<ClipboardHandler>,
    session_handler: Arc<SessionHandler>,
}

impl WrdServer {
    pub async fn new(config: Config) -> Result<Self>;
    pub async fn run(self) -> Result<()>;
    pub async fn handle_connection(stream: TcpStream) -> Result<()>;
}
```

#### 2. Display Update Handler
**Location:** `src/display/`

**Responsibilities:**
- Implement IronRDP's DisplayUpdateHandler trait
- Convert PipeWire frames to RDP bitmaps
- Manage surface updates and damage tracking
- Handle cursor updates

**Sub-components:**
- **BitmapConverter:** Convert BGRA/RGB to RDP bitmap format
- **DamageTracker:** Track changed regions
- **CursorManager:** Extract and send cursor metadata

**Interfaces:**
```rust
pub struct DisplayUpdateHandler {
    pipewire_receiver: Arc<PipeWireReceiver>,
    bitmap_converter: BitmapConverter,
    damage_tracker: DamageTracker,
    cursor_manager: CursorManager,
}

#[async_trait]
impl ironrdp_server::DisplayUpdateHandler for DisplayUpdateHandler {
    async fn handle_update(&mut self, surface_id: u32) -> Result<UpdatePdu>;
    async fn get_cursor_update(&mut self) -> Result<Option<CursorPdu>>;
}
```

#### 3. Input Handler Component
**Location:** `src/input/`

**Responsibilities:**
- Implement IronRDP's InputHandler trait
- Forward input events to portal
- Translate RDP input to Wayland events
- Handle keyboard and mouse events

**Sub-components:**
- **InputForwarder:** Send events via RemoteDesktop portal
- **KeyboardTranslator:** RDP scancode to keysym conversion
- **PointerTranslator:** Mouse coordinate transformation

**Interfaces:**
```rust
pub struct InputHandler {
    portal: Arc<RemoteDesktopPortal>,
    keyboard_translator: KeyboardTranslator,
    pointer_translator: PointerTranslator,
}

#[async_trait]
impl ironrdp_server::InputHandler for InputHandler {
    async fn handle_keyboard(&mut self, event: KeyboardEvent) -> Result<()>;
    async fn handle_mouse(&mut self, event: MouseEvent) -> Result<()>;
    async fn handle_sync(&mut self, flags: SyncFlags) -> Result<()>;
}
```

#### 4. Clipboard Handler Component
**Location:** `src/clipboard/`

**Responsibilities:**
- Implement IronRDP's ClipboardHandler trait
- Handle CLIPRDR channel protocol
- Bidirectional clipboard synchronization
- Format conversion between RDP and MIME types

**Sub-components:**
- **ClipboardManager:** Manage clipboard state
- **FormatMapper:** Map RDP formats to MIME types
- **ClipboardPortal:** Interface with clipboard portal

**Interfaces:**
```rust
pub struct ClipboardHandler {
    portal: Arc<ClipboardPortal>,
    format_mapper: FormatMapper,
    state: Arc<RwLock<ClipboardState>>,
}

#[async_trait]
impl ironrdp_server::ClipboardHandler for ClipboardHandler {
    async fn handle_format_list(&mut self, formats: Vec<ClipboardFormat>) -> Result<()>;
    async fn handle_data_request(&mut self, format_id: u32) -> Result<Vec<u8>>;
    async fn handle_data_response(&mut self, data: Vec<u8>) -> Result<()>;
}
```

#### 5. Portal Integration Component
**Location:** `src/portal/`

**Responsibilities:**
- D-Bus connection management
- ScreenCast portal for video capture
- RemoteDesktop portal for input injection
- Clipboard portal for clipboard access
- Session lifecycle management

**Sub-components:**
- **ScreenCastPortal:** Video stream acquisition
- **RemoteDesktopPortal:** Input injection + video
- **ClipboardPortal:** Clipboard data access
- **SessionManager:** Portal session lifecycle

**Interfaces:**
```rust
pub struct PortalManager {
    connection: zbus::Connection,
    screencast: Arc<ScreenCastPortal>,
    remote_desktop: Arc<RemoteDesktopPortal>,
    clipboard: Arc<ClipboardPortal>,
}

impl PortalManager {
    pub async fn new(config: &Config) -> Result<Self>;
    pub async fn create_session(&self) -> Result<PortalSessionHandle>;
}

pub struct PortalSessionHandle {
    session_id: String,
    pipewire_fd: RawFd,
    streams: Vec<StreamInfo>,
}
```

#### 6. PipeWire Receiver Component
**Location:** `src/pipewire/`

**Responsibilities:**
- PipeWire connection using FD from portal
- Stream negotiation (BGRA/RGB format)
- Frame reception and buffering
- DMA-BUF handling for zero-copy

**Sub-components:**
- **StreamReceiver:** PipeWire stream management
- **FrameBuffer:** Frame queue management
- **FormatNegotiator:** Video format selection

**Interfaces:**
```rust
pub struct PipeWireReceiver {
    fd: RawFd,
    stream_id: u32,
    format: VideoFormat,
    frame_queue: Arc<Mutex<VecDeque<VideoFrame>>>,
}

impl PipeWireReceiver {
    pub async fn new(fd: RawFd, stream_info: &StreamInfo) -> Result<Self>;
    pub async fn start(&mut self) -> Result<()>;
    pub async fn get_frame(&mut self) -> Result<VideoFrame>;
}
```

#### 7. Bitmap Converter Component
**Location:** `src/bitmap/`

**Responsibilities:**
- Convert PipeWire frames to RDP bitmap format
- Handle pixel format conversion (BGRA → RGB)
- Apply compression (RLE, JPEG, etc.)
- Manage bitmap cache

**Sub-components:**
- **PixelConverter:** BGRA/RGB format conversion
- **Compressor:** RDP bitmap compression
- **BitmapCache:** Cache frequently used bitmaps

**Interfaces:**
```rust
pub struct BitmapConverter {
    pixel_converter: PixelConverter,
    compressor: Compressor,
    cache: BitmapCache,
}

impl BitmapConverter {
    pub fn convert_frame(&mut self, frame: &VideoFrame) -> Result<RdpBitmap>;
    pub fn apply_compression(&mut self, bitmap: &mut RdpBitmap) -> Result<()>;
    pub fn cache_bitmap(&mut self, id: u32, bitmap: RdpBitmap) -> Result<()>;
}
```

#### 8. Multi-Monitor Component
**Location:** `src/multimon/`

**Responsibilities:**
- Monitor discovery via ScreenCast
- Layout calculation
- Stream coordination
- Per-monitor pipeline management

**Sub-components:**
- **MonitorManager:** Monitor enumeration and tracking
- **LayoutCalculator:** Virtual desktop layout computation

**Interfaces:**
```rust
pub struct MultiMonitorManager {
    monitors: Vec<MonitorInfo>,
    streams: Vec<PipeWireStream>,
    pipelines: Vec<VideoPipeline>,
}

impl MultiMonitorManager {
    pub async fn new(portal: &PortalManager) -> Result<Self>;
    pub async fn run(&mut self) -> Result<()>;
}
```

---

## DATA FLOW ARCHITECTURE

### Video Stream Data Flow

```
┌─────────────────┐
│   Compositor    │
│  Framebuffer    │
└────────┬────────┘
         │
         │ (Compositor renders)
         ▼
┌─────────────────┐
│   PipeWire      │
│   (Screen       │
│    Capture)     │
└────────┬────────┘
         │
         │ (Portal API)
         │ (DMA-BUF or SHM)
         ▼
┌─────────────────┐
│ PipeWire        │
│ Receiver        │
└────────┬────────┘
         │
         │ VideoFrame
         │ (BGRA/RGB)
         ▼
┌─────────────────┐
│Display Update   │
│Handler (Trait)  │
└────────┬────────┘
         │
         │ Frame Processing
         ▼
┌─────────────────┐
│ Damage Tracker  │
│ (Detect changes)│
└────────┬────────┘
         │
         │ Changed Regions
         ▼
┌─────────────────┐
│Bitmap Converter │
│  (BGRA→RGB)     │
└────────┬────────┘
         │
         │ RDP Bitmap
         ▼
┌─────────────────┐
│ Cursor Manager  │
│ (Extract cursor)│
└────────┬────────┘
         │
         │ Bitmap + Cursor
         ▼
┌─────────────────┐
│ IronRDP Server  │
│ (Encode & Send) │
└────────┬────────┘
         │
         │ RDP PDUs
         │ (Compressed bitmaps)
         ▼
┌─────────────────┐
│  TLS Stream     │
└────────┬────────┘
         │
         │ Encrypted TCP
         ▼
┌─────────────────┐
│   RDP Client    │
│     Display     │
└─────────────────┘
```

### Input Event Data Flow

```
┌─────────────────┐
│   RDP Client    │
│   (User Input)  │
└────────┬────────┘
         │
         │ Input Event
         ▼
┌─────────────────┐
│  TLS Stream     │
└────────┬────────┘
         │
         │ RDP PDU
         ▼
┌─────────────────┐
│ IronRDP Server  │
│ (Decode PDU)    │
└────────┬────────┘
         │
         │ Input Event
         ▼
┌─────────────────┐
│  Input Handler  │
│    (Trait)      │
└────────┬────────┘
         │
         │ Keyboard/Mouse
         ▼
┌─────────────────┐
│Input Forwarder  │
│ (Translate)     │
└────────┬────────┘
         │
         │ Portal Events
         ▼
┌─────────────────┐
│RemoteDesktop    │
│    Portal       │
└────────┬────────┘
         │
         │ (libei/D-Bus)
         ▼
┌─────────────────┐
│   Compositor    │
│  Input Stack    │
└────────┬────────┘
         │
         │ Wayland Events
         ▼
┌─────────────────┐
│ Wayland Clients │
│  (Applications) │
└─────────────────┘
```

### Clipboard Sync Data Flow

```
CLIENT→SERVER (Copy from client)

┌─────────────────┐
│   RDP Client    │
│   (Ctrl+C)      │
└────────┬────────┘
         │
         │ CLIPRDR Format List
         ▼
┌─────────────────┐
│ IronRDP Server  │
│ (CLIPRDR Proto) │
└────────┬────────┘
         │
         │ Format List Event
         ▼
┌─────────────────┐
│Clipboard Handler│
│    (Trait)      │
└────────┬────────┘
         │
         │ Format Conversion
         │ (CF_UNICODETEXT → text/plain)
         ▼
┌─────────────────┐
│  Clipboard      │
│    Portal       │
└────────┬────────┘
         │
         │ D-Bus call
         ▼
┌─────────────────┐
│   Compositor    │
│   Clipboard     │
└─────────────────┘


SERVER→CLIENT (Copy from server)

┌─────────────────┐
│ Wayland Client  │
│   (Ctrl+C)      │
└────────┬────────┘
         │
         │ Wayland clipboard event
         ▼
┌─────────────────┐
│   Compositor    │
│   Clipboard     │
└────────┬────────┘
         │
         │ (Portal monitoring)
         ▼
┌─────────────────┐
│  Clipboard      │
│    Portal       │
└────────┬────────┘
         │
         │ Change notification
         ▼
┌─────────────────┐
│Clipboard Handler│
│    (Trait)      │
└────────┬────────┘
         │
         │ Format Conversion
         │ (text/plain → CF_UNICODETEXT)
         ▼
┌─────────────────┐
│ IronRDP Server  │
│ (CLIPRDR Proto) │
└────────┬────────┘
         │
         │ Format List PDU
         ▼
┌─────────────────┐
│   RDP Client    │
│   (Ctrl+V)      │
└─────────────────┘
```

---

## THREADING AND CONCURRENCY MODEL

### Thread Architecture

```
┌──────────────────────────────────────────────────────────┐
│              IRONRDP SERVER RUNTIME                      │
│          (Manages its own threading)                     │
└────┬──────────────┬──────────────┬──────────────────────┘
     │              │              │
     │              │              │
┌────▼─────┐  ┌────▼─────┐  ┌────▼─────┐
│ IronRDP  │  │ IronRDP  │  │ IronRDP  │
│ Worker 1 │  │ Worker 2 │  │ Worker 3 │
└──────────┘  └──────────┘  └──────────┘
     │              │              │
     │   Handle RDP Protocol       │
     │   TLS operations            │
     │   Channel multiplexing      │
     │   Bitmap encoding           │
     └──────────────┴──────────────┘
                    │
                    │ Trait callbacks
                    ▼
┌──────────────────────────────────────────────────────────┐
│              TOKIO ASYNC RUNTIME                         │
│          (For portal and PipeWire)                       │
└────┬──────────────┬──────────────┬──────────────────────┘
     │              │              │
┌────▼─────┐  ┌────▼─────┐  ┌────▼─────┐
│ Portal   │  │ PipeWire │  │ Clipboard│
│ Worker   │  │ Worker   │  │  Worker  │
└──────────┘  └──────────┘  └──────────┘
     │              │              │
     │   D-Bus operations          │
     │   Frame reception           │
     │   Clipboard sync            │
     └──────────────┴──────────────┘
```

### Concurrency Primitives

#### Async Channels (tokio::sync::mpsc)
**Use for:** Inter-task communication within async context
```rust
// Video frames from PipeWire to display handler
let (frame_tx, frame_rx) = mpsc::channel::<VideoFrame>(16);

// Clipboard events between handler and portal
let (clipboard_tx, clipboard_rx) = mpsc::channel::<ClipboardEvent>(32);

// Portal events to IronRDP trait implementations
let (portal_tx, portal_rx) = mpsc::channel::<PortalEvent>(64);
```

#### Sync Channels (crossbeam_channel)
**Use for:** Communication between IronRDP threads and async tasks
```rust
// Frames from async PipeWire to IronRDP display handler
let (frame_tx, frame_rx) = bounded::<VideoFrame>(8);

// Input events from IronRDP to async portal
let (input_tx, input_rx) = bounded::<InputEvent>(16);
```

#### Shared State (Arc<RwLock<T>>)
**Use for:** Shared read-heavy state
```rust
// Configuration (read-only after init)
Arc<Config>

// Session state (read-heavy, occasional writes)
Arc<RwLock<SessionState>>

// Metrics (frequent reads, periodic writes)
Arc<RwLock<PerformanceMetrics>>
```

#### Shared State (Arc<Mutex<T>>)
**Use for:** Shared write-heavy state or small critical sections
```rust
// Connection tracking
Arc<Mutex<HashMap<SessionId, Connection>>>

// Resource locks
Arc<Mutex<BufferPool>>
```

#### Atomic Types
**Use for:** Lock-free counters and flags
```rust
// Frame counter
AtomicU64

// Connection count
AtomicUsize

// Shutdown flag
AtomicBool
```

### Task Spawning Strategy

#### IronRDP Server Task
```rust
// IronRDP manages its own thread pool
ironrdp_server::Server::new(config)
    .with_display_handler(display_handler)
    .with_input_handler(input_handler)
    .with_clipboard_handler(clipboard_handler)
    .run(listener)
    .await?;
```

#### Portal Tasks
```rust
// Spawn once at startup for portal communication
tokio::spawn(async move {
    portal_manager.run().await
});

tokio::spawn(async move {
    pipewire_receiver.run().await
});
```

#### Per-Connection Trait Implementations
```rust
// IronRDP calls trait methods per connection
// No manual spawning needed - IronRDP handles it
impl DisplayUpdateHandler for MyHandler {
    async fn handle_update(&mut self, surface_id: u32) -> Result<UpdatePdu> {
        // Called by IronRDP when needed
    }
}
```

---

## PROTOCOL STACK

### Network Protocol Layers

```
┌────────────────────────────────────────┐
│      Application Layer                 │
│  Video, Input, Clipboard, Audio Data   │
└───────────────┬────────────────────────┘
                │
┌───────────────▼────────────────────────┐
│      RDP Virtual Channels              │
│  Graphics, Input, Clipboard Virtual    │
│  Channels (RDPGFX, RDPEI, CLIPRDR)     │
└───────────────┬────────────────────────┘
                │
┌───────────────▼────────────────────────┐
│      RDP Protocol Layer                │
│  PDU Encoding/Decoding                 │
│  Channel Multiplexing                  │
│  Capability Exchange                   │
└───────────────┬────────────────────────┘
                │
┌───────────────▼────────────────────────┐
│      MCS (T.125) Layer                 │
│  Multi-Channel Service                 │
│  Domain/Channel Management             │
└───────────────┬────────────────────────┘
                │
┌───────────────▼────────────────────────┐
│      X.224 Layer                       │
│  Connection-Oriented Transport         │
└───────────────┬────────────────────────┘
                │
┌───────────────▼────────────────────────┐
│      TLS 1.3 Layer                     │
│  Encryption, Authentication            │
│  Cipher: TLS_AES_256_GCM_SHA384        │
└───────────────┬────────────────────────┘
                │
┌───────────────▼────────────────────────┐
│      TCP Layer                         │
│  Port 3389                             │
│  Reliable Stream                       │
└───────────────┬────────────────────────┘
                │
┌───────────────▼────────────────────────┐
│      IP/Ethernet                       │
└────────────────────────────────────────┘
```

### RDP Virtual Channel Architecture

```
┌────────────────────────────────────────────────┐
│         IronRDP Server Connection              │
└───┬────────────┬────────────┬──────────────┬───┘
    │            │            │              │
┌───▼──────┐ ┌──▼──────┐ ┌───▼────────┐ ┌───▼──────────┐
│Graphics  │ │ Input   │ │ Clipboard  │ │ Audio        │
│Channel   │ │ Channel │ │  Channel   │ │Channel(P2)   │
│(RDPGFX)  │ │(RDPEI)  │ │ (CLIPRDR)  │ │ (RDPSND)     │
└──────────┘ └─────────┘ └────────────┘ └──────────────┘
    │            │            │              │
    │ Bitmap     │ Keyboard   │ Text/Image   │ Opus
    │ Updates    │ Mouse      │ Data         │ Audio
    │            │ Touch      │              │
```

---

## STATE MACHINES

### Connection State Machine

```
┌─────────┐
│ INITIAL │
└────┬────┘
     │
     │ TCP Accept
     ▼
┌─────────────┐
│ TLS_HANDSH  │
└────┬────────┘
     │
     │ TLS Complete
     ▼
┌─────────────┐
│ NLA_AUTH    │
└────┬────────┘
     │
     │ Auth Success
     ▼
┌─────────────┐
│ X224_CONN   │
└────┬────────┘
     │
     │ X.224 Connected
     ▼
┌─────────────┐
│ MCS_ATTACH  │
└────┬────────┘
     │
     │ MCS Attached
     ▼
┌─────────────┐
│ CHANNEL_JOIN│
└────┬────────┘
     │
     │ Channels Joined
     ▼
┌─────────────┐
│ CAPABILITY  │
│ _EXCHANGE   │
└────┬────────┘
     │
     │ Capabilities Negotiated
     ▼
┌─────────────┐
│ ACTIVE      │◄──────┐
└────┬────────┘       │
     │                │
     │ Data Flow      │ Reconnect
     ▼                │
┌─────────────┐       │
│ SUSPENDED   ├───────┘
└────┬────────┘
     │
     │ Disconnect
     ▼
┌─────────────┐
│ TERMINATED  │
└─────────────┘
```

### Display Pipeline State Machine

```
┌─────────┐
│  INIT   │
└────┬────┘
     │
     │ Portal Session Created
     ▼
┌─────────────┐
│ PORTAL_WAIT │
└────┬────────┘
     │
     │ User Approved
     ▼
┌─────────────┐
│ PIPEWIRE_   │
│  CONNECT    │
└────┬────────┘
     │
     │ PipeWire Connected
     ▼
┌─────────────┐
│ FORMAT_     │
│ NEGOTIATE   │
└────┬────────┘
     │
     │ Format Agreed (BGRA/RGB)
     ▼
┌─────────────┐
│ READY       │
└────┬────────┘
     │
     │ Start Streaming
     ▼
┌─────────────┐
│ STREAMING   │◄──────┐
└────┬────────┘       │
     │                │
     │ Frame Ready    │ Next Frame
     ▼                │
┌─────────────┐       │
│ CONVERTING  │       │
│ TO BITMAP   ├───────┘
└────┬────────┘
     │
     │ Shutdown Signal
     ▼
┌─────────────┐
│ STOPPING    │
└────┬────────┘
     │
     │ Resources Released
     ▼
┌─────────────┐
│ STOPPED     │
└─────────────┘
```

---

## ERROR HANDLING ARCHITECTURE

### Error Type Hierarchy

```
┌──────────────────────────────────┐
│     anyhow::Error (Application)  │
│     Used in: main, high-level    │
└────────────┬─────────────────────┘
             │
             │ Contains
             ▼
┌──────────────────────────────────┐
│     WrdError (Domain Errors)     │
│     thiserror-based              │
└────────────┬─────────────────────┘
             │
             ├──► ConfigError
             ├──► IronRdpError
             │    ├─► ProtocolError
             │    ├─► TlsError
             │    └─► AuthError
             ├──► DisplayError
             │    ├─► BitmapError
             │    └─► FrameError
             ├──► PortalError
             │    ├─► DbusError
             │    └─► PermissionDenied
             ├──► PipeWireError
             ├──► ClipboardError
             └──► InputError
```

### Error Handling Strategy

```rust
// Library error types (thiserror)
#[derive(Debug, thiserror::Error)]
pub enum DisplayError {
    #[error("Bitmap conversion failed: {0}")]
    BitmapConversion(String),

    #[error("Frame processing failed: {0}")]
    FrameProcessing(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("PipeWire error: {0}")]
    PipeWire(#[from] PipeWireError),
}

// Application error handling (anyhow)
pub async fn run_server(config: Config) -> Result<()> {
    let server = Server::new(config)
        .await
        .context("Failed to create server")?;

    server.run()
        .await
        .context("Server runtime error")?;

    Ok(())
}
```

### Error Propagation Rules

1. **Library code (src/*)**: Use `Result<T, DomainError>` with thiserror
2. **Application code (main.rs)**: Use `Result<T>` (anyhow)
3. **Never panic in library code**: Always return Result
4. **Panic only on:** Logic errors, invariant violations (in debug builds)
5. **Async errors**: Propagate with `?` operator
6. **Logging**: Log before returning error at origin

---

## SECURITY ARCHITECTURE

### Trust Boundaries

```
┌────────────────────────────────────────────┐
│         UNTRUSTED ZONE                     │
│                                            │
│  ┌──────────────────────────────────┐     │
│  │    RDP Client (Network)          │     │
│  │    • Potentially malicious       │     │
│  │    • All input validated         │     │
│  └────────────┬─────────────────────┘     │
└───────────────┼────────────────────────────┘
                │
         ┌──────▼──────┐
         │ TLS Layer   │ ◄──── Trust Boundary 1
         │ (Encrypted) │
         └──────┬──────┘
                │
┌───────────────▼────────────────────────────┐
│         DMZ ZONE                           │
│                                            │
│  ┌──────────────────────────────────┐     │
│  │  RDP Protocol Handler            │     │
│  │  • Validates PDUs                │     │
│  │  • Enforces size limits          │     │
│  │  • Rate limiting                 │     │
│  └────────────┬─────────────────────┘     │
└───────────────┼────────────────────────────┘
                │
         ┌──────▼──────┐
         │ Portal API  │ ◄──── Trust Boundary 2
         │ (Requires   │       (User Approval)
         │  Permission)│
         └──────┬──────┘
                │
┌───────────────▼────────────────────────────┐
│         TRUSTED ZONE                       │
│                                            │
│  ┌──────────────────────────────────┐     │
│  │  Wayland Compositor              │     │
│  │  • System clipboard              │     │
│  │  • Input injection               │     │
│  │  • Screen content                │     │
│  └──────────────────────────────────┘     │
└────────────────────────────────────────────┘
```

### Security Layers

1. **Network Security:**
   - TLS 1.3 mandatory
   - Strong cipher suites only
   - Certificate validation

2. **Authentication:**
   - NLA (CredSSP) required
   - PAM integration
   - Session tokens

3. **Authorization:**
   - Portal permissions (user approval)
   - Clipboard size limits
   - Type filtering

4. **Input Validation:**
   - All RDP PDUs validated
   - Size limits enforced
   - Range checking on coordinates

5. **Resource Limits:**
   - Connection limits
   - Frame rate limits
   - Bitrate limits
   - Timeout enforcement

---

## ARCHITECTURAL CONSTRAINTS

### MUST Follow
1. **Single responsibility:** Each component has ONE clear purpose
2. **Dependency inversion:** High-level modules don't depend on low-level details
3. **Interface segregation:** Thin, focused traits
4. **Portal-first:** All compositor access via portals
5. **Async-first:** All I/O operations async
6. **Thread-safe:** All shared state properly synchronized

### MUST NOT
1. **No circular dependencies:** Between modules
2. **No global mutable state:** Without proper synchronization
3. **No blocking operations:** On async runtime threads
4. **No direct Wayland access:** Only via portals
5. **No hardcoded values:** All configuration-driven

---

## APPENDIX: COMPONENT DEPENDENCY GRAPH

```
main.rs
  └─► WrdServer
       ├─► Config
       ├─► IronRdpServer (ironrdp-server crate)
       │    ├─► TLS/NLA (built-in)
       │    ├─► Protocol State Machine (built-in)
       │    ├─► Channel Management (built-in)
       │    └─► Bitmap Encoding (built-in)
       ├─► DisplayUpdateHandler (trait impl)
       │    ├─► PipeWireReceiver
       │    ├─► BitmapConverter
       │    │    ├─► PixelConverter
       │    │    └─► Compressor
       │    ├─► DamageTracker
       │    └─► CursorManager
       ├─► InputHandler (trait impl)
       │    ├─► InputForwarder
       │    ├─► KeyboardTranslator
       │    └─► PointerTranslator
       ├─► ClipboardHandler (trait impl)
       │    ├─► ClipboardManager
       │    ├─► FormatMapper
       │    └─► ClipboardPortal
       └─► PortalManager
            ├─► ScreenCastPortal
            ├─► RemoteDesktopPortal
            └─► ClipboardPortal
                 └─► PipeWireReceiver
                      └─► FrameBuffer
```

---

**END OF ARCHITECTURE SPECIFICATION**

Proceed to 02-TECHNOLOGY-STACK.md for dependency details.
