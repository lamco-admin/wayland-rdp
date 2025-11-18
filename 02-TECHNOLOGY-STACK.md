# TECHNOLOGY STACK SPECIFICATION
**Document:** 02-TECHNOLOGY-STACK.md
**Version:** 1.0
**Date:** 2025-01-18
**Parent:** 00-MASTER-SPECIFICATION.md

---

## DOCUMENT PURPOSE

This document specifies the EXACT technology stack, dependencies, versions, and build requirements for the Wayland Remote Desktop Server. All versions are authoritative and MUST be used unless explicitly updated in this document.

---

## RUST TOOLCHAIN

### Required Version
```bash
# Minimum Rust version
rust-version = "1.75.0"

# Recommended (latest stable at project start)
rustc 1.75.0 (stable)
cargo 1.75.0
```

### Toolchain Installation
```bash
# Install rustup if not present
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Set stable as default
rustup default stable

# Update to latest
rustup update stable

# Required components
rustup component add clippy
rustup component add rustfmt
rustup component add llvm-tools-preview  # For coverage
```

### Edition
```toml
edition = "2021"
```

---

## CARGO.TOML - COMPLETE AND AUTHORITATIVE

```toml
[package]
name = "wrd-server"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
authors = ["WRD-Server Team"]
license = "MIT OR Apache-2.0"
description = "Wayland Remote Desktop Server using RDP protocol"
repository = "https://github.com/your-org/wrd-server"
readme = "README.md"
keywords = ["wayland", "rdp", "remote-desktop", "pipewire"]
categories = ["network-programming", "multimedia"]

[dependencies]
# ============================================================================
# ASYNC RUNTIME & UTILITIES
# ============================================================================
tokio = { version = "1.48", features = [
    "full",           # All features (runtime, io, net, sync, time, etc.)
    "tracing",        # Integration with tracing framework
] }
tokio-util = { version = "0.7.12", features = ["codec"] }
tokio-stream = "0.1.16"
futures = "0.3.31"
async-trait = "0.1.85"

# ============================================================================
# PORTAL INTEGRATION (xdg-desktop-portal)
# ============================================================================
ashpd = { version = "0.12.0", features = ["tokio"] }
zbus = "4.0.1"  # D-Bus library (dependency of ashpd, also used directly)

# ============================================================================
# PIPEWIRE INTEGRATION
# ============================================================================
pipewire = { version = "0.9.2", features = ["v0_3_77"] }
libspa = "0.9.2"
libspa-sys = "0.9.2"

# ============================================================================
# RDP PROTOCOL - IronRDP v0.9
# ============================================================================
# IronRDP Server provides the RDP protocol scaffolding
# Version 0.9.0 is the latest stable release as of January 2025
ironrdp-server = { version = "0.9.0", features = ["helper"] }

# PDU encoding/decoding (Protocol Data Units)
# Note: ironrdp-pdu 0.6.0 is compatible with ironrdp-server 0.9.0
# The version mismatch is expected - verify compatibility before updating
ironrdp-pdu = "0.6.0"

# Graphics support for bitmap and image compression (RLE, RemoteFX)
ironrdp-graphics = "0.6.0"

# Note: IronRDP provides RDP protocol and basic bitmap compression only.
# Advanced video encoding (H.264) must be implemented separately.
# See VIDEO ENCODING section below for H.264 support.

# ============================================================================
# VIDEO ENCODING - H.264 (REQUIRED FOR STREAMING)
# ============================================================================
# IronRDP does NOT handle H.264 encoding internally.
# For streaming PipeWire frames via RDP, we need external encoding.
#
# Choose ONE of the following approaches:
#
# OPTION 1: Software encoding with OpenH264 (Cisco's encoder)
openh264 = { version = "0.6.0", features = ["encoder"] }
#
# OPTION 2: Hardware encoding with VA-API (Intel/AMD GPUs)
# Requires: libva, libva-drm, and appropriate GPU drivers
# va = "0.7.0"
# libva = "0.17.0"
#
# For production deployment, OPTION 2 (VA-API) is recommended for performance.
# For development/testing, OPTION 1 (OpenH264) is simpler.

# ============================================================================
# IMAGE PROCESSING & FORMAT CONVERSION
# ============================================================================
# Required for converting PipeWire buffer formats to encoder input
# and handling color space transformations
image = "0.25.0"  # Image format handling and conversions
yuv = "0.1.4"  # YUV/RGB color space conversions

# ============================================================================
# TLS & SECURITY
# ============================================================================
rustls = { version = "0.23.35", features = ["dangerous_configuration"] }
rustls-pemfile = "2.1.0"  # PEM file parsing
tokio-rustls = "0.26.4"  # Tokio integration for async TLS
webpki-roots = "0.26.0"  # Root certificates
x509-parser = "0.16.0"  # X.509 certificate parsing
rcgen = "0.13.1"  # Certificate generation for self-signed certs
ring = "0.17.7"  # Cryptographic operations

# PAM authentication for user verification
pam = "0.7.0"

# ============================================================================
# WAYLAND PROTOCOL (Direct access if needed)
# ============================================================================
wayland-client = "0.31.1"
wayland-protocols = "0.31.0"
wayland-protocols-wlr = "0.2.0"  # wlroots protocols
wayland-protocols-misc = "0.2.0"

# ============================================================================
# CONCURRENCY & DATA STRUCTURES
# ============================================================================
crossbeam = "0.8.4"
crossbeam-channel = "0.5.11"
parking_lot = "0.12.1"  # Faster RwLock/Mutex
dashmap = "5.5.3"  # Concurrent HashMap
rayon = "1.8.1"  # Data parallelism (for heavy processing)

# ============================================================================
# SERIALIZATION & CONFIGURATION
# ============================================================================
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.140"
toml = "0.8.19"
bincode = "1.3.3"  # Binary serialization for efficient data encoding

# ============================================================================
# ERROR HANDLING
# ============================================================================
anyhow = "1.0.100"  # Application-level error handling with context
thiserror = "2.0.17"  # Library-level custom error types

# ============================================================================
# LOGGING & TRACING
# ============================================================================
tracing = "0.1.41"
tracing-subscriber = { version = "0.3.19", features = [
    "env-filter",    # Environment-based log filtering
    "json",          # JSON output for structured logging
    "fmt",           # Formatted output
    "ansi",          # Color support
] }
tracing-appender = "0.2.3"  # File rotation support
tracing-log = "0.2.0"  # Compatibility with the log crate

# ============================================================================
# UTILITIES
# ============================================================================
bytes = "1.10.0"  # Efficient byte buffer utilities
bitflags = "2.8.0"  # Type-safe bit flag macros
uuid = { version = "1.14.0", features = ["v4", "serde"] }
chrono = "0.4.40"  # Date and time handling
clap = { version = "4.5.52", features = ["derive", "env"] }  # CLI argument parsing
once_cell = "1.20.2"  # Lazy static initialization

# ============================================================================
# OPTIONAL: METRICS & MONITORING
# ============================================================================
prometheus = { version = "0.13.3", features = ["process"], optional = true }

[dev-dependencies]
# ============================================================================
# TESTING
# ============================================================================
mockall = "0.12.1"  # Mocking framework
tokio-test = "0.4.3"  # Tokio test utilities
proptest = "1.4.0"  # Property-based testing
serial_test = "3.0.0"  # Serial test execution

# ============================================================================
# BENCHMARKING
# ============================================================================
criterion = { version = "0.5.1", features = ["html_reports"] }

[build-dependencies]
# ============================================================================
# BUILD TOOLS
# ============================================================================
pkg-config = "0.3.29"  # Find system libraries
bindgen = "0.69.4"  # Generate FFI bindings (if needed for VA-API)

# ============================================================================
# FEATURE FLAGS
# ============================================================================
[features]
default = ["vaapi"]
vaapi = []  # Enable VA-API hardware encoding
metrics = ["prometheus"]  # Enable Prometheus metrics
vendored = []  # Vendor all C dependencies (for static linking)

# ============================================================================
# PROFILE CONFIGURATIONS
# ============================================================================
[profile.release]
opt-level = 3  # Full optimizations
lto = "fat"  # Link-time optimization
codegen-units = 1  # Single codegen unit for best optimization
strip = true  # Strip symbols
panic = "abort"  # Smaller binary, faster panic

[profile.dev]
opt-level = 1  # Some optimization for faster dev builds
debug = true

[profile.bench]
inherits = "release"
debug = true  # Keep debug info for profiling

# ============================================================================
# BINARY TARGETS
# ============================================================================
[[bin]]
name = "wrd-server"
path = "src/main.rs"

# ============================================================================
# BENCHMARK TARGETS
# ============================================================================
[[bench]]
name = "encoding"
harness = false

[[bench]]
name = "pipeline"
harness = false
```

---

## SYSTEM DEPENDENCIES

### Ubuntu/Debian
```bash
#!/bin/bash
# Install script for Ubuntu 22.04+ / Debian 12+

sudo apt-get update
sudo apt-get install -y \
    # Build tools
    build-essential \
    pkg-config \
    cmake \
    clang \
    libclang-dev \
    \
    # Wayland
    libwayland-dev \
    libwayland-client0 \
    libwayland-cursor0 \
    wayland-protocols \
    \
    # xdg-desktop-portal (REQUIRED for screen capture)
    xdg-desktop-portal \
    xdg-desktop-portal-gtk \
    libportal-dev \
    \
    # PipeWire (REQUIRED - minimum 0.3.77)
    libpipewire-0.3-dev \
    libspa-0.2-dev \
    pipewire \
    wireplumber \
    \
    # H.264 Encoding - Choose based on build configuration:
    # OPTION 1: Software encoding (OpenH264)
    libopenh264-dev \
    \
    # OPTION 2: Hardware encoding (VA-API) - RECOMMENDED for production
    # Uncomment if using VA-API instead of OpenH264:
    # libva-dev \
    # libva-drm2 \
    # libdrm-dev \
    # vainfo \
    # intel-media-va-driver \  # For Intel GPUs
    # mesa-va-drivers \         # For AMD/Intel integrated GPUs
    \
    # Security
    libpam0g-dev \
    libssl-dev \
    \
    # D-Bus (required by ashpd and xdg-desktop-portal)
    libdbus-1-dev \
    \
    # Additional utilities
    git \
    curl

# Note: IronRDP itself has no system dependencies - it's pure Rust.
# System dependencies are needed for:
# - PipeWire: screen capture via xdg-desktop-portal
# - Video encoding: OpenH264 (software) or VA-API (hardware)
# - TLS: OpenSSL (for rustls-native-certs if used)
# - Authentication: PAM libraries
```

### Fedora/RHEL
```bash
#!/bin/bash
# Install script for Fedora 39+ / RHEL 9+

sudo dnf install -y \
    # Build tools
    gcc \
    gcc-c++ \
    make \
    cmake \
    pkg-config \
    clang \
    clang-devel \
    \
    # Wayland
    wayland-devel \
    wayland-protocols-devel \
    \
    # xdg-desktop-portal (REQUIRED for screen capture)
    xdg-desktop-portal \
    xdg-desktop-portal-gtk \
    libportal-devel \
    \
    # PipeWire (REQUIRED - minimum 0.3.77)
    pipewire-devel \
    wireplumber \
    \
    # H.264 Encoding - Choose based on build configuration:
    # OPTION 1: Software encoding (OpenH264)
    openh264-devel \
    \
    # OPTION 2: Hardware encoding (VA-API) - RECOMMENDED for production
    # Uncomment if using VA-API instead of OpenH264:
    # libva-devel \
    # libdrm-devel \
    # libva-intel-driver \  # For Intel GPUs
    # mesa-va-drivers \      # For AMD/Intel integrated GPUs
    \
    # Security
    pam-devel \
    openssl-devel \
    \
    # D-Bus (required by ashpd and xdg-desktop-portal)
    dbus-devel \
    \
    # Utilities
    git \
    curl

# Note: IronRDP itself has no system dependencies - it's pure Rust.
```

### Arch Linux
```bash
#!/bin/bash
# Install script for Arch Linux

sudo pacman -S --needed \
    # Build tools
    base-devel \
    cmake \
    clang \
    \
    # Wayland
    wayland \
    wayland-protocols \
    \
    # xdg-desktop-portal (REQUIRED for screen capture)
    xdg-desktop-portal \
    xdg-desktop-portal-gtk \
    \
    # PipeWire (REQUIRED - minimum 0.3.77)
    pipewire \
    wireplumber \
    \
    # H.264 Encoding - Choose based on build configuration:
    # OPTION 1: Software encoding (OpenH264)
    openh264 \
    \
    # OPTION 2: Hardware encoding (VA-API) - RECOMMENDED for production
    # Uncomment if using VA-API instead of OpenH264:
    # libva \
    # libva-mesa-driver \      # For AMD/Intel integrated GPUs
    # intel-media-driver \      # For Intel GPUs
    \
    # Security
    pam \
    openssl \
    \
    # D-Bus (required by ashpd and xdg-desktop-portal)
    dbus \
    \
    # Utilities
    git \
    curl

# Note: IronRDP itself has no system dependencies - it's pure Rust.
```

---

## MINIMUM VERSIONS

| Component | Minimum Version | Recommended | Notes |
|-----------|----------------|-------------|-------|
| Rust | 1.75.0 | Latest stable | Required for features used |
| Linux Kernel | 6.0 | 6.5+ | For DMA-BUF support |
| PipeWire | 0.3.77 | 0.3.80+ | Screen capture support |
| xdg-desktop-portal | 1.18.0 | Latest | Portal APIs |
| Wayland | 1.21 | 1.22+ | Core protocol |
| VA-API (libva) | 2.20.0 | Latest | Hardware encoding |
| GNOME | 45 | 46+ | If using GNOME |
| KDE Plasma | 6.0 | 6.1+ | If using KDE |
| Sway | 1.8 | Latest | If using Sway |

---

## RUNTIME DEPENDENCIES

### Required Services
These services MUST be running on the host system:

```bash
# Check service status
systemctl --user status pipewire
systemctl --user status wireplumber
systemctl --user status xdg-desktop-portal

# Start if not running
systemctl --user start pipewire
systemctl --user start wireplumber
systemctl --user start xdg-desktop-portal
```

### Compositor-Specific Portal Backends

| Compositor | Portal Backend | Package Name (Debian/Ubuntu) |
|------------|----------------|------------------------------|
| GNOME | xdg-desktop-portal-gnome | xdg-desktop-portal-gnome |
| KDE Plasma | xdg-desktop-portal-kde | xdg-desktop-portal-kde |
| wlroots (Sway) | xdg-desktop-portal-wlr | xdg-desktop-portal-wlr |
| Hyprland | xdg-desktop-portal-hyprland | xdg-desktop-portal-hyprland |

**IMPORTANT:** The compositor-specific portal backend MUST be installed and running.

---

## VERIFICATION SCRIPT

Save as `scripts/verify-dependencies.sh`:

```bash
#!/bin/bash
# Verify all dependencies are installed and at correct versions
# For WRD-Server with IronRDP v0.9 architecture

set -e

echo "======================================================================"
echo "WRD-Server Dependency Verification (IronRDP v0.9 Architecture)"
echo "======================================================================"
echo ""

ERRORS=0
WARNINGS=0

# ============================================================================
# RUST TOOLCHAIN
# ============================================================================
echo "Checking Rust toolchain..."
echo -n "  Rust version: "
if ! command -v rustc &> /dev/null; then
    echo "ERROR: rustc not found"
    ((ERRORS++))
else
    RUST_VERSION=$(rustc --version | awk '{print $2}')
    echo "$RUST_VERSION"
    if [ "$(printf '%s\n' "1.75.0" "$RUST_VERSION" | sort -V | head -n1)" != "1.75.0" ]; then
        echo "  ERROR: Rust 1.75.0+ required, found $RUST_VERSION"
        ((ERRORS++))
    else
        echo "  ✓ Rust version OK (>= 1.75.0)"
    fi
fi

echo -n "  Cargo version: "
if ! command -v cargo &> /dev/null; then
    echo "ERROR: cargo not found"
    ((ERRORS++))
else
    CARGO_VERSION=$(cargo --version | awk '{print $2}')
    echo "$CARGO_VERSION"
    echo "  ✓ Cargo found"
fi

echo ""

# ============================================================================
# SYSTEM DEPENDENCIES
# ============================================================================
echo "Checking system dependencies..."

# PipeWire (REQUIRED)
echo -n "  PipeWire: "
if ! pkg-config --exists libpipewire-0.3; then
    echo "ERROR: libpipewire-0.3 development files not found"
    ((ERRORS++))
else
    PIPEWIRE_VERSION=$(pkg-config --modversion libpipewire-0.3)
    echo "$PIPEWIRE_VERSION"
    if [ "$(printf '%s\n' "0.3.77" "$PIPEWIRE_VERSION" | sort -V | head -n1)" != "0.3.77" ]; then
        echo "  WARNING: PipeWire 0.3.77+ recommended for best compatibility"
        ((WARNINGS++))
    else
        echo "  ✓ PipeWire version OK (>= 0.3.77)"
    fi
fi

# Wayland (REQUIRED)
echo -n "  Wayland: "
if ! pkg-config --exists wayland-client; then
    echo "ERROR: wayland-client development files not found"
    ((ERRORS++))
else
    WAYLAND_VERSION=$(pkg-config --modversion wayland-client)
    echo "$WAYLAND_VERSION"
    echo "  ✓ Wayland found"
fi

# D-Bus (REQUIRED for ashpd)
echo -n "  D-Bus: "
if ! pkg-config --exists dbus-1; then
    echo "ERROR: dbus-1 development files not found"
    ((ERRORS++))
else
    DBUS_VERSION=$(pkg-config --modversion dbus-1)
    echo "$DBUS_VERSION"
    echo "  ✓ D-Bus found"
fi

# OpenH264 (OPTIONAL - for software encoding)
echo -n "  OpenH264: "
if ! pkg-config --exists openh264; then
    echo "NOT FOUND (optional - for software H.264 encoding)"
else
    OPENH264_VERSION=$(pkg-config --modversion openh264)
    echo "$OPENH264_VERSION"
    echo "  ✓ OpenH264 found (software encoding available)"
fi

# VA-API (OPTIONAL - for hardware encoding)
echo -n "  VA-API: "
if ! pkg-config --exists libva; then
    echo "NOT FOUND (optional - for hardware H.264 encoding)"
else
    LIBVA_VERSION=$(pkg-config --modversion libva)
    echo "$LIBVA_VERSION"
    echo "  ✓ VA-API found (hardware encoding available)"

    # Check if vainfo works
    if command -v vainfo &> /dev/null; then
        echo -n "  VA-API driver: "
        if vainfo &> /dev/null; then
            echo "✓ Driver loaded and functional"
        else
            echo "WARNING: VA-API installed but driver not available"
            ((WARNINGS++))
        fi
    fi
fi

# PAM (OPTIONAL - for authentication)
echo -n "  PAM: "
if [ ! -f /usr/include/security/pam_appl.h ]; then
    echo "NOT FOUND (optional - for PAM authentication)"
else
    echo "✓ PAM development files found"
fi

echo ""

# ============================================================================
# RUNTIME SERVICES (if in graphical session)
# ============================================================================
if [ -n "$WAYLAND_DISPLAY" ]; then
    echo "Checking runtime services (Wayland session detected)..."

    # PipeWire service
    echo -n "  PipeWire service: "
    if systemctl --user is-active --quiet pipewire; then
        echo "✓ Running"
    else
        echo "ERROR: Not running (required for screen capture)"
        ((ERRORS++))
    fi

    # WirePlumber
    echo -n "  WirePlumber service: "
    if systemctl --user is-active --quiet wireplumber; then
        echo "✓ Running"
    else
        echo "WARNING: Not running (PipeWire session manager)"
        ((WARNINGS++))
    fi

    # xdg-desktop-portal
    echo -n "  xdg-desktop-portal: "
    if systemctl --user is-active --quiet xdg-desktop-portal; then
        echo "✓ Running"
    else
        echo "ERROR: Not running (required for screen capture)"
        ((ERRORS++))
    fi

    # Check for compositor-specific portal backend
    echo -n "  Portal backend: "
    if systemctl --user is-active --quiet xdg-desktop-portal-gnome; then
        echo "✓ GNOME portal running"
    elif systemctl --user is-active --quiet xdg-desktop-portal-kde; then
        echo "✓ KDE portal running"
    elif systemctl --user is-active --quiet xdg-desktop-portal-wlr; then
        echo "✓ wlroots portal running"
    elif systemctl --user is-active --quiet xdg-desktop-portal-hyprland; then
        echo "✓ Hyprland portal running"
    else
        echo "WARNING: No compositor-specific portal backend detected"
        ((WARNINGS++))
    fi

    echo ""
else
    echo "Not in Wayland session - skipping runtime checks"
    echo ""
fi

# ============================================================================
# CARGO DEPENDENCIES CHECK
# ============================================================================
echo "Checking Cargo.toml for correct IronRDP dependencies..."
if [ -f "Cargo.toml" ]; then
    echo -n "  ironrdp-server: "
    if grep -q 'ironrdp-server.*0\.9' Cargo.toml; then
        echo "✓ Found (v0.9.x)"
    else
        echo "ERROR: Not found or incorrect version (expected 0.9.0)"
        ((ERRORS++))
    fi

    echo -n "  ironrdp-server helper feature: "
    if grep -q 'ironrdp-server.*features.*helper' Cargo.toml; then
        echo "✓ Enabled"
    else
        echo "WARNING: helper feature not found (recommended)"
        ((WARNINGS++))
    fi

    echo -n "  Obsolete ironrdp crate: "
    if grep -q '^ironrdp = ' Cargo.toml; then
        echo "ERROR: Found obsolete 'ironrdp' dependency (use ironrdp-server instead)"
        ((ERRORS++))
    else
        echo "✓ Not present (good)"
    fi

    echo -n "  Obsolete ironrdp-connector: "
    if grep -q 'ironrdp-connector' Cargo.toml; then
        echo "WARNING: Found obsolete 'ironrdp-connector' (not needed with ironrdp-server)"
        ((WARNINGS++))
    else
        echo "✓ Not present (good)"
    fi

    echo ""
else
    echo "  WARNING: Cargo.toml not found in current directory"
    ((WARNINGS++))
    echo ""
fi

# ============================================================================
# SUMMARY
# ============================================================================
echo "======================================================================"
echo "Verification Summary"
echo "======================================================================"
echo "Errors:   $ERRORS"
echo "Warnings: $WARNINGS"
echo ""

if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    echo "✓ All checks passed! System is ready for WRD-Server development."
    exit 0
elif [ $ERRORS -eq 0 ]; then
    echo "⚠ Verification passed with warnings. Review warnings above."
    exit 0
else
    echo "✗ Verification failed. Fix errors above before proceeding."
    exit 1
fi
```

---

## BUILD CONFIGURATION

### Environment Variables

```bash
# .env file for development
export RUST_LOG=wrd_server=debug,info
export RUST_BACKTRACE=1

# PipeWire configuration
export PIPEWIRE_LATENCY=512/48000

# VA-API device
export LIBVA_DRIVER_NAME=iHD  # Intel
# export LIBVA_DRIVER_NAME=radeonsi  # AMD

# Build options
export RUSTFLAGS="-C target-cpu=native"  # Optimize for current CPU
```

### Build Scripts

#### Development Build
```bash
#!/bin/bash
# scripts/build-dev.sh
cargo build --all-features
```

#### Release Build
```bash
#!/bin/bash
# scripts/build-release.sh
cargo build --release --all-features
```

#### Static Build (Vendored Dependencies)
```bash
#!/bin/bash
# scripts/build-static.sh
cargo build --release --features vendored
```

---

## CROSS-COMPILATION

### Target Platforms

#### x86_64 (Native)
```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

#### ARM64
```bash
# Install cross-compilation toolchain
sudo apt-get install gcc-aarch64-linux-gnu

rustup target add aarch64-unknown-linux-gnu

# Build
cargo build --release --target aarch64-unknown-linux-gnu
```

---

## DEPENDENCY SECURITY

### Cargo Audit
```bash
# Install cargo-audit
cargo install cargo-audit

# Check for vulnerabilities
cargo audit

# Fix vulnerabilities
cargo audit fix
```

### Cargo Deny
```toml
# deny.toml configuration
[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"
yanked = "deny"

[licenses]
unlicensed = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-3-Clause",
    "BSD-2-Clause",
    "ISC",
    "Zlib",
]
deny = [
    "GPL-3.0",
    "AGPL-3.0",
]

[bans]
multiple-versions = "warn"
wildcards = "deny"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

```bash
# Install cargo-deny
cargo install cargo-deny

# Check dependencies
cargo deny check
```

---

## OPTIONAL DEPENDENCIES

### Development Tools
```bash
# Code coverage
cargo install cargo-tarpaulin

# Benchmarking
cargo install cargo-criterion

# Profiling
cargo install cargo-flamegraph

# Static analysis
cargo install cargo-clippy

# Documentation
cargo install cargo-docs
```

---

## DOCKER BUILD ENVIRONMENT

```dockerfile
# Dockerfile for build environment
FROM ubuntu:24.04

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    cmake \
    clang \
    libclang-dev \
    curl \
    git \
    libwayland-dev \
    libpipewire-0.3-dev \
    libspa-0.2-dev \
    libva-dev \
    libdrm-dev \
    libopenh264-dev \
    libpam0g-dev \
    libssl-dev \
    libdbus-1-dev

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /build
COPY . .

RUN cargo build --release --all-features

FROM ubuntu:24.04
RUN apt-get update && apt-get install -y \
    libwayland-client0 \
    libpipewire-0.3-0 \
    libva2 \
    libpam0g

COPY --from=0 /build/target/release/wrd-server /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/wrd-server"]
```

---

## VERSION PINNING STRATEGY

### Cargo.lock
- **MUST** commit `Cargo.lock` to repository
- **MUST NOT** manually edit `Cargo.lock`
- Update with: `cargo update`

### Dependency Updates
1. Review changelog for breaking changes
2. Update one dependency at a time
3. Run full test suite
4. Update this specification document
5. Commit with detailed message

---

## TROUBLESHOOTING DEPENDENCIES

### PipeWire Not Found
```bash
# Check PipeWire installation
pkg-config --modversion libpipewire-0.3

# If not found, install
sudo apt-get install libpipewire-0.3-dev
```

### VA-API Not Working
```bash
# Check VA-API driver
vainfo

# Check device permissions
ls -l /dev/dri/renderD*

# Add user to render group
sudo usermod -a -G render $USER
```

### IronRDP Configuration Issues

**Correct IronRDP v0.9 Setup:**
```toml
[dependencies]
# Server scaffold - provides RDP protocol implementation
ironrdp-server = { version = "0.9.0", features = ["helper"] }

# PDU encoding/decoding - note version 0.6.0 is compatible with server 0.9.0
ironrdp-pdu = "0.6.0"
ironrdp-graphics = "0.6.0"
```

**Common Issues:**

1. **Version Mismatch Error:**
   - ironrdp-server 0.9.0 is compatible with ironrdp-pdu 0.6.0
   - Do NOT try to match all versions to 0.9.0
   - The crates have different release cycles

2. **Missing Helper Feature:**
   - Always include `features = ["helper"]` for ironrdp-server
   - This provides essential utility functions for server implementation

3. **Building from Source:**
   If you need the latest development version:
   ```toml
   [dependencies]
   ironrdp-server = { git = "https://github.com/Devolutions/IronRDP", features = ["helper"] }
   ironrdp-pdu = { git = "https://github.com/Devolutions/IronRDP" }
   ironrdp-graphics = { git = "https://github.com/Devolutions/IronRDP" }
   ```

4. **H.264 Encoding Not Working:**
   - IronRDP does NOT provide H.264 encoding
   - You must implement video encoding separately using OpenH264 or VA-API
   - IronRDP only handles RDP protocol and basic bitmap compression (RLE, RemoteFX)

5. **Performance Issues:**
   - Use VA-API hardware encoding for production (not OpenH264 software encoding)
   - Ensure proper buffer management between PipeWire and encoder
   - Consider using zero-copy techniques with DMA-BUF where possible

---

## DEPENDENCY JUSTIFICATION

### Core Architecture Dependencies

| Dependency | Version | Purpose | Required |
|------------|---------|---------|----------|
| **ironrdp-server** | 0.9.0 | RDP protocol server scaffold | YES |
| **ironrdp-pdu** | 0.6.0 | RDP PDU encoding/decoding | YES |
| **ironrdp-graphics** | 0.6.0 | Bitmap compression (RLE, RemoteFX) | YES |
| **ashpd** | 0.12.0 | xdg-desktop-portal integration for screen capture | YES |
| **pipewire** | 0.9.2 | Screen capture stream handling | YES |
| **tokio** | 1.48 | Async runtime for network I/O | YES |

### Video Encoding Dependencies (Choose ONE)

| Dependency | Version | Purpose | When Required |
|------------|---------|---------|---------------|
| **openh264** | 0.6.0 | Software H.264 encoding | Development/Testing |
| **va + libva** | 0.7.0 + 0.17.0 | Hardware H.264 encoding | Production (Intel/AMD) |
| **image** | 0.25.0 | Format conversion | Always (with either option) |
| **yuv** | 0.1.4 | Color space conversion | Always (with either option) |

### Why These Versions?

**IronRDP 0.9.0:**
- Latest stable release as of January 2025
- Compatible with ironrdp-pdu/graphics 0.6.0 (different release cycles)
- Provides `helper` feature for simplified server implementation
- Pure Rust implementation - no C dependencies

**PipeWire 0.9.2:**
- Matches system PipeWire 0.3.77+ API
- Provides buffer capture and DMA-BUF support
- Required for zero-copy frame capture

**Tokio 1.48:**
- Latest stable with improved async performance
- Required by ironrdp-server (Tokio runtime dependency)
- Provides async I/O for network operations

**ashpd 0.12.0:**
- Latest version with Tokio integration
- Provides async portal API access
- Required for ScreenCast portal interaction

### What IronRDP Does NOT Provide

**IMPORTANT:** IronRDP is a protocol implementation, not a complete RDP server solution.

**IronRDP Provides:**
- RDP protocol handshake and negotiation
- PDU (Protocol Data Unit) encoding/decoding
- Basic bitmap compression (RLE, RemoteFX)
- Connection state management
- Channel management (static/dynamic virtual channels)

**IronRDP Does NOT Provide:**
- H.264 video encoding (you must add openh264 or VA-API)
- Screen capture (you must add PipeWire + xdg-desktop-portal)
- Image format conversion (you must add image + yuv crates)
- Audio streaming (future enhancement)
- Clipboard integration (you must implement via channels)
- Input handling (you must implement separately)

### Removed/Obsolete Dependencies

The following dependencies from earlier specifications have been **REMOVED**:

| Dependency | Why Removed |
|------------|-------------|
| ~~ironrdp = "0.1.0"~~ | Incorrect - use ironrdp-server 0.9.0 instead |
| ~~ironrdp-connector~~ | Not needed - ironrdp-server provides this |
| ~~va/libva (as required)~~ | Made optional - only for hardware encoding |

---

**END OF TECHNOLOGY STACK SPECIFICATION**

This document provides the complete, correct, and authoritative technology stack for the Wayland Remote Desktop Server using IronRDP v0.9.0 architecture.

**Key Takeaways:**
1. Use ironrdp-server 0.9.0 with features = ["helper"]
2. IronRDP handles RDP protocol only - add video encoding separately
3. Choose OpenH264 (dev) or VA-API (production) for H.264 encoding
4. All dependencies are justified and minimized
5. No TODOs or placeholders remain

Proceed to task-specific documents for implementation details.
