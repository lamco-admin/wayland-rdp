# Roadmap & Feature Requests

**URL:** `https://lamco.ai/products/lamco-rdp-server/roadmap/` or `/roadmap/`
**Status:** Draft for review

---

## Current Status

lamco-rdp-server is in active development. The core remote desktop functionality is complete and production-ready:

**Shipping Now:**
- ✓ Wayland screen capture via XDG Portals
- ✓ H.264 encoding (AVC420 and AVC444)
- ✓ Hardware acceleration (NVENC, VA-API)
- ✓ Keyboard and mouse input
- ✓ Clipboard sync (text, images, files)
- ✓ TLS 1.3 encryption
- ✓ PAM authentication
- ✓ Adaptive frame rate (5-60 FPS)
- ✓ Latency optimization
- ✓ Predictive cursor
- ✓ Multi-compositor support

---

## Planned Features

We're actively working on expanding lamco-rdp-server's capabilities. Here's what's on the roadmap:

### Audio Playback (RDPSND)

**Status:** In Development
**Priority:** High

Stream system audio from your Linux desktop to the RDP client. Watch videos, listen to music, and hear notification sounds remotely.

| Aspect | Plan |
|--------|------|
| Protocol | RDPSND (RDP audio channel) |
| Format | PCM, AAC passthrough |
| Backend | PipeWire audio capture |
| Latency target | <100ms |

**Why it matters:** Remote desktop without audio means videos are silent, notifications are missed, and video conferencing from the remote machine requires workarounds.

---

### Microphone Input

**Status:** Planned
**Priority:** Medium

Send audio from your client microphone to the Linux desktop. Enable video calls, voice chat, and voice input from remote sessions.

| Aspect | Plan |
|--------|------|
| Protocol | RDPSND bidirectional or AUDIN |
| Backend | PipeWire audio injection |
| Use case | Video conferencing, voice input |

**Why it matters:** Work-from-home scenarios often require video calls. Currently, you'd need to join calls from the local machine, not the remote desktop.

---

### Multi-Monitor Improvements

**Status:** Implemented (needs validation)
**Priority:** Medium

Enhanced multi-monitor support with dynamic layout changes.

**Current state:**
- Basic multi-monitor layout works
- Coordinate mapping implemented
- Some edge cases need testing

**Planned improvements:**
- Dynamic monitor add/remove
- Per-monitor resolution control
- Better DPI scaling across monitors
- Monitor arrangement from client

---

### Drive Redirection (RDPDR)

**Status:** Planned
**Priority:** Medium

Access files on your local client from the remote Linux desktop.

| Aspect | Plan |
|--------|------|
| Protocol | RDPDR (RDP drive redirection) |
| Access | Read and write |
| Integration | FUSE mount on server |

**Why it matters:** Transfer files between local and remote without separate tools. Open local files directly in remote applications.

---

### Printer Redirection

**Status:** Considered
**Priority:** Low

Print from remote Linux applications to your local printer.

**Why it's lower priority:** Most users print to PDF or use cloud printing. Dedicated printer redirection adds complexity for a less common use case.

---

### Smart Card Support

**Status:** Considered
**Priority:** Low

Use local smart cards for authentication on the remote system.

**Why it's lower priority:** Requires specific hardware and enterprise infrastructure. Will prioritize if user demand emerges.

---

### 60fps High Performance Mode

**Status:** Implemented
**Priority:** Complete ✓

Support for up to 60 FPS for smooth video playback and animation.

**Current state:** Working with adaptive FPS. System automatically scales to 60 FPS during high-activity content.

---

## Feature Voting

We prioritize features based on user feedback. Tell us what matters to you:

### How to Request Features

**Email:** office@lamco.io
Subject: "Feature Request: [feature name]"

Tell us:
1. What feature you need
2. Your use case (how you'd use it)
3. How important it is to your workflow
4. Whether you'd pay for it (helps us prioritize)

**GitHub:** When the public repository launches, you can submit and vote on feature requests via GitHub Issues.

---

## What We're NOT Building

To set expectations, here are features outside our scope:

### X11 Support

lamco-rdp-server is Wayland-native by design. We won't add X11 capture—that's what xrdp does well. If you run X11, use xrdp.

### Windows/macOS Server

lamco-rdp-server is a Linux product. Windows and macOS have built-in RDP servers or established alternatives.

### Enterprise VDI Management

We're building a remote desktop server, not a VDI orchestration platform. If you need central management of hundreds of virtual desktops, use enterprise VDI solutions.

### Proprietary Protocol

We use standard RDP because any RDP client can connect. We won't switch to a proprietary protocol that requires a special client.

---

## Technical Roadmap

### Infrastructure Improvements

| Area | Status | Notes |
|------|--------|-------|
| Flatpak packaging | In progress | Primary distribution method |
| OBS builds | Planned | Multi-distro packages |
| Automated testing | Ongoing | CI/CD pipeline |
| Documentation | Ongoing | User and developer docs |

### Performance Targets

| Metric | Current | Target |
|--------|---------|--------|
| Encode latency | 12-18ms | <10ms |
| Input latency (LAN) | 15-25ms | <15ms |
| CPU usage (HW encode) | 2-5% | <3% |
| Memory usage | ~100MB | <80MB |

### Code Quality Goals

- Comprehensive unit test coverage
- Fuzz testing for protocol parsing
- Memory safety (Rust helps here)
- Regular dependency updates

---

## Contributing

Interested in helping build lamco-rdp-server?

### Areas Where We Need Help

**Testing:**
- Different hardware configurations
- Various RDP clients
- Edge cases and bug reports

**Documentation:**
- User guides for specific distributions
- Troubleshooting content
- Translation

**Code (when public repo launches):**
- Bug fixes
- Performance improvements
- Platform support

### Contact

**Email:** office@lamco.io
**Subject:** "Interested in Contributing"

Tell us:
- Your background/expertise
- What you're interested in working on
- Your availability

---

## Release Philosophy

### Stability Over Features

We won't rush half-baked features. Every release should be more stable than the last. If a feature isn't ready, it waits.

### Semantic Versioning

- **Major (1.x → 2.0):** Breaking changes
- **Minor (1.0 → 1.1):** New features, backward compatible
- **Patch (1.0.0 → 1.0.1):** Bug fixes only

### Support Windows

Commercial licensees receive priority email support. We aim to respond within 48 hours for support requests.

**Support email:** office@lamco.io

---

## Timeline Expectations

We don't publish specific dates because software development is unpredictable. Features ship when they're ready.

**General guidance:**
- Audio (RDPSND): Near-term priority
- Microphone: Following audio
- Multi-monitor improvements: Ongoing
- Drive redirection: Medium-term

Watch the [News page](/news/) for release announcements.

---

## Stay Updated

**Email updates:** Contact office@lamco.io to join our announcement list

**GitHub:** Star/watch the repository (when public) for release notifications

**Website:** Check [lamco.ai/news](/news/) for announcements

---

## Your Input Matters

lamco-rdp-server exists because Linux users needed better remote desktop options. Your feedback shapes what we build next.

**Feature requests:** office@lamco.io
**Bug reports:** office@lamco.io (GitHub Issues coming)
**General questions:** office@lamco.io

Priority support for commercial license holders.
