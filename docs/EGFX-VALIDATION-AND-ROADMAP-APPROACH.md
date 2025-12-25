# EGFX Implementation - Validation & Roadmap Approach

**Date:** 2025-12-24
**Strategy:** Validate hypothesis → Build roadmap → Execute systematically

---

## Phase 1: Quick Validation Test (30 minutes)

**Status:** ✅ IMPLEMENTED - Ready for testing

**Code Change:**
- File: `src/server/display_handler.rs:538`
- Changed: `frames_sent * 33` → `frames_sent * 37`
- Effect: 30fps → 27fps
- Commit: Uncommitted (validation test only)

---

### Test Details

### Hypothesis to Validate

**Current Issue:**
- Windows client receives H.264 frames but sends zero FRAME_ACKNOWLEDGE PDUs
- Backpressure stuck at 3 frames in flight
- Connection drops after 4+ seconds

**Hypothesis:**
The H.264 Level 3.2 constraint violation (120,000 MB/s > 108,000 MB/s limit) is causing Windows Media Foundation decoder to silently reject the stream.

### Validation Test: Reduce FPS to 27

**Change Required:**
```rust
// File: src/server/display_handler.rs
// Current: ~30fps (33ms intervals)
let timestamp_ms = (frames_sent * 33) as u64;

// Change to: 27fps (37ms intervals)
let timestamp_ms = (frames_sent * 37) as u64;
```

**Math:**
- 1280×800 = 4,000 macroblocks
- 4,000 MBs × 27 fps = 108,000 MB/s ← **Exactly Level 3.2 limit!**
- OpenH264 should select Level 3.2 (or we stay with its current selection)
- Windows decoder should accept the stream

### Success Criteria

If hypothesis is **CORRECT**:
- ✅ FRAME_ACKNOWLEDGE PDUs appear in logs
- ✅ Backpressure varies (not stuck at 3)
- ✅ Connection stays stable indefinitely
- ✅ Windows client displays video correctly

If hypothesis is **INCORRECT**:
- ❌ Still no frame ACKs
- ❌ Still crashes
- → Need to investigate other causes (SPS parameters, stream structure, etc.)

### Test Procedure

1. Make one-line FPS change
2. Rebuild: `cargo build --release`
3. Deploy: `scp binary to VM`
4. Run: `~/run-server.sh`
5. Connect with Windows mstsc
6. Observe logs for:
   - `grep "on_frame_ack\|FRAME_ACKNOWLEDGE" log`
   - `grep "backpressure" log | tail -20`
   - Connection stability (should NOT crash)

### Expected Log Output (If Working)

```
TRACE lamco_rdp_server::egfx::handler: EGFX: Frame 0 acknowledged, client queue depth: 0
TRACE lamco_rdp_server::egfx::handler: EGFX: Frame 1 acknowledged, client queue depth: 0
TRACE ironrdp_egfx::server: EGFX backpressure cleared frames_in_flight=2
TRACE ironrdp_egfx::server: EGFX backpressure cleared frames_in_flight=1
...
```

---

## Phase 2: Build Comprehensive Roadmap (After Validation)

### Roadmap Scope

Based on validation results, create detailed implementation roadmap covering:

1. **Level Management System**
   - H264Level enum and constraints (✅ Created)
   - LevelAwareEncoder with C API access (⏳ Created, needs fixes)
   - Dynamic level selection per resolution
   - Validation and auto-adjustment

2. **ZGFX Compression** (IronRDP PR)
   - Research project scope and timeline
   - Implementation approach
   - Testing strategy
   - Upstream contribution process

3. **Damage-Aware Encoding**
   - PipeWire damage extraction
   - Multi-region encoding architecture
   - Rectangle merging algorithm
   - Integration with encoder

4. **Framerate Regulation**
   - Dynamic FPS adjustment
   - Level constraint integration
   - Telemetry and monitoring

5. **Multi-Resolution Support**
   - Configuration matrix
   - Resolution change handling
   - Quality adaptation
   - Client capability negotiation

### Roadmap Structure

For each feature:
- **Dependencies:** What must be done first
- **Complexity:** Time/effort estimate
- **Risk:** Technical unknowns
- **Priority:** Critical path vs optimization
- **Testing:** Validation approach
- **Documentation:** User/developer docs needed

### Sequencing Principles

1. **Unblock first:** Fix what prevents basic functionality
2. **Validate assumptions:** Test hypotheses before big implementations
3. **Upstream before downstream:** File IronRDP PRs before depending on them
4. **Simple before complex:** Get basic working before optimizations
5. **Research before commit:** ZGFX needs side project investigation

---

## Phase 3: Execute Roadmap (Systematic Implementation)

### Execution Approach

1. **Follow roadmap sequence strictly**
2. **Validate at each milestone**
3. **Document as we go**
4. **Adjust roadmap based on learnings**
5. **Keep TODO list synced with roadmap**

---

## Current Status

### Completed
- ✅ Comprehensive diagnostic analysis
- ✅ RFX_RECT encoding verification (correct from start)
- ✅ H264Level system design and implementation
- ✅ LevelAwareEncoder design (needs compilation fixes)
- ✅ Identified all optimization opportunities
- ✅ Three comprehensive analysis documents

### In Progress
- ⏳ Quick validation test preparation (27fps)

### Blocked
- 🚫 Level 4.0 testing (until encoder_ext compiles)
- 🚫 ZGFX implementation (until roadmap prioritizes it)
- 🚫 Damage tracking (until roadmap sequences it)

---

## Next Immediate Actions

1. **Make 27fps change** (one line)
2. **Build and deploy**
3. **Test and observe**
4. **Document results**
5. **Build roadmap** based on validation outcome

---

## Decision Point

Based on 27fps test results:

**If frame ACKs appear:**
→ Confirms level constraints are the issue
→ Roadmap prioritizes Level 4.0 configuration
→ Other optimizations are enhancements

**If still no ACKs:**
→ Level constraints aren't the (only) issue
→ Need deeper investigation before roadmap
→ May need to examine SPS structure, timing, or other factors

---

## Time Estimate

- **Phase 1 (Validation):** 30 minutes
  - Code change: 2 minutes
  - Build/deploy: 5 minutes
  - Test: 5 minutes
  - Analysis: 18 minutes

- **Phase 2 (Roadmap):** 2-3 hours
  - Feature breakdown: 45 minutes
  - Dependency analysis: 30 minutes
  - Sequencing and prioritization: 45 minutes
  - Documentation: 60 minutes

- **Phase 3 (Execution):** Depends on roadmap scope
  - Will be estimated in roadmap itself

---

## Commitment

This approach ensures we:
- ✅ Validate before investing in complex solutions
- ✅ Plan before executing (avoid rework)
- ✅ Sequence features properly (ZGFX "at the proper time")
- ✅ Build production-quality architecture
- ✅ Support wide range of configurations (720p-4K, 24-60fps)
- ✅ Meet all specifications (H.264 levels, MS-RDPEGFX)

Let's start with the quick validation test now.
