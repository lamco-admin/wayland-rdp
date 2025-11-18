# PROJECT STRUCTURE SPECIFICATION
**Document:** 03-PROJECT-STRUCTURE.md
**Version:** 1.0
**Date:** 2025-01-18
**Parent:** 00-MASTER-SPECIFICATION.md

---

## DOCUMENT PURPOSE

This document defines the COMPLETE directory structure, file organization, module hierarchy, naming conventions, visibility rules, and architectural organization for the WRD-Server project. All code MUST follow this structure exactly.

---

## TABLE OF CONTENTS

1. [Complete Directory Tree](#complete-directory-tree)
2. [Module Organization](#module-organization)
3. [File Naming Conventions](#file-naming-conventions)
4. [Module Visibility Guidelines](#module-visibility-guidelines)
5. [Import Organization Rules](#import-organization-rules)
6. [Test Organization](#test-organization)
7. [Benchmark Organization](#benchmark-organization)
8. [Example Programs](#example-programs)
9. [Configuration Files](#configuration-files)
10. [Build Artifacts](#build-artifacts)
11. [Module Dependency Graph](#module-dependency-graph)
12. [Public API Surface](#public-api-surface)

---

## COMPLETE DIRECTORY TREE

```
wrd-server/
├── Cargo.toml                          # Rust project manifest
├── Cargo.lock                          # Dependency lock (committed)
├── README.md                           # Project overview
├── LICENSE-MIT                         # MIT license text
├── LICENSE-APACHE                      # Apache 2.0 license text
├── .gitignore                          # Git ignore rules
├── deny.toml                           # Cargo-deny configuration
├── rustfmt.toml                        # Rustfmt configuration
├── clippy.toml                         # Clippy configuration
│
├── src/                                # Source code root
│   ├── main.rs                         # Binary entry point
│   ├── lib.rs                          # Library root (re-exports public API)
│   │
│   ├── config/                         # Configuration module
│   │   ├── mod.rs                      # Module root, Config struct
│   │   ├── types.rs                    # Configuration type definitions
│   │   ├── validation.rs               # Configuration validation
│   │   └── loader.rs                   # File loading and parsing
│   │
│   ├── server/                         # Server coordination module
│   │   ├── mod.rs                      # Server struct, lifecycle
│   │   ├── connection.rs               # ConnectionManager
│   │   ├── session.rs                  # SessionManager
│   │   └── resource_tracker.rs         # Resource monitoring
│   │
│   ├── security/                       # Security module
│   │   ├── mod.rs                      # SecurityManager
│   │   ├── tls.rs                      # TLS configuration
│   │   ├── certificates.rs             # Certificate management
│   │   ├── auth.rs                     # PAM authentication
│   │   └── token.rs                    # Session token generation
│   │
│   ├── rdp/                            # RDP protocol module
│   │   ├── mod.rs                      # RdpServer wrapper
│   │   ├── state.rs                    # Protocol state machine
│   │   ├── capabilities.rs             # Capability negotiation
│   │   ├── pdu.rs                      # PDU encoding/decoding helpers
│   │   └── channels/                   # Virtual channel implementations
│   │       ├── mod.rs                  # Channel registry
│   │       ├── graphics.rs             # Graphics channel (RDPGFX)
│   │       ├── input.rs                # Input channel (RDPEI)
│   │       ├── clipboard.rs            # Clipboard channel (CLIPRDR)
│   │       └── audio.rs                # Audio channel (RDPSND, Phase 2)
│   │
│   ├── portal/                         # xdg-desktop-portal integration
│   │   ├── mod.rs                      # PortalManager
│   │   ├── session.rs                  # Portal session lifecycle
│   │   ├── screencast.rs               # ScreenCast portal
│   │   ├── remote_desktop.rs           # RemoteDesktop portal
│   │   └── clipboard.rs                # Clipboard portal (future)
│   │
│   ├── pipewire/                       # PipeWire integration
│   │   ├── mod.rs                      # PipeWireStream
│   │   ├── stream.rs                   # Stream management
│   │   ├── format.rs                   # Format negotiation
│   │   ├── frame.rs                    # Frame reception
│   │   └── dmabuf.rs                   # DMA-BUF handling
│   │
│   ├── video/                          # Video processing pipeline
│   │   ├── mod.rs                      # VideoPipeline orchestrator
│   │   ├── types.rs                    # VideoFrame, EncodedFrame types
│   │   ├── damage.rs                   # DamageTracker
│   │   ├── cursor.rs                   # CursorManager
│   │   ├── converter.rs                # Format conversion (BGRA→NV12)
│   │   ├── scaler.rs                   # Resolution scaling
│   │   └── encoder/                    # H.264 encoders
│   │       ├── mod.rs                  # VideoEncoder trait, factory
│   │       ├── vaapi.rs                # VA-API encoder
│   │       ├── openh264.rs             # OpenH264 software encoder
│   │       └── common.rs               # Shared encoder utilities
│   │
│   ├── input/                          # Input handling module
│   │   ├── mod.rs                      # InputManager
│   │   ├── translator.rs               # RDP ↔ Wayland translation
│   │   ├── keyboard.rs                 # KeyboardHandler
│   │   ├── pointer.rs                  # PointerHandler
│   │   └── keymaps.rs                  # Keycode mapping tables
│   │
│   ├── clipboard/                      # Clipboard management
│   │   ├── mod.rs                      # ClipboardManager
│   │   ├── sync.rs                     # Synchronization logic
│   │   ├── formats.rs                  # Format conversion (RDP ↔ MIME)
│   │   └── validator.rs                # Size/type validation
│   │
│   ├── multimon/                       # Multi-monitor support
│   │   ├── mod.rs                      # MultiMonitorManager
│   │   ├── layout.rs                   # Layout calculation
│   │   ├── coordinator.rs              # Stream coordination
│   │   └── monitor.rs                  # MonitorInfo structure
│   │
│   ├── audio/                          # Audio pipeline (Phase 2)
│   │   ├── mod.rs                      # AudioPipeline
│   │   ├── capture.rs                  # Audio capture (PipeWire)
│   │   ├── encoder.rs                  # Opus encoder
│   │   ├── playback.rs                 # Audio playback
│   │   └── sync.rs                     # A/V synchronization
│   │
│   ├── protocol/                       # Protocol utilities
│   │   ├── mod.rs                      # Common protocol definitions
│   │   └── wayland.rs                  # Wayland protocol helpers
│   │
│   └── utils/                          # Utility functions
│       ├── mod.rs                      # Common utilities
│       ├── logging.rs                  # Logging configuration
│       ├── metrics.rs                  # Performance metrics
│       ├── buffer_pool.rs              # Buffer allocation pool
│       └── error.rs                    # Error type definitions
│
├── tests/                              # Integration tests
│   ├── common/                         # Shared test utilities
│   │   ├── mod.rs                      # Test helper functions
│   │   ├── mock_portal.rs              # Mock portal implementation
│   │   ├── mock_rdp_client.rs          # Mock RDP client
│   │   └── fixtures.rs                 # Test data fixtures
│   │
│   ├── integration/                    # Integration test suites
│   │   ├── connection_test.rs          # Connection establishment
│   │   ├── video_pipeline_test.rs      # Video end-to-end
│   │   ├── input_test.rs               # Input injection
│   │   ├── clipboard_test.rs           # Clipboard sync
│   │   └── multimon_test.rs            # Multi-monitor
│   │
│   └── security_integration.rs         # Security integration tests
│
├── benches/                            # Performance benchmarks
│   ├── video_encoding.rs               # Encoding benchmarks
│   ├── network_throughput.rs           # Network performance
│   ├── damage_tracking.rs              # Damage detection
│   └── format_conversion.rs            # Pixel format conversion
│
├── examples/                           # Example programs
│   ├── portal_info.rs                  # Query portal capabilities
│   ├── simple_server.rs                # Minimal server example
│   ├── vaapi_test.rs                   # Test VA-API availability
│   └── benchmark_encoder.rs            # Encoder comparison
│
├── config/                             # Default configuration files
│   ├── wrd-server.toml                 # Default server config
│   ├── development.toml                # Development overrides
│   ├── production.toml                 # Production settings
│   └── systemd/                        # Systemd service files
│       ├── wrd-server.service          # System service
│       └── wrd-server@.service         # Per-user service template
│
├── certs/                              # Certificate storage
│   ├── README.md                       # Certificate documentation
│   ├── .gitignore                      # Ignore actual certificates
│   └── generate.sh                     # Certificate generation script
│
├── scripts/                            # Build and deployment scripts
│   ├── setup.sh                        # Initial setup script
│   ├── build.sh                        # Build script
│   ├── test.sh                         # Test runner
│   ├── generate-certs.sh               # Generate self-signed certs
│   ├── verify-dependencies.sh          # Dependency verification
│   ├── install.sh                      # Installation script
│   └── benchmark.sh                    # Run all benchmarks
│
├── docs/                               # User documentation
│   ├── user-guide.md                   # User guide
│   ├── installation.md                 # Installation instructions
│   ├── configuration.md                # Configuration reference
│   ├── troubleshooting.md              # Troubleshooting guide
│   └── api/                            # Generated API docs (rustdoc)
│
├── .github/                            # GitHub-specific files
│   ├── workflows/                      # CI/CD workflows
│   │   ├── ci.yml                      # Continuous integration
│   │   ├── release.yml                 # Release automation
│   │   └── security.yml                # Security scanning
│   │
│   ├── ISSUE_TEMPLATE/                 # Issue templates
│   │   ├── bug_report.md               # Bug report template
│   │   └── feature_request.md          # Feature request template
│   │
│   └── PULL_REQUEST_TEMPLATE.md        # PR template
│
└── target/                             # Build artifacts (gitignored)
    ├── debug/                          # Debug build output
    │   ├── wrd-server                  # Debug binary
    │   ├── deps/                       # Dependencies
    │   ├── build/                      # Build scripts output
    │   └── incremental/                # Incremental compilation
    │
    ├── release/                        # Release build output
    │   ├── wrd-server                  # Optimized binary
    │   └── deps/                       # Dependencies
    │
    ├── doc/                            # Generated documentation
    │   └── wrd_server/                 # Crate documentation
    │
    └── criterion/                      # Benchmark results
        └── reports/                    # HTML benchmark reports
```

---

## MODULE ORGANIZATION

### Root Modules (src/lib.rs)

```rust
// src/lib.rs - Library root
// Re-exports public API for external use

// Public modules - exposed to library users
pub mod config;
pub mod server;
pub mod utils;

// Internal modules - not exposed
mod security;
mod rdp;
mod portal;
mod pipewire;
mod video;
mod input;
mod clipboard;
mod multimon;
mod audio;     // Phase 2
mod protocol;

// Re-export key types for convenience
pub use config::Config;
pub use server::Server;
pub use utils::error::{Error, Result};

// Prelude for common imports
pub mod prelude {
    pub use crate::config::Config;
    pub use crate::server::Server;
    pub use crate::utils::error::{Error, Result};
}
```

### Binary Entry Point (src/main.rs)

```rust
// src/main.rs - Binary entry point
// Minimal main function, delegates to library

use anyhow::Result;
use clap::Parser;
use wrd_server::{Config, Server};

#[derive(Parser)]
#[command(name = "wrd-server")]
#[command(about = "Wayland Remote Desktop Server")]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "/etc/wrd-server/config.toml")]
    config: String,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    wrd_server::utils::logging::init(cli.debug)?;

    // Load configuration
    let config = Config::load(&cli.config)?;

    // Create and run server
    let server = Server::new(config).await?;
    server.run().await?;

    Ok(())
}
```

### Module Structure Rules

#### 1. Module Root (mod.rs)
- MUST contain primary struct/type for the module
- MUST declare all submodules
- MUST re-export public items
- MAY contain module-level documentation

Example:
```rust
// src/video/mod.rs
//! Video processing pipeline module.
//!
//! This module handles the complete video pipeline from PipeWire frames
//! to encoded H.264 output.

mod types;
mod damage;
mod cursor;
mod converter;
mod scaler;
pub mod encoder;  // Public submodule

// Re-export public types
pub use types::{VideoFrame, EncodedFrame, PixelFormat};
pub use damage::DamageTracker;
pub use cursor::CursorManager;
pub use converter::FormatConverter;
pub use scaler::FrameScaler;
pub use encoder::{VideoEncoder, EncoderFactory};

/// Main video processing pipeline coordinator.
pub struct VideoPipeline {
    // Implementation
}
```

#### 2. Submodule Organization
- ONE primary struct/trait per file
- Related helper types in same file
- Implementation in same file as type definition
- Tests in same file (using #[cfg(test)])

#### 3. Module Size Guidelines
- Module file: 200-500 lines (target)
- Single file MUST NOT exceed 1000 lines
- If > 1000 lines, split into submodules
- Keep functions < 50 lines

---

## FILE NAMING CONVENTIONS

### Rust Source Files
| Pattern | Usage | Example |
|---------|-------|---------|
| `mod.rs` | Module root | `src/video/mod.rs` |
| `lib.rs` | Library root | `src/lib.rs` |
| `main.rs` | Binary entry | `src/main.rs` |
| `{concept}.rs` | Single concept | `src/video/damage.rs` |
| `{concept}s.rs` | Multiple items | `src/input/keymaps.rs` |

### Test Files
| Pattern | Usage | Example |
|---------|-------|---------|
| `{module}_test.rs` | Integration test | `tests/integration/video_pipeline_test.rs` |
| `common/` | Shared test code | `tests/common/mod.rs` |
| `#[cfg(test)] mod tests` | Unit tests (inline) | In same file as code |

### Benchmark Files
| Pattern | Usage | Example |
|---------|-------|---------|
| `{feature}_benchmark.rs` | Criterion bench | `benches/video_encoding.rs` |

### Configuration Files
| Pattern | Usage | Example |
|---------|-------|---------|
| `{name}.toml` | TOML config | `config/wrd-server.toml` |
| `{env}.toml` | Environment override | `config/production.toml` |

### Script Files
| Pattern | Usage | Example |
|---------|-------|---------|
| `{action}.sh` | Shell script | `scripts/setup.sh` |
| `.sh` extension | All shell scripts | Mandatory |
| Executable bit | Must be set | `chmod +x` |

### Case Conventions
- **Rust files:** `snake_case.rs`
- **Modules:** `snake_case`
- **Structs/Enums:** `PascalCase`
- **Functions/variables:** `snake_case`
- **Constants:** `SCREAMING_SNAKE_CASE`
- **Type parameters:** `T`, `U`, single uppercase
- **Lifetimes:** `'a`, `'b`, single lowercase

---

## MODULE VISIBILITY GUIDELINES

### Public vs Private Rules

#### MUST be Public (pub)
1. **Core API types** used by library consumers
2. **Configuration structures** loaded from files
3. **Error types** that cross module boundaries
4. **Trait definitions** implemented by external code
5. **Module roots** if submodules need access

#### MUST be Private (no pub)
1. **Implementation details** not part of API
2. **Internal state machines** and helpers
3. **Cache structures** and optimization details
4. **Buffer management** internals
5. **FFI wrappers** around C libraries

#### MAY be Crate-Private (pub(crate))
1. **Inter-module types** used across modules internally
2. **Testing utilities** used by integration tests
3. **Factory functions** for internal construction
4. **Shared constants** between modules

### Visibility Patterns

```rust
// Public module with public API
pub mod config {
    /// Public configuration struct
    pub struct Config {
        pub address: String,      // Public field
        pub port: u16,            // Public field
        internal_state: State,    // Private field
    }

    impl Config {
        /// Public constructor
        pub fn new() -> Self { /* ... */ }

        /// Public method
        pub fn validate(&self) -> Result<()> { /* ... */ }

        /// Private helper
        fn check_internal(&self) -> bool { /* ... */ }
    }

    /// Private helper struct
    struct State {
        // Implementation detail
    }
}

// Private module (implementation detail)
mod internal {
    pub(crate) struct Helper {  // Visible to crate only
        // Implementation
    }
}
```

### API Surface Guidelines

#### Public API MUST
1. Have complete rustdoc documentation
2. Have stability guarantees (semver)
3. Have comprehensive examples
4. Have integration tests
5. Be backwards compatible (within major version)

#### Private API MAY
1. Change at any time
2. Have minimal documentation
3. Use unsafe code (with justification)
4. Have relaxed error handling
5. Optimize aggressively

---

## IMPORT ORGANIZATION RULES

### Import Ordering (Enforced by rustfmt)

```rust
// 1. Standard library imports
use std::collections::HashMap;
use std::sync::Arc;

// 2. External crate imports (alphabetical)
use anyhow::Result;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

// 3. Internal crate imports (alphabetical by module)
use crate::config::Config;
use crate::portal::PortalManager;
use crate::video::VideoFrame;

// 4. Parent/sibling module imports
use super::types::InternalType;
```

### Import Style Rules

#### MUST Use
- Absolute paths for cross-module imports: `use crate::config::Config;`
- Relative paths for same-module imports: `use super::types::Foo;`
- Explicit imports for types: `use crate::video::VideoFrame;`

#### MUST NOT Use
- Glob imports in production code: ~~`use crate::video::*;`~~
- Deeply nested paths: ~~`use crate::a::b::c::d::e::Foo;`~~ (refactor)

#### MAY Use
- Grouped imports: `use tokio::sync::{mpsc, RwLock};`
- Renamed imports for clarity: `use ironrdp::Server as RdpServer;`
- Prelude in tests: `use crate::prelude::*;`

### Common Import Patterns

```rust
// Async runtime
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::task;
use async_trait::async_trait;

// Error handling
use anyhow::{Context, Result};
use thiserror::Error;

// Logging
use tracing::{debug, error, info, instrument, warn};

// Serialization
use serde::{Deserialize, Serialize};
```

---

## TEST ORGANIZATION

### Unit Tests (Inline)

Located in same file as code under test:

```rust
// src/video/damage.rs

pub struct DamageTracker {
    // Implementation
}

impl DamageTracker {
    pub fn track(&mut self, rect: Rect) {
        // Implementation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damage_tracking() {
        let mut tracker = DamageTracker::new();
        tracker.track(Rect::new(0, 0, 100, 100));
        assert_eq!(tracker.damage_count(), 1);
    }

    #[tokio::test]
    async fn test_async_operation() {
        // Async test
    }
}
```

### Integration Tests

Located in `tests/` directory:

```
tests/
├── common/
│   ├── mod.rs              # Test utilities (pub functions)
│   ├── mock_portal.rs      # Mock portal implementation
│   └── fixtures.rs         # Test data
│
└── integration/
    ├── video_pipeline_test.rs     # End-to-end video test
    └── connection_test.rs         # Connection test
```

Example:
```rust
// tests/integration/video_pipeline_test.rs

use wrd_server::prelude::*;

mod common;
use common::{create_test_config, mock_portal};

#[tokio::test]
async fn test_video_pipeline_end_to_end() {
    let config = create_test_config();
    let portal = mock_portal();

    // Test implementation
}
```

### Test File Naming

| Type | Location | Naming |
|------|----------|--------|
| Unit tests | Same file | `#[cfg(test)] mod tests` |
| Integration | `tests/integration/` | `{feature}_test.rs` |
| Common utilities | `tests/common/` | `{utility}.rs` |

### Test Organization Rules

1. **Unit tests MUST** be in same file as implementation
2. **Integration tests MUST** be in `tests/` directory
3. **Test utilities MUST** be in `tests/common/`
4. **Mock implementations MUST** start with `Mock` prefix
5. **Test data MUST** be in fixtures module

---

## BENCHMARK ORGANIZATION

### Benchmark Structure

```
benches/
├── video_encoding.rs          # Encoder benchmarks
├── network_throughput.rs      # Network benchmarks
├── damage_tracking.rs         # Damage detection benchmarks
└── format_conversion.rs       # Pixel format benchmarks
```

### Benchmark File Template

```rust
// benches/video_encoding.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use wrd_server::video::{VideoEncoder, EncoderFactory, VideoFrame};

fn benchmark_h264_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("h264_encoding");

    // VA-API encoder
    group.bench_function("vaapi_1080p", |b| {
        let encoder = EncoderFactory::create_vaapi().unwrap();
        let frame = create_test_frame(1920, 1080);

        b.iter(|| {
            encoder.encode(black_box(&frame))
        });
    });

    // OpenH264 encoder
    group.bench_function("openh264_1080p", |b| {
        let encoder = EncoderFactory::create_openh264().unwrap();
        let frame = create_test_frame(1920, 1080);

        b.iter(|| {
            encoder.encode(black_box(&frame))
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_h264_encoding);
criterion_main!(benches);
```

### Benchmark Naming

- Function: `benchmark_{feature}`
- Group: `{feature}_{variant}`
- File: `{feature}_benchmark.rs` or `{feature}.rs`

---

## EXAMPLE PROGRAMS

### Example Structure

```
examples/
├── portal_info.rs          # Query portal capabilities
├── simple_server.rs        # Minimal server
├── vaapi_test.rs          # Test VA-API
└── benchmark_encoder.rs    # Compare encoders
```

### Example File Template

```rust
// examples/portal_info.rs
//! Query and display xdg-desktop-portal capabilities.
//!
//! Usage: cargo run --example portal_info

use anyhow::Result;
use wrd_server::portal::PortalManager;

#[tokio::main]
async fn main() -> Result<()> {
    // Example implementation
    let portal = PortalManager::new().await?;
    println!("Portal version: {}", portal.version()?);

    Ok(())
}
```

### Example Naming

- File: `{purpose}.rs` (snake_case)
- Documentation: Top-level `//!` comment with usage
- Runnable: `cargo run --example {name}`

---

## CONFIGURATION FILES

### Configuration File Locations

| Environment | File Path | Purpose |
|-------------|-----------|---------|
| System default | `/etc/wrd-server/config.toml` | System-wide config |
| User default | `~/.config/wrd-server/config.toml` | Per-user config |
| Development | `config/development.toml` | Dev overrides |
| Production | `config/production.toml` | Production settings |
| Runtime | `/var/run/wrd-server/` | Runtime state |

### Log File Locations

| Log Type | Path | Rotation |
|----------|------|----------|
| System service | `/var/log/wrd-server/server.log` | Daily, 7 days |
| User service | `~/.local/share/wrd-server/logs/` | Daily, 7 days |
| Debug logs | `/tmp/wrd-server-debug.log` | Session only |

### Certificate Locations

| Type | Path | Purpose |
|------|------|---------|
| Server cert | `/etc/wrd-server/certs/server.crt` | TLS server certificate |
| Server key | `/etc/wrd-server/certs/server.key` | TLS private key (0600) |
| CA cert | `/etc/wrd-server/certs/ca.crt` | Optional CA certificate |
| User cert | `~/.config/wrd-server/certs/` | Per-user certificates |

### Runtime State Locations

| Type | Path | Purpose |
|------|------|---------|
| PID file | `/var/run/wrd-server/wrd-server.pid` | Process ID |
| Socket | `/var/run/wrd-server/control.sock` | Control socket |
| Session state | `/var/run/wrd-server/sessions/` | Active sessions |

---

## BUILD ARTIFACTS

### Target Directory Structure

```
target/
├── debug/                      # Debug build (cargo build)
│   ├── wrd-server             # Debug binary
│   ├── libwrd_server.rlib     # Debug library
│   ├── deps/                  # Dependency artifacts
│   ├── build/                 # Build script outputs
│   ├── examples/              # Compiled examples
│   └── incremental/           # Incremental compilation cache
│
├── release/                    # Release build (cargo build --release)
│   ├── wrd-server             # Optimized binary (stripped)
│   ├── libwrd_server.rlib     # Optimized library
│   └── deps/                  # Dependency artifacts
│
├── doc/                        # Generated documentation (cargo doc)
│   └── wrd_server/            # Crate documentation
│       ├── index.html         # Documentation entry point
│       └── ...                # Generated HTML
│
└── criterion/                  # Benchmark results (cargo bench)
    ├── {benchmark}/           # Per-benchmark results
    │   ├── base/              # Baseline measurements
    │   └── change/            # Change detection
    └── report/                # HTML reports
        └── index.html         # Benchmark report
```

### Build Artifact Naming

| Artifact | Location | Naming |
|----------|----------|--------|
| Binary | `target/{profile}/` | `wrd-server` |
| Library | `target/{profile}/` | `libwrd_server.rlib` |
| Examples | `target/{profile}/examples/` | `{example_name}` |
| Tests | `target/{profile}/deps/` | `{test_name}-{hash}` |

---

## MODULE DEPENDENCY GRAPH

### Layer Architecture

```
┌─────────────────────────────────────────────────────┐
│ Layer 1: Application (main.rs)                      │
│   - CLI parsing                                      │
│   - Initialization                                   │
└────────────────┬────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────┐
│ Layer 2: Coordination (server)                      │
│   - Server lifecycle                                 │
│   - Connection management                            │
│   - Resource coordination                            │
└──┬────────────┬─────────────┬────────────┬──────────┘
   │            │             │            │
   ▼            ▼             ▼            ▼
┌──────┐  ┌──────────┐  ┌─────────┐  ┌─────────┐
│config│  │ security │  │   rdp   │  │ portal  │  Layer 3: Core Modules
└──┬───┘  └────┬─────┘  └────┬────┘  └────┬────┘
   │           │             │            │
   │           │             │            ▼
   │           │             │      ┌──────────┐
   │           │             │      │pipewire  │  Layer 4: Integration
   │           │             │      └────┬─────┘
   │           │             │           │
   │           │             │           ▼
   │           │             │      ┌──────────┐
   │           │             │      │  video   │  Layer 5: Processing
   │           │             │      │  input   │
   │           │             │      │clipboard │
   │           │             │      │multimon  │
   │           │             │      └──────────┘
   │           │             │
   └───────────┴─────────────┴──────► utils     Layer 6: Utilities
                                     (error, logging, metrics)
```

### Module Dependencies (Detailed)

```
main
 └─► server
      ├─► config
      ├─► security
      │    ├─► config
      │    └─► utils::error
      ├─► rdp
      │    ├─► config
      │    ├─► security
      │    ├─► rdp::channels
      │    │    ├─► video
      │    │    ├─► input
      │    │    └─► clipboard
      │    └─► utils
      └─► portal
           ├─► config
           ├─► pipewire
           │    ├─► video::types
           │    └─► utils
           └─► utils

video
 ├─► video::encoder
 │    ├─► video::types
 │    └─► utils
 ├─► video::damage
 ├─► video::cursor
 └─► utils

input
 ├─► portal::remote_desktop
 └─► utils

clipboard
 ├─► portal::clipboard
 ├─► rdp::channels::clipboard
 └─► utils

multimon
 ├─► portal::screencast
 ├─► video
 └─► utils
```

### Dependency Rules

1. **No circular dependencies** between modules
2. **Downward dependencies only** (higher layers depend on lower)
3. **utils module** has no dependencies on other modules
4. **config module** depends only on utils
5. **Protocol modules** (rdp, portal) don't depend on each other

---

## PUBLIC API SURFACE

### Exported Types (src/lib.rs)

```rust
// Configuration
pub use config::{
    Config,
    ServerConfig,
    SecurityConfig,
    VideoConfig,
    AudioConfig,
};

// Server
pub use server::{
    Server,
    ServerError,
};

// Errors
pub use utils::error::{
    Error,
    Result,
    ErrorKind,
};

// Video types (for custom encoding)
pub use video::{
    VideoFrame,
    EncodedFrame,
    PixelFormat,
};

// Traits (for extension)
pub use video::encoder::VideoEncoder;
```

### Public Module Structure

```rust
pub mod config {
    pub struct Config { /* ... */ }
    pub struct ServerConfig { /* ... */ }
    pub struct SecurityConfig { /* ... */ }
    pub struct VideoConfig { /* ... */ }

    // Builder pattern
    pub struct ConfigBuilder { /* ... */ }
}

pub mod server {
    pub struct Server { /* ... */ }

    impl Server {
        pub async fn new(config: Config) -> Result<Self>;
        pub async fn run(self) -> Result<()>;
        pub async fn shutdown(&self) -> Result<()>;
    }
}

pub mod utils {
    pub mod error {
        pub type Result<T> = std::result::Result<T, Error>;

        pub struct Error { /* ... */ }
        pub enum ErrorKind { /* ... */ }
    }

    pub mod logging {
        pub fn init(debug: bool) -> Result<()>;
    }
}
```

### Crate Features

```toml
[features]
default = ["vaapi"]

# Hardware acceleration
vaapi = []

# Metrics and monitoring
metrics = ["dep:prometheus"]

# Static linking (vendor C dependencies)
vendored = []

# Development features
dev = ["metrics"]
```

---

## NAMING CONVENTIONS SUMMARY

### Files and Directories
- **Source files:** `snake_case.rs`
- **Directories:** `snake_case/`
- **Test files:** `{name}_test.rs`
- **Benchmark files:** `{name}_benchmark.rs` or `{name}.rs`

### Rust Code
- **Modules:** `snake_case`
- **Structs:** `PascalCase`
- **Enums:** `PascalCase`
- **Traits:** `PascalCase`
- **Functions:** `snake_case`
- **Methods:** `snake_case`
- **Variables:** `snake_case`
- **Constants:** `SCREAMING_SNAKE_CASE`
- **Statics:** `SCREAMING_SNAKE_CASE`
- **Type parameters:** `T`, `U`, `V` (single uppercase)
- **Lifetimes:** `'a`, `'b`, `'c` (single lowercase)

### Acronyms in Names
- **In types:** Keep uppercase: `RdpServer`, `TLSConfig`, `HTTPClient`
- **In functions:** Lowercase: `create_rdp_server()`, `init_tls()`
- **Exception:** Well-known types: `Io`, `Url` (not `IO`, `URL`)

### Module Names
- Singular for single concept: `video`, `audio`, `input`
- Plural for collections: `utils`, `channels`, `keymaps`
- Descriptive: `remote_desktop` not `rd`, `clipboard` not `clip`

---

## ARCHITECTURAL CONSTRAINTS

### Module Organization MUST
1. Follow layer architecture (no upward dependencies)
2. Keep modules focused (single responsibility)
3. Limit module size (< 1000 lines per file)
4. Use clear naming (no abbreviations unless standard)
5. Document public APIs completely

### Module Organization MUST NOT
1. Create circular dependencies
2. Expose implementation details
3. Use glob imports in production code
4. Mix concerns in single module
5. Have untested public APIs

### File Organization MUST
1. One primary type per file
2. Tests in same file (unit tests)
3. Integration tests in tests/ directory
4. Related helpers in same file
5. Clear module hierarchy

---

**END OF PROJECT STRUCTURE SPECIFICATION**

All implementation MUST conform to this structure. Deviations require specification update.
