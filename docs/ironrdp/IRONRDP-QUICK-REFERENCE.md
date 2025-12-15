# IronRDP v0.9 Quick Reference Card

## Correct Cargo.toml Configuration

```toml
[dependencies]
# ============================================================================
# RDP PROTOCOL - IronRDP v0.9
# ============================================================================
ironrdp-server = { version = "0.9.0", features = ["helper"] }
ironrdp-pdu = "0.6.0"  # Compatible with server 0.9.0
ironrdp-graphics = "0.6.0"

# ============================================================================
# SCREEN CAPTURE (REQUIRED)
# ============================================================================
ashpd = { version = "0.12.0", features = ["tokio"] }
pipewire = { version = "0.9.2", features = ["v0_3_77"] }
libspa = "0.9.2"

# ============================================================================
# VIDEO ENCODING (CHOOSE ONE)
# ============================================================================
# OPTION 1: Software encoding (for development/testing)
openh264 = { version = "0.6.0", features = ["encoder"] }

# OPTION 2: Hardware encoding (for production - RECOMMENDED)
# va = "0.7.0"
# libva = "0.17.0"

# ============================================================================
# IMAGE FORMAT CONVERSION (REQUIRED)
# ============================================================================
image = "0.25.0"
yuv = "0.1.4"

# ============================================================================
# ASYNC RUNTIME (REQUIRED)
# ============================================================================
tokio = { version = "1.48", features = ["full", "tracing"] }
```

## What IronRDP Provides

✓ RDP protocol implementation
✓ PDU encoding/decoding
✓ Basic bitmap compression (RLE, RemoteFX)
✓ Connection state management
✓ Virtual channel management

## What You Must Add

✗ H.264 video encoding → Use OpenH264 or VA-API
✗ Screen capture → Use PipeWire + xdg-desktop-portal
✗ Image format conversion → Use image + yuv crates
✗ Input handling → Implement separately
✗ Clipboard → Implement via virtual channels

## Common Mistakes

### WRONG
```toml
ironrdp = "0.1.0"  # ← Wrong crate name
ironrdp-server = "0.9.0"  # ← Missing helper feature
ironrdp-pdu = "0.9.0"  # ← Wrong version (doesn't exist)
ironrdp-connector = "0.9.0"  # ← Not needed
```

### CORRECT
```toml
ironrdp-server = { version = "0.9.0", features = ["helper"] }
ironrdp-pdu = "0.6.0"  # Version mismatch is expected
ironrdp-graphics = "0.6.0"
```

## System Dependencies

### Required
- libpipewire-0.3-dev (>= 0.3.77)
- libwayland-dev
- libdbus-1-dev
- xdg-desktop-portal (runtime)

### Optional (Choose One for Video Encoding)
- libopenh264-dev (software encoding)
- libva-dev + libva-drm2 (hardware encoding - RECOMMENDED)

### Not Required
- IronRDP has NO system dependencies (pure Rust)

## Build Commands

```bash
# Verify dependencies
./scripts/verify-dependencies.sh

# Development build
cargo build --features default

# Production build with hardware encoding
cargo build --release --features vaapi

# Run tests
cargo test

# Check for issues
cargo clippy
```

## Version Compatibility

| Crate | Version | Note |
|-------|---------|------|
| ironrdp-server | 0.9.0 | Latest stable |
| ironrdp-pdu | 0.6.0 | Compatible with 0.9.0 server |
| ironrdp-graphics | 0.6.0 | Compatible with 0.9.0 server |

**Note:** The version numbers don't match because IronRDP components have different release cycles. This is CORRECT and EXPECTED.

## Troubleshooting

**Problem:** Can't find ironrdp crate
**Solution:** Use `ironrdp-server` not `ironrdp`

**Problem:** Version mismatch errors
**Solution:** Use ironrdp-pdu 0.6.0 with ironrdp-server 0.9.0 (this is correct)

**Problem:** H.264 encoding not working
**Solution:** Add OpenH264 or VA-API - IronRDP doesn't provide video encoding

**Problem:** Screen capture fails
**Solution:** Ensure PipeWire and xdg-desktop-portal services are running

**Problem:** Build fails with missing symbols
**Solution:** Install system dependencies: `sudo apt install libpipewire-0.3-dev`

## Performance Tips

1. Use VA-API hardware encoding for production (not OpenH264)
2. Enable LTO in release profile (already configured)
3. Use DMA-BUF for zero-copy frame capture
4. Configure proper PipeWire latency: `PIPEWIRE_LATENCY=512/48000`
5. Optimize for target CPU: `RUSTFLAGS="-C target-cpu=native"`

## Getting Help

- Documentation: `/home/greg/wayland/wrd-server-specs/02-TECHNOLOGY-STACK.md`
- Verification: `./scripts/verify-dependencies.sh`
- IronRDP Repo: https://github.com/Devolutions/IronRDP
- Issues: Check TROUBLESHOOTING section in main spec

---

**Quick Reference Version:** 1.0
**IronRDP Version:** 0.9.0
**Last Updated:** 2025-01-18
