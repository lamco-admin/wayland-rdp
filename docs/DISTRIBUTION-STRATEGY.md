# Distribution Strategy Notes

**Date**: 2025-12-08
**Status**: Planning - Not Ready for Implementation

---

## Overview

This project has two distinct products that need different distribution strategies:

1. **Portal Mode** - Desktop screen sharing RDP server (for end users)
2. **Compositor Mode** - Headless VDI server (for cloud/enterprise)

Additionally, there's a companion GNOME Shell extension for clipboard monitoring.

---

## GNOME Clipboard Extension Distribution

**Decision**: Distribute via BOTH channels

### Channel 1: extensions.gnome.org
- Primary discovery channel
- Auto-updates independent of main server
- Users searching "clipboard RDP" or "remote desktop" find it
- Works standalone for other RDP servers (community benefit)
- Review process takes weeks - plan ahead

### Channel 2: Bundled with main package
- Version-matched to server
- Offline install capability
- Post-install script or `--install-extension` flag
- Copies to `~/.local/share/gnome-shell/extensions/`

---

## Portal Mode Distribution (Desktop Users)

### Target Formats (Priority Order)

| Priority | Format | Target Audience | Notes |
|----------|--------|-----------------|-------|
| P1 | **Flatpak** | General users | Cross-distro, Flathub discovery, sandboxed |
| P1 | **Deb** | Ubuntu/Debian | Large user base |
| P2 | **RPM** | Fedora/RHEL | Enterprise Linux |
| P2 | **AUR** | Arch users | Community maintained |
| P3 | **AppImage** | Portable/testing | Single file, no install |
| P3 | **Copr/PPA** | Easy repo add | Community repos |

### Flatpak Considerations
- Portal access needs proper permissions in manifest
- PipeWire access for video capture
- D-Bus access for Portal APIs
- Larger download size (~50-100MB with runtime)

### Native Package Considerations
- System dependencies: pipewire-dev, libspa-dev, etc.
- Systemd user service file for auto-start
- Desktop file for application menu
- Man page

---

## Compositor Mode Distribution (Headless VDI)

### Target Formats (Priority Order)

| Priority | Format | Target Audience | Notes |
|----------|--------|-----------------|-------|
| P1 | **Container** | Cloud/K8s | Docker/Podman, reproducible |
| P1 | **Deb** | Server installs | Bare metal, systemd |
| P2 | **RPM** | RHEL/Rocky | Enterprise servers |
| P2 | **Helm Chart** | Kubernetes | Multi-tenant VDI |
| P3 | **Nix** | DevOps | Reproducible builds |

### Container Considerations
- Base image: Debian slim or Alpine
- Multi-stage build for small image
- Health checks for orchestration
- Environment variable configuration
- Volume mounts for persistent config

### Kubernetes Considerations
- StatefulSet for session persistence
- Service for RDP port exposure
- ConfigMap for configuration
- Secrets for TLS certificates
- HorizontalPodAutoscaler for scaling

---

## Distribution Roadmap

### Phase 1: Developer/Early Adopter (Current)
- Manual build from source: `cargo build --release`
- GitHub releases with pre-built binaries
- Clear dependency documentation
- Install script for dependencies

### Phase 2: Easy Install (v1.0)
```bash
# Flatpak
flatpak install flathub <TBD-app-id>

# Ubuntu/Debian
sudo apt install <TBD-package-name>

# Fedora
sudo dnf install <TBD-package-name>

# Arch
yay -S <TBD-package-name>
```

### Phase 3: Enterprise/Cloud (v1.x)
```bash
# Container
docker pull ghcr.io/<TBD-org>/<TBD-image>:latest

# Kubernetes
helm install <TBD-name> <TBD-repo>/<TBD-chart>
```

---

## Open Source Strategy (TBD)

Potential components to open source:

| Component | Open Source? | Notes |
|-----------|--------------|-------|
| Core library | Yes | Community contributions, trust |
| Clipboard extension | Yes | Other projects can use |
| Portal mode | TBD | Full transparency vs commercial |
| Compositor mode | TBD | Enterprise differentiation? |

---

## Naming Consistency (TBD)

Need to establish consistent naming across:
- GitHub repositories
- Package names
- Binary names
- D-Bus service names
- Flatpak app ID
- Documentation

See separate naming discussion.

---

## Dependencies to Document

### Build Dependencies
- Rust toolchain (1.70+)
- pkg-config
- OpenSSL development headers
- PipeWire development headers
- libspa development headers

### Runtime Dependencies
- PipeWire
- Portal-compatible compositor (GNOME, KDE, Sway)
- D-Bus session bus
- XDG Desktop Portal

---

## Notes

- Distribution strategy can evolve after v1.0
- Focus on getting the software working first
- Community feedback will guide packaging priorities
- Consider hiring/contracting for packaging if needed

---

**TODO**: Revisit this document when ready to implement distribution.
