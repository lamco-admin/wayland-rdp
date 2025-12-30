# lamco-rdp-server Codebase Reality Check

**Generated:** 2025-12-30
**Purpose:** Accurate inventory of implemented features for website content and documentation

---

## Product Identity

**Product:** lamco-rdp-server
**Description:** Wayland RDP Server for Linux - Portal mode for desktop sharing
**License:** Non-commercial (honor system commercial license TBD)

---

## Module Structure

```
src/
  lib.rs              # Main library, re-exports lamco-* crates
  main.rs             # CLI entry point

  # Core Modules
  config/             # Configuration (types.rs: 800+ lines of config structs)
  server/             # RDP server implementation
  security/           # TLS and authentication

  # Video Pipeline
  egfx/               # EGFX/H.264 encoding (8 files, ~230KB)
    encoder.rs        # AVC420 encoder
    avc444_encoder.rs # AVC444 encoder (dual-stream)
    color_space.rs    # Color space config and VUI
    color_convert.rs  # BGRA->YUV (SIMD: AVX2, NEON)
    h264_level.rs     # H.264 level management
    yuv444_packing.rs # YUV444 packing
    hardware/         # Hardware encoders
      nvenc/          # NVIDIA NVENC
      vaapi/          # Intel/AMD VA-API
  damage/             # Tile-based damage detection (SIMD)

  # Display
  multimon/           # Multi-monitor layout calculation

  # Input/Clipboard
  clipboard/          # Clipboard orchestration

  # Premium Features
  performance/        # Adaptive FPS, Latency Governor
  cursor/             # Predictive cursor
  services/           # Service Advertisement Registry
  compositor/         # Compositor capability probing
```

---

## Feature Implementation Status

### Core Features (All Complete)

| Feature | Status | Implementation |
|---------|--------|----------------|
| Video streaming | COMPLETE | Portal -> PipeWire -> Display Handler -> IronRDP |
| H.264/AVC420 | COMPLETE | OpenH264 software encoder with VUI support |
| H.264/AVC444 | COMPLETE | Dual-stream 4:4:4 chroma, aux omission |
| Keyboard input | COMPLETE | lamco-rdp-input crate |
| Mouse input | COMPLETE | lamco-rdp-input crate |
| TLS 1.3 | COMPLETE | rustls-based |
| Certificate gen | COMPLETE | Auto-generate self-signed |

### Video Encoding (Extensive)

| Codec | Status | Details |
|-------|--------|---------|
| AVC420 (4:2:0) | COMPLETE | Standard H.264, good for video |
| AVC444 (4:4:4) | COMPLETE | Superior text/UI, dual-stream |
| Aux omission | COMPLETE | Bandwidth optimization (0.81 MB/s proven) |
| Color space config | COMPLETE | BT.709/BT.601/sRGB, full/limited range |
| VUI signaling | COMPLETE | OpenH264 fork, NVENC native |
| H.264 levels | COMPLETE | Auto-select 3.0-5.2 based on resolution |

### Hardware Encoding

| Backend | Status | Details |
|---------|--------|---------|
| NVENC (NVIDIA) | IMPLEMENTED | Full VUI support, color space config |
| VA-API (Intel/AMD) | IMPLEMENTED | Color conversion works, VUI limited by API |
| Software fallback | COMPLETE | OpenH264 always available |

### Premium/Advanced Features

| Feature | Status | Details |
|---------|--------|---------|
| Adaptive FPS | COMPLETE | 5-60 FPS based on activity |
| Latency Governor | COMPLETE | Interactive/Balanced/Quality modes |
| Damage tracking | COMPLETE | SIMD tile-based, 90%+ bandwidth savings |
| Predictive cursor | COMPLETE | Physics-based latency compensation |
| Service Registry | COMPLETE | Wayland capability -> RDP translation |
| Compositor probing | COMPLETE | GNOME/KDE/Sway/Hyprland detection |

### Clipboard

| Feature | Status | Details |
|---------|--------|---------|
| Text sync | COMPLETE | Bidirectional |
| Image sync | COMPLETE | DIB/PNG/JPEG formats |
| File transfer | IMPLEMENTED | Via lamco-rdp-clipboard |
| Loop detection | COMPLETE | Via lamco-clipboard-core |
| GNOME D-Bus | COMPLETE | Fallback for portal issues |

### Display Features

| Feature | Status | Details |
|---------|--------|---------|
| Single monitor | COMPLETE | Default mode |
| Multi-monitor | PARTIAL | Layout code exists in multimon/, needs testing |
| Dynamic resize | PARTIAL | DISPLAYCONTROL handler exists, needs testing |
| DPI scaling | PARTIAL | Config exists, implementation TBD |

### Authentication

| Method | Status | Details |
|--------|--------|---------|
| No auth | COMPLETE | Development mode |
| PAM | COMPLETE | System auth integration |
| NLA | PARTIAL | Config exists, implementation TBD |
| Certificate | NOT STARTED | Planned |

### Not Started

| Feature | Priority | Notes |
|---------|----------|-------|
| Audio playback (RDPSND) | P2 | Architecture planned |
| Microphone input | P3 | - |
| Drive redirection (RDPDR) | P3 | - |
| Printer redirection | P4 | - |
| Smart card | P4 | - |

---

## Configuration Capabilities

The config system (types.rs) supports extensive tuning:

### Server
- Listen address, max connections, session timeout
- Portal mode toggle

### Video/EGFX
- Codec selection (auto/avc420/avc444)
- Bitrate, QP range, H.264 level
- Color matrix (auto/bt709/bt601/srgb)
- Color range (limited/full)
- AVC444 aux omission parameters
- ZGFX compression mode

### Hardware Encoding
- Enable/disable, device path
- NVENC preference over VA-API
- Quality preset, zero-copy DMA-BUF
- Software fallback

### Performance
- Adaptive FPS (min/max, activity thresholds)
- Latency mode (interactive/balanced/quality)
- Thread counts, buffer sizes

### Cursor
- Mode (metadata/painted/hidden/predictive)
- Auto-mode based on latency
- Predictor physics parameters

### Damage Tracking
- Tile size, diff threshold
- Region merging

---

## Dependencies (Published Crates)

| Crate | Purpose |
|-------|---------|
| lamco-portal | XDG Desktop Portal integration |
| lamco-pipewire | PipeWire screen capture |
| lamco-video | Frame processing |
| lamco-rdp-input | Input event translation |
| lamco-clipboard-core | Clipboard primitives |
| lamco-rdp-clipboard | RDP clipboard bridge |

---

## IronRDP Integration

Uses upstream IronRDP with these crates:
- ironrdp-server (core RDP server)
- ironrdp-cliprdr (clipboard channel)
- ironrdp-displaycontrol (display control)
- ironrdp-graphics (ZGFX decompressor, we contributed compressor)

---

## What's Production Ready

1. **Video pipeline** - AVC420/AVC444 encoding working
2. **Input handling** - Keyboard/mouse fully functional
3. **Basic clipboard** - Text and images sync
4. **TLS security** - 1.3 encryption standard
5. **Software encoding** - OpenH264 reliable
6. **Performance features** - Adaptive FPS, damage tracking

## What Needs Testing/Verification

1. **Hardware encoding** - NVENC/VA-API paths need real-world validation
2. **Multi-monitor** - Code exists but not extensively tested
3. **Dynamic resize** - Handler exists, needs client testing
4. **File clipboard** - Implemented but needs validation
5. **Predictive cursor** - Implemented, needs latency testing

## Documentation Gaps

1. **No API reference** - Config options documented in code only
2. **No deployment guide** - Installation/packaging incomplete
3. **No performance tuning guide** - Many knobs, no documentation
4. **No troubleshooting guide** - Common issues not documented
