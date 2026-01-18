# Finalization and Publication Plan
**Date:** 2026-01-18
**Status:** Ready to Execute
**Target:** Multi-channel publication (Flatpak + Native Packages)

---

## Overview

This plan executes the **complete finalization and publication** of lamco-rdp-server based on the comprehensive audit findings. All issues identified in `COMPREHENSIVE-AUDIT-REPORT-2026-01-18.md` are addressed with specific, actionable tasks.

---

## Phase 1: Critical Code Fixes (BLOCKING)
**Duration:** 2-3 days
**Priority:** 🔴 CRITICAL - Must complete before any publication

### Task 1.1: Fix Portal Clipboard Crash Bug

**Issue:** Shared session lock causes clipboard operations to block input injection, leading to portal crashes.

**Files to modify:**
1. `src/clipboard/manager.rs`
2. `src/clipboard/ironrdp_backend.rs`
3. `src/server/mod.rs` (session initialization)
4. `src/session/strategies/portal_token.rs`

**Implementation:**
```rust
// BEFORE (src/session/strategies/portal_token.rs):
pub struct PortalSession {
    session: Arc<RwLock<ashpd::desktop::Session>>,  // SHARED LOCK
}

// AFTER:
pub struct PortalSession {
    clipboard_session: Arc<RwLock<ashpd::desktop::Session>>,
    input_session: Arc<RwLock<ashpd::desktop::Session>>,
}

// Both sessions created from same Portal permissions but with separate locks
```

**Testing:**
1. Deploy to Ubuntu 24.04 test VM (192.168.10.205)
2. Copy complex Excel spreadsheet (15+ formats)
3. Paste into LibreOffice Calc
4. Verify:
   - No portal crash
   - Mouse continues working during paste
   - No "no available capacity" errors
5. Repeat 10 times to confirm stability

**Success Criteria:**
- ✅ Excel paste completes without portal crash
- ✅ Input events process during clipboard operation
- ✅ No queue overflow errors in logs

---

### Task 1.2: Downgrade MemFd Size=0 Log Level

**Issue:** PipeWire sends empty buffers during setup (normal), causing log spam.

**File to modify:**
- `lamco-pipewire` crate (if in workspace) or vendored pipewire handling

**Implementation:**
```rust
// BEFORE:
warn!("Received MemFd buffer with size=0");

// AFTER:
debug!("Received MemFd buffer with size=0 (normal during stream setup)");
```

**Testing:**
1. Start server with `-vvv` flag
2. Connect RDP client
3. Verify logs don't spam MemFd warnings
4. Confirm DEBUG level shows the message

**Success Criteria:**
- ✅ No WARN-level MemFd messages in normal operation
- ✅ DEBUG level shows informational message

---

## Phase 2: Documentation Updates (HIGH PRIORITY)
**Duration:** 1-2 days
**Priority:** 🟡 HIGH - Required before publication

### Task 2.1: Update README.md

**File:** `README.md`

**Changes Required:**

#### Add Section: Dependency Architecture (after line 212)

```markdown
## Dependency Architecture

lamco-rdp-server uses a layered dependency strategy:

### Published Lamco Crates (crates.io)

These are stable, reusable components published independently:

- **lamco-wayland** (0.2.3) - Wayland protocol bindings
- **lamco-rdp** (0.5.0) - Core RDP utilities
- **lamco-portal** (0.3.0) - XDG Desktop Portal integration
- **lamco-pipewire** (0.1.4) - PipeWire screen capture
- **lamco-video** (0.1.2) - Video frame processing
- **lamco-rdp-input** (0.1.1) - Input event translation

### Bundled Crates (Local Path Dependencies)

These crates are **intentionally not published** because they implement traits from our forked IronRDP:

- **lamco-clipboard-core** (0.5.0) - Clipboard protocol core
- **lamco-rdp-clipboard** (0.2.2) - IronRDP clipboard backend

**Why bundled?** These crates implement `CliprdrBackend` from `ironrdp-cliprdr`. Since we patch IronRDP to our fork (for file transfer methods), these crates must compile against the same patched version to avoid trait conflicts.

### Forked Dependencies

We maintain a fork of IronRDP with features pending upstream merge:

- **Repository:** https://github.com/lamco-admin/IronRDP (master branch)
- **Pending PRs:**
  - #1057: MS-RDPEGFX Graphics Pipeline Extension
  - #1063: reqwest feature fix
  - #1064-1066: Clipboard file transfer methods

All 11 IronRDP crates are patched via `[patch.crates-io]` to ensure consistency.
```

#### Add Section: Session Persistence Strategies (after Features section)

```markdown
## Session Persistence & Unattended Operation

lamco-rdp-server implements **multi-strategy session management** to enable unattended operation across different Linux environments:

### Available Strategies

1. **Mutter Direct API** (GNOME-specific)
   - Zero permission dialogs (even first time)
   - Uses `org.gnome.Mutter.ScreenCast` and `RemoteDesktop` D-Bus APIs
   - GNOME 42+ only
   - Best experience for GNOME desktops

2. **wlr-direct** (wlroots native)
   - Zero permission dialogs
   - Uses native Wayland protocols (`zwp_virtual_keyboard_v1`, `zwlr_virtual_pointer_v1`)
   - Requires `--features wayland`
   - Works on Sway, Hyprland, River, labwc
   - Sub-millisecond input latency

3. **Portal + libei/EIS** (Flatpak-compatible wlroots)
   - One-time permission dialog
   - Uses Portal RemoteDesktop + EIS protocol bridge
   - Requires `--features libei`
   - Works in Flatpak sandbox
   - Compatible with wlroots compositors that support ConnectToEIS

4. **Portal + Restore Tokens** (Universal)
   - One-time permission dialog, automatic restore on reconnect
   - Works on all desktops (GNOME, KDE, wlroots)
   - Portal v4+ required for tokens
   - Credentials stored securely (Secret Service, TPM 2.0, or encrypted file)

5. **Basic Portal** (Fallback)
   - Permission dialog on every restart
   - Works on all Portal-supported desktops
   - Graceful degradation when persistence unavailable

### Automatic Strategy Selection

The server automatically selects the best strategy based on:
- Detected compositor (GNOME Mutter, KWin, wlroots, etc.)
- Available Portal version and capabilities
- Deployment context (Flatpak, systemd, native binary)
- Session persistence support

See `docs/architecture/SESSION-PERSISTENCE-ARCHITECTURE.md` for complete technical details.
```

#### Add Section: Flatpak vs Native Builds (after Building section)

```markdown
## Flatpak vs Native Builds

Different deployment methods support different features:

### Flatpak Build

```bash
cargo --offline build --release --no-default-features --features "h264,libei"
```

**Enabled Features:**
- ✅ **h264** - OpenH264 video encoding (essential)
- ✅ **libei** - Portal + EIS/libei input (wlroots support in Flatpak)

**Disabled Features:**
- ❌ **pam-auth** - Not sandboxable in Flatpak
- ❌ **vaapi** - Incomplete color space API (being fixed)
- ❌ **wayland** - wlr-direct protocols blocked by sandbox
- ❌ **nvenc** - CUDA not available in Flatpak

**Strategy Availability:**
- Portal + Token (universal)
- libei (wlroots via Portal)

### Native Build

```bash
cargo build --release --features "default,vaapi"
# Or with all hardware encoding:
cargo build --release --features "default,hardware-encoding"
```

**Enabled Features:**
- ✅ **pam-auth** - PAM authentication
- ✅ **h264** - OpenH264 encoding
- ✅ **vaapi** - Intel/AMD GPU encoding (optional)
- ✅ **nvenc** - NVIDIA GPU encoding (optional)
- ✅ **wayland** - wlr-direct protocols (optional, for wlroots)
- ✅ **libei** - Portal + EIS (optional)

**Strategy Availability:**
- All 5 strategies (Mutter Direct, wlr-direct, libei, Portal + Token, Basic Portal)

### Feature Comparison Matrix

| Feature | Flatpak | Native | Notes |
|---------|---------|--------|-------|
| Video capture | Portal | Portal or Mutter | Both work |
| Input injection | Portal/libei | All strategies | More options native |
| Hardware encoding | ❌ | ✅ | VA-API/NVENC native only |
| wlr-direct | ❌ | ✅ | Requires direct Wayland access |
| Zero dialogs (GNOME) | ❌ | ✅ | Mutter Direct native only |
| Zero dialogs (wlroots) | ❌ | ✅ | wlr-direct native only |
| Portability | ✅ Best | Limited | Flatpak runs anywhere |
```

**Testing:**
1. Review changes in README.md
2. Verify markdown rendering (preview)
3. Check all links work
4. Validate code blocks render correctly

**Success Criteria:**
- ✅ README.md explains bundled vs published crates
- ✅ README.md documents all 5 session strategies
- ✅ README.md clarifies Flatpak vs native feature differences
- ✅ libei feature is documented

---

### Task 2.2: Update SERVICE-REGISTRY-TECHNICAL.md

**File:** `docs/SERVICE-REGISTRY-TECHNICAL.md`

**Changes:**

Lines 40-55: Update service count and list

```markdown
### ServiceId (18 services)  <!-- WAS: 11 services -->

```rust
pub enum ServiceId {
    // Display Services (8)
    DamageTracking,      // Bandwidth optimization via dirty region detection
    DmaBufZeroCopy,      // GPU buffer zero-copy path
    ExplicitSync,        // Tear-free display synchronization
    FractionalScaling,   // HiDPI support
    MetadataCursor,      // Client-side cursor rendering
    MultiMonitor,        // Multiple display support
    WindowCapture,       // Per-window capture capability
    HdrColorSpace,       // HDR passthrough (future)

    // I/O Services (3)
    Clipboard,           // Bidirectional clipboard
    RemoteInput,         // Keyboard/mouse injection
    VideoCapture,        // PipeWire video stream

    // Session Persistence Services (7) - Phase 2
    SessionPersistence,  // Portal restore token support
    DirectCompositorAPI, // Mutter/compositor direct APIs
    CredentialStorage,   // Token encryption backends
    UnattendedAccess,    // Zero-dialog capability
    WlrScreencopy,       // wlroots direct capture
    WlrDirectInput,      // wlroots virtual keyboard/pointer
    LibeiInput,          // Portal + EIS/libei protocol
}
```
```

**Testing:**
1. Verify service count is correct (18)
2. Check all services listed match src/services/service.rs
3. Validate formatting

**Success Criteria:**
- ✅ Service count updated to 18
- ✅ All services listed with descriptions
- ✅ Phase 2 services clearly marked

---

### Task 2.3: Create Website Publishing Data

**New directory:** `docs/website-data/`

**Files to create:**

#### 1. `docs/website-data/supported-distros.json`

```json
{
  "lastUpdated": "2026-01-18",
  "version": "0.1.0",
  "distributions": [
    {
      "name": "Ubuntu 24.04 LTS",
      "gnome": "46.0",
      "portal": "5",
      "kernel": "6.8+",
      "strategy": "Portal + Token",
      "status": "tested",
      "testDate": "2026-01-15",
      "persistence": "rejected-by-backend",
      "packaging": ["flatpak"],
      "issues": [
        {
          "id": "portal-crash-clipboard",
          "severity": "critical",
          "description": "xdg-desktop-portal-gnome crashes on complex Excel paste",
          "workaround": "Avoid pasting Excel with 15+ formats"
        }
      ],
      "capabilities": {
        "video": "guaranteed",
        "input": "guaranteed",
        "clipboard": "best-effort",
        "hardwareEncoding": false,
        "sessionPersistence": "unavailable"
      }
    },
    {
      "name": "RHEL 9",
      "gnome": "40.10",
      "portal": "4",
      "kernel": "5.14+",
      "strategy": "Portal",
      "status": "tested",
      "testDate": "2026-01-15",
      "persistence": "rejected-by-backend",
      "packaging": ["rpm", "flatpak"],
      "issues": [
        {
          "id": "no-clipboard-portal-v1",
          "severity": "medium",
          "description": "Portal RemoteDesktop v1 lacks clipboard support",
          "workaround": "Clipboard unavailable on RHEL 9"
        }
      ],
      "capabilities": {
        "video": "guaranteed",
        "input": "guaranteed",
        "clipboard": "unavailable",
        "hardwareEncoding": true,
        "sessionPersistence": "unavailable"
      }
    },
    {
      "name": "Pop!_OS 24.04 COSMIC",
      "compositor": "cosmic-comp 0.1.0",
      "portal": "5",
      "kernel": "6.17+",
      "strategy": "None",
      "status": "not-supported",
      "testDate": "2026-01-16",
      "persistence": "n/a",
      "packaging": ["flatpak"],
      "issues": [
        {
          "id": "cosmic-no-input",
          "severity": "blocker",
          "description": "COSMIC Portal backend lacks RemoteDesktop interface",
          "workaround": "Wait for Smithay PR #1388 completion"
        }
      ],
      "capabilities": {
        "video": "guaranteed",
        "input": "unavailable",
        "clipboard": "unavailable",
        "hardwareEncoding": false,
        "sessionPersistence": "unavailable"
      }
    },
    {
      "name": "Fedora 40",
      "gnome": "46.0",
      "portal": "5",
      "strategy": "Portal + Token",
      "status": "untested",
      "packaging": ["rpm", "flatpak"],
      "capabilities": {
        "video": "guaranteed",
        "input": "guaranteed",
        "clipboard": "best-effort",
        "hardwareEncoding": true,
        "sessionPersistence": "best-effort"
      }
    }
  ]
}
```

#### 2. `docs/website-data/feature-matrix.json`

```json
{
  "lastUpdated": "2026-01-18",
  "version": "0.1.0",
  "compositors": {
    "gnome": {
      "name": "GNOME (Mutter)",
      "versions": "42+",
      "features": {
        "video": "guaranteed",
        "input": "guaranteed",
        "clipboard": "best-effort",
        "multiMonitor": "best-effort",
        "damageTracking": "guaranteed",
        "metadataCursor": "guaranteed",
        "sessionPersistence": "unavailable",
        "zeroDialogs": "guaranteed"
      },
      "notes": "Mutter Direct API provides zero-dialog operation. Portal backend rejects session persistence for RemoteDesktop."
    },
    "kde": {
      "name": "KDE Plasma (KWin)",
      "versions": "6+",
      "features": {
        "video": "guaranteed",
        "input": "guaranteed",
        "clipboard": "guaranteed",
        "multiMonitor": "best-effort",
        "damageTracking": "guaranteed",
        "metadataCursor": "best-effort",
        "sessionPersistence": "guaranteed",
        "zeroDialogs": "best-effort"
      },
      "notes": "SelectionOwnerChanged works correctly. Portal tokens supported."
    },
    "wlroots": {
      "name": "wlroots (Sway, Hyprland, River)",
      "versions": "0.17+",
      "features": {
        "video": "best-effort",
        "input": "guaranteed",
        "clipboard": "best-effort",
        "multiMonitor": "best-effort",
        "damageTracking": "unavailable",
        "metadataCursor": "unavailable",
        "sessionPersistence": "guaranteed",
        "zeroDialogs": "guaranteed"
      },
      "notes": "Native deployment (wlr-direct) provides zero dialogs. Flatpak requires libei (Portal + EIS)."
    },
    "cosmic": {
      "name": "COSMIC (Smithay)",
      "versions": "0.1+",
      "features": {
        "video": "guaranteed",
        "input": "unavailable",
        "clipboard": "unavailable",
        "multiMonitor": "unavailable",
        "damageTracking": "unavailable",
        "metadataCursor": "unavailable",
        "sessionPersistence": "unavailable",
        "zeroDialogs": "unavailable"
      },
      "notes": "Portal RemoteDesktop not yet implemented. Waiting on Smithay PR #1388 (Ei protocol support)."
    }
  }
}
```

#### 3. `docs/website-data/README.md`

```markdown
# Website Publishing Data

**Purpose:** Structured data exports for lamco.io website consumption.

## Files

- **supported-distros.json** - Tested distributions with capabilities, issues, packaging options
- **feature-matrix.json** - Feature support by compositor (GNOME, KDE, wlroots, COSMIC)

## Schema

See individual JSON files for structure. All dates in ISO 8601 format (YYYY-MM-DD).

## Updates

Regenerate from `DISTRO-TESTING-MATRIX.md` after each test campaign.

```bash
# Future: automated extraction
./scripts/generate-website-data.sh
```

## Consumption

Website can fetch these JSON files and render:
- Distribution compatibility tables
- Feature comparison matrices
- Known issues lists
- Packaging download options
```

**Testing:**
1. Validate JSON syntax (use `jq` or JSON validator)
2. Check all fields populated correctly
3. Verify dates are current

**Success Criteria:**
- ✅ JSON files parse correctly
- ✅ Data matches DISTRO-TESTING-MATRIX.md
- ✅ Schema is logical and complete

---

## Phase 3: Build System Finalization (MEDIUM PRIORITY)
**Duration:** 1-2 days
**Priority:** 🟡 MEDIUM - Required for native packages

### Task 3.1: Align Flatpak Manifests

**Files:**
- `packaging/io.lamco.rdp-server.yml` (Flatpak primary)
- `packaging/ai.lamco.rdp-server.obs.yml` (OBS build)

**Changes to OBS manifest:**

```yaml
# Line 8: Standardize app-id
app-id: io.lamco.rdp-server  # WAS: ai.lamco.rdp-server

# After line 13: Add SDK extensions
sdk-extensions:
  - org.freedesktop.Sdk.Extension.rust-stable
  - org.freedesktop.Sdk.Extension.llvm18  # ADD THIS

# After line 40: Add libfuse3 module (before lamco-rdp-server module)
modules:
  # FUSE3 library for clipboard file transfer (drive redirection)
  - name: libfuse3
    buildsystem: meson
    config-opts:
      - -Dexamples=false
      - -Duseroot=false
      - -Dtests=false
      - -Dudevrulesdir=/app/lib/udev/rules.d
    sources:
      - type: archive
        url: https://github.com/libfuse/libfuse/releases/download/fuse-3.16.2/fuse-3.16.2.tar.gz
        sha256: f797055d9296b275e981f5f62d4e32e089614fc253d1ef2985851025b8a0ce87

# Line 45: Update build-options
  - name: lamco-rdp-server
    buildsystem: simple
    build-options:
      append-path: /usr/lib/sdk/rust-stable/bin:/usr/lib/sdk/llvm18/bin  # ADD llvm18
      env:
        CARGO_HOME: /run/build/lamco-rdp-server/cargo
        RUST_BACKTRACE: '1'
        LIBCLANG_PATH: /usr/lib/sdk/llvm18/lib  # ADD THIS

# Line 50: Update features (align with Flatpak)
    build-commands:
      - cargo --offline build --release --no-default-features --features "h264,libei"  # WAS: "default,vaapi"
```

**Rationale:**
- Standardize on `io.lamco.rdp-server` (matches Flathub conventions)
- Add llvm18 for OpenH264 NASM optimizations (3x speedup)
- Add libfuse3 for clipboard file transfer support
- Align features: Flatpak uses h264+libei, native RPM can use full features

**Testing:**
1. Validate YAML syntax
2. Build with flatpak-builder locally
3. Test resulting package

**Success Criteria:**
- ✅ Both manifests use same app-id
- ✅ OBS manifest includes libfuse3
- ✅ llvm18 SDK extension added

---

### Task 3.2: Create Flatpak Hash Update Script

**New file:** `scripts/update-flatpak-hash.sh`

```bash
#!/bin/bash
set -e

TARBALL="packaging/lamco-rdp-server-0.1.0.tar.xz"
MANIFEST="packaging/io.lamco.rdp-server.yml"

if [ ! -f "$TARBALL" ]; then
    echo "Error: Tarball not found at $TARBALL"
    echo "Run packaging/create-vendor-tarball.sh first"
    exit 1
fi

# Calculate new hash
NEW_HASH=$(sha256sum "$TARBALL" | awk '{print $1}')

echo "Tarball: $TARBALL"
echo "New SHA256: $NEW_HASH"

# Update manifest (line 76)
sed -i "s/sha256: .*/sha256: $NEW_HASH/" "$MANIFEST"

echo "✅ Updated $MANIFEST with new hash"
echo ""
echo "Verify the change:"
grep "sha256:" "$MANIFEST"
```

**Make executable:**
```bash
chmod +x scripts/update-flatpak-hash.sh
```

**Usage:**
```bash
# After rebuilding tarball
./packaging/create-vendor-tarball.sh
./scripts/update-flatpak-hash.sh
```

**Testing:**
1. Create test tarball
2. Run script
3. Verify hash updated in manifest
4. Test Flatpak build with new hash

**Success Criteria:**
- ✅ Script calculates correct sha256
- ✅ Manifest updated automatically
- ✅ Flatpak build succeeds with updated hash

---

### Task 3.3: Create Native Package Specs

#### RPM Spec File

**New file:** `packaging/lamco-rdp-server.spec`

```spec
Name:           lamco-rdp-server
Version:        0.1.0
Release:        1%{?dist}
Summary:        Professional RDP Server for Wayland/Linux Desktop Sharing

License:        BUSL-1.1
URL:            https://lamco.io
Source0:        %{name}-%{version}.tar.xz

BuildRequires:  rust >= 1.77
BuildRequires:  cargo
BuildRequires:  nasm >= 2.15
BuildRequires:  openssl-devel
BuildRequires:  pipewire-devel >= 0.3.77
BuildRequires:  libva-devel >= 1.20.0
BuildRequires:  xkbcommon-devel
BuildRequires:  systemd-rpm-macros

Requires:       pipewire >= 0.3.77
Requires:       xdg-desktop-portal
Requires:       pam

%description
lamco-rdp-server is a modern, production-tested remote desktop server for
Wayland-based Linux desktops. It implements the Remote Desktop Protocol (RDP)
with native Wayland support via XDG Desktop Portal and PipeWire.

%prep
%setup -q

%build
# Build with vendored dependencies (offline)
cargo build --release --offline --features "default,vaapi"

%install
# Install binary
install -Dm755 target/release/lamco-rdp-server %{buildroot}%{_bindir}/lamco-rdp-server

# Install config
install -Dm644 config.toml.example %{buildroot}%{_sysconfdir}/wrd-server/config.toml

# Install systemd user service
mkdir -p %{buildroot}%{_userunitdir}
cat > %{buildroot}%{_userunitdir}/lamco-rdp-server.service <<EOF
[Unit]
Description=Lamco RDP Server
After=graphical-session.target
Wants=xdg-desktop-portal.service pipewire.service

[Service]
Type=simple
ExecStart=%{_bindir}/lamco-rdp-server -c %{_sysconfdir}/wrd-server/config.toml
Restart=on-failure
RestartSec=5

[Install]
WantedBy=graphical-session.target
EOF

%files
%license LICENSE LICENSE-APACHE
%doc README.md
%{_bindir}/lamco-rdp-server
%config(noreplace) %{_sysconfdir}/wrd-server/config.toml
%{_userunitdir}/lamco-rdp-server.service

%changelog
* Sat Jan 18 2026 Lamco <contact@lamco.io> - 0.1.0-1
- Initial package
- Features: Portal session persistence, VA-API hardware encoding, H.264 video
```

#### Debian Package Files

**New directory:** `packaging/debian/`

**File:** `packaging/debian/control`

```
Source: lamco-rdp-server
Section: net
Priority: optional
Maintainer: Lamco <contact@lamco.io>
Build-Depends: debhelper-compat (= 13),
               cargo,
               rustc (>= 1.77),
               nasm (>= 2.15),
               libssl-dev,
               libpipewire-0.3-dev (>= 0.3.77),
               libva-dev (>= 1.20.0),
               libxkbcommon-dev,
               pkg-config
Standards-Version: 4.6.0
Homepage: https://lamco.io

Package: lamco-rdp-server
Architecture: amd64 arm64
Depends: ${shlibs:Depends}, ${misc:Depends},
         pipewire (>= 0.3.77),
         xdg-desktop-portal,
         libpam0g
Description: Professional RDP Server for Wayland/Linux
 lamco-rdp-server is a modern remote desktop server for Wayland-based
 Linux desktops. It implements the Remote Desktop Protocol (RDP) with
 native Wayland support via XDG Desktop Portal and PipeWire.
 .
 Features include H.264 video encoding, hardware acceleration (VA-API),
 session persistence, and multi-monitor support.
```

**File:** `packaging/debian/rules`

```makefile
#!/usr/bin/make -f

%:
	dh $@

override_dh_auto_build:
	cargo build --release --offline --features "default,vaapi"

override_dh_auto_install:
	install -Dm755 target/release/lamco-rdp-server debian/lamco-rdp-server/usr/bin/lamco-rdp-server
	install -Dm644 config.toml.example debian/lamco-rdp-server/etc/wrd-server/config.toml
	install -Dm644 packaging/debian/lamco-rdp-server.service debian/lamco-rdp-server/usr/lib/systemd/user/lamco-rdp-server.service

override_dh_auto_test:
	# Tests require Wayland session, skip for package build
```

**File:** `packaging/debian/lamco-rdp-server.service`

```ini
[Unit]
Description=Lamco RDP Server
After=graphical-session.target
Wants=xdg-desktop-portal.service pipewire.service

[Service]
Type=simple
ExecStart=/usr/bin/lamco-rdp-server -c /etc/wrd-server/config.toml
Restart=on-failure
RestartSec=5

[Install]
WantedBy=graphical-session.target
```

**Testing:**
1. Build RPM on RHEL 9 / Fedora 40
2. Build DEB on Ubuntu 24.04 / Debian 13
3. Install packages
4. Test systemd service
5. Verify binary works

**Success Criteria:**
- ✅ RPM builds successfully on RHEL 9
- ✅ DEB builds successfully on Ubuntu 24.04
- ✅ systemd service starts correctly
- ✅ Binary runs without errors

---

## Phase 4: lamco-admin Publishing Pipeline (BLOCKING)
**Duration:** 3-5 days
**Priority:** 🔴 CRITICAL - Required for public release

### Task 4.1: Create Public Repository

**Repository:** `github.com/lamco-admin/lamco-rdp-server`

**Structure:**
```
lamco-rdp-server/  (public repo)
├── src/
├── docs/
├── packaging/
├── bundled-crates/       # Include bundled clipboard crates
├── certs/                # Exclude (add to .gitignore)
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
├── LICENSE-APACHE
└── .gitignore
```

**Setup steps:**

1. Create repo on GitHub (lamco-admin organization)
2. Clone to local: `git clone git@github.com:lamco-admin/lamco-rdp-server.git`
3. Copy all project files EXCEPT:
   - `certs/` (test certificates)
   - `target/` (build artifacts)
   - `vendor/` (vendored deps, recreate during package build)
   - `.git/` (new git history)
   - `logs/` (local logs)
4. Update `.gitignore`:
   ```
   /target/
   /vendor/
   /certs/*.pem
   /logs/
   **/*.orig
   Cargo.lock  # Remove this line (we want Cargo.lock in repo)
   ```
5. Initialize git:
   ```bash
   cd lamco-rdp-server
   git init
   git add .
   git commit -m "Initial commit: lamco-rdp-server v0.1.0

   Professional RDP server for Wayland/Linux desktop sharing.

   Features:
   - Multi-strategy session persistence (5 strategies)
   - Hardware-accelerated H.264 encoding (VA-API, NVENC)
   - Service registry with 18 advertised services
   - Portal + Mutter Direct + wlr-direct support
   - Flatpak and native packaging

   Co-Authored-By: Claude Sonnet 4.5 (1M context) <noreply@anthropic.com>"

   git branch -M main
   git remote add origin git@github.com:lamco-admin/lamco-rdp-server.git
   git push -u origin main
   ```

6. Create release tag:
   ```bash
   git tag -a v0.1.0 -m "Release v0.1.0

   First production release of lamco-rdp-server.

   Tested on:
   - Ubuntu 24.04 (GNOME 46, Portal v5)
   - RHEL 9.7 (GNOME 40, Portal v4)
   - Pop!_OS 24.04 COSMIC (limited support)

   Known issues:
   - Portal crash on complex clipboard paste (Ubuntu 24.04)
   - GNOME rejects session persistence
   - COSMIC lacks RemoteDesktop portal"

   git push origin v0.1.0
   ```

**Testing:**
1. Clone public repo to fresh directory
2. Build from public repo source
3. Verify all dependencies resolve
4. Test binary functionality

**Success Criteria:**
- ✅ Public repo created and accessible
- ✅ All source code pushed
- ✅ v0.1.0 tag created
- ✅ Build succeeds from public repo

---

### Task 4.2: Create Vendor Tarball for Packaging

**Script:** Use existing `packaging/create-vendor-tarball.sh` (if exists) or create:

**File:** `packaging/create-vendor-tarball.sh`

```bash
#!/bin/bash
set -e

VERSION="0.1.0"
TARBALL="lamco-rdp-server-${VERSION}.tar.xz"
TMPDIR=$(mktemp -d)
SRCDIR="${TMPDIR}/lamco-rdp-server-${VERSION}"

echo "Creating source tarball: $TARBALL"

# Create source directory
mkdir -p "$SRCDIR"

# Copy source files (exclude build artifacts, certs, logs)
rsync -av \
  --exclude='.git' \
  --exclude='target' \
  --exclude='vendor' \
  --exclude='certs/*.pem' \
  --exclude='logs' \
  --exclude='*.orig' \
  --exclude='packaging/*.flatpak' \
  --exclude='packaging/.flatpak-builder' \
  --exclude='packaging/build-dir' \
  --exclude='packaging/repo' \
  ./ "$SRCDIR/"

# Vendor dependencies for offline builds
cd "$SRCDIR"
cargo vendor vendor/

# Create .cargo/config.toml for vendored build
mkdir -p .cargo
cat > .cargo/config.toml <<EOF
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

# Create tarball
cd "$TMPDIR"
tar -cJf "$TARBALL" "lamco-rdp-server-${VERSION}/"

# Move to packaging directory
mv "$TARBALL" "$OLDPWD/packaging/"

# Cleanup
cd "$OLDPWD"
rm -rf "$TMPDIR"

# Show result
ls -lh "packaging/$TARBALL"
echo "✅ Tarball created: packaging/$TARBALL"
echo ""
echo "SHA256:"
sha256sum "packaging/$TARBALL"
echo ""
echo "Next steps:"
echo "1. Update Flatpak manifest hash: ./scripts/update-flatpak-hash.sh"
echo "2. Test Flatpak build: flatpak-builder ..."
```

**Make executable:**
```bash
chmod +x packaging/create-vendor-tarball.sh
```

**Run:**
```bash
./packaging/create-vendor-tarball.sh
./scripts/update-flatpak-hash.sh
```

**Testing:**
1. Run tarball creation script
2. Extract tarball to test directory
3. Build from extracted source (offline mode)
4. Verify binary works

**Success Criteria:**
- ✅ Tarball created successfully
- ✅ Vendor directory populated
- ✅ Offline build succeeds
- ✅ Flatpak hash updated

---

### Task 4.3: Build and Test Flatpak Package

**Build Flatpak:**

```bash
cd packaging

# Build Flatpak bundle
flatpak-builder \
  --repo=repo \
  --force-clean \
  build-dir \
  io.lamco.rdp-server.yml

# Create .flatpak bundle for distribution
flatpak build-bundle \
  repo \
  io.lamco.rdp-server.flatpak \
  io.lamco.rdp-server \
  --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo

ls -lh io.lamco.rdp-server.flatpak
```

**Test Flatpak:**

```bash
# Install locally
flatpak install --user -y io.lamco.rdp-server.flatpak

# Run
flatpak run io.lamco.rdp-server --help

# Test on VM
scp io.lamco.rdp-server.flatpak user@192.168.10.205:~/
ssh user@192.168.10.205 "flatpak install --user -y ~/io.lamco.rdp-server.flatpak"
ssh user@192.168.10.205 "flatpak run io.lamco.rdp-server --show-capabilities"
```

**Success Criteria:**
- ✅ Flatpak builds without errors
- ✅ Bundle size reasonable (~6-7 MB)
- ✅ Installs and runs on test VM
- ✅ All features work (video, input, clipboard)

---

### Task 4.4: Build and Test Native Packages

**Build RPM:**

```bash
# On RHEL 9 / Fedora
rpmbuild -ba packaging/lamco-rdp-server.spec

# Test install
sudo dnf install -y ~/rpmbuild/RPMS/x86_64/lamco-rdp-server-0.1.0-1.*.rpm

# Test service
systemctl --user enable --now lamco-rdp-server
journalctl --user -u lamco-rdp-server -f
```

**Build DEB:**

```bash
# On Ubuntu 24.04 / Debian
cd packaging
dpkg-buildpackage -us -uc -b

# Test install
sudo apt install -y ../lamco-rdp-server_0.1.0_amd64.deb

# Test service
systemctl --user enable --now lamco-rdp-server
journalctl --user -u lamco-rdp-server -f
```

**Success Criteria:**
- ✅ RPM builds on RHEL 9
- ✅ DEB builds on Ubuntu 24.04
- ✅ Packages install correctly
- ✅ systemd service starts
- ✅ RDP connections work

---

## Phase 5: Final Integration Testing (CRITICAL)
**Duration:** 2-3 days
**Priority:** 🔴 CRITICAL - Validate everything works

### Task 5.1: End-to-End Testing Checklist

**Test Matrix:**

| Platform | Deployment | Features | Video | Input | Clipboard | Persistence | Status |
|----------|------------|----------|-------|-------|-----------|-------------|--------|
| Ubuntu 24.04 | Flatpak | h264, libei | ✅ | ✅ | ⚠️ (crash) | ❌ (rejected) | ⏳ |
| RHEL 9 | RPM | default, vaapi | ✅ | ✅ | ❌ (Portal v1) | ❌ (rejected) | ⏳ |
| RHEL 9 | Flatpak | h264, libei | ⏳ | ⏳ | ⏳ | ⏳ | ⏳ |
| Fedora 40 | RPM | default, vaapi | ⏳ | ⏳ | ⏳ | ⏳ | ⏳ |
| Ubuntu 22.04 | DEB | default | ⏳ | ⏳ | ⏳ | ⏳ | ⏳ |

**For each test:**

1. **Installation**
   - [ ] Package installs without errors
   - [ ] Config file created in correct location
   - [ ] systemd service unit installed
   - [ ] Binary has correct permissions

2. **Initial Run**
   - [ ] Service starts without errors
   - [ ] Portal dialog appears (expected)
   - [ ] User grants permission
   - [ ] Video stream starts
   - [ ] RDP client connects successfully

3. **Functionality**
   - [ ] Video displays correctly (H.264 frames)
   - [ ] Mouse moves accurately
   - [ ] Keyboard input works (all keys, modifiers)
   - [ ] Right-click menu appears
   - [ ] Clipboard text copy/paste (if supported)
   - [ ] Clipboard file transfer (if supported)
   - [ ] Multi-monitor layout (if applicable)

4. **Session Persistence**
   - [ ] Token stored (check credential backend)
   - [ ] Restart server
   - [ ] No new dialog on restart (if persistence supported)
   - [ ] Session restores automatically

5. **Performance**
   - [ ] Frame rate smooth (15-30 FPS)
   - [ ] Input latency low (<100ms)
   - [ ] CPU usage reasonable (<50%)
   - [ ] Memory stable (no leaks)

6. **Logs**
   - [ ] No ERROR-level messages (except known issues)
   - [ ] Service Registry detected correctly
   - [ ] Strategy selected appropriately
   - [ ] Capabilities logged accurately

**Success Criteria:**
- ✅ All CRITICAL platforms pass (Ubuntu 24.04, RHEL 9)
- ✅ All functionality works as documented
- ✅ Known issues match test results
- ✅ No new regressions introduced

---

### Task 5.2: Update Distribution Testing Matrix

**File:** `docs/DISTRO-TESTING-MATRIX.md`

**Updates to make:**

1. Mark all tested platforms with latest results
2. Update test dates
3. Add new platforms tested
4. Update known issues section
5. Add OBS build status for each distro

**Example entry:**

```markdown
| Distribution | Version | Rust | Build Status | Package | Test Status | Issues |
|--------------|---------|------|--------------|---------|-------------|--------|
| RHEL 9 | 9.7 | 1.84 | ✅ Built | RPM | ✅ Tested (2026-01-18) | Portal rejects persistence |
```

**Success Criteria:**
- ✅ All tested platforms documented
- ✅ Test dates current
- ✅ Build status accurate
- ✅ Issues listed match reality

---

## Phase 6: Publication (FINAL)
**Duration:** 1 day
**Priority:** 🟢 FINAL - Publish everything

### Task 6.1: Publish Flatpak to Flathub

**Prerequisites:**
- ✅ Flatpak builds and tests successfully
- ✅ Public repo exists
- ✅ Manifest validated

**Steps:**

1. Fork flathub/flathub on GitHub
2. Create new branch: `lamco-rdp-server`
3. Copy `packaging/io.lamco.rdp-server.yml` to fork
4. Update manifest source to use public repo:
   ```yaml
   sources:
     - type: archive
       url: https://github.com/lamco-admin/lamco-rdp-server/releases/download/v0.1.0/lamco-rdp-server-0.1.0.tar.xz
       sha256: <hash>
   ```
5. Create PR to flathub/flathub
6. Wait for review and approval

**Alternative: Self-hosted Flatpak repo** (faster, no approval needed)
```bash
# Add remote
flatpak remote-add --user lamco https://flatpak.lamco.io/repo

# Publish
flatpak build-bundle repo io.lamco.rdp-server.flatpak io.lamco.rdp-server
rsync -av repo/ flatpak.lamco.io:/var/www/flatpak/repo/
```

**Success Criteria:**
- ✅ Flatpak available on Flathub OR
- ✅ Self-hosted repo accessible
- ✅ Users can install: `flatpak install io.lamco.rdp-server`

---

### Task 6.2: Publish Native Packages via OBS

**Prerequisites:**
- ✅ RPM spec file created
- ✅ Source tarball created
- ✅ OBS project configured

**Steps:**

1. Upload source tarball to OBS
2. Upload RPM spec to OBS
3. Trigger builds for:
   - Fedora 42, 41, 40
   - RHEL 9 (AlmaLinux 9)
   - openSUSE Tumbleweed, Leap 15.6
   - Debian 13 (Trixie)
4. Monitor build results
5. Publish successful builds to repos

**For Debian/Ubuntu:**
1. Create Launchpad PPA or
2. Self-host APT repository

**Success Criteria:**
- ✅ Packages build on all target distros
- ✅ Repositories published
- ✅ Users can install via package manager

---

### Task 6.3: Publish Release Notes

**Create:** GitHub release for v0.1.0

**Release notes template:**

```markdown
# lamco-rdp-server v0.1.0

**First production release** of lamco-rdp-server, a professional RDP server for Wayland/Linux desktop sharing.

## 🎉 Features

- **Multi-Strategy Session Persistence** - 5 strategies for unattended operation
  - Mutter Direct API (GNOME, zero dialogs)
  - wlr-direct (wlroots native, zero dialogs)
  - Portal + libei/EIS (Flatpak-compatible wlroots)
  - Portal + Restore Tokens (universal)
  - Basic Portal (fallback)

- **Hardware-Accelerated H.264 Encoding**
  - OpenH264 (software, universal)
  - VA-API (Intel/AMD GPUs)
  - NVENC (NVIDIA GPUs)
  - AVC444 premium mode (full 4:4:4 chroma)

- **Service Advertisement Registry** - 18 services with 4-level guarantees
- **Multi-Monitor Support** - Layout negotiation and coordinate transformation
- **Damage Detection** - SIMD-optimized tile differencing (90%+ bandwidth savings)
- **Clipboard Sync** - Bidirectional text and file transfer

## 📦 Installation

### Flatpak (Universal)
```bash
flatpak install flathub io.lamco.rdp-server
flatpak run io.lamco.rdp-server
```

### Fedora / RHEL
```bash
sudo dnf install lamco-rdp-server
systemctl --user enable --now lamco-rdp-server
```

### Ubuntu / Debian
```bash
sudo apt install lamco-rdp-server
systemctl --user enable --now lamco-rdp-server
```

## ✅ Tested Platforms

- ✅ **Ubuntu 24.04 LTS** (GNOME 46, Portal v5) - Fully functional
- ✅ **RHEL 9.7** (GNOME 40, Portal v4) - Fully functional (no clipboard)
- ⚠️ **Pop!_OS 24.04 COSMIC** - Limited support (no input)

## ⚠️ Known Issues

### Critical
- **Portal clipboard crash** (Ubuntu 24.04) - Complex Excel paste can crash xdg-desktop-portal-gnome
- **Session persistence rejected** (GNOME) - GNOME portal policy rejects persistence for RemoteDesktop

### Medium
- **No clipboard on RHEL 9** - Portal RemoteDesktop v1 lacks clipboard support
- **COSMIC no input** - Portal RemoteDesktop not implemented yet (Smithay PR #1388)

See [Distribution Testing Matrix](docs/DISTRO-TESTING-MATRIX.md) for complete compatibility details.

## 📖 Documentation

- [Architecture Overview](docs/architecture/)
- [Session Persistence Guide](docs/architecture/SESSION-PERSISTENCE-ARCHITECTURE.md)
- [Service Registry Technical Docs](docs/SERVICE-REGISTRY-TECHNICAL.md)
- [wlroots Implementation](docs/WLR-FULL-IMPLEMENTATION.md)

## 🔧 Building from Source

```bash
# Default build
cargo build --release

# With hardware encoding
cargo build --release --features hardware-encoding

# Flatpak features
cargo build --release --no-default-features --features "h264,libei"
```

## 📋 License

BUSL-1.1 (Business Source Use License)
- Free for non-profits and small businesses (<3 employees, <$1M revenue)
- Commercial license required for larger organizations
- Converts to Apache 2.0 on December 31, 2028

## 🙏 Acknowledgments

Built with:
- [IronRDP](https://github.com/Devolutions/IronRDP) - RDP protocol
- [tokio](https://tokio.rs/) - Async runtime
- [PipeWire](https://pipewire.org/) - Screen capture
- [ashpd](https://github.com/bilelmoussaoui/ashpd) - Portal bindings

## 📞 Support

- Website: https://lamco.io
- Email: office@lamco.io
- Issues: https://github.com/lamco-admin/lamco-rdp-server/issues
```

**Attach to release:**
- Source tarball (`lamco-rdp-server-0.1.0.tar.xz`)
- Flatpak bundle (`io.lamco.rdp-server.flatpak`)
- SHA256SUMS file

**Success Criteria:**
- ✅ Release created on GitHub
- ✅ Release notes complete
- ✅ Artifacts attached
- ✅ SHA256 sums provided

---

## Summary Timeline

| Phase | Duration | Tasks | Priority |
|-------|----------|-------|----------|
| **Phase 1: Critical Code Fixes** | 2-3 days | Portal crash fix, log levels | 🔴 CRITICAL |
| **Phase 2: Documentation Updates** | 1-2 days | README, service registry, website data | 🟡 HIGH |
| **Phase 3: Build System** | 1-2 days | Manifests, specs, scripts | 🟡 MEDIUM |
| **Phase 4: lamco-admin Publishing** | 3-5 days | Public repo, tarballs, packages | 🔴 CRITICAL |
| **Phase 5: Integration Testing** | 2-3 days | End-to-end validation | 🔴 CRITICAL |
| **Phase 6: Publication** | 1 day | Flathub, OBS, releases | 🟢 FINAL |

**TOTAL ESTIMATED TIME:** 10-16 days (2-3 weeks)

---

## Risks & Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Portal crash fix doesn't work | Medium | High | Extensive testing, fallback to single-lock with queuing |
| Flatpak rejection from Flathub | Low | Medium | Use self-hosted repo initially |
| Native package build failures | Medium | Medium | Test on all target distros, fix deps |
| Integration test failures | Medium | High | Allocate buffer time, prioritize critical platforms |
| Documentation incomplete | Low | Low | Review checklist before publication |

---

## Success Metrics

**Code Quality:**
- ✅ Portal crash bug fixed
- ✅ No critical errors in logs
- ✅ All features work as documented

**Documentation:**
- ✅ README.md complete and accurate
- ✅ Service registry docs updated
- ✅ Website publishing data generated

**Packaging:**
- ✅ Flatpak builds on Flathub
- ✅ RPM builds on Fedora/RHEL
- ✅ DEB builds on Ubuntu/Debian

**Publication:**
- ✅ Public repo accessible
- ✅ Packages available via package managers
- ✅ Release notes published

**Testing:**
- ✅ 2+ distros fully tested
- ✅ All features validated
- ✅ Known issues documented

---

## Next Steps After Publication

1. Monitor GitHub issues for user reports
2. Expand testing coverage (Ubuntu 22.04, Fedora 40, KDE, Sway)
3. Fix remaining known issues (clipboard crash, FUSE in Flatpak)
4. Prepare v0.2.0 with improvements
5. Submit IronRDP PRs for upstream merge

---

**END OF PLAN**

*This plan is ready for immediate execution. All tasks are well-defined, actionable, and prioritized.*
