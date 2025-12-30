# EGFX Architecture Deep Dive

## The Fundamental Problem Explained

### What We Have vs What We Need

**Current capability (RemoteFX path):**
```
Your Code                          IronRDP                           Network
─────────────────────────────────────────────────────────────────────────────
WrdDisplayHandler
       │
       │ DisplayUpdate::Bitmap(pixels)
       ▼
   [tokio channel]
       │
       ▼
 RdpServer.client_loop()  ───►  UpdateEncoder  ───►  RemoteFX PDUs  ───►  Client
       │                              │                    │
       │                         [encodes to              │
       │                          RemoteFX]               │
       └──────────────────────────────────────────────────┘
                    All in one place, simple
```

**What we need for EGFX (H.264 path):**
```
Your Code                          IronRDP                           Network
─────────────────────────────────────────────────────────────────────────────
WrdDisplayHandler
       │
       │ VideoFrame (BGRA pixels)
       ▼
 EgfxVideoHandler
       │
       │ [OpenH264 encodes to H.264]
       ▼
  H.264 NAL data
       │
       │  ???????? HOW DO WE GET HERE ????????
       ▼
 GraphicsPipelineServer.send_avc420_frame()
       │
       │ [wraps in EGFX protocol]
       ▼
   EGFX PDUs (DvcMessage)
       │
       │  ???????? AND HERE ????????
       ▼
 DrdynvcServer (DVC channel)  ───►  Wire format  ───►  Client
```

The "????????" gaps are the problem.

---

## Understanding Rust Ownership Here

### How DVC Channels Work

When we build the server, channels are set up like this:

```rust
// In ironrdp-server/src/server.rs attach_channels()

// Step 1: Create the EGFX server (we have it here!)
let handler = gfx_factory.build_gfx_handler();
let gfx_server = GraphicsPipelineServer::new(handler);  // ◄── We create it

// Step 2: Move it into DrdynvcServer (now we lose access!)
dvc = dvc.with_dynamic_channel(gfx_server);  // ◄── MOVED, gone forever
                                │
                                ▼
                    DrdynvcServer now OWNS gfx_server
                    It's stored as Box<dyn DvcProcessor>
                    We can't get it back!

// Step 3: Attach to acceptor
acceptor.attach_static_channel(dvc);  // ◄── dvc is also moved
```

After this, the ownership chain is:
```
Acceptor
   └─► StaticChannelSet
          └─► DrdynvcServer (SVC processor)
                 └─► dynamic_channels: Slab<DynamicChannel>
                        └─► DynamicChannel
                               └─► processor: Box<dyn DvcProcessor>
                                      └─► GraphicsPipelineServer  ◄── Buried here!
```

**The GraphicsPipelineServer is buried 5 levels deep** and we have no public API to reach it.

---

## Why This Design Exists (It's Not Wrong)

The DVC infrastructure was designed for **reactive** channels:

```
Normal DVC Flow (e.g., clipboard):
──────────────────────────────────

1. Client sends data ───► DrdynvcServer.process()
                                │
                                ▼
                         Routes to correct DVC
                                │
                                ▼
                         DvcProcessor.process()
                                │
                                ▼
                         Returns Vec<DvcMessage>  ───► Sent to client
                                │
                    Response is sent as return value!
```

This works great for request/response patterns. But video streaming is **proactive**:

```
Video Streaming Flow (what we need):
────────────────────────────────────

1. Frame arrives from PipeWire (no client request!)
        │
        ▼
2. We encode it to H.264
        │
        ▼
3. We need to PUSH it to GraphicsPipelineServer
        │
        ▼
4. GraphicsPipelineServer wraps it in EGFX protocol
        │
        ▼
5. We need to PUSH those PDUs to DrdynvcServer
        │
        ▼
6. DrdynvcServer sends to client

Steps 3-6 have no triggering event from client!
```

---

## The Specific Technical Barriers

### Barrier 1: Can't Access GraphicsPipelineServer

```rust
// What we WANT to do:
fn send_frame(&mut self, h264_data: &[u8]) {
    // Get the EGFX server somehow
    let gfx: &mut GraphicsPipelineServer = ???;  // HOW?

    // Call its method
    gfx.send_avc420_frame(surface_id, h264_data, &regions, timestamp);
}
```

**Why it's hard:**
- `DrdynvcServer` stores channels as `Box<dyn DvcProcessor>` (type-erased)
- No method like `get_channel_by_name<T>()` exists
- Even if it did, we'd need mutable access through multiple layers

### Barrier 2: Can't Send DVC Data Proactively

Even if we could call `send_avc420_frame()`, the PDUs go into an internal queue:

```rust
// Inside GraphicsPipelineServer
pub fn send_avc420_frame(&mut self, ...) {
    // PDUs are queued internally
    self.output_queue.push_back(GfxPdu::StartFrame(...));
    self.output_queue.push_back(GfxPdu::WireToSurface1(...));
    self.output_queue.push_back(GfxPdu::EndFrame(...));
    // Returns nothing! PDUs are stuck in the queue!
}

// The queue is only drained when process() is called:
impl DvcProcessor for GraphicsPipelineServer {
    fn process(&mut self, payload: &[u8]) -> Vec<DvcMessage> {
        // Handle incoming data...
        Ok(self.drain_output())  // ◄── Only here are PDUs returned!
    }
}
```

So even with access, we'd need a way to:
1. Call `drain_output()`
2. Get those `Vec<DvcMessage>` out
3. Route them through DrdynvcServer to the wire

### Barrier 3: No DVC Send Method

`DrdynvcServer` has no method like:
```rust
// This doesn't exist:
fn send_for_channel(&mut self, channel_name: &str, messages: Vec<DvcMessage>);
```

It only sends data as responses to incoming requests.

---

## Solution: The Bridge Pattern

We need to **keep a reference** to GraphicsPipelineServer while also giving one to DrdynvcServer.

### Using Arc<Mutex<>>

```rust
// Shared ownership via Arc
let gfx_server = Arc::new(Mutex::new(GraphicsPipelineServer::new(handler)));

// Clone for DVC system
let gfx_for_dvc = Arc::clone(&gfx_server);

// Clone for our frame sending
let gfx_for_frames = Arc::clone(&gfx_server);
```

But `DrdynvcServer::with_dynamic_channel()` takes `T: DvcProcessor`, not `Arc<Mutex<T>>`.

### The Bridge Wrapper

We create a thin wrapper that implements `DvcProcessor`:

```rust
/// Bridge that holds shared reference to GraphicsPipelineServer
pub struct GfxDvcBridge {
    inner: Arc<Mutex<GraphicsPipelineServer>>,
}

impl DvcProcessor for GfxDvcBridge {
    fn channel_name(&self) -> &str {
        "Microsoft::Windows::RDS::Graphics"  // EGFX channel name
    }

    fn process(&mut self, channel_id: u32, payload: &[u8]) -> PduResult<Vec<DvcMessage>> {
        // Lock the shared server
        let mut server = self.inner.lock().unwrap();

        // Delegate to real implementation
        // (This is simplified - actual impl handles the protocol)
        server.process(channel_id, payload)
    }
}
```

Now:
```rust
// Create shared server
let gfx_server = Arc::new(Mutex::new(GraphicsPipelineServer::new(handler)));

// Bridge for DVC system (gets one Arc clone)
let bridge = GfxDvcBridge { inner: Arc::clone(&gfx_server) };
dvc = dvc.with_dynamic_channel(bridge);

// Keep another clone for frame sending
self.gfx_server = Some(gfx_server);  // ◄── Now we have access!
```

---

## Complete Data Flow With Solution

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        FRAME SENDING PATH                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PipeWire ──► WrdDisplayHandler                                            │
│                     │                                                       │
│                     │ VideoFrame (BGRA)                                     │
│                     ▼                                                       │
│              EgfxVideoHandler                                               │
│                     │                                                       │
│                     │ [OpenH264 encode]                                     │
│                     ▼                                                       │
│              H.264 NAL data                                                 │
│                     │                                                       │
│                     │ We have Arc<Mutex<GraphicsPipelineServer>>!          │
│                     ▼                                                       │
│         ┌─────────────────────────┐                                        │
│         │ gfx_server.lock()       │                                        │
│         │ .send_avc420_frame(...) │                                        │
│         │ .drain_output()         │ ──► Vec<DvcMessage>                    │
│         └─────────────────────────┘           │                            │
│                                               │                            │
│                                               ▼                            │
│                                    send_dvc_messages()  [NEW METHOD]       │
│                                               │                            │
│                                               ▼                            │
│                                    DrdynvcServer encodes                   │
│                                               │                            │
│                                               ▼                            │
│                                         Wire ──► Client                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                        CLIENT RESPONSE PATH                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Client ──► DrdynvcServer.process()                                        │
│                     │                                                       │
│                     │ Routes to EGFX channel                               │
│                     ▼                                                       │
│              GfxDvcBridge.process()                                        │
│                     │                                                       │
│                     │ Locks shared server                                  │
│                     ▼                                                       │
│         ┌─────────────────────────┐                                        │
│         │ gfx_server.lock()       │                                        │
│         │ .process(payload)       │ ──► Handles FrameAck, QoE, etc.       │
│         │ .drain_output()         │ ──► Any response PDUs                  │
│         └─────────────────────────┘                                        │
│                     │                                                       │
│                     ▼                                                       │
│              Response to client (if any)                                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## What We Need to Build

### 1. In ironrdp-server (upstream PR)

```rust
// New: GfxDvcBridge wrapper
pub struct GfxDvcBridge {
    inner: Arc<Mutex<GraphicsPipelineServer>>,
}

impl DvcProcessor for GfxDvcBridge { ... }

// New: Method to send DVC messages proactively
impl DrdynvcServer {
    pub fn encode_messages(&self, channel_id: u32, messages: Vec<DvcMessage>) -> Vec<u8> {
        // Encode DVC messages into wire format
    }
}

// Modified: GfxServerFactory returns shared access
pub trait GfxServerFactory: Send {
    fn build_gfx_handler(&self) -> Box<dyn GraphicsPipelineHandler>;
    fn get_server_handle(&self) -> Option<Arc<Mutex<GraphicsPipelineServer>>>;
}
```

### 2. In wrd-server-specs (proprietary)

```rust
// WrdGfxFactory stores the shared server
pub struct WrdGfxFactory {
    server: Option<Arc<Mutex<GraphicsPipelineServer>>>,
    handler_ready_tx: watch::Sender<bool>,
}

// Display handler has access
pub struct WrdDisplayHandler {
    gfx_server: Option<Arc<Mutex<GraphicsPipelineServer>>>,
    egfx_ready: watch::Receiver<bool>,
}

impl WrdDisplayHandler {
    async fn send_egfx_frame(&self, h264_data: &[u8], timestamp: u32) -> Result<()> {
        let gfx = self.gfx_server.as_ref().ok_or("EGFX not available")?;
        let mut server = gfx.lock().await;

        // Ensure surface exists
        if server.surfaces().is_empty() {
            let surface_id = server.create_surface(self.width, self.height)?;
            server.map_surface_to_output(surface_id, 0, 0)?;
        }

        // Send frame
        server.send_avc420_frame(surface_id, h264_data, &regions, timestamp);

        // Get PDUs and send
        let messages = server.drain_output();
        self.send_dvc_messages(messages).await?;

        Ok(())
    }
}
```

---

## Capability Negotiation (Automatic Best Codec)

This part already works! When client connects:

```
1. Client sends CapabilitiesAdvertise
   - Lists all supported capability versions (V8.1, V10, V10.7, etc.)
   - Each version implies certain codec support

2. GraphicsPipelineServer calls handler.capabilities_advertise()
   - Your handler sees what client supports

3. Server responds with CapabilitiesConfirm
   - Picks best mutually-supported version

4. GraphicsPipelineServer calls handler.on_ready(negotiated_caps)
   - Now you know exactly what codecs are available:
     - server.supports_avc420() → true/false
     - server.supports_avc444() → true/false

5. Your display handler checks and uses appropriate codec:
   if server.supports_avc420() {
       // Use H.264 path
   } else {
       // Fall back to RemoteFX
   }
```

The `WrdGraphicsHandler.on_ready()` we already implemented handles this!

---

## Summary: What's Blocking Us

| Barrier | Solution | Where to Implement |
|---------|----------|-------------------|
| Can't access GraphicsPipelineServer | Arc<Mutex<>> + Bridge wrapper | ironrdp-server |
| Can't send DVC data proactively | Add encode_messages() method | ironrdp-server |
| Need to coordinate readiness | watch channel from handler | wrd-server-specs |
| Need to route PDUs to wire | Server loop polls/receives messages | wrd-server-specs |

**Total upstream changes needed:** 2 additions to ironrdp-server
**Everything else:** Proprietary implementation in wrd-server-specs
