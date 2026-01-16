# lamco-rdp-server Feature Roadmap

**Product**: lamco-rdp-server - Wayland RDP Server for Linux
**License**: Non-commercial use (honor system commercial license TBD)
**Status**: Active Development

---

## Feature Status Overview

| Category | Feature | Status | Priority |
|----------|---------|--------|----------|
| **Core** | Video streaming (RemoteFX) | ✅ Complete | - |
| **Core** | H.264/EGFX encoding | ✅ Complete | - |
| **Core** | Keyboard input | ✅ Complete | - |
| **Core** | Mouse input | ✅ Complete | - |
| **Core** | TLS 1.3 encryption | ✅ Complete | - |
| **Core** | Certificate generation | ✅ Complete | - |
| **Clipboard** | Text sync | ✅ Implemented | P0 - Rewire |
| **Clipboard** | Image sync (DIB/PNG/JPEG) | ✅ Implemented | P0 - Rewire |
| **Clipboard** | File transfer | ✅ Implemented | P0 - Rewire |
| **Clipboard** | Loop detection | ✅ Implemented | P0 - Rewire |
| **Clipboard** | GNOME D-Bus fallback | ✅ Implemented | P0 - Rewire |
| **Display** | Single monitor | ✅ Complete | - |
| **Display** | Multi-monitor layout | ✅ Complete | - |
| **Display** | Server-side resolution | ✅ Complete | - |
| **Display** | Client-initiated resize | ⏸️ Deferred | P3 |
| **Auth** | No authentication | ✅ Complete | - |
| **Auth** | PAM authentication | ✅ Complete | - |
| **Auth** | Certificate auth | 🟡 Partial | P2 |
| **Audio** | Playback (RDPSND) | ❌ Not started | P2 |
| **Audio** | Microphone input | ❌ Not started | P3 |
| **Redirection** | Drive/USB (RDPDR) | ❌ Not started | P3 |
| **Redirection** | Printer | ❌ Not started | P4 |
| **Redirection** | Smart card | ❌ Not started | P4 |

---

## Phase 1: Foundation (Current Sprint)

### P0: Clipboard Rewiring
**Goal**: Replace 5,700 LOC with ~600 LOC using published crates

| Task | From | To |
|------|------|-----|
| Format conversion | `clipboard/formats.rs` (980 LOC) | `lamco-clipboard-core::formats` |
| Loop detection | `clipboard/sync.rs` (818 LOC) | `lamco-clipboard-core::loop_detector` |
| Transfer engine | `clipboard/transfer.rs` (608 LOC) | `lamco-clipboard-core::transfer` |
| D-Bus bridge | `clipboard/dbus_bridge.rs` (346 LOC) | `lamco-portal::dbus_clipboard` |
| IronRDP backend | `clipboard/ironrdp_backend.rs` (435 LOC) | `lamco-rdp-clipboard` |
| Error types | `clipboard/error.rs` (446 LOC) | `lamco-clipboard-core::error` + extend |
| Manager | `clipboard/manager.rs` (1,954 LOC) | **Keep** - thin orchestration glue |

**Result**: Clean separation between library code and server glue

### P0: Verify Core Pipeline
Ensure existing implementations work correctly:
- [ ] Video: Portal → PipeWire → Display Handler → EGFX → Client
- [ ] Input: Client → IronRDP → Input Handler → Portal → Compositor
- [ ] Clipboard: Full bidirectional sync testing

---

## Phase 2: Enhanced Display

### Resolution Terminology (Definitions)

| Term | Definition | Status |
|------|------------|--------|
| **Server-side resolution** | Resolution of the Wayland desktop being captured | ✅ Working |
| **Multi-monitor layout** | Multiple displays with different positions/sizes | ✅ Working |
| **Mixed-resolution multimon** | Different monitors at different resolutions (e.g., 1080p + 4K) | ✅ Working |
| **Client-initiated resize** | RDP client requests a different resolution than server has | ⏸️ Deferred |
| **Dynamic resize** | Server changes resolution mid-session | ⏸️ Deferred |

### Current Resolution Behavior (✅ Complete)

**How it works:**
1. Server starts a Portal session with the Wayland compositor
2. Portal returns stream info with the desktop's native resolution
3. RDP client receives this resolution and must accept it
4. If client window differs, client handles scaling locally

**What's supported:**
- ✅ Any server-side resolution (whatever the compositor provides)
- ✅ Different resolutions per monitor in multi-monitor setups
- ✅ High DPI displays (4K, 5K, etc.)
- ✅ Client-side scaling when window size differs from server resolution

**Configuration:** Resolution is determined by the Wayland compositor, not by config file.
For headless VDI, configure resolution in the compositor (e.g., Weston virtual output).

### Multi-Monitor Support (✅ Complete)

Full layout code exists in `src/multimon/` with:
- ✅ Monitor discovery from Portal StreamInfo
- ✅ Layout calculation (horizontal, vertical, grid, preserve-positions)
- ✅ Coordinate transformation between RDP and monitor-local space
- ✅ Mixed resolutions across monitors

**Remaining work (future):**
- [ ] Monitor hotplug events (add/remove during session)
- [ ] Per-monitor DPI reporting to client

### P3: Client-Initiated Resize (Deferred)

This would allow RDP clients to request a specific resolution.

**Why deferred:**
- Requires compositor cooperation to change resolution
- GNOME/KDE sessions can't easily change resolution programmatically
- Headless compositors (Weston) could support it but adds complexity
- Current behavior (server dictates, client scales) works for most use cases

**DISPLAYCONTROL channel:** The protocol handler exists (`display_handler.rs:1286`) but
currently logs and ignores client resize requests. Clients handle this gracefully by
scaling locally.

---

## Phase 3: Authentication & Security

### P2: Enhanced Authentication
Currently supports PAM and no-auth.

**Tasks**:
- [ ] NLA (Network Level Authentication) support
- [ ] Certificate-based client authentication
- [ ] TOTP/2FA integration (via PAM)
- [ ] Session recording/audit logging

---

## Phase 4: Media Channels

### P2: Audio Playback (RDPSND)
Play desktop audio on RDP client.

**Architecture**:
```
PipeWire Audio Capture → Opus/AAC Encoding → RDPSND Channel → Client
```

**Tasks**:
- [ ] PipeWire audio source capture
- [ ] Audio encoder (Opus preferred, AAC fallback)
- [ ] RDPSND channel implementation
- [ ] Volume synchronization
- [ ] Latency optimization

### P3: Microphone Input
Capture client microphone for Linux apps.

**Architecture**:
```
Client Mic → RDPSND/AUDIN Channel → Decoder → PipeWire Sink → Apps
```

---

## Phase 5: Device Redirection

### P3: Drive Redirection (RDPDR)
Access client drives from Linux session.

**Tasks**:
- [ ] RDPDR channel implementation
- [ ] Virtual filesystem mount (FUSE)
- [ ] File transfer optimization
- [ ] Permission handling

### P4: Printer Redirection
Print to client-local printers.

### P4: Smart Card Redirection
Use client smart cards for Linux authentication.

---

## Architecture After Rewiring

```
lamco-rdp-server (Product)
├── Thin glue code (~3,000 LOC total)
│   ├── clipboard/     (~600 LOC - orchestration only)
│   ├── server/        (~2,400 LOC - main server)
│   ├── egfx/          (~1,800 LOC - H.264 encoding)
│   ├── config/        (~500 LOC)
│   ├── security/      (~600 LOC)
│   └── multimon/      (~900 LOC)
│
├── Published Crates (reused)
│   ├── lamco-portal           # Portal integration
│   ├── lamco-pipewire         # Video capture
│   ├── lamco-video            # Frame processing
│   ├── lamco-rdp-input        # Input translation
│   ├── lamco-clipboard-core   # Clipboard primitives
│   └── lamco-rdp-clipboard    # RDP clipboard bridge
│
└── IronRDP (upstream)
    ├── ironrdp-server         # RDP server framework
    ├── ironrdp-cliprdr        # Clipboard channel
    ├── ironrdp-displaycontrol # Display control
    └── ironrdp-*              # Other channels
```

---

## Configuration

```toml
# /etc/lamco-rdp-server/config.toml

[server]
listen = "0.0.0.0"
port = 3389
max_connections = 10

[display]
cursor_mode = "embedded"     # embedded | hidden | metadata
framerate_limit = 60
quality = "balanced"         # quality | balanced | performance

[video]
codec = "h264"               # h264 | remotefx
hardware_accel = true

[clipboard]
enabled = true
max_size_mb = 16
enable_files = true
enable_images = true

[audio]
enabled = false              # Phase 4
playback = true
recording = false

[auth]
method = "pam"               # none | pam | certificate
pam_service = "login"

[security]
tls_cert = "/etc/lamco-rdp-server/cert.pem"
tls_key = "/etc/lamco-rdp-server/key.pem"
min_tls_version = "1.3"
```

---

## Success Metrics

| Metric | Target | Current |
|--------|--------|---------|
| End-to-end latency | <100ms | TBD |
| Video framerate | 30-60 FPS | TBD |
| Clipboard sync time | <500ms | TBD |
| Connection setup | <3s | TBD |
| Memory usage | <200MB | TBD |
| CPU usage (idle) | <5% | TBD |

---

## Development Priorities

1. **P0 (This Sprint)**: Clipboard rewiring + core verification
2. **P1 (Next)**: Multi-monitor support
3. **P2 (Following)**: Dynamic resize, audio playback, enhanced auth
4. **P3 (Future)**: Microphone, drive redirection
5. **P4 (Backlog)**: Printer, smart card

---

## Testing Strategy

### Unit Tests
- Each module has unit tests
- Mocked IronRDP/Portal interfaces

### Integration Tests
- Full pipeline tests with real Portal
- Requires Wayland session

### End-to-End Tests
- Windows RDP client → Linux desktop
- macOS RDP client → Linux desktop
- FreeRDP → Linux desktop

### Performance Tests
- Latency benchmarks
- Throughput benchmarks
- Memory leak detection
