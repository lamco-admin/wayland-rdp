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

**Implementation**:
- Use OpenH264 for encoding (BSD license, runtime download)
- Implement EGFX (Extended Graphics Pipeline) channel
- Add to lamco-video or create lamco-rdp-egfx crate

**Work Items**:
1. Add OpenH264 integration to lamco-video
2. Implement RDP EGFX channel in lamco-rdp
3. Wire up in lamco-rdp-server

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

| Crate | Change | Priority |
|-------|--------|----------|
| lamco-portal | Add PortalClipboardSink (feature-gated) | CRITICAL |
| lamco-portal | Add session accessor methods | MEDIUM |
| lamco-rdp-input | Consolidate multi-monitor logic | LOW |
| lamco-video | Add OpenH264/H.264 support | HIGH |

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
