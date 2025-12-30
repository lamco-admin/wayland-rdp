# Structural Analysis: lamco-rdp-server Pre-Release Review

**Date:** 2025-12-30
**Purpose:** Critical and exhaustive analysis of architecture, boundaries, and upstream dependencies

---

## Executive Summary

This analysis examines the structural integrity of the lamco-rdp-server codebase before its first release. The review covers:

1. **IronRDP Upstream Strategy** - 6 PRs pending, 1 under review
2. **openh264-rs Upstream Strategy** - 3 features ready for PR
3. **Lamco Crates Publication** - 8 published, 2 pending updates
4. **AVC444 Placement** - Confirmed: keep in BSL-1.1 codebase
5. **Architecture Boundaries** - 5 layers, premium features properly isolated

### Critical Findings

| Area | Status | Action Required |
|------|--------|-----------------|
| IronRDP PRs | 6 open, 1 under review | Submit ZGFX PR, monitor #1057 |
| openh264-rs | 3 features uncommitted | Commit and submit PR |
| lamco-clipboard-core | v0.5.0 uncommitted | Commit DIBV5 changes |
| lamco-pipewire (rdp-ws) | Untracked directory | Git add |
| lamco-admin | 2 uncommitted changes | Commit PR tracking updates |
| Architecture | Sound | Verify boundary checklist |

---

## 1. IronRDP Upstream Analysis

### 1.1 Current Fork State

**Fork Location:** `/home/greg/wayland/IronRDP/`
**Branch:** `combined-egfx-file-transfer`
**Divergence:** 17 commits ahead (+8,877 lines, 54 files)

### 1.2 Pull Request Status

| PR | Title | Size | Status | CI | Blocking? |
|----|-------|------|--------|----| --------- |
| #1057 | EGFX Graphics Pipeline | ~6,265 lines | Under Review | ✅ | **Yes** - critical for H.264 |
| #1063 | reqwest Feature Fix | Small | Open | ✅ | No |
| #1064 | Clipboard Locking | Medium | Open | ✅ | Depends on #1063 |
| #1065 | request_file_contents | Medium | Open | ✅ | Depends on #1064 |
| #1066 | SendFileContentsResponse | Medium | Open | ✅ | Depends on #1065 |

**PR Chain:** #1063 → #1064 → #1065 → #1066 (sequential merge required)

### 1.3 Unsubmitted Changes

#### ZGFX O(1) Optimization (READY FOR PR)
- **Commits:** `a0eacc50`, `4a93ffae`, `57608dad`
- **Impact:** 100-1000x compression speedup
- **Lines:** ~2,233
- **Tests:** All 46 ZGFX tests pass
- **Status:** Production-ready, should submit immediately

#### Hybrid Architecture for EGFX
- **Commit:** `70d30654`
- **Purpose:** Enables proactive H.264 streaming via EGFX DVC
- **Components:** GfxDvcBridge, ServerEvent::Egfx, build_server_with_handle()
- **Decision:** Could be upstream or kept in lamco-rdp-server

### 1.4 Recommendations

1. **Submit ZGFX PR immediately** - Independent of #1057, high value
2. **Monitor #1057 review** - CBenoit is reviewing, address feedback promptly
3. **Follow up on #1063** - Simple fix, should merge quickly
4. **Document Hybrid Architecture decision** - Upstream vs local

---

## 2. openh264-rs Upstream Analysis

### 2.1 Current Fork State

**Fork Location:** `/home/greg/openh264-rs/`
**Branch:** `feature/vui-support`
**Status:** 2 commits + uncommitted changes

### 2.2 Implemented Features

#### VUI Support (COMMITTED)
- **Purpose:** Color space signaling for H.264
- **Types:** ColorPrimaries, TransferCharacteristics, MatrixCoefficients
- **Presets:** bt709(), bt709_full(), bt601(), srgb(), bt2020()
- **Lines:** ~230
- **Status:** Ready for upstream PR

#### NUM_REF_FRAMES Extension (UNCOMMITTED)
- **Purpose:** Reference frame buffer control (1-16 frames)
- **Critical for:** AVC444 dual-stream encoding
- **Status:** Fully implemented with documentation

#### TEMPORAL_LAYERS Extension (UNCOMMITTED)
- **Purpose:** Temporal scalability (1-4 layers)
- **Enables:** Frame rate scalability, non-reference frame marking
- **Status:** Fully implemented with documentation

### 2.3 Recommendations

1. **Commit uncommitted changes** to `openh264/src/encoder.rs`
2. **Run test suite** to verify no regressions
3. **Submit single PR** with all three features to `ralfbiedert/openh264-rs`
4. **Track in lamco-admin** - already have `upstream/openh264-rs/prs/` directory

---

## 3. Lamco Crates Publication Analysis

### 3.1 Publication Status Overview

**Central Tracking:** `~/lamco-admin/`

| Repository | Crates | Latest Version | crates.io | Notes |
|------------|--------|----------------|-----------|-------|
| lamco-wayland | 4 | v0.2.2 | ✅ Current | Clean |
| lamco-rdp | 4 | v0.4.0 | ⚠️ Outdated | Pending v0.5.0 |

### 3.2 lamco-wayland Workspace (CLEAN)

**Location:** `/home/greg/wayland/lamco-wayland/`
**Git Status:** Clean, fully synced

| Crate | Version | Status |
|-------|---------|--------|
| lamco-wayland | 0.2.2 | Published |
| lamco-portal | 0.2.2 | Published |
| lamco-pipewire | 0.1.3 | Published |
| lamco-video | 0.1.2 | Published |

**No action required.**

### 3.3 lamco-rdp-workspace (NEEDS ATTENTION)

**Location:** `/home/greg/wayland/lamco-rdp-workspace/`
**Git Status:** Modified files + untracked directory

#### Uncommitted Changes

1. **lamco-clipboard-core v0.5.0**
   - Feature: DIBV5 support (CF_DIBV5 format 17)
   - Impact: Transparent image clipboard support
   - Files: `formats.rs`, `image.rs`
   - Tests: 7 new tests included

2. **lamco-pipewire (untracked)**
   - Status: Entire `crates/lamco-pipewire/` directory untracked
   - Version: 0.1.3 (matches published on crates.io)
   - Note: Already published from lamco-wayland repo!

#### Blocking Issue

Cannot publish lamco-rdp-workspace to crates.io until IronRDP v0.5.0 releases:
- Current strategy: Uses `git` dependency on glamberson/IronRDP fork
- crates.io requires published dependencies

### 3.4 lamco-admin Repository

**Location:** `~/lamco-admin/`
**Git Status:** 2 uncommitted changes

1. **Modified:** `upstream/ironrdp/prs/PR-BRANCHES-STATUS.md`
   - Added PR #1057 tracking
   - Updated total PR count

2. **Untracked:** `upstream/openh264-rs/prs/`
   - New PR tracking directory

### 3.5 Recommendations

1. **lamco-rdp-workspace:**
   - Commit lamco-clipboard-core v0.5.0 changes
   - Clarify lamco-pipewire situation (duplicate across repos?)

2. **lamco-admin:**
   - Commit PR tracking updates
   - Set up openh264-rs PR tracking

3. **Publication timing:**
   - Wait for IronRDP v0.5.0 release
   - Then publish all pending crate updates

---

## 4. AVC444 Placement Analysis

### 4.1 Current Location

```
src/egfx/
├── avc444_encoder.rs  (~1,418 lines) - Dual H.264 encoding
├── color_convert.rs   (~1,200 lines) - BGRA→YUV444 SIMD
├── yuv444_packing.rs  (~1,100 lines) - Dual YUV420 views
└── encoder.rs         (~665 lines)   - OpenH264 wrapper
```

Total: ~4,400 lines of premium encoding code

### 4.2 Decision Matrix

| Factor | Keep in BSL-1.1 | Move to lamco-video |
|--------|-----------------|---------------------|
| Licensing | ✅ Aligns with premium value | ❌ Requires relicensing to MIT |
| Coupling | ✅ Tight integration with video_handler | ❌ Would need abstraction layer |
| Dependencies | ✅ Uses ironrdp internally | ❌ Would add ironrdp to MIT crate |
| External consumers | ✅ None anticipated | ❌ No demand exists |
| Maintenance | ✅ Single codebase | ❌ Two codebases to maintain |
| Existing decision | ✅ EGFX-INTEGRATION-PLAN.md | ❌ Would reverse decision |

### 4.3 Decision: KEEP IN LAMCO-RDP-SERVER

**Rationale:**
- AVC444 orchestration represents proprietary value (BSL-1.1 appropriate)
- No external consumers need this code
- Tight coupling with IronRDP EGFX makes extraction complex
- Previous architectural decision confirmed

**lamco-video scope remains:**
- Pixel format conversion (BgrX32, Bgr24, etc.)
- Frame rate limiting
- Damage region tracking
- Multi-stream dispatch

---

## 5. Architecture Boundary Audit

### 5.1 Layer Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│ L5: APPLICATION LAYER (BSL-1.1)                                 │
│                                                                 │
│   Server Orchestration:                                         │
│   └── src/server/mod.rs           Main server loop              │
│   └── src/server/display_handler.rs Frame processing            │
│   └── src/server/input_handler.rs   Input translation           │
│                                                                 │
│   Configuration:                                                │
│   └── src/config/                   TOML configuration          │
│                                                                 │
│   Premium - Service Registry:                                   │
│   └── src/services/registry.rs      Capability advertising      │
│   └── src/services/wayland_feature.rs Compositor features       │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│ L4: FEATURE LAYER (BSL-1.1)                                     │
│                                                                 │
│   Premium - Performance:                                        │
│   └── src/performance/adaptive_fps.rs   Dynamic FPS             │
│   └── src/performance/latency_governor.rs Latency modes         │
│                                                                 │
│   Premium - Cursor:                                             │
│   └── src/cursor/predictor.rs       Physics prediction          │
│   └── src/cursor/strategy.rs        Cursor modes                │
│                                                                 │
│   Open Features:                                                │
│   └── src/clipboard/                Clipboard sync              │
│   └── src/multimon/                 Multi-monitor               │
│   └── src/damage/                   Tile-based detection        │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│ L3: VIDEO ENCODING LAYER (BSL-1.1)                              │
│                                                                 │
│   Premium - AVC444:                                             │
│   └── src/egfx/avc444_encoder.rs    Dual H.264 streams          │
│   └── src/egfx/color_convert.rs     BGRA→YUV444 SIMD            │
│   └── src/egfx/yuv444_packing.rs    YUV444→dual YUV420          │
│                                                                 │
│   Standard - AVC420:                                            │
│   └── src/egfx/encoder.rs           OpenH264 wrapper            │
│   └── src/egfx/handler.rs           GraphicsPipelineHandler     │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│ L2: INTEGRATION LAYER (MIT/Apache-2.0)                          │
│                                                                 │
│   lamco-portal (0.2.2)    - XDG Desktop Portal                  │
│   lamco-pipewire (0.1.3)  - PipeWire screen capture             │
│   lamco-video (0.1.2)     - Frame processing                    │
│   lamco-rdp-* (0.4.0)     - RDP utilities                       │
│   lamco-clipboard-core    - Format conversion                   │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│ L1: PROTOCOL LAYER (MIT/Apache-2.0 - Upstream)                  │
│                                                                 │
│   ironrdp-* (fork)        - RDP protocol handling               │
│   ironrdp-egfx (PR #1057) - Graphics pipeline                   │
│   openh264 (fork)         - H.264 codec                         │
│   pipewire, zbus, ashpd   - System integration                  │
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 Premium Feature Isolation

| Feature | Module | Config Flag | Isolation Quality |
|---------|--------|-------------|-------------------|
| AVC444 | src/egfx/avc444_encoder.rs | `video.encoder_type = "avc444"` | ✅ Good - separate encoder |
| Adaptive FPS | src/performance/adaptive_fps.rs | `performance.adaptive_fps.*` | ✅ Good - isolated module |
| Latency Governor | src/performance/latency_governor.rs | `performance.latency.mode` | ✅ Good - isolated module |
| Cursor Prediction | src/cursor/ | `cursor.mode = "predictive"` | ✅ Good - strategy pattern |
| Service Registry | src/services/ | Always active | ⚠️ Note - no opt-out |

### 5.3 Boundary Verification Checklist

**Verified by code inspection (2025-12-30):**

- [x] **No IronRDP types in public API** - lib.rs only re-exports lamco crates (MIT/Apache)
- [x] **License separation** - BSL-1.1 code clearly separate from MIT/Apache
- [x] **No license mixing** - MIT code doesn't depend on BSL code
- [x] **Clean dependency direction** - Upper layers depend on lower, not reverse
- [x] **Feature flags for optional deps** - h264, vaapi, nvenc all feature-gated

**All verified complete:**

- [x] **Premium feature config sections exist** - Full config in `config/types.rs`:
  - `AdaptiveFpsConfig` (60fps support, activity thresholds) - lines 132-209
  - `LatencyConfig` (interactive/balanced/quality modes) - lines 211-274
  - `CursorConfig` + `CursorPredictorConfig` (predictive physics) - lines 276-410
  - `EgfxConfig` with AVC444 aux omission settings - lines 549-702
- [x] **Service Registry** - Always active (architectural decision, no opt-out needed)
- [x] **Premium features always compiled** - Runtime config controls behavior

### 5.4 Dependency Flow

```
lamco-rdp-server (BSL-1.1)
    │
    ├──► ironrdp-* (fork, MIT) ──► upstream after PRs merge
    │       └── PR #1057 (EGFX)
    │       └── PR #1064-1066 (clipboard)
    │
    ├──► openh264 (fork, BSD) ──► upstream after PR
    │       └── VUI support
    │       └── NUM_REF_FRAMES
    │       └── TEMPORAL_LAYERS
    │
    ├──► lamco-wayland/* (MIT/Apache)
    │       └── lamco-portal
    │       └── lamco-pipewire
    │       └── lamco-video
    │
    └──► lamco-rdp/* (MIT/Apache)
            └── lamco-clipboard-core
            └── lamco-rdp-clipboard
            └── lamco-rdp-input
```

---

## 6. Issue Summary

### 6.1 Critical Issues (Must Fix)

| Issue | Location | Action | Priority |
|-------|----------|--------|----------|
| ZGFX PR not submitted | IronRDP fork | Submit PR | High |
| openh264-rs uncommitted | openh264/src/encoder.rs | Commit | High |
| lamco-clipboard-core v0.5.0 | lamco-rdp-workspace | Commit | High |

### 6.2 Moderate Issues (Should Fix)

| Issue | Location | Action | Priority |
|-------|----------|--------|----------|
| lamco-pipewire untracked | lamco-rdp-workspace | Clarify ownership | Medium |
| lamco-admin uncommitted | ~/lamco-admin | Commit tracking | Medium |
| Hybrid Architecture decision | IronRDP fork | Document decision | Medium |

### 6.3 Low Priority (Nice to Have)

| Issue | Location | Action | Priority |
|-------|----------|--------|----------|
| Service Registry no opt-out | src/services/ | Add config flag | Low |
| crates.io publication blocked | lamco-rdp | Wait for IronRDP v0.5.0 | Low |

---

## 7. Action Plan

### Immediate Actions (Today)

1. **IronRDP - Submit ZGFX PR**
   ```bash
   cd /home/greg/wayland/IronRDP
   git checkout -b zgfx-optimization
   git cherry-pick a0eacc50 4a93ffae 57608dad
   git push fork zgfx-optimization
   # Create PR on GitHub
   ```

2. **openh264-rs - Commit and prepare PR**
   ```bash
   cd /home/greg/openh264-rs
   git add openh264/src/encoder.rs
   git commit -m "feat(encoder): add NUM_REF_FRAMES and TEMPORAL_LAYERS support"
   git push fork feature/vui-support
   # Create PR on GitHub
   ```

3. **lamco-rdp-workspace - Commit changes**
   ```bash
   cd /home/greg/wayland/lamco-rdp-workspace
   git add crates/lamco-clipboard-core/
   git commit -m "feat(clipboard-core): add DIBV5 support for transparent images"
   git push
   ```

4. **lamco-admin - Commit tracking updates**
   ```bash
   cd ~/lamco-admin
   git add upstream/
   git commit -m "docs: update PR tracking for EGFX and openh264-rs"
   git push
   ```

### Short-term Actions (This Week)

1. Monitor IronRDP PR #1057 review
2. Follow up on PR #1063 if no activity
3. Document Hybrid Architecture decision
4. Clarify lamco-pipewire ownership (lamco-wayland vs lamco-rdp)

### Blocked Actions (Waiting)

1. Publish lamco-rdp-workspace to crates.io → needs IronRDP v0.5.0
2. Switch from fork to upstream IronRDP → needs PRs merged

---

## 8. Conclusion

The lamco-rdp-server codebase has a **sound architectural foundation** with clear layer separation and proper premium feature isolation. The main structural concerns are:

1. **Uncommitted work** across multiple repositories that should be committed
2. **Unsubmitted PRs** (ZGFX optimization) that are production-ready
3. **Publication blockers** due to upstream IronRDP release timing

The AVC444 placement decision to keep encoding in the BSL-1.1 codebase is correct and should remain unchanged.

**Overall Assessment:** Ready for Phase 2 code quality audit after addressing the immediate action items above.
