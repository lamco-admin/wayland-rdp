# Platform Support Matrix and Service Detection

This document describes the platform detection, quirk system, and service availability
matrix for lamco-rdp-server. Understanding this system is critical for:

1. Diagnosing platform-specific issues
2. Understanding why certain features are unavailable on specific platforms
3. Contributing new platform profiles

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Platform Detection Flow](#platform-detection-flow)
- [Known Platform Quirks](#known-platform-quirks)
- [Platform Support Matrix](#platform-support-matrix)
- [Video Codec Matrix](#video-codec-matrix)
- [Service Detection](#service-detection)
- [Adding New Platforms](#adding-new-platforms)

---

## Architecture Overview

The server uses a three-layer detection system:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Capability Probing Layer                      │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────────┐  │
│  │ OS Detection    │  │ Compositor       │  │ Portal         │  │
│  │ /etc/os-release │  │ Detection        │  │ Probing        │  │
│  │                 │  │ (GNOME, KDE...)  │  │ (v1, v2...)    │  │
│  └────────┬────────┘  └────────┬─────────┘  └───────┬────────┘  │
│           │                    │                     │           │
│           └────────────────────┼─────────────────────┘           │
│                                ▼                                 │
│                    ┌───────────────────────┐                     │
│                    │  CompositorProfile    │                     │
│                    │  - Quirks             │                     │
│                    │  - Recommended settings│                    │
│                    └───────────┬───────────┘                     │
└────────────────────────────────┼─────────────────────────────────┘
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Service Registry Layer                        │
│                                                                  │
│  Translates capabilities into available services:               │
│  - Video: AVC420, AVC444, RemoteFX                              │
│  - Clipboard: Full, Read-only, None                             │
│  - Input: Full, Keyboard-only, None                             │
│  - Multi-monitor: Supported, Single-only                        │
└─────────────────────────────────────────────────────────────────┘
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Handler Configuration Layer                   │
│                                                                  │
│  Handlers receive quirk-aware configuration:                    │
│  - EGFX handler: force_avc420_only flag                         │
│  - Clipboard handler: disabled if unavailable                   │
│  - Input handler: cursor mode selection                         │
└─────────────────────────────────────────────────────────────────┘
```

---

## Platform Detection Flow

### 1. OS Detection (`probing.rs::detect_os_release()`)

Parses `/etc/os-release` to identify the Linux distribution:

```rust
pub struct OsRelease {
    pub id: String,           // "rhel", "fedora", "ubuntu"
    pub version_id: String,   // "9", "40", "24.04"
    pub name: String,         // Full name
    pub pretty_name: String,  // Display name
    pub id_like: Vec<String>, // Parent distros
}
```

**Helper Methods:**
- `is_rhel9()` - Check if running on RHEL 9.x
- `is_rhel8()` - Check if running on RHEL 8.x
- `is_rhel_family()` - Check if RHEL or derivative (CentOS, Rocky, Alma)
- `major_version()` - Get major version as integer

### 2. Compositor Detection (`probing.rs::identify_compositor()`)

Detects the Wayland compositor using multiple methods:

1. **Environment Variables** (most reliable):
   - `XDG_CURRENT_DESKTOP` - GNOME, KDE, sway, etc.
   - `DESKTOP_SESSION` - Session name
   - `SWAYSOCK`, `HYPRLAND_INSTANCE_SIGNATURE` - Compositor-specific

2. **Process Detection** (fallback):
   - `pgrep gnome-shell`, `pgrep kwin_wayland`, etc.

3. **Version Detection**:
   - `gnome-shell --version` → "46.0"
   - `plasmashell --version` → "6.0"
   - `sway --version` → "1.9"

### 3. Portal Capability Probing (`portal_caps.rs`)

Queries XDG Desktop Portal for available features:

```rust
pub struct PortalCapabilities {
    pub version: u32,              // Portal version (1, 2, etc.)
    pub supports_clipboard: bool,  // Clipboard sync available
    pub source_types: SourceType,  // Screen/window/monitor
    pub cursor_modes: CursorMode,  // Hidden/embedded/metadata
}
```

### 4. Profile Generation (`profiles.rs`)

Combines all detection results into a `CompositorProfile`:

```rust
pub struct CompositorProfile {
    pub compositor: CompositorType,
    pub quirks: Vec<Quirk>,
    pub recommended_capture: CaptureBackend,
    pub recommended_buffer_type: BufferType,
    pub supports_damage_hints: bool,
    pub recommended_fps_cap: u32,
    // ...
}
```

---

## Known Platform Quirks

Quirks are platform-specific issues that require workarounds.

### `Avc444Unreliable`

**Affected Platforms:** RHEL 9.x (all minor versions)

**Symptoms:** AVC444 (H.264 YUV444) encoding produces blurry, washed-out video
output. Text is particularly affected, appearing smeared.

**Root Cause:** Combination of:
- GNOME 40 (older mutter)
- Mesa 22.x
- Older GPU driver stack

**Workaround:** Server automatically forces AVC420 on RHEL 9. The EGFX handler
receives `force_avc420_only: true` and ignores client AVC444 capability.

**Detection:**
```rust
if os.is_rhel9() {
    quirks.push(Quirk::Avc444Unreliable);
}
```

### `ClipboardUnavailable`

**Affected Platforms:**
- RHEL 9.x (Portal v1)
- Any system with Portal version < 2

**Symptoms:** Clipboard synchronization is not available. Copy/paste between
RDP client and host fails silently.

**Root Cause:** XDG Desktop Portal version 1 does not include clipboard APIs.
Portal v2 (available in GNOME 45+) adds clipboard support.

**Workaround:** Server disables clipboard initialization. Log message indicates
"Clipboard sync unavailable (Portal v1 limitation)".

### `RequiresWaylandSession`

**Affected Platforms:** All GNOME, KDE Wayland sessions

**Symptoms:** Server fails to start if running under X11 or SSH without display.

**Workaround:** Check `WAYLAND_DISPLAY` and `XDG_SESSION_TYPE` environment
variables before initialization.

### `RestartCaptureOnResize`

**Affected Platforms:** GNOME (all versions)

**Symptoms:** After display resolution change, frames freeze or show stale content.

**Workaround:** Portal screen capture session must be restarted after resolution
changes. Server monitors for resolution change events and reinitializes capture.

### `NeedsExplicitCursorComposite`

**Affected Platforms:** Sway, Hyprland, wlroots-based compositors

**Symptoms:** Mouse cursor is not visible in captured frames.

**Workaround:** Server composites cursor into frames before encoding. Uses
cursor position and hotspot from compositor.

### `MultiMonitorPositionQuirk`

**Affected Platforms:** KDE Plasma 5.x (pre-Plasma 6)

**Symptoms:** Multi-monitor coordinates are offset incorrectly.

**Workaround:** Apply coordinate transformation based on detected monitor layout.

---

## Platform Support Matrix

| Platform | Video | Input | Clipboard | Multi-mon | Notes |
|----------|-------|-------|-----------|-----------|-------|
| **RHEL 9.x** | AVC420 | Full | None | Yes | Portal v1, AVC444 disabled |
| **RHEL 8.x** | AVC420 | Full | None | Yes | Portal v1 |
| **Fedora 40+** | AVC420+444 | Full | Full | Yes | Full support |
| **Ubuntu 24.04** | AVC420+444 | Full | Full | Yes | Full support |
| **Debian 12+** | AVC420+444 | Full | Full | Yes | Full support |
| **GNOME 45+** | AVC420+444 | Full | Full | Yes | Portal v2 |
| **GNOME 40-44** | AVC420 | Full | None | Yes | Portal v1 |
| **KDE Plasma 6** | AVC420+444 | Full | Full | Yes | Full support |
| **KDE Plasma 5** | AVC420+444 | Full | Full | Quirky | Position quirk |
| **Sway** | AVC420+444 | Full | Full | Yes | Cursor composite needed |
| **Hyprland** | AVC420+444 | Full | Full | Yes | Cursor composite needed |
| **Weston** | AVC420 | Limited | None | No | Reference compositor |

---

## Video Codec Matrix

### AVC420 (H.264 YUV 4:2:0)

**Description:** Standard H.264 encoding with 4:2:0 chroma subsampling.
Widely compatible, good for video content.

**Client Requirements:**
- EGFX V8.1+ with `AVC420_ENABLED` flag
- Windows 10/11 with MSTSC
- FreeRDP 2.x+

**Server Requirements:**
- OpenH264 encoder (software)
- VA-API (hardware, optional)

**Quality:** Good for video/images, text may show minor artifacts

### AVC444 (H.264 YUV 4:4:4)

**Description:** H.264 with full chroma resolution via dual-stream encoding.
Superior text/UI rendering.

**Client Requirements:**
- EGFX V10+ capability
- Windows 10/11 with MSTSC
- FreeRDP 2.x+

**Server Requirements:**
- OpenH264 encoder
- Platform without `Avc444Unreliable` quirk

**Quality:** Excellent for text and UI, ~50% more bandwidth

**Known Issues:**
- RHEL 9: Blurry output (quirk applied)
- Some GPU drivers: Color space issues

---

## Service Detection

The `ServiceRegistry` translates capabilities into advertised services.

### Service Levels

```rust
pub enum ServiceLevel {
    Full,        // Complete feature set
    Partial,     // Some features available
    ReadOnly,    // Read access only
    None,        // Feature disabled
}
```

### Detection Logic

```rust
impl ServiceRegistry {
    pub fn from_compositor(caps: CompositorCapabilities) -> Self {
        let mut services = HashMap::new();

        // Video codec selection based on quirks
        if caps.profile.has_quirk(&Quirk::Avc444Unreliable) {
            services.insert(ServiceId::VideoCodec, ServiceLevel::Partial);
            // Only AVC420 available
        } else {
            services.insert(ServiceId::VideoCodec, ServiceLevel::Full);
            // AVC420 + AVC444 available
        }

        // Clipboard based on Portal version
        if caps.portal.supports_clipboard {
            services.insert(ServiceId::Clipboard, ServiceLevel::Full);
        } else {
            services.insert(ServiceId::Clipboard, ServiceLevel::None);
        }

        // Input based on Portal capabilities
        if caps.portal.supports_remote_desktop {
            services.insert(ServiceId::Input, ServiceLevel::Full);
        }

        Self { services }
    }
}
```

### Querying Services

```rust
// Check if clipboard is available
if registry.get_level(ServiceId::Clipboard) == ServiceLevel::Full {
    initialize_clipboard();
}

// Check video codec support
match registry.get_level(ServiceId::VideoCodec) {
    ServiceLevel::Full => {
        // AVC420 + AVC444 available
    }
    ServiceLevel::Partial => {
        // AVC420 only (platform quirk)
    }
    _ => {
        // Software fallback
    }
}
```

---

## Adding New Platforms

### 1. Add OS Detection (if needed)

In `probing.rs`, add helper methods to `OsRelease`:

```rust
impl OsRelease {
    pub fn is_new_distro(&self) -> bool {
        self.id == "newdistro" && self.major_version() >= Some(1)
    }
}
```

### 2. Add Quirk (if needed)

In `profiles.rs`, add to the `Quirk` enum:

```rust
pub enum Quirk {
    // ... existing quirks ...

    /// Description of the new quirk
    NewPlatformIssue,
}
```

Update `description()`:

```rust
Self::NewPlatformIssue => "Description for logs",
```

### 3. Update Profile Generation

In `profiles.rs`, modify the relevant profile function:

```rust
fn gnome_profile(version: Option<&str>) -> Self {
    let os = detect_os_release();

    let mut quirks = vec![/* base quirks */];

    if let Some(ref os) = os {
        if os.is_new_distro() {
            quirks.push(Quirk::NewPlatformIssue);
            tracing::info!("New distro detected - applying quirk");
        }
    }

    // ... rest of profile
}
```

### 4. Handle Quirk in Relevant Handler

Example for EGFX handler:

```rust
if self.has_quirk(Quirk::NewPlatformIssue) {
    // Apply workaround
}
```

### 5. Update Server Startup Logging

In `server/mod.rs`, add to quirk match:

```rust
crate::compositor::Quirk::NewPlatformIssue => {
    info!("Applying new platform workaround");
}
```

### 6. Update This Documentation

Add the new platform to the support matrix tables above.

---

## Testing Platforms

When testing on a new platform:

1. **Check logs for detection:**
   ```
   INFO Detected compositor: GNOME 46.0
   INFO Detected OS: fedora 40
   INFO Applying quirk: ...
   ```

2. **Verify service registry:**
   ```
   INFO Service registry:
   INFO   VideoCodec: Full
   INFO   Clipboard: Full
   INFO   Input: Full
   ```

3. **Test each service:**
   - Video: Check for artifacts, especially text
   - Clipboard: Test copy/paste both directions
   - Input: Test mouse accuracy and keyboard

4. **Log quirk application:**
   ```
   WARN EGFX: Client supports AVC444 but platform has Avc444Unreliable quirk
   INFO EGFX: AVC420 encoding enabled (AVC444 disabled due to platform quirk)
   ```

---

## Troubleshooting

### Video Quality Issues

1. Check which codec is active in logs:
   ```
   INFO EGFX: AVC420 encoding enabled
   ```

2. Verify quirk detection:
   ```
   INFO RHEL 9 detected (9.4) - applying platform quirks
   ```

3. Check client capability negotiation:
   ```
   INFO EGFX: Client advertised 5 capability sets
   DEBUG EGFX capability: V10_7 { flags: SMALL_CACHE }
   ```

### Clipboard Not Working

1. Check Portal version:
   ```
   INFO Portal capabilities: version=1, clipboard=false
   ```

2. Verify quirk was applied:
   ```
   INFO Clipboard sync unavailable (Portal v1 limitation)
   ```

### Input Not Working

1. Check Portal RemoteDesktop support:
   ```
   INFO Portal RemoteDesktop: available=true
   ```

2. Verify libei connection:
   ```
   INFO Input: libei connection established
   ```

---

## Version History

| Date | Change |
|------|--------|
| 2026-01-14 | Added Avc444Unreliable and ClipboardUnavailable quirks |
| 2026-01-14 | Added OS detection via /etc/os-release |
| 2026-01-14 | Initial support matrix documentation |
