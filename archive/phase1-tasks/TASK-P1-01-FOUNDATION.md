# TASK P1-01: PROJECT FOUNDATION & CONFIGURATION
**Task ID:** TASK-P1-01
**Phase:** 1
**Milestone:** Foundation
**Duration:** 3-5 days
**Assigned To:** [Agent/Developer Name]
**Dependencies:** None (first task)
**Status:** NOT_STARTED

---

## TASK OVERVIEW

### Objective
Set up the complete project structure, configuration system, logging infrastructure, and build system for the WRD-Server project. This task creates the foundation that all subsequent tasks will build upon.

### Success Criteria
- ✅ `cargo build` completes without errors
- ✅ `cargo test` passes all tests
- ✅ `wrd-server --help` displays help information
- ✅ Configuration loads from file correctly
- ✅ CLI argument overrides work
- ✅ Logging outputs to console and file
- ✅ All validation tests pass

### Deliverables
1. Complete directory structure
2. `Cargo.toml` with all dependencies
3. Configuration module (`src/config/`)
4. Main entry point (`src/main.rs`)
5. Logging infrastructure
6. Example configuration file
7. Unit tests for configuration
8. Build and setup scripts

---

## TECHNICAL SPECIFICATION

### 1. Project Initialization

#### Step 1.1: Create Cargo Project
```bash
cargo new --bin wrd-server
cd wrd-server
```

#### Step 1.2: Create Cargo.toml
Copy the EXACT `Cargo.toml` from document `02-TECHNOLOGY-STACK.md` section "CARGO.TOML - COMPLETE AND AUTHORITATIVE".

**CRITICAL:** Do NOT deviate from the specified dependencies and versions.

#### Step 1.3: Initial Build Test
```bash
cargo build
```
Expected: Builds successfully with warnings about unused dependencies (normal at this stage).

---

### 2. Directory Structure Creation

#### Step 2.1: Create All Source Directories
```bash
mkdir -p src/{config,server,rdp,portal,pipewire,video,input,clipboard,multimon,security,protocol,utils}
mkdir -p src/rdp/channels
mkdir -p src/video/encoder
mkdir -p tests/integration
mkdir -p tests/fixtures/test_data
mkdir -p benches
mkdir -p config
mkdir -p certs
mkdir -p scripts
mkdir -p docs
```

#### Step 2.2: Create Module Files
Create `mod.rs` in each directory with module documentation:

```rust
// Example: src/config/mod.rs
//! Configuration management for WRD-Server
//!
//! This module handles loading, validation, and management of server configuration
//! from TOML files, environment variables, and command-line arguments.

pub mod types;

// Module code will go here
```

Repeat for ALL directories listed in section 2.1.

#### Step 2.3: Create lib.rs
```rust
// src/lib.rs
//! WRD-Server Library
//!
//! Wayland Remote Desktop Server using RDP protocol.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod config;
pub mod server;
pub mod rdp;
pub mod portal;
pub mod pipewire;
pub mod video;
pub mod input;
pub mod clipboard;
pub mod multimon;
pub mod security;
pub mod protocol;
pub mod utils;
```

---

### 3. Configuration Module Implementation

#### Step 3.1: Define Configuration Types
Create `src/config/types.rs`:

```rust
//! Configuration type definitions

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Copy all struct definitions from 00-MASTER-SPECIFICATION.md
// Section 4.2 "Configuration Module"

// Include:
// - ServerConfig
// - SecurityConfig
// - VideoConfig
// - InputConfig
// - ClipboardConfig
// - MultiMonitorConfig
// - PerformanceConfig
// - LoggingConfig

// Copy all default value functions
```

**IMPORTANT:** Copy the EXACT struct definitions from the master specification. Do not modify field names, types, or defaults.

#### Step 3.2: Implement Configuration Module
Create `src/config/mod.rs`:

```rust
//! Configuration management
//!
//! Handles loading, validation, and merging of configuration from:
//! - TOML files
//! - Environment variables
//! - CLI arguments

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::net::SocketAddr;
use anyhow::{Result, Context};

pub mod types;

// Use types from types.rs
use types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub security: SecurityConfig,
    pub video: VideoConfig,
    pub input: InputConfig,
    pub clipboard: ClipboardConfig,
    pub multimon: MultiMonitorConfig,
    pub performance: PerformanceConfig,
    pub logging: LoggingConfig,
}

impl Config {
    /// Load configuration from file
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context(format!("Failed to read config file: {}", path))?;

        let config: Config = toml::from_str(&content)
            .context("Failed to parse config file")?;

        config.validate()?;
        Ok(config)
    }

    /// Create default configuration
    pub fn default_config() -> Result<Self> {
        // Copy implementation from master specification
        Ok(Config {
            server: ServerConfig {
                listen_addr: "0.0.0.0:3389".to_string(),
                max_connections: 10,
                session_timeout: 0,
                use_portals: true,
            },
            security: SecurityConfig {
                cert_path: PathBuf::from("/etc/wrd-server/cert.pem"),
                key_path: PathBuf::from("/etc/wrd-server/key.pem"),
                enable_nla: true,
                auth_method: "pam".to_string(),
                require_tls_13: true,
            },
            video: VideoConfig {
                encoder: "auto".to_string(),
                vaapi_device: PathBuf::from("/dev/dri/renderD128"),
                target_fps: 30,
                bitrate: 4000,
                damage_tracking: true,
                cursor_mode: "metadata".to_string(),
            },
            input: InputConfig {
                use_libei: true,
                keyboard_layout: "auto".to_string(),
                enable_touch: false,
            },
            clipboard: ClipboardConfig {
                enabled: true,
                max_size: 10485760,  // 10 MB
                allowed_types: vec![],
            },
            multimon: MultiMonitorConfig {
                enabled: true,
                max_monitors: 4,
            },
            performance: PerformanceConfig {
                encoder_threads: 0,
                network_threads: 0,
                buffer_pool_size: 16,
                zero_copy: true,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                log_dir: None,
                metrics: true,
            },
        })
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate listen address
        self.server.listen_addr.parse::<SocketAddr>()
            .context("Invalid listen address")?;

        // Validate cert paths exist
        if !self.security.cert_path.exists() {
            anyhow::bail!("Certificate not found: {:?}", self.security.cert_path);
        }
        if !self.security.key_path.exists() {
            anyhow::bail!("Private key not found: {:?}", self.security.key_path);
        }

        // Validate encoder choice
        match self.video.encoder.as_str() {
            "vaapi" | "openh264" | "auto" => {},
            _ => anyhow::bail!("Invalid encoder: {}", self.video.encoder),
        }

        // Validate cursor mode
        match self.video.cursor_mode.as_str() {
            "embedded" | "metadata" | "hidden" => {},
            _ => anyhow::bail!("Invalid cursor mode: {}", self.video.cursor_mode),
        }

        Ok(())
    }

    /// Override config with CLI arguments
    pub fn with_overrides(mut self, args: &crate::Args) -> Self {
        if let Some(listen) = &args.listen {
            self.server.listen_addr = format!("{}:{}", listen, args.port);
        } else {
            // Just update port
            if let Ok(mut addr) = self.server.listen_addr.parse::<SocketAddr>() {
                addr.set_port(args.port);
                self.server.listen_addr = addr.to_string();
            }
        }

        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::default_config().expect("Failed to create default config")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default_config().unwrap();
        assert_eq!(config.server.listen_addr, "0.0.0.0:3389");
        assert!(config.server.use_portals);
        assert_eq!(config.video.target_fps, 30);
    }

    #[test]
    fn test_config_validation_invalid_address() {
        let mut config = Config::default_config().unwrap();
        config.server.listen_addr = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_encoder() {
        let mut config = Config::default_config().unwrap();
        config.video.encoder = "invalid_encoder".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_cursor_mode() {
        let mut config = Config::default_config().unwrap();
        config.video.cursor_mode = "invalid_mode".to_string();
        assert!(config.validate().is_err());
    }
}
```

---

### 4. Main Entry Point Implementation

#### Step 4.1: Create src/main.rs
```rust
//! WRD-Server - Wayland Remote Desktop Server
//!
//! Entry point for the server binary.

use anyhow::Result;
use clap::Parser;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
// mod server;  // Commented out until implemented
// ... other modules commented for now

use config::Config;
// use server::Server;  // Commented until implemented

#[derive(Parser, Debug)]
#[command(name = "wrd-server")]
#[command(version, about = "Wayland Remote Desktop Server", long_about = None)]
pub struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "/etc/wrd-server/config.toml")]
    pub config: String,

    /// Listen address
    #[arg(short, long, env = "WRD_LISTEN_ADDR")]
    pub listen: Option<String>,

    /// Listen port
    #[arg(short, long, env = "WRD_PORT", default_value = "3389")]
    pub port: u16,

    /// Verbose logging (can be specified multiple times)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Log format (json|pretty|compact)
    #[arg(long, default_value = "pretty")]
    pub log_format: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    init_logging(&args)?;

    info!("Starting WRD-Server v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = Config::load(&args.config)
        .or_else(|e| {
            tracing::warn!("Failed to load config: {}, using defaults", e);
            Config::default_config()
        })?;

    // Override config with CLI args
    let config = config.with_overrides(&args);

    info!("Configuration loaded successfully");
    tracing::debug!("Config: {:?}", config);

    // TODO: Create and start server (in future tasks)
    // let server = Server::new(config).await?;
    // server.run().await?;

    info!("Server would start here (not yet implemented)");

    // For now, just wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    info!("Shutdown signal received, exiting");

    Ok(())
}

fn init_logging(args: &Args) -> Result<()> {
    let log_level = match args.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new(format!("wrd_server={},warn", log_level))
        });

    match args.log_format.as_str() {
        "json" => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().json())
                .init();
        }
        "compact" => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().compact())
                .init();
        }
        _ => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().pretty())
                .init();
        }
    }

    Ok(())
}
```

---

### 5. Example Configuration File

#### Step 5.1: Create config/wrd-server.toml
Copy the EXACT configuration from `02-TECHNOLOGY-STACK.md` section "Default Configuration File".

#### Step 5.2: Create Placeholder Certificates
```bash
# For testing only - real certs needed for production
mkdir -p certs
cd certs
openssl req -x509 -newkey rsa:4096 -nodes \
    -keyout test-key.pem \
    -out test-cert.pem \
    -days 365 \
    -subj "/CN=wrd-server-test"
```

Update `config/wrd-server.toml` to point to test certificates:
```toml
[security]
cert_path = "certs/test-cert.pem"
key_path = "certs/test-key.pem"
```

---

### 6. Build and Setup Scripts

#### Step 6.1: Create scripts/setup.sh
```bash
#!/bin/bash
# Development environment setup script

set -e

echo "Setting up WRD-Server development environment..."

# Check Rust installation
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust not installed. Install from https://rustup.rs/"
    exit 1
fi

echo "✓ Rust found: $(rustc --version)"

# Install required components
rustup component add clippy rustfmt llvm-tools-preview

# Create necessary directories
mkdir -p certs
mkdir -p logs

# Generate test certificates if they don't exist
if [ ! -f certs/test-cert.pem ]; then
    echo "Generating test certificates..."
    openssl req -x509 -newkey rsa:4096 -nodes \
        -keyout certs/test-key.pem \
        -out certs/test-cert.pem \
        -days 365 \
        -subj "/CN=wrd-server-test"
    echo "✓ Test certificates generated"
fi

# Check system dependencies
echo "Checking system dependencies..."
for pkg in libwayland-dev libpipewire-0.3-dev libva-dev; do
    if ! pkg-config --exists ${pkg%-dev} 2>/dev/null; then
        echo "WARNING: $pkg not found (optional for build, required for runtime)"
    fi
done

echo ""
echo "Setup complete!"
echo "Run 'cargo build' to build the project"
```

Make executable:
```bash
chmod +x scripts/setup.sh
```

#### Step 6.2: Create scripts/build.sh
```bash
#!/bin/bash
# Build script

set -e

echo "Building WRD-Server..."

# Format check
echo "Checking formatting..."
cargo fmt -- --check || {
    echo "ERROR: Code not formatted. Run 'cargo fmt'"
    exit 1
}

# Clippy check
echo "Running clippy..."
cargo clippy -- -D warnings || {
    echo "ERROR: Clippy warnings found"
    exit 1
}

# Build
echo "Building..."
cargo build --all-features

echo "✓ Build successful"
```

Make executable:
```bash
chmod +x scripts/build.sh
```

#### Step 6.3: Create scripts/test.sh
```bash
#!/bin/bash
# Test script

set -e

echo "Running tests..."

# Unit tests
cargo test --lib --all-features

# Integration tests (when they exist)
# cargo test --test '*' --all-features

# Doc tests
cargo test --doc --all-features

echo "✓ All tests passed"
```

Make executable:
```bash
chmod +x scripts/test.sh
```

---

### 7. Testing

#### Step 7.1: Verify Build
```bash
cargo build
```
Expected output: Successful compilation

#### Step 7.2: Run Tests
```bash
cargo test
```
Expected: All config tests pass

#### Step 7.3: Test CLI
```bash
cargo run -- --help
```
Expected: Help text displays

```bash
cargo run -- --config config/wrd-server.toml
```
Expected: Loads config, logs startup message, waits for Ctrl+C

#### Step 7.4: Test Configuration Loading
```bash
# Should fail (certs don't exist yet for default path)
cargo run -- --config /nonexistent/config.toml
# Expected: Falls back to defaults, but errors on cert validation

# Should work
cargo run -- --config config/wrd-server.toml
# Expected: Loads successfully
```

---

## VERIFICATION CHECKLIST

Before marking this task complete, verify ALL of the following:

### Build Verification
- [ ] `cargo build` completes without errors
- [ ] `cargo build --release` completes without errors
- [ ] `cargo clippy` shows no warnings
- [ ] `cargo fmt --check` shows no formatting issues

### Test Verification
- [ ] `cargo test` passes all tests
- [ ] All config validation tests pass
- [ ] Config loading from file works
- [ ] Default config creation works

### Functionality Verification
- [ ] `wrd-server --help` displays help
- [ ] `wrd-server --version` displays version
- [ ] Config file loads correctly
- [ ] CLI argument overrides work (--port, --listen)
- [ ] Logging outputs correctly (pretty/json/compact formats)
- [ ] Verbose flag increases log level

### File Structure Verification
- [ ] All directories created as specified
- [ ] All `mod.rs` files created
- [ ] `src/lib.rs` exists with all modules listed
- [ ] Example config file exists
- [ ] Test certificates generated
- [ ] All scripts created and executable

### Documentation Verification
- [ ] All public items have rustdoc comments
- [ ] `cargo doc --open` generates documentation
- [ ] README.md exists with basic information

---

## COMMON ISSUES AND SOLUTIONS

### Issue: "cannot find value `Args` in this scope"
**Solution:** The Args struct is defined in main.rs. Make sure it's public and accessible where needed.

### Issue: Cert validation fails
**Solution:** Update config/wrd-server.toml to point to test certificates in certs/ directory.

### Issue: Missing system libraries
**Solution:** Run dependency installation script from 02-TECHNOLOGY-STACK.md for your platform.

### Issue: "failed to parse toml"
**Solution:** Check TOML syntax in config file. Use `toml-cli` or online validator.

---

## DELIVERABLE CHECKLIST

Mark each deliverable as complete:

- [ ] Project structure created
- [ ] Cargo.toml configured with all dependencies
- [ ] Configuration module implemented (`src/config/`)
- [ ] Main entry point implemented (`src/main.rs`)
- [ ] Logging infrastructure working
- [ ] Example configuration file created
- [ ] Test certificates generated
- [ ] Build scripts created
- [ ] Unit tests written and passing
- [ ] All verification checks passed
- [ ] Documentation complete

---

## HANDOFF NOTES

### For Next Task (TASK-P1-02: Security Module)
The next task will implement the security module which requires:
- The Config struct from this task
- Access to certificate paths
- TLS configuration from config

Ensure Config is well-documented and the SecurityConfig struct is correctly defined.

### Integration Points
This task creates the foundation. Future tasks will:
- Add modules to `src/lib.rs`
- Use the Config struct
- Add their own config sections
- Use the logging infrastructure

---

## COMPLETION CRITERIA

This task is considered COMPLETE when:
1. All items in "Verification Checklist" are checked
2. All items in "Deliverable Checklist" are checked
3. Code review passes (if applicable)
4. All tests pass
5. Documentation is complete

**Estimated Completion Time:** 3-5 days for experienced Rust developer

---

**END OF TASK SPECIFICATION**

Report completion status and any blockers or deviations from this specification.
