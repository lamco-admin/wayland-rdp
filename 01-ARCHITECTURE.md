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
     Video          Input      Clipboard        Audio
     Stream         Events      Data            Stream
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
│  │           CONNECTION MANAGER LAYER                       │ │
│  │  • TLS Termination                                       │ │
│  │  • NLA Authentication                                    │ │
│  │  • Session Management                                    │ │
│  │  • Resource Tracking                                     │ │
│  └────────┬──────────────────────────────────┬──────────────┘ │
│           │                                  │                │
│  ┌────────▼──────────────────────────────────▼──────────────┐ │
│  │         RDP PROTOCOL HANDLER LAYER                       │ │
│  │  • Protocol State Machine                                │ │
│  │  • Capability Negotiation                                │ │
│  │  • Channel Multiplexing                                  │ │
│  │  • PDU Encoding/Decoding                                 │ │
│  └─┬──────────┬─────────────┬──────────┬────────────┬───────┘ │
│    │          │             │          │            │         │
│ ┌──▼────┐ ┌──▼────┐ ┌──────▼───┐ ┌────▼──────┐ ┌──▼──────┐  │
│ │Video  │ │Input  │ │Clipboard │ │Multi-Mon  │ │Audio    │  │
│ │Channel│ │Channel│ │ Channel  │ │ Manager   │ │(Phase2) │  │
│ └──┬────┘ └──┬────┘ └──────┬───┘ └────┬──────┘ └──┬──────┘  │
│    │         │             │          │            │         │
│ ┌──▼──────┐ ┌▼────────┐ ┌─▼──────┐ ┌─▼─────┐   ┌─▼──────┐  │
│ │ Video   │ │ Input   │ │Clipbrd │ │Monitor│   │ Audio  │  │
│ │Pipeline │ │ Handler │ │Manager │ │Manager│   │Pipeline│  │
│ └──┬──────┘ └──┬──────┘ └─┬──────┘ └───────┘   └────────┘  │
│    │           │           │                                 │
└────┼───────────┼───────────┼─────────────────────────────────┘
     │           │           │
┌────▼───────────▼───────────▼─────────────────────────────────┐
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
2. **Protocol Layer:** RDP 10.x protocol handling
3. **Channel Layer:** Multiplexed data channels (video, input, clipboard, audio)
4. **Processing Layer:** Video encoding, input translation, clipboard conversion
5. **Integration Layer:** Portal-based access to compositor resources
6. **System Layer:** Wayland compositor, PipeWire, libei

---

## COMPONENT ARCHITECTURE

### Core Components

#### 1. Server Component
**Location:** `src/server/`

**Responsibilities:**
- Accept TCP connections
- Manage server lifecycle
- Coordinate all subsystems
- Handle graceful shutdown

**Sub-components:**
- **ConnectionManager:** Manages all active connections
- **SessionManager:** Tracks session state per connection
- **ResourceTracker:** Monitors resource usage

**Interfaces:**
```rust
pub struct Server {
    config: Arc<Config>,
    listener: TcpListener,
    connection_manager: Arc<ConnectionManager>,
    portal_manager: Arc<PortalManager>,
    security_manager: Arc<SecurityManager>,
    shutdown_tx: broadcast::Sender<()>,
}

impl Server {
    pub async fn new(config: Config) -> Result<Self>;
    pub async fn run(self) -> Result<()>;
    async fn shutdown(&self) -> Result<()>;
}
```

#### 2. Security Component
**Location:** `src/security/`

**Responsibilities:**
- TLS configuration and termination
- Certificate management
- User authentication (PAM)
- Credential validation

**Sub-components:**
- **TlsManager:** TLS 1.3 configuration
- **CertificateManager:** Certificate loading and generation
- **Authenticator:** PAM-based authentication
- **TokenManager:** Session token generation

**Interfaces:**
```rust
pub struct SecurityManager {
    tls_config: Arc<TlsConfig>,
    cert_manager: CertificateManager,
    authenticator: Authenticator,
}

impl SecurityManager {
    pub async fn new(config: &Config) -> Result<Self>;
    pub fn create_acceptor(&self) -> Result<TlsAcceptor>;
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<bool>;
}
```

#### 3. RDP Protocol Component
**Location:** `src/rdp/`

**Responsibilities:**
- RDP protocol state machine
- Capability negotiation
- Channel management
- PDU encoding/decoding

**Sub-components:**
- **RdpServer:** Main protocol handler (wraps IronRDP)
- **CapabilityNegotiator:** Advertise and negotiate capabilities
- **ChannelManager:** Multiplex data channels

**Interfaces:**
```rust
pub struct RdpServer {
    session_id: SessionId,
    state: RdpState,
    channels: HashMap<ChannelId, Box<dyn Channel>>,
    encoder_tx: mpsc::Sender<EncodedFrame>,
    input_rx: mpsc::Receiver<RdpInputEvent>,
}

impl RdpServer {
    pub async fn new(stream: TlsStream) -> Result<Self>;
    pub async fn run(&mut self) -> Result<()>;
    pub async fn send_frame(&mut self, frame: EncodedFrame) -> Result<()>;
}
```

#### 4. Portal Integration Component
**Location:** `src/portal/`

**Responsibilities:**
- D-Bus connection management
- ScreenCast portal interaction
- RemoteDesktop portal interaction
- Clipboard portal interaction
- Session lifecycle management

**Sub-components:**
- **ScreenCastManager:** Video stream acquisition
- **RemoteDesktopManager:** Input injection + video
- **ClipboardManager:** Clipboard access
- **SessionManager:** Portal session lifecycle

**Interfaces:**
```rust
pub struct PortalManager {
    connection: zbus::Connection,
    screencast: Arc<ScreenCastManager>,
    remote_desktop: Arc<RemoteDesktopManager>,
    clipboard: Arc<ClipboardManager>,
}

impl PortalManager {
    pub async fn new(config: &Arc<Config>) -> Result<Self>;
    pub async fn create_session(&self) -> Result<PortalSessionHandle>;
}

pub struct PortalSessionHandle {
    session_id: String,
    pipewire_fd: RawFd,
    streams: Vec<StreamInfo>,
}
```

#### 5. PipeWire Component
**Location:** `src/pipewire/`

**Responsibilities:**
- PipeWire connection using FD from portal
- Stream negotiation (format, resolution)
- Frame reception and buffering
- DMA-BUF handling (zero-copy)

**Sub-components:**
- **StreamManager:** PipeWire stream lifecycle
- **FrameReceiver:** Async frame reception
- **FormatNegotiator:** Video format selection

**Interfaces:**
```rust
pub struct PipeWireStream {
    fd: RawFd,
    stream_id: u32,
    format: VideoFormat,
    frame_tx: mpsc::Sender<VideoFrame>,
}

impl PipeWireStream {
    pub async fn new(fd: RawFd, stream_info: &StreamInfo) -> Result<Self>;
    pub async fn start(&mut self) -> Result<()>;
    pub async fn receive_frames(&mut self) -> Result<()>;
}
```

#### 6. Video Processing Component
**Location:** `src/video/`

**Responsibilities:**
- Video encoding pipeline orchestration
- Encoder selection and management
- Damage tracking
- Cursor extraction
- Format conversion
- Frame scaling

**Sub-components:**
- **VideoPipeline:** Pipeline orchestrator
- **EncoderManager:** H.264 encoder (VA-API or OpenH264)
- **DamageTracker:** Incremental update tracking
- **CursorManager:** Cursor metadata handling
- **FormatConverter:** Pixel format conversion
- **FrameScaler:** Resolution scaling

**Interfaces:**
```rust
pub struct VideoPipeline {
    encoder: Box<dyn VideoEncoder>,
    damage_tracker: DamageTracker,
    cursor_manager: CursorManager,
    frame_rx: mpsc::Receiver<VideoFrame>,
    encoded_tx: mpsc::Sender<EncodedFrame>,
}

#[async_trait]
pub trait VideoEncoder: Send + Sync {
    async fn encode(&mut self, frame: &VideoFrame) -> Result<EncodedFrame>;
    async fn flush(&mut self) -> Result<Vec<EncodedFrame>>;
    fn set_bitrate(&mut self, kbps: u32) -> Result<()>;
}
```

#### 7. Input Handling Component
**Location:** `src/input/`

**Responsibilities:**
- RDP input event translation
- Keyboard event injection
- Pointer event injection
- Touch event injection (future)
- Coordinate transformation

**Sub-components:**
- **InputTranslator:** RDP ↔ Wayland translation
- **KeyboardHandler:** Keyboard events via portal/libei
- **PointerHandler:** Mouse events via portal/libei
- **TouchHandler:** Touch events (Phase 2 or future)

**Interfaces:**
```rust
pub struct InputManager {
    translator: Arc<InputTranslator>,
    keyboard: Arc<KeyboardHandler>,
    pointer: Arc<PointerHandler>,
}

impl InputManager {
    pub async fn new(
        config: Arc<Config>,
        remote_desktop: Arc<RemoteDesktopManager>,
    ) -> Result<Self>;

    pub async fn handle_event(&self, event: RdpInputEvent) -> Result<()>;
}
```

#### 8. Clipboard Component
**Location:** `src/clipboard/`

**Responsibilities:**
- Bidirectional clipboard synchronization
- Format conversion (RDP ↔ MIME types)
- Size validation
- Type filtering

**Sub-components:**
- **ClipboardSyncManager:** Sync orchestrator
- **FormatConverter:** RDP format ↔ MIME type conversion

**Interfaces:**
```rust
pub struct ClipboardManager {
    rdp_channel: Arc<ClipboardChannel>,
    portal: Arc<ClipboardPortal>,
    sync_state: Arc<RwLock<ClipboardState>>,
}

impl ClipboardManager {
    pub async fn sync_to_server(&mut self, data: Vec<u8>, format: u32) -> Result<()>;
    pub async fn sync_to_client(&mut self, mime_type: &str) -> Result<()>;
}
```

#### 9. Multi-Monitor Component
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
│  Frame Receiver │
│  (PipeWire API) │
└────────┬────────┘
         │
         │ VideoFrame
         │ (BGRA/NV12)
         ▼
┌─────────────────┐
│ Damage Tracker  │
│ (Detect changes)│
└────────┬────────┘
         │
         │ VideoFrame + DamageRects
         ▼
┌─────────────────┐
│Format Converter │
│  (BGRA→NV12)    │
└────────┬────────┘
         │
         │ VideoFrame (NV12)
         ▼
┌─────────────────┐
│ Cursor Manager  │
│ (Extract cursor)│
└────────┬────────┘
         │
         │ VideoFrame + CursorInfo
         ▼
┌─────────────────┐
│  H.264 Encoder  │
│ (VA-API/OpenH264│
└────────┬────────┘
         │
         │ EncodedFrame (NAL units)
         ▼
┌─────────────────┐
│ RDP Graphics    │
│    Channel      │
└────────┬────────┘
         │
         │ RDP PDUs
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
│  RDP Input      │
│   Channel       │
└────────┬────────┘
         │
         │ RdpInputEvent
         ▼
┌─────────────────┐
│ Input Translator│
│ (RDP→Wayland)   │
└────────┬────────┘
         │
         │ Translated Event
         ▼
┌─────────────────┐
│ Input Handler   │
│(Keyboard/Pointer│
└────────┬────────┘
         │
         │ (Portal API)
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
         │ Clipboard Format List
         ▼
┌─────────────────┐
│  Clipboard      │
│   Channel       │
└────────┬────────┘
         │
         │ Format Request
         ▼
┌─────────────────┐
│   RDP Client    │
│ (Clipboard Data)│
└────────┬────────┘
         │
         │ Clipboard Data PDU
         ▼
┌─────────────────┐
│ Clipboard       │
│   Manager       │
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
│ Clipboard       │
│   Manager       │
└────────┬────────┘
         │
         │ Read clipboard
         │ Format Conversion
         │ (text/plain → CF_UNICODETEXT)
         ▼
┌─────────────────┐
│  Clipboard      │
│   Channel       │
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
│              TOKIO ASYNC RUNTIME                         │
│          (Multi-threaded work-stealing)                  │
└────┬──────────────┬──────────────┬──────────────┬────────┘
     │              │              │              │
     │              │              │              │
┌────▼─────┐  ┌────▼─────┐  ┌────▼─────┐  ┌────▼─────┐
│ Network  │  │ Network  │  │ Network  │  │ Network  │
│ Worker 1 │  │ Worker 2 │  │ Worker 3 │  │ Worker 4 │
└──────────┘  └──────────┘  └──────────┘  └──────────┘
     │              │              │              │
     │   Handle RDP Protocol I/O                  │
     │   TLS operations                            │
     │   Channel multiplexing                      │
     └──────────────┴──────────────┴──────────────┘
                    │
                    │ Async channels
                    ▼
┌──────────────────────────────────────────────────────────┐
│         DEDICATED ENCODING THREAD POOL                   │
│         (Blocking operations, configurable size)         │
└────┬──────────────┬──────────────┬──────────────────────┘
     │              │              │
┌────▼─────┐  ┌────▼─────┐  ┌────▼─────┐
│ Encoder  │  │ Encoder  │  │ Encoder  │
│ Thread 1 │  │ Thread 2 │  │ Thread 3 │
│(Monitor1)│  │(Monitor2)│  │ (Spare)  │
└──────────┘  └──────────┘  └──────────┘
     │              │              │
     │   H.264 encoding (VA-API or OpenH264)
     │   Format conversion
     │   CPU/GPU intensive work
     └──────────────┴──────────────┘
                    │
                    │ Crossbeam channels
                    ▼
            Network Workers
            (send to RDP client)
```

### Concurrency Primitives

#### Async Channels (tokio::sync::mpsc)
**Use for:** Inter-task communication within async context
```rust
// Video frames from PipeWire to encoder
let (frame_tx, frame_rx) = mpsc::channel::<VideoFrame>(16);

// Encoded frames from encoder to RDP
let (encoded_tx, encoded_rx) = mpsc::channel::<EncodedFrame>(32);

// Input events from RDP to input handler
let (input_tx, input_rx) = mpsc::channel::<RdpInputEvent>(64);
```

#### Sync Channels (crossbeam_channel)
**Use for:** Communication between async and sync (blocking) threads
```rust
// Frames to blocking encoder threads
let (encoder_tx, encoder_rx) = bounded::<VideoFrame>(8);

// Encoded frames from blocking threads
let (encoded_tx, encoded_rx) = bounded::<EncodedFrame>(16);
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

#### Long-Lived Tasks
```rust
// Spawn once at startup, run until shutdown
tokio::spawn(async move {
    portal_session.run().await
});

tokio::spawn(async move {
    video_pipeline.run().await
});
```

#### Per-Connection Tasks
```rust
// Spawn per client connection
tokio::spawn(async move {
    rdp_server.handle_session(stream).await
});
```

#### Blocking Operations
```rust
// Offload to dedicated thread pool
tokio::task::spawn_blocking(move || {
    encoder.encode_sync(frame)
}).await?;
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
│         RDP Connection                         │
└───┬────────────┬────────────┬──────────────┬───┘
    │            │            │              │
┌───▼──────┐ ┌──▼──────┐ ┌───▼────────┐ ┌───▼──────────┐
│Graphics  │ │ Input   │ │ Clipboard  │ │ Audio        │
│Channel   │ │ Channel │ │  Channel   │ │Channel(P2)   │
│(RDPGFX)  │ │(RDPEI)  │ │ (CLIPRDR)  │ │ (RDPSND)     │
└──────────┘ └─────────┘ └────────────┘ └──────────────┘
    │            │            │              │
    │ H.264      │ Keyboard   │ Text/Image   │ Opus
    │ Frames     │ Mouse      │ Data         │ Audio
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

### Video Pipeline State Machine

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
     │ Format Agreed
     ▼
┌─────────────┐
│ ENCODER_    │
│   INIT      │
└────┬────────┘
     │
     │ Encoder Ready
     ▼
┌─────────────┐
│ STREAMING   │◄──────┐
└────┬────────┘       │
     │                │
     │ Process Frame  │ Next Frame
     ▼                │
┌─────────────┐       │
│ ENCODING    ├───────┘
└────┬────────┘
     │
     │ Shutdown Signal
     ▼
┌─────────────┐
│ FLUSHING    │
└────┬────────┘
     │
     │ Flush Complete
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
             ├──► SecurityError
             │    ├─► TlsError
             │    └─► AuthError
             ├──► RdpError
             │    ├─► ProtocolError
             │    ├─► ChannelError
             │    └─► CodecError
             ├──► PortalError
             │    ├─► DbusError
             │    └─► PermissionDenied
             ├──► PipeWireError
             ├──► VideoError
             │    └─► EncodingError
             └──► InputError
```

### Error Handling Strategy

```rust
// Library error types (thiserror)
#[derive(Debug, thiserror::Error)]
pub enum VideoError {
    #[error("Encoder initialization failed: {0}")]
    EncoderInit(String),

    #[error("Encoding failed: {0}")]
    EncodingFailed(String),

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
  └─► Server
       ├─► Config
       ├─► SecurityManager
       │    ├─► TlsConfig
       │    ├─► Authenticator (PAM)
       │    └─► CertificateManager
       ├─► ConnectionManager
       │    └─► RdpServer (IronRDP)
       │         ├─► GraphicsChannel
       │         │    └─► VideoPipeline
       │         │         ├─► VideoEncoder (VA-API/OpenH264)
       │         │         ├─► DamageTracker
       │         │         └─► CursorManager
       │         ├─► InputChannel
       │         │    └─► InputManager
       │         │         ├─► KeyboardHandler
       │         │         └─► PointerHandler
       │         └─► ClipboardChannel
       │              └─► ClipboardManager
       └─► PortalManager
            ├─► ScreenCastManager
            ├─► RemoteDesktopManager
            └─► ClipboardPortal
                 └─► PipeWireStream
                      └─► FrameReceiver
```

---

**END OF ARCHITECTURE SPECIFICATION**

Proceed to 02-TECHNOLOGY-STACK.md for dependency details.
