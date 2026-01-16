# wlr-direct Native Build & Testing Guide

**Date:** 2026-01-16
**Purpose:** Build and test wlr-direct strategy on wlroots compositors (Sway, Hyprland)
**Status:** Ready for testing

---

## ⚠️ CRITICAL: Deployment Context

**wlr-direct is NOT available in Flatpak**

- Flatpak sandbox blocks direct Wayland socket access
- wlr-direct requires native Wayland protocol binding
- Flatpak builds should use Portal-only approach

**This guide is for NATIVE builds only** (systemd user service, direct execution).

---

## Prerequisites

### Target Environment
- wlroots-based compositor (Sway, Hyprland, River, labwc)
- Wayland session (not X11)
- Rust 1.77+ toolchain
- Development libraries:
  ```bash
  # Arch/Fedora
  sudo pacman -S wayland-protocols libxkbcommon

  # Ubuntu/Debian
  sudo apt install libwayland-dev libxkbcommon-dev
  ```

### Test VM Setup
According to DISTRO-TESTING-MATRIX.md:
- Sway on Arch/Fedora (recommended)
- Hyprland on Arch (has known Portal bugs, test native wlr-direct)

---

## Build Instructions

### 1. Build with wayland Feature

```bash
# Clean build to ensure all dependencies are fresh
cargo clean

# Build with wayland feature enabled (includes wlr-direct)
cargo build --release --features "wayland,h264"
```

**What this enables:**
- `wayland-client`, `wayland-protocols`, `wayland-protocols-wlr` dependencies
- `rustix` for memfd (XKB keymap sharing)
- wlr_direct module compiled and available
- Service registry detects zwp_virtual_keyboard_v1, zwlr_virtual_pointer_v1 protocols

### 2. Install Binary

```bash
# Install to /usr/local/bin (requires sudo)
sudo install -Dm755 target/release/lamco-rdp-server /usr/local/bin/lamco-rdp-server

# Or install to ~/.local/bin (no sudo)
install -Dm755 target/release/lamco-rdp-server ~/.local/bin/lamco-rdp-server
```

### 3. Create Configuration

```bash
# Create config directory
mkdir -p ~/.config/lamco-rdp-server/certs

# Copy template config
cp config/rhel9-config.toml ~/.config/lamco-rdp-server/config.toml

# Generate TLS certificates
openssl req -x509 -newkey rsa:2048 \
  -keyout ~/.config/lamco-rdp-server/certs/key.pem \
  -out ~/.config/lamco-rdp-server/certs/cert.pem \
  -days 365 -nodes -subj '/CN=rdp-server'

# Update certificate paths in config
sed -i "s|cert_path = .*|cert_path = \"$HOME/.config/lamco-rdp-server/certs/cert.pem\"|" \
  ~/.config/lamco-rdp-server/config.toml
sed -i "s|key_path = .*|key_path = \"$HOME/.config/lamco-rdp-server/certs/key.pem\"|" \
  ~/.config/lamco-rdp-server/config.toml
```

---

## Running & Testing

### 1. Start Server with Verbose Logging

```bash
lamco-rdp-server \
  -c ~/.config/lamco-rdp-server/config.toml \
  --log-file ~/rdp-server.log \
  -vvv
```

### 2. Verify Strategy Selection

**Expected log output:**
```
╔════════════════════════════════════════════════════════════╗
║              Service Advertisement Registry                ║
╚════════════════════════════════════════════════════════════╝
  Compositor: Sway 1.9
  Services: 8 guaranteed, 4 best-effort, 1 degraded, 3 unavailable
  ─────────────────────────────────────────────────────────
  ✅ wlr-direct Input   [Guaranteed] → Input (full)
      ↳ Direct input injection without portal permission dialog
  ...

✅ Selected: wlr-direct strategy
   Native Wayland protocols for wlroots compositors
   Compositor: Sway 1.9
   Note: Input only (video via Portal ScreenCast)

🚀 wlr_direct: Creating session with native Wayland protocols
🔌 wlr_direct: Connected to Wayland display
✅ wlr_direct: Bound to virtual keyboard and pointer protocols
🔑 wlr_direct: Creating virtual keyboard with XKB keymap
✅ wlr_direct: Virtual keyboard created with system keymap
✅ wlr_direct: Virtual pointer created successfully
```

**Key indicators of success:**
- Service registry lists "wlr-direct Input" as **Guaranteed**
- Strategy selector chooses **wlr-direct strategy**
- Virtual keyboard and pointer created successfully

### 3. Test Input Injection

Connect from RDP client:
```bash
# From another machine
xfreerdp /v:<VM-IP>:3389 /u:<username> /cert:ignore
```

**Keyboard test checklist:**
- [ ] Type in terminal - text appears
- [ ] Press Enter - command executes
- [ ] Modifier keys: Ctrl+C, Alt+Tab, Shift+letters
- [ ] Special keys: F1-F12, arrows, Home/End, Page Up/Down
- [ ] International layout (if applicable)

**Mouse test checklist:**
- [ ] Cursor moves smoothly
- [ ] Left click - selects text, clicks buttons
- [ ] Right click - opens context menu
- [ ] Middle click (if available)
- [ ] Scroll wheel - vertical scrolling works
- [ ] Horizontal scroll (Shift+wheel)

**Multi-monitor test (if available):**
- [ ] Mouse moves across monitors
- [ ] Coordinates transform correctly
- [ ] Click works on all monitors

---

## Expected Behavior Differences

### vs. Flatpak (Portal-only)
| Aspect | Flatpak | Native wlr-direct |
|--------|---------|-------------------|
| Input strategy | Portal RemoteDesktop | wlr-direct protocols |
| Permission dialog | One-time (if tokens work) | **ZERO dialogs** |
| Latency | ~2-5ms Portal overhead | Sub-millisecond (direct) |
| Setup | Automatic | Requires compositor support |

### vs. Portal on wlroots
**Why wlr-direct is better:**
- xdg-desktop-portal-wlr does NOT implement RemoteDesktop API
- Portal-only approach would have **NO input injection** on wlroots
- wlr-direct is the ONLY working input path for Sway/Hyprland/etc.

---

## Troubleshooting

### "wlr-direct strategy unavailable"

**Check service registry output:**
```bash
grep "wlr-direct Input" ~/rdp-server.log
```

**Expected if unavailable:**
```
❌ wlr-direct Input   [Unavailable]
   ↳ Virtual keyboard/pointer protocols not found
```

**Causes:**
1. **Compositor not wlroots-based:**
   - Running on GNOME, KDE, etc.
   - Solution: This is correct, use Portal strategy instead

2. **Protocols not exposed:**
   - Check with: `wayland-info | grep -E 'virtual_keyboard|virtual_pointer'`
   - Should see: `zwp_virtual_keyboard_manager_v1`, `zwlr_virtual_pointer_manager_v1`

3. **wayland feature not enabled:**
   - Rebuild with: `cargo build --release --features "wayland,h264"`

### "Failed to bind required Wayland protocols"

**Check Wayland connection:**
```bash
echo $WAYLAND_DISPLAY  # Should show: wayland-0 or similar
```

**Check protocol availability:**
```bash
wayland-info | grep -A5 "zwp_virtual_keyboard_manager_v1"
wayland-info | grep -A5 "zwlr_virtual_pointer_manager_v1"
```

**Both protocols should appear.** If missing:
- zwp_virtual_keyboard missing → Compositor doesn't support virtual keyboard
- zwlr_virtual_pointer missing → Compositor not wlroots or version < 0.12

### "Failed to generate XKB keymap"

**Check xkbcommon installation:**
```bash
ldconfig -p | grep xkbcommon
# Should show: libxkbcommon.so.0
```

**Check XKB environment:**
```bash
echo $XKB_DEFAULT_RULES    # Optional, shows if set
echo $XKB_DEFAULT_LAYOUT   # Optional, shows if set
```

**If missing:** Install libxkbcommon-dev package.

### Keyboard works but wrong layout

**Check XKB configuration:**
```bash
setxkbmap -query  # Shows current layout
```

**wlr-direct uses system defaults** from xkbcommon library.
To override, set environment variables:
```bash
export XKB_DEFAULT_LAYOUT="us"
export XKB_DEFAULT_VARIANT="dvorak"  # Optional
lamco-rdp-server -c config.toml
```

---

## Performance Metrics

### Expected Latency (Native wlr-direct)
- Input injection: < 1ms (direct Wayland protocol)
- No D-Bus overhead
- No Portal serialization
- Direct compositor event queue

### Comparison to Portal
- Portal RemoteDesktop: ~2-5ms overhead (D-Bus + libei)
- wlr-direct: ~0.5ms (direct protocol)
- **4-10x lower latency** for input

---

## Known Limitations (MVP)

### Input Only
- **Video:** Uses Portal ScreenCast (separate session)
- **Input:** Uses wlr-direct (this implementation)
- **Clipboard:** Uses FUSE or separate Portal session

### Why Hybrid Approach?
- wlr-screencopy (video capture) not yet implemented
- Portal ScreenCast works universally for video
- wlr-direct provides best input experience

### Future Enhancement
- Integrate wlr-screencopy for zero-dialog video capture
- Full wlr-only session (no Portal dependency)

---

## Success Criteria

### ✅ Implementation Complete When:
1. Service registry detects WlrDirectInput as Guaranteed
2. Strategy selector chooses wlr-direct on wlroots
3. Virtual keyboard created with XKB keymap
4. Virtual pointer created successfully
5. Keyboard input works in RDP session
6. Mouse input works in RDP session
7. No permission dialogs (zero-dialog operation)

### ✅ Testing Complete When:
1. Tested on Sway (Arch or Fedora)
2. Tested on Hyprland (Arch)
3. All keyboard test cases pass
4. All mouse test cases pass
5. Multi-monitor works (if available)
6. Performance metrics meet expectations

---

## Test Results Template

```markdown
## Test: wlr-direct on [Compositor] [Version]

**Environment:**
- Compositor: [Sway 1.9 / Hyprland / etc.]
- Distribution: [Arch Linux / Fedora 40 / etc.]
- Kernel: [uname -r]
- Portal version: [N/A for wlr-direct]

**Build:**
- Features: wayland, h264
- Build command: cargo build --release --features "wayland,h264"

**Service Detection:**
- [✅/❌] WlrDirectInput detected as Guaranteed
- [✅/❌] Strategy selector chose wlr-direct
- [✅/❌] Virtual keyboard created
- [✅/❌] Virtual pointer created

**Keyboard Tests:**
- [✅/❌] Basic typing
- [✅/❌] Modifiers (Ctrl+C, Alt+Tab)
- [✅/❌] Special keys (F-keys, arrows)

**Mouse Tests:**
- [✅/❌] Cursor movement
- [✅/❌] Left/right/middle click
- [✅/❌] Scroll wheel

**Performance:**
- Input latency: [< 1ms / measured value]
- CPU usage: [%]

**Notes:**
[Any issues, quirks, or observations]
```

---

## See Also

- [DISTRO-TESTING-MATRIX.md](../DISTRO-TESTING-MATRIX.md) - Testing status
- [WLR-INPUT-IMPLEMENTATION-HANDOVER.md](../WLR-INPUT-IMPLEMENTATION-HANDOVER.md) - Implementation details
- [FLATPAK-DEPLOYMENT.md](../FLATPAK-DEPLOYMENT.md) - Portal-only approach
