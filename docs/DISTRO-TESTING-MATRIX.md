# Distribution Testing Matrix

**Date:** 2026-01-15
**Purpose:** Track testing and build status across Linux distributions
**Goal:** Verify session persistence and full functionality on all target platforms

---

## OBS Build Status

*Project: lamco | OBS: https://192.168.10.8*

### Native Package Builds

| Distribution | Version | Rust | Build Status | Package | RHEL Compat |
|--------------|---------|------|--------------|---------|-------------|
| Fedora | 42 | 1.88+ | 🔨 Building | RPM | - |
| Fedora | 41 | 1.87 | 🔨 Building | RPM | - |
| Fedora | 40 | 1.79 | 🔨 Building | RPM | - |
| openSUSE | Tumbleweed | 1.82+ | 🔨 Building | RPM | - |
| openSUSE | Leap 15.6 | 1.78+ | 🔨 Building | RPM | - |
| Debian | 13 (Trixie) | 1.79 | 🔨 Building | DEB | - |
| AlmaLinux | 9 | 1.84 | 🔨 Building | RPM | ✅ RHEL 9/Rocky 9 |
| Ubuntu | 24.04 | 1.75 | ❌ Unresolvable | - | - |
| Debian | 12 | 1.63 | ❌ Unresolvable | - | - |

### Universal Packages

| Format | Status | Notes |
|--------|--------|-------|
| Flatpak | ✅ Built & Tested | `packaging/io.lamco.rdp-server.yml` |

**Flatpak Bundle:** `packaging/io.lamco.rdp-server.flatpak` (6.4 MB) - portable bundle for VM testing

### Unresolvable Distributions

These require Flatpak due to old Rust versions in distro repos:

- **Ubuntu 24.04**: Rust 1.75 (need >= 1.77)
- **Debian 12**: Rust 1.63 (need >= 1.77)

---

## Testing Priority

### 🔴 CRITICAL - Must Test Before Enterprise Launch

| Distribution | Version | GNOME | Portal | Expected Strategy | Test Status | VM Status |
|--------------|---------|-------|--------|-------------------|-------------|-----------|
| **RHEL 9** | 9.7 | 40.10 | v4 (rejects persist) | Portal (no persist) | ✅ **RDP WORKING** | ✅ 192.168.10.6 |
| **Ubuntu 22.04 LTS** | 22.04.3 | 42.x | v3 (no tokens) | Mutter (if works) | ⏳ **UNTESTED** | Need VM |

**RHEL 9 Update (2026-01-15):** Full RDP session tested. GNOME Portal backend **rejects persistence** for RemoteDesktop sessions with error "Remote desktop sessions cannot persist". This is deliberate policy, not a missing feature. RDP functionality works fully (video, input), but each server restart requires user permission dialog.

**RHEL 9 Capabilities (Tested):**
```
Portal version: 4
ScreenCast: ✅  RemoteDesktop: ✅  Clipboard: ❌ (v1)
Session Persistence: ❌ REJECTED by GNOME portal
Credential Storage: Encrypted File (Flatpak sandbox)
RDP Functionality: Video ✅, Keyboard ✅, Mouse ✅
```

**Ubuntu 22.04 Still Critical:**
- Portal v3 doesn't support restore tokens
- Without Mutter: Dialog appears EVERY restart (unacceptable for servers)
- With Mutter: Zero dialogs (if API works on GNOME 42)
- **This is the system Mutter was designed for**

**Test Plan:**
1. Deploy lamco-rdp-server ✅ (RHEL 9 done)
2. Check capabilities output ✅ (RHEL 9 done)
3. Test video (does PipeWire stream receive frames?)
4. Test input (does mouse/keyboard work?)
5. Test clipboard
6. Restart server (verify zero dialogs on second run)

---

### 🟡 HIGH - Should Test Before Launch

These confirm Portal strategy works correctly:

| Distribution | Version | GNOME | Portal | Expected Strategy | Test Status | VM Status |
|--------------|---------|-------|--------|-------------------|-------------|-----------|
| **Ubuntu 24.04 LTS** | 24.04.3 | 46.0 | v5 (rejects persist) | Portal (no persist) | ✅ **RDP WORKING** | ✅ 192.168.10.205 |
| **Fedora 40** | 40 | 46.0 | v5 (tokens) | Portal | ⏳ Need test | Need VM |
| **SUSE Enterprise** | 15 SP6 | 45.x | v5 (tokens) | Mutter or Portal | ⏳ Need test | Need VM |
| **Debian 12** | Bookworm | 43.x | v4 (tokens) | Portal | ⏳ Need test | Need VM |

**Why Important:**
- Verifies Portal works across different versions
- Tests token persistence
- Different portal backends (gnome, kde, wlr)

---

### 🟢 MEDIUM - Good Coverage

Broader ecosystem validation:

| Distribution | Version | GNOME | Portal | Expected Strategy | Test Status | VM Status |
|--------------|---------|-------|--------|-------------------|-------------|-----------|
| **Fedora 39** | 39 | 45.x | v5 | Mutter or Portal | ⏳ Need test | Need VM |
| **Pop!_OS 22.04** | 22.04 | 42.x | v3 | Mutter (if works) | ⏳ Need test | Need VM |
| **RHEL 8** | 8.9 | 3.38 | v3 | Portal | ⏳ Need test | Need VM |
| **Arch Linux** | Rolling | 47.x | v5 | Portal | ⏳ Need test | Need VM |

---

### 🔵 LOW - Nice To Have

Edge cases and less common systems:

| Distribution | Version | GNOME | Portal | Expected Strategy | Test Status | VM Status |
|--------------|---------|-------|--------|-------------------|-------------|-----------|
| **Fedora 41** | 41 | 47.x | v5 | Portal | ⏳ Need test | Need VM |
| **Manjaro** | Rolling | 46.x | v5 | Portal | ⏳ Need test | Need VM |
| **Ubuntu 23.10** | 23.10 | 45.x | v5 | Mutter or Portal | ⏳ Need test | Need VM |
| **openSUSE Tumbleweed** | Rolling | 47.x | v5 | Portal | ⏳ Need test | Need VM |

---

## Non-GNOME Platforms (Portal Only)

### KDE Plasma Testing

| Distribution | Version | KDE | Portal | Test Status | VM Status |
|--------------|---------|-----|--------|-------------|-----------|
| **Kubuntu 24.04** | 24.04 | 6.x | portal-kde | ⏳ Need test | Need VM |
| **KDE neon** | Latest | 6.x | portal-kde | ⏳ Need test | Need VM |
| **Fedora KDE** | 40 | 6.x | portal-kde | ⏳ Need test | Need VM |

**Test Focus:**
- Portal token persistence (should work)
- SelectionOwnerChanged signals (should work, unlike GNOME)
- KWallet credential storage

---

### wlroots Compositors

| Compositor | Distribution | Portal | Test Status | Notes |
|------------|--------------|--------|-------------|-------|
| **Sway** | Arch/Fedora | portal-wlr | ⏳ Need test | Should work perfectly |
| **Hyprland** | Arch | portal-hyprland | ⏳ Known bugs | Token bugs documented |
| **Wayfire** | Raspberry Pi OS | portal-wlr | ⏳ Optional | Interesting market |

**Test Focus:**
- Portal token persistence
- SelectionOwnerChanged signals
- Encrypted file credential storage

---

## Test Results Template

### Per-Distribution Test Report

```markdown
## [Distribution Name] [Version]

**Date Tested:** YYYY-MM-DD
**GNOME Version:** X.Y
**Portal Version:** vX
**Kernel:** X.Y.Z

### Service Registry Detection
- Compositor: [detected]
- DirectCompositorAPI: [Guaranteed/BestEffort/Unavailable]
- Session Persistence: [level]
- Portal Version: [detected]

### Strategy Selected
- [X] Mutter Direct
- [ ] Portal + Token

### Video Test
- [ ] Video displays correctly
- [ ] Frame rate smooth (30fps)
- [ ] No artifacts

### Input Test
- [ ] Mouse moves correctly
- [ ] Mouse alignment perfect
- [ ] Keyboard works
- [ ] Right-click works

### Clipboard Test
- [ ] Linux → Windows (copy file)
- [ ] Windows → Linux (paste text)
- [ ] Both directions work

### Session Persistence
- [ ] First run: Dialogs shown [count]
- [ ] Second run: Dialogs shown [count]
- [ ] Token saved correctly
- [ ] Token restores session

### Issues Found
- [List any problems]

### Verdict
- [ ] Production Ready
- [ ] Needs Fixes
- [ ] Not Supported
```

---

## VM Requirements

### Minimum Specs (Per VM)
- **CPU:** 2 cores
- **RAM:** 4GB
- **Disk:** 20GB
- **Network:** Bridge or NAT with port forwarding
- **Display:** Wayland session required

### Setup Requirements
- xdg-desktop-portal installed
- Appropriate portal backend (gnome/kde/wlr)
- PipeWire 0.3.77+
- systemd (for credential storage testing)
- SSH access for deployment

### Test Server Configuration
- Hostname: Descriptive (e.g., rhel9-test, ubuntu2204-test)
- User: greg (or specify)
- IP: Static or DHCP with reservation
- SSH keys: Configured
- Certificates: Generated (certs/ directory)

---

## Testing Workflow

### 1. Deploy to Test VM

```bash
# Build
cargo build --release

# Deploy (adjust IP for each VM)
ssh user@VM_IP "rm -f ~/lamco-rdp-server"
scp target/release/lamco-rdp-server user@VM_IP:~/lamco-rdp-server
scp config.toml certs/ user@VM_IP:~/
ssh user@VM_IP "chmod +x ~/lamco-rdp-server"
```

### 2. Run and Capture Logs

```bash
ssh user@VM_IP
./lamco-rdp-server -c config.toml -vvv --log-file test-$(date +%Y%m%d).log

# Copy logs back
scp user@VM_IP:~/test-*.log ./logs/[distro-name]/
```

### 3. Analyze Locally

```bash
cd logs/[distro-name]/
rg "Service Registry|Strategy.*selected|ERROR|DirectCompositorAPI" test-*.log
rg "Session created|Dialog|Token" test-*.log
```

### 4. Document Results

Add to this matrix and create individual test reports.

---

## Expected Behavior Per Category

### Portal v5 + GNOME 46 (Ubuntu 24.04) - TESTED
```
✅ Portal version: 5
✅ Strategy: Portal (persistence REJECTED by backend)
✅ ScreenCast: Yes, RemoteDesktop v2: Yes
⚠️  Clipboard: Text works, but CRASH BUG exists (see below)
✅ Credential storage: Encrypted file (AES-256-GCM)
⚠️  First run: 1 dialog (screen sharing)
⚠️  Subsequent runs: 1 dialog (persistence rejected)
✅ RDP Functionality: Video (H.264/AVC444v2), keyboard, mouse all working
✅ Latest test (2026-01-15): 593 frames encoded, ~10ms latency
✅ Encoding: AVC420 + AVC444v2 with aux omission (bandwidth saving)
⚠️  FUSE: Failed to mount (libfuse3 not available in Flatpak sandbox)
❌  PORTAL CRASH: xdg-desktop-portal-gnome crashes during Excel→Calc paste
⚠️  Verdict: Functional with known clipboard crash bug
```

**🔴 CRITICAL BUG: Portal Crash During Clipboard Paste**

Reproducible crash when pasting Excel cells into LibreOffice Calc:
1. Copy cells in Excel (Windows RDP client)
2. Right-click → Paste in LibreOffice Calc (Linux)
3. xdg-desktop-portal-gnome crashes after ~2 second hang
4. All input injection fails after crash

Technical details:
- `selection_write()` hangs for ~2 seconds, then fails
- Error: "Message recipient disconnected from message bus"
- Excel sends 15 clipboard formats (Biff12, Biff8, HTML, RTF, etc.)
- Crash occurs during Portal's processing of the write

**Root Cause:** Two issues:
1. **xdg-desktop-portal-gnome bug**: Crashes when processing complex Excel data
2. **lamco-rdp-server design flaw**: Clipboard and input share same session lock

When clipboard `selection_write()` blocks waiting for Portal response:
- Input injection is blocked waiting for session lock
- Mouse queue fills up → "no available capacity" errors
- After 2 seconds, Portal crashes and all queued input fails

**Fix Required:** Separate session locks for clipboard vs input injection.

**Same as RHEL 9:** GNOME's portal backend also rejects persistence for
RemoteDesktop sessions on Ubuntu 24.04 with Portal v5.

### Portal v4 + GNOME 40 (RHEL 9) - TESTED
```
✅ Portal version: 4
✅ Strategy: Portal (persistence REJECTED by backend)
✅ ScreenCast: Yes, RemoteDesktop: Yes
❌ Clipboard: No (Portal RemoteDesktop v1)
✅ Credential storage: Encrypted file
⚠️  First run: 1 dialog (screen sharing)
⚠️  Subsequent runs: 1 dialog (persistence rejected by GNOME portal)
✅ RDP Functionality: Video, keyboard, mouse all working
⚠️  Verdict: Functional (dialog on each restart)
```

**Root Cause:** GNOME's xdg-desktop-portal-gnome backend rejects persistence
for RemoteDesktop sessions with error: "Remote desktop sessions cannot persist"
This is a deliberate portal backend policy, not a missing feature.

### Portal v3 + GNOME 42+ (Ubuntu 22.04) - UNTESTED
```
❓ DirectCompositorAPI: BestEffort (unknown if works)
❓ Strategy: Mutter (preferred) OR Portal (fallback)

If Mutter works:
  ✅ First run: 0-1 dialogs
  ✅ Subsequent runs: 0 dialogs
  ✅ Verdict: Production Ready (zero-dialog achieved)

If Mutter broken:
  ⚠️ First run: 1 dialog
  ⚠️ Subsequent runs: 1 dialog (no tokens on Portal v3)
  ⚠️ Verdict: Functional but not ideal for servers
```

### Portal v5 + KDE/Sway (Non-GNOME)
```
✅ DirectCompositorAPI: Unavailable (not GNOME)
✅ Strategy: Portal + Token
✅ First run: 1 dialog
✅ Subsequent runs: 0 dialogs (token restores)
✅ Clipboard: SelectionOwnerChanged works (unlike GNOME)
✅ Verdict: Should be Production Ready
```

---

## Test Data Collection

### For Each Test, Record:

**System Info:**
```bash
lsb_release -a
uname -a
gnome-shell --version
pipewire --version
gdbus introspect --session --dest org.freedesktop.portal.Desktop --object-path /org/freedesktop/portal/desktop | grep version
```

**Service Registry Output:**
```bash
./lamco-rdp-server --show-capabilities > capabilities-[distro].txt
```

**Session Test:**
```bash
# First run
./lamco-rdp-server > first-run.log 2>&1
# Count dialogs, verify functionality

# Second run
./lamco-rdp-server > second-run.log 2>&1
# Verify no dialogs (or expected dialogs)
```

**Logs to Collect:**
- capabilities-[distro].txt
- first-run.log
- second-run.log
- console-output.log
- Any crash logs

---

## Current Status Summary

**Last Updated:** 2026-01-15

**Tested:** 2 platforms - Both fully tested with RDP sessions
- Ubuntu 24.04 / GNOME 46 (Portal v5) - Full RDP tested ✅
- RHEL 9.7 / GNOME 40 (Portal v4) - Full RDP tested ✅

**Key Findings:**

1. **GNOME rejects persistence for RemoteDesktop sessions** (BOTH platforms)
   - Error: "Remote desktop sessions cannot persist"
   - Affects: RHEL 9 (Portal v4) AND Ubuntu 24.04 (Portal v5)
   - This is GNOME portal backend policy, not a bug
   - RDP works fully, but requires permission dialog on each server restart

2. **Clipboard varies by Portal version**
   - RHEL 9 (Portal RemoteDesktop v1): No clipboard support
   - Ubuntu 24.04 (Portal RemoteDesktop v2): Clipboard working (text + files via staging)
     - 35 initial errors (normal - client clipboard empty at connection)
     - 4 successful format announcements
     - 31 file transfers via staging (FUSE not available in Flatpak)

**Working:**
- Portal screen capture and input injection
- Video encoding (EGFX/H.264 AVC444v2 with aux omission)
- Keyboard and mouse input
- Encrypted credential storage (Flatpak sandbox, AES-256-GCM)
- Text clipboard (Ubuntu 24.04 via D-Bus GNOME extension)

**Not Working / Known Issues:**
- Session persistence on GNOME (both platforms) - GNOME policy rejects persistence
- Clipboard sync on RHEL 9 (Portal RemoteDesktop v1 limitation)
- FUSE file clipboard in Flatpak (libfuse3 mount fails - using staging fallback)

**Next Steps:**
1. ~~Run full RDP session test on RHEL 9~~ ✅ Complete
2. ~~Run full RDP session test on Ubuntu 24.04~~ ✅ Complete (2026-01-15)
3. Investigate FUSE mounting in Flatpak sandbox
4. Test Ubuntu 22.04 (Portal v3) when VM available
5. Test non-GNOME platforms (KDE/Sway) for persistence verification

---

## Known Issues for Commercial Release

### 🔴 CRITICAL - Must Fix

| Issue | Impact | Root Cause | Fix |
|-------|--------|------------|-----|
| Portal crash on Excel paste | Session dies | xdg-portal-gnome bug + our lock contention | Separate session locks |
| Clipboard blocks input | Mouse queue overflow, lag | Shared session mutex | Use separate locks for clipboard vs input |
| File paste fails in Flatpak | Can't paste files to ~/Downloads | Sandbox read-only | Use XDG portal for file access |

### 🟡 MEDIUM - Should Fix

| Issue | Impact | Root Cause | Fix |
|-------|--------|------------|-----|
| MemFd size=0 warnings | Log spam | PipeWire sends empty buffers normally | Downgrade WARN→DEBUG |
| Format parameter building | Using fallback negotiation | PipeWire format negotiation | Investigate proper format building |
| Clipboard format errors at start | 35 errors on connect | Client clipboard empty | Expected behavior, improve logging |

### 🟢 LOW - Nice to Have

| Issue | Impact | Root Cause | Fix |
|-------|--------|------------|-----|
| FUSE unavailable in Flatpak | File clipboard uses staging | libfuse3 not in sandbox | Add FUSE to Flatpak manifest |
| Session persistence rejected | Dialog on every restart | GNOME policy | Cannot fix (GNOME decision) |

---

*End of Distribution Testing Matrix*
