# Hardware Encoding Quick Reference

**For full documentation, see:** `HARDWARE-ENCODING-BUILD-GUIDE.md`

---

## Build Commands Cheatsheet

```bash
# === ENVIRONMENT SETUP (for NVENC builds) ===
export PATH=/usr/local/cuda/bin:$PATH
export CUDA_PATH=/usr/local/cuda
export CUDARC_CUDA_VERSION=12090  # Required for CUDA 13.x

# === BUILD VARIANTS ===

# Software only (no GPU dependencies)
cargo build --release --features "h264,pam-auth"

# VAAPI only (Intel/AMD GPU)
cargo build --release --features "h264,vaapi,pam-auth"

# NVENC only (NVIDIA GPU) - requires CUDA toolkit
cargo build --release --features "h264,nvenc,pam-auth"

# All backends (recommended for distribution)
cargo build --release --features "h264,hardware-encoding,pam-auth"

# Strip for smaller binary
strip target/release/lamco-rdp-server
```

---

## Dependency Installation

### Debian/Ubuntu - VAAPI
```bash
sudo apt-get install libva-dev libdrm-dev
sudo apt-get install intel-media-va-driver  # Intel GPU
sudo apt-get install mesa-va-drivers        # AMD GPU
```

### Debian/Ubuntu - NVENC
```bash
# 1. NVIDIA driver (must have libnvidia-encode)
nvidia-smi  # Verify driver works

# 2. CUDA Toolkit from NVIDIA
wget https://developer.download.nvidia.com/compute/cuda/repos/debian13/x86_64/cuda-keyring_1.1-1_all.deb
sudo dpkg -i cuda-keyring_1.1-1_all.deb
sudo apt-get update
sudo apt-get install cuda-toolkit-13-1
sudo ln -sf /usr/local/cuda-13.1 /usr/local/cuda
```

---

## Runtime Requirements

### VAAPI Users Need:
```bash
sudo apt-get install libva2 libva-drm2 intel-media-va-driver
sudo usermod -aG video $USER  # DRM access
```

### NVENC Users Need:
```bash
# Just NVIDIA driver with encode support
nvidia-smi  # Must work
ldconfig -p | grep libnvidia-encode  # Must exist
```

---

## Verification Commands

```bash
# Check VAAPI
vainfo --display drm --device /dev/dri/renderD128

# Check NVENC
nvidia-smi
ldconfig -p | grep libnvidia-encode

# Check CUDA
nvcc --version
```

---

## Configuration (config.toml)

```toml
[hardware_encoding]
enabled = true
vaapi_device = "/dev/dri/renderD128"
enable_dmabuf_zerocopy = true
fallback_to_software = true
quality_preset = "balanced"  # speed|balanced|quality
prefer_nvenc = true
```

---

## Troubleshooting Quick Fixes

| Error | Fix |
|-------|-----|
| "VA-API device not found" | `sudo usermod -aG video $USER` |
| "CUDA device not found" | Check `nvidia-smi` works |
| "Unsupported cuda toolkit version" | Set `CUDARC_CUDA_VERSION=12090` |
| "libva.h not found" | `sudo apt-get install libva-dev` |
| "nvcc not found" | `export PATH=/usr/local/cuda/bin:$PATH` |
| "cudarc build failed" | Set all 3 CUDA env vars, clean rebuild |

---

## Binary Distribution Summary

**Recommended:** Single binary with all backends (`hardware-encoding` feature)
- Runtime auto-detects available hardware
- Fallback chain: NVENC → VAAPI → Software
- Build machine needs all dependencies

**Alternative:** Separate binaries per GPU vendor
- Smaller individual binaries
- Users choose correct variant
