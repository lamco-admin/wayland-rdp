# CRATE BREAKDOWN AND OPEN SOURCE STRATEGY
## Granular Analysis for Product Decisions
**Date:** 2025-12-10
**Purpose:** Define crate structure and open source boundaries

---

## EXECUTIVE SUMMARY

This document analyzes the current monolithic `wrd-server` codebase (60 Rust files, ~21,500 lines) and proposes breaking it into **10-12 granular crates**. Each crate is evaluated for:
- **Reusability** (can others use this?)
- **Open Source Value** (community benefit)
- **Proprietary Value** (commercial differentiation)
- **IronRDP Relationship** (dependency analysis)

**Recommendation:** Create a **core open source foundation** (5-6 crates) with **proprietary orchestration** layers.

---

## CURRENT MONOLITHIC STRUCTURE

### Source Organization (as-is)

```
wrd-server (current monolith)
├── clipboard/      8 files, ~7,826 lines
├── input/          7 files, ~3,717 lines
├── pipewire/      11 files, ~3,700 lines
├── server/         6 files, ~2,100 lines
├── video/          5 files, ~1,500 lines
├── portal/         5 files, ~600 lines
├── multimon/       3 files, ~400 lines
├── security/       4 files, ~300 lines
├── utils/          4 files, ~600 lines
├── config/         2 files, ~150 lines
├── rdp/            2 files, ~100 lines
├── protocol/       1 file, ~50 lines
├── lib.rs          1 file, 20 lines
└── main.rs         1 file, ~150 lines
```

**Total:** 60 files, ~21,506 lines

---

## PROPOSED CRATE STRUCTURE

### Overview

```
Lamco RDP Server Product Family
├── Open Source Foundation (MIT/Apache-2.0)
│   ├── lamco-rdp-clipboard        # RDP clipboard protocol
│   ├── lamco-rdp-input            # RDP input translation
│   ├── lamco-portal-integration   # XDG Portal bindings
│   ├── lamco-pipewire-capture     # PipeWire screen capture
│   ├── lamco-video-pipeline       # Video processing
│   └── lamco-rdp-protocol         # RDP protocol extensions
│
└── Proprietary Components (Commercial License)
    ├── lamco-rdp-server           # Portal mode orchestration
    ├── lamco-rdp-compositor       # Headless VDI mode
    ├── lamco-rdp-multimon         # Multi-monitor management
    └── lamco-rdp-security         # Enterprise security features
```

---

## DETAILED CRATE ANALYSIS

### CRATE 1: `lamco-rdp-clipboard` ⭐ HIGH REUSE

**Status:** ✅ **RECOMMEND OPEN SOURCE** (MIT/Apache-2.0)

**Current Location:** `src/clipboard/` (8 files, 7,826 lines)

**Purpose:**
Complete bidirectional RDP clipboard implementation (text, images, files) with loop prevention, format conversion, and MS-RDPECLIP protocol compliance.

**Files:**
- `manager.rs` (1,649 lines) - Central state machine
- `formats.rs` (948 lines) - RDP↔MIME format conversion
- `sync.rs` (769 lines) - Loop prevention, ownership tracking
- `transfer.rs` (601 lines) - Chunked transfer (large files)
- `ironrdp_backend.rs` (~300 lines) - IronRDP trait impl
- `dbus_bridge.rs` (~250 lines) - D-Bus communication
- `error.rs` (473 lines) - Error types
- `mod.rs` (~200 lines) - Public API

**Dependencies:**
- External: `ironrdp-cliprdr`, `ashpd`, `zbus`, `image`, `sha2`
- Internal: None (fully self-contained!)

**Key Features:**
- Content hash deduplication
- 3-layer duplicate detection (time, pending, content)
- Format conversion (UTF-8↔UTF-16, PNG↔JPEG, HTML, RTF)
- Clipboard ownership state machine
- MS-RDPECLIP FileContents protocol (file transfer)

**Reusability:** ⭐⭐⭐⭐⭐ **EXTREMELY HIGH**
- Any Rust RDP server needs this
- FreeRDP/xrdp developers could use reference
- Wayland clipboard integration useful beyond RDP

**Open Source Value:** ⭐⭐⭐⭐⭐ **EXTREMELY HIGH**
- Solves hard problem (loop prevention, format conversion)
- Reference implementation of MS-RDPECLIP
- Community would contribute improvements
- Builds trust and credibility

**Proprietary Value:** ⭐ **LOW**
- Not a differentiator (expected feature)
- Competitors have this (FreeRDP, xrdp)

**IronRDP Relationship:**
- Uses `ironrdp-cliprdr` PDUs
- Implements `CliprdrBackend` trait
- **Could be contributed upstream** (generic clipboard logic)

**Open Source Recommendation:** ✅ **YES**
**Rationale:** High community value, low competitive risk, builds ecosystem

---

### CRATE 2: `lamco-rdp-input` ⭐ MEDIUM REUSE

**Status:** ✅ **RECOMMEND OPEN SOURCE** (MIT/Apache-2.0)

**Current Location:** `src/input/` (7 files, 3,717 lines)

**Purpose:**
RDP input event translation (keyboard scancodes, mouse coordinates) with multi-monitor support and 200+ keymap translations.

**Files:**
- `translator.rs` (599 lines) - Main coordinator
- `coordinates.rs` (621 lines) - Multi-monitor coordinate transform
- `mapper.rs` (757 lines) - Scancode→keysym mapping (200+ codes)
- `keyboard.rs` (528 lines) - Keyboard handler
- `mouse.rs` (502 lines) - Mouse handler
- `error.rs` (473 lines) - Error types
- `mod.rs` (~200 lines) - Public API

**Dependencies:**
- External: `ironrdp-pdu`, `xkbcommon`
- Internal: None (self-contained)

**Key Features:**
- 200+ scancode mappings (US, DE, FR, UK, Dvorak, AZERTY, QWERTZ)
- Multi-monitor coordinate transformation
- DPI scaling
- Modifier tracking (Shift, Ctrl, Alt, Meta)
- Toggle key handling (Caps Lock, Num Lock)

**Reusability:** ⭐⭐⭐⭐ **HIGH**
- Any RDP server needs keyboard/mouse translation
- Coordinate transforms useful for any remote desktop

**Open Source Value:** ⭐⭐⭐⭐ **HIGH**
- Keymap data valuable to community
- Reference implementation of input translation

**Proprietary Value:** ⭐ **LOW**
- Standard feature (not differentiating)

**IronRDP Relationship:**
- Uses IronRDP input PDUs
- Independent of IronRDP logic

**Open Source Recommendation:** ✅ **YES**
**Rationale:** Generic utility, community would expand keymaps

---

### CRATE 3: `lamco-portal-integration` ⭐⭐ CRITICAL DEPENDENCY

**Status:** ⚠️ **CONSIDER OPEN SOURCE** (dual license option)

**Current Location:** `src/portal/` (5 files, ~600 lines)

**Purpose:**
XDG Desktop Portal integration (ScreenCast, RemoteDesktop, Clipboard APIs).

**Files:**
- `mod.rs` (~100 lines) - PortalManager
- `session.rs` (~100 lines) - Session lifecycle
- `screencast.rs` (~150 lines) - ScreenCast portal
- `remote_desktop.rs` (~150 lines) - RemoteDesktop portal
- `clipboard.rs` (~100 lines) - Clipboard portal

**Dependencies:**
- External: `ashpd`, `zbus`
- Internal: None (self-contained)

**Key Features:**
- D-Bus connection management
- Session permission handling
- Stream info extraction
- Async Portal API wrappers

**Reusability:** ⭐⭐⭐⭐⭐ **EXTREMELY HIGH**
- **ANY remote desktop project needs this** (VNC, TeamViewer alternatives)
- Useful for screen recording tools
- Video conferencing apps could use

**Open Source Value:** ⭐⭐⭐⭐⭐ **CRITICAL**
- Portal adoption needs more examples
- Portal ecosystem benefits
- Reference implementation for Wayland developers

**Proprietary Value:** ⭐ **LOW**
- Portal is security boundary (can't be bypassed)
- Not a differentiator

**Decision Point:** This is **infrastructure code** - no competitive advantage in keeping proprietary.

**Open Source Recommendation:** ✅ **YES** (or contribute to `ashpd` crate directly)
**Rationale:** Ecosystem benefit, builds Portal adoption, no competitive risk

---

### CRATE 4: `lamco-pipewire-capture` ⭐⭐⭐ HIGH TECHNICAL VALUE

**Status:** ⚠️ **STRATEGIC DECISION REQUIRED**

**Current Location:** `src/pipewire/` (11 files, ~3,700 lines)

**Purpose:**
Screen capture via PipeWire with dedicated thread for non-Send types, DMA-BUF zero-copy support.

**Files:**
- `pw_thread.rs` (846 lines) - Dedicated thread (non-Send handling)
- `connection.rs` (484 lines) - PipeWire connection
- `stream.rs` (509 lines) - Stream management
- `buffer.rs` (527 lines) - Buffer types (MemPtr, MemFd, DmaBuf)
- `frame.rs` (~200 lines) - Frame extraction
- `thread_comm.rs` (~200 lines) - Thread communication
- `coordinator.rs` (~150 lines) - Multi-stream coordination
- `format.rs` (~150 lines) - Format negotiation
- `ffi.rs` (~150 lines) - FFI bindings
- `error.rs` (~400 lines) - Error types
- `mod.rs` (~150 lines) - Public API

**Dependencies:**
- External: `pipewire`, `libspa`, `libspa-sys`, `tokio`
- Internal: None (self-contained)

**Key Features:**
- Non-Send PipeWire types in dedicated thread
- DMA-BUF support (zero-copy from GPU)
- MemFd (shared memory) support
- Multi-stream coordination (multiple monitors)
- Format negotiation

**Reusability:** ⭐⭐⭐⭐⭐ **EXTREMELY HIGH**
- **ANY screen capture app needs this**
- Screen recorders (OBS, etc.) could use
- Video conferencing apps
- Remote desktop projects

**Open Source Value:** ⭐⭐⭐⭐⭐ **EXTREMELY HIGH**
- Solves hard PipeWire problem (non-Send types)
- Reference implementation for PipeWire screen capture
- Industry-standard pattern (dedicated thread + channels)

**Proprietary Value:** ⭐⭐ **MEDIUM**
- Our implementation is good but not unique
- Others will figure this out eventually
- DMA-BUF support is technical achievement but not secret

**Decision Point:**
**Option A:** Open source - Build PipeWire ecosystem, gain contributors
**Option B:** Keep proprietary - Maintain technical lead (temporary)

**IronRDP Relationship:** Independent (no IronRDP dependencies)

**Open Source Recommendation:** ✅ **YES** (strategic ecosystem play)
**Rationale:**
- PipeWire needs more adoption
- We benefit from ecosystem growth
- Technical lead is temporary (others will solve this)
- Builds credibility as Wayland experts

**Alternative:** Keep proprietary for 1 year, then open source after market position established.

---

### CRATE 5: `lamco-video-pipeline` ⭐ STANDARD COMPONENT

**Status:** ✅ **RECOMMEND OPEN SOURCE** (MIT/Apache-2.0)

**Current Location:** `src/video/` (5 files, ~1,500 lines)

**Purpose:**
Video frame processing (format conversion, encoding interface, dispatching).

**Files:**
- `converter.rs` (742 lines) - Pixel format conversion
- `dispatcher.rs` (545 lines) - Frame dispatch
- `processor.rs` (~200 lines) - Frame processing
- `encoder/mod.rs` (~100 lines) - Encoder interface
- `mod.rs` (~16 lines) - Exports

**Dependencies:**
- External: None (pure Rust pixel conversion)
- Internal: `pipewire` (for frame types)

**Key Features:**
- BGRA↔RGB conversion
- Frame hashing (change detection)
- Encoder trait abstraction
- Statistics tracking

**Reusability:** ⭐⭐⭐ **MEDIUM**
- Pixel conversion useful for video projects
- Encoder abstraction is generic

**Open Source Value:** ⭐⭐⭐ **MEDIUM**
- Standard video processing code
- Nothing special here

**Proprietary Value:** ⭐ **LOW**
- Commodity code

**Open Source Recommendation:** ✅ **YES**
**Rationale:** Generic utility, no competitive advantage

---

### CRATE 6: `lamco-rdp-protocol` ⭐ THIN WRAPPER

**Status:** ✅ **RECOMMEND OPEN SOURCE** (MIT/Apache-2.0)

**Current Location:** `src/protocol/mod.rs`, `src/rdp/` (3 files, ~150 lines)

**Purpose:**
Thin wrappers and extensions to IronRDP protocol types.

**Files:**
- `protocol/mod.rs` (~50 lines)
- `rdp/mod.rs` (~50 lines)
- `rdp/channels/mod.rs` (~50 lines)

**Reusability:** ⭐⭐ **LOW**
**Open Source Value:** ⭐ **LOW**
**Proprietary Value:** ⭐ **NONE**

**Open Source Recommendation:** ✅ **YES** (or merge into other crates)
**Rationale:** Too small to matter, just utility code

---

### CRATE 7: `lamco-rdp-server` 🔒 ORCHESTRATION

**Status:** 🔒 **RECOMMEND PROPRIETARY** (Commercial License)

**Current Location:** `src/server/` (6 files, ~2,100 lines)

**Purpose:**
Portal mode server orchestration, event multiplexing, IronRDP integration.

**Files:**
- `mod.rs` (430 lines) - Main server orchestration
- `display_handler.rs` (612 lines) - Video streaming
- `input_handler.rs` (564 lines) - Input processing
- `event_multiplexer.rs` (300+ lines) - Priority QoS
- `graphics_drain.rs` (~150 lines) - Graphics coalescing
- `multiplexer_loop.rs` (~200 lines) - Event drain loop

**Dependencies:**
- External: `ironrdp-server`, `tokio`
- Internal: All open source crates above

**Key Features:**
- IronRDP trait implementations
- 4-queue priority multiplexer (Input/Control/Clipboard/Graphics)
- Frame rate regulation (30 FPS token bucket)
- Graphics coalescing
- Input batching (10ms windows)
- Portal session lifecycle

**Reusability:** ⭐ **LOW**
- Specific to our Portal mode product

**Open Source Value:** ⭐⭐ **LOW**
- Others need different orchestration

**Proprietary Value:** ⭐⭐⭐⭐ **HIGH**
- **This is our product** - how we glue everything together
- Performance optimizations (multiplexer, batching)
- Specific integration patterns

**Open Source Recommendation:** ❌ **NO**
**Rationale:** This is the portal mode product - keep proprietary

---

### CRATE 8: `lamco-rdp-compositor` 🔒 HEADLESS VDI

**Status:** 🔒 **RECOMMEND PROPRIETARY** (Commercial License)

**Current Location:** Not implemented yet (future)

**Purpose:**
Headless compositor mode for VDI (Smithay-based, no physical display).

**Proprietary Value:** ⭐⭐⭐⭐⭐ **EXTREMELY HIGH**
- **Enterprise product differentiator**
- Multi-user VDI capability
- Direct rendering (no Portal overhead)

**Open Source Recommendation:** ❌ **NO**
**Rationale:** Core enterprise offering

---

### CRATE 9: `lamco-rdp-multimon` 🔒 VALUE-ADD

**Status:** 🔒 **RECOMMEND PROPRIETARY** (Commercial License)

**Current Location:** `src/multimon/` (3 files, ~400 lines)

**Purpose:**
Multi-monitor layout calculation and stream coordination.

**Files:**
- `manager.rs` (~200 lines) - Monitor management
- `layout.rs` (~150 lines) - Layout calculation
- `mod.rs` (~50 lines) - Exports

**Reusability:** ⭐⭐ **LOW**
**Proprietary Value:** ⭐⭐⭐ **MEDIUM-HIGH**
- Enterprise feature (multi-monitor VDI)
- Complex layout algorithms

**Open Source Recommendation:** ❌ **NO**
**Rationale:** Enterprise value-add feature

---

### CRATE 10: `lamco-rdp-security` 🔒 ENTERPRISE

**Status:** 🔒 **RECOMMEND PROPRIETARY** (Commercial License)

**Current Location:** `src/security/` (4 files, ~300 lines)

**Purpose:**
TLS certificate management, PAM authentication, enterprise security features.

**Files:**
- `mod.rs` (~80 lines) - SecurityManager
- `tls.rs` (~80 lines) - TLS configuration
- `certificates.rs` (~80 lines) - Certificate generation
- `auth.rs` (~60 lines) - PAM authentication

**Proprietary Value:** ⭐⭐⭐ **MEDIUM-HIGH**
- Enterprise authentication hooks
- Certificate management
- Security policies

**Open Source Recommendation:** ❌ **NO**
**Rationale:** Enterprise security features, integration points for SSO/LDAP

---

### CRATE 11: `lamco-rdp-config` ✅ UTILITY

**Status:** ✅ **RECOMMEND OPEN SOURCE** (MIT/Apache-2.0)

**Current Location:** `src/config/` (2 files, ~150 lines)

**Purpose:**
Configuration loading and validation (TOML/env/CLI).

**Reusability:** ⭐⭐⭐ **MEDIUM**
**Proprietary Value:** ⭐ **NONE**

**Open Source Recommendation:** ✅ **YES**
**Rationale:** Generic utility

---

### CRATE 12: `lamco-rdp-utils` ✅ UTILITY

**Status:** ✅ **RECOMMEND OPEN SOURCE** (MIT/Apache-2.0)

**Current Location:** `src/utils/` (4 files, ~600 lines)

**Purpose:**
Error types, diagnostics, metrics.

**Reusability:** ⭐⭐⭐ **MEDIUM**
**Proprietary Value:** ⭐ **NONE**

**Open Source Recommendation:** ✅ **YES**
**Rationale:** Generic utility

---

## STRATEGIC RECOMMENDATIONS

### Open Source Foundation (5-6 Crates)

| Crate | Lines | License | Rationale |
|-------|-------|---------|-----------|
| `lamco-rdp-clipboard` | 7,826 | MIT/Apache-2.0 | Ecosystem builder, high reuse |
| `lamco-rdp-input` | 3,717 | MIT/Apache-2.0 | Generic utility, community expansion |
| `lamco-portal-integration` | 600 | MIT/Apache-2.0 | Portal adoption, ecosystem |
| `lamco-pipewire-capture` | 3,700 | MIT/Apache-2.0 | Strategic ecosystem play ⭐ |
| `lamco-video-pipeline` | 1,500 | MIT/Apache-2.0 | Commodity code |
| `lamco-rdp-utils` | 750 | MIT/Apache-2.0 | Generic utility |

**Total Open Source:** ~18,100 lines (~84% of codebase)

### Proprietary Components (4 Crates)

| Crate | Lines | License | Rationale |
|-------|-------|---------|-----------|
| `lamco-rdp-server` | 2,100 | Commercial | Portal mode product |
| `lamco-rdp-compositor` | (future) | Commercial | Headless VDI product |
| `lamco-rdp-multimon` | 400 | Commercial | Enterprise feature |
| `lamco-rdp-security` | 300 | Commercial | Enterprise integration |

**Total Proprietary:** ~2,800+ lines (~16% of codebase)

---

## IRONRDP RELATIONSHIP STRATEGY

### What IronRDP Provides

| IronRDP Crate | Usage | Our Dependency |
|---------------|-------|----------------|
| `ironrdp-server` | Server skeleton, TLS, NLA | Core (trait impls) |
| `ironrdp-pdu` | PDU encoding/decoding | Protocol parsing |
| `ironrdp-cliprdr` | Clipboard PDUs | Clipboard crate |
| `ironrdp-graphics` | RemoteFX encoding | Video crate |
| `ironrdp-core` | Core types | Throughout |

### Fork Strategy

**Current:** Using `glamberson/IronRDP` fork (branch: `update-sspi-with-clipboard-fix`)
- 8 commits ahead of upstream
- Critical patch: Server clipboard initiation (may never be accepted upstream)

**Recommendation:**

1. **Maintain Fork for Server-Specific Features**
   - Server clipboard initiation (required for our use case)
   - Any server-specific protocol extensions

2. **Contribute Generic Improvements Upstream**
   - Bug fixes
   - Protocol correctness improvements
   - Client features (if we add any)

3. **Monthly Rebase Against Upstream**
   - Stay current with allan2/IronRDP
   - Minimize divergence

4. **Document All Patches**
   - Clear commit messages
   - Separate branch for each patch set
   - Merge path if upstream accepts

---

## IMPLEMENTATION ROADMAP

### Phase 1: Extract Open Source Crates (Week 1-2)

**Priority Order:**
1. `lamco-rdp-utils` - Foundation (no dependencies)
2. `lamco-rdp-protocol` - Simple extraction
3. `lamco-video-pipeline` - No internal deps
4. `lamco-rdp-input` - Self-contained
5. `lamco-portal-integration` - Clean boundaries
6. `lamco-pipewire-capture` - Complex (dedicated thread)
7. `lamco-rdp-clipboard` - Most complex (many features)

**Per Crate Tasks:**
1. Create new crate directory: `crates/lamco-{name}/`
2. Move source files
3. Define `Cargo.toml` with dependencies
4. Create `lib.rs` with public API
5. Write `README.md` (usage examples)
6. Add tests
7. Document with rustdoc

### Phase 2: Create Workspace (Week 2)

**Workspace Structure:**
```
lamco-rdp/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── lamco-rdp-clipboard/
│   ├── lamco-rdp-input/
│   ├── lamco-portal-integration/
│   ├── lamco-pipewire-capture/
│   ├── lamco-video-pipeline/
│   ├── lamco-rdp-protocol/
│   ├── lamco-rdp-utils/
│   ├── lamco-rdp-server/         # Proprietary
│   ├── lamco-rdp-multimon/       # Proprietary
│   └── lamco-rdp-security/       # Proprietary
├── LICENSE-MIT                    # For open source crates
├── LICENSE-APACHE                 # For open source crates
├── LICENSE-COMMERCIAL             # For proprietary crates
└── README.md
```

**Workspace Cargo.toml:**
```toml
[workspace]
members = [
    "crates/lamco-rdp-clipboard",
    "crates/lamco-rdp-input",
    "crates/lamco-portal-integration",
    "crates/lamco-pipewire-capture",
    "crates/lamco-video-pipeline",
    "crates/lamco-rdp-protocol",
    "crates/lamco-rdp-utils",
    "crates/lamco-rdp-server",
    "crates/lamco-rdp-multimon",
    "crates/lamco-rdp-security",
]

[workspace.dependencies]
# Shared version declarations
tokio = "1.35"
ironrdp-server = { git = "...", branch = "..." }
# ...
```

### Phase 3: Publish Open Source Crates (Week 3)

**Publication Order:**
1. `lamco-rdp-utils` → crates.io
2. `lamco-rdp-protocol` → crates.io
3. `lamco-rdp-input` → crates.io
4. `lamco-video-pipeline` → crates.io
5. `lamco-portal-integration` → crates.io
6. `lamco-pipewire-capture` → crates.io (strategic announcement)
7. `lamco-rdp-clipboard` → crates.io (major announcement)

**Each Publication Includes:**
- Complete rustdoc
- Usage examples
- CHANGELOG.md
- README.md with badges
- CI/CD (tests, clippy, rustfmt)
- Apache-2.0 OR MIT license

### Phase 4: Marketing & Community (Ongoing)

**Open Source Announcement:**
- Blog post: "Open Sourcing our Wayland RDP Components"
- Reddit: /r/rust, /r/linux, /r/wayland
- Hacker News submission
- Lobste.rs
- Twitter/Mastodon

**Community Engagement:**
- GitHub Discussions enabled
- Issue templates
- Contributing guide
- Code of conduct

---

## LICENSING STRATEGY

### Dual License for Open Source

**All open source crates:**
```
Licensed under either of:
 * Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

at your option.
```

**Rationale:**
- Apache-2.0 provides patent protection
- MIT provides simplicity
- Dual license maximizes adoption

### Commercial License for Proprietary

**Proprietary crates:**
```
Copyright (c) 2025 Lamco [Your Company Name]
All rights reserved.

This software is proprietary and confidential.
```

**Commercial Licensing Options:**
1. **Free for Desktop** - Portal mode (compiled binary distribution)
2. **Paid for VDI** - Compositor mode (per-seat or subscription)

---

## COMPETITIVE ANALYSIS

### How This Positions Us

**vs FreeRDP:**
- ✅ We have modern Rust implementation
- ✅ Open source components build trust
- ✅ Portal integration is safer
- ❌ They have more mature codec support (H.264)

**vs xrdp:**
- ✅ Native Wayland (they're X11-based)
- ✅ Better architecture (async, Rust)
- ✅ Open source foundation matches theirs (GPL)
- ❌ They have larger user base

**vs RustDesk:**
- ✅ We're RDP (industry standard)
- ✅ Enterprise features (they're P2P focused)
- ❌ They have consumer traction

**vs GNOME Remote Desktop:**
- ✅ More complete protocol support
- ✅ Better performance (optimized)
- ✅ Enterprise features
- ❌ They're GNOME default (distribution advantage)

**Our Differentiators:**
1. **Hybrid open source model** - Build trust while maintaining business
2. **Modern Rust implementation** - Security + performance
3. **Portal-first** - Best Wayland integration
4. **Enterprise VDI mode** - Headless compositor for cloud

---

## RISK ANALYSIS

### Risk 1: Competitors Fork Our Open Source

**Likelihood:** HIGH
**Impact:** MEDIUM

**Mitigation:**
- Proprietary orchestration layer (hard to replicate)
- Continuous innovation (H.264, features)
- Enterprise support and services
- Network effects from community

**Perspective:** This is **expected and desired** - builds ecosystem.

### Risk 2: IronRDP Rejects Our Contributions

**Likelihood:** MEDIUM
**Impact:** LOW

**Mitigation:**
- Maintain fork for server-specific features
- Contribute only generic improvements
- Clear fork documentation

### Risk 3: Community Doesn't Adopt Open Source Crates

**Likelihood:** MEDIUM
**Impact:** MEDIUM

**Mitigation:**
- Ensure high quality (tests, docs)
- Active maintenance
- Solve real problems (PipeWire non-Send, clipboard loops)

### Risk 4: Maintaining Multiple Crates is Overhead

**Likelihood:** HIGH
**Impact:** MEDIUM

**Mitigation:**
- Cargo workspace (shared builds)
- CI/CD automation
- Clear ownership boundaries

---

## SUCCESS METRICS

### Open Source Success

**6 Months:**
- [ ] 50+ GitHub stars (per major crate)
- [ ] 5+ external contributors
- [ ] 10+ issues/discussions (community engagement)
- [ ] 500+ crates.io downloads

**1 Year:**
- [ ] 200+ GitHub stars
- [ ] 10+ external contributors
- [ ] Used by 1+ other projects
- [ ] 2,000+ crates.io downloads

### Business Success

**6 Months:**
- [ ] Portal mode: 100+ installations (free users)
- [ ] Compositor mode: 3+ paying customers
- [ ] Revenue: $X/month

**1 Year:**
- [ ] Portal mode: 1,000+ installations
- [ ] Compositor mode: 20+ paying customers
- [ ] Revenue: $Y/month

---

## DECISION MATRIX

### IMMEDIATE DECISIONS REQUIRED

| Decision | Options | Recommendation |
|----------|---------|----------------|
| **PipeWire Crate** | Open source vs Proprietary | ✅ Open source (strategic) |
| **Portal Crate** | Open source vs Proprietary | ✅ Open source (ecosystem) |
| **Clipboard Crate** | Open source vs Proprietary | ✅ Open source (community) |
| **Server Orchestration** | Open source vs Proprietary | 🔒 Proprietary (product) |
| **Compositor Mode** | Open source vs Proprietary | 🔒 Proprietary (enterprise) |
| **Product Name** | Lamco-RDP-*, WayRDP, other | *Await your decision* |

---

## NEXT STEPS

**This Week:**
1. **Review this document** - Make strategic decisions
2. **Choose product naming** - Lamco-RDP-* or different
3. **Confirm open source strategy** - Approve/modify recommendations
4. **Begin crate extraction** - Start with `lamco-rdp-utils`

**Next Week:**
5. **Create workspace structure** - Set up Cargo workspace
6. **Extract remaining crates** - Follow roadmap
7. **Write documentation** - READMEs, rustdoc
8. **Set up CI/CD** - Automated testing

**Month 1:**
9. **Publish first crates** - Start with utilities
10. **Announce open source** - Blog post, social media
11. **Implement file transfer** - Complete MS-RDPECLIP
12. **Prepare v1.0 release** - Portal mode product

---

## CONCLUSION

**Recommended Strategy:**

1. **Open source the foundation** (~84% of code) - Builds trust, ecosystem, community
2. **Keep orchestration proprietary** (~16% of code) - Portal + Compositor mode products
3. **Strategic licensing** - MIT/Apache-2.0 for OSS, Commercial for proprietary
4. **Phased extraction** - Workspace first, then publish incrementally
5. **Active community engagement** - Make this the reference Wayland RDP implementation

**Competitive Position:**
- Best Wayland RDP integration (Portal-first)
- Modern Rust implementation (security, performance)
- Hybrid model (trust + business)
- Enterprise-ready (VDI mode, multi-monitor, security)

**Next Action:** Review and approve/modify strategy, then begin crate extraction.

---

**END OF CRATE BREAKDOWN**

*Awaiting strategic decisions on product naming and open source boundaries.*
