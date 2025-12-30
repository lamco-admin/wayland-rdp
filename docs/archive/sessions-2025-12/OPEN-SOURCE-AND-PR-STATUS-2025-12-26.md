# Open Source & Upstream PR Status Report

**Date:** 2025-12-26
**Purpose:** Status of open source crates and pending IronRDP upstream contributions

---

## Summary

| Area | Status | Action Required |
|------|--------|-----------------|
| **wrd-server-specs** | ✅ Pushed | None |
| **lamco-rdp-workspace** | ✅ Clean | None |
| **IronRDP PRs** | ⚠️ 5 Open | Follow up on review |
| **IronRDP New PRs** | 🔴 2 Needed | Submit ZGFX + Server PRs |

---

## 1. lamco-rdp-workspace (Published Crates)

**Repository:** `/home/greg/wayland/lamco-rdp-workspace/`
**Status:** ✅ Up to date with origin, no changes needed

### Published Crates on crates.io

| Crate | Version | Description |
|-------|---------|-------------|
| `lamco-rdp` | latest | Meta-crate with feature flags |
| `lamco-rdp-input` | latest | uinput keyboard/mouse input |
| `lamco-clipboard-core` | latest | Clipboard abstraction layer |
| `lamco-rdp-clipboard` | latest | RDP clipboard channel integration |

### Recent Updates (Already Published)
- RTF and synthesized text format support
- Privacy flag handling
- Publication compatibility with IronRDP fork

### No Updates Needed
The workspace is clean and up-to-date. The dev-only change (path deps to local IronRDP) was correctly not committed.

---

## 2. IronRDP Fork Status

**Repository:** `/home/greg/wayland/IronRDP/`
**Fork:** `https://github.com/glamberson/IronRDP`
**Upstream:** `https://github.com/Devolutions/IronRDP`
**Current Branch:** `combined-egfx-file-transfer`

### Existing Open PRs (Awaiting Review)

| PR # | Title | Branch | Status | Lines |
|------|-------|--------|--------|-------|
| **#1057** | feat(egfx): add MS-RDPEGFX Graphics Pipeline Extension | `egfx-server-complete` | OPEN | +1400 |
| **#1063** | fix(server): enable reqwest feature for ironrdp-tokio dependency | `fix-server-reqwest-feature` | OPEN | Small |
| **#1064** | feat(cliprdr): add clipboard data locking methods | `pr1-clipboard-lock-unlock` | OPEN | Medium |
| **#1065** | feat(cliprdr): add request_file_contents method | `pr2-request-file-contents` | OPEN | Medium |
| **#1066** | feat(cliprdr): add SendFileContentsResponse message variant | `pr3-file-contents-response` | OPEN | Medium |

**Merged:**
- **#1053** `fix(cliprdr): allow servers to announce clipboard ownership` ✅

### Action Required: Follow Up on Open PRs

All 5 PRs have been open since 2025-12-16 to 2025-12-23. Consider:
1. Pinging maintainers for review
2. Addressing any review comments
3. Rebasing if upstream has advanced

---

## 3. NEW PRs to Submit

### PR Candidate #1: ZGFX Compression Optimizations (CRITICAL)

**Priority:** HIGH - Fixes exponential slowdown bug

**Commits to Include:**
```
a0eacc50 feat(zgfx): implement O(1) hash table optimization for compression
4a93ffae fix(zgfx): prevent duplicate hash table entries causing exponential slowdown
57608dad feat(zgfx): add hash table size limits and compaction
```

**Files Changed:**
- `crates/ironrdp-graphics/src/zgfx/compressor.rs` - Core optimizations
- `crates/ironrdp-graphics/src/zgfx/api.rs` - New public API
- `crates/ironrdp-graphics/src/zgfx/wrapper.rs` - Wrapper for ease of use
- `crates/ironrdp-graphics/src/zgfx/circular_buffer.rs` - Buffer improvements
- `crates/ironrdp-egfx/src/server.rs` - Integration

**Total:** +1,493 lines, -27 lines

**Impact:**
- Fixes critical performance bug where duplicate hash table entries caused O(n²) behavior
- Implements O(1) hash table lookup for compression
- Adds size limits and compaction to prevent memory bloat

**Recommended PR Title:**
`fix(zgfx): optimize compression with O(1) hash table and fix duplicate entry bug`

---

### PR Candidate #2: EGFX Server Integration

**Priority:** MEDIUM - Extends existing EGFX PR

**Commits to Include:**
```
3938b4fa feat(ironrdp-server): add EGFX (Graphics Pipeline) server integration
70d30654 feat(server): implement Hybrid architecture for proactive EGFX frame sending
60c3add8 fix: update ironrdp-graphics version to 0.7 for compatibility
```

**Files Changed:**
- `crates/ironrdp-server/src/gfx.rs` - Graphics handling (+186 lines)
- `crates/ironrdp-server/src/server.rs` - Server integration (+80 lines)
- `crates/ironrdp-server/src/builder.rs` - Builder pattern
- `crates/ironrdp-server/Cargo.toml` - Dependencies

**Total:** +302 lines

**Note:** This may overlap with or extend PR #1057 (EGFX). Consider:
- Option A: Add as additional commits to PR #1057
- Option B: Submit as separate follow-up PR after #1057 merges

**Recommended Approach:** Wait for PR #1057 to merge, then submit as follow-up

---

## 4. Recommended Actions

### Immediate (This Week)

1. **Submit ZGFX PR** - Critical bug fix, should be submitted ASAP
   ```bash
   cd /home/greg/wayland/IronRDP
   git checkout -b zgfx-optimization origin/master
   git cherry-pick a0eacc50 4a93ffae 57608dad
   git push fork zgfx-optimization
   gh pr create --title "fix(zgfx): optimize compression with O(1) hash table and fix duplicate entry bug"
   ```

2. **Follow up on existing PRs** - Ping maintainers on #1057, #1063-1066

### After EGFX PR (#1057) Merges

3. **Submit Server Integration PR** - The Hybrid architecture and EGFX server integration

### Ongoing

4. **Keep fork synced** - Regularly rebase on upstream master
5. **Monitor IronRDP releases** - Update lamco-rdp-workspace when new versions publish

---

## 5. Dependency Notes

### Current IronRDP Usage

**wrd-server-specs uses:**
- Local IronRDP fork (combined-egfx-file-transfer branch)
- Required for EGFX, ZGFX, and clipboard features

**lamco-rdp-workspace uses:**
- Git dependency to glamberson/IronRDP fork (master branch)
- Published to crates.io with fallback to upstream

### Future State (After PRs Merge)

Once all PRs are merged upstream:
1. Update wrd-server-specs to use upstream IronRDP
2. Update lamco-rdp-workspace to use upstream/crates.io
3. Remove local fork dependency

---

## 6. Uncommitted IronRDP Changes

The fork has uncommitted changes on `combined-egfx-file-transfer`:

```
modified:   crates/ironrdp-egfx/src/pdu/avc.rs
modified:   crates/ironrdp-server/src/gfx.rs
modified:   crates/ironrdp-server/src/server.rs
```

**Review needed:** Determine if these are:
- Work in progress (should be committed)
- Temporary dev changes (should be discarded)
- Additional features for future PRs

---

*Report generated 2025-12-26*
