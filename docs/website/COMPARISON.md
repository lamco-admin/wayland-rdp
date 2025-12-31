# Feature Comparison: Linux Remote Desktop Solutions

**URL:** `https://lamco.ai/products/lamco-rdp-server/comparison/` or `/comparison/`
**Status:** Draft for review

---

## Overview

Choosing a remote desktop solution for Linux involves tradeoffs. This page provides an honest comparison of lamco-rdp-server against alternatives, helping you decide which solution fits your needs.

---

## Quick Comparison

| Feature | lamco-rdp-server | xrdp | gnome-remote-desktop | VNC (TigerVNC) | NoMachine |
|---------|------------------|------|----------------------|----------------|-----------|
| **Wayland Native** | ✓ | ✗ | ✓ | ✗ | ✗ |
| **X11 Support** | Via Xwayland | ✓ | ✗ | ✓ | ✓ |
| **Protocol** | RDP | RDP | RDP | VNC | NX |
| **H.264 Encoding** | ✓ | ✓ | ✓ | ✗ | ✓ |
| **AVC444 (4:4:4)** | ✓ | ✗ | ✗ | N/A | Proprietary |
| **Hardware Encode** | NVENC, VA-API | Limited | No | No | Yes |
| **Open Source** | BSL→Apache | GPL | GPL | GPL | No |
| **Price** | Free/$49+ | Free | Free | Free | Free/$50+ |

---

## Detailed Comparisons

### vs xrdp

**xrdp** is the most widely deployed open-source RDP server for Linux.

| Aspect | lamco-rdp-server | xrdp |
|--------|------------------|------|
| **Wayland Support** | Native (XDG Portals) | None (X11 only) |
| **Capture Method** | PipeWire | X11 or Xvnc backend |
| **Text Quality** | AVC444 (excellent) | AVC420 only (fuzzy text) |
| **Hardware Encoding** | NVENC + VA-API | VA-API (limited) |
| **Color Management** | Full VUI signaling | Basic |
| **Codebase** | Rust | C |
| **Maturity** | New | Very mature |
| **Documentation** | Growing | Extensive |

**Choose lamco-rdp-server if:**
- You run Wayland (GNOME, KDE Plasma, Sway)
- Text clarity matters (coding, documents)
- You want hardware encoding with NVIDIA GPUs

**Choose xrdp if:**
- You run X11 desktops
- You need maximum compatibility with existing infrastructure
- Maturity and extensive documentation are priorities

---

### vs gnome-remote-desktop

**gnome-remote-desktop** is GNOME's built-in RDP server.

| Aspect | lamco-rdp-server | gnome-remote-desktop |
|--------|------------------|----------------------|
| **Desktop Support** | Any Wayland compositor | GNOME only |
| **Installation** | Separate | Pre-installed on GNOME |
| **AVC444** | ✓ | ✗ |
| **Hardware Encoding** | NVENC + VA-API | No |
| **Adaptive FPS** | ✓ (5-60 FPS) | No |
| **Configuration** | Extensive | Limited |
| **Predictive Cursor** | ✓ | No |
| **License** | BSL | GPL |

**Choose lamco-rdp-server if:**
- You use KDE, Sway, Hyprland, or other non-GNOME compositors
- You want AVC444 text clarity
- You need hardware acceleration
- You want performance tuning options

**Choose gnome-remote-desktop if:**
- You use GNOME and want zero setup
- Basic remote access is sufficient
- You prefer fully GPL software

---

### vs VNC (TigerVNC, TightVNC, etc.)

**VNC** uses a different protocol than RDP, with different tradeoffs.

| Aspect | lamco-rdp-server | VNC |
|--------|------------------|-----|
| **Protocol** | RDP (H.264 video) | VNC (raw/JPEG frames) |
| **Compression** | Excellent (H.264) | Good (JPEG) to Poor (raw) |
| **Bandwidth** | Low | Higher |
| **Client Availability** | Excellent (built into Windows) | Requires VNC client |
| **Wayland** | Native | Requires X11 |
| **Security** | TLS 1.3 built-in | Varies (often needs SSH tunnel) |
| **Sound** | Planned (RDPSND) | Separate solution needed |

**Choose lamco-rdp-server if:**
- Bandwidth efficiency matters
- You want to use built-in Windows RDP client
- You run Wayland

**Choose VNC if:**
- You need cross-platform server (VNC servers exist for everything)
- You prefer VNC's simpler protocol
- You're in an environment where VNC is standard

---

### vs NoMachine

**NoMachine** is a commercial remote desktop solution with a free tier.

| Aspect | lamco-rdp-server | NoMachine |
|--------|------------------|-----------|
| **Protocol** | RDP (standard) | NX (proprietary) |
| **Client** | Any RDP client | NoMachine client required |
| **Open Source** | BSL→Apache | Proprietary |
| **Wayland** | Native | X11 capture |
| **Quality** | AVC444 available | Proprietary codec |
| **Pricing** | Free/$49+ BSL | Free tier, $50+ commercial |
| **Cross-Platform** | Linux server only | Linux, Windows, macOS |

**Choose lamco-rdp-server if:**
- You want standard RDP protocol
- You prefer open-source foundations
- Native Wayland matters
- You already use RDP clients

**Choose NoMachine if:**
- You need Windows/macOS servers too
- You're willing to use their client
- You want a single vendor solution
- Audio and USB redirection are critical now

---

### vs Commercial VDI Solutions

**Comparison with VMware Horizon, Citrix, AWS WorkSpaces:**

| Aspect | lamco-rdp-server | Enterprise VDI |
|--------|------------------|----------------|
| **Target** | Individual Linux desktops | Large-scale deployments |
| **Management** | Per-machine | Centralized orchestration |
| **Pricing** | Simple per-server | Per-user, complex licensing |
| **Features** | Remote desktop | Full VDI ecosystem |
| **Setup** | Install and run | Significant infrastructure |

**lamco-rdp-server is not a VDI solution.** It's a remote desktop server for individual Linux machines. If you need enterprise VDI with central management, user provisioning, and hundreds of virtual desktops, enterprise solutions are more appropriate.

However, lamco-rdp-server can be a component of custom VDI solutions or provide remote access to development machines in enterprise environments.

---

## Feature Deep Dives

### Text Clarity (AVC444 vs AVC420)

This is lamco-rdp-server's most significant advantage for knowledge workers.

```
AVC420 (xrdp, gnome-remote-desktop):
┌─────────────────────────────────────┐
│  def hello_world():                 │  ← Slight color fringing
│      print("Hello, World!")         │    around text edges
└─────────────────────────────────────┘

AVC444 (lamco-rdp-server):
┌─────────────────────────────────────┐
│  def hello_world():                 │  ← Sharp, clean text
│      print("Hello, World!")         │    identical to local
└─────────────────────────────────────┘
```

For coding, document editing, or design work—anything with sharp edges and colored text—AVC444 makes a visible difference.

### Hardware Encoding Coverage

| Solution | NVENC | VA-API (Intel) | VA-API (AMD) |
|----------|-------|----------------|--------------|
| lamco-rdp-server | Full | Full | Full |
| xrdp | No | Partial | Partial |
| gnome-remote-desktop | No | No | No |
| NoMachine | Yes | Unknown | Unknown |

Hardware encoding is essential for maintaining low CPU usage while streaming high-resolution content.

### Color Accuracy

| Solution | Color Matrix | Range Signaling | VUI Metadata |
|----------|--------------|-----------------|--------------|
| lamco-rdp-server | BT.709/BT.601/sRGB | Full + Limited | Complete |
| xrdp | Fixed | Limited only | Partial |
| gnome-remote-desktop | Unknown | Unknown | Unknown |

For design work or any color-sensitive application, proper color management prevents the washed-out or tinted colors that plague many remote desktop solutions.

---

## Migration Guide

### From xrdp

If you're currently using xrdp on X11:

1. **Switch to Wayland** (if not already)
   - Ubuntu: Usually default, or select "Ubuntu on Wayland" at login
   - Fedora: Default
   - Other: Consult distribution documentation

2. **Install lamco-rdp-server**
   ```bash
   flatpak install flathub ai.lamco.rdp-server
   ```

3. **Configure**
   - Most xrdp users can start with defaults
   - Copy any custom authentication settings if needed

4. **Update firewall** (if different port)
   - Default RDP port is 3389 (same as xrdp)

5. **Test**
   - Connect with same RDP client you used for xrdp

### From VNC

1. **Install lamco-rdp-server**

2. **Switch RDP client**
   - Windows: Use built-in mstsc.exe (Remote Desktop Connection)
   - macOS: Microsoft Remote Desktop (free from App Store)
   - Linux: FreeRDP or Remmina

3. **Update connection settings**
   - RDP uses port 3389 (vs VNC's 5900+)
   - Authentication may differ

---

## When NOT to Use lamco-rdp-server

Be honest about limitations:

| Scenario | Better Alternative |
|----------|-------------------|
| X11 desktop (not Wayland) | xrdp |
| Windows or macOS server | Windows RDP, NoMachine |
| Enterprise VDI deployment | VMware Horizon, Citrix |
| Need audio right now | NoMachine (we're adding it) |
| Maximum maturity/stability | xrdp (more years of testing) |

---

## Summary

**lamco-rdp-server is the best choice when:**
- ✓ You run a Wayland desktop (GNOME, KDE, Sway, Hyprland)
- ✓ Text clarity matters for your work
- ✓ You want hardware encoding (especially NVIDIA)
- ✓ You prefer standard RDP protocol
- ✓ You value modern, memory-safe code

**Consider alternatives when:**
- You run X11 (xrdp is mature and works well)
- You need features we haven't implemented yet (audio, USB)
- You need cross-platform server support (NoMachine)
- You need enterprise VDI management

---

## Try It

The best way to compare is to try lamco-rdp-server yourself. It's free for personal use.

[Download →](/download/) | [View Pricing →](/pricing/)

Questions? Contact us at office@lamco.io
