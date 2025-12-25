# EGFX/H.264 Integration Handover Document

## Session Date: 2024-12-24

## Overview

This document details the implementation of EGFX/H.264 video streaming integration between IronRDP (upstream) and wrd-server-specs (downstream), using the "Hybrid Architecture" (Option E) for proactive frame sending.

---

## Repositories Involved

### 1. IronRDP (Upstream)
- **Path:** `/home/greg/wayland/IronRDP`
- **Role:** RDP protocol library providing EGFX infrastructure
- **Key Changes:** Added `GfxDvcBridge`, `GfxServerHandle`, `GfxServerFactory`, `ServerEvent::Egfx`

### 2. wrd-server-specs (Downstream)
- **Path:** `/home/greg/wayland/wrd-server-specs`
- **Role:** Wayland RDP server implementation
- **Key Changes:** Integrated EGFX pipeline with display handler

---

## Architecture: Hybrid Option E

The "Hybrid" approach enables **proactive frame sending** from the display handler while maintaining proper DVC infrastructure integration:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        WRD-SERVER-SPECS                                │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  PipeWire → VideoFrame → EgfxVideoHandler → Avc420Encoder             │
│                               │                    │                   │
│                               └────────────────────┘                   │
│                                       │ H.264 NAL data                 │
│                                       ▼                                │
│                              EgfxFrameSender                           │
│                                       │                                │
│                                       │ send_avc420_frame()            │
│                                       ▼                                │
│  ┌─────────────────────────────────────────────────────────────────┐  │
│  │ GfxServerHandle (Arc<Mutex<GraphicsPipelineServer>>)            │  │
│  │    - send_avc420_frame(surface_id, h264_data, regions, ts)      │  │
│  │    - drain_output() → Vec<DvcMessage>                           │  │
│  └─────────────────────────────────────────────────────────────────┘  │
│                                       │                                │
│                                       │ encode_dvc_messages()          │
│                                       ▼                                │
│                          ServerEvent::Egfx                             │
│                          (SendMessages { channel_id, messages })       │
│                                       │                                │
└───────────────────────────────────────│────────────────────────────────┘
                                        │
┌───────────────────────────────────────│────────────────────────────────┐
│                           IRONRDP                                      │
├───────────────────────────────────────│────────────────────────────────┤
│                                       ▼                                │
│                            RdpServer Event Loop                        │
│                                       │                                │
│                                       ▼                                │
│   GfxDvcBridge (DvcProcessor) ← handles client messages                │
│                                       │                                │
│                                       ▼                                │
│                              DVC Wire Protocol                         │
│                                       │                                │
└───────────────────────────────────────│────────────────────────────────┘
                                        │
                                        ▼
                                   RDP Client
```

### Key Components

1. **GfxDvcBridge** (ironrdp-server) - Wraps `Arc<Mutex<GraphicsPipelineServer>>`, implements `DvcProcessor`
2. **GfxServerHandle** (ironrdp-server) - Type alias for `Arc<Mutex<GraphicsPipelineServer>>`
3. **GfxServerFactory** (ironrdp-server) - Trait for creating EGFX handlers, returns both bridge and handle
4. **ServerEvent::Egfx** (ironrdp-server) - Routes pre-encoded DVC messages to the wire
5. **WrdGfxFactory** (wrd-server-specs) - Factory implementation with shared state
6. **EgfxFrameSender** (wrd-server-specs) - Clean API for frame sending, handles DVC encoding
7. **WrdGraphicsHandler** (wrd-server-specs) - Capability negotiation, state synchronization

---

## API Boundaries

### Critical Design Principle: NO IronRDP Type Leakage

**The public API of wrd-server-specs MUST NOT expose IronRDP types.** This maintains:
- Clean separation between downstream and upstream
- Ability to version/update IronRDP independently
- Clear API contracts

### Public Exports (wrd-server-specs)

**`src/egfx/mod.rs`**:
```rust
// Our encoder types (clean API - no IronRDP types)
pub use encoder::{
    align_to_16, annex_b_to_avc, Avc420Encoder, EncoderConfig,
    EncoderError, EncoderResult, EncoderStats, H264Frame,
};

// Our handler implementation
pub use handler::{SharedGraphicsHandler, WrdGraphicsHandler};

// Our video handler types
pub use video_handler::{EgfxVideoConfig, EgfxVideoHandler, EncodedFrame, EncodingStats};
```

**`src/server/mod.rs`**:
```rust
pub use gfx_factory::{HandlerState, SharedHandlerState, WrdGfxFactory};
pub use egfx_sender::{EgfxFrameSender, SendError};
```

### Internal IronRDP Usage

IronRDP types are used ONLY internally:
- `ironrdp_egfx::server::{GraphicsPipelineHandler, GraphicsPipelineServer}`
- `ironrdp_egfx::pdu::{Avc420Region, CapabilitySet, ...}`
- `ironrdp_server::{GfxDvcBridge, GfxServerFactory, GfxServerHandle, ServerEvent}`
- `ironrdp_dvc::encode_dvc_messages`
- `ironrdp_svc::ChannelFlags`

---

## Files Modified/Created

### IronRDP (Previous Sessions)

| File | Changes |
|------|---------|
| `crates/ironrdp-server/src/gfx.rs` | NEW: `GfxDvcBridge`, `GfxServerHandle`, `GfxServerFactory` |
| `crates/ironrdp-server/src/server.rs` | Added `ServerEvent::Egfx`, `EgfxServerMessage` |
| `crates/ironrdp-server/src/builder.rs` | Added `with_gfx_factory()` builder method |
| `crates/ironrdp-server/src/lib.rs` | Re-exported new types |

### wrd-server-specs (This Session)

| File | Status | Description |
|------|--------|-------------|
| `src/server/egfx_sender.rs` | **NEW** | EGFX frame sender with clean API |
| `src/server/gfx_factory.rs` | Modified | Added `SharedHandlerState`, updated `build_server_with_handle()` |
| `src/server/display_handler.rs` | Modified | Added EGFX fields, conditional routing, lazy initialization |
| `src/server/mod.rs` | Modified | Exported types, wired EGFX to server initialization |
| `src/egfx/handler.rs` | Modified | Added `SharedHandlerState` sync in callbacks |
| `src/egfx/mod.rs` | Modified | Removed IronRDP re-exports for clean API |

---

## State Synchronization

### Problem
`EgfxFrameSender` runs in the display handler async context and needs to check EGFX readiness without locking `GraphicsPipelineServer`.

### Solution: `SharedHandlerState`

```rust
pub struct HandlerState {
    pub is_ready: bool,           // EGFX channel ready
    pub is_avc420_enabled: bool,  // H.264 supported
    pub is_avc444_enabled: bool,  // Reserved for future
    pub primary_surface_id: u16,  // Surface for frames
    pub dvc_channel_id: u32,      // DVC channel ID (see limitation below)
}

pub type SharedHandlerState = Arc<RwLock<Option<HandlerState>>>;
```

### Synchronization Flow

1. `WrdGfxFactory` creates `SharedHandlerState`
2. `WrdGraphicsHandler::with_shared_state()` receives reference
3. Handler callbacks (`on_ready`, `on_surface_created`, etc.) call `sync_shared_state()`
4. `EgfxFrameSender` reads `SharedHandlerState` to check readiness

---

## Channel ID Propagation: FIXED ✅

### Problem Solved
The DVC channel_id is assigned by `DrdynvcServer` when the EGFX channel opens. It was being passed to `GraphicsPipelineServer::start(channel_id)` but **ignored** (the parameter had `_channel_id` prefix).

### Solution Implemented
**Store channel_id in `GraphicsPipelineServer`, query via getter**

```rust
// In ironrdp-egfx/src/server.rs
pub struct GraphicsPipelineServer {
    // ... other fields
    channel_id: Option<u32>,  // NEW
}

impl GraphicsPipelineServer {
    pub fn channel_id(&self) -> Option<u32> {  // NEW
        self.channel_id
    }
}

impl DvcProcessor for GraphicsPipelineServer {
    fn start(&mut self, channel_id: u32) -> PduResult<Vec<DvcMessage>> {
        self.channel_id = Some(channel_id);  // NOW STORED
        debug!(channel_id, "EGFX channel started");
        Ok(vec![])
    }
}
```

### Usage in EgfxFrameSender
```rust
// Get channel_id from the server while holding the lock
let (frame_id, dvc_messages, channel_id) = {
    let mut server = self.gfx_server.lock().await;
    let channel_id = server.channel_id().ok_or(SendError::NotReady)?;
    // ... send frame, drain output
    (frame_id, messages, channel_id)
};
```

### Benefits
- **Single source of truth**: channel_id stored in GraphicsPipelineServer
- **No callback needed**: Simple getter method
- **Natural data flow**: Received in `start()`, stored there
- **Backward compatible**: No trait changes required

---

## Frame Sending Path (Complete Flow)

```
1. PipeWire delivers DMA-BUF frame
       │
       ▼
2. WrdDisplayHandler receives frame in process_frame()
       │
       ▼
3. Check: is_egfx_ready()? (reads SharedHandlerState)
       │
       ├─ No  → Fall back to RemoteFX path (existing)
       │
       ▼ Yes
4. Lazy initialize Avc420Encoder (once)
       │
       ▼
5. EgfxVideoHandler.encode_frame() → H.264 NAL data
       │
       ▼
6. annex_b_to_avc() conversion (start codes → length prefix)
       │
       ▼
7. EgfxFrameSender.send_frame()
       │
       ├─ Lock GraphicsPipelineServer (via GfxServerHandle)
       ├─ send_avc420_frame(surface_id, h264_data, regions, ts)
       ├─ drain_output() → Vec<DvcMessage>
       ├─ encode_dvc_messages(channel_id, messages, flags)
       │
       ▼
8. ServerEvent::Egfx(SendMessages { channel_id, messages })
       │
       ▼
9. IronRDP event loop → DVC wire protocol → RDP Client
```

---

## H.264 Encoding Details

### Annex B vs AVC Format

- **Annex B**: Start codes (`00 00 00 01` or `00 00 01`) before each NAL
- **AVC**: 4-byte length prefix before each NAL

MS-RDPEGFX requires **AVC format**. The `annex_b_to_avc()` helper converts:

```rust
pub fn annex_b_to_avc(annex_b: &[u8]) -> Vec<u8>
```

### AVC420 Region

```rust
Avc420Region::full_frame(width, height, qp)
// qp = 22 is default (balance quality vs bitrate)
```

---

## Testing Checklist

Once channel_id propagation is fixed:

- [ ] Client connects, EGFX channel opens
- [ ] `on_ready()` called with V10.x or V8.1 capabilities
- [ ] AVC420 detected as enabled
- [ ] Surface created and tracked
- [ ] H.264 frame encoding succeeds
- [ ] DVC messages encoded and sent
- [ ] Client renders video

---

## Next Steps for New Session

### Priority 1: Test Integration
1. Run server with debug logging
2. Connect with mstsc.exe or FreeRDP
3. Verify EGFX negotiation succeeds
4. Verify H.264 frames are received
5. Debug any issues in the frame path

### Priority 2: Performance Optimization
1. Tune encoder parameters (bitrate, QP)
2. Consider frame pacing
3. Add QoE metrics feedback loop
4. Profile encode + send latency

### Priority 3: Handle Edge Cases
1. Desktop resize with EGFX active
2. Client disconnection cleanup
3. Multi-monitor support
4. Fallback to RemoteFX when H.264 not available

---

## Reference Links

- [MS-RDPEGFX](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/)
- [OpenH264](https://github.com/cisco/openh264)
- [IronRDP](https://github.com/Devolutions/IronRDP)

---

## Session Summary

### Completed
✅ Created `EgfxFrameSender` with clean API
✅ Integrated EGFX with display handler
✅ Added conditional EGFX routing in frame processing
✅ Implemented lazy encoder initialization
✅ Added `SharedHandlerState` synchronization
✅ Updated handler callbacks to sync state
✅ **FIXED channel_id propagation in IronRDP** (stored in GraphicsPipelineServer)
✅ **Simplified EgfxFrameSender** (queries channel_id from server)
✅ Verified compilation succeeds

### Previously Blocked - NOW RESOLVED
~~⚠️ Frame sending blocked on channel_id propagation~~ → **FIXED**

### Code Compiles
✅ `cargo check` passes on both repositories:
- `/home/greg/wayland/IronRDP` - ironrdp-egfx with channel_id fix
- `/home/greg/wayland/wrd-server-specs` - wrd-server with integrated EGFX
