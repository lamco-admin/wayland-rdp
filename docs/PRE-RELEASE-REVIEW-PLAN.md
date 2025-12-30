# Pre-Release Review Plan: lamco-rdp-server v1.0

**Date:** 2025-12-30
**Purpose:** Comprehensive review before migrating codebase to clean repository

---

## Overview

This document outlines the complete review process for lamco-rdp-server before its first release. The review is divided into two major phases:

1. **Phase 1: Structural Review** (Execute Now)
   - Architecture and boundary audit
   - Upstream PR strategy (IronRDP, openh264-rs)
   - Open-source crate publication status
   - AVC444 placement decision
   - Premium feature isolation verification

2. **Phase 2: Code Quality Audit** (Execute Later)
   - Dead code identification
   - Efficiency review
   - Clarity and documentation
   - Functionality verification
   - Security audit

---

## Phase 1: Structural Review

### 1.1 IronRDP Upstream Strategy

#### Current Fork Status
- **Location:** `/home/greg/wayland/IronRDP/`
- **Branch:** `combined-egfx-file-transfer`
- **Commits ahead of upstream:** 17 commits (+8,877 lines across 54 files)

#### Open Pull Requests

| PR # | Title | Status | Priority | Dependencies |
|------|-------|--------|----------|--------------|
| #1057 | EGFX Graphics Pipeline Extension | Under Review | Critical | None |
| #1063 | reqwest Feature Fix | Open | High | None |
| #1064 | Clipboard Locking Methods | Open | High | #1063 |
| #1065 | request_file_contents Method | Open | High | #1064 |
| #1066 | SendFileContentsResponse Variant | Open | High | #1065 |

#### Changes Not Yet Submitted as PRs

1. **ZGFX O(1) Hash Table Optimization** (commits `a0eacc50`, `4a93ffae`, `57608dad`)
   - 100-1000x compression speedup
   - Ready for PR submission
   - **Action:** Create new PR

2. **Hybrid Architecture for EGFX** (commit `70d30654`)
   - Enables proactive H.264 streaming
   - **Decision needed:** Upstream or keep in lamco-rdp-server?

#### Recommended Actions

1. Monitor PR #1057 review - address any feedback from CBenoit
2. Submit ZGFX optimization as separate PR
3. Follow up on #1063 if no activity in 48 hours
4. Decide on Hybrid Architecture placement

---

### 1.2 openh264-rs Upstream Strategy

#### Current Fork Status
- **Location:** `/home/greg/openh264-rs/`
- **Branch:** `feature/vui-support`
- **Commits ahead:** 2 committed + uncommitted changes

#### Implemented Features

1. **VUI Support (Committed)**
   - Complete color space signaling for H.264
   - ColorPrimaries, TransferCharacteristics, MatrixCoefficients enums
   - VuiConfig presets: bt709(), bt709_full(), bt601(), srgb(), bt2020()
   - ~230 lines of new code

2. **NUM_REF_FRAMES Extension (Uncommitted)**
   - Controls reference frame buffer (1-16 frames)
   - Critical for AVC444 dual-stream encoding
   - Fully documented

3. **TEMPORAL_LAYERS Extension (Uncommitted)**
   - Temporal scalability (1-4 layers)
   - Enables frame rate scalability
   - Supports AVC444 auxiliary stream as non-reference

#### Recommended Actions

1. Commit uncommitted changes to `openh264/src/encoder.rs`
2. Submit PR to upstream `ralfbiedert/openh264-rs`
3. Run test suite before submission

---

### 1.3 Publishing Pipeline (lamco-admin)

#### Central Repository Status
- **Location:** `~/lamco-admin/`
- **Purpose:** Governs all publishing operations
- **Git Status:** 2 uncommitted changes

#### Uncommitted Changes in lamco-admin
1. **Modified:** `upstream/ironrdp/prs/PR-BRANCHES-STATUS.md`
   - Updated to track PR #1057 (EGFX)
   - Notes pending work on ZGFX and Hybrid Architecture

2. **Untracked:** `upstream/openh264-rs/prs/` directory
   - New PR tracking infrastructure for openh264-rs

#### Three-Repository Workflow
```
SOURCE REPOS (Private)          ADMIN (Pipeline)           PUBLIC (Published)
wrd-server-specs        →       lamco-admin        →       lamco-wayland
                                (documentation)             lamco-rdp
                                (staging)                   netkit-kotlin
```

#### Published Versions (Current)

**Rust Crates (8 total on crates.io):**
- lamco-wayland repo: 4 crates (v0.2.2)
- lamco-rdp repo: 4 crates (v0.4.0)

**Kotlin Libraries:**
- NetKit: v1.0.1 on JitPack (Maven Central in progress)

---

### 1.5 Lamco Open-Source Crates Status

#### lamco-wayland Workspace (CLEAN)
| Crate | Version | Published | Status |
|-------|---------|-----------|--------|
| lamco-wayland | 0.2.2 | Yes | Current |
| lamco-portal | 0.2.2 | Yes | Current |
| lamco-pipewire | 0.1.3 | Yes | Current |
| lamco-video | 0.1.2 | Yes | Current |

**Git Status:** Clean, no uncommitted changes, fully synced

#### lamco-rdp-workspace (NEEDS ATTENTION)
| Crate | Version | Published | Status |
|-------|---------|-----------|--------|
| lamco-rdp-input | 0.1.1 | Yes | Current |
| lamco-clipboard-core | 0.5.0 (local) / 0.4.0 (crates.io) | Partial | **NEEDS PUBLISH** |
| lamco-rdp-clipboard | 0.2.2 | Yes | Current |
| lamco-pipewire | 0.1.3 (local) | Untracked | **NEEDS GIT ADD** |

**Uncommitted Changes:**
- lamco-clipboard-core v0.5.0: DIBV5 support (CF_DIBV5 format 17)
- lamco-pipewire: Entire directory untracked

**Blocking Issue:** Cannot publish to crates.io until IronRDP v0.5.0 releases

#### Recommended Actions

1. Commit lamco-clipboard-core v0.5.0 changes
2. Git-track lamco-pipewire in lamco-rdp-workspace
3. Prepare for publication once IronRDP blocker resolves

---

### 1.6 AVC444 Placement Decision

#### Current Location
- `src/egfx/avc444_encoder.rs` (~1,418 lines)
- `src/egfx/color_convert.rs` (BGRA→YUV444)
- `src/egfx/yuv444_packing.rs` (dual YUV420 views)

#### Analysis

**Arguments for keeping in lamco-rdp-server (BSL-1.1):**
1. Licensing alignment - AVC444 orchestration provides proprietary value
2. Tight coupling to video_handler.rs and IronRDP's GraphicsPipelineServer
3. No external consumers anticipated
4. Documented decision in EGFX-INTEGRATION-PLAN.md

**Arguments against moving to lamco-video:**
1. lamco-video is MIT/Apache-2.0 (would require relicensing)
2. Would add openh264 and ironrdp dependencies to lamco-video
3. Breaks lamco-video's clean scope (pixel format conversion only)
4. Published crate stability concerns

#### Decision: KEEP IN LAMCO-RDP-SERVER

The existing architectural decision is correct:
- **Protocol layer** (ironrdp-egfx): MIT/Apache-2.0 (upstream)
- **Orchestration** (video_handler, encoder, avc444): BSL-1.1 (proprietary)

---

### 1.7 Architecture Boundary Audit

#### Layer Structure

```
┌─────────────────────────────────────────────────────────────────┐
│ Layer 5: Application (BSL-1.1)                                  │
│   src/server/mod.rs - Server orchestration                      │
│   src/config/ - Configuration system                            │
│   src/services/ - Service Advertisement Registry (Premium)      │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│ Layer 4: Features (BSL-1.1)                                     │
│   src/performance/ - Adaptive FPS, Latency Governor (Premium)   │
│   src/cursor/ - Cursor Prediction (Premium)                     │
│   src/clipboard/ - Clipboard sync                               │
│   src/multimon/ - Multi-monitor support                         │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│ Layer 3: Video Encoding (BSL-1.1)                               │
│   src/egfx/ - H.264 encoding pipeline                           │
│   ├── encoder.rs - AVC420 wrapper                               │
│   ├── avc444_encoder.rs - AVC444 dual-stream (Premium)          │
│   ├── color_convert.rs - BGRA→YUV                               │
│   ├── yuv444_packing.rs - YUV444 dual views                     │
│   └── handler.rs - GraphicsPipelineHandler                      │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│ Layer 2: Integration (Mixed licensing)                          │
│   lamco-portal (MIT/Apache) - XDG Portal                        │
│   lamco-pipewire (MIT/Apache) - Screen capture                  │
│   lamco-video (MIT/Apache) - Frame processing                   │
│   lamco-rdp-* (MIT/Apache) - RDP utilities                      │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│ Layer 1: Protocol (MIT/Apache-2.0 - Upstream)                   │
│   ironrdp-* - RDP protocol handling                             │
│   ironrdp-egfx - Graphics pipeline (PR #1057)                   │
│   openh264 - H.264 codec                                        │
└─────────────────────────────────────────────────────────────────┘
```

#### Premium Features Inventory

| Feature | Location | Isolation Status |
|---------|----------|------------------|
| AVC444 Encoding | src/egfx/avc444_encoder.rs | Isolated, opt-in |
| Adaptive FPS | src/performance/adaptive_fps.rs | Isolated module |
| Latency Governor | src/performance/latency_governor.rs | Isolated module |
| Cursor Prediction | src/cursor/predictor.rs + strategy.rs | Isolated module |
| Service Registry | src/services/ (5 files) | Isolated module |

#### Boundary Verification Checklist

- [ ] No IronRDP types exported in public API
- [ ] Premium features behind feature flags or config options
- [ ] Clear separation between open-source and proprietary layers
- [ ] No license-incompatible code mixing
- [ ] Dependencies properly isolated per layer

---

## Phase 2: Code Quality Audit (Future)

### 2.1 Dead Code Analysis
- Unused functions and modules
- Unused feature flags
- Orphaned test utilities
- Commented-out code blocks

### 2.2 Efficiency Review
- Hot path optimization opportunities
- Memory allocation patterns
- Async task efficiency
- Lock contention analysis

### 2.3 Clarity and Documentation
- Missing documentation
- Unclear function names
- Complex logic without comments
- API consistency

### 2.4 Functionality Verification
- Edge case handling
- Error recovery paths
- Configuration validation
- Feature interaction testing

### 2.5 Security Audit
- Input validation
- Buffer handling
- Credential management
- Network exposure

---

## Summary of Actions

### Immediate (Phase 1 - Today)

1. **IronRDP:**
   - Submit ZGFX optimization PR
   - Monitor PR #1057 review status
   - Document Hybrid Architecture decision

2. **openh264-rs:**
   - Commit uncommitted encoder changes
   - Prepare upstream PR

3. **Lamco crates:**
   - Commit lamco-clipboard-core v0.5.0
   - Git-track lamco-pipewire

4. **Architecture:**
   - Verify boundary checklist
   - Confirm AVC444 placement

### Blocked (Waiting on Dependencies)

- Publish lamco-rdp-workspace to crates.io (needs IronRDP v0.5.0)
- Switch wrd-server-specs from fork to published IronRDP

### Future (Phase 2)

- Complete code quality audit
- Address findings before v1.0 release
- Prepare clean repository migration

---

## Appendix: File Locations

### IronRDP Fork
- `/home/greg/wayland/IronRDP/`
- Key files: `crates/ironrdp-egfx/`, `crates/ironrdp-cliprdr/`, `crates/ironrdp-graphics/src/zgfx/`

### openh264-rs Fork
- `/home/greg/openh264-rs/`
- Key file: `openh264/src/encoder.rs`

### Lamco Workspaces
- `/home/greg/wayland/lamco-wayland/` (published, clean)
- `/home/greg/wayland/lamco-rdp-workspace/` (needs commits)

### Main Codebase
- `/home/greg/wayland/wrd-server-specs/`
- Premium features: `src/egfx/avc444_encoder.rs`, `src/performance/`, `src/cursor/`, `src/services/`
