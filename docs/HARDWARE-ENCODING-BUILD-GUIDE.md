# Hardware Encoding Build & Distribution Guide

**Document Version:** 1.0
**Last Updated:** 2025-12-26
**Status:** Production Ready (Implementation Complete)

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture Summary](#architecture-summary)
3. [Feature Flags](#feature-flags)
4. [Build Requirements](#build-requirements)
   - [VAAPI (Intel/AMD)](#vaapi-build-requirements)
   - [NVENC (NVIDIA)](#nvenc-build-requirements)
5. [Runtime Requirements](#runtime-requirements)
6. [Building Binaries](#building-binaries)
7. [Binary Distribution Strategy](#binary-distribution-strategy)
8. [Configuration Reference](#configuration-reference)
9. [Troubleshooting](#troubleshooting)
10. [Testing Checklist](#testing-checklist)

---

## Overview

The lamco-rdp-server includes hardware-accelerated H.264 encoding for significant CPU reduction (50-70%) when streaming remote desktops. Two backends are supported:

| Backend | GPU Vendor | Technology | Typical CPUs |
|---------|------------|------------|--------------|
| **VAAPI** | Intel, AMD | VA-API 1.0+ | Intel HD 4000+, AMD GCN+ |
| **NVENC** | NVIDIA | Video Codec SDK | Kepler (GTX 600) and newer |

**Key Benefits:**
- 50-70% CPU reduction vs software encoding (OpenH264)
- Often better quality (hardware encoders tuned by GPU vendors)
- Enables 4K streaming on modest hardware
- Essential for multi-user/multi-session deployments

---

## Architecture Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                    HardwareEncoder Trait                        │
│  encode_bgra() -> H264Frame                                     │
│  force_keyframe(), stats(), backend_name()                      │
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  VaapiEncoder   │  │  NvencEncoder   │  │  (Software)     │
│  (864 lines)    │  │  (738 lines)    │  │  OpenH264       │
│                 │  │                 │  │                 │
│  Intel iHD/i965 │  │  CUDA + NVENC   │  │  Fallback       │
│  AMD radeonsi   │  │  Video Codec SDK│  │                 │
└─────────────────┘  └─────────────────┘  └─────────────────┘
```

**Source Files:**

| File | Lines | Purpose |
|------|-------|---------|
| `src/egfx/hardware/mod.rs` | 343 | Core trait definitions, types |
| `src/egfx/hardware/factory.rs` | 307 | Backend selection, auto-detection |
| `src/egfx/hardware/error.rs` | 278 | Error types (unified + backend-specific) |
| `src/egfx/hardware/stats.rs` | 254 | Performance monitoring |
| `src/egfx/hardware/vaapi/mod.rs` | 864 | VA-API implementation |
| `src/egfx/hardware/nvenc/mod.rs` | 738 | NVENC implementation |

---

## Feature Flags

The hardware encoding backends are **optional** and controlled via Cargo features:

```toml
# Cargo.toml feature definitions

[features]
default = ["pam-auth", "h264"]

# Software H.264 encoding (always available fallback)
h264 = ["openh264", "openh264-sys2"]

# VA-API: Intel (iHD/i965) and AMD (radeonsi) GPU encoding
vaapi = ["cros-libva"]

# NVENC: NVIDIA GPU encoding via Video Codec SDK
nvenc = ["nvidia-video-codec-sdk", "cudarc"]

# Convenience: enable all hardware backends
hardware-encoding = ["vaapi", "nvenc"]
```

### Build Command Examples

```bash
# Software only (smallest binary, maximum compatibility)
cargo build --release --features h264

# VAAPI only (Intel/AMD GPUs)
cargo build --release --features "h264,vaapi"

# NVENC only (NVIDIA GPUs)
cargo build --release --features "h264,nvenc"

# All hardware backends (maximum hardware support)
cargo build --release --features "h264,hardware-encoding"

# Full build with all features
cargo build --release --features "h264,hardware-encoding,pam-auth"
```

---

## Build Requirements

### Common Requirements (All Builds)

```bash
# Rust toolchain (1.77+)
rustup update stable

# Build essentials
sudo apt-get install build-essential pkg-config

# For OpenH264 (software fallback) - 3x faster with NASM
sudo apt-get install nasm
```

### VAAPI Build Requirements

**Compile-time dependencies:**

```bash
# Debian/Ubuntu
sudo apt-get install libva-dev libdrm-dev

# Fedora/RHEL
sudo dnf install libva-devel libdrm-devel

# Arch Linux
sudo pacman -S libva libdrm
```

**Minimum versions:**
- libva >= 1.20.0 (for H.264 encoding support)
- libdrm >= 2.4.0

**Verification:**

```bash
# Check libva version
pkg-config --modversion libva
# Should output: 2.20.0 or higher

# Check header availability
ls /usr/include/va/va.h
```

**Crate dependency:**
- `cros-libva = "0.0.13"` (ChromeOS VA-API bindings)

### NVENC Build Requirements

**CRITICAL: NVENC requires both CUDA toolkit AND NVIDIA driver with encode support.**

#### Step 1: NVIDIA Driver (Runtime + Headers)

The NVIDIA driver must be installed with `libnvidia-encode.so`:

```bash
# Check if NVIDIA driver is installed
nvidia-smi

# Check for encode library
ldconfig -p | grep libnvidia-encode
# Should output: libnvidia-encode.so.1 -> libnvidia-encode.so.XXX.XX.XX
```

#### Step 2: CUDA Toolkit Installation

**Option A: NVIDIA Official Repository (Recommended)**

```bash
# Debian 13 (Trixie)
wget https://developer.download.nvidia.com/compute/cuda/repos/debian13/x86_64/cuda-keyring_1.1-1_all.deb
sudo dpkg -i cuda-keyring_1.1-1_all.deb
sudo apt-get update
sudo apt-get install cuda-toolkit-13-1

# Create symlink
sudo ln -sf /usr/local/cuda-13.1 /usr/local/cuda

# Debian 12 (Bookworm)
wget https://developer.download.nvidia.com/compute/cuda/repos/debian12/x86_64/cuda-keyring_1.1-1_all.deb
sudo dpkg -i cuda-keyring_1.1-1_all.deb
sudo apt-get update
sudo apt-get install cuda-toolkit-12-6

# Ubuntu 22.04/24.04
wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.1-1_all.deb
sudo dpkg -i cuda-keyring_1.1-1_all.deb
sudo apt-get update
sudo apt-get install cuda-toolkit-12-6
```

**Option B: Runfile Installer (Manual)**

Download from: https://developer.nvidia.com/cuda-downloads

```bash
# Example for CUDA 13.1
wget https://developer.download.nvidia.com/compute/cuda/13.1.0/local_installers/cuda_13.1.0_550.54.14_linux.run
sudo sh cuda_13.1.0_550.54.14_linux.run --toolkit --silent
```

#### Step 3: Environment Variables

**For building (compile-time):**

```bash
# Add to ~/.bashrc or build script
export PATH=/usr/local/cuda/bin:$PATH
export LD_LIBRARY_PATH=/usr/local/cuda/lib64:$LD_LIBRARY_PATH
export CUDA_PATH=/usr/local/cuda

# CRITICAL: Override for CUDA 13.x compatibility with cudarc
# cudarc 0.16.x only supports up to CUDA 12.9, but 13.x is backward compatible
export CUDARC_CUDA_VERSION=12090
```

**Verification:**

```bash
# Verify nvcc (CUDA compiler)
nvcc --version
# Should output: Cuda compilation tools, release 13.1, V13.1.80

# Verify CUDA libraries
ls /usr/local/cuda/lib64/libcudart.so
```

#### CUDA Version Compatibility Matrix

| CUDA Version | cudarc Support | CUDARC_CUDA_VERSION Override |
|--------------|----------------|------------------------------|
| 11.4 - 11.8 | Native | Not needed |
| 12.0 - 12.9 | Native | Not needed |
| 13.0 - 13.1 | Via override | `12090` |
| 13.2+ | Unknown | Test with `12090` |

**Note:** The `CUDARC_CUDA_VERSION=12090` override works because CUDA 13.x maintains backward compatibility with 12.x APIs. The NVENC API is stable across versions.

#### Crate Dependencies

```toml
# In Cargo.toml
nvidia-video-codec-sdk = { version = "0.4", optional = true }
cudarc = { version = "0.16", optional = true, default-features = false,
           features = ["driver", "cuda-version-from-build-system"] }
```

---

## Runtime Requirements

### VAAPI Runtime Requirements

Users need these installed on their system:

```bash
# Core VA-API runtime
sudo apt-get install libva2 libva-drm2

# Intel GPU drivers (choose based on generation)
# - Intel Gen 8+ (Broadwell and newer): intel-media-va-driver (iHD)
# - Intel Gen 4-7 (Sandy Bridge to Haswell): i965-va-driver
sudo apt-get install intel-media-va-driver  # or i965-va-driver

# AMD GPU drivers (included in Mesa)
sudo apt-get install mesa-va-drivers

# Verification tool
sudo apt-get install vainfo
vainfo  # Lists supported profiles
```

**Required VA-API profiles:**
- `VAProfileH264Main` or `VAProfileH264High`
- `VAEntrypointEncSlice`

**DRM render node access:**
```bash
# User must have access to /dev/dri/renderD128
# Usually via 'video' or 'render' group
sudo usermod -aG video $USER
sudo usermod -aG render $USER  # Some distros
```

### NVENC Runtime Requirements

Users need these installed:

```bash
# NVIDIA driver with encode support (usually included)
# Minimum driver version: 470.x for basic support
# Recommended: 525.x or newer for best compatibility

# Verify encode library exists
ldconfig -p | grep libnvidia-encode
# Must show: libnvidia-encode.so.1

# Verify GPU is accessible
nvidia-smi
```

**CUDA Runtime (NOT full toolkit):**

For runtime, users only need the CUDA runtime libraries, not the full toolkit:

```bash
# Option 1: Install minimal CUDA runtime
sudo apt-get install nvidia-cuda-toolkit  # Includes runtime

# Option 2: Just the runtime (smaller)
# The NVIDIA driver usually includes necessary runtime
```

**NVIDIA device nodes:**
```bash
# These must exist:
/dev/nvidia0        # GPU device
/dev/nvidiactl      # Control device
/dev/nvidia-uvm     # Unified memory (optional)
```

---

## Building Binaries

### Development Build

```bash
cd /path/to/lamco-rdp-server

# Set environment for NVENC
export PATH=/usr/local/cuda/bin:$PATH
export CUDA_PATH=/usr/local/cuda
export CUDARC_CUDA_VERSION=12090

# Build with all hardware backends
cargo build --features "h264,hardware-encoding"
```

### Release Build

```bash
# Optimized release build
export PATH=/usr/local/cuda/bin:$PATH
export CUDA_PATH=/usr/local/cuda
export CUDARC_CUDA_VERSION=12090

cargo build --release --features "h264,hardware-encoding,pam-auth"

# Binary location
ls -la target/release/lamco-rdp-server
```

### Build Script Example

Create `build-release.sh`:

```bash
#!/bin/bash
set -e

# CUDA environment (for NVENC builds)
export PATH=/usr/local/cuda/bin:$PATH
export CUDA_PATH=/usr/local/cuda
export CUDARC_CUDA_VERSION=12090

# Build variant based on argument
case "${1:-full}" in
    "minimal")
        echo "Building minimal (software only)..."
        cargo build --release --features "h264,pam-auth"
        ;;
    "vaapi")
        echo "Building with VAAPI support..."
        cargo build --release --features "h264,vaapi,pam-auth"
        ;;
    "nvenc")
        echo "Building with NVENC support..."
        cargo build --release --features "h264,nvenc,pam-auth"
        ;;
    "full")
        echo "Building with all hardware backends..."
        cargo build --release --features "h264,hardware-encoding,pam-auth"
        ;;
    *)
        echo "Usage: $0 [minimal|vaapi|nvenc|full]"
        exit 1
        ;;
esac

# Strip debug symbols for smaller binary
strip target/release/lamco-rdp-server

echo "Build complete: target/release/lamco-rdp-server"
ls -lh target/release/lamco-rdp-server
```

---

## Binary Distribution Strategy

### Option 1: Single Universal Binary (Recommended)

Build with all backends; runtime detection handles availability:

```bash
cargo build --release --features "h264,hardware-encoding,pam-auth"
```

**Advantages:**
- Single binary to distribute
- Automatic fallback chain: NVENC → VAAPI → Software
- Users don't need to choose correct binary

**Disadvantages:**
- Larger binary (~430MB debug, ~50MB release stripped)
- Build machine needs all dependencies (CUDA toolkit, libva-dev)

**Runtime behavior:**
1. Checks for NVIDIA GPU → Uses NVENC if available
2. Checks for VA-API device → Uses VAAPI if available
3. Falls back to OpenH264 software encoding

### Option 2: Multiple Binary Variants

Build separate binaries for different GPU vendors:

| Binary | Features | Size (est.) | Target Users |
|--------|----------|-------------|--------------|
| `lamco-rdp-server` | h264 | ~30MB | Cloud VMs, no GPU |
| `lamco-rdp-server-vaapi` | h264,vaapi | ~35MB | Intel/AMD GPU |
| `lamco-rdp-server-nvenc` | h264,nvenc | ~40MB | NVIDIA GPU |
| `lamco-rdp-server-full` | h264,hardware-encoding | ~50MB | Any GPU |

**Advantages:**
- Smaller individual binaries
- Can build VAAPI binary without CUDA toolkit

**Disadvantages:**
- Users must choose correct binary
- More binaries to maintain/distribute

### Option 3: Dynamic Loading (Future Enhancement)

Load hardware backends as shared libraries at runtime:

```
lamco-rdp-server           # Core binary
liblamco-vaapi.so          # VAAPI plugin (optional)
liblamco-nvenc.so          # NVENC plugin (optional)
```

**Not currently implemented** but would provide:
- Smallest core binary
- Optional hardware support via plugins
- No compile-time GPU dependencies for core

### Recommended Distribution Package Contents

```
lamco-rdp-server-1.0.0/
├── bin/
│   └── lamco-rdp-server           # Main binary (full features)
├── etc/
│   └── lamco-rdp-server/
│       └── config.toml.example    # Example configuration
├── lib/
│   └── systemd/
│       └── system/
│           └── lamco-rdp-server.service
├── share/
│   └── doc/
│       └── lamco-rdp-server/
│           ├── README.md
│           ├── HARDWARE-ENCODING.md
│           └── LICENSE
└── INSTALL.md
```

---

## Configuration Reference

### Hardware Encoding Configuration

```toml
# config.toml

[hardware_encoding]
# Master enable switch for hardware encoding
enabled = true

# VA-API device path (Intel/AMD)
# Common paths: /dev/dri/renderD128, /dev/dri/renderD129
vaapi_device = "/dev/dri/renderD128"

# Enable DMA-BUF zero-copy (VA-API optimization)
# Reduces CPU usage when PipeWire provides DMA-BUF frames
enable_dmabuf_zerocopy = true

# Fall back to software (OpenH264) if hardware init fails
fallback_to_software = true

# Quality preset: "speed", "balanced", "quality"
# Affects bitrate, GOP size, and encoder tuning
quality_preset = "balanced"

# Prefer NVENC over VAAPI when both are available
# NVENC typically has lower latency
prefer_nvenc = true
```

### Quality Presets

| Preset | Bitrate | GOP Size | Use Case |
|--------|---------|----------|----------|
| `speed` | 3000 kbps | 60 frames | Low bandwidth, fast motion |
| `balanced` | 5000 kbps | 30 frames | Default, general use |
| `quality` | 10000 kbps | 15 frames | High quality, LAN |

### Backend Selection Logic

```
if prefer_nvenc && nvenc_available:
    use NVENC
elif vaapi_available:
    use VAAPI
elif nvenc_available:
    use NVENC
elif fallback_to_software && h264_feature:
    use OpenH264
else:
    error: no encoder available
```

---

## Troubleshooting

### VAAPI Issues

**Error: "VA-API device not found"**
```bash
# Check DRM render nodes exist
ls -la /dev/dri/renderD*

# Verify permissions
groups  # Should include 'video' or 'render'

# Check VA-API works
vainfo --display drm --device /dev/dri/renderD128
```

**Error: "H.264 encoding not supported"**
```bash
# Check supported profiles
vainfo 2>&1 | grep -i h264
# Should show VAProfileH264Main or VAProfileH264High with VAEntrypointEncSlice
```

**Error: "libva error: /usr/lib/dri/iHD_drv_video.so init failed"**
```bash
# Wrong driver for GPU generation
# Try i965 driver instead
export LIBVA_DRIVER_NAME=i965
vainfo
```

### NVENC Issues

**Error: "CUDA device not found"**
```bash
# Verify NVIDIA driver
nvidia-smi

# Check CUDA runtime
ldconfig -p | grep cuda

# Verify device nodes
ls -la /dev/nvidia*
```

**Error: "Failed to initialize NVENC"**
```bash
# Check encode library
ldconfig -p | grep libnvidia-encode
# If missing, reinstall NVIDIA driver

# Check GPU supports NVENC
nvidia-smi -q | grep -i encoder
```

**Error: "Unsupported cuda toolkit version"**
```bash
# Set override for CUDA 13.x
export CUDARC_CUDA_VERSION=12090

# Rebuild
cargo clean -p cudarc
cargo build --features nvenc
```

### Build Issues

**Error: "libva.h not found"**
```bash
sudo apt-get install libva-dev
```

**Error: "nvcc not found"**
```bash
export PATH=/usr/local/cuda/bin:$PATH
# Or install CUDA toolkit
```

**Error: "cudarc build failed"**
```bash
# Ensure CUDA environment is set
export PATH=/usr/local/cuda/bin:$PATH
export CUDA_PATH=/usr/local/cuda
export CUDARC_CUDA_VERSION=12090

# Clean and rebuild
cargo clean
cargo build --features nvenc
```

---

## Testing Checklist

### Build Verification

- [ ] `cargo build --features h264` succeeds (software only)
- [ ] `cargo build --features "h264,vaapi"` succeeds (if libva-dev installed)
- [ ] `cargo build --features "h264,nvenc"` succeeds (if CUDA installed)
- [ ] `cargo build --features "h264,hardware-encoding"` succeeds (both backends)
- [ ] Release build completes: `cargo build --release --features "h264,hardware-encoding"`

### VAAPI Runtime Testing

- [ ] Server starts with VAAPI enabled
- [ ] Logs show "VAAPI encoder initialized"
- [ ] RDP client receives video stream
- [ ] CPU usage lower than software encoding
- [ ] Quality acceptable at all presets
- [ ] Fallback to software works when VAAPI fails

### NVENC Runtime Testing

- [ ] Server starts with NVENC enabled
- [ ] Logs show "NVENC encoder initialized"
- [ ] CUDA context created successfully
- [ ] RDP client receives video stream
- [ ] CPU usage significantly lower (should be <20% for 1080p)
- [ ] GPU encoder utilization visible in `nvidia-smi`
- [ ] Quality acceptable at all presets
- [ ] Fallback to VAAPI/software works when NVENC fails

### Performance Benchmarks

| Resolution | Software CPU | VAAPI CPU | NVENC CPU | Target |
|------------|--------------|-----------|-----------|--------|
| 720p30 | ~15% | <8% | <5% | ✓ |
| 1080p30 | ~25% | <12% | <8% | ✓ |
| 1440p30 | ~40% | <18% | <12% | ✓ |
| 4K30 | ~60% | <25% | <15% | ✓ |

---

## Appendix: Supported Hardware

### VAAPI (Intel)

| Generation | Codename | Example CPUs | Driver |
|------------|----------|--------------|--------|
| Gen 7 | Haswell | i5-4xxx, i7-4xxx | i965 |
| Gen 8 | Broadwell | i5-5xxx, i7-5xxx | iHD |
| Gen 9 | Skylake | i5-6xxx, i7-6xxx | iHD |
| Gen 9.5 | Kaby Lake | i5-7xxx, i7-7xxx | iHD |
| Gen 11 | Ice Lake | i5-10xxG7 | iHD |
| Gen 12 | Tiger Lake | i5-11xx, i7-11xx | iHD |
| Xe | Alder Lake+ | i5-12xxx+ | iHD |

### VAAPI (AMD)

| Architecture | Example GPUs | Driver |
|--------------|--------------|--------|
| GCN 1.0+ | R7 260, R9 290 | radeonsi |
| Polaris | RX 470, RX 580 | radeonsi |
| Vega | Vega 56, Vega 64 | radeonsi |
| RDNA | RX 5700 | radeonsi |
| RDNA 2 | RX 6800 | radeonsi |
| RDNA 3 | RX 7900 | radeonsi |

### NVENC (NVIDIA)

| Architecture | Example GPUs | NVENC Gen |
|--------------|--------------|-----------|
| Kepler | GTX 680, GTX 780 | 1st |
| Maxwell | GTX 960, GTX 980 | 4th |
| Pascal | GTX 1060, GTX 1080 | 5th |
| Turing | RTX 2060, RTX 2080 | 7th |
| Ampere | RTX 3060, RTX 3090 | 7th |
| Ada Lovelace | RTX 4060, RTX 4090 | 8th |

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-12-26 | Initial documentation |

---

## References

- [NVIDIA Video Codec SDK](https://developer.nvidia.com/video-codec-sdk)
- [CUDA Toolkit Downloads](https://developer.nvidia.com/cuda-downloads)
- [VA-API Documentation](https://github.com/intel/libva)
- [cros-libva Crate](https://crates.io/crates/cros-libva)
- [nvidia-video-codec-sdk Crate](https://crates.io/crates/nvidia-video-codec-sdk)
- [cudarc Crate](https://crates.io/crates/cudarc)
