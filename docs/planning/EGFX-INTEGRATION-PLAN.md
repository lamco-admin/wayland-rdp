# EGFX Integration Plan

**Date:** 2025-12-24
**Status:** Planning Complete - Ready for Implementation
**Estimated Effort:** 4-6 hours

---

## Executive Summary

This document outlines the strategy to integrate H.264 video streaming (EGFX/MS-RDPEGFX) into lamco-rdp-server. The integration leverages PR #1057's `ironrdp-egfx` crate while preserving our existing OpenH264 encoder and PipeWire integration.

**Key Decision:** PR #1057 provides protocol handling and frame sending; we provide encoding and frame capture.

---

## Architecture Boundary Analysis

### Classification Decision

EGFX follows the **existing boundary strategy** - no special "premium feature" treatment needed:

| Layer | License | Content |
|-------|---------|---------|
| **Protocol** | MIT/Apache-2.0 (upstream) | ironrdp-egfx - PDUs, GraphicsPipelineServer |
| **Orchestration** | BSL-1.1 (proprietary) | lamco-rdp-server - frame coordination, encoder integration |

### Component Boundaries

```
┌─────────────────────────────────────────────────────────────────────┐
│                    PROPRIETARY (BSL-1.1)                             │
│                    lamco-rdp-server/src/egfx/                        │
│                                                                      │
│  encoder.rs          ← OpenH264 wrapper, could extract later         │
│  video_handler.rs    ← Server orchestration (PROPRIETARY VALUE)      │
│  handler.rs          ← GraphicsPipelineHandler impl                  │
│                                                                      │
│  FUTURE PREMIUM FEATURES (will remain here):                         │
│  └── Adaptive bitrate algorithms                                     │
│  └── Network condition optimization                                  │
│  └── Multi-monitor EGFX coordination                                 │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    OPEN SOURCE (MIT/Apache-2.0)                      │
│                    ironrdp-egfx (upstream PR #1057)                  │
│                                                                      │
│  GraphicsPipelineServer   ← DvcProcessor implementation              │
│  All RDPGFX PDUs          ← Protocol encoding/decoding               │
│  AVC420/AVC444 frames     ← H.264 frame structures                   │
│  Surface management       ← Protocol-level surface tracking          │
└─────────────────────────────────────────────────────────────────────┘
```

### Rationale

**Why EGFX is NOT a premium feature requiring special protection:**

1. **H.264 encoding is commoditized** - OpenH264 is BSD-licensed, freely available
2. **EGFX protocol is standardized** - MS-RDPEGFX is Microsoft's public specification
3. **ironrdp-egfx becomes upstream** - Protocol layer will be MIT/Apache-2.0 anyway
4. **Value is in orchestration** - How you *use* EGFX, not the protocol itself

**What DOES provide proprietary value:**

1. **Integration quality** - Smooth Portal/PipeWire coordination (video_handler.rs)
2. **Future adaptive algorithms** - Quality vs bandwidth trade-offs
3. **Enterprise features** - Multi-monitor EGFX, VDI compositor integration
4. **Operational reliability** - Frame rate regulation, error recovery

### encoder.rs Extraction Consideration

The `encoder.rs` file (665 lines) wraps OpenH264 and could theoretically be extracted to a separate `lamco-video-h264` crate (MIT/Apache-2.0). However, this is **low priority** because:

- Small codebase (665 lines)
- Tightly coupled to video_handler.rs orchestration
- No external consumers anticipated
- Extraction adds maintenance overhead

**Decision:** Keep encoder.rs in lamco-rdp-server (BSL-1.1) for now. Revisit if there's demand for standalone H.264 encoding crate.

### Alignment with Existing Documents

This classification aligns with:
- `docs/strategy/CRATE-BREAKDOWN-AND-OPEN-SOURCE-STRATEGY.md`
- `docs/DEPENDENCY-ARCHITECTURE.md`
- `docs/LICENSING-AND-PUBLICATION-HANDOVER.md`

No updates to those documents are required.

---

## Current State Analysis

### IronRDP Fork Status

**Location:** `/home/greg/wayland/IronRDP`
**Current Branch:** `pr3-file-contents-response-v2`

| Branch | Purpose | Status |
|--------|---------|--------|
| `pr3-file-contents-response-v2` | File transfer (PRs #1064-1066) | Active, used by lamco-rdp-server |
| `fork/egfx-server-complete` | EGFX (PR #1057) | Complete, 5 commits, 4,293 lines |

**Critical Finding:** Test merge shows **CLEAN MERGE** - no conflicts between file transfer and EGFX branches.

### Pending IronRDP PRs (All by glamberson)

| PR | Branch | Purpose | Lines | Dependencies |
|----|--------|---------|-------|--------------|
| #1057 | egfx-server-complete | EGFX/H.264 support | +4,293 | None |
| #1063 | fix-server-reqwest-feature | Reqwest fix | +1 | None (already in our fork) |
| #1064 | pr1-clipboard-lock-unlock | lock/unlock clipboard | +25 | None |
| #1065 | pr2-request-file-contents | request_file_contents | +45 | #1064 |
| #1066 | pr3-file-contents-response | SendFileContentsResponse | +50 | #1065 |

### What PR #1057 Provides

New crate: `ironrdp-egfx` (4,293 lines total)

```
crates/ironrdp-egfx/
├── Cargo.toml                    (28 lines)
├── src/
│   ├── lib.rs                    (8 lines)
│   ├── client.rs                 (74 lines) - Client scaffolding
│   ├── server.rs                 (1,108 lines) - GraphicsPipelineServer
│   └── pdu/
│       ├── mod.rs                (24 lines)
│       ├── cmd.rs                (2,078 lines) - All 23 RDPGFX PDUs
│       ├── avc.rs                (546 lines) - AVC420/AVC444 codecs
│       └── common.rs             (129 lines) - Timestamp, QuantQuality, etc.
└── tests/                        (250 lines)
```

**Key APIs:**
- `GraphicsPipelineServer` - DvcProcessor implementation
- `GraphicsPipelineHandler` - Callback trait for events
- `send_avc420_frame()` - Queue H.264 frame for transmission
- `send_avc444_frame()` - Queue AVC444 frame (full chroma)
- `create_surface()` / `delete_surface()` - Surface lifecycle
- `supports_avc420()` / `supports_avc444()` - Capability queries

### lamco-rdp-server EGFX Module Status

**Location:** `src/egfx/` (1,801 lines)

| File | Lines | Purpose | After Integration |
|------|-------|---------|-------------------|
| `mod.rs` | 444 | Custom EgfxServer (DvcProcessor) | **DELETE** - replaced by ironrdp-egfx |
| `surface.rs` | 240 | Surface management | **DELETE** - replaced by ironrdp-egfx |
| `encoder.rs` | 665 | OpenH264 H.264 encoder | **KEEP** - ironrdp-egfx doesn't encode |
| `video_handler.rs` | 452 | PipeWire → H.264 bridge | **KEEP** - adapter layer |

---

## Integration Strategy

### Architecture After Integration

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        lamco-rdp-server                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  PipeWire ────► VideoFrame ────► EgfxVideoHandler ────► Avc420Encoder   │
│  (capture)      (BGRA)           (src/egfx/)            (OpenH264)       │
│                                        │                     │           │
│                                        │                H.264 NAL data   │
│                                        │                     │           │
│                                        ▼                     ▼           │
│                                  WrdGraphicsHandler                      │
│                                  (implements GraphicsPipelineHandler)    │
│                                        │                                 │
│                                        │ send_avc420_frame()             │
│                                        ▼                                 │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    ironrdp-egfx                                  │    │
│  │  GraphicsPipelineServer                                          │    │
│  │  - DvcProcessor implementation                                   │    │
│  │  - Surface management                                            │    │
│  │  - Frame tracking & flow control                                 │    │
│  │  - PDU encoding (StartFrame, WireToSurface1, EndFrame)          │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                        │                                 │
│                                        │ DVC messages                    │
│                                        ▼                                 │
│                              IronRDP Server Core                         │
│                                        │                                 │
│                                        ▼                                 │
│                                   RDP Client                             │
│                              (H.264 hardware decode)                     │
└─────────────────────────────────────────────────────────────────────────┘
```

### Division of Responsibilities

| Component | Responsibility |
|-----------|----------------|
| **PipeWire** | Frame capture from Wayland compositor |
| **EgfxVideoHandler** | Frame timing, rate control, encoder lifecycle |
| **Avc420Encoder** | BGRA → H.264 encoding via OpenH264 |
| **WrdGraphicsHandler** | Implements GraphicsPipelineHandler, coordinates surfaces |
| **ironrdp-egfx** | Protocol handling, PDU encoding, flow control |

---

## Implementation Steps

### Phase 1: IronRDP Fork Preparation (15 min)

1. **Create combined branch:**
   ```bash
   cd /home/greg/wayland/IronRDP
   git checkout pr3-file-contents-response-v2
   git checkout -b combined-egfx-file-transfer
   git merge fork/egfx-server-complete -m "feat: merge EGFX support with file transfer"
   ```

2. **Verify build:**
   ```bash
   cargo build --workspace
   cargo test -p ironrdp-egfx
   ```

3. **Update lamco-rdp-server to use new branch** (Cargo.toml patches already point to local path)

### Phase 2: Dependency Updates (10 min)

**File:** `Cargo.toml`

Add after other ironrdp dependencies:
```toml
ironrdp-egfx = { path = "/home/greg/wayland/IronRDP/crates/ironrdp-egfx" }
```

Add to patch section:
```toml
ironrdp-egfx = { path = "/home/greg/wayland/IronRDP/crates/ironrdp-egfx" }
```

### Phase 3: Refactor src/egfx/ (1-2 hours)

#### 3a. Delete Replaced Files

```bash
rm src/egfx/mod.rs      # Replaced by ironrdp-egfx::server::GraphicsPipelineServer
rm src/egfx/surface.rs  # Replaced by ironrdp-egfx::server::Surfaces
```

#### 3b. Create New mod.rs

**File:** `src/egfx/mod.rs` (new)

```rust
//! EGFX Integration
//!
//! Bridges PipeWire video capture to ironrdp-egfx for H.264 streaming.
//!
//! # Architecture
//!
//! - `encoder.rs` - OpenH264 H.264 encoding
//! - `video_handler.rs` - PipeWire → encoder bridge
//! - `handler.rs` - GraphicsPipelineHandler implementation

mod encoder;
mod handler;
mod video_handler;

pub use encoder::{Avc420Encoder, EncoderConfig, EncoderError, EncoderResult, H264Frame};
pub use handler::WrdGraphicsHandler;
pub use video_handler::{EgfxVideoConfig, EgfxVideoHandler, EncodedFrame, EncodingStats};
```

#### 3c. Create GraphicsPipelineHandler Implementation

**File:** `src/egfx/handler.rs` (new, ~150 lines)

```rust
//! WrdGraphicsHandler - Implements ironrdp_egfx::GraphicsPipelineHandler

use ironrdp_egfx::pdu::{Avc420Region, CapabilitiesAdvertisePdu, CapabilitySet, PixelFormat};
use ironrdp_egfx::server::{GraphicsPipelineHandler, GraphicsPipelineServer, QoeMetrics, Surface};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

use crate::egfx::{Avc420Encoder, EncoderConfig, EncodedFrame};

/// Handler for EGFX graphics pipeline events
pub struct WrdGraphicsHandler {
    /// Surface dimensions
    width: u16,
    height: u16,

    /// Encoder (created when ready)
    encoder: Option<Avc420Encoder>,

    /// Channel to send encoded frames for transmission
    frame_tx: mpsc::Sender<EncodedFrame>,

    /// Whether AVC420 was negotiated
    avc420_enabled: bool,
}

impl WrdGraphicsHandler {
    pub fn new(width: u16, height: u16, frame_tx: mpsc::Sender<EncodedFrame>) -> Self {
        Self {
            width,
            height,
            encoder: None,
            frame_tx,
            avc420_enabled: false,
        }
    }

    /// Get encoder reference (if initialized)
    pub fn encoder(&self) -> Option<&Avc420Encoder> {
        self.encoder.as_ref()
    }

    /// Get mutable encoder reference
    pub fn encoder_mut(&mut self) -> Option<&mut Avc420Encoder> {
        self.encoder.as_mut()
    }

    /// Check if H.264 encoding is available
    pub fn is_avc420_enabled(&self) -> bool {
        self.avc420_enabled
    }
}

impl GraphicsPipelineHandler for WrdGraphicsHandler {
    fn capabilities_advertise(&mut self, pdu: &CapabilitiesAdvertisePdu) {
        info!("Client advertised {} EGFX capability sets", pdu.0.len());
        for cap in &pdu.0 {
            debug!("  Capability: {:?}", cap);
        }
    }

    fn on_ready(&mut self, negotiated: &CapabilitySet) {
        info!("EGFX ready with capabilities: {:?}", negotiated);

        // Check for AVC420 support
        self.avc420_enabled = matches!(
            negotiated,
            CapabilitySet::V8_1 { flags, .. } if flags.contains(
                ironrdp_egfx::pdu::CapabilitiesV81Flags::AVC420_ENABLED
            )
        ) || matches!(
            negotiated,
            CapabilitySet::V10 { .. } |
            CapabilitySet::V10_1 { .. } |
            CapabilitySet::V10_2 { .. } |
            CapabilitySet::V10_3 { .. } |
            CapabilitySet::V10_4 { .. } |
            CapabilitySet::V10_5 { .. } |
            CapabilitySet::V10_6 { .. } |
            CapabilitySet::V10_7 { .. }
        );

        if self.avc420_enabled {
            info!("AVC420 (H.264) encoding enabled");

            // Initialize encoder
            match Avc420Encoder::new(EncoderConfig::default()) {
                Ok(encoder) => {
                    self.encoder = Some(encoder);
                    info!("OpenH264 encoder initialized");
                }
                Err(e) => {
                    warn!("Failed to initialize H.264 encoder: {:?}", e);
                    self.avc420_enabled = false;
                }
            }
        } else {
            info!("AVC420 not supported by client, falling back to RemoteFX");
        }
    }

    fn on_frame_ack(&mut self, frame_id: u32, queue_depth: u32) {
        debug!("Frame {} acknowledged, queue depth: {}", frame_id, queue_depth);
    }

    fn on_qoe_metrics(&mut self, metrics: QoeMetrics) {
        debug!("QoE metrics received: {:?}", metrics);
        // Could use this to adjust encoding quality dynamically
    }

    fn on_surface_created(&mut self, surface: &Surface) {
        debug!("Surface {} created: {}x{}", surface.id, surface.width, surface.height);
    }

    fn on_close(&mut self) {
        info!("EGFX channel closed");
        self.encoder = None;
    }

    fn max_frames_in_flight(&self) -> u32 {
        3 // Allow 3 frames in flight for smooth streaming
    }

    fn preferred_capabilities(&self) -> Vec<CapabilitySet> {
        // Prefer V8.1 with AVC420
        vec![CapabilitySet::V8_1 {
            flags: ironrdp_egfx::pdu::CapabilitiesV81Flags::AVC420_ENABLED,
        }]
    }
}
```

### Phase 4: Server Integration (1-2 hours)

#### 4a. Update Server Initialization

Locate where DVC processors are registered and add EGFX.

**File:** `src/server/mod.rs` or wherever RDP server is built

```rust
use ironrdp_egfx::server::GraphicsPipelineServer;
use crate::egfx::WrdGraphicsHandler;

// In server initialization:
let (frame_tx, frame_rx) = mpsc::channel(16);
let handler = WrdGraphicsHandler::new(width, height, frame_tx);
let egfx_server = GraphicsPipelineServer::new(Box::new(handler));

// Register with DVC manager
dvc_manager.register(egfx_server);
```

#### 4b. Wire Frame Pipeline

Create task that:
1. Receives VideoFrames from PipeWire
2. Encodes with Avc420Encoder
3. Calls `egfx_server.send_avc420_frame()`
4. Drains output and sends via DVC

```rust
async fn egfx_frame_loop(
    mut frame_rx: mpsc::Receiver<VideoFrame>,
    egfx_server: Arc<RwLock<GraphicsPipelineServer>>,
    handler: Arc<RwLock<WrdGraphicsHandler>>,
) {
    while let Some(frame) = frame_rx.recv().await {
        let mut handler = handler.write().await;
        let mut server = egfx_server.write().await;

        if !server.is_ready() || !server.supports_avc420() {
            continue;
        }

        // Encode frame
        if let Some(encoder) = handler.encoder_mut() {
            match encoder.encode_bgra(&frame.data, frame.width, frame.height, timestamp_ms) {
                Ok(Some(h264_frame)) => {
                    let region = Avc420Region::full_frame(
                        frame.width as u16,
                        frame.height as u16,
                        26, // QP
                    );

                    if let Some(frame_id) = server.send_avc420_frame(
                        surface_id,
                        &h264_frame.data,
                        &[region],
                        timestamp_ms as u32,
                    ) {
                        trace!("Queued frame {}", frame_id);
                    }
                }
                Ok(None) => {} // Frame skipped
                Err(e) => warn!("Encoding failed: {:?}", e),
            }
        }

        // Drain output messages to DVC
        for msg in server.drain_output() {
            dvc_channel.send(msg).await?;
        }
    }
}
```

### Phase 5: Update display_handler.rs (30 min)

Add EGFX path selection:

```rust
pub enum VideoEncoder {
    RemoteFx,  // Existing path via IronRDP's built-in encoding
    Egfx(Arc<RwLock<GraphicsPipelineServer>>),
}

impl WrdDisplayHandler {
    pub async fn new(
        /* existing params */,
        egfx_enabled: bool,
    ) -> Result<Self> {
        let encoder = if egfx_enabled && cfg!(feature = "h264") {
            VideoEncoder::Egfx(/* ... */)
        } else {
            VideoEncoder::RemoteFx
        };
        // ...
    }
}
```

### Phase 6: Testing (1 hour)

1. **Build with H.264:**
   ```bash
   cargo build --release --features h264
   ```

2. **Run with debug logging:**
   ```bash
   RUST_LOG=debug ./target/release/lamco-rdp-server
   ```

3. **Connect from Windows mstsc.exe**

4. **Verify in logs:**
   - "Client advertised X EGFX capability sets"
   - "EGFX ready with capabilities: V8_1"
   - "AVC420 (H.264) encoding enabled"
   - "OpenH264 encoder initialized"
   - "Queued frame N"
   - "Frame N acknowledged"

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Merge conflicts with future upstream | Low | Medium | Clean merge today; rebase when PRs merge |
| OpenH264 build issues on CI | Medium | Low | Feature-gated, fallback to RemoteFX |
| Client compatibility issues | Medium | Medium | Test with mstsc.exe and FreeRDP |
| Performance regression | Low | High | Profile before/after, use flame graphs |

---

## Success Criteria

- [ ] IronRDP fork has combined branch with EGFX + file transfer
- [ ] lamco-rdp-server builds with `--features h264`
- [ ] EGFX capability negotiation succeeds
- [ ] H.264 frames encode and transmit
- [ ] Client displays video via EGFX
- [ ] Fallback to RemoteFX works when EGFX unavailable
- [ ] No regression in existing clipboard/input functionality

---

## Alternative Approaches Considered

### A. Wait for Upstream Merge
**Rejected:** Could take weeks; blocks development.

### B. Maintain Separate Fork Branches
**Rejected:** Branch management complexity; risk of divergence.

### C. Duplicate EGFX Code in lamco-rdp-server
**Rejected:** 4,293 lines to maintain; defeats purpose of using IronRDP.

### D. Combined Branch (Selected)
**Chosen:** Single branch with both features; clean merge proven.

---

## Next Steps After This Plan

1. **Execute Phase 1** - Create combined IronRDP branch
2. **Execute Phases 2-4** - Integration code
3. **Execute Phase 5-6** - Testing
4. **Document in HANDOVER** - Update handover doc with results
5. **Consider PR cleanup** - Once upstream merges PRs, rebase to published crates

---

## Appendix: PR #1057 Key Commits

| Commit | Description |
|--------|-------------|
| `415ff81a` | Initial EGFX implementation (PDUs + server) |
| `b9fd1048` | Fix MapSurfaceToScaledWindowPdu size |
| `cc6eb7e8` | Simplify NAL parsing loop |
| `3cc22538` | Address PR review feedback |
| `7e269d09` | Code clarity improvements, rustdoc fixes |

---

**Document Version:** 1.1
**Last Updated:** 2025-12-24
**Changes:** Added Architecture Boundary Analysis section
