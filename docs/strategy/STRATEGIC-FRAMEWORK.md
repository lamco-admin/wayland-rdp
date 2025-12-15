# Strategic Framework
## Product, Crate, and Protocol Architecture
**Date:** 2025-12-11
**Purpose:** Comprehensive strategy for IronRDP relationship, crate structure, products, and branding

---

## PART 1: IRONRDP PROTOCOL COMPLIANCE ASSESSMENT

### Microsoft Protocol Specification Currency

| MS Specification | Current Version | Published | IronRDP Crate | IronRDP Status |
|------------------|----------------|-----------|---------------|----------------|
| **MS-RDPBCGR** (Core RDP) | 61.0 | April 7, 2025 | ironrdp-pdu, ironrdp-connector | Unknown version, likely older |
| **MS-RDPECLIP** (Clipboard) | 16.0 | April 23, 2024 | ironrdp-cliprdr | V1/V2 capability support (older) |
| **MS-RDPEDISP** (Display Control) | 10.0 | April 23, 2024 | ironrdp-displaycontrol | PDUs only, "processing (TODO)" |
| **MS-RDPEA** (Audio Output) | 20.0 | April 23, 2024 | ironrdp-rdpsnd | Client playback only |
| **MS-RDPEFS** (File System) | Unknown | Unknown | ironrdp-rdpdr | Client-side only |
| **MS-RDPEGFX** (Graphics Pipeline/H.264) | Unknown | Unknown | ironrdp-pdu/gfx | PDU structures only (~20%) |

**Key Finding:** IronRDP does NOT claim to implement current spec versions. They implement:
- Protocol wire format (PDU encode/decode)
- Client-side logic
- Minimal or no server-side business logic

**MS-RDPBCGR is at version 61.0** (April 2025) - very actively maintained by Microsoft.

**Implication:** IronRDP provides **wire format compatibility** but not necessarily **full spec compliance** or **latest features**.

---

### IronRDP Graphics Support Analysis

**What They Have:**
- **RDP 6.0 bitmap compression** (rdp6/rle.rs) - Full implementation
- **RemoteFX** (dwt.rs, quantization.rs, rlgr.rs) - Full encoder/decoder
- **ZGFX compression** - Full implementation
- **Basic bitmap** - Full implementation

**What They DON'T Have:**
- **H.264/AVC420** - PDU structures only, no codec integration
- **AVC444** - PDU structures only, no codec integration
- **Graphics Pipeline (RDPEGFX) business logic** - Only wire format

**Graphics Capability Sets Supported:**
From GFX PDU analysis: V8, V8.1, V10, V10.1, V10.2, V10.3, V10.4, V10.5, V10.6, V10.7

These are capability versions for graphics pipeline negotiation, but **no actual H.264 encoding/decoding implementation exists**.

**GFX Status (PR #648):**
- Draft PR from Red Hat developer (elmarco)
- Open for 11 months (Jan 2025)
- Checklist shows: AVC codecs NOT implemented, server support NOT implemented
- Stalled - no recent activity

**Conclusion:** You WILL implement H.264 yourself. IronRDP has no path to this.

---

## PART 2: DIVISION OF WORK - IRONRDP VS YOUR CODE

### Current Reality of Your Implementation

**Your wrd-server uses:**

**From IronRDP (working):**
- Core protocol connection (MS-RDPBCGR) - TLS, NLA, capability exchange
- PDU encode/decode (all protocols)
- RemoteFX encoder (temporary - deprecated)
- Basic channel management
- Input PDU types
- **Clipboard PDUs** (with your 1-commit patch)

**Your Implementation (not IronRDP):**
- Clipboard loop prevention logic
- Clipboard format conversion (UTF-8↔UTF-16, PNG↔JPEG, etc.)
- FIFO request/response correlation
- PipeWire screen capture (dedicated thread, DMA-BUF)
- Portal integration (ScreenCast, RemoteDesktop, Clipboard APIs)
- Input event translation (scancode→keysym, 200+ mappings)
- Multi-monitor coordinate transformation
- Server orchestration (multiplexer, queues, batching)
- Frame rate regulation (token bucket)
- Graphics coalescing

**What You WILL Implement (IronRDP doesn't have it):**
- H.264 encoding (VA-API hardware acceleration)
- RDPEGFX business logic (surface management, frame ack, caching)
- File transfer logic (FileGroupDescriptorW parsing, streaming)
- Multi-monitor layout calculation
- Display resolution change handling (PipeWire stream reconfig)

---

### Clean Division Strategy

```
┌─────────────────────────────────────────────────────────────┐
│                     YOUR PRODUCT STACK                      │
└─────────────────────────────────────────────────────────────┘

LAYER 1: Protocol Foundation (Use IronRDP)
├── Core RDP connection (MS-RDPBCGR) ✅ ironrdp-connector
├── TLS/NLA security ✅ ironrdp-tls
├── PDU encode/decode ✅ ironrdp-pdu
├── Channel framework ✅ ironrdp-svc, ironrdp-dvc
└── Server skeleton ✅ ironrdp-server (with your 1-commit fork)

LAYER 2: Protocol Extensions (Your Crates - Potentially Open Source)
├── lamco-rdp-clipboard (MS-RDPECLIP business logic)
│   ├── Uses: ironrdp-cliprdr PDUs
│   ├── Implements: Loop prevention, format conversion, FileContents
│   └── Decision: Open source? (7,800 lines, high reuse value)
│
├── lamco-rdp-input (Input translation)
│   ├── Uses: ironrdp-pdu input types
│   ├── Implements: Scancode mapping, coordinate transform
│   └── Decision: Open source? (3,700 lines, generic utility)
│
├── lamco-rdp-egfx (H.264 Graphics Pipeline)
│   ├── Uses: ironrdp-pdu GFX structures
│   ├── Implements: H.264 encoder integration, surface mgmt, frame ack
│   └── Decision: Open source later? OR proprietary? (will be ~5,000 lines)
│
└── lamco-rdp-multimon (Multi-monitor)
    ├── Uses: ironrdp-displaycontrol PDUs
    ├── Implements: Layout calculation, resolution change
    └── Decision: Proprietary? (enterprise feature)

LAYER 3: Platform Integration (Your Crates - Potentially Open Source)
├── lamco-portal (XDG Portal integration)
│   ├── Uses: ashpd
│   ├── Implements: ScreenCast, RemoteDesktop, Clipboard portal APIs
│   └── Decision: Open source? (600 lines, ecosystem value)
│
├── lamco-pipewire (PipeWire screen capture)
│   ├── Uses: pipewire-rs
│   ├── Implements: Dedicated thread, DMA-BUF, multi-stream
│   └── Decision: Open source? (3,700 lines, high technical value)
│
└── lamco-video (Video processing)
    ├── Uses: None
    ├── Implements: Pixel conversion, encoder abstraction
    └── Decision: Open source? (1,500 lines, generic)

LAYER 4: Application Orchestration (Proprietary Products)
├── lamco-rdp-portal-server (Portal mode - free for non-commercial)
│   ├── Uses: All above layers
│   ├── Implements: Multiplexer, batching, frame regulation, Portal lifecycle
│   └── License: Free for non-commercial, paid for commercial
│
└── lamco-rdp-vdi-server (Headless VDI mode - commercial)
    ├── Uses: All protocol + platform layers + compositor
    ├── Implements: Multi-user, session management, enterprise features
    └── License: Commercial only

LAYER 5: Compositor (Decision Required)
└── lamco-compositor (Headless Wayland compositor)
    ├── Uses: Smithay
    ├── Implements: Headless rendering, multi-user isolation
    └── Decision: Open source? OR proprietary?
```

---

## PART 3: BRANCH EVALUATION AND CONSOLIDATION

### Current Branches

| Branch | Commits | Status | Decision |
|--------|---------|--------|----------|
| **main** | Base | Production | Keep |
| **feature/gnome-clipboard-extension** | 14 ahead | Current working branch | **Merge to main** |
| **feature/smithay-compositor** | 1 | Documentation only | **Evaluate work** |
| **feature/lamco-compositor-clipboard** | 10 | Compositor + X11 backend | **Evaluate work** |
| **feature/headless-infrastructure** | 1 | Documentation only | **Archive** |
| feature/clipboard-monitoring-solution | ? | Old work | **Archive or merge** |
| feature/embedded-portal | ? | Experimental | **Evaluate** |
| feature/wlr-clipboard-backend | ? | Alternative backend | **Evaluate** |

**Action Required:**
1. Merge feature/gnome-clipboard-extension to main (current working state)
2. Review feature/lamco-compositor-clipboard for compositor work
3. Archive documentation-only branches
4. Consolidate or delete experimental branches

---

## PART 4: PRODUCT ARCHITECTURE DECISIONS

### Product Line Strategy

**PRODUCT 1: Lamco RDP Portal Server** (Free for non-commercial)
```
What: Wayland RDP server using XDG Portals
Mode: Portal mode (requires existing compositor)
Target: Linux desktop users, developers, home users
License: Free for non-commercial use, paid for commercial
Binary: lamco-rdp-portal OR wrd-server (name TBD)
Features:
  - Screen sharing via Portal
  - Clipboard (bidirectional)
  - Input (keyboard/mouse)
  - H.264 encoding (v1.1+)
  - Multi-monitor (basic)
```

**PRODUCT 2: Lamco RDP VDI Server** (Commercial only)
```
What: Headless VDI solution (no physical display required)
Mode: Compositor mode (built-in Smithay compositor)
Target: Enterprise VDI, cloud workspaces, multi-user servers
License: Commercial license (per-seat or subscription)
Binary: lamco-rdp-vdi OR lamco-vdi-server (name TBD)
Features:
  - All Portal Server features PLUS:
  - Multi-user sessions
  - Headless rendering (no X11/Wayland compositor needed)
  - Session isolation
  - Enterprise authentication (SSO, LDAP integration hooks)
  - Advanced multi-monitor (display topology)
  - Management API
```

**PRODUCT 3: Lamco Compositor** (Decision: Open source OR part of VDI)
```
What: Headless Wayland compositor (Smithay-based)
Use: Backend for VDI Server
Decision Options:
  A) Open source standalone - Reusable headless compositor
  B) Proprietary component - Part of VDI product
  C) Dual model - Basic open source, advanced proprietary
Binary: lamco-compositor (if standalone)
```

---

### Licensing Structure

**Open Source Components (MIT/Apache-2.0 dual):**
```
lamco-portal          (~600 lines)   - XDG Portal integration
lamco-pipewire        (~3,700 lines) - PipeWire screen capture
lamco-video           (~1,500 lines) - Video processing
lamco-rdp-clipboard   (~7,800 lines) - RDP clipboard protocol
lamco-rdp-input       (~3,700 lines) - Input translation
lamco-rdp-protocol    (~150 lines)   - RDP protocol utilities

Total: ~17,450 lines (81% of current codebase)
```

**Proprietary Components (Commercial license):**
```
lamco-rdp-portal-server  (~2,100 lines) - Portal mode orchestration
lamco-rdp-vdi-server     (~2,100 lines + compositor) - VDI orchestration
lamco-rdp-multimon       (~400 lines) - Multi-monitor mgmt
lamco-rdp-security       (~300 lines) - Enterprise security
lamco-compositor         (? lines) - Decision required

Total: ~4,900+ lines (19% of current codebase + compositor)
```

**H.264 Implementation (lamco-rdp-egfx):**
```
Decision Required:
A) Open source after 1 year of exclusive use
B) Keep proprietary (competitive advantage)
C) Open source immediately (ecosystem play)

Estimated: ~5,000 lines
Timeline: 2-3 weeks implementation
```

---

## PART 5: IRONRDP RELATIONSHIP STRATEGY

### What to Use From IronRDP (Dependency)

**Core Dependencies (Fork with 1 commit):**
- ironrdp-server - Server skeleton
- ironrdp-connector/acceptor - Connection handling
- ironrdp-pdu - All PDU types
- ironrdp-cliprdr - Clipboard PDUs (with your patch)
- ironrdp-displaycontrol - Display PDUs
- ironrdp-graphics - RemoteFX (temporary)
- ironrdp-core - Core traits
- ironrdp-svc/dvc - Channel traits

**Fork Strategy:**
- Maintain minimal fork (1 commit: clipboard server initiation)
- Monthly rebase against Devolutions/master
- Fork only for server-specific protocol behavior
- Use upstream for everything else

---

### What to Develop as Separate Crates

**Category A: RDP Protocol Components (Can Submit to IronRDP)**
- **lamco-rdp-egfx** - H.264/RDPEGFX implementation
  - Uses IronRDP's PDU structures
  - Implements surface management, H.264 integration, business logic
  - **Decision:** Develop separately, offer to IronRDP after proven
  - **Rationale:** GFX PR #648 stalled 11 months, they won't implement this
  - **Submission potential:** Medium (if high quality and well-tested)

**Category B: RDP Protocol Logic (Generic, Open Source)**
- **lamco-rdp-clipboard** - Clipboard business logic
  - Loop prevention, format conversion, FileContents
  - **Decision:** Open source immediately
  - **Submission to IronRDP:** Low (too opinionated, server-specific)

- **lamco-rdp-input** - Input translation
  - Scancode mapping, coordinate transforms
  - **Decision:** Open source immediately
  - **Submission to IronRDP:** Low (too specific)

**Category C: Platform Integration (Non-RDP, Open Source)**
- **lamco-portal** - XDG Portal integration
  - **Decision:** Open source OR contribute to ashpd crate
  - **Submission to IronRDP:** None (not RDP-specific)

- **lamco-pipewire** - PipeWire capture
  - **Decision:** Open source (ecosystem value)
  - **Submission to IronRDP:** None (not RDP-specific)

- **lamco-video** - Video processing
  - **Decision:** Open source
  - **Submission to IronRDP:** None (not RDP-specific)

**Category D: Product Orchestration (Proprietary)**
- **lamco-rdp-portal-server** - Portal mode product
  - **Decision:** Proprietary (this is your product)

- **lamco-rdp-vdi-server** - VDI mode product
  - **Decision:** Proprietary (enterprise product)

- **lamco-rdp-multimon** - Multi-monitor logic
  - **Decision:** Proprietary (enterprise feature)

- **lamco-rdp-security** - Enterprise security
  - **Decision:** Proprietary (enterprise hooks)

**Category E: Compositor (Strategic Decision)**
- **lamco-compositor** - Headless Wayland compositor
  - **Decision Options:**
    - A) Open source - Build Smithay ecosystem, attract contributors
    - B) Proprietary - Keep VDI competitive advantage
    - C) Hybrid - Basic open source, advanced features proprietary

---

## PART 6: COMPOSITOR DECISION FRAMEWORK

### Compositor Open Source Analysis

**Work Done (feature/lamco-compositor-clipboard branch):**
- 10 commits including X11 backend and compositor mode integration
- Product packaging architecture documented
- "Compositor mode complete and ready for testing"

**If Open Source Compositor:**

**Pros:**
- Attracts Smithay contributors
- Reference implementation for headless VDI
- Ecosystem benefit (few headless compositors exist)
- Builds credibility and trust
- Community finds bugs and contributes features

**Cons:**
- Competitors can use it (anyone can build VDI on it)
- Lose exclusive technical advantage
- Must support community (issues, PRs, questions)

**If Proprietary Compositor:**

**Pros:**
- Exclusive VDI capability (competitors must build their own)
- Technical moat (years to replicate)
- Full control (no community obligation)

**Cons:**
- No external contributors
- Must maintain alone
- Smithay ecosystem doesn't benefit
- Less trust from enterprise (closed source compositor)

**Hybrid Option:**

**Open source basic compositor:**
- Single-user headless compositor
- Basic window management
- Standard Smithay features

**Proprietary extensions:**
- Multi-user session isolation
- Advanced window policies
- Performance optimizations
- Enterprise integration hooks

**This matches commercial open source model** (like GitLab, Redis, MongoDB).

---

### Recommended Compositor Strategy

**Phase 1 (v1.0):** Portal mode only - No compositor decision needed yet

**Phase 2 (v1.1):** Build VDI with proprietary compositor
- Prove market fit
- Get paying customers
- Establish product

**Phase 3 (v2.0):** **Open source basic compositor**
- After 1 year of exclusive use
- Keep multi-user features proprietary
- Build ecosystem while maintaining advantage

**Rationale:**
- Delay decision until you have market validation
- Open sourcing after success is easier than closing later
- Gives time to identify what's truly differentiating

---

## PART 7: CRATE AND PRODUCT STRUCTURE

### Repository Organization

**Option A: Monorepo (Recommended)**
```
lamco-rdp/
├── crates/
│   ├── lamco-portal/              [Open source]
│   ├── lamco-pipewire/            [Open source]
│   ├── lamco-video/               [Open source]
│   ├── lamco-rdp-clipboard/       [Open source]
│   ├── lamco-rdp-input/           [Open source]
│   ├── lamco-rdp-egfx/            [Open source after 1 year?]
│   ├── lamco-rdp-protocol/        [Open source]
│   ├── lamco-rdp-utils/           [Open source]
│   ├── lamco-rdp-portal-server/   [Proprietary]
│   ├── lamco-rdp-vdi-server/      [Proprietary]
│   ├── lamco-rdp-multimon/        [Proprietary]
│   ├── lamco-rdp-security/        [Proprietary]
│   └── lamco-compositor/          [Decision pending]
├── LICENSE-MIT
├── LICENSE-APACHE
├── LICENSE-COMMERCIAL
└── README.md
```

**Pros:**
- Single repo to manage
- Easy inter-crate development
- Clear licensing per crate
- CI/CD efficiency

**Cons:**
- Mixed open source + proprietary (clear separation required)

**Option B: Split Repos**
```
lamco-rdp-foundation/    [Open source monorepo]
├── lamco-portal
├── lamco-pipewire
├── lamco-video
├── lamco-rdp-clipboard
├── lamco-rdp-input
└── lamco-rdp-protocol

lamco-rdp-server/        [Proprietary monorepo]
├── lamco-rdp-portal-server
├── lamco-rdp-vdi-server
├── lamco-rdp-multimon
├── lamco-rdp-security
└── lamco-compositor
```

**Pros:**
- Clear licensing separation
- Open source repo is clean (no commercial code)
- Can accept external contributions easily

**Cons:**
- Two repos to manage
- Cross-repo dependency management
- More CI/CD complexity

**Recommendation:** **Option A (Monorepo)** - Clear per-crate licensing, easier to manage.

---

## PART 8: BRANDING AND NAMING

### Product Names

**Option Set 1: Technical and Clear**
- **Lamco RDP Server** (Portal mode free edition)
- **Lamco RDP VDI** (Enterprise headless edition)
- **Lamco Compositor** (If standalone)

**Option Set 2: Branded Names**
- **Lamco Portal** (Portal mode)
- **Lamco VDI** (Enterprise mode)
- **Lamco Compositor** (Headless compositor)

**Option Set 3: Descriptive**
- **Lamco Wayland RDP** (Portal mode)
- **Lamco VDI Pro** (Enterprise mode)
- **Lamco Headless** (Compositor)

**Crate Naming Convention:**
```
lamco-{category}-{component}

Examples:
- lamco-portal           (not RDP-specific)
- lamco-pipewire         (not RDP-specific)
- lamco-rdp-clipboard    (RDP protocol component)
- lamco-rdp-egfx         (RDP protocol component)
- lamco-rdp-portal-server (RDP server product)
- lamco-compositor       (not RDP-specific)
```

**Recommendation:**
- **Products:** Lamco RDP Server (free), Lamco VDI (commercial)
- **Crates:** lamco-{category}-{component} pattern
- **Compositor:** Lamco Compositor (if standalone)

---

## PART 9: OPEN SOURCE STRATEGY

### What to Open Source and Why

**HIGH PRIORITY - Open Source Immediately:**

1. **lamco-portal** (600 lines)
   - **Why:** Portal adoption needs examples, ecosystem benefit
   - **Risk:** None (can't bypass Portal security)
   - **Benefit:** Trust, credibility, Portal ecosystem growth

2. **lamco-rdp-clipboard** (7,800 lines)
   - **Why:** Solves hard problem (loop prevention), reference implementation
   - **Risk:** Low (clipboard is expected feature, not differentiator)
   - **Benefit:** Community contributions, bug fixes, format additions

3. **lamco-rdp-input** (3,700 lines)
   - **Why:** Keymap database valuable to community
   - **Risk:** None (standard feature)
   - **Benefit:** Community expands keymaps (international layouts)

**MEDIUM PRIORITY - Open Source After v1.0:**

4. **lamco-pipewire** (3,700 lines)
   - **Why:** Solves non-Send PipeWire problem, technical showcase
   - **Risk:** Medium (good implementation, but temporary advantage)
   - **Benefit:** PipeWire ecosystem, attract contributors
   - **Timeline:** Open source after 6 months exclusive use

5. **lamco-video** (1,500 lines)
   - **Why:** Generic utility, no competitive value
   - **Risk:** None
   - **Benefit:** Small but completes the foundation

**STRATEGIC DECISION - H.264 Implementation:**

6. **lamco-rdp-egfx** (~5,000 lines, will implement)
   - **Option A:** Keep proprietary (competitive advantage for 1-2 years)
   - **Option B:** Open source immediately (submit to IronRDP, build credibility)
   - **Option C:** Open source after 1 year (validate product first)

   **Consideration:** GFX PR #648 stalled for 11 months shows IronRDP won't implement this. If you open source a working H.264/RDPEGFX implementation, you become THE reference for RDP graphics pipeline in Rust.

   **Recommendation:** **Option C** - Prove it works, get customers, then open source (2026).

**NEVER OPEN SOURCE:**

7. **lamco-rdp-portal-server** - This IS your Portal mode product
8. **lamco-rdp-vdi-server** - This IS your VDI product
9. **lamco-rdp-multimon** - Enterprise feature differentiation
10. **lamco-rdp-security** - Enterprise integration hooks

**COMPOSITOR DECISION:**

11. **lamco-compositor**
   - **If Product 2 (VDI) succeeds:** Open source basic compositor after 1 year
   - **If Product 2 fails:** Keep proprietary (no benefit to open sourcing)
   - **Timeline:** Decide after v1.1 (2026)

---

## PART 10: PROTOCOL IMPLEMENTATION ROADMAP

### v1.0 - Portal Mode MVP (Current)

| Protocol | Source | Status |
|----------|--------|--------|
| MS-RDPBCGR (Core) | IronRDP | ✅ Using |
| MS-RDPECLIP (Text/Images) | IronRDP PDUs + your logic | ✅ Working |
| RemoteFX Graphics | IronRDP | ✅ Temporary |
| MS-RDPEI (Input) | IronRDP PDUs + your logic | ✅ Working |

**Product:** Free for non-commercial Portal mode server

---

### v1.1 - H.264 Graphics (Q1 2026)

| Protocol | Source | Work Required |
|----------|--------|---------------|
| **MS-RDPEGFX** | **Your implementation** | 2-3 weeks |
| **H.264/AVC420** | **Your implementation** | Part of RDPEGFX |
| MS-RDPEDISP (Resolution) | IronRDP PDUs + your logic | 2-3 days |
| MS-RDPECLIP (Files) | IronRDP PDUs + your logic | 6-8 hours |

**Product:** Portal mode with modern codecs (deprecate RemoteFX)

**Crate:** lamco-rdp-egfx (decision: proprietary for 1 year, then open source)

---

### v1.2 - VDI Mode (Q2 2026)

| Component | Source | Work Required |
|-----------|--------|---------------|
| **Headless Compositor** | **Evaluate feature/lamco-compositor-clipboard** | ? |
| Multi-user support | **Your implementation** | 2-3 weeks |
| Session isolation | **Your implementation** | 1 week |
| Management API | **Your implementation** | 1 week |

**Product:** Commercial VDI offering

**Crate:** lamco-rdp-vdi-server (proprietary)

---

### v2.0+ - Extended Protocols (2026+)

| Protocol | Priority | Estimated Work |
|----------|----------|----------------|
| MS-RDPEA (Audio output) | Medium | 2-3 weeks |
| MS-RDPEAI (Audio input) | Low | 2-3 weeks |
| MS-RDPEUSB (USB redirect) | Low | 4-6 weeks |
| MS-RDPEFS (Drive redirect) | Medium | 3-4 weeks |

---

## PART 11: DECISION MATRIX

### Critical Decisions to Make Now

| Decision | Options | Impact | Timeline |
|----------|---------|--------|----------|
| **1. Compositor licensing** | Open source / Proprietary / Hybrid | VDI product strategy | Before v1.2 |
| **2. H.264 (lamco-rdp-egfx) licensing** | Proprietary temporary / Open source now | Competitive position | Before implementing |
| **3. Portal vs VDI priority** | Portal first / VDI first / Parallel | Development timeline | This week |
| **4. Branch consolidation** | Merge compositor work / Start fresh | Codebase cleanliness | This week |
| **5. Product naming** | Lamco RDP Server / Other | Branding | Before v1.0 release |
| **6. IronRDP engagement** | Active / Minimal / None | Maintenance burden | Based on PR response |

---

## PART 12: RECOMMENDED STRATEGIC PATH

### Phase 1: Consolidate Current Work (This Week)

**Actions:**
1. **Merge feature/gnome-clipboard-extension to main**
   - This is your working state
   - Clipboard + performance fixes complete
   - Clean IronRDP fork in place

2. **Evaluate feature/lamco-compositor-clipboard**
   - Review 10 commits of compositor work
   - Decide if usable or start fresh
   - Document compositor architecture

3. **Archive experimental branches**
   - feature/headless-infrastructure (docs only)
   - Old clipboard branches if superseded
   - Keep smithay-compositor if has useful work

4. **Document IronRDP strategy**
   - Based on PR response (submitted or pending)
   - Monthly rebase procedure
   - Protocol compliance gaps

**Deliverable:** Clean main branch, clear branch strategy

---

### Phase 2: Extract Open Source Crates (Weeks 2-3)

**Priority order:**
1. lamco-rdp-utils (foundation, no deps)
2. lamco-rdp-protocol (thin wrappers)
3. lamco-video (self-contained)
4. lamco-rdp-input (self-contained)
5. lamco-portal (self-contained)
6. lamco-pipewire (complex - dedicated thread)
7. lamco-rdp-clipboard (most complex - many features)

**Per crate:**
- Create crate in workspace
- Move source files
- Define clean public API
- Write README with examples
- Add basic tests
- rustdoc documentation

**Deliverable:** 7 open source crates ready for publication

---

### Phase 3: Publish Open Source Foundation (Week 4)

**Announcement Strategy:**
- GitHub org: lamco-rdp
- Publish to crates.io (all 7 crates)
- Blog post: "Building a Modern Wayland RDP Server in Rust"
- Reddit: /r/rust, /r/linux, /r/wayland
- Hacker News

**Positioning:**
- Reference Wayland RDP implementation
- Portal-first design (secure)
- Modern Rust architecture
- "We're open sourcing our foundation, building products on top"

**Deliverable:** Public open source presence

---

### Phase 4: Build Products (Ongoing)

**Product 1: Portal Mode (v1.0 - Q1 2026)**
- Binary: lamco-rdp-server (or chosen name)
- License: Free for non-commercial
- Features: Current + file transfer
- Distribution: GitHub releases, package repos (AUR, Flathub)

**Product 2: H.264 Implementation (v1.1 - Q2 2026)**
- Crate: lamco-rdp-egfx
- License: Proprietary initially, open source 2027?
- Timeline: 2-3 weeks implementation
- Replace RemoteFX

**Product 3: VDI Mode (v1.2 - Q3 2026)**
- Binary: lamco-vdi-server (or chosen name)
- License: Commercial (per-seat or subscription)
- Features: Headless compositor, multi-user, enterprise
- Compositor: Proprietary initially, evaluate open source later

---

## PART 13: IRONRDP ENGAGEMENT STRATEGY

### Based on PR Response

**Scenario A: PR Accepted (60% probability)**
- **Implication:** They're receptive to server improvements
- **Strategy:** Collaborative fork
  - Use upstream for all standard features
  - Contribute generic improvements
  - Maintain minimal fork only for server-specific features
  - Consider submitting lamco-rdp-egfx to IronRDP after proven

**Scenario B: PR Rejected or Ignored (40% probability)**
- **Implication:** They're client-focused, minimal server interest
- **Strategy:** Independent fork
  - Maintain permanent fork (monthly rebase)
  - Minimal upstream engagement
  - Develop all server features independently
  - Don't offer lamco-rdp-egfx to IronRDP (they won't want it)
  - Focus on your product, not their ecosystem

---

### What to Submit to IronRDP vs Keep Separate

**Submit to IronRDP (if PR accepted):**
- Protocol correctness fixes (like your clipboard patch)
- Bug fixes (if you find any)
- **Maybe lamco-rdp-egfx** (if they show interest in server features)

**Develop as Lamco Crates (either way):**
- lamco-portal - Not RDP-specific
- lamco-pipewire - Not RDP-specific
- lamco-video - Not RDP-specific
- lamco-rdp-clipboard - Too opinionated for IronRDP
- lamco-rdp-input - Too specific
- lamco-rdp-multimon - Enterprise feature
- lamco-rdp-security - Enterprise hooks
- All server products - Proprietary

**Key Insight:** Most of your value is NOT in RDP protocol implementation, it's in:
- Wayland/Portal integration (platform-specific)
- Product orchestration (how you combine things)
- Enterprise features (multi-user, security)
- Performance optimizations (multiplexer, batching)

**IronRDP is just your wire format library.** Your product is everything else.

---

## PART 14: BUSINESS MODEL

### Revenue Structure

**Free Tier: Lamco RDP Server**
- Portal mode only
- Non-commercial use
- Community edition
- GitHub releases
- **Monetization:** None (lead generation for VDI)
- **Purpose:** Adoption, community, trust

**Paid Tier: Lamco VDI**
- Headless compositor mode
- Multi-user support
- Enterprise features
- Commercial license required
- **Pricing Models:**
  - Per-seat: $50-100/user/year
  - Per-server: $500-1000/server/year
  - Self-hosted: One-time $2000-5000
- **Target:** Cloud VDI providers, enterprises, hosting companies

**Professional Services:**
- Custom integration
- Enterprise support
- Training
- Consulting

---

### Market Positioning

**vs FreeRDP/xrdp (GPL):**
- "Modern Rust implementation with commercial-friendly licensing"
- "Portal-first for maximum Wayland security"
- "Enterprise-ready VDI with support"

**vs GNOME Remote Desktop:**
- "Full RDP protocol support (not limited subset)"
- "Works on any compositor (not GNOME-only)"
- "Enterprise features and commercial support"

**vs RustDesk:**
- "Industry-standard RDP protocol (not proprietary)"
- "Enterprise VDI focus (not P2P consumer)"
- "Open source foundation with commercial products"

---

## PART 15: IMMEDIATE ACTION PLAN

### Week 1: Consolidation and Decision

**Day 1-2: Branch Consolidation**
- [ ] Merge feature/gnome-clipboard-extension to main
- [ ] Review feature/lamco-compositor-clipboard (compositor work)
- [ ] Document smithay-compositor branch status
- [ ] Archive experimental branches
- [ ] Tag current state as v0.9 (pre-release)

**Day 3-4: Strategic Decisions**
- [ ] Review this framework document
- [ ] Decide: Compositor licensing (open source after 1 year recommended)
- [ ] Decide: H.264 licensing (proprietary for 1 year recommended)
- [ ] Decide: Portal vs VDI priority (Portal first recommended)
- [ ] Decide: Product names (Lamco RDP Server / Lamco VDI recommended)
- [ ] Decide: IronRDP engagement (based on PR response)

**Day 5: Documentation**
- [ ] Create PRODUCT-ROADMAP.md (v1.0, v1.1, v1.2 plans)
- [ ] Create LICENSING-STRATEGY.md (per-crate decisions)
- [ ] Create BRANCH-CONSOLIDATION.md (which branches to keep/archive)
- [ ] Update README.md with product vision

---

### Week 2-3: Crate Extraction

- [ ] Create Cargo workspace
- [ ] Extract open source crates (7 crates)
- [ ] Define clean public APIs
- [ ] Write documentation
- [ ] Basic test coverage
- [ ] CI/CD setup

---

### Week 4: Initial Publication

- [ ] Publish open source crates to crates.io
- [ ] Create GitHub org: lamco-rdp (or lamco)
- [ ] Write announcement blog post
- [ ] Submit to Rust community channels
- [ ] Begin accepting contributions

---

## PART 16: QUESTIONS TO ANSWER

### Compositor Decision (Critical)

**Question:** Open source lamco-compositor or keep proprietary?

**Considerations:**
- You have working compositor code (feature/lamco-compositor-clipboard)
- Smithay ecosystem is small (would benefit from examples)
- VDI competitors could use it (risk)
- Community contributors could improve it (benefit)
- Multi-user isolation can stay proprietary even if basic compositor is open

**Options:**
1. Proprietary always (maintain VDI moat)
2. Open source after 1 year (delayed ecosystem play)
3. Hybrid (basic open, advanced proprietary)

**Recommendation:** Decide after v1.1 (you have time, no rush)

---

### H.264 (lamco-rdp-egfx) Decision (Important)

**Question:** Open source your H.264/RDPEGFX implementation or keep proprietary?

**Context:**
- IronRDP GFX PR #648 stalled 11 months
- No working H.264 in Rust RDP ecosystem
- You'll implement this (~5,000 lines, 2-3 weeks)
- Major technical achievement

**Options:**
1. **Proprietary** - Exclusive advantage (1-2 years lead time for competitors)
2. **Open source immediately** - Become THE Rust RDP graphics reference
3. **Open source after 1 year** - Validate product, then release

**Consideration:** If you're open sourcing most other components, H.264 implementation could be the open source "crown jewel" that drives adoption while you monetize orchestration/VDI.

**Recommendation:** **Option 3** (proprietary for 1 year) - Validate market, then release.

---

### Product Priority (Immediate)

**Question:** Focus on Portal mode (v1.0) or VDI mode (v1.2) first?

**Portal Mode (Recommended):**
- Faster to market (already 90% done)
- Free tier builds user base
- Simpler (no compositor complexity)
- Validates RDP components before VDI

**VDI Mode:**
- Requires compositor work
- Higher complexity
- Smaller initial market
- But higher revenue potential

**Recommendation:** Portal mode first (v1.0 → v1.1), then VDI (v1.2).

---

## PART 17: EXISTING WORK EVALUATION

### Branches to Review

**feature/lamco-compositor-clipboard (10 commits):**
- Content: Compositor + X11 backend + packaging
- Status: "Complete and ready for testing"
- **Action:** Review this work, decide if it's the VDI foundation
- **Questions:** What works? What's missing? Is it production-ready?

**feature/smithay-compositor (1 commit):**
- Content: Documentation only
- **Action:** Archive (no code)

**feature/headless-infrastructure (1 commit):**
- Content: Documentation only
- **Action:** Archive (no code)

**Other clipboard branches:**
- Likely superseded by feature/gnome-clipboard-extension
- **Action:** Compare, archive if obsolete

**Recommendation:** Focus review on feature/lamco-compositor-clipboard - this has actual compositor implementation.

---

## PART 18: STRATEGIC FRAMEWORK SUMMARY

### The Clean Division

**Layer 1: IronRDP (Forked Dependency)**
- Use for: Wire format, core protocol, PDU types
- Fork maintenance: Monthly rebase (1 commit divergence)
- Contribution: Based on PR reception

**Layer 2: Lamco RDP Protocol Crates (Open Source)**
- lamco-rdp-clipboard - Business logic for MS-RDPECLIP
- lamco-rdp-input - Input translation
- lamco-rdp-egfx - H.264/RDPEGFX (proprietary 1 year, then open source)
- lamco-rdp-protocol - Utilities

**Layer 3: Lamco Platform Crates (Open Source)**
- lamco-portal - XDG Portal integration
- lamco-pipewire - Screen capture
- lamco-video - Video processing

**Layer 4: Lamco Products (Proprietary)**
- lamco-rdp-server - Portal mode (free for non-commercial)
- lamco-vdi-server - VDI mode (commercial)
- lamco-compositor - Headless compositor (decision pending)

**Layer 5: Lamco Enterprise (Proprietary)**
- lamco-rdp-multimon - Multi-monitor
- lamco-rdp-security - Enterprise integration

---

### Business Strategy

**Open Source Foundation (81% of code):**
- Build trust and credibility
- Attract contributors
- Become reference implementation
- Generate leads for commercial products

**Commercial Products (19% of code):**
- Portal mode orchestration (free for non-commercial, paid for commercial)
- VDI mode (commercial only)
- Enterprise features and support
- Professional services

**Hybrid Model:** Like GitLab, MongoDB, Redis
- Strong open source foundation
- Clear commercial value-add
- Community + customers
- Sustainable business

---

## NEXT ACTIONS

**Immediate (This Session):**
1. Review this framework
2. Answer the 6 critical questions in Part 11
3. Decide on product priority (Portal first recommended)
4. Decide on product names

**This Week:**
1. Merge current working branch to main
2. Review compositor work (feature/lamco-compositor-clipboard)
3. Create product roadmap based on decisions
4. Begin branch consolidation

**Next Week:**
1. Start crate extraction (if decisions made)
2. OR focus on compositor evaluation (if VDI priority)
3. OR implement file transfer (finish clipboard)

---

**END OF STRATEGIC FRAMEWORK**

Review this framework and make the key decisions. Then we can execute.
