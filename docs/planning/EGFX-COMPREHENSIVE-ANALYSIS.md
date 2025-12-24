# EGFX Architecture: Comprehensive Analysis

## Executive Summary

This document provides a deep analysis of IronRDP's architecture, patterns, and preferences for handling the EGFX frame sending challenge. It also covers Rust best practices, MS-RDP protocol considerations, and future development implications.

**Key Finding**: IronRDP acknowledges this exact problem exists with `FIXME(#61)` comments in both client and server DVC code, indicating the team knows proactive DVC sending needs improvement.

---

## 1. IronRDP Architecture Analysis

### 1.1 Tier System

IronRDP uses a three-tier architecture:

| Tier | Constraints | Examples |
|------|-------------|----------|
| **Core** | `no_std`, no I/O, must be fuzzed, no platform code | ironrdp-pdu, ironrdp-dvc, ironrdp-egfx |
| **Extra** | Higher-level, I/O allowed, relaxed constraints | ironrdp-client, ironrdp-tokio |
| **Community** | Community-maintained, may break with core changes | ironrdp-server (@mihneabuz) |

**Implication**: `ironrdp-server` is Community Tier, meaning changes are more acceptable than Core Tier. The maintainer (@mihneabuz) is the decision-maker for PRs.

### 1.2 Key Architectural Invariants

From `ARCHITECTURE.md`:

1. **No I/O in Core Tier** - Core crates must never interact with the outside world
2. **Minimal monomorphization** - Avoid generic bloat in downstream code
3. **Dependency injection** - When runtime info is needed (no system calls)
4. **Object-safe traits** - `DvcProcessor`, `SvcProcessor` use `AsAny` pattern

### 1.3 Channel Architecture

```
Static Virtual Channels (SVC)                Dynamic Virtual Channels (DVC)
─────────────────────────────────           ─────────────────────────────────

SvcProcessor trait                          DvcProcessor trait
├── channel_name() -> ChannelName          ├── channel_name() -> &str
├── start() -> Vec<SvcMessage>             ├── start(channel_id) -> Vec<DvcMessage>
├── process(payload) -> Vec<SvcMessage>    ├── process(channel_id, payload) -> Vec<DvcMessage>
└── compression_condition()                 └── close(channel_id)

Examples: CLIPRDR, RDPSND                   Examples: EGFX, DisplayControl, AInput

Registered directly on Acceptor             Wrapped by DrdynvcServer (itself an SVC)
```

---

## 2. How Proactive Sending Works Today

### 2.1 Static Channels (Working Pattern)

Clipboard and Sound use the `ServerEvent` pattern:

```rust
// In ironrdp-server/src/server.rs

enum ServerEvent {
    Quit(String),
    Clipboard(ClipboardMessage),    // ← Clipboard proactive messages
    Rdpsnd(RdpsndServerMessage),    // ← Sound proactive messages
    SetCredentials(Credentials),
    GetLocalAddr(oneshot::Sender<Option<SocketAddr>>),
}

// The server loop dispatches these events:
async fn dispatch_server_events(&mut self, events: &mut Vec<ServerEvent>, ...) {
    for event in events.drain(..) {
        match event {
            ServerEvent::Clipboard(c) => {
                // 1. Get the SVC processor by type
                let cliprdr = self.get_svc_processor::<CliprdrServer>()?;

                // 2. Call the appropriate method
                let msgs = match c {
                    ClipboardMessage::SendInitiateCopy(formats) => cliprdr.initiate_copy(&formats),
                    // ...
                };

                // 3. Encode and send
                let channel_id = self.get_channel_id_by_type::<CliprdrServer>()?;
                let data = server_encode_svc_messages(msgs.into(), channel_id, user_channel_id)?;
                writer.write_all(&data).await?;
            }
            ServerEvent::Rdpsnd(s) => {
                // Same pattern...
            }
        }
    }
}
```

**Why this works**: `CliprdrServer` is stored directly in `StaticChannelSet` and can be accessed via `get_svc_processor::<T>()`.

### 2.2 Dynamic Channels (The Gap)

DVCs work differently:

```rust
// DrdynvcServer owns all DVC processors
pub struct DrdynvcServer {
    dynamic_channels: Slab<DynamicChannel>,  // ← Processors are here
}

struct DynamicChannel {
    state: ChannelState,
    processor: Box<dyn DvcProcessor>,  // ← Type-erased, can't downcast easily
    complete_data: CompleteData,
}
```

There's **NO** equivalent to `dispatch_server_events` for DVCs. The server can't:
1. Access `GraphicsPipelineServer` from outside `DrdynvcServer`
2. Call `send_avc420_frame()` proactively
3. Get the resulting PDUs to the wire

### 2.3 Client-Side DVC Pattern

The **client** solved this with `RdpInputEvent::SendDvcMessages`:

```rust
// In ironrdp-client/src/rdp.rs
enum RdpInputEvent {
    // ...
    SendDvcMessages {
        channel_id: u32,
        messages: Vec<SvcMessage>,  // Already encoded SVC messages
    },
}

// In the event loop:
RdpInputEvent::SendDvcMessages { channel_id, messages } => {
    let frame = active_stage.encode_dvc_messages(messages)?;
    vec![ActiveStageOutput::ResponseFrame(frame)]
}
```

The client uses `DvcPipeProxyFactory` which creates named pipe proxies that can send DVC messages to the event loop.

### 2.4 The FIXME Comments

Both client and server DVC code acknowledge this limitation:

```rust
// ironrdp-dvc/src/client.rs:54
// FIXME(#61): it's likely we want to enable adding dynamic channels
// at any point during the session (message passing? other approach?)

// ironrdp-dvc/src/server.rs:77
// FIXME(#61): it's likely we want to enable adding dynamic channels
// at any point during the session (message passing? other approach?)
```

Issue #61 is the tracking issue for this architectural gap.

---

## 3. Solution Options Analysis

### Option A: ServerEvent::Egfx (Mirror Clipboard Pattern)

**Approach**: Add `ServerEvent::Egfx(EgfxMessage)` to ironrdp-server.

```rust
enum ServerEvent {
    // ...existing...
    Egfx(EgfxMessage),
}

enum EgfxMessage {
    SendAvc420Frame { surface_id: u16, h264_data: Vec<u8>, regions: Vec<Avc420Region>, timestamp: u32 },
    CreateSurface { width: u16, height: u16 },
    // ...
}
```

**IronRDP Reception**: ⭐⭐⭐⭐ (Very Favorable)
- Follows established pattern exactly
- Minimal conceptual overhead
- Easy to review and maintain

**Rust Best Practices**: ⭐⭐⭐⭐⭐
- Uses message passing (idiomatic async Rust)
- Clear ownership model
- No shared mutable state

**MS-RDP Alignment**: ⭐⭐⭐
- Works, but adds latency through event queue
- Frame data copied through channel

**Downsides**:
- Requires accessing DVC processor by type (new method on DrdynvcServer)
- Frame data copied through event channel

---

### Option B: Arc<Mutex<>> + Bridge Wrapper

**Approach**: Wrap `GraphicsPipelineServer` in shared ownership.

```rust
// Bridge wrapper implements DvcProcessor
pub struct GfxDvcBridge {
    inner: Arc<Mutex<GraphicsPipelineServer>>,
}

impl DvcProcessor for GfxDvcBridge {
    fn process(&mut self, channel_id: u32, payload: &[u8]) -> PduResult<Vec<DvcMessage>> {
        let mut server = self.inner.lock().unwrap();
        server.process(channel_id, payload)
    }
}

// Factory returns Arc for display handler access
impl GfxServerFactory for WrdGfxFactory {
    fn build_gfx_handler(&self) -> Box<dyn GraphicsPipelineHandler>;
    fn get_server(&self) -> Arc<Mutex<GraphicsPipelineServer>>;  // ← New!
}
```

**IronRDP Reception**: ⭐⭐⭐ (Acceptable with Discussion)
- Introduces shared ownership pattern not used elsewhere
- Adds complexity to factory trait
- Requires careful documentation

**Rust Best Practices**: ⭐⭐⭐
- Arc<Mutex<>> is acceptable but not preferred
- Potential for mutex contention
- Works well with tokio (use tokio::sync::Mutex)

**MS-RDP Alignment**: ⭐⭐⭐⭐⭐
- Direct frame sending, minimal latency
- No data copying overhead

**Downsides**:
- New ownership pattern for IronRDP
- Mutex contention possible (though unlikely with async)
- Bridge adds indirection

---

### Option C: DVC Message Injection (Bypass GraphicsPipelineServer)

**Approach**: Create EGFX PDUs directly and inject into DVC channel.

```rust
// In wrd-server-specs
fn send_frame(&self, h264_data: &[u8], regions: &[Avc420Region]) {
    // Create PDUs directly using ironrdp-egfx::pdu types
    let pdus = vec![
        GfxPdu::StartFrame(...),
        GfxPdu::WireToSurface1(...),
        GfxPdu::EndFrame(...),
    ];

    // Encode to DVC messages
    let messages = encode_dvc_messages(channel_id, pdus, flags)?;

    // Inject via new server method
    self.server.send_dvc_messages(messages).await?;
}
```

**IronRDP Reception**: ⭐⭐ (Less Favorable)
- Bypasses the server abstraction layer
- Duplicates logic from GraphicsPipelineServer
- Harder to maintain

**Rust Best Practices**: ⭐⭐⭐
- Works, but violates DRY principle
- Manual frame tracking required

**MS-RDP Alignment**: ⭐⭐⭐⭐
- Direct control over protocol
- Must re-implement frame tracking, QoE handling

**Downsides**:
- Loses GraphicsPipelineServer benefits (frame tracking, QoE, surface management)
- Must stay synchronized with ironrdp-egfx updates

---

### Option D: Output Channel in Handler (Callback Pattern)

**Approach**: Handler receives a channel to send frames.

```rust
pub trait GraphicsPipelineHandler: Send {
    // Existing callbacks...

    // New: Called by server when channel is created
    fn set_output_sender(&mut self, sender: mpsc::Sender<Vec<DvcMessage>>);
}
```

**IronRDP Reception**: ⭐⭐⭐ (Possible)
- Adds complexity to handler trait
- Pushes responsibility to implementor
- Less clean than ServerEvent

**Rust Best Practices**: ⭐⭐⭐⭐
- Dependency injection pattern
- Clear async channel semantics

**MS-RDP Alignment**: ⭐⭐⭐⭐
- Works well for streaming

**Downsides**:
- Handler becomes more complex
- Every implementor must handle output channel

---

### Option E: Hybrid (Recommended)

**Approach**: Combine Bridge wrapper with ServerEvent for encoding/sending.

```rust
// 1. Bridge wrapper for shared access
pub struct GfxDvcBridge {
    inner: Arc<Mutex<GraphicsPipelineServer>>,
}

// 2. Factory returns shared reference
impl GfxServerFactory for WrdGfxFactory {
    fn build_gfx_handler(&self) -> Box<dyn GraphicsPipelineHandler>;
    fn get_server_handle(&self) -> Option<Arc<Mutex<GraphicsPipelineServer>>>;
}

// 3. ServerEvent for frame output routing
enum ServerEvent {
    // ...
    Egfx(EgfxOutputMessage),  // DVC messages ready to send
}

enum EgfxOutputMessage {
    SendMessages { channel_id: u32, messages: Vec<SvcMessage> },
}
```

**IronRDP Reception**: ⭐⭐⭐⭐ (Favorable)
- Uses ServerEvent for output (familiar pattern)
- Bridge is implementation detail
- Minimal API surface change

**Rust Best Practices**: ⭐⭐⭐⭐
- Separates concerns (access vs. sending)
- Uses channels for async coordination

**MS-RDP Alignment**: ⭐⭐⭐⭐⭐
- Full GraphicsPipelineServer benefits
- Efficient frame path

---

## 4. Rust Best Practices Perspective

### 4.1 Ownership Patterns in Async Rust

| Pattern | Use Case | Complexity |
|---------|----------|------------|
| Message passing (mpsc) | Cross-task communication | Low |
| Arc<Mutex<T>> | Shared mutable state | Medium |
| Arc<RwLock<T>> | Read-heavy shared state | Medium |
| Interior mutability | Cell/RefCell variants | High (careful!) |

**Recommendation**: Prefer message passing (Option A/E) for IronRDP consistency.

### 4.2 Trait Design

IronRDP's traits use `AsAny` for downcasting:

```rust
pub trait DvcProcessor: AsAny + Send {
    // ...
}

// Allows downcasting from Box<dyn DvcProcessor> to concrete type
impl DynamicVirtualChannel {
    pub fn channel_processor_downcast_ref<T: DvcProcessor>(&self) -> Option<&T> {
        self.channel_processor.as_any().downcast_ref()
    }
}
```

This pattern enables accessing typed processors from type-erased storage.

### 4.3 Error Handling

IronRDP uses:
- `PduResult<T>` for protocol operations
- `anyhow::Result<T>` in binaries
- Concrete error types in libraries

---

## 5. MS-RDP Protocol Considerations

### 5.1 EGFX Frame Flow

MS-RDPEGFX specifies:

```
Server                                          Client
  |                                               |
  |--- StartFrame(frame_id, timestamp) --------->|
  |--- WireToSurface1(H.264 data) --------------->|
  |--- EndFrame(frame_id) ----------------------->|
  |                                               |
  |<-- FrameAcknowledge(frame_id, queue_depth) --|
  |<-- QoeFrameAcknowledge (optional) -----------|
```

**Critical**: Frames must be sent contiguously (StartFrame → WireToSurface → EndFrame) without interleaving.

### 5.2 Flow Control

Per MS-RDPEGFX:
- Server tracks "Unacknowledged Frames ADM element"
- Client reports `queue_depth` in FrameAcknowledge
- `queue_depth = 0xFFFFFFFF` means acknowledgments suspended

**Implication**: GraphicsPipelineServer already handles this correctly. Any solution should preserve this.

### 5.3 Codec Recommendations

From MS-RDPEGFX and best practices:
- **V10.7** preferred (latest with all features)
- **AVC420** for most content (4:2:0 chroma subsampling)
- **AVC444** for high-fidelity UI (4:4:4 full chroma)
- **Max frames in flight**: 3-8 typical

---

## 6. Future Development Implications

### 6.1 What Each Option Enables/Limits

| Option | Multimon | Resize | Dynamic Codec | Caching | Complexity |
|--------|----------|--------|---------------|---------|------------|
| A (Event) | ✅ | ✅ | ✅ | ✅ | Low |
| B (Arc) | ✅ | ✅ | ✅ | ✅ | Medium |
| C (Inject) | ⚠️ Manual | ⚠️ Manual | ⚠️ Manual | ❌ | High |
| D (Callback) | ✅ | ✅ | ✅ | ✅ | Medium |
| E (Hybrid) | ✅ | ✅ | ✅ | ✅ | Medium |

### 6.2 Upstream PR Strategy

**For upstream acceptance**:
1. Keep changes minimal in Core Tier crates
2. Follow existing patterns (ServerEvent for Community Tier)
3. Add tests (especially for Core Tier)
4. Document architectural decisions

**Likely PR breakdown**:
1. **PR 1**: Add `get_dvc_processor<T>()` method to `DrdynvcServer` (Core Tier, small)
2. **PR 2**: Add `ServerEvent::Egfx` variant (Community Tier, larger)
3. **PR 3**: Add `GfxDvcBridge` helper (Community Tier, optional)

### 6.3 Maintenance Burden

| Option | Coupling to IronRDP | Breaking Change Risk |
|--------|---------------------|----------------------|
| A | High (trait changes) | Medium |
| B | Medium (bridge isolates) | Low |
| C | Low (uses stable PDU types) | Low |
| D | High (trait changes) | Medium |
| E | Medium | Low |

---

## 7. Recommendation

### Primary Recommendation: Option E (Hybrid)

**Rationale**:
1. Uses familiar ServerEvent pattern (IronRDP acceptance likely)
2. Bridge provides isolation (future-proof)
3. Preserves GraphicsPipelineServer benefits
4. Minimal upstream API changes

### Implementation Order

1. **wrd-server-specs first**: Implement Bridge pattern locally
2. **Validate with real frames**: Ensure H.264 streaming works
3. **Prepare upstream PR**: Extract minimal changes for ironrdp-server
4. **Submit PR**: Focus on ServerEvent addition

### Fallback: Option A (Pure ServerEvent)

If Bridge wrapper is deemed too complex, Option A is simpler:
- Single upstream change (ServerEvent::Egfx)
- Requires method to access DVC processor by type
- More data copying but acceptable for v1

---

## 8. Summary

| Aspect | Best Choice | Reason |
|--------|-------------|--------|
| IronRDP Pattern Fit | Option A or E | Uses ServerEvent pattern |
| Rust Best Practices | Option E | Separates concerns |
| MS-RDP Compliance | Option B or E | Uses GraphicsPipelineServer |
| Future Flexibility | Option E | Bridge isolates changes |
| Simplicity | Option A | Least code |

**Final Recommendation**: Start with Option E (Hybrid), but be prepared to simplify to Option A based on upstream feedback.
