# lamco-rdp-server

**Professional RDP Server for Wayland/Linux Desktop Sharing**

Production-ready RDP server that provides secure remote desktop access to Linux systems running Wayland, using the XDG Desktop Portal for screen capture and input injection.

## Overview

`lamco-rdp-server` is a modern, production-tested remote desktop server for Wayland-based Linux desktops. It implements the Remote Desktop Protocol (RDP) with native Wayland support via XDG Desktop Portal and PipeWire, enabling secure remote access without X11 dependencies.

Built in Rust with a focus on security, performance, and compatibility with modern Linux desktop environments (GNOME, KDE Plasma, etc.).

## Features

### Core Features (Implemented)
- **RDP Protocol Support**: Full RDP 10.x server implementation via IronRDP
- **Wayland Native**: Portal mode using XDG Desktop Portal (no X11 required)
- **PipeWire Screen Capture**: Zero-copy DMA-BUF support for efficient streaming
- **H.264 Video Encoding**: EGFX channel with AVC420/AVC444 codec support
- **Secure Authentication**: TLS 1.3 and Network Level Authentication (NLA)
- **Input Handling**: Full keyboard and mouse support with 200+ key mappings
- **Clipboard Sharing**: Bidirectional clipboard sync (text and images)
- **Multi-Monitor**: Layout negotiation and display management
- **Damage Detection**: SIMD-optimized tile-based frame differencing (90%+ bandwidth reduction)

### Premium Features (Optional)
- **Hardware Encoding (VA-API)**: Intel/AMD GPU acceleration (`--features vaapi`)
- **Hardware Encoding (NVENC)**: NVIDIA GPU acceleration (`--features nvenc`)
- **AVC444**: Full 4:4:4 chroma with sRGB/full-range VUI signaling for perfect text clarity

### wlroots Compositor Support (NEW - 2026-01-16)

**Two strategies for wlroots-based compositors (Sway, Hyprland, River, labwc):**

**Native Deployment (wlr-direct):**
- Direct Wayland protocol usage (`zwp_virtual_keyboard_v1` + `zwlr_virtual_pointer_v1`)
- Zero permission dialogs (direct compositor access)
- Sub-millisecond input latency
- Build with: `--features wayland`
- Deployment: systemd user service or direct execution
- Status: ✅ Production-ready, fully implemented

**Flatpak Deployment (libei/EIS):**
- Portal RemoteDesktop + EIS protocol via `reis` crate
- Flatpak-compatible (Portal provides socket across sandbox)
- One-time permission dialog (standard Portal flow)
- Build with: `--features libei`
- Deployment: Flatpak bundle
- Status: ✅ Fully implemented, requires portal backend with ConnectToEIS support

**Supported Compositors:**
- ✅ Sway 1.7+ (native: wlr-direct, Flatpak: libei when portal supports it)
- ✅ Hyprland (native: wlr-direct, Flatpak: libei when portal supports it)
- ✅ River (native: wlr-direct)
- ✅ labwc (native: wlr-direct)
- ✅ Any wlroots-based compositor

**See:** `docs/WLR-FULL-IMPLEMENTATION.md` for complete details

## Architecture

```
lamco-rdp-server
  ├─> Portal Session (screen capture + input injection permissions)
  ├─> PipeWire Manager (video frame capture)
  ├─> EGFX Video Handler (H.264 encoding via OpenH264/VAAPI/NVENC)
  ├─> Input Handler (keyboard/mouse from RDP clients)
  ├─> Clipboard Manager (bidirectional clipboard sync)
  └─> IronRDP Server (RDP protocol, TLS, channel management)
```

See `docs/architecture/COMPREHENSIVE-ARCHITECTURE-AUDIT-2025-12-27.md` for detailed architecture documentation.

## Building

### Prerequisites

- Rust 1.77 or later
- OpenSSL development libraries
- PipeWire development libraries
- For H.264: `nasm` (3x speedup for OpenH264)

### Build Instructions

```bash
# Default build (software H.264 encoding)
cargo build --release

# With VA-API hardware encoding (Intel/AMD)
cargo build --release --features vaapi

# With NVENC hardware encoding (NVIDIA)
cargo build --release --features nvenc

# With all hardware backends
cargo build --release --features hardware-encoding
```

### Hardware Encoding Requirements

**VA-API (Intel/AMD):**
- `libva-dev` >= 1.20.0
- Intel iHD driver (modern Intel) or i965 (older Intel)
- AMD radeonsi driver

**NVENC (NVIDIA):**
- NVIDIA driver with `libnvidia-encode.so`
- CUDA toolkit
- NVENC-capable GPU (GTX 6xx+, any RTX)

## Quick Start

### Prerequisites for Running

1. **TLS Certificates** in `certs/` directory:
   ```bash
   # Generate test certificates
   ./scripts/generate-certs.sh
   # Or copy existing test certs
   cp certs/test-cert.pem certs/cert.pem
   cp certs/test-key.pem certs/key.pem
   ```

2. **D-Bus Session** (required for portal access via SSH):
   ```bash
   export DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$(id -u)/bus"
   ```

3. **PipeWire** running for screen capture

### Running

```bash
# Run with local configuration
./target/release/lamco-rdp-server -c config.toml

# With verbose logging
./target/release/lamco-rdp-server -c config.toml -vv

# With log file
./target/release/lamco-rdp-server -c config.toml -vv --log-file server.log
```

### Connecting

Use any RDP client:
- Windows: `mstsc.exe` (Remote Desktop Connection)
- Linux: `xfreerdp`, `remmina`
- macOS: Microsoft Remote Desktop

```bash
# FreeRDP example
xfreerdp /v:hostname:3389 /u:username /p:password /gfx:AVC444
```

## Configuration

Configuration can be provided via:
1. **TOML file** (default: `/etc/wrd-server/config.toml`)
2. **Environment variables** (prefixed with `WRD_`)
3. **Command-line arguments** (highest priority)

### Command-Line Options

```
Options:
  -c, --config <CONFIG>          Configuration file path [default: /etc/wrd-server/config.toml]
  -l, --listen <LISTEN>          Listen address [env: WRD_LISTEN_ADDR=]
  -p, --port <PORT>              Listen port [env: WRD_PORT=] [default: 3389]
  -v, --verbose...               Verbose logging (can be specified multiple times)
      --log-format <LOG_FORMAT>  Log format (json|pretty|compact) [default: pretty]
      --log-file <PATH>          Log to file
  -h, --help                     Print help
  -V, --version                  Print version
```

See `config.toml` for a complete example configuration.

## Project Structure

```
lamco-rdp-server/
├── src/
│   ├── lib.rs          # Library root, module exports
│   ├── main.rs         # Binary entry point
│   ├── config/         # Configuration management
│   ├── server/         # Main server implementation
│   ├── rdp/            # RDP channel management
│   ├── egfx/           # EGFX video pipeline
│   │   ├── encoder.rs        # OpenH264 AVC420 encoder
│   │   ├── avc444_encoder.rs # Dual-stream AVC444 encoder
│   │   ├── color_space.rs    # VUI parameters, color presets
│   │   ├── color_convert.rs  # BGRA→YUV444 with SIMD
│   │   ├── yuv444_packing.rs # AVC444 dual-stream packing
│   │   └── hardware/         # Hardware encoders
│   │       ├── vaapi/        # VA-API (Intel/AMD)
│   │       └── nvenc/        # NVENC (NVIDIA)
│   ├── clipboard/      # Clipboard orchestration
│   ├── damage/         # Damage region detection
│   ├── multimon/       # Multi-monitor support
│   ├── security/       # Authentication and TLS
│   ├── protocol/       # Protocol utilities
│   └── utils/          # Common utilities
├── docs/               # Documentation
│   ├── architecture/   # Architecture docs
│   ├── specs/          # Specifications
│   └── guides/         # User guides
├── certs/              # TLS certificates
├── scripts/            # Build and setup scripts
└── benches/            # Performance benchmarks
```

## Documentation

- **Architecture**: `docs/architecture/COMPREHENSIVE-ARCHITECTURE-AUDIT-2025-12-27.md`
- **Color Infrastructure**: `docs/architecture/NVENC-AND-COLOR-INFRASTRUCTURE.md`
- **IronRDP Integration**: `docs/ironrdp/IRONRDP-INTEGRATION-GUIDE.md`
- **Testing Setup**: `docs/guides/TESTING-ENVIRONMENT-RECOMMENDATIONS.md`

## Development

### Running Tests

```bash
cargo test
cargo test -- --nocapture  # With output
```

### Benchmarks

```bash
cargo bench --bench video_encoding
cargo bench --bench color_conversion
cargo bench --bench damage_detection
```

### Code Quality

```bash
cargo fmt
cargo clippy
```

## License

`lamco-rdp-server` is licensed under the **Business Source License 1.1 (BSL)**.

### Free Use

You may use lamco-rdp-server **for free** if you meet **ALL** of the following conditions:

1. You are a **non-profit organization**, OR
2. You are a **for-profit organization** with:
   - **3 or fewer employees**, AND
   - **Less than $1,000,000 USD in annual revenue**, AND
3. Your use **does not include** creating a competitive or derivative RDP server product or VDI solution

### Commercial License Required

Larger organizations or commercial deployments require a commercial license:

- **Annual License**: $49.99/year per server
- **Perpetual License**: $99.00 one-time per server

**Contact**: office@lamco.io

### Future Open Source

This software will **automatically convert** to the **Apache License 2.0** three years after each version's release (first release: December 2025 → converts December 31, 2028).

See the [LICENSE](LICENSE) file for complete terms.

## Contributing

Contributions are welcome! Please open an issue before starting significant work.

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Acknowledgments

Built with:
- [IronRDP](https://github.com/Devolutions/IronRDP) - RDP protocol implementation
- [tokio](https://tokio.rs/) - Async runtime
- [OpenH264](https://github.com/cisco/openh264) - H.264 codec
- [PipeWire](https://pipewire.org/) - Screen capture
- [ashpd](https://github.com/bilelmoussaoui/ashpd) - XDG Portal bindings
