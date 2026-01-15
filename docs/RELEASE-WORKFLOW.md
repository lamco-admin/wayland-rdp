# lamco-rdp-server Release Workflow

This document describes the complete development, testing, and release workflow for lamco-rdp-server.

## Workflow Overview

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ Development │ -> │   Testing   │ -> │  Packaging  │ -> │   Release   │
│             │    │             │    │             │    │             │
│ - Code      │    │ - Unit      │    │ - Version   │    │ - Tag       │
│ - Features  │    │ - Manual    │    │ - Tarball   │    │ - OBS       │
│ - Fixes     │    │ - Platform  │    │ - Upload    │    │ - Announce  │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

## Phase 1: Development

### Local Development Setup

```bash
# Clone repository
git clone https://github.com/lamco-admin/lamco-rdp-server
cd lamco-rdp-server

# Build
cargo build --release --features "default,vaapi"

# Run locally
./target/release/lamco-rdp-server --config config.toml
```

### Branch Strategy

| Branch | Purpose |
|--------|---------|
| `main` | Stable releases only |
| `develop` | Active development |
| `feature/*` | New features |
| `fix/*` | Bug fixes |
| `release/*` | Release preparation |

### Commit Guidelines

- Use conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`
- Reference issues: `fix: resolve clipboard timeout (#42)`
- Keep commits atomic and focused

## Phase 2: Testing

### 2.1 Automated Tests

```bash
# Run all tests
cargo test

# Run with specific features
cargo test --features "vaapi"

# Run benchmarks
cargo bench
```

### 2.2 Manual Testing Checklist

Before any release, verify:

#### Core Functionality
- [ ] Server starts without errors
- [ ] RDP client can connect (Windows, FreeRDP)
- [ ] Screen capture works via portal
- [ ] Mouse input works
- [ ] Keyboard input works
- [ ] Multi-monitor detected correctly

#### Clipboard Features
- [ ] Text copy: Linux → Windows
- [ ] Text paste: Windows → Linux
- [ ] Image copy: Linux → Windows
- [ ] Image paste: Windows → Linux
- [ ] File list transfer
- [ ] Large clipboard content (>1MB)
- [ ] Clipboard timeout handling

#### Service Discovery
- [ ] mDNS advertisement works
- [ ] Service visible from Windows
- [ ] Service visible from other Linux clients
- [ ] Multiple instances have unique names

#### Platform-Specific
- [ ] GNOME (Wayland) - full test
- [ ] KDE Plasma (Wayland) - full test
- [ ] RHEL 9 / AlmaLinux 9 - portal quirks
- [ ] Headless mode (if applicable)

### 2.3 Platform Testing Matrix

| Platform | Desktop | Status | Notes |
|----------|---------|--------|-------|
| Fedora 40+ | GNOME | Primary | Full support |
| openSUSE Tumbleweed | KDE/GNOME | Primary | Full support |
| Debian 13 | GNOME | Primary | Full support |
| AlmaLinux 9 | GNOME | Primary | Portal quirks handled |
| Ubuntu 24.04 | GNOME | Flatpak only | Old Rust in repos |

## Phase 3: Packaging

### 3.1 Version Bump

1. Update version in `Cargo.toml`:
   ```toml
   version = "X.Y.Z"
   ```

2. Update workspace crates if needed:
   ```bash
   # In lamco-rdp-workspace
   vim Cargo.toml  # Update workspace.package.version
   ```

3. Update packaging files:
   ```bash
   vim packaging/lamco-rdp-server.spec  # Update Version:
   vim packaging/debian/changelog       # Add new entry
   vim packaging/lamco-rdp-server.dsc   # Update Version:
   ```

### 3.2 Create Vendored Tarball

```bash
cd /home/greg/wayland/wrd-server-specs

# Create tarball with all dependencies
bash packaging/create-vendor-tarball.sh X.Y.Z

# Verify tarball
ls -lh packaging/lamco-rdp-server-X.Y.Z.tar.xz
```

### 3.3 Update Flatpak Manifest

1. Calculate new SHA256:
   ```bash
   sha256sum packaging/lamco-rdp-server-X.Y.Z.tar.xz
   ```

2. Update `packaging/ai.lamco.rdp-server.yml`:
   ```yaml
   sources:
     - type: archive
       path: lamco-rdp-server-X.Y.Z.tar.xz
       sha256: <new-hash>
   ```

### 3.4 Regenerate Debian Tarball

```bash
cd packaging
tar czf debian.tar.gz debian/
```

## Phase 4: OBS Upload

### 4.1 Upload Files to OBS

```bash
# SSH to OBS or use API
OBS_HOST="192.168.10.8"
VERSION="X.Y.Z"

# Upload source tarball
scp packaging/lamco-rdp-server-${VERSION}.tar.xz root@${OBS_HOST}:/tmp/

# Upload via API
ssh root@${OBS_HOST} "curl -k -u Admin:opensuse -X PUT \
  'https://localhost/source/lamco/lamco-rdp-server/lamco-rdp-server-${VERSION}.tar.xz' \
  -T /tmp/lamco-rdp-server-${VERSION}.tar.xz"

# Upload spec file
curl -k -u Admin:opensuse -X PUT \
  "https://${OBS_HOST}/source/lamco/lamco-rdp-server/lamco-rdp-server.spec" \
  -T packaging/lamco-rdp-server.spec

# Upload debian files
curl -k -u Admin:opensuse -X PUT \
  "https://${OBS_HOST}/source/lamco/lamco-rdp-server/lamco-rdp-server.dsc" \
  -T packaging/lamco-rdp-server.dsc

curl -k -u Admin:opensuse -X PUT \
  "https://${OBS_HOST}/source/lamco/lamco-rdp-server/debian.tar.gz" \
  -T packaging/debian.tar.gz
```

### 4.2 Trigger Rebuild

```bash
curl -k -u Admin:opensuse -X POST \
  "https://${OBS_HOST}/build/lamco?cmd=rebuild&package=lamco-rdp-server"
```

### 4.3 Monitor Builds

```bash
# Check status
curl -s -k -u Admin:opensuse "https://${OBS_HOST}/build/lamco/_result"

# View build log
curl -s -k -u Admin:opensuse \
  "https://${OBS_HOST}/build/lamco/Fedora_42/x86_64/lamco-rdp-server/_log" | tail -50
```

## Phase 5: Release

### 5.1 Git Tag

```bash
git add -A
git commit -m "chore: release version X.Y.Z"
git tag -a vX.Y.Z -m "Release X.Y.Z"
git push origin main --tags
```

### 5.2 GitHub Release

Create release at: https://github.com/lamco-admin/lamco-rdp-server/releases/new

Include:
- Changelog highlights
- Download links for each platform
- Flatpak installation instructions
- Known issues

### 5.3 Announcement Template

```markdown
## lamco-rdp-server vX.Y.Z Released

### Highlights
- [Feature 1]
- [Feature 2]
- [Bug fix 1]

### Installation

**Fedora 40/41/42:**
```bash
dnf config-manager --add-repo https://download.opensuse.org/repositories/lamco/Fedora_42/lamco.repo
dnf install lamco-rdp-server
```

**openSUSE:**
```bash
zypper ar https://download.opensuse.org/repositories/lamco/openSUSE_Tumbleweed/lamco.repo
zypper install lamco-rdp-server
```

**Debian 13:**
```bash
# Add repository and install
apt install lamco-rdp-server
```

**RHEL 9 / Rocky / Alma:**
```bash
dnf config-manager --add-repo https://download.opensuse.org/repositories/lamco/AlmaLinux_9/lamco.repo
dnf install lamco-rdp-server
```

**Flatpak (Ubuntu 24.04, Debian 12, any distro):**
```bash
flatpak install flathub ai.lamco.rdp-server
```
```

## Quick Reference

### OBS Information
- **URL**: https://192.168.10.8
- **Project**: lamco
- **Credentials**: Admin / opensuse

### Build Targets

| Target | Format | RHEL Compatible |
|--------|--------|-----------------|
| Fedora 42 | RPM | No |
| Fedora 41 | RPM | No |
| Fedora 40 | RPM | No |
| openSUSE Tumbleweed | RPM | No |
| openSUSE Leap 15.6 | RPM | No |
| Debian 13 | DEB | No |
| AlmaLinux 9 | RPM | Yes (RHEL 9, Rocky 9) |

### Unresolvable (Use Flatpak)
- Ubuntu 24.04 (Rust 1.75 < 1.77)
- Debian 12 (Rust 1.63 < 1.77)

---

## Version History

| Date | Version | Notes |
|------|---------|-------|
| 2026-01-14 | 0.1.0 | Initial OBS setup, workflow documented |
