# WRD-Server

Wayland Remote Desktop Server using RDP protocol.

## Overview

WRD-Server is a modern remote desktop server for Wayland compositors that implements the RDP (Remote Desktop Protocol). It enables secure remote access to Wayland sessions with hardware-accelerated video encoding and comprehensive input support.

## Features (Planned)

- **RDP Protocol Support**: Full implementation of RDP for remote desktop access
- **Hardware-Accelerated Encoding**: VAAPI and OpenH264 support for efficient video streaming
- **Wayland Integration**: Native Wayland support via XDG Desktop Portal and PipeWire
- **Secure Authentication**: TLS 1.3 and Network Level Authentication (NLA)
- **Multi-Monitor Support**: Handle multiple displays seamlessly
- **Input Handling**: Keyboard, mouse, and touch input via libei
- **Clipboard Sharing**: Synchronized clipboard between client and server

## Current Status

**TASK P1-01: PROJECT FOUNDATION & CONFIGURATION - COMPLETED**

This is the initial foundation task. The following components are implemented:

- ✅ Project structure and build system
- ✅ Configuration management (TOML, CLI, environment variables)
- ✅ Logging infrastructure (tracing with multiple formats)
- ✅ Command-line interface with argument parsing
- ✅ Unit tests for configuration module
- ✅ Development and build scripts

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

MIT OR Apache-2.0

## Contributing

This project is currently in early development. Contributions will be welcome once the core functionality is implemented.

## Acknowledgments

Built with:
- [tokio](https://tokio.rs/) - Async runtime
- [clap](https://github.com/clap-rs/clap) - CLI parsing
- [tracing](https://github.com/tokio-rs/tracing) - Structured logging
- [serde](https://serde.rs/) - Serialization
