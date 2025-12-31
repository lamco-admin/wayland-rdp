# lamco-rdp-server Product Page

**URL:** `https://lamco.ai/products/lamco-rdp-server/`
**Status:** Draft for review

---

## Hero Section

### Headline
Wayland RDP Server for Linux

### Subheadline
Professional remote desktop access with hardware-accelerated H.264 encoding, crystal-clear text rendering, and premium performance features. Built native to Wayland—no X11 compromises.

### Hero CTAs
- **Primary:** Download Free →
- **Secondary:** View Pricing →

---

## Introduction

Remote desktop on Linux has always meant compromise. X11-based solutions don't work properly with modern Wayland desktops. VNC lacks the efficiency of H.264 encoding. Built-in options are limited to specific desktop environments.

lamco-rdp-server changes that.

Built from the ground up for Wayland using XDG Desktop Portals, it delivers professional remote desktop capabilities to any modern Linux desktop. Connect from Windows, macOS, Linux, or mobile using standard RDP clients you already have.

---

## Key Benefits

### Wayland Native
The first RDP server built specifically for XDG Desktop Portals. Works with GNOME, KDE Plasma, Sway, Hyprland, and other Wayland compositors without requiring Xwayland or X11 fallbacks. Your Wayland security model stays intact.

### Crystal-Clear Text
AVC444 encoding delivers full 4:4:4 chroma resolution—the same quality used by professional video workflows. Text, code, and UI elements render sharp and readable, not muddy like standard 4:2:0 video. Perfect for programming, document editing, and design work.

### Hardware Accelerated
Offload H.264 encoding to your GPU. Support for NVIDIA NVENC and Intel/AMD VA-API means your CPU stays free for actual work. Automatic fallback to optimized software encoding when hardware isn't available.

### Premium Performance
Adaptive frame rate adjusts from 5 to 60 FPS based on screen activity—full speed for video, power-saving for static content. Latency governor optimizes for interactive work or visual quality. Predictive cursor technology compensates for network delay.

---

## Features

### Video Encoding

| Feature | Details |
|---------|---------|
| **Codecs** | AVC420 (standard), AVC444 (premium text clarity) |
| **Encoders** | OpenH264 (software), NVENC (NVIDIA), VA-API (Intel/AMD) |
| **Frame Rate** | 5-60 FPS adaptive based on activity |
| **H.264 Levels** | 3.0-5.2 (auto-selected by resolution) |
| **Max Resolution** | 4K UHD (level dependent) |
| **Color Spaces** | BT.709, BT.601, sRGB (full and limited range) |
| **Color Metadata** | Full VUI signaling for accurate color reproduction |

### Input & Clipboard

| Feature | Details |
|---------|---------|
| **Keyboard** | Full scancode translation, all layouts |
| **Mouse** | Absolute and relative positioning, all buttons |
| **Multi-Monitor** | Coordinate mapping across displays |
| **Text Clipboard** | Bidirectional sync with loop prevention |
| **Image Clipboard** | PNG, JPEG, DIB format support |
| **File Clipboard** | Drag-and-drop file transfer |

### Security

| Feature | Details |
|---------|---------|
| **Encryption** | TLS 1.3 |
| **Authentication** | None (development), PAM (system auth) |
| **Certificates** | Auto-generated self-signed or custom |

### Compositor Support

Tested and optimized for:
- GNOME (Ubuntu, Fedora default)
- KDE Plasma
- Sway
- Hyprland

**Integrated capability discovery:** Our Service Advertisement Registry probes your system at startup—compositor, portal versions, hardware encoders, GPU capabilities—and automatically selects optimal code paths. When something isn't available, you get clear diagnostics instead of cryptic failures. [Learn more about our Wayland integration →](/technology/wayland/)

---

## How It Works

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────┐
│  Your Desktop   │     │ lamco-rdp-server │     │ RDP Client  │
│    (Wayland)    │     │                  │     │             │
├─────────────────┤     ├──────────────────┤     ├─────────────┤
│                 │     │                  │     │             │
│  XDG Portal ────┼────►│  Screen Capture  │     │  Windows    │
│                 │     │        ▼         │     │  mstsc.exe  │
│  PipeWire ──────┼────►│  H.264 Encode    │────►│             │
│                 │     │        ▼         │     │  FreeRDP    │
│  Compositor ◄───┼─────│  Input Inject    │◄────│             │
│                 │     │                  │     │  macOS RD   │
└─────────────────┘     └──────────────────┘     └─────────────┘
```

1. **Screen Capture:** PipeWire streams your desktop via XDG ScreenCast portal
2. **Encoding:** Frames are encoded to H.264 (hardware or software)
3. **Transmission:** Encoded video streams to RDP client over TLS
4. **Input:** Client keyboard/mouse events are translated and injected via libei

---

## System Requirements

### Server (Linux)

**Required:**
- Linux with Wayland compositor (GNOME, KDE, Sway, Hyprland)
- PipeWire (screen capture)
- XDG Desktop Portal support

**For Hardware Encoding:**
- NVIDIA: GPU with NVENC support, nvidia-driver, libnvidia-encode
- Intel: VA-API support, intel-media-va-driver or i965-va-driver
- AMD: VA-API support, mesa-va-drivers

### Compatible RDP Clients

| Platform | Clients |
|----------|---------|
| **Windows** | Built-in Remote Desktop (mstsc.exe), FreeRDP |
| **macOS** | Microsoft Remote Desktop, FreeRDP |
| **Linux** | FreeRDP, Remmina |
| **Android** | Microsoft Remote Desktop |
| **iOS** | Microsoft Remote Desktop |

---

## Pricing

lamco-rdp-server is **free for personal use and small businesses**.

Commercial licenses required only for organizations with more than 3 employees AND more than $1M annual revenue.

| Plan | Price | Servers |
|------|-------|---------|
| Monthly | $4.99/mo | 1 |
| Annual | $49/yr | 5 |
| Perpetual | $99 | 10 |
| Corporate | $599 | 100 |
| Service Provider | $2,999 | Unlimited |

[View Full Pricing →](/pricing/)

---

## Get Started

### Download
Available as Flatpak, .deb, .rpm, or source.

[Download →](/download/)

### Documentation
Installation guides, configuration reference, and troubleshooting.

[View Docs →](/docs/lamco-rdp-server/)

### Technology
Deep dives into video encoding, color management, and performance optimization.

[Explore Technology →](/technology/)

---

## Support

**Community Support:** GitHub Issues and Discussions

**Priority Email Support:** office@lamco.io
Priority response for commercial license holders.

---

## Open Source Foundation

lamco-rdp-server is built on open source infrastructure that we publish and maintain:

| Crate | Purpose |
|-------|---------|
| [lamco-portal](https://crates.io/crates/lamco-portal) | XDG Desktop Portal integration |
| [lamco-pipewire](https://crates.io/crates/lamco-pipewire) | PipeWire screen capture |
| [lamco-video](https://crates.io/crates/lamco-video) | Video frame processing |
| [lamco-rdp-input](https://crates.io/crates/lamco-rdp-input) | Input event translation |
| [lamco-rdp-clipboard](https://crates.io/crates/lamco-rdp-clipboard) | Clipboard synchronization |

These crates are MIT/Apache-2.0 dual-licensed and available for anyone building remote desktop infrastructure.

[View All Open Source →](/open-source/)

---

## License

lamco-rdp-server is licensed under the Business Source License 1.1.

- **Free:** Personal use, non-profits, small businesses (≤3 employees), companies under $1M revenue
- **Commercial:** License required for larger organizations
- **Open Source Conversion:** Becomes Apache-2.0 on December 31, 2028

[View License Details →](/pricing/)

---

## What's Next?

We're actively developing lamco-rdp-server based on user feedback. Features under consideration:

- **Audio Playback (RDPSND)** — Stream system audio to client
- **Microphone Input** — Bidirectional audio for calls
- **Multi-Monitor Improvements** — Dynamic layout changes
- **Drive Redirection** — Access client drives from server

**Have a feature request?** We want to hear from you.

[Contact Us →](/contact/) | office@lamco.io
