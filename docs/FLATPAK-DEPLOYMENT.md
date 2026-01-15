# Flatpak Deployment Guide

**Last Updated:** 2026-01-15
**Applies To:** lamco-rdp-server Flatpak bundle

---

## Design Philosophy

This project implements **native Wayland and XDG Desktop Portal capabilities only**. We do not invest significant effort in supporting legacy X11, Mutter D-Bus introspection workarounds, or other older technologies.

**What this means:**

- **Portal-first approach**: All screen capture, input injection, and clipboard operations go through standardized Portal APIs
- **Wayland-native**: Built for the Wayland display protocol, not X11 compatibility layers
- **Token-based persistence**: Leverages Portal v4+ restore tokens for zero-dialog operation after initial setup
- **Forward-looking**: As desktop environments improve their Portal implementations, this server benefits automatically

**Why this approach:**

1. **Standards compliance**: Portal is the freedesktop.org standard for sandboxed app access to desktop features
2. **Cross-desktop compatibility**: Works across GNOME, KDE, Sway, and other Wayland compositors
3. **Security model**: Respects user consent through Portal dialogs rather than privileged access
4. **Maintainability**: Single, well-documented API rather than compositor-specific hacks

**Known limitations of this approach:**

- Older systems without Portal v4+ may show permission dialogs on each restart
- Clipboard sync depends on Portal RemoteDesktop v2+ (not available on all platforms)
- Some features may lag behind native compositor capabilities

---

## Quick Start

### Prerequisites

- Flatpak installed on the system
- Freedesktop Platform 24.08 runtime
- Active graphical (Wayland) session
- TLS certificates

### Installation

```bash
# Install runtime (one-time)
flatpak install --user flathub org.freedesktop.Platform//24.08

# Install from bundle
flatpak install --user ./io.lamco.rdp-server.flatpak
```

### Running

```bash
flatpak run io.lamco.rdp-server -c ~/.config/lamco-rdp-server/config.toml
```

---

## Configuration

### Config File Location

The Flatpak has access to `~/.config/lamco-rdp-server/` for configuration and data storage.

**Required structure:**
```
~/.config/lamco-rdp-server/
├── config.toml          # Main configuration
└── certs/
    ├── cert.pem         # TLS certificate
    └── key.pem          # TLS private key
```

### Config File Format

**CRITICAL:** The config.toml requires ALL mandatory sections. A minimal config will fail to parse.

**Required sections:**
- `[server]` - Listen address, max connections
- `[security]` - Certificate paths, authentication
- `[video]` - Encoder settings, FPS, bitrate
- `[video_pipeline.processor]` - Frame processing
- `[video_pipeline.dispatcher]` - Frame dispatch
- `[video_pipeline.converter]` - Color conversion
- `[input]` - Keyboard/mouse settings
- `[clipboard]` - Clipboard sync settings
- `[multimon]` - Multi-monitor support
- `[performance]` - Threading, buffers
- `[logging]` - Log level, metrics

**Optional sections (have defaults):**
- `[egfx]` - Graphics pipeline extension
- `[damage_tracking]` - Efficient updates
- `[hardware_encoding]` - VA-API settings
- `[display]` - Resolution control
- `[advanced_video]` - Frame skip, quality

### Minimal Working Config

Copy the full example from `config/rhel9-config.toml` and modify only:
- `security.cert_path` - Path to your certificate
- `security.key_path` - Path to your private key

**DO NOT** try to create a minimal config with just a few sections - it will fail.

---

## TLS Certificate Setup

### Generate Self-Signed Certificates (Testing)

```bash
mkdir -p ~/.config/lamco-rdp-server/certs

openssl req -x509 -newkey rsa:2048 \
  -keyout ~/.config/lamco-rdp-server/certs/key.pem \
  -out ~/.config/lamco-rdp-server/certs/cert.pem \
  -days 365 -nodes -subj '/CN=rdp-server'
```

### Production Certificates

For production, use certificates from your organization's CA or a service like Let's Encrypt.

Update `config.toml`:
```toml
[security]
cert_path = "/home/username/.config/lamco-rdp-server/certs/cert.pem"
key_path = "/home/username/.config/lamco-rdp-server/certs/key.pem"
```

**Note:** Use absolute paths. The Flatpak sandbox maps the home directory, so `/home/username/...` paths work correctly.

---

## Known Issues

### Initial Connection Delay

**Symptom:** First RDP connection after server start takes 5-10 seconds to display video.

**Cause:** PipeWire buffer negotiation. During initial connection, the server attempts MemFd buffer mapping which may fail several times before succeeding. This is normal behavior.

**Log messages you may see:**
```
WARN lamco_rdp_server::video::pipewire_capture: MemFd buffer mmap failed (safe, retrying)
WARN lamco_rdp_server::video::pipewire_capture: Buffer mmap failed again
```

**Status:** This is expected behavior. The connection succeeds after buffer negotiation completes. Subsequent reconnections are faster.

### Session Persistence Rejected

**Symptom:** "Portal rejected persistence request" in logs, session won't persist across restarts.

**Actual Error:** `Remote desktop sessions cannot persist`

**Cause:** Some Portal implementations (notably GNOME's xdg-desktop-portal-gnome) reject session persistence for RemoteDesktop sessions. This is a deliberate policy decision by the Portal backend, not a bug.

**What this means:**
- First run: Permission dialog appears, session works
- Server restart: Permission dialog appears again (token not saved)
- The RDP session itself works correctly

**Workaround:** The server automatically detects this error and retries without persistence. Functionality is preserved, but each server restart requires user approval.

**Future:** This behavior may change in newer GNOME/Portal versions. We are monitoring upstream changes.

---

## Troubleshooting

### "Failed to parse config file"

**Cause:** Missing required config sections.

**Solution:** Use the complete config template from `config/rhel9-config.toml`. Do not try to create a minimal config.

### "Failed to load TLS certificates"

**Cause:** Certificate files not found or wrong path.

**Checklist:**
1. Verify files exist:
   ```bash
   ls -la ~/.config/lamco-rdp-server/certs/
   ```

2. Verify Flatpak can see them:
   ```bash
   flatpak run --command=ls io.lamco.rdp-server -la ~/.config/lamco-rdp-server/certs/
   ```

3. Check paths in config.toml use absolute paths (starting with `/home/`)

4. Verify certificate format:
   ```bash
   head -1 ~/.config/lamco-rdp-server/certs/cert.pem
   # Should show: -----BEGIN CERTIFICATE-----
   ```

### "Could not identify compositor"

**Cause:** Running via SSH without graphical session environment.

**Solution:** This warning is normal when checking capabilities via SSH. The server needs to be run from within an active graphical session (logged in at the console or via VNC).

### Portal Permission Dialog

**First run:** You will see a permission dialog asking to share your screen. Accept it.

**Subsequent runs:** If Portal version 4+ is detected, the restore token will be saved and no dialog appears on restart.

### Clipboard Not Available

**Cause:** GNOME Portal on older versions (v3/v4) doesn't expose clipboard API.

**Status:** This is a platform limitation on GNOME 40-45. Clipboard sync is not available on these systems through the Portal.

---

## Platform-Specific Notes

### RHEL 9 / AlmaLinux 9 / Rocky Linux 9

- **Portal Version:** 4 (supports restore tokens)
- **GNOME Version:** 40.x
- **Clipboard:** Not available (Portal RemoteDesktop v1)
- **Session Persistence:** Portal REJECTS persistence for RemoteDesktop sessions

**Known behavior:**
- Portal capabilities report v4 with restore token support
- However, GNOME's portal backend rejects persistence for RemoteDesktop
- Each server restart requires a permission dialog

**Expected behavior:**
- First run: 1 permission dialog (screen sharing)
- Server restart: 1 permission dialog again (persistence rejected)
- RDP functionality: Full video, keyboard, mouse working

### Ubuntu 24.04 LTS

- **Portal Version:** 5
- **GNOME Version:** 46.x
- **Clipboard:** Not available (GNOME 46 bug)
- **Session Persistence:** Works via restore tokens

### Ubuntu 22.04 LTS

- **Portal Version:** 3 (NO restore tokens)
- **GNOME Version:** 42.x
- **Session Persistence:** May require Mutter direct API

**Note:** Portal v3 does not support restore tokens. Each restart may show a permission dialog.

---

## Verbose Logging

For debugging, run with verbose logging:

```bash
flatpak run io.lamco.rdp-server \
  -c ~/.config/lamco-rdp-server/config.toml \
  --log-file ~/rdp-server.log \
  -vvv
```

Log levels:
- `-v` - Info + warnings
- `-vv` - Debug messages
- `-vvv` - Trace (very verbose)

---

## Flatpak Permissions

The Flatpak requests these permissions:

| Permission | Purpose |
|------------|---------|
| `network` | Accept RDP connections |
| `wayland` | Screen capture |
| `pulseaudio` | Audio (future) |
| `ipc` | D-Bus communication |
| `session-bus` | Portal access |
| `~/.config/lamco-rdp-server:create` | Config storage |
| `xdg-run/pipewire-0` | PipeWire video streams |

Portal D-Bus access:
- `org.freedesktop.portal.Desktop`
- `org.freedesktop.portal.ScreenCast`
- `org.freedesktop.portal.RemoteDesktop`

---

## See Also

- [DISTRO-TESTING-MATRIX.md](DISTRO-TESTING-MATRIX.md) - Platform compatibility
- [config/rhel9-config.toml](../config/rhel9-config.toml) - Full config template

