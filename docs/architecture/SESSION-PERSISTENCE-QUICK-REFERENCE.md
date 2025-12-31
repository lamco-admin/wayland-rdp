# Session Persistence Quick Reference

**For:** SESSION-PERSISTENCE-ARCHITECTURE.md v1.2.0
**Date:** 2025-12-31

---

## TL;DR: What Happens When Things Fail

### The One-Sentence Answer

**If service discovery fails, the system falls back to "Portal + MemFd + Manual Dialog Each Time" which works on any compositor with portal support.**

---

## Failure → Fallback Quick Matrix

| What Failed | Fallback | Still Works? | User Impact |
|-------------|----------|--------------|-------------|
| Compositor detection | `Unknown` → Portal + MemFd | ✅ Yes | Slower, no optimizations |
| Portal v0-3 (no tokens) | Dialog every time | ✅ Yes | Must click Allow each restart |
| Secret Service missing | Encrypted file | ✅ Yes | Weaker encryption binding |
| TPM missing | Secret Service or file | ✅ Yes | No TPM-bound keys |
| machine-id missing | Hostname or static key | ✅ Yes | Weak key derivation |
| Deployment unknown | Assume Native | ✅ Yes | May log warnings |
| Portal completely missing | wlr-screencopy (wlroots) or **FATAL** | ⚠️ Maybe | Can't start if no alternative |

---

## The Only Fatal Failures

Only **2 scenarios** prevent the server from starting:

1. **No portal AND no wlr-screencopy**
   - Affects: Flatpak (no wlr access), KDE without portal, GNOME without portal
   - Fix: Install `xdg-desktop-portal` + backend for your compositor

2. **No D-Bus session**
   - Affects: initd/OpenRC without proper setup
   - Fix: Set `DBUS_SESSION_BUS_ADDRESS` and `XDG_RUNTIME_DIR`

**Everything else degrades gracefully.**

---

## Safe Mode Always Works

If detection completely fails, invoke safe mode:

```bash
lamco-rdp-server --safe-mode
```

**Guaranteed to work** on any compositor with portal support:
- Portal capture via MemFd
- Software H.264 encoding
- Painted cursor
- No token persistence (dialog each time)
- 30 FPS

---

## Diagnostic Command Reference

```bash
# See what was detected
lamco-rdp-server --show-capabilities

# Test individual components
lamco-rdp-server --test-compositor-detection
lamco-rdp-server --test-portal-connection
lamco-rdp-server --test-credential-storage
lamco-rdp-server --test-deployment-detection

# Full diagnostic report
lamco-rdp-server --diagnose

# Safe mode (skip detection, use hardcoded defaults)
lamco-rdp-server --safe-mode

# Show persistence status
lamco-rdp-server --persistence-status
```

---

## Defaults by Component

### Compositor Detection

| Priority | Method | Default if Failed |
|----------|--------|-------------------|
| 1 | `XDG_CURRENT_DESKTOP` env | Try next |
| 2 | `DESKTOP_SESSION` env | Try next |
| 3 | Compositor-specific env vars | Try next |
| 4 | Process detection (`pgrep`) | Try next |
| 5 | **FALLBACK** | `CompositorType::Unknown` |

### Portal Capabilities

| Property | Default if Failed |
|----------|-------------------|
| `version` | 0 (assume no portal) |
| `supports_screencast` | false |
| `supports_remote_desktop` | false |
| `supports_clipboard` | false |
| `available_cursor_modes` | empty vector |
| `available_source_types` | empty vector |

### Credential Storage

| Priority | Method | Default if Failed |
|----------|--------|-------------------|
| 1 | TPM 2.0 via systemd-creds | Try next (systemd only) |
| 2 | Flatpak Secret Portal | Try next (Flatpak only) |
| 3 | Secret Service (GNOME Keyring/KWallet) | Try next |
| 4 | Encrypted file + machine-id key | Try next |
| 5 | Encrypted file + hostname key | Try next |
| 6 | **FALLBACK** | Encrypted file + static salt (weak) |

**Never fails completely** - encrypted file storage always available.

### Deployment Context

| Detection Check | Default if Failed |
|-----------------|-------------------|
| `/.flatpak-info` exists? | No → Not Flatpak |
| `INVOCATION_ID` set? | No → Not systemd |
| `/run/systemd/system` exists? | No → Not systemd |
| `/run/openrc` exists? | No → Not OpenRC |
| **FALLBACK** | `DeploymentContext::Native` |

---

## Service Level Fallback Behavior

When code queries a service:

```rust
match service_registry.service_level(ServiceId::Something) {
    ServiceLevel::Guaranteed => {
        // Use optimal path
    }
    ServiceLevel::BestEffort => {
        // Use with caution, may have issues
    }
    ServiceLevel::Degraded => {
        // Use fallback implementation
    }
    ServiceLevel::Unavailable => {
        // Feature not available, skip or disable
    }
}
```

**Missing from registry:**
```rust
service_registry.service_level(ServiceId::NewFeature)
// Returns: ServiceLevel::Unavailable (safe default)
```

---

## What Each ServiceId Falls Back To

| ServiceId | If Unavailable | Fallback Behavior |
|-----------|----------------|-------------------|
| `VideoCapture` | **FATAL** | No fallback - must have screen capture |
| `RemoteInput` | **FATAL** | No fallback - must have input injection |
| `DamageTracking` | Frame differencing | Full frame comparison (slower, works) |
| `DmaBufZeroCopy` | Memory copy | Copy from MemFd buffers (slower, works) |
| `MetadataCursor` | Painted cursor | Embed cursor in video stream (compatible) |
| `ExplicitSync` | Implicit sync | May have tearing (usually fine) |
| `FractionalScaling` | Integer scaling | UI may be slightly blurry on HiDPI |
| `MultiMonitor` | Single monitor | Only primary monitor captured |
| `WindowCapture` | Full screen only | Cannot capture individual windows |
| `Clipboard` | Disable feature | No clipboard sync (non-critical) |
| `SessionPersistence` | Dialog every time | Manual permission grant required |
| `DirectCompositorAPI` | Use portal | One-time dialog, then token persistence |
| `WlrScreencopy` | Use portal | One-time dialog, then token persistence |
| `CredentialStorage` | Encrypted file | Weak key binding (still encrypted) |

---

## Common Failure Scenarios & Fixes

### "Unknown compositor" Warning

**Symptom:**
```
Detected compositor: Unknown
⚠️  Using generic Portal support
⚠️  DMA-BUF zero-copy disabled
```

**Cause:** Environment variables not set

**Fix:**
```bash
# Set before starting server
export XDG_CURRENT_DESKTOP=GNOME  # or KDE, sway, etc.
```

**Impact:** Reduces performance, but system still works.

---

### "Portal version < 4, tokens not supported"

**Symptom:**
```
⚠️  Portal v3 does not support restore tokens
⚠️  Permission dialog will appear on every server start
```

**Cause:** Old portal version installed

**Fix:**
```bash
# Update portal package
sudo apt update && sudo apt install xdg-desktop-portal

# Or upgrade your distro (Portal v4 shipped in 2023)
```

**Impact:** Manual dialog required each restart, but system works.

---

### "Secret Service unavailable"

**Symptom:**
```
⚠️  Secret Service not detected
    Fallback: Using encrypted file storage
```

**Cause:** No keyring daemon running

**Fix:**
```bash
# GNOME
sudo apt install gnome-keyring libsecret-1-0

# KDE
sudo apt install kwalletmanager

# Start keyring
eval $(gnome-keyring-daemon --start)
```

**Impact:** Tokens stored in encrypted file instead of keyring (still secure with machine-id).

---

### "FATAL: No screen capture capability detected"

**Symptom:**
```
❌ FATAL: No screen capture capability detected
```

**Cause:** Portal not running or no backend installed

**Fix:**
```bash
# Check if portal is running
systemctl --user status xdg-desktop-portal.service

# Install backend for your compositor
# GNOME:
sudo apt install xdg-desktop-portal-gnome

# KDE:
sudo apt install xdg-desktop-portal-kde

# Sway/wlroots:
sudo apt install xdg-desktop-portal-wlr

# Restart portal
systemctl --user restart xdg-desktop-portal.service
```

**Impact:** Server cannot start until fixed.

---

## Deployment Decision Quick Guide

| If you're... | Use this deployment | You get this fallback chain |
|--------------|---------------------|----------------------------|
| **Ubuntu/Fedora desktop user** | Native .deb/.rpm + systemd user | Portal token → File encryption |
| **Running headless server** | Native + systemd user + linger | Portal token → File encryption |
| **Want maximum compatibility** | Flatpak from Flathub | Portal token → Flatpak Secret Portal → File |
| **Security-focused** | Flatpak | Portal token → Flatpak Secret Portal (sandboxed) |
| **On Gentoo with OpenRC** | Native + OpenRC script | Portal token → Secret Service → File |
| **Testing/development** | Native binary | All strategies attempted → Portal token |

---

## The Absolute Worst Case

**Everything fails:**
- Compositor: Unknown
- Portal: v1 (no tokens)
- Secret Service: None
- TPM: None
- machine-id: None
- hostname: None

**Result:**
```
Compositor: Unknown (Portal + MemFd)
Persistence: Degraded (Manual dialog each time)
Credentials: EncryptedFile with static salt (WEAK)

System status: ✅ FUNCTIONAL
               ⚠️  DEGRADED SECURITY & UX
```

**System STILL WORKS** - just requires manual dialog and has weak token encryption.

---

## Key Takeaways

1. **Portal is the universal fallback** - If portal works, system works
2. **Only 2 truly fatal errors** - No portal AND no wlr-screencopy, or no D-Bus
3. **Everything else degrades** - Slower, less secure, more manual, but functional
4. **Flatpak forces portal-only** - But this is fine, portal tokens work well
5. **Phase 1 + 2 handle all fallbacks** - Later phases are optimizations only

---

*For comprehensive failure analysis, see: FAILURE-MODES-AND-FALLBACKS.md*
