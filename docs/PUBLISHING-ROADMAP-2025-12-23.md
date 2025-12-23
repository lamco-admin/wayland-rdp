# Publishing Roadmap and Architecture Compliance Report

**Date:** 2025-12-23
**Purpose:** Architecture audit results and publishing execution plan
**Status:** Ready to execute

---

## Executive Summary

This document presents the results of a comprehensive architecture audit across all lamco repositories, confirming compliance with the intended layered crate design, and provides a step-by-step publishing roadmap.

**Audit Result:** ✅ COMPLIANT - All code is properly placed according to architectural boundaries.

---

## Part 1: Repository Overview

### Repository Matrix

| Repository | GitHub Location | Purpose | License | Current State |
|------------|-----------------|---------|---------|---------------|
| **lamco-wayland** | `lamco-admin/lamco-wayland` | Portal, PipeWire, Video | MIT/Apache-2.0 | ✅ Committed & pushed |
| **lamco-rdp** | `lamco-admin/lamco-rdp` | Input, Clipboard | MIT/Apache-2.0 | ✅ Clean |
| **lamco-rdp-server** | `lamco-admin/wayland-rdp` | Main RDP server | BSL-1.1 | ✅ Committed & pushed |
| **IronRDP fork** | `glamberson/IronRDP` | Upstream patches | MIT/Apache-2.0 | ✅ Pushed to fork |

### Crate Hierarchy

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        COMMERCIAL LAYER (BSL-1.1)                        │
│                                                                          │
│  lamco-rdp-server (lamco-admin/wayland-rdp)                             │
│  └── Orchestration glue: Portal ↔ RDP bridging                          │
│      11,588 lines - main server application                             │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┴───────────────┐
                    ▼                               ▼
┌──────────────────────────────────┐  ┌──────────────────────────────────┐
│   OPEN SOURCE: lamco-wayland    │  │    OPEN SOURCE: lamco-rdp        │
│   (MIT/Apache-2.0)               │  │    (MIT/Apache-2.0)              │
│                                  │  │                                  │
│  lamco-portal v0.2.0 → v0.2.1   │  │  lamco-clipboard-core v0.2.0    │
│  └── XDG Portal D-Bus           │  │  └── Format conversion           │
│                                  │  │  └── Loop detection              │
│  lamco-pipewire v0.1.2 → v0.1.3 │  │  └── FileDescriptor (NEW)        │
│  └── PipeWire video capture     │  │                                  │
│                                  │  │  lamco-rdp-clipboard v0.2.0     │
│  lamco-video v0.1.1 → v0.1.2    │  │  └── IronRDP CLIPRDR integration │
│  └── Frame processing           │  │                                  │
│                                  │  │  lamco-rdp-input v0.1.0         │
│  lamco-wayland v0.2.0 → v0.2.1  │  │  └── Keyboard/mouse handling     │
│  └── Meta crate                 │  │                                  │
└──────────────────────────────────┘  └──────────────────────────────────┘
                    │                               │
                    └───────────────┬───────────────┘
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    UPSTREAM: IronRDP (Devolutions)                       │
│                                                                          │
│  Fork: glamberson/IronRDP                                               │
│  Branch: cliprdr-request-file-contents (4 commits, 157 lines)           │
│  Branch: egfx-server-complete (PR #1057 open)                           │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Part 2: Architecture Compliance Audit

### 2.1 lamco-wayland Crates ✅ COMPLIANT

| Crate | Responsibility | Lines | Verdict |
|-------|---------------|-------|---------|
| lamco-portal | XDG Desktop Portal D-Bus integration | ~95KB | ✅ Proper scope |
| lamco-pipewire | PipeWire screen capture with DMA-BUF | ~3,400 | ✅ Proper scope |
| lamco-video | Video frame processing | ~1,735 | ✅ Proper scope |

**Key Principle:** These crates abstract Linux desktop infrastructure (Portal, PipeWire) without any RDP-specific code.

### 2.2 lamco-rdp Crates ✅ COMPLIANT

| Crate | Responsibility | Lines | Verdict |
|-------|---------------|-------|---------|
| lamco-clipboard-core | Format conversion, loop detection, sanitization | ~107KB | ✅ Protocol-agnostic |
| lamco-rdp-clipboard | IronRDP CLIPRDR channel integration | ~23KB | ✅ RDP bridge layer |
| lamco-rdp-input | Keyboard/mouse coordinate transformation | ~108KB | ✅ Input utilities |

**Key Principle:** These crates provide RDP protocol utilities that could be reused by any RDP implementation.

**Recent Addition (Already Committed):**
- `FileDescriptor` struct for FILEDESCRIPTORW parsing/building (871 lines)
- `sanitize.rs` module for cross-platform filename handling
- Format constants for file transfer

### 2.3 lamco-rdp-server ✅ COMPLIANT

| Module | Lines | Purpose | Verdict |
|--------|-------|---------|---------|
| src/clipboard/ | 3,939 | Portal ↔ RDP clipboard orchestration | ✅ Glue code |
| src/server/ | ~2,000 | RDP server lifecycle management | ✅ Server logic |
| src/egfx/ | ~1,800 | EGFX/H.264 integration | ✅ Server feature |
| src/multimon/ | ~600 | Multi-monitor layout | ✅ Server feature |
| src/security/ | ~800 | TLS/authentication | ✅ Server config |
| src/config/ | ~400 | Configuration loading | ✅ Server config |
| src/rdp/ | ~200 | RDP channel setup | ✅ Server glue |
| **Total** | 11,588 | | |

**Note on clipboard/manager.rs (2,580 lines):**
This is large but appropriate because it's a **state machine coordinator** handling:
- Event routing between Portal and RDP
- File transfer state tracking
- Format negotiation orchestration
- Echo/loop prevention

The actual protocol handling is delegated to the lamco crates.

### 2.4 IronRDP Fork ✅ COMPLIANT

**Branch:** `cliprdr-request-file-contents`
**Commits:** 4 (157 lines across 4 files)

| File | Changes | Purpose |
|------|---------|---------|
| ironrdp-cliprdr/src/backend.rs | +41 | SendFileContentsRequest/Response message variants |
| ironrdp-cliprdr/src/lib.rs | +93 | request_file_contents() method |
| ironrdp-server/Cargo.toml | +1 | reqwest feature flag |
| ironrdp-server/src/server.rs | +24 | Message handlers |

**Assessment:** Clean, focused patches ready for upstream PR. Follows IronRDP code patterns.

**Separate Branch:** `egfx-server-complete`
- PR #1057 is open at Devolutions/IronRDP
- Complete EGFX implementation (1,527 lines server code)
- Waiting for maintainer review

---

## Part 3: Publishing Roadmap

### Phase 1: IronRDP Upstream PRs

**Priority:** HIGH - Unblocks downstream publishing

#### Step 1.1: cliprdr-request-file-contents PR
```
Branch: cliprdr-request-file-contents
Target: Devolutions/IronRDP master
Content:
- SendFileContentsRequest message variant
- SendFileContentsResponse message variant
- request_file_contents() method
- Server handlers

Status: Pushed to fork, needs PR creation
```

#### Step 1.2: EGFX PR Status Check
```
Branch: egfx-server-complete
PR: #1057 (already open)
Status: Awaiting review
```

### Phase 2: lamco-wayland Publishing

**Dependencies:** None (can proceed immediately)

#### Step 2.1: Version Bumps
| Crate | Current | Target | Change Type |
|-------|---------|--------|-------------|
| lamco-portal | 0.2.0 | 0.2.1 | Patch (bug fix) |
| lamco-pipewire | 0.1.2 | 0.1.3 | Patch (improvements) |
| lamco-video | 0.1.1 | 0.1.2 | Patch (dependency) |
| lamco-wayland | 0.2.0 | 0.2.1 | Patch (meta) |

#### Step 2.2: Publishing Order
```bash
# 1. lamco-pipewire (no deps on other lamco crates)
cd crates/lamco-pipewire
# bump version in Cargo.toml to 0.1.3
cargo publish

# 2. lamco-portal (no deps on other lamco crates)
cd crates/lamco-portal
# bump version in Cargo.toml to 0.2.1
cargo publish

# 3. lamco-video (depends on lamco-pipewire)
cd crates/lamco-video
# bump version, update lamco-pipewire dep to 0.1.3
cargo publish

# 4. lamco-wayland meta crate
cd ../..
# bump version to 0.2.1, update all deps
cargo publish
```

### Phase 3: lamco-rdp Publishing

**Dependencies:** None (can proceed immediately)

#### Step 3.1: Version Bumps
| Crate | Current | Target | Change Type |
|-------|---------|--------|-------------|
| lamco-clipboard-core | 0.2.0 | 0.2.1 | Patch (new features) |
| lamco-rdp-clipboard | 0.2.0 | 0.2.0 | No change |
| lamco-rdp-input | 0.1.0 | 0.1.0 | No change |
| lamco-rdp | 0.2.0 | 0.2.1 | Patch (meta) |

#### Step 3.2: Publishing Order
```bash
# 1. lamco-clipboard-core (new FileDescriptor feature)
cd crates/lamco-clipboard-core
# bump version to 0.2.1
cargo publish

# 2. lamco-rdp-clipboard (if needed)
# No changes, may skip

# 3. lamco-rdp-input (if needed)
# No changes, may skip

# 4. lamco-rdp meta crate
cd ../..
# bump version to 0.2.1
cargo publish
```

### Phase 4: lamco-rdp-server Updates

**Dependencies:** Phases 2 and 3 complete, IronRDP PR merged (or use git deps)

#### Step 4.1: Update Dependencies
```toml
# Cargo.toml - switch from path to published versions
[dependencies]
lamco-portal = { version = "0.2.1", features = ["dbus-clipboard"] }
lamco-pipewire = "0.1.3"
lamco-video = "0.1.2"
lamco-clipboard-core = { version = "0.2.1", features = ["image"] }
lamco-rdp-clipboard = "0.2.0"
lamco-rdp-input = "0.1.0"

# IronRDP - either published version or git
# Option A: If PR merged and published
ironrdp = "0.6"  # or whatever version includes our changes

# Option B: Continue with git deps
ironrdp = { git = "https://github.com/glamberson/IronRDP", branch = "cliprdr-request-file-contents" }
```

#### Step 4.2: Test Build
```bash
cargo build --release
cargo test
```

#### Step 4.3: Deploy for Testing
```bash
./test-kde.sh deploy
# Test video, clipboard, file transfer
```

---

## Part 4: Execution Checklist

### Immediate Actions ✅ COMPLETED
- [x] Commit lamco-wayland FD ownership fix
- [x] Push lamco-wayland to origin/master
- [x] Commit lamco-rdp-server documentation
- [x] Push lamco-rdp-server to origin/main

### Next Actions (IronRDP)
- [ ] Create PR for cliprdr-request-file-contents branch
- [ ] Check status of EGFX PR #1057
- [ ] Address any review feedback

### Publishing Actions (After IronRDP)
- [ ] Bump lamco-wayland crate versions
- [ ] Publish lamco-wayland crates to crates.io
- [ ] Bump lamco-rdp crate versions
- [ ] Publish lamco-rdp crates to crates.io
- [ ] Update lamco-rdp-server dependencies
- [ ] Test full integration
- [ ] Prepare lamco-rdp-server for clean repo migration

---

## Part 5: Risk Assessment

### Low Risk Items
- lamco-wayland publishing (no external deps)
- lamco-rdp publishing (no external deps)
- Documentation updates

### Medium Risk Items
- IronRDP PR review timeline (depends on Devolutions)
- crates.io indexing delays (~2 minutes typical)

### Mitigation Strategies
- Can use git dependencies for IronRDP if PR takes time
- All crates can be published independently
- Local path deps allow continued development

---

## Part 6: Repository Commit Status

### lamco-wayland
```
Commit: 1b9b213
Branch: master
Remote: lamco-admin/lamco-wayland
Status: ✅ Up to date with origin
```

### lamco-rdp-workspace
```
Commit: fc9646b (FileDescriptor support)
Branch: main
Remote: lamco-admin/lamco-rdp
Status: ✅ Clean, ready to publish
```

### lamco-rdp-server (wrd-server-specs)
```
Commit: eec3b97
Branch: main
Remote: lamco-admin/wayland-rdp
Status: ✅ Up to date with origin
```

### IronRDP Fork
```
Commit: 8462968b
Branch: cliprdr-request-file-contents
Remote: glamberson/IronRDP
Status: ✅ Pushed, awaiting PR creation
```

---

## Appendix: Key Files Reference

### lamco-wayland Critical Fix
- `crates/lamco-portal/src/remote_desktop.rs` - FD ownership (OwnedFd → RawFd)
- `crates/lamco-portal/src/session.rs` - RawFd field type
- `crates/lamco-pipewire/src/pw_thread.rs` - Debug logging, AUTOCONNECT handling

### lamco-rdp New Features
- `crates/lamco-clipboard-core/src/formats.rs` - FileDescriptor struct
- `crates/lamco-clipboard-core/src/sanitize.rs` - Cross-platform filename handling

### IronRDP Patches
- `crates/ironrdp-cliprdr/src/backend.rs` - Message variants
- `crates/ironrdp-cliprdr/src/lib.rs` - request_file_contents()
- `crates/ironrdp-server/src/server.rs` - Handlers

---

**Document Version:** 1.0
**Last Updated:** 2025-12-23
**Next Review:** After IronRDP PR submission
