# lamco-rdp-server Development Decisions

**Document Date**: 2025-12-16
**Status**: Active Development
**Branch**: `feature/lamco-rdp-server-prep`

## Overview

This document captures all architectural decisions, work items, and implementation plans for completing the lamco-rdp-server product. The server has been refactored to consume published lamco-* crates, reducing local code by ~11,400 lines.

---

## Product Naming Decisions

| Product | Name | Status |
|---------|------|--------|
| Portal-mode RDP Server | `lamco-rdp-server` | **Decided** |
| VDI/Headless Server | TBD (lamco-vdi-server or lamco-headless-server) | Pending |

### Licensing Model
- **lamco-rdp-server**: Honor-system commercial licensing
- **lamco-vdi-server**: Separate licensing model TBD

---

## Repository Structure Decision

**Decision**: Private repository contains ONLY commercial/product code

```
lamco-rdp-server (private)
├── src/
│   ├── clipboard/    # Glue code: portal ↔ RDP clipboard
│   ├── config/       # Server configuration
│   ├── multimon/     # Multi-monitor orchestration (to be consolidated)
│   ├── protocol/     # Protocol-specific code
│   ├── rdp/          # RDP session management
│   ├── security/     # TLS, authentication
│   ├── server/       # Main server orchestration
│   └── utils/        # Server utilities
└── Cargo.toml        # Depends on public crates via crates.io
```

All reusable components are in published crates:
- lamco-portal, lamco-pipewire, lamco-video
- lamco-rdp-input, lamco-clipboard-core, lamco-rdp-clipboard
- lamco-wayland, lamco-rdp

---

## Critical Issues and Decisions

### Issue #1: ClipboardSink Implementation Gap

**Severity**: CRITICAL
**Decision**: Add PortalClipboardSink to lamco-portal as feature-gated module

**Problem**: `lamco-clipboard-core::ClipboardSink` trait exists but no implementation bridges `lamco-portal::ClipboardManager` to this trait.

**Implementation Plan**:

```rust
// In lamco-portal/src/clipboard_sink.rs (feature-gated: "clipboard-sink")

use lamco_clipboard_core::{ClipboardSink, ClipboardData, ClipboardFormat};
use crate::ClipboardManager;

pub struct PortalClipboardSink {
    manager: Arc<ClipboardManager>,
    session: Arc<Mutex<OwnedObjectPath>>,
}

impl PortalClipboardSink {
    pub fn new(
        manager: Arc<ClipboardManager>,
        session: Arc<Mutex<OwnedObjectPath>>,
    ) -> Self {
        Self { manager, session }
    }
}

#[async_trait]
impl ClipboardSink for PortalClipboardSink {
    async fn set_available_formats(&self, formats: &[ClipboardFormat]) -> Result<()> {
        // Convert to MIME types and call portal SetSelection
    }

    async fn get_data(&self, format: ClipboardFormat) -> Result<ClipboardData> {
        // Request data via portal SelectionRead
    }

    async fn set_data(&self, data: ClipboardData) -> Result<()> {
        // Write data via portal SelectionWrite
    }

    fn supports_format(&self, format: ClipboardFormat) -> bool {
        // Check MIME type mappings
    }
}
```

**Staging**: Add to lamco-portal in lamco-admin, publish with other updates.

---

### Issue #2: IronRDP Version Mismatch

**Severity**: HIGH
**Decision**: Use Cargo `[patch]` section (Option C)

**Problem**: Published lamco crates use IronRDP 0.5.0; we need git master for server features.

**Implementation**:

```toml
# In lamco-rdp-server/Cargo.toml

[patch.crates-io]
ironrdp = { git = "https://github.com/Devolutions/IronRDP", branch = "master" }
ironrdp-pdu = { git = "https://github.com/Devolutions/IronRDP", branch = "master" }
ironrdp-server = { git = "https://github.com/Devolutions/IronRDP", branch = "master" }
ironrdp-graphics = { git = "https://github.com/Devolutions/IronRDP", branch = "master" }
ironrdp-cliprdr = { git = "https://github.com/Devolutions/IronRDP", branch = "master" }
ironrdp-svc = { git = "https://github.com/Devolutions/IronRDP", branch = "master" }
ironrdp-dvc = { git = "https://github.com/Devolutions/IronRDP", branch = "master" }
```

**Rationale**: This transparently overrides all IronRDP dependencies (including transitive ones from lamco crates) to use git master, ensuring consistent versions across the entire dependency tree.

---

### Issue #3: PortalConfig Mapping

**Severity**: MEDIUM
**Decision**: Create proper mapping from server Config to PortalConfig

**Problem**: Server currently uses `PortalConfig::default()`, ignoring user configuration.

**Implementation Location**: `src/server/mod.rs`

```rust
fn map_portal_config(config: &Config) -> lamco_portal::PortalConfig {
    lamco_portal::PortalConfigBuilder::default()
        .source_types(config.capture.source_types.clone())
        .cursor_mode(config.capture.cursor_mode)
        .persist_mode(config.capture.persist_mode)
        .multiple_streams(config.capture.multiple_streams)
        // ... map all relevant settings
        .build()
        .expect("Valid portal config")
}
```

**Work Item**: Audit Config struct, ensure all portal-relevant settings are mapped.

---

### Issue #4: Session Ownership Pattern

**Severity**: MEDIUM
**Decision**: Keep session in PortalManager, provide access methods

**Problem**: `PortalSessionHandle.session` is currently consumed and wrapped in `Arc<Mutex<>>` for sharing.

**Recommendation**: Modify lamco-portal to:
1. Keep session ownership in PortalManager
2. Provide `session_path(&self) -> &OwnedObjectPath` accessor
3. Provide input/clipboard methods that use internal session reference

This avoids the `Arc<Mutex<>>` wrapping pattern in the server and provides cleaner API.

**Staging**: Update lamco-portal API in lamco-admin.

---

### Issue #5: Multi-Monitor Consolidation

**Severity**: LOW
**Decision**: Consolidate into `lamco-rdp-input::multimon`

**Problem**: Multi-monitor logic split between:
- `lamco-rdp-input::CoordinateTransformer` (coordinate mapping)
- Server's `src/multimon/` (monitor enumeration, spanning modes)

**Implementation Plan**:
1. Move monitor enumeration to lamco-rdp-input
2. Add spanning mode support to CoordinateTransformer
3. Server's multimon/ becomes thin wrapper or is removed

**Staging**: Update lamco-rdp-input in lamco-admin.

---

### Issue #6: Clipboard Lifecycle Edge Cases

**Severity**: LOW
**Decision**: No changes required for now

The identified edge cases (selection owner changes, format negotiation timing) are documented but not blocking. Will address if issues arise in testing.

---

## Missing Features

### H.264/EGFX Support

**Priority**: HIGH
**Decision**: Implement as RDP protocol feature using OpenH264

**User Perspective**: H.264 is viewed as "Microsoft RDP protocol implementation" rather than generic video codec. This frames it as protocol completeness rather than video feature.

#### MS-RDPEGFX Protocol Specification (Strict Implementation)

**Reference**: [MS-RDPEGFX](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/da5c75f9-cd99-450c-98c4-014a496942b0)

##### Codec IDs (per spec)
| Codec | ID | Mode | Notes |
|-------|-----|------|-------|
| `RDPGFX_CODECID_AVC420` | 0x000B | YUV420p | H.264 Baseline/Main profile |
| `RDPGFX_CODECID_AVC444` | 0x000E | YUV444 | H.264 High profile |
| `RDPGFX_CODECID_AVC444v2` | 0x000F | YUV444v2 | Enhanced variant |

##### Capability Flags
| Flag | Value | Version | Purpose |
|------|-------|---------|---------|
| `RDPGFX_CAPS_FLAG_AVC420_ENABLED` | 0x00000010 | 8.1+ | Enable YUV420 H.264 |
| `RDPGFX_CAPS_FLAG_AVC_DISABLED` | 0x00000020 | 10.0+ | Disable all AVC |
| `RDPGFX_CAPS_FLAG_AVC_THINCLIENT` | 0x00000040 | 10.3+ | Thin client mode |

##### Pixel Formats
- `GFX_PIXEL_FORMAT_XRGB_8888` (0x20): 32-bit RGB, no alpha
- `GFX_PIXEL_FORMAT_ARGB_8888` (0x21): 32-bit ARGB with alpha

##### PDU Structure for H.264

**RFX_AVC420_METABLOCK** (informational, per spec section 2.2.4.4):
- `numRegionRects` (u32): Count of regions
- `regionRects[]`: Array of `RDPGFX_RECT16` (8 bytes each)
- `quantQualityVals[]`: Array of `RDPGFX_AVC420_QUANT_QUALITY` (2 bytes each)
  - bits 0-5: qp (quantization parameter, 0-51 per ITU-H.264)
  - bit 7: progressive flag
  - byte 2: quality level

**RDPGFX_WIRE_TO_SURFACE_PDU_2** (for H.264 data):
- `surfaceId` (u16): Target surface
- `codecId` (u16): 0x000B for AVC420
- `codecContextId` (u32): Encoder context ID
- `pixelFormat` (u8): 0x20 for XRGB_8888
- `bitmapDataLength` (u32): Length of encoded data
- `bitmapData[]`: H.264 NAL units wrapped in AVC420 metablock

##### EGFX Channel Flow
```
1. Client → CapabilitiesAdvertisePdu (flags include AVC420_ENABLED)
2. Server → CapabilitiesConfirmPdu (select AVC420 codec)
3. Server → CreateSurfacePdu (create output surface)
4. Server → MapSurfaceToOutputPdu (map to monitor)
5. For each frame:
   Server → StartFramePdu (frame_id)
   Server → WireToSurface2Pdu (H.264 data)
   Server → EndFramePdu (frame_id)
   Client → FrameAcknowledgePdu (frame_id)
```

#### Current State in IronRDP (2025-12-16)

IronRDP provides:
- ✅ EGFX PDU types: `Avc420BitmapStream`, `Avc444BitmapStream`, `WireToSurface2Pdu`
- ✅ GFX capability sets: `CapabilitiesV8Flags`, `CapabilitiesV10Flags`, etc.
- ✅ Server PDU encoding for GFX channel (in `ironrdp-pdu/src/rdp/vc/dvc/gfx/`)
- ❌ **No H.264 encoder implementation** in `ironrdp-server`
- ❌ No EGFX channel handler in server (only client side in `ironrdp-glutin-renderer`)
- ❌ `BitmapUpdateHandler` trait doesn't support EGFX (different channel architecture)

Current `BitmapUpdater` enum in `ironrdp-server/src/encoder/mod.rs`:
- `None` (raw)
- `Bitmap` (RLE)
- `RemoteFx` (RFX codec)
- `Qoi` / `Qoiz` (feature-gated, non-standard)

**Key Finding**: EGFX uses Dynamic Virtual Channels (DVC), not the bitmap update pathway. H.264 support requires implementing a separate EGFX channel handler, not just adding another codec to `BitmapUpdater`.

#### Implementation Plan

**Architecture Decision**: EGFX requires a **separate DVC channel handler**, not integration with `BitmapUpdater`.

**Option A: Contribute EGFX Server to IronRDP** (Recommended)
1. Create `ironrdp-egfx-server` module (parallel to existing client side)
2. Implement `EgfxServerHandler` trait for DVC channel
3. Add `openh264` optional dependency for AVC encoding
4. Implement capability negotiation (CapabilitiesAdvertise → CapabilitiesConfirm)
5. Implement surface management (CreateSurface, MapSurfaceToOutput)
6. Implement frame encoding loop (StartFrame, WireToSurface2, EndFrame)
7. Handle FrameAcknowledge for flow control

**Option B: Implement as lamco-rdp-egfx crate** (Alternative)
1. Create `lamco-rdp-egfx` crate with EGFX channel implementation
2. Reuse `ironrdp-pdu::rdp::vc::dvc::gfx::*` PDU types
3. Create `H264Encoder` trait in lamco-video with OpenH264 backend
4. Wire into server's video pipeline as separate output path
5. Coordinate with existing bitmap update path for fallback

**Key Implementation Components**:
```rust
// EGFX Channel Handler (new component needed)
trait EgfxHandler {
    /// Handle client capabilities advertisement
    async fn handle_capabilities_advertise(&mut self, caps: CapabilitiesAdvertisePdu)
        -> Result<CapabilitiesConfirmPdu>;

    /// Handle frame acknowledgment (flow control)
    async fn handle_frame_ack(&mut self, ack: FrameAcknowledgePdu) -> Result<()>;
}

// Encoder integration
struct Avc420Encoder {
    encoder: openh264::Encoder,
    context_id: u32,
    surface_id: u16,
}

impl Avc420Encoder {
    fn encode_frame(&mut self, frame: &YuvFrame) -> Result<Vec<u8>>;
    fn create_wire_to_surface_pdu(&self, data: Vec<u8>, rect: Rect) -> WireToSurface2Pdu;
}
```

#### Technical Details

**H.264 Encoding Flow:**
```
PipeWire Frame (BGRA/NV12)
    → Color Space Conversion (BGRA → YUV420)
    → OpenH264 Encoder
    → NAL Units (H.264 bitstream)
    → Avc420BitmapStream PDU wrapper
    → WireToSurface2Pdu
    → EGFX DVC Channel
    → RDP Client
```

**OpenH264 Rust Integration:**

Crate: [`openh264`](https://crates.io/crates/openh264) (v0.9+)

```rust
use openh264::encoder::Encoder;

// Create encoder
let mut encoder = Encoder::new()?;

// Encode YUV frame to H.264 NAL units
let bitstream = encoder.encode(&yuv)?;
```

Key features:
- BSD license (Cisco OpenH264), bundles source code
- Supports YUV420 input format (matches RDP AVC420 mode)
- Dynamic resolution support (v0.6+)
- **Performance**: `nasm` in PATH provides ~3x speedup
- **Security**: Use v0.6.6+ (CVE-2025-27091 patched)

Alternative: [`shiguredo_openh264`](https://crates.io/crates/shiguredo_openh264) - another Rust wrapper

**Color Space Conversion:**
```rust
// PipeWire delivers BGRA, need YUV420 for H.264
fn bgra_to_yuv420(bgra: &[u8], width: u32, height: u32) -> YuvBuffer {
    // Standard BT.601 or BT.709 conversion
    // Y = 0.299*R + 0.587*G + 0.114*B
    // U = -0.169*R - 0.331*G + 0.500*B + 128
    // V = 0.500*R - 0.419*G - 0.081*B + 128
}
```

Consider existing implementations:
- `lamco-video` already has frame conversion utilities
- Could add YUV420 output format option

**EGFX Channel:**
- Dynamic Virtual Channel (DVC)
- Capability negotiation via `CapabilitiesAdvertisePdu` / `CapabilitiesConfirmPdu`
- Frame markers: `StartFramePdu` / `EndFramePdu`
- Surface management: `CreateSurfacePdu`, `MapSurfaceToOutputPdu`

**Capability Flags:**
```rust
// For AVC420 (YUV420 H.264)
CapabilitiesV81Flags::AVC420_ENABLED
// For AVC444 (YUV444 H.264, higher quality)
CapabilitiesV104Flags::AVC_DISABLED_UNUSED // Clear this to enable
```

#### Work Items (Updated)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | ✅ Research MS-RDPEGFX specification | - | Complete |
| 2 | Design EGFX DVC channel handler | TBD | **Decision needed: IronRDP or lamco-rdp-egfx** |
| 3 | Add OpenH264 to lamco-video (feature-gated) | lamco-video | Pending |
| 4 | Add BGRA→YUV420 conversion to lamco-video | lamco-video | Pending |
| 5 | Implement capability negotiation | EGFX crate | Pending |
| 6 | Implement surface management (Create/Map) | EGFX crate | Pending |
| 7 | Implement frame encoding loop | EGFX crate | Pending |
| 8 | Wire EGFX channel into lamco-rdp-server | server | Pending |
| 9 | Test with mstsc.exe (Windows client) | - | Pending |
| 10 | Test with FreeRDP (Linux client) | - | Pending |

**Decision Needed**: Where to implement EGFX server channel?
- **Option A**: Contribute to IronRDP upstream (cleaner, benefits ecosystem)
- **Option B**: Create lamco-rdp-egfx crate (faster iteration, full control)

### Audio Support

**Priority**: MEDIUM
**Status**: Deferred

PipeWire audio capture exists but RDP audio channel not implemented.

### File Transfer

**Priority**: MEDIUM
**Status**: Believed complete in crates

User believes full file transfer implementation exists. **Verify during crate review.**

---

## Work Item Summary

### Phase 1: Crate Updates (Stage in lamco-admin)

| Crate | Change | Priority | Status |
|-------|--------|----------|--------|
| lamco-portal | Add PortalClipboardSink (feature-gated) | CRITICAL | ✅ Done |
| lamco-portal | Add session accessor methods | MEDIUM | ✅ Done (already public) |
| lamco-rdp-input | Consolidate multi-monitor logic | LOW | Deferred |
| lamco-video | Add OpenH264 encoder (feature-gated) | HIGH | Pending |
| lamco-video | Add BGRA→YUV420 conversion | HIGH | Pending |
| NEW: lamco-rdp-egfx **or** ironrdp-egfx-server | EGFX DVC channel | HIGH | Design pending |

#### Completed: PortalClipboardSink (2025-12-16)

Added `clipboard_sink.rs` to lamco-portal with:
- `PortalClipboardSink` implementing `lamco_clipboard_core::ClipboardSink`
- Feature-gated behind `clipboard-sink` feature
- Bridges ClipboardManager + Portal session to abstract clipboard interface
- Handles announce_formats, read_clipboard, subscribe_changes
- File transfer methods stubbed (Portal uses drag-and-drop)

Usage:
```toml
[dependencies]
lamco-portal = { version = "0.1.1", features = ["clipboard-sink"] }
```

### Phase 2: Server Updates

| File | Change | Priority |
|------|--------|----------|
| Cargo.toml | Add `[patch.crates-io]` for IronRDP | HIGH |
| src/server/mod.rs | Add PortalConfig mapping | MEDIUM |
| src/clipboard/ | Use PortalClipboardSink when available | CRITICAL |
| src/multimon/ | Thin wrapper or remove after input consolidation | LOW |

### Phase 3: Verification

- [ ] Verify file transfer implementation completeness
- [ ] Test clipboard with new PortalClipboardSink
- [ ] Test multi-monitor scenarios
- [ ] Performance testing with H.264

---

## Staging Plan for lamco-admin

Updates should be batched in lamco-admin before publishing:

```
lamco-admin/
├── lamco-portal/       # Add PortalClipboardSink, session accessors
├── lamco-rdp-input/    # Add multi-monitor consolidation
├── lamco-video/        # Add H.264/OpenH264
└── lamco-rdp/          # Add EGFX channel (if needed)
```

**Batch Release Strategy**:
1. Make all changes in lamco-admin
2. Test integration locally
3. Publish updated crates together
4. Update lamco-rdp-server dependencies

---

## Technical Notes

### Current Server Architecture

```
lamco-rdp-server
├── Portal Subsystem (lamco-portal)
│   ├── ScreenCast (video capture permissions)
│   ├── RemoteDesktop (input injection)
│   └── Clipboard (needs ClipboardSink impl)
├── PipeWire Subsystem (lamco-pipewire)
│   └── Video frame capture
├── Video Processing (lamco-video)
│   └── Frame conversion, RemoteFX encoding
├── Input Handling (lamco-rdp-input)
│   ├── Keyboard translation
│   ├── Mouse coordinate transformation
│   └── Multi-monitor support
├── Clipboard (lamco-clipboard-core + lamco-rdp-clipboard)
│   ├── Format conversion
│   ├── Loop detection
│   └── RDP CLIPRDR channel
└── RDP Protocol (ironrdp-server)
    └── TLS, capabilities, channels
```

### Data Flow Paths

**Video**: Portal → PipeWire → lamco-video → IronRDP → Client
**Input**: Client → IronRDP → lamco-rdp-input → lamco-portal → Compositor
**Clipboard**: Client ↔ lamco-rdp-clipboard ↔ lamco-clipboard-core ↔ **PortalClipboardSink** ↔ lamco-portal ↔ Compositor

---

## References

- SESSION-HANDOVER-2025-12-16.md - Session context
- docs/strategy/STRATEGIC-FRAMEWORK.md - Product strategy
- docs/strategy/CRATE-BREAKDOWN-AND-OPEN-SOURCE-STRATEGY.md - Crate architecture
- Commit: "refactor: Migrate to published lamco-* crates" (55 files, -11,397 lines)
