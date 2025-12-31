# Technology: Wayland Integration

**URL:** `https://lamco.ai/technology/wayland/`
**Status:** Draft for review

---

## The Wayland Transition

Linux desktops are transitioning from X11 to Wayland. Major distributions now default to Wayland:

- **Ubuntu 24.04+:** Wayland default
- **Fedora:** Wayland default since Fedora 25
- **GNOME:** Wayland default since 3.22
- **KDE Plasma:** Wayland default on many distributions

This transition brings significant benefits—better security, improved performance, proper HiDPI support—but it also breaks traditional remote desktop approaches that relied on X11's open architecture.

---

## Why X11 Remote Desktop Doesn't Work on Wayland

### The X11 Approach

X11 was designed with network transparency. Any application could:
- Read the contents of any window
- Inject keyboard and mouse events
- Access the clipboard globally

Remote desktop servers like xrdp exploited this openness.

### The Wayland Security Model

Wayland applications are isolated by design:
- Applications cannot see other windows
- Input injection requires explicit permission
- Clipboard access is mediated

This is **good for security** but means the X11 remote desktop approach simply doesn't work.

### The Xwayland Workaround

Some solutions run an Xwayland server (X11 compatibility layer) to capture. This works but has serious drawbacks:

| Issue | Impact |
|-------|--------|
| Extra layer | Added latency and complexity |
| Partial capture | Native Wayland windows may not appear |
| Security bypass | Defeats Wayland's isolation |
| Resource overhead | Running two display servers |

---

## The Portal-Based Solution

### XDG Desktop Portals

The XDG Desktop Portal specification provides a **sanctioned way** for applications to request privileged operations with user consent.

```
Application                Portal                    Compositor
     │                        │                           │
     │  "I want to capture    │                           │
     │   the screen"          │                           │
     │ ───────────────────────►                           │
     │                        │                           │
     │                        │    "User, allow this?"    │
     │                        │ ──────────────────────────►
     │                        │                           │
     │                        │    "Yes, user clicked     │
     │                        │     allow"                │
     │                        │ ◄──────────────────────────
     │                        │                           │
     │  "Here's your          │                           │
     │   PipeWire stream"     │                           │
     │ ◄───────────────────────                           │
     │                        │                           │
```

The user explicitly grants permission. The compositor provides the screen content. The security model is maintained.

### lamco-rdp-server Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        lamco-rdp-server                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐        │
│  │ lamco-portal│    │lamco-pipewire│   │ lamco-video │        │
│  │             │    │             │    │             │        │
│  │ Portal      │    │ PipeWire    │    │ Frame       │        │
│  │ Negotiation │───►│ Capture     │───►│ Processing  │        │
│  │             │    │             │    │             │        │
│  └─────────────┘    └─────────────┘    └─────────────┘        │
│         │                  │                  │                │
│         ▼                  ▼                  ▼                │
│  ┌─────────────────────────────────────────────────────┐      │
│  │              H.264 Encoding Pipeline                │      │
│  │         (OpenH264 / NVENC / VA-API)                 │      │
│  └─────────────────────────────────────────────────────┘      │
│                            │                                   │
│                            ▼                                   │
│  ┌─────────────────────────────────────────────────────┐      │
│  │                    RDP Protocol                      │      │
│  │                    (IronRDP)                         │      │
│  └─────────────────────────────────────────────────────┘      │
│                            │                                   │
└────────────────────────────┼───────────────────────────────────┘
                             │
                             ▼
                       RDP Client
```

---

## Portal Components

### ScreenCast Portal

Provides screen capture via PipeWire streams.

**D-Bus interface:** `org.freedesktop.portal.ScreenCast`

**Capabilities:**
- Single monitor capture
- Multi-monitor capture
- Window capture (compositor dependent)
- Cursor metadata or embedded

**lamco-portal integration:**
```rust
// Simplified flow
let portal = ScreenCastPortal::new().await?;
let session = portal.create_session().await?;
let stream = portal.start(session).await?;  // User sees permission dialog
// stream is now a PipeWire node ID
```

### RemoteDesktop Portal

Provides input injection via libei (Emulated Input).

**D-Bus interface:** `org.freedesktop.portal.RemoteDesktop`

**Capabilities:**
- Keyboard events (scancodes)
- Mouse movement (absolute and relative)
- Mouse buttons and scroll
- Multi-touch (compositor dependent)

**Combined session:**
The ScreenCast and RemoteDesktop portals can share a session, providing both capture and input with a single user permission prompt.

### Clipboard Access

Two approaches for clipboard:

1. **Portal clipboard** (`org.freedesktop.portal.Clipboard`)
   - Sandboxed, permission-based
   - Not yet widely supported

2. **D-Bus clipboard** (GNOME-specific)
   - Direct D-Bus communication with gnome-shell
   - Fallback for older systems

lamco-rdp-server detects which is available and uses the appropriate method.

---

## PipeWire Integration

### What Is PipeWire?

PipeWire is the modern Linux multimedia framework, replacing both PulseAudio (audio) and much of GStreamer's role. For screen capture, it provides efficient zero-copy frame delivery.

### DMA-BUF Zero-Copy

When both GPU and PipeWire support it, frames never leave GPU memory:

```
Compositor GPU Buffer
        │
        │ (DMA-BUF file descriptor)
        ▼
PipeWire Stream ──────────► lamco-pipewire
        │                         │
        │                         │ (same GPU buffer)
        ▼                         ▼
   No CPU copy              H.264 Encoder
                           (NVENC/VA-API)
```

**Benefit:** Minimal latency, zero CPU memory bandwidth.

### Memory-Mapped Fallback

When DMA-BUF isn't available:

```
Compositor ──► PipeWire ──► Memory Map ──► lamco-pipewire ──► Encoder
                                 │
                            CPU memory copy
```

Still efficient, but requires one CPU copy.

### lamco-pipewire Crate

Our open-source PipeWire integration:

| Feature | Status |
|---------|--------|
| Stream negotiation | ✓ |
| DMA-BUF import | ✓ |
| Memory-mapped buffers | ✓ |
| Format negotiation | ✓ |
| Cursor metadata | ✓ |
| Hardware cursor extraction | ✓ |
| Multi-stream (multi-monitor) | ✓ |

[View on crates.io →](https://crates.io/crates/lamco-pipewire)

---

## Input Injection (libei)

### The libei Protocol

libei (Emulated Input) is the standard for injecting input events on Wayland with user consent.

```
lamco-rdp-server                  Compositor
       │                               │
       │  EI client connection         │
       │ ─────────────────────────────►│
       │                               │
       │  Capabilities negotiation     │
       │ ◄─────────────────────────────│
       │                               │
       │  Keyboard/mouse events        │
       │ ─────────────────────────────►│
       │                               │
       │              Events injected  │
       │              into desktop     │
       │                               │
```

### lamco-rdp-input Crate

Translates RDP input events to libei format:

| RDP Event | libei Event |
|-----------|-------------|
| Keyboard scancode | ei_keyboard_key |
| Mouse movement | ei_pointer_motion / ei_pointer_motion_absolute |
| Mouse button | ei_pointer_button |
| Mouse scroll | ei_scroll_delta |

**Coordinate mapping:**
Multi-monitor setups require coordinate transformation. lamco-rdp-input handles mapping RDP client coordinates to the correct output and position.

[View on crates.io →](https://crates.io/crates/lamco-rdp-input)

---

## Service Advertisement Registry

One of lamco-rdp-server's distinguishing features is its **Service Advertisement Registry**—an integrated system for discovering Wayland capabilities and translating them to RDP features.

### The Problem

Wayland is not a single specification. It's a protocol plus dozens of extensions, and support varies by:
- Compositor (GNOME, KDE, Sway, Hyprland)
- Portal implementation
- System configuration
- Installed packages

A remote desktop server can't assume anything. What works on GNOME may not work on Sway. What works with portal version X may fail with version Y.

### Our Solution: Runtime Capability Discovery

At startup, lamco-rdp-server probes the system to build a complete picture of available capabilities:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Service Advertisement Registry                    │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐ │
│  │  Compositor     │    │  Portal         │    │  System         │ │
│  │  Probing        │    │  Probing        │    │  Probing        │ │
│  └────────┬────────┘    └────────┬────────┘    └────────┬────────┘ │
│           │                      │                      │          │
│           ▼                      ▼                      ▼          │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                    Capability Registry                       │  │
│  │                                                              │  │
│  │  screen_capture: Available (PipeWire + ScreenCast)          │  │
│  │  input_injection: Available (libei + RemoteDesktop)         │  │
│  │  clipboard: Available (D-Bus fallback)                      │  │
│  │  multi_monitor: Available (2 outputs detected)              │  │
│  │  dma_buf: Available (GPU: Intel UHD)                        │  │
│  │  hardware_encode: Available (VA-API: iHD driver)            │  │
│  │                                                              │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                              │                                     │
│                              ▼                                     │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                 RDP Capability Advertisement                 │  │
│  │                                                              │  │
│  │  → Advertise RDPGFX (video encoding available)              │  │
│  │  → Advertise CLIPRDR (clipboard available)                  │  │
│  │  → Advertise RDPINPUT (input available)                     │  │
│  │  → Advertise multi-monitor layout                           │  │
│  │  → Select optimal encoder backend                           │  │
│  │                                                              │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### What Gets Probed

#### Compositor Detection

| Check | Method | Information Gathered |
|-------|--------|----------------------|
| Desktop environment | `XDG_CURRENT_DESKTOP`, `DESKTOP_SESSION` | GNOME, KDE, etc. |
| GNOME | `GNOME_DESKTOP_SESSION_ID` | GNOME-specific paths |
| KDE | `KDE_FULL_SESSION`, `KDE_SESSION_VERSION` | Plasma version |
| Sway | `SWAYSOCK` | wlroots portal needed |
| Hyprland | `HYPRLAND_INSTANCE_SIGNATURE` | Hyprland portal path |
| Wayland display | `WAYLAND_DISPLAY` | Confirms Wayland session |

#### Portal Capability Probing

| Portal | D-Bus Interface | Capabilities Checked |
|--------|-----------------|----------------------|
| ScreenCast | `org.freedesktop.portal.ScreenCast` | Available sources, cursor modes |
| RemoteDesktop | `org.freedesktop.portal.RemoteDesktop` | Device types, combined session |
| Clipboard | `org.freedesktop.portal.Clipboard` | Format support |

```rust
// Simplified capability check
let screencast = dbus_proxy("org.freedesktop.portal.ScreenCast");
let version = screencast.property::<u32>("version").await?;
let sources = screencast.property::<u32>("AvailableSourceTypes").await?;

capabilities.screencast = ScreenCastCaps {
    available: true,
    version,
    supports_monitor: sources & 1 != 0,
    supports_window: sources & 2 != 0,
    supports_virtual: sources & 4 != 0,
};
```

#### System Capability Probing

| Component | Detection Method | Information |
|-----------|------------------|-------------|
| PipeWire | Socket check, version query | Stream support, DMA-BUF |
| libei | Library availability | Input injection support |
| VA-API | `vaInitialize()`, device enumeration | HW encode profiles |
| NVENC | `nvEncodeAPICreateInstance()` | NVIDIA encode availability |
| GPU | DRM device enumeration | DMA-BUF support, vendor |
| Displays | Wayland output enumeration | Monitor count, resolution |

### Capability-to-RDP Translation

The registry maps discovered capabilities to RDP protocol features:

| Wayland Capability | RDP Feature | Fallback |
|--------------------|-------------|----------|
| ScreenCast portal | RDPGFX video stream | — (required) |
| RemoteDesktop portal | Input injection | — (required) |
| Portal clipboard | CLIPRDR | D-Bus clipboard |
| libei | RDPINPUT extended | Basic input |
| VA-API/NVENC | Hardware RDPGFX | Software encode |
| Multiple outputs | Multi-monitor | Single monitor |
| DMA-BUF | Zero-copy capture | Memory-mapped |

### Why This Matters

**Graceful degradation:** Instead of crashing when a feature is unavailable, lamco-rdp-server advertises only what it can actually deliver.

**Optimal path selection:** The registry chooses the best available implementation:
- NVENC over VA-API over software (encoding)
- Portal clipboard over D-Bus fallback
- DMA-BUF over memory-mapped (capture)

**Clear diagnostics:** When something doesn't work, you know exactly what's missing:
```
[INFO] Service Registry initialized:
  ✓ ScreenCast: v4 (monitor, window)
  ✓ RemoteDesktop: v2 (keyboard, pointer)
  ✓ Clipboard: D-Bus fallback (portal unavailable)
  ✓ Hardware encode: VA-API (Intel iHD)
  ✓ DMA-BUF: Available
  ✓ Outputs: 2 monitors (3840x1080 total)
```

**Compositor-specific workarounds:** Known quirks are handled automatically:
- GNOME: D-Bus clipboard when portal clipboard unavailable
- Sway: Adjusted portal timeout values
- Hyprland: Layer-shell awareness

### Configuration

The registry runs automatically. You can inspect results in logs or override detection:

```toml
[services]
# Override auto-detection (usually unnecessary)
# force_software_encode = false
# force_dbus_clipboard = false
# assume_dma_buf = true

# Logging level for capability discovery
log_capability_discovery = true
```

---

## Compositor Compatibility

### Tested Compositors

| Compositor | ScreenCast | RemoteDesktop | DMA-BUF | Notes |
|------------|------------|---------------|---------|-------|
| **GNOME (Mutter)** | ✓ | ✓ | ✓ | Best tested |
| **KDE (KWin)** | ✓ | ✓ | ✓ | Full support |
| **Sway** | ✓ | ✓ | ✓ | wlroots-based |
| **Hyprland** | ✓ | ✓ | ✓ | wlroots-based |
| **weston** | Partial | Partial | ✓ | Reference compositor |

### Compositor Detection

lamco-rdp-server automatically detects your compositor:

```
Check environment variables:
  GNOME_DESKTOP_SESSION → GNOME
  KDE_FULL_SESSION → KDE Plasma
  SWAYSOCK → Sway
  HYPRLAND_INSTANCE_SIGNATURE → Hyprland

Apply compositor-specific behavior:
  - GNOME: Use D-Bus clipboard fallback
  - KDE: Adjust portal timeout
  - Sway: Use wlroots portal paths
  - Hyprland: Use wlroots portal paths
```

### Portal Implementations

Each compositor (or family) has its own portal implementation:

| Compositor | Portal Implementation |
|------------|-----------------------|
| GNOME | xdg-desktop-portal-gnome |
| KDE | xdg-desktop-portal-kde |
| wlroots | xdg-desktop-portal-wlr |
| Hyprland | xdg-desktop-portal-hyprland |

Ensure the correct portal implementation is installed for your compositor.

---

## Permission Flow

When you first connect:

1. **lamco-rdp-server requests screen capture**
2. **Portal daemon forwards to compositor**
3. **Compositor shows permission dialog:**

   ```
   ┌─────────────────────────────────────────┐
   │                                         │
   │  "lamco-rdp-server" wants to            │
   │  share your screen                      │
   │                                         │
   │  ○ Share entire screen                  │
   │  ○ Share window: [dropdown]             │
   │                                         │
   │         [Cancel]  [Share]               │
   │                                         │
   └─────────────────────────────────────────┘
   ```

4. **User grants permission**
5. **Capture begins**

### Persistent Permissions

Some portal implementations support persistent permissions:
- GNOME: Permissions remembered per-application
- KDE: Configurable permission persistence
- wlroots: Typically per-session

---

## Troubleshooting

### "No portal available"

**Symptom:** lamco-rdp-server fails to start with portal error.

**Fix:**
1. Install correct portal implementation for your compositor
2. Ensure xdg-desktop-portal service is running:
   ```bash
   systemctl --user status xdg-desktop-portal
   ```

### "Permission denied" or no dialog appears

**Symptom:** Capture fails silently or immediately.

**Fix:**
1. Check portal logs: `journalctl --user -u xdg-desktop-portal`
2. Try from a local session (not SSH)
3. Ensure you're running on Wayland (not Xwayland)

### No DMA-BUF support

**Symptom:** High CPU usage, logs show "falling back to memory-mapped buffers"

**Fix:**
1. Verify GPU driver supports DMA-BUF
2. Check PipeWire version (needs 0.3.x+)
3. Some VM environments don't support DMA-BUF

### Input not working

**Symptom:** Screen capture works but keyboard/mouse don't respond.

**Fix:**
1. Verify libei is installed
2. Check RemoteDesktop portal is available:
   ```bash
   gdbus introspect --session \
     --dest org.freedesktop.portal.Desktop \
     --object-path /org/freedesktop/portal/desktop \
     | grep RemoteDesktop
   ```
3. Ensure combined session (ScreenCast + RemoteDesktop) was requested

---

## Further Reading

- [XDG Desktop Portal Specification](https://flatpak.github.io/xdg-desktop-portal/)
- [PipeWire Documentation](https://docs.pipewire.org/)
- [libei Protocol](https://gitlab.freedesktop.org/libinput/libei)
- [Video Encoding Technology →](/technology/video-encoding/)
- [Performance Features →](/technology/performance/)
