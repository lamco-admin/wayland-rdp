# Session End Status - 2026-01-05

**Focus:** RHEL 9 testing, Mutter investigation complete, Portal-only transition
**Status:** Partial success - connection works but quality and mouse issues
**Next Session Priority:** Fix config system and mouse coordinates

---

## What We Accomplished

### 1. Mutter Investigation Complete ✅

**Tested:** GNOME 40 (RHEL 9) and GNOME 46 (previous)

**Results:**
- GNOME 40: Video works, input 100% broken (1,137 errors)
- GNOME 46: Everything broken

**Decision:** Mutter is non-viable, disabled in Service Registry

**Code Status:** Mutter code preserved in src/mutter/ (dormant), marked Unavailable

### 2. Portal-Only Strategy Working ✅

**RHEL 9 Results:**
- Connection: ✅ Works
- Video: ✅ Displays
- Input: ✅ Keyboard works, mouse works but MISALIGNED
- Quality: ❌ Poor (config not applied)

### 3. Build System Working ✅

- Binary builds on RHEL 9 (glibc 2.34)
- Deployment to RHEL 9 VM functional
- Cargo.toml path fixes working

---

## Critical Issues Found

### Issue #1: Config System Broken 🔴

**Problem:** Partial configs don't work, fall back to defaults

**Evidence:**
```
rhel9-config.toml had:
  h264_bitrate = 8000

But log shows:
  h264_bitrate: 5000 (default)
```

**Cause:** Config requires ALL sections or falls back to defaults

**Fix Required:**
1. Use full config.toml (not partial)
2. Modify only needed values
3. Test config actually loads

### Issue #2: Two Dialog Bug Still Present 🔴

**Problem:** Hybrid mode creates duplicate Portal session

**Evidence:**
```
PipeWire FD: 16 (first Portal session)
PipeWire FD: 15 (second Portal session)
```

**Cause:** My fixes to server/mod.rs didn't actually deploy or were broken

**Fix Required:**
1. Carefully edit server/mod.rs line 316
2. Change: `if let Some(clipboard) = session_handle.portal_clipboard()`
3. To: `if session_handle.session_type() == SessionType::Portal`
4. Test builds before deploying

### Issue #3: Mouse Misalignment 🔴

**Problem:** Mouse coordinates wrong (context menu appears in wrong location)

**Evidence:** Screenshot shows menu offset from cursor

**Possible causes:**
1. Two Portal sessions (two different streams/coordinates)
2. Wrong stream ID for input (using 51 but video on different stream?)
3. Coordinate transformation issue
4. Multi-monitor offset bug

**Fix Required:**
1. Fix two dialog bug first (eliminate duplicate session)
2. Verify single stream coordinates
3. Check coordinate transformation code
4. May need to revert mouse changes from Mutter work

### Issue #4: Poor Video Quality 🟡

**Problem:** Text blurry, Red Hat logo unclear

**Evidence:** Screenshot shows poor quality compared to background console

**Causes:**
1. Config settings not applied (using defaults: qp 40, bitrate 5000, aux omission ON)
2. RemoteFX used initially (7s delay before EGFX)
3. Default encoder settings inadequate for text

**Fix Required:**
1. Fix config system (Issue #1)
2. Investigate EGFX initialization delay
3. May need higher bitrate (10000+) for clear text

---

## What's on RHEL 9

**Location:** ~/wayland-build/wrd-server-specs/

**Binary:**
- target/release/lamco-rdp-server (23MB, glibc 2.34)
- Built: 2026-01-05 18:47
- Contains: OLD code (two dialog bug present)

**Config:**
- rhel9-config.toml (PARTIAL, defaults used)
- Certs: Absolute paths (work)

**Source:**
- src/server/mod.rs (OLD version, hybrid bug present)
- src/session/strategies/portal_token.rs (Fixed: Optional clipboard)
- src/services/translation.rs (Fixed: Mutter disabled)

---

## What Works

1. ✅ Portal strategy selected on RHEL 9
2. ✅ Connection established
3. ✅ Video displays
4. ✅ Keyboard input works
5. ✅ Right click works (but wrong location)
6. ✅ No crashes
7. ✅ Certs load (after using full config with absolute paths)
8. ✅ H.264/AVC444 encoding initializes
9. ✅ Mutter disabled (Portal-only working)

---

## What Doesn't Work

1. ❌ Mouse location wrong (offset/misaligned)
2. ❌ Video quality poor (config settings ignored)
3. ❌ Two permission dialogs (hybrid mode bug)
4. ❌ Config system (partial configs don't work)

---

## Lessons Learned

### Config System

**Problem:** TOML deserialization requires complete structs

**Solution:** Must use complete config.toml with all sections, then override specific values

**Don't:** Create minimal configs with just changed values

**Do:** Copy full config.toml, edit specific lines

### Deployment Process

**Problem:** Multiple failed attempts to fix server/mod.rs

**Why:** Using Edit tool without verifying file content, sed commands, patches all failed

**Solution:**
1. Read file first
2. Make ONE change at a time
3. Build and test locally
4. Only then deploy to RHEL 9
5. Verify deployed version matches local

### Testing Workflow

**Problem:** Old files in wrong directories caused confusion

**Solution:** Clean RHEL 9 completely before each deployment, use ONE directory only

---

## Next Session Priorities

### Priority 1: Fix Config System (CRITICAL)

**Goal:** Quality settings actually apply

**Steps:**
1. Verify config.toml has all sections
2. Edit only EGFX values (bitrate, qp, aux_omission)
3. Test locally that config loads
4. Deploy to RHEL 9
5. Verify settings in log
6. Test quality improvement

### Priority 2: Fix Two Dialog Bug (HIGH)

**Goal:** ONE permission dialog only

**Steps:**
1. Create clean server/mod.rs fix
2. Test builds locally
3. Deploy ONLY when verified working
4. Test shows one dialog

### Priority 3: Fix Mouse Alignment (HIGH)

**Goal:** Mouse cursor matches pointer location

**Investigate:**
1. Is duplicate Portal session causing this? (two streams?)
2. Did coordinate code change during Mutter work?
3. What's the stream offset/position?
4. Is this related to FD 16 vs node 51 mismatch?

**Likely fix:** Fixing two dialog bug may fix mouse (eliminate duplicate stream)

---

## Code Status

**Git Commits:**
- c572605: Portal-only mode with quality improvements (broken)
- 7932616: eliminate duplicate Portal session (broken - malformed file)
- 3e48f06: same (attempted fix, also broken)

**Working Code:**
- Local: Needs fixing (last commit broken)
- RHEL 9: OLD version from before fixes (two dialog bug present)

**Action Required:**
- Git reset to working commit
- Fix config system properly
- Fix two dialog bug properly
- Test both locally before deploying

---

## Session Notes

**Time Spent:** ~6 hours on deployment issues

**Main Obstacles:**
1. Config system not well understood (partial configs don't work)
2. Two dialog bug fix attempted 5+ times, all broke the build
3. Deployment verification lacking (assumed fixes deployed when they didn't)

**For Next Session:**
1. Start with working code (verify git status clean)
2. Make ONE change at a time
3. Test locally until it works
4. Only then deploy
5. Verify deployed code matches local

---

## Technical Findings

### Mouse Coordinates

**Log shows:**
```
RDP(1142, 140) -> Stream(1142.00, 140.00)
```

**This is 1:1 mapping, should be correct** unless:
- Multiple streams with offsets
- Wrong stream being targeted
- Portal using different coordinate space

**Investigate:** Are there two streams from two Portal sessions?

### Video Quality

**Actual settings used (from log):**
```
Bitrate: 5000 kbps (not 8000)
QP max: 40 (not 28)
Aux omission: true (not false)
Scene change: 0.7 (not 0.4)
```

**Config system failed to apply settings**

**Also:** RemoteFX used initially (poor bitmap compression)

### Two Portal Sessions

**Clear evidence:**
```
Portal Manager initialized (first)
Creating portal session
Portal Manager initialized (second - DUPLICATE!)
Creating portal session (DUPLICATE!)
PipeWire FD: 16 (first)
PipeWire FD: 15 (second)
```

**My server/mod.rs fix never deployed to binary**

---

## Deployment Checklist for Next Session

**Before starting:**
- [ ] Git status clean
- [ ] Code compiles locally
- [ ] Tests pass

**For each fix:**
- [ ] Make change locally
- [ ] Compile and verify
- [ ] Commit
- [ ] Deploy to RHEL 9
- [ ] Rebuild on RHEL 9
- [ ] Verify fix in deployed binary (check source file)
- [ ] Test

**Don't:**
- Make multiple changes at once
- Deploy without local verification
- Assume deployment worked
- Use sed/patches (too error-prone)

---

**END OF SESSION - Resume with config fix and mouse investigation**
