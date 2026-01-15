# Open Build Service (OBS) Setup for lamco-rdp-server

This document describes the complete OBS appliance setup for building lamco-rdp-server
packages across multiple Linux distributions.

## OBS Appliance Information

- **IP Address**: 192.168.10.8
- **Web Interface**: https://192.168.10.8
- **OBS Version**: 2.10.30
- **Base OS**: openSUSE Leap 15.1
- **Default Credentials**:
  - Web UI: Admin / opensuse
  - SSH: root / Bibi4189

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    OBS Appliance (192.168.10.8)                      │
│                                                                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │
│  │ srcserver   │  │ repserver   │  │ scheduler   │  │ dispatcher  │ │
│  │ (sources)   │  │ (repos)     │  │ (jobs)      │  │ (workers)   │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘ │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                    6 KVM Workers (512MB each)                   ││
│  │  worker_1  worker_2  worker_3  worker_4  worker_5  worker_6    ││
│  └─────────────────────────────────────────────────────────────────┘│
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                 Interconnect to build.opensuse.org              ││
│  │         (Provides access to Fedora, Debian, Ubuntu, etc.)       ││
│  └─────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
```

## Services Status

All services should be running. Check with:

```bash
ssh root@192.168.10.8 "systemctl is-active obs{srcserver,repserver,scheduler,dispatcher,publisher,warden,service,worker}"
```

### Service Descriptions

| Service | Purpose | Port |
|---------|---------|------|
| obssrcserver | Source package storage | 5352 |
| obsrepserver | Repository management | 5252 |
| obsscheduler | Job scheduling (per-arch) | - |
| obsdispatcher | Worker job assignment | - |
| obspublisher | Package publishing | - |
| obswarden | Worker monitoring | - |
| obsservice | Source services | 5152 |
| obsworker | Build workers (6 instances) | - |

## Interconnect Configuration

The OBS appliance is connected to build.opensuse.org for access to distribution
packages. This was configured via:

```xml
<project name="openSUSE.org">
  <title>openSUSE Build Service</title>
  <description>This project links to the openSUSE Build Service</description>
  <remoteurl>https://api.opensuse.org/public</remoteurl>
</project>
```

## Build Targets Configured

The **lamco** project is configured to build for:

### Working Builds (7 targets)

| Target | Description | Package Format | Status |
|--------|-------------|----------------|--------|
| Fedora_42 | Fedora 42 x86_64 | RPM | ✅ |
| Fedora_41 | Fedora 41 x86_64 | RPM | ✅ |
| Fedora_40 | Fedora 40 x86_64 | RPM | ✅ |
| openSUSE_Tumbleweed | Rolling release x86_64 | RPM | ✅ |
| openSUSE_Leap_15.6 | Stable release x86_64 | RPM | ✅ |
| Debian_13 | Trixie x86_64 | DEB | ✅ |
| AlmaLinux_9 | RHEL 9 compatible x86_64 | RPM | ✅ |

### Unresolvable (Flatpak recommended)

| Target | Description | Issue |
|--------|-------------|-------|
| Ubuntu_24.04 | Noble Numbat x86_64 | Rust 1.75 < 1.77 required |
| Debian_12 | Bookworm x86_64 | Rust 1.63 < 1.77 required |

### RHEL 9 / Rocky Linux 9 Compatibility

AlmaLinux 9 packages are **100% binary compatible** with RHEL 9 and Rocky Linux 9.
Users on any EL9 distribution can install the AlmaLinux 9 packages directly.

## Package Structure

### Files in OBS Package

```
lamco/lamco-rdp-server/
├── lamco-rdp-server-0.1.0.tar.xz  (64MB vendored source)
├── lamco-rdp-server.spec          (RPM spec file)
├── lamco-rdp-server.dsc           (Debian source control)
└── debian.tar.gz                  (Debian packaging files)
```

### Source Tarball Contents

The vendored source tarball includes:
- Main lamco-rdp-server source code
- lamco-rdp-workspace/ with local crates:
  - lamco-clipboard-core
  - lamco-rdp-clipboard
- vendor/ directory with all Rust dependencies (400+ crates)
- .cargo/config.toml for offline builds

## Build Dependencies

### RPM (Fedora, openSUSE)

```
rust >= 1.77, cargo, pkgconfig, gcc, make, nasm, clang, clang-devel,
pipewire-devel, wayland-devel, xkbcommon-devel, dbus-devel,
libva-devel >= 1.20.0, pam-devel, openssl-devel
```

### DEB (Debian, Ubuntu)

```
rustc (>= 1.77), cargo, pkg-config, nasm, clang, libclang-dev,
libpipewire-0.3-dev, libspa-0.2-dev, libwayland-dev, libxkbcommon-dev,
libdbus-1-dev, libva-dev (>= 2.20), libpam0g-dev, libssl-dev
```

## Managing the OBS

### Starting All Services

```bash
ssh root@192.168.10.8

# Start in order
systemctl start obssrcserver
systemctl start obsrepserver
systemctl start obsscheduler
systemctl start obsdispatcher
systemctl start obspublisher
systemctl start obswarden
systemctl start obsservice
systemctl start obsworker
```

### Checking Build Status

```bash
# Via API
curl -s -k -u Admin:opensuse "https://192.168.10.8/build/lamco/_result"

# Via osc (if installed)
osc -A https://192.168.10.8 results lamco lamco-rdp-server
```

### Rebuilding a Package

```bash
curl -s -k -u Admin:opensuse -X POST \
  "https://192.168.10.8/build/lamco?cmd=rebuild&package=lamco-rdp-server"
```

### Viewing Build Logs

```bash
curl -s -k -u Admin:opensuse \
  "https://192.168.10.8/build/lamco/Fedora_40/x86_64/lamco-rdp-server/_log"
```

## Creating New Releases

1. Update version in Cargo.toml
2. Regenerate vendored tarball:
   ```bash
   cd /home/greg/wayland/wrd-server-specs
   bash packaging/create-vendor-tarball.sh VERSION
   ```
3. Update spec file version
4. Update debian/changelog
5. Upload new files to OBS:
   ```bash
   scp packaging/lamco-rdp-server-VERSION.tar.xz root@192.168.10.8:/tmp/
   ssh root@192.168.10.8 'curl -k -u Admin:opensuse -X PUT \
     "https://localhost/source/lamco/lamco-rdp-server/lamco-rdp-server-VERSION.tar.xz" \
     -T /tmp/lamco-rdp-server-VERSION.tar.xz'
   ```

## Flatpak Builds

A Flatpak manifest is provided at `packaging/ai.lamco.rdp-server.yml`.

Build locally with:
```bash
flatpak-builder --user --install build-dir packaging/ai.lamco.rdp-server.yml
```

## Known Issues

### Ubuntu 24.04 and Debian 12 - Unresolvable

These distributions ship with Rust versions older than our 1.77 requirement:

| Distribution | Rust Version | Required |
|--------------|-------------|----------|
| Ubuntu 24.04 | 1.75 | >= 1.77 |
| Debian 12 | 1.63 | >= 1.77 |

**Possible solutions:**
1. Wait for newer Rust packages in distro repos
2. Add external Rust toolchain repositories as build dependencies
3. Lower the minimum Rust version in Cargo.toml (if feasible)

Additionally, Ubuntu 24.04 is missing: `nasm`, `clang`, `libclang-dev`, `libva-dev >= 2.20`
Debian 12 is missing: `libva-dev >= 2.20` (has 2.17)

### Working Build Targets

The following targets build successfully:
- Fedora 40
- openSUSE Tumbleweed
- openSUSE Leap 15.6
- Debian 13 (Trixie)

## Services Configuration

### Required Services (Must be Enabled)

All these services must be enabled for OBS to work properly after reboot:

**Backend Services:**
```bash
systemctl enable obssrcserver obsrepserver obsscheduler obsdispatcher \
  obspublisher obswarden obsservice obsservicedispatch obsworker \
  obssignd obssigner obsdeltastore obsdodup obsstoragesetup
```

**API Support Services:**
```bash
systemctl enable obs-clockwork obs-sphinx obs-api-support.target \
  obs-delayedjob-queue-consistency_check obs-delayedjob-queue-default \
  obs-delayedjob-queue-issuetracking obs-delayedjob-queue-mailers \
  obs-delayedjob-queue-project_log_rotate obs-delayedjob-queue-releasetracking \
  obs-delayedjob-queue-staging obs-delayedjob-queue-quick@0 \
  obs-delayedjob-queue-quick@1 obs-delayedjob-queue-quick@2
```

**Infrastructure:**
```bash
systemctl enable apache2 mariadb memcached
```

### Verify All Services Running

```bash
systemctl list-units --type=service --state=running | grep -E 'obs|apache|maria|memcache'
```

## Troubleshooting

### API Returns 500 Errors

**Cause 1: Disk Full**
```bash
# Check disk usage
df -h

# OBS worker cache can grow very large
du -sh /var/cache/obs/worker

# Clean worker cache if needed (stop workers first)
systemctl stop obsworker
rm -rf /var/cache/obs/worker/*
systemctl start obsworker
```

**Cause 2: Database Issues**
```bash
# Test database connection
mysql -u root -popensuse -e 'SELECT 1'

# Check MariaDB status
systemctl status mariadb
```

### Scheduler Not Running / x86_64 Builds Not Starting

The obsscheduler service starts per-architecture schedulers. If x86_64 builds aren't
being scheduled, the x86_64 scheduler may not be running:

```bash
# Check which schedulers are running
ps aux | grep bs_sched

# Should see x86_64 scheduler - if missing, restart:
/usr/sbin/obsscheduler start

# Verify x86_64 scheduler is now running
ps aux | grep bs_sched | grep x86_64
```

### Build Stuck in "blocked"

Usually means dependencies not available. Check:
```bash
curl -s -k -u Admin:opensuse \
  "https://192.168.10.8/build/lamco/Fedora_40/x86_64/lamco-rdp-server/_buildinfo"
```

### Workers Not Building

```bash
# Check worker status
systemctl status obsworker

# Check worker processes (should see 6)
ps aux | grep bs_worker | grep -v grep | wc -l

# Check worker activity
curl -s -k -u Admin:opensuse 'https://localhost/build/_workerstatus'
```

### Disk Space Management

The appliance disk was expanded to 456GB (from original 96GB). The main space consumers:

| Directory | Description | Can Clean? |
|-----------|-------------|------------|
| /var/cache/obs/worker | Worker build cache | Yes (stops workers) |
| /srv/obs/remotecache | Remote package cache | Yes (will re-download) |
| /srv/obs/build | Build results | Careful |

To expand disk further (if more space available):
```bash
# Check available space
parted /dev/sda print free

# Extend partition (example: to 1TB)
parted /dev/sda resizepart 2 1000GB

# Resize filesystem
resize2fs /dev/sda2
```

### Current Build Issue: signature crate edition2024

Builds are failing with error:
```
feature `edition2024` is required
The package requires the Cargo feature called `edition2024`, but that feature is
not stabilized in this version of Cargo (1.77.0).
```

The `signature` crate in the vendor directory requires Rust edition 2024, which requires
a newer Rust version than available in most distros. This needs to be fixed by either:

1. Using an older version of the signature crate in dependencies
2. Regenerating the vendor tarball with compatible dependency versions
3. Updating the minimum Rust version requirement

---

## Flatpak Builds

### OBS Flatpak: NOT VIABLE

OBS Flatpak builds are **not viable** for Rust projects due to severely outdated base images:

| Available in OBS:Flatpak | Current Flathub |
|--------------------------|-----------------|
| org.freedesktop.Platform 20.08 | 24.08 |
| org.freedesktop.Sdk 20.08 | 24.08 |
| org.gnome.Platform 3.38 | 47 |

**Critical Missing:**
- `org.freedesktop.Sdk.Extension.rust-stable` - Not available
- Runtime versions 23.08, 24.08 - Not available

### Local Flatpak Builds (Recommended)

Build locally using flatpak-builder with Flathub runtimes:

```bash
# Install prerequisites
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install flathub org.freedesktop.Sdk//24.08
flatpak install flathub org.freedesktop.Platform//24.08
flatpak install flathub org.freedesktop.Sdk.Extension.rust-stable//24.08

# Build
cd /home/greg/wayland/wrd-server-specs
flatpak-builder --user --install build-dir packaging/ai.lamco.rdp-server.yml
```

### Manifest Files

| File | Purpose |
|------|---------|
| `packaging/ai.lamco.rdp-server.yml` | Local builds (uses `path:` source) |
| `packaging/flatpak.yaml` | OBS format (uses `url:` source, not usable due to runtime limitations) |

### Future: Flathub Submission

For universal Flatpak distribution, consider submitting to Flathub directly:
- https://github.com/flathub/flathub/wiki/App-Submission

Flathub provides automated CI builds, distribution through software centers, and automatic updates.

---

## Version History

| Date | Change |
|------|--------|
| 2026-01-14 | Initial OBS appliance setup |
| 2026-01-14 | Created vendored source tarball (64MB) |
| 2026-01-14 | Added Flatpak manifest |
| 2026-01-14 | Renamed project to "lamco" for proper branding |
| 2026-01-14 | Final targets: Fedora 40/41/42, openSUSE Tumbleweed/Leap 15.6, Debian 13, AlmaLinux 9 |
| 2026-01-14 | Documented RHEL 9 strategy (AlmaLinux 9 packages compatible with RHEL/Rocky) |
| 2026-01-14 | Documented unresolvable targets (Ubuntu 24.04/Debian 12 - old Rust) |
| 2026-01-15 | Fixed disk full issue (expanded to 456GB, cleaned worker cache) |
| 2026-01-15 | Enabled all API support services for autostart |
| 2026-01-15 | Fixed x86_64 scheduler not running |
| 2026-01-15 | Documented services configuration and troubleshooting |
| 2026-01-15 | Determined OBS Flatpak not viable (base images outdated, no Rust SDK extension) |
| 2026-01-15 | Documented local Flatpak builds as alternative |
