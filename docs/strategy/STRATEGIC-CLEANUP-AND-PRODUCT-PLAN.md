# STRATEGIC CLEANUP AND PRODUCT PLAN
## Comprehensive Analysis and Decision Framework
**Date:** 2025-12-10
**Status:** Strategic Planning Document
**Purpose:** Define cleanup, architecture, and product strategy

---

## EXECUTIVE SUMMARY

This document provides a complete strategic analysis of the Wayland RDP server project, examining:
1. **Codebase health** - What we have, what works, what's messy
2. **Architecture decisions** - IronRDP usage, protocol implementation scope
3. **Product strategy** - Open source vs commercial, distribution channels
4. **Cleanup roadmap** - Prioritized tasks to make this production-ready
5. **Clear next steps** - What to do now vs later

**Current State:** Mature, production-capable codebase (21,500+ lines) with excellent core architecture but significant organizational debt (100+ docs, 7 branches, unclear product direction).

**Recommendation:** Execute phased cleanup while making strategic product decisions.

---

## PART 1: CURRENT STATE ANALYSIS

### 1.1 Codebase Health Assessment

#### ✅ **STRENGTHS (What's Excellent)**

| Component | Quality | Status |
|-----------|---------|--------|
| **Core Architecture** | A+ | Async-first, well-modularized, production patterns |
| **Clipboard System** | A | Bidirectional sync, 3-layer dedup, format conversion |
| **Input Handling** | A | 200+ scancode mappings, multi-monitor transforms |
| **Video Pipeline** | B+ | PipeWire integration, 30 FPS stable |
| **Security** | A | TLS 1.3, Portal permission model |
| **Error Handling** | A | Comprehensive error types, no panics |
| **Documentation (code)** | B+ | Rustdoc, inline comments, architecture docs |

**Total Code:** 21,506 lines across 10 major modules:
- clipboard/ (7,826 lines) - Bidirectional RDP↔Wayland sync
- input/ (3,717 lines) - Keyboard/mouse with coordinate transforms
- pipewire/ (3,700+ lines) - Screen capture with dedicated thread
- server/ (2,100+ lines) - Orchestration, multiplexing
- video/ (1,500+ lines) - Encoding, format conversion
- portal/ (600+ lines) - XDG Portal integration
- Plus: security/, multimon/, config/, utils/

**Key Technical Achievements:**
1. **Solved non-Send PipeWire integration** - Dedicated thread + channel pattern
2. **Priority-based event multiplexing** - 4 queues with QoS
3. **Clipboard loop prevention** - Content hashing + state machine
4. **Portal-based security** - No direct Wayland access (safer)
5. **Format conversion** - RDP formats ↔ MIME types

#### ❌ **WEAKNESSES (What's Messy)**

| Problem | Severity | Impact |
|---------|----------|--------|
| **Documentation Sprawl** | HIGH | 100+ .md files (mostly session notes) |
| **Branch Chaos** | HIGH | 7 feature branches, unclear status |
| **Naming Inconsistency** | MEDIUM | Binary, repo, package names misaligned |
| **IronRDP Fork Dependency** | MEDIUM | Maintenance burden, upstream divergence |
| **Test Coverage** | MEDIUM | Limited integration tests |
| **Product Strategy Undefined** | HIGH | Open source? Commercial? Both? |
| **Distribution Not Implemented** | LOW | No packages yet (but planned) |

### 1.2 Git Repository Status

#### **Branch Inventory**

| Branch | Purpose | Status | Recommendation |
|--------|---------|--------|----------------|
| `main` | Stable baseline | Active | ✅ Keep |
| `feature/gnome-clipboard-extension` | **CURRENT** - All latest work | Active | ✅ **PRIMARY** |
| `feature/smithay-compositor` | Headless compositor mode | Unknown | ⚠️ Evaluate |
| `feature/headless-infrastructure` | Headless deployment | Unknown | ⚠️ Evaluate |
| `feature/embedded-portal` | Portal backend | Unknown | ⚠️ Evaluate |
| `feature/lamco-compositor-clipboard` | Compositor clipboard | Unknown | ⚠️ Evaluate |
| `feature/wlr-clipboard-backend` | wlroots clipboard | Abandoned? | ❌ Consider deleting |
| `feature/clipboard-monitoring-solution` | Earlier approach | Superseded | ❌ Delete |

**Remote Branches:**
- `origin/claude/*` - Unreviewed CCW output (2 branches)

**Recommendation:** Audit all feature branches, merge valuable work to main, delete abandoned branches.

#### **Documentation Chaos**

**Current:** 100+ markdown files in root directory:
- 20+ "SESSION-HANDOVER-*.md" files
- 15+ "LOG-ANALYSIS-*.md" files
- 10+ architecture/planning docs
- 5+ clipboard-specific docs
- Plus: extension/, docs/, reference/ subdirectories

**Problem:** No clear entry point, historical cruft mixed with current docs.

### 1.3 IronRDP Fork Analysis

#### **Current Fork Status**

**Our Fork:** `glamberson/IronRDP`
**Branch:** `update-sspi-with-clipboard-fix`
**Upstream:** `allan2/IronRDP`
**Divergence:** 8 commits ahead

**Our Custom Changes:**
1. `fix(cliprdr): enable server clipboard ownership announcements` - **CRITICAL**
2. Debug logging for PDU encoding (7 commits)

**The Critical Change:**
```rust
// In ironrdp-cliprdr/src/backend.rs
// Allows servers to initiate clipboard copy (not just respond to client)
```

**Problem:** This feature may never be accepted upstream (servers traditionally passive in RDP).

#### **IronRDP Capabilities Analysis**

**What IronRDP Provides Well:**

| Crate | Purpose | Our Usage | Maturity |
|-------|---------|-----------|----------|
| ironrdp-server | Server skeleton, connection handling | Core | Good |
| ironrdp-pdu | Protocol Data Units (encoding/decoding) | Core | Excellent |
| ironrdp-cliprdr | Clipboard PDUs | Core | Good (needs our patch) |
| ironrdp-graphics | RemoteFX encoding | Core | Good (deprecated codec) |
| ironrdp-svc | Static virtual channels | Core | Good |
| ironrdp-dvc | Dynamic virtual channels | Not used | N/A |
| ironrdp-displaycontrol | Display control | Not used | Limited |
| ironrdp-ainput | Advanced input | Not used | Experimental |

**What IronRDP Lacks:**

| Protocol | Status in IronRDP | Our Need |
|----------|-------------------|----------|
| **MS-RDPEGFX** (H.264 graphics) | Minimal/incomplete | High (RemoteFX deprecated) |
| **MS-RDPEDISP** (resolution change) | Limited | Medium |
| **File transfer (MS-RDPECLIP)** | PDUs only, no logic | High (planned next) |
| **MS-RDPEA** (audio output) | Basic (ironrdp-rdpsnd) | Low |

#### **Fork Strategy Options**

**Option A: Maintain Our Fork** (Current)
- ✅ Full control over clipboard behavior
- ✅ Can add features quickly
- ❌ Maintenance burden (merge upstream changes)
- ❌ Upstream divergence risk
- ❌ Community fragmentation

**Option B: Contribute Upstream**
- ✅ Community benefit
- ✅ Shared maintenance
- ❌ PR may be rejected (servers as clipboard initiators)
- ❌ Slower development velocity
- ⚠️ Prior experience: Not well received

**Option C: Hybrid (Fork for Server-Specific Features)**
- ✅ Contribute generic improvements upstream
- ✅ Keep server-specific patches in fork
- ✅ Minimize divergence
- ✅ Clear boundary (client features → upstream, server features → fork)

**Recommendation:** **Option C** - Maintain fork only for server-specific clipboard initiation, contribute everything else upstream.

---

## PART 2: PRODUCT STRATEGY DECISIONS

### 2.1 Product Definition

#### **Two Distinct Products Identified**

**Product 1: Wayland RDP Portal Server (Desktop Mode)**
- **Target:** End users with GNOME/KDE desktops
- **Use Case:** Remote access to existing desktop session
- **Deployment:** User-installed package (Flatpak, deb, rpm)
- **Features:** Screen sharing, clipboard, input injection via Portal
- **Pricing Model:** TBD (Open source? Freemium?)

**Product 2: Wayland RDP Compositor Server (Headless VDI)**
- **Target:** Enterprise, cloud providers, VDI vendors
- **Use Case:** Multi-user virtual desktops (no physical display)
- **Deployment:** Container, Kubernetes, bare metal server
- **Features:** Built-in compositor (Smithay-based), direct rendering
- **Pricing Model:** TBD (Enterprise license? SaaS?)

**Critical Decision Needed:** Are these separate products or deployment modes of one product?

**Recommendation:** **Unified codebase with compile-time features:**
```toml
[features]
default = ["portal-mode"]
portal-mode = ["ashpd", "portal-integration"]
compositor-mode = ["smithay", "headless-vdi"]
```

Benefits:
- Shared core (clipboard, input, video pipeline)
- Different distribution channels
- Clear product differentiation
- Single codebase to maintain

### 2.2 Open Source Strategy

#### **Component-by-Component Decision**

| Component | License | Reasoning |
|-----------|---------|-----------|
| **Core library (wrd-core)** | MIT/Apache-2.0 | Community trust, contributions |
| **Clipboard logic** | MIT/Apache-2.0 | Reusable by others (goodwill) |
| **Input handling** | MIT/Apache-2.0 | Standard keyboard/mouse logic |
| **Portal mode binary** | MIT/Apache-2.0 | Desktop users expect open source |
| **GNOME extension** | GPL-3.0 | GNOME requirement |
| **Compositor mode** | **Commercial/Dual License** | Enterprise differentiation |
| **Container images** | **Proprietary** | Hosted service revenue |
| **Helm charts** | **Proprietary** | Enterprise offering |

**Hybrid Strategy Benefits:**
- ✅ Open source portal mode builds trust and community
- ✅ Commercial compositor mode funds development
- ✅ Core libraries benefit everyone
- ✅ Clear value proposition: Free for desktop, paid for VDI

**Risk:** Community backlash if not communicated clearly.

**Mitigation:**
- Be transparent from day one
- Provide free compositor mode for small deployments (<5 users)
- Contribute improvements to upstream dependencies
- Clear documentation of what's free vs paid

### 2.3 Naming Strategy

#### **Current Naming Chaos**

| Context | Current Name | Issues |
|---------|--------------|--------|
| Repository | `wrd-server-specs` | "specs" implies documentation, not code |
| Binary | `wrd-server` | Generic, not memorable |
| GitHub org | `lamco-admin` | Not product-branded |
| Package name | (undefined) | N/A |
| Flatpak ID | (undefined) | N/A |
| D-Bus service | (undefined) | N/A |

#### **Proposed Naming Scheme**

**Product Names:**
- **Portal Mode:** "WayRDP" or "Wayland Remote Desktop"
- **Compositor Mode:** "WayRDP Enterprise" or "WayVDI"

**Repository Names:**
- `wayrdp/wayrdp` - Main repository
- `wayrdp/gnome-extension` - Extension repository

**Package Names:**
- `wayrdp` - Portal mode package
- `wayrdp-enterprise` - Compositor mode package
- `gnome-shell-extension-wayrdp-clipboard` - Extension

**Flatpak ID:**
- `org.wayrdp.Desktop` - Portal mode
- `org.wayrdp.Enterprise` - Compositor mode (if packaged)

**D-Bus Service:**
- `org.wayrdp.Server`

**Binary Names:**
- `wayrdp` - Portal mode
- `wayrdp-enterprise` - Compositor mode

**Advantages:**
- ✅ Short, memorable ("WayRDP")
- ✅ Clear association (Wayland + RDP)
- ✅ Consistent across all contexts
- ✅ Searchable, unique
- ✅ Professional

**Decision Required:** Choose final product name.

### 2.4 Distribution Channels

#### **Portal Mode Distribution (Priority Order)**

| Channel | Timeline | Priority | Notes |
|---------|----------|----------|-------|
| **GitHub Releases** | Now | P0 | Pre-built binaries for testing |
| **Flatpak (Flathub)** | v1.0 | P1 | Primary discovery for desktop users |
| **Debian/Ubuntu (deb)** | v1.0 | P1 | Large user base |
| **Fedora (rpm)** | v1.0 | P2 | Official Fedora repos or Copr |
| **Arch (AUR)** | v1.0 | P2 | Community-maintained |
| **AppImage** | v1.1 | P3 | Portable, testing |

#### **Compositor Mode Distribution**

| Channel | Timeline | Priority | Notes |
|---------|----------|----------|-------|
| **Docker Hub / GHCR** | v1.0 | P1 | Container images |
| **Debian/RHEL packages** | v1.0 | P1 | Bare metal installs |
| **Helm Chart** | v1.1 | P2 | Kubernetes deployments |
| **AWS/Azure Marketplace** | v2.0 | P3 | SaaS offering |

#### **GNOME Extension Distribution**

| Channel | Timeline | Priority | Notes |
|---------|----------|----------|-------|
| **extensions.gnome.org** | v1.0 | P1 | Primary discovery |
| **Bundled with package** | v1.0 | P1 | Version-matched |

---

## PART 3: ARCHITECTURE DECISIONS

### 3.1 Protocol Implementation Scope

#### **RDP Protocols: What to Implement**

**Priority Tier 1 (MUST HAVE - v1.0):**

| Protocol | Purpose | Status | Effort | Decision |
|----------|---------|--------|--------|----------|
| **RDP Core** | Basic connection | ✅ Done (IronRDP) | N/A | Use IronRDP |
| **MS-RDPECLIP (Text)** | Clipboard text | ✅ Done | N/A | ✅ Keep |
| **MS-RDPECLIP (Images)** | Clipboard images | ✅ Done | N/A | ✅ Keep |
| **MS-RDPECLIP (Files)** | Copy/paste files | ⚠️ 30% | 6-8h | ✅ Implement |
| **RemoteFX** | Video (temp) | ✅ Done | N/A | ⚠️ Keep temporarily |
| **RDP Input** | Keyboard/mouse | ✅ Done | N/A | ✅ Keep |
| **TLS 1.3** | Security | ✅ Done | N/A | ✅ Keep |

**Priority Tier 2 (IMPORTANT - v1.1):**

| Protocol | Purpose | Status | Effort | Decision |
|----------|---------|--------|--------|----------|
| **MS-RDPEGFX** | H.264 video | ❌ Not started | 2-3w | ✅ Plan for v1.1 |
| **MS-RDPEDISP** | Resolution change | ❌ Not started | 2-3d | ✅ Plan for v1.1 |
| **AVC444** | Lossless text | ❌ Not started | 3-5d | ⚠️ After RDPEGFX |

**Priority Tier 3 (NICE TO HAVE - v2.0+):**

| Protocol | Purpose | Status | Effort | Decision |
|----------|---------|--------|--------|----------|
| **MS-RDPEA** | Audio output | ❌ Not started | 1-2w | ⏸️ Defer |
| **MS-RDPEAI** | Audio input | ❌ Not started | 1-2w | ⏸️ Defer |
| **MS-RDPEUSB** | USB redirection | ❌ Not started | 3-4w | ⏸️ Defer |
| **MS-RDPEPNP** | Printer | ❌ Not started | 2-3w | ⏸️ Defer |

**Rationale:**
- File transfer is #1 user request, quick win
- H.264 solves RemoteFX deprecation (Microsoft removed 2021, CVE-2020-1036)
- Resolution change is standard expectation
- Audio/USB/print are specialized, defer until validated need

#### **RemoteFX Status and Migration Plan**

**Current Situation:**
- ✅ RemoteFX working (30 FPS, minor horizontal lines)
- ❌ Microsoft deprecated July 2020, removed April 2021
- ❌ Security vulnerability (CVE-2020-1036) - unfixable
- ⚠️ Industry standard is now H.264/AVC444

**Migration Plan to H.264:**

**Phase 1: Assessment (1 week)**
- Audit IronRDP MS-RDPEGFX support (incomplete currently)
- Design graphics pipeline integration
- Evaluate H.264 encoders (VA-API, x264, OpenH264)

**Phase 2: RDPEGFX Protocol (1 week)**
- Implement graphics pipeline channel
- Frame acknowledgement
- Surface management
- Reset graphics PDU

**Phase 3: H.264 Integration (1 week)**
- VA-API hardware encoding (primary)
- x264/OpenH264 software fallback
- Quality/bitrate configuration
- Frame sequencing

**Phase 4: AVC444 (3-5 days)**
- 4:4:4 chroma sampling (lossless text)
- Split YUV encoding

**Total: 2-3 weeks for v1.1**

**Decision:** Keep RemoteFX for v1.0, migrate to H.264 for v1.1.

### 3.2 IronRDP Integration Strategy

#### **Clear Boundaries: What IronRDP Does vs What We Do**

**IronRDP Responsibilities:**
- ✅ RDP protocol (PDU encoding/decoding)
- ✅ TLS connection handling
- ✅ Channel management (SVC, DVC)
- ✅ Basic input event structures
- ✅ RemoteFX encoding
- ⚠️ MS-RDPEGFX (partial, needs completion)

**Our Responsibilities:**
- ✅ Portal integration (screen capture, input injection)
- ✅ PipeWire integration (video capture)
- ✅ Clipboard business logic (format conversion, loop prevention)
- ✅ File transfer logic (FileGroupDescriptorW, streaming)
- ✅ Multi-monitor coordination
- ✅ Event multiplexing (priority QoS)
- ✅ Configuration management
- ✅ Deployment (systemd, containers)

**IronRDP Usage Philosophy:**
1. **Use IronRDP for all protocol work** - Don't reinvent RDP
2. **Don't fight IronRDP's architecture** - Work with its patterns
3. **Fork only when necessary** - Minimize divergence
4. **Contribute generic improvements upstream** - Be a good citizen
5. **Keep server-specific logic in our codebase** - Don't bloat IronRDP

**Fork Maintenance Strategy:**
- Maintain `update-sspi-with-clipboard-fix` branch for server clipboard initiation
- Monthly rebase against upstream `allan2/IronRDP`
- Document all custom patches clearly
- If upstream releases breaking changes, adapt within 1 week

### 3.3 Multiplexer Architecture Decision

#### **Current Status: Phase 1 Complete**

**What's Implemented:**
- Graphics Queue (4 frames) - Drop/coalesce policy
- Graphics Drain Task - Dedicated async task
- Non-blocking sends - Never blocks IronRDP

**What's NOT Implemented (Phase 2-4):**
- Input Queue - Direct to Portal currently
- Control Queue - IronRDP manages directly
- Clipboard Queue - Direct processing

**Full Multiplexer Would Require:**
- Forking IronRDP event loop (or adding hooks upstream)
- Routing all ServerEvent through our queues
- Priority-based drain loop
- **Effort:** 1-2 weeks

#### **Decision: Keep Phase 1 Only**

**Rationale:**
1. **Graphics isolation achieved** - Main goal accomplished (graphics can't block input)
2. **Diminishing returns** - Input is fast, doesn't need isolation
3. **Maintenance burden** - Full multiplexer requires IronRDP event loop fork
4. **Sufficient for v1.0** - No user complaints about input latency

**Document as:**
- ✅ Intentional design decision (not technical debt)
- ⚠️ Full multiplexer available if real-world need emerges
- 📋 Keep notes on how to implement if needed later

---

## PART 4: CLEANUP ROADMAP

### 4.1 Immediate Cleanup (Week 1)

#### **Task 1: Documentation Consolidation (2-3 hours)**

**Actions:**
1. Create `docs/archive/` directory
2. Move all session handover notes: `SESSION-*.md` → `docs/archive/`
3. Move all log analysis: `LOG-ANALYSIS-*.md` → `docs/archive/`
4. Keep in root:
   - `README.md` - Project overview
   - `QUICKSTART.md` - Getting started
   - `ARCHITECTURE.md` - System design
   - `CONTRIBUTING.md` - (new) How to contribute
   - `CHANGELOG.md` - (new) Release history

5. Create `docs/`:
   - `docs/PROTOCOLS.md` - RDP protocol implementation status
   - `docs/DEPLOYMENT.md` - How to deploy (Portal vs Compositor modes)
   - `docs/DEVELOPMENT.md` - Development guide
   - `docs/TESTING.md` - Testing procedures

**Result:** Clear documentation structure, easy entry point for new contributors.

#### **Task 2: Branch Cleanup (1-2 hours)**

**Actions:**
1. **Audit each feature branch:**
   ```bash
   git log main..feature/branch-name --oneline
   git diff main...feature/branch-name --stat
   ```

2. **Decision matrix:**
   - Has valuable code? → Merge to main or keep as feature branch
   - Abandoned/superseded? → Delete
   - Experimental? → Tag and delete

3. **Proposed outcomes:**
   - `feature/gnome-clipboard-extension` → **Rename to `develop`** (main development branch)
   - `feature/smithay-compositor` → Evaluate commits, merge or delete
   - `feature/headless-infrastructure` → Evaluate, likely merge
   - `feature/embedded-portal` → Likely superseded, delete
   - `feature/lamco-compositor-clipboard` → Likely superseded, delete
   - `feature/wlr-clipboard-backend` → Abandoned, delete
   - `feature/clipboard-monitoring-solution` → Superseded, delete

4. **New branch structure:**
   - `main` - Stable releases (tags: v0.9.0, v1.0.0, etc.)
   - `develop` - Active development (formerly feature/gnome-clipboard-extension)
   - Feature branches only for major features in progress

**Result:** Clean branch structure, clear development flow.

#### **Task 3: Naming Consistency (1 hour)**

**Actions:**
1. **Choose final product name** (decision required)
   - Proposed: "WayRDP"
   - Alternative: "Wayland Remote Desktop", "WayVDI"

2. **Update all references:**
   - Repository name (GitHub rename)
   - Binary name in Cargo.toml
   - README.md title
   - All documentation
   - D-Bus service names (future)
   - Flatpak ID (future)

**Result:** Consistent branding across all touchpoints.

### 4.2 Short-Term Cleanup (Week 2-3)

#### **Task 4: Repository Restructuring (4-6 hours)**

**Current:**
```
wrd-server-specs/
├── src/
├── docs/
├── extension/
├── 100+ .md files
└── Cargo.toml
```

**Proposed:**
```
wayrdp/
├── crates/
│   ├── wayrdp-core/           # Core library (reusable)
│   ├── wayrdp-portal/         # Portal mode binary
│   ├── wayrdp-compositor/     # Compositor mode binary
│   └── wayrdp-protocol/       # RDP protocol extensions
├── extension/                  # GNOME extension
├── docs/
│   ├── architecture/
│   ├── development/
│   ├── deployment/
│   └── archive/               # Historical notes
├── scripts/                    # Build, setup scripts
├── examples/                   # Code examples
├── tests/
│   ├── integration/
│   └── e2e/
├── README.md
├── ARCHITECTURE.md
├── QUICKSTART.md
├── CONTRIBUTING.md
├── CHANGELOG.md
├── LICENSE-MIT
├── LICENSE-APACHE
└── Cargo.toml                  # Workspace
```

**Benefits:**
- ✅ Workspace structure (faster builds, shared deps)
- ✅ Clear product separation (portal vs compositor)
- ✅ Reusable core library
- ✅ Professional project layout
- ✅ Easier for contributors to navigate

**Cargo Workspace:**
```toml
[workspace]
members = [
    "crates/wayrdp-core",
    "crates/wayrdp-portal",
    "crates/wayrdp-compositor",
    "crates/wayrdp-protocol",
]
```

#### **Task 5: Test Infrastructure (6-8 hours)**

**Current:** 3 integration tests, minimal coverage.

**Proposed:**
1. **Unit tests** (in module files):
   - Coordinate transformation
   - Format conversion
   - Hash deduplication

2. **Integration tests** (`tests/integration/`):
   - Portal session creation
   - PipeWire stream capture
   - Clipboard loop prevention
   - File descriptor serialization

3. **E2E tests** (`tests/e2e/`):
   - RDP connection establishment
   - Keyboard/mouse input flow
   - Clipboard text/image transfer
   - Video streaming validation

4. **CI/CD** (GitHub Actions):
   ```yaml
   name: CI
   on: [push, pull_request]
   jobs:
     test:
       runs-on: ubuntu-latest
       steps:
         - uses: actions/checkout@v4
         - uses: dtolnay/rust-toolchain@stable
         - run: cargo test --all-features
         - run: cargo clippy -- -D warnings
         - run: cargo fmt -- --check
   ```

**Result:** Confidence in changes, prevent regressions.

### 4.3 Medium-Term Cleanup (Month 1-2)

#### **Task 6: IronRDP Fork Maintenance (ongoing)**

**Actions:**
1. Document custom patches in `docs/IRONRDP-FORK.md`
2. Set up monthly rebase schedule
3. Track upstream changes (GitHub watch, RSS)
4. Prepare for upstream contribution (clean patches)

#### **Task 7: Performance Profiling (1 week)**

**Current Issues:**
- Some frames take 2-2.5ms vs 100μs typical
- Unknown cause of variance

**Profiling Tasks:**
1. **Flamegraph analysis** (cargo flamegraph)
2. **Perf profiling** (Linux perf)
3. **Heap profiling** (valgrind, heaptrack)
4. **Lock contention** (parking_lot debug features)

**Goal:** Identify and fix remaining performance bottlenecks.

#### **Task 8: Security Audit (1 week)**

**Audit Areas:**
1. **Dependency audit** (cargo audit)
2. **Unsafe code review** (clippy, miri)
3. **Fuzzing** (cargo fuzz)
4. **TLS configuration** (rustls best practices)
5. **Input validation** (filename sanitization, path traversal)

**Result:** Production-ready security posture.

---

## PART 5: IMPLEMENTATION PRIORITIES

### 5.1 Feature Roadmap

#### **v0.9 (Current) - Stabilization**

**Goals:**
- ✅ Clipboard text/images working
- ✅ Video streaming 30 FPS
- ✅ Input responsive
- ⏳ Documentation cleanup
- ⏳ Branch cleanup
- ⏳ Repository restructuring

**Release:** Internal testing only.

#### **v1.0 - Production Ready (3-4 weeks)**

**Must Have:**
- ✅ File transfer (copy/paste files) - **6-8 hours**
- ✅ Documentation complete
- ✅ Test coverage >70%
- ✅ Packaging (Flatpak, deb, rpm)
- ✅ GNOME extension published
- ✅ Security audit passed
- ✅ Performance optimized
- ⚠️ RemoteFX kept (with warnings)

**Release:** Public beta, community testing.

#### **v1.1 - Modern Codecs (2-3 months after v1.0)**

**Major Features:**
- ✅ MS-RDPEGFX + H.264 support - **2-3 weeks**
- ✅ MS-RDPEDISP (resolution change) - **2-3 days**
- ✅ AVC444 (lossless text) - **3-5 days**
- ✅ VA-API hardware encoding
- ✅ Remove RemoteFX

**Release:** First production release.

#### **v1.2 - Compositor Mode (4-6 months after v1.0)**

**Major Features:**
- ✅ Smithay compositor integration
- ✅ Headless VDI mode
- ✅ Multi-user support
- ✅ Container images
- ✅ Kubernetes Helm chart

**Release:** Enterprise offering.

#### **v2.0+ - Advanced Features**

**Potential:**
- Audio output (MS-RDPEA)
- Audio input (MS-RDPEAI)
- USB redirection (MS-RDPEUSB)
- Printer redirection
- RemoteApp support

### 5.2 Development Workflow

#### **Branching Strategy (GitFlow Variant)**

```
main (stable releases)
  │
  ├── develop (active development)
  │     │
  │     ├── feature/file-transfer
  │     ├── feature/h264-encoding
  │     └── feature/compositor-mode
  │
  └── release/v1.0 (release preparation)
```

**Rules:**
1. **main** = Tagged releases only (v1.0.0, v1.1.0, etc.)
2. **develop** = Integration branch, always buildable
3. **feature/*** = Feature development, short-lived
4. **PR required** for all merges to develop/main
5. **CI must pass** before merge

#### **Release Process**

1. Create `release/vX.Y` branch from develop
2. Version bump, changelog update
3. Final testing
4. Merge to main, tag release
5. Build and publish packages
6. Merge back to develop

### 5.3 Immediate Next Steps (This Session)

**Priority Order:**

1. **Documentation Consolidation** (2 hours)
   - Archive session notes
   - Create clear README/QUICKSTART
   - Establish documentation structure

2. **Decision: Product Name** (30 minutes)
   - Choose: "WayRDP", "Wayland Remote Desktop", or other
   - Update branding document

3. **Decision: Open Source Strategy** (30 minutes)
   - Confirm: Portal mode = open source, Compositor mode = commercial/dual
   - Document licensing plan

4. **Branch Cleanup** (1 hour)
   - Audit feature branches
   - Delete abandoned branches
   - Rename primary dev branch to `develop`

5. **File Transfer Implementation** (6-8 hours)
   - Highest user value
   - Clear implementation plan exists
   - Low risk

---

## PART 6: CRITICAL DECISIONS REQUIRED

### 6.1 Product Decisions

| Decision | Options | Recommendation | Urgency |
|----------|---------|----------------|---------|
| **Product Name** | WayRDP, Wayland RDP, WayVDI, other | **WayRDP** (short, memorable) | HIGH |
| **Open Source Strategy** | All open, all commercial, hybrid | **Hybrid** (portal=open, compositor=commercial) | HIGH |
| **Primary Distribution** | Flatpak, native packages, both | **Both** (Flatpak primary, packages important) | MEDIUM |
| **Compositor Mode Timing** | v1.0, v1.2, v2.0 | **v1.2** (portal mode first) | LOW |

### 6.2 Technical Decisions

| Decision | Options | Recommendation | Urgency |
|----------|---------|----------------|---------|
| **IronRDP Fork** | Maintain, contribute, hybrid | **Hybrid** (fork server features, contribute generic) | HIGH |
| **Full Multiplexer** | Implement now, phase 1 only, defer | **Phase 1 only** (sufficient) | LOW |
| **RemoteFX Migration** | v1.0, v1.1, v2.0 | **v1.1** (RemoteFX acceptable for v1.0) | MEDIUM |
| **Repository Structure** | Monorepo, multi-repo | **Monorepo** (workspace) | MEDIUM |

### 6.3 Organization Decisions

| Decision | Options | Recommendation | Urgency |
|----------|---------|----------------|---------|
| **GitHub Organization** | Create new, keep lamco-admin | **Create new** (brand-aligned) | MEDIUM |
| **Documentation Platform** | GitHub Pages, mdBook, Docusaurus | **mdBook** (Rust ecosystem) | LOW |
| **Community** | Discord, GitHub Discussions, both | **GitHub Discussions** (lower overhead) | LOW |

---

## PART 7: EXECUTION PLAN

### This Week (Week 1)

**Monday-Tuesday:**
- [ ] Make critical decisions (product name, open source strategy)
- [ ] Documentation consolidation
- [ ] Branch cleanup

**Wednesday-Thursday:**
- [ ] File transfer implementation (6-8 hours)
- [ ] Test file transfer thoroughly

**Friday:**
- [ ] Update README/QUICKSTART with new branding
- [ ] Create CONTRIBUTING.md
- [ ] Start CHANGELOG.md

### Next Week (Week 2)

**Monday-Tuesday:**
- [ ] Repository restructuring (workspace)
- [ ] Move code to crates/
- [ ] Update build scripts

**Wednesday-Friday:**
- [ ] Test infrastructure
- [ ] CI/CD setup
- [ ] First automated tests

### Month 1

**Week 3-4:**
- [ ] Security audit
- [ ] Performance profiling and optimization
- [ ] Packaging preparation (Flatpak manifest, deb rules)

**Week 5:**
- [ ] v1.0 beta release preparation
- [ ] Documentation finalization
- [ ] Community testing

### Month 2-3

**Month 2:**
- [ ] H.264/RDPEGFX research and design
- [ ] VA-API encoder integration
- [ ] Resolution negotiation (RDPEDISP)

**Month 3:**
- [ ] H.264 implementation completion
- [ ] v1.1 release
- [ ] Community feedback integration

---

## PART 8: SUCCESS METRICS

### v1.0 Success Criteria

**Technical:**
- [ ] All clipboard operations work (text, images, files)
- [ ] Video streaming stable 30 FPS for >1 hour
- [ ] Input latency <10ms
- [ ] No crashes in 24-hour session
- [ ] Security audit passed

**Documentation:**
- [ ] README clear for new users
- [ ] Architecture documented
- [ ] Deployment guides complete
- [ ] Contributing guide exists

**Distribution:**
- [ ] Flatpak published on Flathub
- [ ] Debian package available
- [ ] GNOME extension on extensions.gnome.org
- [ ] GitHub releases with binaries

**Community:**
- [ ] 10+ GitHub stars (initial traction)
- [ ] 3+ contributors (beyond maintainer)
- [ ] 5+ issues/feedback (community engagement)

### v1.1 Success Criteria

**Technical:**
- [ ] H.264 encoding working with hardware acceleration
- [ ] Resolution change supported
- [ ] Better video quality than RemoteFX
- [ ] Lower bandwidth usage

**Performance:**
- [ ] 60 FPS capable (vs 30 FPS RemoteFX)
- [ ] 50% bandwidth reduction
- [ ] Sub-frame latency

---

## APPENDIX A: MICROSOFT RDP PROTOCOL STATUS

### Implemented Protocols

| Protocol | Spec | Status | IronRDP | Our Code |
|----------|------|--------|---------|----------|
| **MS-RDPBCGR** | Basic connectivity | ✅ Complete | ✅ | Use IronRDP |
| **MS-RDPECLIP** | Clipboard (text/images) | ✅ 90% | PDUs only | Full logic |
| **MS-RDPECLIP** | Clipboard (files) | ⚠️ 30% | PDUs only | Need to implement |
| **MS-RDPEGFX** | RemoteFX graphics | ✅ Complete | ✅ | Use IronRDP |
| **MS-RDPEI** | Input (mouse/keyboard) | ✅ Complete | ✅ | Use IronRDP |

### Planned Protocols

| Protocol | Spec | Timeline | Complexity |
|----------|------|----------|------------|
| **MS-RDPEGFX** | Graphics pipeline (H.264) | v1.1 | HIGH |
| **MS-RDPEDISP** | Display control | v1.1 | MEDIUM |
| **MS-RDPEA** | Audio output | v2.0+ | MEDIUM |

### Not Planned

| Protocol | Reason |
|----------|--------|
| **MS-RDPEUSB** | USB redirection - specialized use case |
| **MS-RDPEPNP** | Printer - low priority |
| **MS-RDPEAI** | Audio input - defer until validated need |

---

## APPENDIX B: COMPETITIVE ANALYSIS

### Existing Solutions

| Solution | License | Strengths | Weaknesses |
|----------|---------|-----------|------------|
| **xrdp** | GPL | Mature, stable | X11 only, no Wayland |
| **FreeRDP** | Apache 2.0 | Complete protocol | Client-focused, complex |
| **GNOME RDP** | GPL | Native GNOME | Limited features |
| **RustDesk** | AGPL | Modern, Rust | P2P focus, not RDP |

**Our Differentiation:**
- ✅ Native Wayland support (modern Linux)
- ✅ Rust (memory safety, performance)
- ✅ Clean architecture (maintainable)
- ✅ Portal integration (secure)
- ⚠️ Newer (less mature)

---

## CONCLUSION

This project has a **solid technical foundation** (21,500 lines of production-quality Rust) but needs **strategic cleanup and clear product direction** before v1.0 release.

**Immediate Actions Required:**
1. Choose product name ("WayRDP" recommended)
2. Confirm open source strategy (hybrid recommended)
3. Execute documentation consolidation
4. Clean up branches
5. Implement file transfer

**Timeline to v1.0:** 3-4 weeks with focused effort.

**Long-term Vision:** Leading Wayland RDP solution with both desktop and enterprise offerings.

**Next Step:** Review this document, make critical decisions, begin execution.

---

**END OF STRATEGIC PLAN**
