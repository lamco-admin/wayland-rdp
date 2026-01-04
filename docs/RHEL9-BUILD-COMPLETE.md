# RHEL 9 Build Complete - Ready for Testing

**Date:** 2026-01-04
**Status:** ✅ Build successful, binary ready for testing
**Platform:** RHEL 9.7, GNOME 40.10, glibc 2.34

---

## Build Summary

### Binary Information
```
Location: ~/wayland-build/wrd-server-specs/target/release/lamco-rdp-server
Size: 23MB (stripped)
Linked to: glibc 2.34 (RHEL 9 compatible)
Built with: Rust 1.92.0
Build time: 2 minutes 19 seconds
```

### Bug Fixes Applied
1. ✅ **portal_manager scope bug** - Fixed in portal_token.rs
   - Now tracks active_manager through match branches
   - Correct manager used for remote_desktop

2. ✅ **Duplicate Portal session** - Fixed in server/mod.rs
   - PortalManager only created where needed
   - No wasteful duplicate sessions

3. ✅ **All tests passing** - 296/296 tests pass

---

## Detected Capabilities on RHEL 9

**Environment:**
- Compositor: GNOME 40.10
- Portal version: 4 (restore tokens ✅ supported)
- Deployment: Native Package
- Credential Storage: GNOME Keyring (unlocked)

**Mutter D-Bus Services:**
- ✅ org.gnome.Mutter.ScreenCast (available)
- ✅ org.gnome.Mutter.RemoteDesktop (available)

**Portal Features:**
- ScreenCast: true
- RemoteDesktop: true
- Clipboard: false (interesting - may need GNOME Shell extension)

**Cursor Modes Available:**
- Hidden, Embedded, Metadata

**Recommended:**
- Capture: Portal
- Buffer: MemFd

---

## Critical Question to Answer

**Will Mutter Direct API work on GNOME 40?**

**Service Registry will decide:**
- GNOME 40.10 → version check: `v >= 40.0 && v < 46.0`
- Should mark DirectCompositorAPI as: **BestEffort**
- Strategy selector should TRY Mutter first
- If Mutter fails: Falls back to Portal

**Expected outcomes:**

**Scenario A: Mutter Works**
- Logs show: "✅ Selected: Mutter Direct D-Bus API strategy"
- Video works (screen visible)
- Zero permission dialogs
- Input works
- **Result:** Keep Mutter for GNOME 40-45

**Scenario B: Mutter Fails to Connect**
- Logs show: "Service Registry reports available but connection failed"
- Falls back to: "✅ Selected: Portal + Token strategy"
- One permission dialog
- Everything works via Portal

**Scenario C: Mutter Connects but Broken**
- Logs show: "✅ Selected: Mutter Direct API"
- Then: Black screen or input failures
- Same issues as GNOME 46
- **Result:** Disable Mutter entirely

---

## How to Test

### SSH to RHEL 9 VM

```bash
ssh greg@192.168.10.6
# Password: Bibi4189
```

### Run Test Script

```bash
cd ~/wayland-build/wrd-server-specs
./run-server.sh
```

**What it does:**
- Shows binary info and glibc version
- Starts lamco-rdp-server on port 3389
- Logs everything to ~/rhel9-test-TIMESTAMP.log
- Watch console output for strategy selection

### Connect from RDP Client

**From another machine (Windows, macOS, or Linux):**
```bash
# FreeRDP (Linux):
xfreerdp /v:192.168.10.6:3389 /u:test /size:1280x1024

# Microsoft RDP (Windows):
mstsc /v:192.168.10.6

# Remmina (Linux GUI):
# Add connection: 192.168.10.6:3389
```

### What to Test

**Video:**
- Does screen appear?
- Is it black or showing RHEL 9 desktop?
- Is quality good?

**Mouse:**
- Does cursor move correctly?
- Is alignment accurate?
- Any lag?

**Keyboard:**
- Does typing work?
- Are keys correct?

**Permission Dialog:**
- How many dialogs appeared? (0 or 1)
- If Mutter: Should be 0 for video, maybe 1 for clipboard
- If Portal: Should be 1

### After Test - Check Logs

```bash
# On RHEL 9:
cd ~/wayland-build/wrd-server-specs
ls -lh ~/rhel9-test-*.log

# Look for these lines:
grep "Selected.*strategy" ~/rhel9-test-*.log
grep "Session created successfully" ~/rhel9-test-*.log
grep "HYBRID MODE\|Mutter Direct" ~/rhel9-test-*.log
```

---

## Test Results Template

**Fill this out after testing:**

```
Date: 2026-01-04
Platform: RHEL 9.7, GNOME 40.10
Binary: lamco-rdp-server (glibc 2.34)

Strategy Selected: [ ] Mutter Direct API  [ ] Portal + Token

Mutter Status:
  [ ] Mutter selected and worked
  [ ] Mutter selected but failed (specify error)
  [ ] Mutter not selected (Portal used instead)

Functionality:
  Video: [ ] Works [ ] Black screen [ ] Error
  Mouse: [ ] Works [ ] Misaligned [ ] Doesn't work
  Keyboard: [ ] Works [ ] Doesn't work
  Clipboard: [ ] Works [ ] Doesn't work [ ] Not tested

Dialog Count: [ ] 0 [ ] 1 [ ] 2+ [ ] Error before dialog

Overall: [ ] Production Ready [ ] Has Issues [ ] Completely Broken

Notes:
<your observations>
```

---

## Next Steps Based on Results

### If Mutter Works ✅
1. Document GNOME 40-45 support
2. Test on Ubuntu 22.04 LTS (GNOME 42) for confirmation
3. Keep Mutter strategy in codebase
4. Document version-specific behavior
5. Freeze code for publication

### If Mutter Broken ❌
1. Update Service Registry: Mark ALL Mutter as Unavailable
2. Remove Mutter code (~1,100 lines)
3. Simplify to Portal-only
4. Document Portal v4 requirement
5. Freeze code for publication

### If Build/Test Issues 🔧
1. Debug and fix
2. Re-test
3. Document any RHEL 9-specific quirks

---

## Files Created

**On RHEL 9:**
- `~/wayland-build/` - All source code
- `~/wayland-build/wrd-server-specs/target/release/lamco-rdp-server` - Binary (23MB)
- `~/wayland-build/wrd-server-specs/run-server.sh` - Test script

**Locally:**
- `scripts/deploy-to-rhel9.sh` - Deployment automation
- `scripts/RHEL9-BUILD-PLAN.md` - Build documentation
- `docs/RHEL9-BUILD-COMPLETE.md` - This file

**Logs:**
- `/tmp/rhel9-deploy.log` - Deployment log
- `/tmp/rhel9-build-final.log` - Build log
- `~/rhel9-test-TIMESTAMP.log` - Test run logs (on RHEL 9)

---

**Ready for testing. Run `./run-server.sh` on RHEL 9 to start.**
