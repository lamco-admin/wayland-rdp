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
tokio = { version = "1.35", features = [
    "full",           # All features
    "tracing",        # Integration with tracing
] }
tokio-util = { version = "0.7.10", features = ["codec"] }
tokio-stream = "0.1.14"
futures = "0.3.30"
async-trait = "0.1.77"

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
# RDP PROTOCOL
# Note: IronRDP versions - check crates.io for latest
# As of 2025-01, these are the expected package names
# Verify actual availability and update if needed
# ============================================================================
ironrdp = "0.1.0"  # Core RDP implementation
ironrdp-pdu = "0.1.0"  # PDU encoding/decoding
ironrdp-connector = "0.1.0"  # Connection handling
ironrdp-graphics = "0.1.0"  # Graphics pipeline

# Alternative if above are not available:
# Check https://github.com/Devolutions/IronRDP for current structure
# May need to use ironrdp with features instead

# ============================================================================
# VIDEO ENCODING - H.264
# ============================================================================
# OpenH264 (Cisco's encoder)
openh264 = { version = "0.6.0", features = ["encoder", "decoder"] }

# VA-API bindings for hardware acceleration
# Note: These may need updating based on availability
va = "0.7.0"  # VA-API Rust bindings
libva = "0.17.0"  # VA-API system bindings

# ============================================================================
# IMAGE PROCESSING & VIDEO UTILITIES
# ============================================================================
image = "0.25.0"  # Image format handling
yuv = "0.1.4"  # YUV format conversions

# ============================================================================
# TLS & SECURITY
# ============================================================================
rustls = { version = "0.23.4", features = ["dangerous_configuration"] }
rustls-pemfile = "2.1.0"  # PEM file parsing
tokio-rustls = "0.26.0"  # Tokio integration
webpki-roots = "0.26.0"  # Root certificates
x509-parser = "0.16.0"  # X.509 certificate parsing
rcgen = "0.13.1"  # Certificate generation
ring = "0.17.7"  # Cryptographic operations

# PAM authentication
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
serde = { version = "1.0.195", features = ["derive"] }
serde_json = "1.0.111"
toml = "0.8.10"
bincode = "1.3.3"  # Binary serialization

# ============================================================================
# ERROR HANDLING
# ============================================================================
anyhow = "1.0.79"  # Application errors
thiserror = "1.0.56"  # Library errors

# ============================================================================
# LOGGING & TRACING
# ============================================================================
tracing = "0.1.40"
tracing-subscriber = { version = "0.3.18", features = [
    "env-filter",
    "json",
    "fmt",
    "ansi",
] }
tracing-appender = "0.2.3"  # File rotation
tracing-log = "0.2.0"  # Log crate compatibility

# ============================================================================
# UTILITIES
# ============================================================================
bytes = "1.5.0"  # Byte buffer utilities
bitflags = "2.4.2"  # Bit flag macros
uuid = { version = "1.7.0", features = ["v4", "serde"] }
chrono = "0.4.33"  # Date/time handling
clap = { version = "4.5.0", features = ["derive", "env"] }  # CLI parsing
once_cell = "1.19.0"  # Lazy statics

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
    # xdg-desktop-portal
    xdg-desktop-portal \
    xdg-desktop-portal-gtk \
    libportal-dev \
    \
    # PipeWire (0.3.77+)
    libpipewire-0.3-dev \
    libspa-0.2-dev \
    pipewire \
    wireplumber \
    \
    # VA-API (hardware encoding)
    libva-dev \
    libva-drm2 \
    libdrm-dev \
    vainfo \
    # GPU drivers
    intel-media-va-driver \  # Intel GPUs
    mesa-va-drivers \         # AMD/Intel GPUs
    \
    # Video codecs
    libopenh264-dev \
    \
    # Security
    libpam0g-dev \
    libssl-dev \
    \
    # D-Bus
    libdbus-1-dev \
    \
    # Additional utilities
    git \
    curl
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
    # xdg-desktop-portal
    xdg-desktop-portal \
    xdg-desktop-portal-gtk \
    libportal-devel \
    \
    # PipeWire
    pipewire-devel \
    wireplumber \
    \
    # VA-API
    libva-devel \
    libdrm-devel \
    libva-intel-driver \
    mesa-va-drivers \
    \
    # Video codecs
    openh264-devel \
    \
    # Security
    pam-devel \
    openssl-devel \
    \
    # D-Bus
    dbus-devel \
    \
    # Utilities
    git \
    curl
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
    # xdg-desktop-portal
    xdg-desktop-portal \
    xdg-desktop-portal-gtk \
    \
    # PipeWire
    pipewire \
    wireplumber \
    \
    # VA-API
    libva \
    libva-mesa-driver \
    intel-media-driver \
    \
    # Video codecs
    openh264 \
    \
    # Security
    pam \
    openssl \
    \
    # D-Bus
    dbus \
    \
    # Utilities
    git \
    curl
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

set -e

echo "Verifying WRD-Server dependencies..."
echo "===================================="

# Rust version
echo -n "Checking Rust version... "
RUST_VERSION=$(rustc --version | awk '{print $2}')
echo "$RUST_VERSION"
if [ "$(printf '%s\n' "1.75.0" "$RUST_VERSION" | sort -V | head -n1)" != "1.75.0" ]; then
    echo "ERROR: Rust 1.75.0+ required"
    exit 1
fi
echo "✓ Rust version OK"

# PipeWire
echo -n "Checking PipeWire... "
if ! pkg-config --exists libpipewire-0.3; then
    echo "ERROR: PipeWire development files not found"
    exit 1
fi
PIPEWIRE_VERSION=$(pkg-config --modversion libpipewire-0.3)
echo "$PIPEWIRE_VERSION"
echo "✓ PipeWire found"

# VA-API
echo -n "Checking VA-API... "
if ! pkg-config --exists libva; then
    echo "WARNING: VA-API not found (hardware encoding disabled)"
else
    LIBVA_VERSION=$(pkg-config --modversion libva)
    echo "$LIBVA_VERSION"
    echo "✓ VA-API found"
fi

# Check if vainfo works
if command -v vainfo &> /dev/null; then
    echo -n "Checking VA-API driver... "
    if vainfo &> /dev/null; then
        echo "✓ VA-API driver loaded"
    else
        echo "WARNING: VA-API driver not available"
    fi
fi

# Wayland
echo -n "Checking Wayland... "
if ! pkg-config --exists wayland-client; then
    echo "ERROR: Wayland development files not found"
    exit 1
fi
WAYLAND_VERSION=$(pkg-config --modversion wayland-client)
echo "$WAYLAND_VERSION"
echo "✓ Wayland found"

# Runtime checks (if in graphical session)
if [ -n "$WAYLAND_DISPLAY" ]; then
    echo "Running in Wayland session"

    # PipeWire running
    echo -n "Checking PipeWire service... "
    if systemctl --user is-active --quiet pipewire; then
        echo "✓ Running"
    else
        echo "ERROR: PipeWire not running"
        exit 1
    fi

    # Portal running
    echo -n "Checking xdg-desktop-portal... "
    if systemctl --user is-active --quiet xdg-desktop-portal; then
        echo "✓ Running"
    else
        echo "WARNING: xdg-desktop-portal not running"
    fi
else
    echo "Not in Wayland session (skipping runtime checks)"
fi

echo ""
echo "===================================="
echo "All dependency checks passed!"
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

### IronRDP Package Issues
If IronRDP packages are not available as specified:
1. Check https://github.com/Devolutions/IronRDP
2. Clone repository and use as path dependency:
```toml
[dependencies]
ironrdp = { path = "../IronRDP" }
```

---

**END OF TECHNOLOGY STACK SPECIFICATION**

Proceed to task-specific documents for implementation details.
