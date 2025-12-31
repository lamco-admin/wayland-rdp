# Distribution Testing Matrix

**Date:** 2025-12-31
**Purpose:** Track testing status across Linux distributions
**Goal:** Verify session persistence works on all target platforms

---

## Testing Priority

### 🔴 CRITICAL - Must Test Before Enterprise Launch

These are Portal v3 systems where Mutter is the only way to avoid dialog-every-restart:

| Distribution | Version | GNOME | Portal | Expected Strategy | Test Status | VM Status |
|--------------|---------|-------|--------|-------------------|-------------|-----------|
| **RHEL 9** | 9.3 | 40.x | v3 (no tokens) | Mutter (if works) | ⏳ **UNTESTED** | Need VM |
| **Ubuntu 22.04 LTS** | 22.04.3 | 42.x | v3 (no tokens) | Mutter (if works) | ⏳ **UNTESTED** | Need VM |

**Why Critical:**
- Portal v3 doesn't support restore tokens
- Without Mutter: Dialog appears EVERY restart (unacceptable for servers)
- With Mutter: Zero dialogs (if API works on these versions)
- **These are the systems Mutter was designed for**

**Test Plan:**
1. Deploy lamco-rdp-server
2. Check Service Registry output (should show Mutter BestEffort)
3. Verify strategy selected (should be Mutter Direct)
4. Test video (does PipeWire stream receive frames?)
5. Test input (does mouse/keyboard work?)
6. Test clipboard
7. Restart server (does it start without dialog?)

**Expected Results if Mutter Works:**
- First run: 0 dialogs for video/input, 1 for clipboard
- Second run: 0 dialogs (Mutter doesn't use tokens)
- **Zero-dialog operation achieved** ✅

**Expected Results if Mutter Broken:**
- Falls back to Portal
- First run: 1 dialog
- Second run: 1 dialog (no token support on Portal v3)
- **Need alternative solution** ❌

---

### 🟡 HIGH - Should Test Before Launch

These confirm Portal strategy works correctly:

| Distribution | Version | GNOME | Portal | Expected Strategy | Test Status | VM Status |
|--------------|---------|-------|--------|-------------------|-------------|-----------|
| **Ubuntu 24.04 LTS** | 24.04.1 | 46.0 | v5 (tokens) | Portal | ✅ **TESTED** | ✅ Ready (192.168.10.205) |
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

### Portal v5 + GNOME 46+ (Ubuntu 24.04, Fedora 40+)
```
✅ DirectCompositorAPI: Unavailable (GNOME 46+ known broken)
✅ Strategy: Portal + Token
✅ First run: 1 dialog
✅ Subsequent runs: 0 dialogs (token restores)
✅ Credential storage: GNOME Keyring
✅ Verdict: Production Ready
```

### Portal v3 + GNOME 40-45 (RHEL 9, Ubuntu 22.04)
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

**Tested:** 1 platform (Ubuntu 24.04 / GNOME 46)
**Working:** 1 platform (Portal strategy)
**Broken:** Mutter on GNOME 46 (documented, not fixable)
**Unknown:** Everything else (especially RHEL 9, Ubuntu 22.04)

**Critical Path:** Get RHEL 9 / Ubuntu 22.04 VMs, test Mutter on GNOME 40/42.

---

*End of Distribution Testing Matrix*
