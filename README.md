# lamco-rdp-server

**Professional RDP Server for Wayland/Linux Desktop Sharing**

Production-ready RDP server that provides secure remote desktop access to Linux systems running Wayland, using the XDG Desktop Portal for screen capture and input injection.

## Overview

`lamco-rdp-server` is a modern, production-tested remote desktop server for Wayland-based Linux desktops. It implements the Remote Desktop Protocol (RDP) with native Wayland support via XDG Desktop Portal and PipeWire, enabling secure remote access without X11 dependencies.

Built in Rust with a focus on security, performance, and compatibility with modern Linux desktop environments (GNOME, KDE Plasma, etc.).

## Features

- **✅ RDP Protocol Support**: Full RDP server implementation via IronRDP
- **✅ Wayland Native**: Portal mode using XDG Desktop Portal (no X11 required)
- **✅ PipeWire Screen Capture**: Zero-copy DMA-BUF support for efficient streaming
- **✅ Video Encoding**: H.264 support via OpenH264 with EGFX channel
- **✅ Secure Authentication**: TLS 1.3 and Network Level Authentication (NLA)
- **✅ Input Handling**: Full keyboard and mouse support with 200+ key mappings
- **✅ Clipboard Sharing**: Bidirectional clipboard sync (text and images)
- **✅ Multi-Monitor**: Layout negotiation and display management

## Production Ready

All core modules are implemented and tested:
- Portal integration (600+ lines)
- PipeWire capture (3,392 lines)
- Video pipeline (1,735 lines)
- Input handling (3,727 lines)
- Clipboard sync (3,145 lines)
- Security and authentication
- Configuration management

## Building

### Prerequisites

- Rust 1.70 or later
- OpenSSL (for certificate generation)

### Build Instructions

```bash
# Run setup script (generates test certificates, checks dependencies)
./scripts/setup.sh

# Build the project
cargo build

# Or use the build script (includes formatting and clippy checks)
./scripts/build.sh

# Run tests
cargo test
# Or use the test script
./scripts/test.sh
```

## Quick Start (Development)

**IMPORTANT**: The default config path is `/etc/wrd-server/config.toml`. For development, you MUST specify the local config file:

```bash
# Build release (from project root)
cargo build --release

# Run with local config (REQUIRED for development)
./target/release/wrd-server -c config.toml

# Or with verbose logging
./target/release/wrd-server -c config.toml -vv
```

### Prerequisites for Running

1. **TLS Certificates** in `certs/` directory:
   - `certs/cert.pem` - Certificate file
   - `certs/key.pem` - Private key file
   - Generate with: `./scripts/generate-certs.sh` or copy from test certs:
     ```bash
     cp certs/test-cert.pem certs/cert.pem
     cp certs/test-key.pem certs/key.pem
     ```

2. **D-Bus Session** (for portal access via SSH):
   ```bash
   export DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$(id -u)/bus"
   ```

3. **GNOME Extension** (for clipboard on GNOME):
   - Install from `extension/` directory
   - Log out/in to activate

### One-Liner for SSH Testing

```bash
ssh user@host 'export DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$(id -u)/bus" && cd ~/wayland/wrd-server-specs && ./target/release/wrd-server -c config.toml'
```

## Usage

### Basic Usage

```bash
# Run with local configuration (development)
./target/release/wrd-server -c config.toml

# Run with system configuration (production)
wrd-server  # Uses /etc/wrd-server/config.toml

# Show help
wrd-server --help

# Run with custom port
./target/release/wrd-server -c config.toml --port 5000

# Enable verbose logging
./target/release/wrd-server -c config.toml -vv

# Use JSON logging format
./target/release/wrd-server -c config.toml --log-format json
```

### Configuration

Configuration can be provided via:

1. **TOML file** (default: `/etc/wrd-server/config.toml`)
2. **Environment variables** (prefixed with `WRD_`)
3. **Command-line arguments** (highest priority)

See `config/wrd-server.toml` for a complete example configuration.

### Command-Line Options

```
Options:
  -c, --config <CONFIG>          Configuration file path [default: /etc/wrd-server/config.toml]
  -l, --listen <LISTEN>          Listen address [env: WRD_LISTEN_ADDR=]
  -p, --port <PORT>              Listen port [env: WRD_PORT=] [default: 3389]
  -v, --verbose...               Verbose logging (can be specified multiple times)
      --log-format <LOG_FORMAT>  Log format (json|pretty|compact) [default: pretty]
  -h, --help                     Print help
  -V, --version                  Print version
```

## Development

### Project Structure

```
wrd-server/
├── src/
│   ├── config/         # Configuration management
│   ├── server/         # Server implementation (future)
│   ├── rdp/            # RDP protocol (future)
│   ├── portal/         # XDG Portal integration (future)
│   ├── pipewire/       # PipeWire integration (future)
│   ├── video/          # Video encoding (future)
│   ├── input/          # Input handling (future)
│   ├── clipboard/      # Clipboard sync (future)
│   ├── multimon/       # Multi-monitor support (future)
│   ├── security/       # Authentication and TLS (future)
│   ├── protocol/       # Protocol utilities (future)
│   └── utils/          # Common utilities (future)
├── config/             # Example configurations
├── certs/              # TLS certificates (test only)
├── scripts/            # Build and setup scripts
├── tests/              # Integration tests
└── benches/            # Benchmarks

```

### Running Tests

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_default_config
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run linter
cargo clippy

# Run with all warnings as errors
cargo clippy -- -D warnings
```

## Roadmap

### Phase 1: Foundation (Current)
- [x] Project structure and configuration
- [ ] Security and authentication module
- [ ] Basic RDP protocol implementation

### Phase 2: Core Functionality
- [ ] PipeWire screen capture integration
- [ ] Video encoding (VAAPI/OpenH264)
- [ ] Input handling via libei
- [ ] Basic RDP server

### Phase 3: Advanced Features
- [ ] Multi-monitor support
- [ ] Clipboard synchronization
- [ ] Performance optimizations
- [ ] Comprehensive testing

### Phase 4: Production Ready
- [ ] Security hardening
- [ ] Documentation
- [ ] Packaging (deb, rpm, flatpak)
- [ ] CI/CD pipeline

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

- **Annual License**: $49.99/year per server (includes updates and support)
- **Perpetual License**: $99.00 one-time per server (lifetime use of purchased version)

**Purchase**: [Coming Soon - Lemon Squeezy store]
**Contact**: office@lamco.io

### Future Open Source

This software will **automatically convert** to the **Apache License 2.0** three years after each version's release (first release: December 2025 → converts December 31, 2028).

See the [LICENSE](LICENSE) file for complete terms.

### License Summary

- ✅ **Free** for personal use, students, hobbyists
- ✅ **Free** for non-profits and charities
- ✅ **Free** for tiny businesses (≤3 employees, <$1M revenue)
- ✅ **View and modify** source code
- ❌ **Commercial use** requires paid license
- ❌ **Cannot** build competing RDP/VDI products
- ⏰ **Becomes Apache-2.0** after 3 years

## Contributing

Contributions are welcome! This project uses the BSL 1.1 license, which means:

- You can fork and modify for personal/non-commercial use
- Contributions will be licensed under the same BSL 1.1 terms
- The codebase will become Apache-2.0 in 3 years

Please open an issue before starting significant work to discuss your proposed changes.

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Acknowledgments

Built with:
- [tokio](https://tokio.rs/) - Async runtime
- [clap](https://github.com/clap-rs/clap) - CLI parsing
- [tracing](https://github.com/tokio-rs/tracing) - Structured logging
- [serde](https://serde.rs/) - Serialization
