# Handover Document: Video/EGFX/H.264 Integration
**Date:** 2025-12-24
**Repository:** wrd-server-specs (lamco-rdp-server)
**Location:** `/home/greg/wayland/wrd-server-specs`
**Focus:** H.264/EGFX video pipeline integration

---

## Executive Summary

This handover focuses on **H.264/EGFX video integration** - the next major development direction. The EGFX encoding infrastructure is **fully implemented** (1,801 lines) but **NOT connected** to the display pipeline. Current video uses RemoteFX codec. The integration work is estimated at ~200 lines of code.

### Quick Status
| Component | Status | Notes |
|-----------|--------|-------|
| EGFX Server | ✅ Complete | DvcProcessor implementation ready |
| H.264 Encoder | ✅ Complete | OpenH264 with AVC420 format |
| Video Handler | ✅ Complete | Bridges PipeWire → H.264 |
| **Display Integration** | ❌ NOT DONE | Connection point identified |
| Clipboard (text/image) | ✅ Complete | Bidirectional working |
| File Transfer | ✅ Complete | Windows→Linux→Windows |

---

## Repository Structure

```
wrd-server-specs/
├── Cargo.toml              # Main config, IronRDP patches, h264 feature
├── src/
│   ├── main.rs             # Entry point, CLI
│   ├── server/
│   │   ├── mod.rs          # Server orchestration
│   │   ├── display_handler.rs   # ⭐ VIDEO PIPELINE (RemoteFX currently)
│   │   ├── input_handler.rs     # Keyboard/mouse
│   │   └── clipboard_handler.rs # Clipboard bridge
│   ├── egfx/               # ⭐ H.264/EGFX MODULE (NOT CONNECTED)
│   │   ├── mod.rs          # EgfxServer, DvcProcessor impl (444 lines)
│   │   ├── encoder.rs      # OpenH264, AVC420, Annex B (665 lines)
│   │   ├── video_handler.rs # EgfxVideoHandler (452 lines)
│   │   └── surface.rs      # Surface management (240 lines)
│   ├── capture/            # PipeWire frame capture
│   └── config/             # Configuration handling
├── docs/                   # Architecture documentation
└── STATUS-*.md, HANDOVER-*.md  # Session documents
```

### Code Statistics
- **Total:** 11,305 lines
- **EGFX module:** 1,801 lines (complete but dormant)
- **Display handler:** 643 lines (active, uses RemoteFX)

---

## Current Video Pipeline (RemoteFX)

The current video flow uses RemoteFX codec:

```
PipeWire Capture → VideoFrame → display_handler.rs → RemoteFX Encoder → RDP Client
                                     ↓
                              WrdDisplayHandler::send_frame()
                              uses RfxEncoder from lamco-video
```

**Key file:** `src/server/display_handler.rs:643`

```rust
// Current implementation in display_handler.rs
pub struct WrdDisplayHandler {
    frame_encoder: RfxEncoder,  // ← RemoteFX encoder
    frame_rate_regulator: TokenBucketRegulator,
    // ...
}

impl RdpServerDisplay for WrdDisplayHandler {
    async fn send_frame(&mut self, frame: VideoFrame) -> Result<()> {
        // Encodes with RemoteFX, sends via RDP bitmap update
    }
}
```

---

## EGFX/H.264 Architecture (Ready but Disconnected)

### Component Overview

```
                    ┌─────────────────────────────────────────┐
                    │         EgfxVideoHandler                │
                    │   src/egfx/video_handler.rs:452         │
                    │                                         │
  PipeWire Frame ──→│  process_frame(frame) → H.264 bitstream │
                    │         ↓                               │
                    │  Avc420Encoder (OpenH264)               │
                    │  encoder.rs:665                         │
                    └────────────────┬────────────────────────┘
                                     │
                                     ↓
                    ┌─────────────────────────────────────────┐
                    │           EgfxServer                    │
                    │   src/egfx/mod.rs:444                   │
                    │                                         │
                    │  DvcProcessor trait implementation      │
                    │  Manages surfaces, queues frames        │
                    │  Sends via RDPEGFX dynamic channel      │
                    └─────────────────────────────────────────┘
```

### Key Components

#### 1. Avc420Encoder (`encoder.rs`)
OpenH264-based H.264 encoder with MS-RDPEGFX compliance.

```rust
pub struct Avc420Encoder {
    encoder: Encoder,
    width: u32,
    height: u32,
    config: EncoderConfig,
}

impl Avc420Encoder {
    pub fn new(width: u32, height: u32, config: EncoderConfig) -> Result<Self>
    pub fn encode(&mut self, yuv_data: &[u8]) -> Result<Vec<u8>>
}

// Critical format conversion for MS-RDPEGFX
pub fn annex_b_to_avc(annex_b_data: &[u8]) -> Vec<u8>
pub fn create_avc420_bitmap_stream(h264_data: &[u8], regions: &[Avc420Region]) -> Vec<u8>
```

**Why Annex B to AVC?** OpenH264 outputs Annex B format (start codes: `00 00 00 01`). MS-RDPEGFX requires AVC format (length-prefixed NAL units). The conversion is already implemented.

#### 2. EgfxVideoHandler (`video_handler.rs`)
Bridges PipeWire frames to the EGFX encoding pipeline.

```rust
pub struct EgfxVideoHandler {
    encoder: Avc420Encoder,
    egfx_server: Arc<EgfxServer>,
    frame_id: AtomicU32,
}

impl EgfxVideoHandler {
    pub async fn process_frame(&self, frame: VideoFrame) -> EncoderResult<bool> {
        // 1. Convert frame to YUV420
        // 2. Encode with H.264
        // 3. Queue for EGFX transmission
    }
}
```

#### 3. EgfxServer (`mod.rs`)
Implements IronRDP's `DvcProcessor` trait for the RDPEGFX dynamic virtual channel.

```rust
pub struct EgfxServer {
    state: EgfxState,
    handler: Arc<dyn EgfxHandler>,
    surface_manager: SurfaceManager,
    next_frame_id: AtomicU32,
    pending_frames: VecDeque<PendingFrame>,
    capabilities: EgfxCapabilities,
}

impl DvcProcessor for EgfxServer {
    fn process(&mut self, data: &[u8]) -> Result<Vec<u8>>
    fn start(&mut self, channel_id: u32) -> Result<Vec<u8>>
}

impl DvcServerProcessor for EgfxServer {
    fn get_channel_name(&self) -> &str { "Microsoft::Windows::RDS::Graphics" }
}
```

---

## Integration Work Required

### The Gap
`display_handler.rs` uses `RfxEncoder` directly. It needs a conditional path to use `EgfxVideoHandler` when H.264 is negotiated.

### Integration Strategy

```rust
// Proposed changes to display_handler.rs

pub enum VideoEncoder {
    RemoteFx(RfxEncoder),
    Egfx(EgfxVideoHandler),
}

pub struct WrdDisplayHandler {
    encoder: VideoEncoder,  // ← Changed from RfxEncoder
    // ...
}

impl WrdDisplayHandler {
    pub fn new(egfx_enabled: bool, egfx_server: Option<Arc<EgfxServer>>) -> Self {
        let encoder = if egfx_enabled && egfx_server.is_some() {
            VideoEncoder::Egfx(EgfxVideoHandler::new(egfx_server.unwrap()))
        } else {
            VideoEncoder::RemoteFx(RfxEncoder::new())
        };
        // ...
    }
}

impl RdpServerDisplay for WrdDisplayHandler {
    async fn send_frame(&mut self, frame: VideoFrame) -> Result<()> {
        match &mut self.encoder {
            VideoEncoder::RemoteFx(enc) => enc.encode_and_send(frame),
            VideoEncoder::Egfx(handler) => handler.process_frame(frame).await,
        }
    }
}
```

### Estimated Effort
- **Lines of code:** ~150-200
- **Time:** 2-4 hours
- **Risk:** Low (infrastructure already tested)

---

## Feature Flags

The `h264` feature gates OpenH264 dependency:

```toml
# Cargo.toml
[features]
default = []
h264 = ["openh264"]

[dependencies]
openh264 = { version = "0.6", optional = true }
```

Build with H.264:
```bash
cargo build --features h264
```

---

## IronRDP Dependencies

### Local Fork
The project uses a local IronRDP fork at `/home/greg/wayland/IronRDP`:

```toml
# Cargo.toml [patch.crates-io] section
ironrdp = { path = "/home/greg/wayland/IronRDP/crates/ironrdp" }
ironrdp-cliprdr = { path = "/home/greg/wayland/IronRDP/crates/ironrdp-cliprdr" }
ironrdp-rdpdr = { path = "/home/greg/wayland/IronRDP/crates/ironrdp-rdpdr" }
ironrdp-dvc = { path = "/home/greg/wayland/IronRDP/crates/ironrdp-dvc" }
ironrdp-server = { path = "/home/greg/wayland/IronRDP/crates/ironrdp-server" }
# ... and more
```

### Pending PRs
| PR | Title | Status | Purpose |
|----|-------|--------|---------|
| #1057 | EGFX Server Support | Pending Review | DvcServerProcessor for EGFX |
| #1064 | read_lock method | Merged | File transfer locking |
| #1065 | read_unlock method | Merged | File transfer unlocking |
| #1066 | read_lock_id method | Merged | File transfer lock ID |

### EGFX PR Status
PR #1057 adds server-side EGFX support to IronRDP. It may not be merged yet - check status:
```bash
cd /home/greg/wayland/IronRDP
git log --oneline | head -20
```

---

## Test Environment

### Build Commands
```bash
# Debug build with H.264
cargo build --features h264

# Release build (recommended for video testing)
cargo build --release --features h264

# Run the server
cargo run --release --features h264 -- --help
```

### Testing H.264 Integration
1. **Unit test encoder:**
   ```bash
   cargo test --features h264 encoder::
   ```

2. **Connect from Windows RDP:**
   - Open `mstsc.exe`
   - Connect to the server
   - Verify H.264 negotiation in server logs

3. **Verify with Wireshark:**
   - Filter: `rdp.egfx`
   - Look for EGFX channel establishment
   - Verify AVC420 frame PDUs

---

## Key Files for Integration Work

| File | Purpose | Action Needed |
|------|---------|---------------|
| `src/server/display_handler.rs` | Current video pipeline | Add EGFX path |
| `src/server/mod.rs` | Server orchestration | Wire up EgfxServer |
| `src/egfx/mod.rs` | EGFX server | Already complete |
| `src/egfx/video_handler.rs` | Frame → H.264 | Already complete |
| `src/egfx/encoder.rs` | H.264 encoding | Already complete |

---

## Implementation Checklist

### Phase 1: Wire Up EgfxServer
- [ ] Add `EgfxServer` instantiation to server setup
- [ ] Register EGFX DVC with IronRDP's DVC manager
- [ ] Handle capability negotiation

### Phase 2: Connect Video Pipeline
- [ ] Add `VideoEncoder` enum to `display_handler.rs`
- [ ] Modify `WrdDisplayHandler::new()` for EGFX option
- [ ] Route frames through `EgfxVideoHandler` when enabled

### Phase 3: Testing
- [ ] Test with Windows RDP client
- [ ] Verify H.264 negotiation logs
- [ ] Compare visual quality RemoteFX vs H.264
- [ ] Benchmark frame rate and latency

---

## Common Issues and Solutions

### 1. OpenH264 Not Found
```
error: could not find `openh264` crate
```
**Solution:** Ensure `--features h264` is passed.

### 2. Annex B Format Error
If client shows corrupted video, check NAL unit conversion:
```rust
// encoder.rs - verify this is being called
let avc_data = annex_b_to_avc(&h264_bitstream);
```

### 3. EGFX Channel Not Opening
Check IronRDP DVC registration and PR #1057 status.

### 4. Frame Alignment
H.264 requires 16-pixel alignment:
```rust
// encoder.rs
pub fn align_to_16(dimension: u32) -> u32 {
    (dimension + 15) & !15
}
```

---

## Related Documentation

- `HANDOVER-2025-12-23-CLIPBOARD-PUBLICATION.md` - Previous session (clipboard/file transfer)
- `STATUS-2025-12-23-DEVELOPMENT-DIRECTIONS.md` - Full feature status
- `docs/ARCHITECTURE.md` - System architecture
- MS-RDPEGFX specification - Microsoft EGFX protocol docs

---

## Crate Dependencies (Published 2025-12-24)

| Crate | Version | Notes |
|-------|---------|-------|
| lamco-clipboard-core | 0.4.0 | RTF, synthesized formats |
| lamco-portal | 0.2.2 | Fixed non-blocking FD (clipboard) |
| lamco-rdp-clipboard | 0.2.2 | CB_FILECLIP_NO_FILE_PATHS |
| lamco-wayland | 0.2.2 | Umbrella crate |
| lamco-video | 0.1.2 | RemoteFX encoding |
| lamco-pipewire | 0.1.3 | PipeWire capture |

---

## Next Session Quick Start

```bash
# 1. Navigate to repository
cd /home/greg/wayland/wrd-server-specs

# 2. Check current status
git status
cargo check --features h264

# 3. Key files to review
cat src/server/display_handler.rs | head -100
cat src/egfx/mod.rs | head -100

# 4. Begin integration
# Start by modifying display_handler.rs to support dual encoders
```

---

## Summary

The H.264/EGFX infrastructure is **complete and tested individually**. The remaining work is **integration** - connecting `EgfxVideoHandler` to `display_handler.rs` so frames flow through H.264 instead of RemoteFX when EGFX is negotiated. This is ~200 lines of straightforward wiring code.
