# lamco.ai Website Content Outline

**Generated:** 2025-12-30
**Purpose:** Content structure for lamco.ai product and technology pages

---

## Site Structure

```
lamco.ai/
├── Home (landing)
├── Products/
│   ├── lamco-rdp-server (main product page)
│   └── lamco-vdi (future)
├── Technology/
│   ├── Video Encoding
│   ├── Color Management
│   ├── Performance Features
│   └── Wayland Integration
├── Docs/
│   ├── Getting Started
│   ├── Configuration Reference
│   ├── Hardware Encoding Setup
│   └── Troubleshooting
├── Open Source/
│   └── lamco-* crates
└── Pricing
```

---

## Product Page: lamco-rdp-server

### Hero Section

**Headline:** Wayland RDP Server for Linux

**Subheadline:** Professional remote desktop access with hardware-accelerated H.264 video encoding and premium performance features.

**Key Points:**
- Native Wayland support via XDG Desktop Portals
- AVC420 and AVC444 (4:4:4 chroma) video codecs
- NVIDIA NVENC and Intel/AMD VA-API hardware encoding
- Adaptive frame rate and latency optimization

### Features Section

#### Video Quality

| Feature | Description |
|---------|-------------|
| AVC444 | Full 4:4:4 chroma for crystal-clear text and UI |
| Hardware Encoding | NVENC (NVIDIA) and VA-API (Intel/AMD) |
| Adaptive FPS | 5-60 FPS based on screen activity |
| Color Accuracy | BT.709/BT.601 with VUI signaling |

#### Performance

| Feature | Description |
|---------|-------------|
| Damage Tracking | SIMD-optimized, 90%+ bandwidth savings |
| Latency Governor | Interactive, Balanced, Quality modes |
| Predictive Cursor | Physics-based latency compensation |
| 60fps Support | High-performance mode for powerful systems |

#### Integration

| Feature | Description |
|---------|-------------|
| Wayland Native | XDG Desktop Portals (no X11 dependency) |
| Clipboard Sync | Text, images, files with loop detection |
| Multi-Monitor | Layout calculation and coordinate mapping |
| Compositor Support | GNOME, KDE, Sway, Hyprland auto-detection |

### Technical Specifications Table

| Specification | Details |
|---------------|---------|
| **Video Codecs** | AVC420, AVC444 |
| **Encoders** | OpenH264, NVENC, VA-API |
| **Color Spaces** | BT.709, BT.601, sRGB (full/limited range) |
| **Frame Rate** | 5-60 FPS (adaptive) |
| **H.264 Levels** | 3.0-5.2 (auto-selected) |
| **Max Resolution** | 4K UHD (level dependent) |
| **Clipboard** | Text, images (DIB/PNG/JPEG), files |
| **Authentication** | None, PAM |
| **Encryption** | TLS 1.3 |
| **Platforms** | Linux (Wayland compositors) |

### Requirements Section

**System Requirements:**
- Linux with Wayland compositor (GNOME, KDE Plasma, Sway, Hyprland)
- XDG Desktop Portal support
- PipeWire for screen capture

**For Hardware Encoding:**
- NVIDIA: GPU with NVENC, libnvidia-encode
- Intel/AMD: VA-API support, libva

**Compatible RDP Clients:**
- Windows: Built-in Remote Desktop, FreeRDP
- macOS: Microsoft Remote Desktop, FreeRDP
- Linux: FreeRDP, Remmina

---

## Technology Pages

### Video Encoding Technology

**Page: /technology/video-encoding**

**Content:**
1. H.264/AVC Overview
2. AVC420 vs AVC444 Comparison
3. Hardware Acceleration (NVENC, VA-API)
4. Quality Parameters (bitrate, QP, levels)
5. Performance Optimization (damage tracking, adaptive FPS)

**Source docs:**
- docs/VIDEO-CODEC-REFERENCE.md
- docs/architecture/COLOR-ARCHITECTURE-EXECUTIVE-SUMMARY.md

### Color Management Technology

**Page: /technology/color-management**

**Content:**
1. Why Color Matters in Remote Desktop
2. Color Space Standards (BT.709, BT.601, sRGB)
3. Full vs Limited Range
4. VUI Signaling
5. SIMD Color Conversion

**Source docs:**
- docs/architecture/COLOR-ARCHITECTURE-EXECUTIVE-SUMMARY.md

### Performance Features

**Page: /technology/performance**

**Content:**
1. Adaptive Frame Rate
2. Damage Tracking (SIMD)
3. Latency Governor Modes
4. Predictive Cursor
5. AVC444 Auxiliary Omission

**Source docs:**
- src/config/types.rs (config structs)
- src/lib.rs (module docs)

### Wayland Integration

**Page: /technology/wayland**

**Content:**
1. XDG Desktop Portal Architecture
2. PipeWire Screen Capture
3. Compositor Compatibility
4. Input Injection (libei)
5. Clipboard via Portal

**Source docs:**
- docs/strategy/STRATEGIC-FRAMEWORK.md

---

## Documentation Pages

### Getting Started

**Page: /docs/getting-started**

**Content:**
1. Installation (package managers, build from source)
2. First connection
3. Basic configuration
4. Troubleshooting common issues

**Source:** Create new from INSTALL.md + CONFIGURATION.md

### Configuration Reference

**Page: /docs/configuration**

**Content:**
1. config.toml structure
2. All configuration options (from types.rs)
3. Example configurations
4. Environment variables

**Source:**
- src/config/types.rs
- CONFIGURATION.md

### Hardware Encoding Setup

**Page: /docs/hardware-encoding**

**Content:**
1. NVENC setup (NVIDIA)
2. VA-API setup (Intel/AMD)
3. Verification and testing
4. Performance tuning

**Source:**
- docs/HARDWARE-ENCODING-BUILD-GUIDE.md
- docs/HARDWARE-ENCODING-QUICKREF.md

---

## Open Source Page

**Page: /open-source**

### Published Crates

| Crate | Description | crates.io |
|-------|-------------|-----------|
| lamco-portal | XDG Desktop Portal integration | link |
| lamco-pipewire | PipeWire screen capture | link |
| lamco-video | Video frame processing | link |
| lamco-rdp-input | RDP input event translation | link |
| lamco-clipboard-core | Clipboard primitives | link |
| lamco-rdp-clipboard | RDP clipboard bridge | link |

### IronRDP Contributions

- PR #1053: Clipboard fix (merged)
- PR #1057: EGFX/H.264 support (open)
- PRs #1063-1066: Clipboard file transfer (open)
- Issue #1067: ZGFX compression (ready)

---

## Content Gaps to Fill

1. **Installation guide** - Package-based install, build from source
2. **Quickstart tutorial** - First connection step-by-step
3. **Troubleshooting guide** - Common issues and solutions
4. **Performance tuning guide** - Optimizing for different scenarios
5. **Multi-monitor guide** - Setup and configuration
6. **Comparison page** - vs other Linux RDP solutions

---

## SEO Keywords

- Wayland RDP server
- Linux remote desktop
- H.264 RDP encoding
- NVENC RDP
- VA-API RDP
- AVC444 remote desktop
- Linux desktop sharing
- PipeWire screen capture
- XDG portal RDP
