# EGFX Frame Sending Path Architecture

## Current State Analysis

### Data Flow for EGFX

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           CURRENT ARCHITECTURE                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PipeWire ──► WrdDisplayHandler ──► DisplayUpdate ──► UpdateEncoder        │
│                                                              │              │
│                                                              ▼              │
│                                                     RemoteFX PDUs          │
│                                                              │              │
│                                                              ▼              │
│                                                       RDP Client            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                           DESIRED EGFX PATH                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PipeWire ──► WrdDisplayHandler ──► EgfxVideoHandler ──► H.264 NAL         │
│                                           │                                 │
│                                           ▼                                 │
│                               GraphicsPipelineServer.send_avc420_frame()   │
│                                           │                                 │
│                                           ▼                                 │
│                                     DVC Messages                            │
│                                           │                                 │
│                                           ▼                                 │
│                                   DrdynvcServer ──► RDP Client             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### The Problem

1. **GraphicsPipelineServer is owned by DrdynvcServer** (inside static_channels)
2. **No external access** to call `send_avc420_frame()`
3. **Output queue pattern**: PDUs accumulate in `output_queue`, only drained on `process()`
4. **DVC infrastructure gap**: No method to proactively send DVC data

### How GraphicsPipelineServer Works

```rust
// In ironrdp-egfx/src/server.rs
impl GraphicsPipelineServer {
    // Queues PDU internally
    pub fn send_avc420_frame(&mut self, surface_id, h264_data, regions, timestamp) {
        self.output_queue.push_back(GfxPdu::StartFrame(...));
        self.output_queue.push_back(GfxPdu::WireToSurface1(...));
        self.output_queue.push_back(GfxPdu::EndFrame(...));
    }

    // Returns queued PDUs
    pub fn drain_output(&mut self) -> Vec<DvcMessage> {
        self.output_queue.drain(..).map(|pdu| Box::new(pdu)).collect()
    }
}

// DvcProcessor::process() calls drain_output() at the end
impl DvcProcessor for GraphicsPipelineServer {
    fn process(&mut self, channel_id: u32, payload: &[u8]) -> PduResult<Vec<DvcMessage>> {
        // ... handle incoming PDU ...
        Ok(self.drain_output())  // Only time output is sent!
    }
}
```

**The issue**: `drain_output()` is only called when processing INCOMING data.
For video streaming, we need to send frames PROACTIVELY.

---

## Architectural Options

### Option A: ServerEvent Pattern (Like Clipboard)

**Approach**: Add `ServerEvent::Egfx(EgfxMessage)` to ironrdp-server.

```rust
// In ironrdp-server
enum ServerEvent {
    // ... existing ...
    Egfx(EgfxMessage),
}

enum EgfxMessage {
    SendAvc420Frame { surface_id: u16, h264_data: Vec<u8>, regions: Vec<Avc420Region> },
    CreateSurface { width: u16, height: u16 },
    // ...
}

// In dispatch_server_events:
ServerEvent::Egfx(msg) => {
    let gfx = get_gfx_processor()?;  // NEW: need way to access GraphicsPipelineServer
    match msg {
        EgfxMessage::SendAvc420Frame { .. } => {
            gfx.send_avc420_frame(...);
            let msgs = gfx.drain_output();
            // encode and send via DVC
        }
    }
}
```

**Boundaries**:
- **ironrdp-server**: Add ServerEvent::Egfx, dispatch logic (upstream PR)
- **ironrdp-dvc**: Add method to access DVC processor by type (upstream PR)
- **wrd-server-specs**: Send EgfxMessage events from display handler

**Pros**:
- Follows existing pattern (clipboard, rdpsnd)
- Clean separation

**Cons**:
- Multiple upstream changes needed
- Frame data copying through event channel
- Latency from event queue

---

### Option B: Shared GraphicsPipelineServer

**Approach**: Wrap GraphicsPipelineServer in Arc<Mutex<>> for shared access.

```rust
// In wrd-server-specs
struct SharedGfxServer {
    inner: Arc<Mutex<GraphicsPipelineServer>>,
    output_tx: mpsc::Sender<Vec<DvcMessage>>,  // Sends to DVC layer
}

// Factory provides the Arc
impl GfxServerFactory for WrdGfxFactory {
    fn build_gfx_handler(&self) -> Box<dyn GraphicsPipelineHandler> {
        // Create server, store Arc, return handler
    }

    fn get_server(&self) -> Arc<Mutex<GraphicsPipelineServer>> {
        // Provide access for frame sending
    }
}

// Display handler uses the Arc
async fn send_frame(&self, frame: EncodedFrame) {
    let mut server = self.gfx_server.lock().await;
    server.send_avc420_frame(...);
    let msgs = server.drain_output();
    self.output_tx.send(msgs).await;  // Goes to DVC layer
}
```

**Issue**: Can't easily share `GraphicsPipelineServer` because:
1. It's moved into DrdynvcServer via `with_dynamic_channel()`
2. DrdynvcServer takes `Box<dyn DvcProcessor>`, not Arc

**Would require**:
- Wrapper struct implementing DvcProcessor that holds Arc<Mutex<GraphicsPipelineServer>>
- Changes to how ironrdp-server creates the GFX server

**Boundaries**:
- **ironrdp-egfx**: No changes
- **ironrdp-server**: Modify GfxServerFactory to support shared access
- **wrd-server-specs**: SharedGfxWrapper, direct frame sending

---

### Option C: Output Channel in Handler

**Approach**: Handler receives a channel to send frames OUT.

```rust
// In wrd-server-specs
pub struct WrdGraphicsHandler {
    frame_output: mpsc::Sender<GfxPdu>,  // Frames go here
}

impl GraphicsPipelineHandler for WrdGraphicsHandler {
    fn on_ready(&mut self, caps: &CapabilitySet) {
        // Signal that we can start sending frames
        // Handler knows the negotiated codec
    }
}

// Separate task polls frame_output and sends to DVC
```

**Issue**: GraphicsPipelineHandler only receives callbacks, can't send frames.
The handler doesn't have access to GraphicsPipelineServer's methods.

**Would require**:
- New callback in GraphicsPipelineHandler: `fn get_frame_sender(&mut self) -> FrameSender`
- Or: Trait extension in wrd-server-specs

---

### Option D: Direct DVC Message Injection (Bypass GraphicsPipelineServer)

**Approach**: Create EGFX PDUs directly and inject into DVC channel.

```rust
// In wrd-server-specs
fn send_frame(&self, h264_data: &[u8], regions: &[Avc420Region]) {
    // Create EGFX PDUs directly using ironrdp-egfx::pdu types
    let pdus = vec![
        GfxPdu::StartFrame(...),
        GfxPdu::WireToSurface1(...),
        GfxPdu::EndFrame(...),
    ];

    // Encode and send via DVC channel
    self.dvc_sender.send(pdus);
}
```

**Boundaries**:
- **ironrdp-dvc**: Add public method to send DVC data for a channel (upstream PR)
- **ironrdp-egfx**: Use PDU types only (no GraphicsPipelineServer)
- **wrd-server-specs**: Direct PDU creation and DVC injection

**Pros**:
- Minimal ironrdp changes
- Direct control over frame sending

**Cons**:
- Duplicates frame tracking logic from GraphicsPipelineServer
- Need to handle frame IDs, surface management ourselves
- Loses benefits of GraphicsPipelineServer (QoE, ack handling)

---

### Option E: Hybrid - GraphicsPipelineServer with External Polling

**Approach**: Store GraphicsPipelineServer externally, use wrapper for DVC.

```rust
// In ironrdp-server (or wrd-server-specs)
struct GfxDvcBridge {
    server: Arc<RwLock<GraphicsPipelineServer>>,
}

impl DvcProcessor for GfxDvcBridge {
    fn process(&mut self, channel_id: u32, payload: &[u8]) -> PduResult<Vec<DvcMessage>> {
        let mut server = self.server.write().unwrap();
        server.handle_incoming(payload)?;
        Ok(server.drain_output())
    }
}

// In server loop - poll for outgoing frames
async fn poll_gfx_output(&self) {
    let mut server = self.gfx_server.write().unwrap();
    if server.has_pending_output() {
        let msgs = server.drain_output();
        self.send_dvc_messages(msgs).await;
    }
}
```

**Boundaries**:
- **ironrdp-egfx**: No changes (use GraphicsPipelineServer as-is)
- **ironrdp-server**: Add GfxDvcBridge wrapper (upstream PR)
- **ironrdp-server**: Add DVC message sending method (upstream PR)
- **wrd-server-specs**: Use shared Arc<RwLock<GraphicsPipelineServer>>

---

## Recommended Approach

### Phase 1: Minimal Viable (Option E - Hybrid)

1. **Create GfxDvcBridge** in ironrdp-server (upstreamable)
   - Wrapper holding `Arc<RwLock<GraphicsPipelineServer>>`
   - Implements `DvcProcessor` by delegating to inner server

2. **Add send_dvc_data()** to ironrdp-server (upstreamable)
   - Method to send arbitrary DVC messages for a channel
   - Similar to how SVC messages are sent

3. **Modify WrdGfxFactory** to return the Arc
   - Factory provides access to the shared server
   - Display handler can call `send_avc420_frame()` directly

4. **Polling task** in server loop
   - Periodically checks for pending output
   - Sends via DVC channel

### Phase 2: Event-Driven (Option A refinement)

Once basic streaming works, refactor to event-driven:
1. Add `ServerEvent::Egfx` for cleaner separation
2. Move frame encoding to dedicated task
3. Add backpressure handling

---

## Code Placement Summary

| Component | Location | License | Upstreamable? |
|-----------|----------|---------|---------------|
| `GfxDvcBridge` | ironrdp-server | Apache/MIT | Yes |
| `send_dvc_data()` | ironrdp-server | Apache/MIT | Yes |
| `WrdGfxFactory` | wrd-server-specs | BSL-1.1 | No |
| `WrdGraphicsHandler` | wrd-server-specs | BSL-1.1 | No |
| `EgfxVideoHandler` | wrd-server-specs | BSL-1.1 | No |
| Frame polling task | wrd-server-specs | BSL-1.1 | No |
| H.264 encoding | wrd-server-specs | BSL-1.1 | No |

---

## Open Questions

1. **Threading**: Should frame encoding run in tokio or blocking thread?
2. **Backpressure**: How to handle when client can't keep up?
3. **Dual path**: Run RemoteFX and EGFX simultaneously or switch?
4. **Resize handling**: What happens when desktop resizes mid-stream?
5. **Frame acknowledgments**: How to use QoE feedback for bitrate control?
