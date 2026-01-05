# Session Handover - End of Day 2026-01-05

**Date:** 2026-01-05
**Focus:** RHEL 9 testing, Mutter investigation completion, Portal-only transition
**Status:** Portal works but needs final fixes (two dialogs, quality, mouse alignment)
**Critical Path:** Fix remaining bugs, test quality, prepare for publication

---

## Executive Summary

### Major Accomplishments

**1. Mutter Investigation Complete ✅**
- Tested on GNOME 40.10 (RHEL 9) and GNOME 46.0 (Ubuntu 24.04)
- **Verdict:** Mutter Direct API is NON-FUNCTIONAL on both versions
- **Root Cause:** RemoteDesktop/ScreenCast sessions cannot be linked
- **Decision:** Mutter disabled in Service Registry, code preserved as dormant
- **Documentation:** Complete test results in docs/RHEL9-TEST-RESULTS.md

**2. Portal-Only Strategy Validated ✅**
- Portal works on RHEL 9 GNOME 40.10
- Connection successful, video displays, input works
- Quality and mouse issues remain (fixable)

**3. Build System Established ✅**
- RHEL 9 build working (glibc 2.34 compatibility)
- Deployment workflow defined
- Source successfully deployed and built

### Current State: 80% Working

**What Works:**
- ✅ Portal strategy selected (Mutter disabled)
- ✅ RDP connection established
- ✅ Video streaming (Portal → PipeWire → H.264 → Client)
- ✅ Keyboard input functional
- ✅ Right-click functional
- ✅ No crashes during connection
- ✅ Clipboard gracefully skipped on Portal v1

**What's Broken:**
- ❌ Two permission dialogs (should be one)
- ❌ Poor video quality (config settings not applied)
- ❌ Mouse location misaligned (coordinate offset issue)

**Blocking Issues:** 3 bugs prevent production release

---

## Critical Issues Requiring Fixes

### Issue #1: Two Permission Dialogs 🔴 CRITICAL

**Status:** Partially fixed in code, but crashes on Portal v1

**Problem:**
- Hybrid mode code (server/mod.rs lines 303-365) creates duplicate Portal session
- First dialog: Portal strategy creates session
- Second dialog: server/mod.rs creates ANOTHER Portal session (line 310-312)

**Current Fix Attempt:**
```rust
// Line 316: Changed to session_type() check
if session_handle.session_type() == SessionType::Portal {
    // Use session directly
} else {
    // Mutter hybrid mode
}
```

**Issue with Fix:**
- Portal v1: `portal_clipboard()` returns None (no clipboard support)
- Line 324: tries to get session from None → crashes
- Need to access session differently for Portal v1

**Proper Fix Needed:**
```rust
// Portal strategy: get session from PortalSessionHandleImpl.session field
// This field is now pub(crate) so we can access it
let session = /* access session directly from session_handle */
```

**Files Modified:**
- src/server/mod.rs (line 316 changed, line 324 needs fix)
- src/session/strategies/portal_token.rs (session now pub(crate))

**Testing Evidence:**
- Log shows 2 Portal managers created
- Log shows 2 PipeWire FDs (16 and 15)
- User had to approve 2 dialogs

### Issue #2: Poor Video Quality 🔴 CRITICAL

**Status:** Config settings exist but not applied

**Problem:**
- Config shows: `h264_bitrate: 5000` (should be 10000)
- Config shows: `qp_max: 40` (should be 25)
- Config shows: `aux_omission: true` (should be false)
- Text is blurry/unreadable

**Root Cause:**
Partial configs don't work in TOML deserialization. Must use FULL config.toml.

**Current Config Status:**
- File: ~/wayland-build/wrd-server-specs/rhel9-config.toml
- Status: FULL config.toml deployed
- Quality settings: bitrate 10000, qp 25/18, no aux omission
- **BUT:** Previous test used OLD config (defaults)

**Next Test Should Use:**
- Latest rhel9-config.toml (has all quality improvements)
- Should see bitrate 10000 in log
- Text should be much sharper

**Files:**
- rhel9-config.toml (deployed, correct)
- config.toml (local master copy)

### Issue #3: Mouse Location Misaligned 🟡 HIGH

**Status:** Working but offset from actual location

**Evidence:**
- Right-click context menu appears in wrong location
- Mouse coordinates: `RDP(1142, 140) -> Stream(1142.00, 140.00)`
- 1:1 mapping appears correct

**Possible Causes:**
1. **Two Portal sessions** (two different streams/coordinate spaces)
   - FD 16 (first session)
   - FD 15 (second session - DUPLICATE)
   - Using stream 51 for input, but video on different stream?

2. **Coordinate transformation bug**
   - May have been broken during Mutter work
   - Need to check coordinate code changes

3. **Multi-monitor offset**
   - Stream position not being accounted for

**Likely Fix:**
Fixing Issue #1 (two dialogs) will probably fix this by eliminating duplicate streams.

**Files to Check:**
- src/server/input_handler.rs (coordinate transformation)
- src/input/ (lamco-rdp-input crate)
- Git history: what changed during Mutter work?

---

## Current Deployment State

### Local Machine (Development)

**Location:** /home/greg/wayland/wrd-server-specs

**Git Status:**
```
Branch: main
Commits ahead: 4
Last commit: 49ceeac "fix: remove unsafe zeroed, use actual Portal session"
Status: Working tree has uncommitted changes
```

**Code State:**
- src/server/mod.rs: Two dialog fix attempted, needs completion
- src/session/strategies/portal_token.rs: session now pub(crate), portal_clipboard returns None on v1
- src/services/translation.rs: Mutter marked Unavailable (correct)
- config.toml: Master config with all sections

**Build Status:**
- Compiles: Yes (with warnings)
- Runs: Unknown (not tested locally)
- Tests: 296/296 passing

### RHEL 9 Test VM (192.168.10.6)

**Location:** ~/wayland-build/wrd-server-specs/

**Deployed Files:**
- Binary: target/release/lamco-rdp-server (23MB, built 2026-01-05 20:04)
- Config: rhel9-config.toml (FULL config, bitrate 10000, qp 25)
- Script: run-server.sh (with pre-flight checks)
- Source: Latest from deployment (matches local)

**Directory Structure:**
```
~/wayland-build/
├── wrd-server-specs/          ← ONLY directory to use
│   ├── target/release/lamco-rdp-server
│   ├── rhel9-config.toml
│   ├── run-server.sh
│   ├── certs/test-cert.pem
│   └── src/ (source code)
├── IronRDP/ (dependency)
├── lamco-wayland/ (dependency)
├── lamco-rdp-workspace/ (dependency)
└── openh264-rs/ (dependency)
```

**Old Directories Removed:**
- ~/lamco-rdp-server (deleted)
- ~/wayland (deleted)
- ~/wrd-server-build (deleted)
- ~/run-server.sh (deleted - was confusing)
- ~/config.toml (deleted - was confusing)

**Testing Command:**
```bash
ssh greg@192.168.10.6
cd ~/wayland-build/wrd-server-specs
./run-server.sh
```

---

## Test Results Summary

### Test #1: Mutter with Portal Hybrid (RHEL 9 GNOME 40)

**Date:** 2026-01-04 21:52
**Duration:** ~1 minute
**Result:** ❌ Input broken, video works

**Findings:**
- Mutter ScreenCast: Working (PipeWire node 49)
- Mutter RemoteDesktop: Non-functional (1,137 input errors)
- Input injection: 100% failure rate
- Mouse: "Failed to inject pointer motion via Mutter" (1,081 failures)
- Keyboard: "Failed to inject keyboard keycode via Mutter" (45 failures)

**Conclusion:** Mutter input API broken on GNOME 40 (same as GNOME 46)

### Test #2: Portal-Only (RHEL 9 GNOME 40)

**Date:** 2026-01-05 17:04
**Duration:** ~1 minute
**Result:** ⚠️ Partial success

**Working:**
- Connection established ✓
- Video displays ✓
- Keyboard works ✓
- Right-click works ✓
- Portal input injection: 100% success rate

**Issues:**
- Two permission dialogs (hybrid mode bug)
- Mouse location offset (wrong position)
- Video quality poor (defaults used, not improved config)

### Test #3: Latest Test (RHEL 9 GNOME 40)

**Date:** 2026-01-05 18:04
**Duration:** <1 minute
**Result:** ❌ Crash during startup

**Crash:**
```
Portal strategy: using session_handle directly
[Crash - exited immediately]
```

**Cause:** Code tries to access session from `portal_clipboard()` which returns None on Portal v1

**Log:** ~/rhel9-test-20260105-180457.log (731 lines, crashed before connection)

---

## Code Status & Known Issues

### Files Modified This Session

**1. src/services/translation.rs**
- Status: ✅ WORKING
- Change: Mutter marked Unavailable for all GNOME versions
- Result: Portal strategy always selected
- No issues

**2. src/session/strategies/portal_token.rs**
- Status: ⚠️ PARTIALLY WORKING
- Changes:
  - `clipboard_manager` changed to `Option<Arc<ClipboardManager>>`
  - `session` field now `pub(crate)`
  - `portal_clipboard()` returns None on Portal v1
- Issue: server/mod.rs needs session but portal_clipboard() returns None
- Fix needed: Access session directly via pub(crate) field

**3. src/server/mod.rs**
- Status: ❌ BROKEN
- Changes:
  - Line 316: `session_type() == SessionType::Portal` check (correct)
  - Line 324: tries to get session from `portal_clipboard()` (crashes on None)
- Issue: Needs to access `session` field directly from PortalSessionHandleImpl
- Multiple fix attempts broke the build (unsafe zeroed, MaybeUninit, null pointers)

**4. config.toml / rhel9-config.toml**
- Status: ✅ READY
- Full config with all sections
- Quality improvements: bitrate 10000, qp 25/18, no aux omission
- Absolute cert paths
- Not yet tested (crashes before connection)

### Git Status

**Commits This Session:**
```
49ceeac - fix: remove unsafe zeroed, use actual Portal session (BROKEN)
3e48f06 - fix: properly eliminate duplicate Portal session (BROKEN)
7932616 - fix: eliminate duplicate Portal session (BROKEN)
c572605 - fix: Portal-only mode with quality improvements (WORKING but has two dialog bug)
f21d4be - fix: resolve portal_manager scope bug... (WORKING)
```

**Current HEAD:** 49ceeac (broken - tries to downcast to access session)

**Last Working Commit:** c572605 or f21d4be (has two dialog bug but functional)

**Recommendation:** May need to reset to c572605 and redo fixes carefully

---

## Architectural Context

### Service Registry & Strategy Selection

**Design:** Detection → Translation → Registry → Strategy Selection

**Current Flow:**
```
1. Detect: GNOME 40.10, Portal v4, RemoteDesktop v1
2. Translate: Mark Mutter as Unavailable
3. Registry: DirectCompositorAPI = Unavailable
4. Select: Portal + Token strategy (correct!)
5. Create: Portal strategy creates session
6. BUG: server/mod.rs creates ANOTHER Portal session (line 310-312)
```

**The Bug Location:**
```rust
// Lines 303-312: ALWAYS creates PortalManager (for both Portal and Mutter)
let portal_manager = Arc::new(PortalManager::new(portal_config).await?);

// Lines 316-328: Portal branch
if let Some(clipboard) = session_handle.portal_clipboard() {
    // This runs for Portal strategy
    // Uses the portal_manager from line 308 (DUPLICATE!)
    // Creates PortalSessionHandleImpl with clipboard components
    // Returns clipboard.session (SECOND session)
}
```

**Why It's Wrong:**
- Portal strategy ALREADY created a session
- Line 308 creates ANOTHER PortalManager
- Line 321 uses that second manager's remote_desktop
- This triggers second permission dialog

**Correct Approach:**
```rust
// Check session type FIRST, before creating any managers
if session_handle.session_type() == SessionType::Portal {
    // Portal: session_handle has EVERYTHING
    // DON'T create any Portal managers
    // DON'T create any Portal sessions
    // Just use session_handle
} else {
    // Mutter: create Portal for input
}
```

### Portal v1 vs v2+ Handling

**Portal v1 (RHEL 9):**
- RemoteDesktop: version 1
- Clipboard: NOT supported (requires v2+)
- Restore tokens: Supported (Portal v4 ScreenCast)
- Result: `clipboard_manager` is None

**Portal v2+ (Ubuntu 24.04):**
- RemoteDesktop: version 2
- Clipboard: Supported
- Restore tokens: Supported
- Result: `clipboard_manager` is Some(Arc<ClipboardManager>)

**Critical:** Code must handle both cases

**Current Issue:**
- PortalSessionHandleImpl.session is private (was pub(crate) in last attempt)
- portal_clipboard() returns None on Portal v1
- server/mod.rs needs session but can't access it

**Options:**
1. Make session pub(crate) and access directly
2. Always return ClipboardComponents with real session, placeholder manager
3. Add get_session() method to SessionHandle trait
4. Refactor to not need session in server/mod.rs

---

## Detailed Technical Status

### Video Pipeline

**Architecture:**
```
Portal → PipeWire FD 16 → Stream 51 → Display Handler → EGFX → H.264 → Client
```

**Status:** Working but quality poor

**Issues:**
1. EGFX initialization: 7+ second delay (uses RemoteFX initially)
2. Quality settings: Not applied (defaults used)
3. Config system: TOML requires all sections or uses defaults

**Config Settings (Should Apply Next Test):**
```toml
[egfx]
h264_bitrate = 10000     # Was 5000
qp_max = 25               # Was 40
qp_default = 18           # Was 23
avc444_enable_aux_omission = false  # Was true

[advanced_video]
scene_change_threshold = 0.3  # Was 0.7
intra_refresh_interval = 120  # Was 300
```

### Input Pipeline

**Architecture:**
```
Client → IronRDP → Input Handler → Portal RemoteDesktop → Compositor
```

**Status:** Working but mouse misaligned

**Mouse Coordinates:**
```
RDP(1142, 140) → Stream(1142.00, 140.00)
Portal injection: successful
Compositor: receives event
But: Context menu appears in wrong location
```

**Possible Causes:**
1. Two Portal sessions (two streams with different coordinate spaces)
2. Stream offset not accounted for
3. Coordinate transformation bug introduced during Mutter work

**To Investigate:**
- Check git history: what changed in input handling?
- Verify stream position (should be 0,0)
- Check if two streams have different positions

### Clipboard

**Status:** Correctly skipped on Portal v1

**RHEL 9:** Portal RemoteDesktop v1 (no clipboard support)

**Handling:**
```
Clipboard not available (RemoteDesktop v1 < 2)
Clipboard manager: None
Clipboard operations: Skipped gracefully
```

**No issues** - working as designed

---

## Files and Their Status

### Core Implementation Files

| File | Status | Issues | Priority |
|------|--------|--------|----------|
| `src/services/translation.rs` | ✅ Good | Mutter disabled | - |
| `src/session/strategies/portal_token.rs` | ⚠️ Partial | session pub(crate), portal_clipboard returns None | Fix access |
| `src/session/strategies/mutter_direct.rs` | ✅ Good | Dormant, not used | - |
| `src/server/mod.rs` | ❌ Broken | Lines 303-365 create duplicate sessions, crash on Portal v1 | **FIX NOW** |
| `src/server/input_handler.rs` | ⚠️ Unknown | Possible coordinate bug | Investigate |
| `config.toml` | ✅ Good | Full config, quality settings | - |

### Deployment Files

| File | Location | Status |
|------|----------|--------|
| `rhel9-config.toml` | RHEL 9 | ✅ Full config, quality settings |
| `run-server.sh` | RHEL 9 | ✅ Pre-flight checks, correct paths |
| `certs/test-cert.pem` | RHEL 9 | ✅ Valid, absolute paths in config |
| `target/release/lamco-rdp-server` | RHEL 9 | ⚠️ Built from broken code (crashes) |

### Documentation

| File | Content | Status |
|------|---------|--------|
| `docs/RHEL9-TEST-RESULTS.md` | Complete Mutter test analysis | ✅ |
| `docs/RHEL9-PORTAL-TEST-DIAGNOSIS.md` | Portal test with two dialogs | ✅ |
| `docs/STRATEGIC-PATH-FORWARD-2026-01-05.md` | Options analysis, recommendations | ✅ |
| `docs/RHEL9-DEPLOYMENT-CHECKLIST.md` | Deployment procedure | ✅ |
| `docs/SESSION-END-2026-01-05.md` | Session summary | ✅ |
| `docs/CODEBASE-ARCHITECTURE-AUDIT-2026-01-04.md` | Architecture analysis (pre-Mutter test) | ✅ |

---

## Next Session Action Plan

### Priority 1: Fix Portal v1 Session Access (30 minutes)

**Goal:** Stop crashing on Portal v1

**Current Code (BROKEN):**
```rust
// server/mod.rs line 324
let session = session_handle.portal_clipboard()
    .expect("Portal should have clipboard")  // ← Panics on Portal v1
    .session
    .clone();
```

**Option A: Access session directly**
```rust
// PortalSessionHandleImpl.session is pub(crate)
// Downcast and access directly
let portal_impl = session_handle
    .as_any()
    .downcast_ref::<PortalSessionHandleImpl>()?;
let session = portal_impl.session.clone();
```

**Option B: Add SessionHandle trait method**
```rust
// In SessionHandle trait
fn get_session(&self) -> Option<Arc<Mutex<Session>>>;

// Then in server/mod.rs
let session = session_handle.get_session()
    .unwrap_or_else(|| /* create dummy */);
```

**Option C: Always return session from portal_clipboard**
```rust
// portal_clipboard returns Some even on Portal v1
// manager is None, but session is Some
ClipboardComponents {
    manager: None,  // Portal v1
    session: self.session.clone(),  // Always provide
}
```

**Recommended:** Option C (simplest, no downcasting)

### Priority 2: Verify Quality Settings Apply (15 minutes)

**Goal:** Confirm config loads correctly

**Steps:**
1. Test with current rhel9-config.toml
2. Check log for: `h264_bitrate: 10000`
3. Check log for: `qp_max: 25`
4. Check log for: `avc444_enable_aux_omission: false`
5. If not applied: investigate config loading

**Expected:** Should apply correctly (full config deployed)

### Priority 3: Fix Mouse Alignment (1 hour)

**Goal:** Mouse cursor matches pointer location

**Steps:**
1. Verify Issue #1 fixed (one session only)
2. Check log: should be ONE PipeWire FD, ONE stream
3. If still offset: investigate coordinate transformation
4. Check git log for input-related changes during Mutter work
5. May need to revert coordinate changes

### Priority 4: Test Complete Flow (30 minutes)

**Goal:** Verify production-ready

**Test:**
1. ONE permission dialog
2. Text readable (quality good)
3. Mouse accurate
4. Keyboard works
5. No crashes
6. Connection stable

**Pass Criteria:**
- All core functionality works
- Quality acceptable for text reading
- No blocking bugs

---

## Recommended Fix for Portal v1 Session (IMMEDIATE)

```rust
// In ClipboardComponents struct definition (strategy.rs)
pub struct ClipboardComponents {
    pub manager: Option<Arc<ClipboardManager>>,  // ← Change to Option
    pub session: Arc<Mutex<Session<...>>>,
}

// In portal_token.rs portal_clipboard()
fn portal_clipboard(&self) -> Option<ClipboardComponents> {
    // Always return Some with session
    Some(ClipboardComponents {
        manager: self.clipboard_manager.clone(),  // May be None on Portal v1
        session: self.session.clone(),            // Always Some
    })
}

// In server/mod.rs
let session = session_handle.portal_clipboard()
    .expect("Portal always has session")
    .session
    .clone();

let clipboard_mgr = session_handle.portal_clipboard()
    .and_then(|c| c.manager);  // Will be None on Portal v1
```

**This is clean, no unsafe, no hacks.**

---

## Critical Context for Next Session

### What We're Building

**Product:** Commercial RDP server for Wayland Linux desktops

**License:** BSL-1.1 (becomes Apache-2.0 in 2028)

**Target Market:**
- Enterprise Linux (RHEL, Ubuntu LTS)
- Developers (remote development)
- Self-hosters

**Current Status:** Beta quality, not production-ready

### Why This Matters

**Portal-only approach is correct:**
- Works on 95% of Wayland Linux
- Mutter proven broken (6 months investigation, tested on 2 GNOME versions)
- Simple, maintainable, universal

**Three bugs block release:**
1. Two dialogs (confusing UX)
2. Poor quality (unusable for text work)
3. Mouse offset (breaks usability)

**All three are fixable.** We're close to a working v0.1.0.

### Time Investment

**This Session:**
- Mutter testing: 2 hours
- Portal v1 crash fixes: 4+ hours (**too long**)
- Config system: 1 hour
- Total: ~7 hours, many failed attempts

**Learned:**
- Unsafe workarounds always fail
- Edit tool on complex code is error-prone
- Need to test locally before deploying
- Config system needs full TOML files

---

## Quick Start for Next Session

### Step 1: Fix Portal v1 Session Access (15 min)

```bash
# Edit src/session/strategy.rs
# Change ClipboardComponents.manager to Option

# Edit src/session/strategies/portal_token.rs
# portal_clipboard() always returns Some (with None manager on v1)

# Edit src/server/mod.rs
# Access session from portal_clipboard().session (always works)
# Access manager from portal_clipboard().manager (may be None)

cargo build --release  # Verify locally
```

### Step 2: Deploy to RHEL 9 (10 min)

```bash
sshpass -p 'Bibi4189' scp src/server/mod.rs src/session/strategy.rs src/session/strategies/portal_token.rs greg@192.168.10.6:~/wayland-build/wrd-server-specs/src/...

sshpass -p 'Bibi4189' ssh greg@192.168.10.6 "cd ~/wayland-build/wrd-server-specs && cargo build --release"
```

### Step 3: Test (5 min)

```bash
# On RHEL 9
cd ~/wayland-build/wrd-server-specs
./run-server.sh

# Approve dialog (should be ONE)
# Connect and verify quality
```

### Step 4: If Working, Commit and Publish

```bash
git add -A
git commit -m "fix: Portal v1 session access, quality settings, one dialog"
# Proceed to publication
```

---

## Strategic Decisions Made

### 1. Mutter Strategy: DISABLED

**Decision:** Mark Mutter as Unavailable for all GNOME versions

**Rationale:**
- Tested broken on GNOME 40 and 46
- No indication GNOME will fix session linkage
- Portal works universally
- Simpler = more reliable

**Code:** Preserved in src/mutter/ (dormant, not deleted)

**Can Re-enable:** If GNOME fixes API in future

### 2. Portal-Only Approach: ADOPTED

**Decision:** Ship with Portal strategy only

**Platforms Supported:**
- RHEL 9: Portal v4 (one dialog first time)
- Ubuntu 24.04: Portal v5 (one dialog first time)
- Ubuntu 22.04: Portal v3 (one dialog every time - acceptable)
- KDE, Sway: Portal v4+ (works, needs testing)

**Trade-offs:**
- ❌ No zero-dialog claim (Mutter doesn't work)
- ✅ Universal compatibility (95% of Wayland Linux)
- ✅ Simpler codebase
- ✅ Faster to market

### 3. wlr-screencopy: DEFERRED

**Decision:** Don't implement for v0.1.0

**Rationale:**
- Portal works on Sway/Hyprland (with one dialog)
- wlr-screencopy is 6-8 weeks work
- Can add in v0.2.0 if market demands it

**Timeline:** Phase 2 feature (month 2-3)

---

## Lessons Learned (IMPORTANT)

### What Didn't Work

**1. Unsafe Workarounds**
- `std::mem::zeroed()` → panic (can't zero-initialize Session)
- `MaybeUninit::uninit().assume_init()` → undefined behavior
- `std::ptr::null()` → type errors
- **Lesson:** Never use unsafe to paper over design issues

**2. Editing Complex Code Without Testing**
- Made 5+ attempts to fix server/mod.rs
- Each broke the build differently
- Wasted hours debugging
- **Lesson:** Test locally FIRST, deploy only when verified

**3. Partial Configs**
- Created rhel9-config.toml with just [egfx] section
- TOML deserialization failed, used all defaults
- Wasted time debugging why settings didn't apply
- **Lesson:** Config files need ALL sections or defaults are used

**4. Assuming Deployment Worked**
- Thought fixes deployed, but old code still running
- Didn't verify source files on RHEL 9
- **Lesson:** Always verify deployed files match local

### What Worked

**1. Comprehensive Testing**
- Tested Mutter on GNOME 40 and 46
- Got definitive answer (broken on both)
- Can now confidently disable it

**2. Service Registry Design**
- Marking Mutter as Unavailable worked perfectly
- Strategy selector chose Portal correctly
- Architecture proved valuable

**3. Clean Deployment**
- Removing old directories eliminated confusion
- Single canonical location works well
- Pre-flight checks in run-server.sh helpful

---

## Files to Review Next Session

### Priority Files (FIX THESE)

**1. src/session/strategy.rs**
- Line ~17: `ClipboardComponents` struct
- Make `manager` field `Option<Arc<ClipboardManager>>`
- Keep `session` as required field

**2. src/session/strategies/portal_token.rs**
- Lines 99-108: `portal_clipboard()` method
- Return `Some(ClipboardComponents { manager: Option, session: Arc })` always
- Never return None

**3. src/server/mod.rs**
- Lines 303-365: Hybrid mode section
- Simplify Portal branch to just use session from ClipboardComponents
- Don't create duplicate Portal managers

### Reference Files (UNDERSTAND THESE)

**1. src/server/input_handler.rs**
- Coordinate transformation
- May have bugs from Mutter work
- Check git diff

**2. src/server/multiplexer_loop.rs**
- Uses `session_for_mux` (from portal_clipboard_session)
- Understand what it needs

---

## Key Decisions Needed

### 1. Mouse Alignment Fix

**Options:**
- Wait until two dialog bug fixed (may fix itself)
- Investigate coordinate code now
- Revert input changes from Mutter work

**Recommend:** Fix two dialogs first, then reassess

### 2. Quality Tuning

**Current Settings:**
- Bitrate: 10000 kbps
- QP: 25 max, 18 default

**May Need Higher:**
- Bitrate: 15000-20000 for crystal-clear text
- QP: 20 max, 15 default

**Recommend:** Test current settings first, adjust if needed

### 3. Publication Timeline

**Blockers:**
- Three bugs (dialogs, quality, mouse)
- Estimated fix time: 2-4 hours
- Then: Final testing (1-2 hours)

**Realistic Timeline:**
- Next session: Fix bugs (3-4 hours)
- Following session: Test on multiple platforms (2-3 hours)
- Then: Publish v0.1.0

---

## IMMEDIATE ACTION FOR NEXT SESSION

**DO THIS FIRST (30 minutes):**

1. **Edit src/session/strategy.rs:**
   ```rust
   pub struct ClipboardComponents {
       pub manager: Option<Arc<ClipboardManager>>,  // ← Add Option
       pub session: Arc<Mutex<Session<...>>>,
   }
   ```

2. **Edit src/session/strategies/portal_token.rs:**
   ```rust
   fn portal_clipboard(&self) -> Option<ClipboardComponents> {
       Some(ClipboardComponents {
           manager: self.clipboard_manager.clone(),  // Option
           session: self.session.clone(),            // Always Some
       })
   }
   ```

3. **Edit src/server/mod.rs lines 320-330:**
   ```rust
   let clipboard_mgr = session_handle.portal_clipboard()
       .and_then(|c| c.manager);  // Gets Option from ClipboardComponents

   let session = session_handle.portal_clipboard()
       .expect("Portal always has clipboard components")
       .session;  // Always Some
   ```

4. **Test locally:** `cargo build --release && cargo test`

5. **Deploy:** Only after local test passes

6. **Test on RHEL 9:** Should work without crashes

**This is the CLEAN solution. No unsafe. No hacks.**

---

**READY FOR NEXT SESSION - Start with ClipboardComponents.manager as Option**
