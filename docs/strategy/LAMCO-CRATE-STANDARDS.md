# Lamco Crate Standards
## Quality, Documentation, and Publication Requirements
**Date:** 2025-12-11
**Purpose:** Establish consistent standards for all Lamco open source crates
**Applies to:** All crates published under lamco-* naming

---

## 1. CODE QUALITY STANDARDS

### 1.1 Rust Edition and Toolchain

**Required:**
- Rust edition: `2021`
- MSRV (Minimum Supported Rust Version): Latest stable minus 2 versions
- Toolchain: stable (not nightly features)

**Rationale:** Balance between modern features and wide compatibility.

---

### 1.2 Formatting (rustfmt)

**Configuration file:** `rustfmt.toml` (root of workspace)

```toml
max_width = 120
reorder_imports = true
imports_granularity = "Module"
group_imports = "StdExternalCrate"
```

**Enforcement:**
- All code MUST pass `cargo fmt --check`
- CI fails on formatting violations
- No exceptions

**Rationale:** Consistent formatting reduces bikeshedding, improves readability. Based on IronRDP configuration (proven).

---

### 1.3 Linting (Clippy)

**Configuration:** Workspace-level lints in root Cargo.toml

**Required Lint Categories (Based on IronRDP's Comprehensive Set):**

**Unsafe Code:**
```toml
[workspace.lints.rust]
unsafe_op_in_unsafe_fn = "warn"
invalid_reference_casting = "warn"
unused_unsafe = "warn"

[workspace.lints.clippy]
undocumented_unsafe_blocks = "warn"
multiple_unsafe_ops_per_block = "warn"
missing_safety_doc = "warn"
```

**Correctness:**
```toml
[workspace.lints.clippy]
as_conversions = "warn"
cast_lossless = "warn"
cast_possible_truncation = "warn"
cast_possible_wrap = "warn"
unwrap_used = "warn"
panic = "warn"
```

**Style and Readability:**
```toml
[workspace.lints.rust]
elided_lifetimes_in_paths = "warn"
single_use_lifetimes = "warn"
unreachable_pub = "warn"

[workspace.lints.clippy]
similar_names = "warn"
wildcard_imports = "warn"
```

**Performance:**
```toml
[workspace.lints.clippy]
large_futures = "warn"
rc_buffer = "warn"
or_fun_call = "warn"
```

**Full lint list:** See IronRDP Cargo.toml (70+ lints)

**Per-crate application:**
```toml
[lints]
workspace = true
```

**Enforcement:**
- All code MUST pass `cargo clippy -- -D warnings`
- CI fails on clippy warnings
- No `#[allow]` without inline comment explaining why

---

### 1.4 Logging and Tracing

**Required:** Use `tracing` crate (not `log` directly)

**Import pattern:**
```rust
use tracing::{debug, error, info, trace, warn};
```

**Logging levels:**
- `error!()` - Unrecoverable errors, failures
- `warn!()` - Unexpected conditions, degraded functionality
- `info!()` - Important state transitions, lifecycle events (MINIMAL)
- `debug!()` - Detailed diagnostic information (normal operations)
- `trace!()` - Very detailed debugging (every step)

**Style rules (from IronRDP STYLE.md):**
```rust
// GOOD - Structured fields
info!(?server_addr, "Looked up server address");
error!(?error, "Active stage failed");

// BAD - Printf-style
info!("Looked up server address: {server_addr}");
error!("Active stage failed: {}", error);
```

**Messages:**
- Start with capital letter
- NO period at end
- Be concise

**Example:**
```rust
info!("Connect to RDP host"); // Good
info!("connect to RDP host."); // Bad
```

**Dependency:**
```toml
tracing = { version = "0.1", features = ["log"] }
```

**For application crates only** (not libraries):
```toml
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

**Rationale:** Structured logging is superior, allows filtering, follows IronRDP patterns.

---

### 1.5 Error Handling

**Pattern:** Use `thiserror` for library errors, `anyhow` for applications

**Library crates (lamco-*):**
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("I/O error")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, MyError>;
```

**Error message format (from IronRDP STYLE.md):**
- Lowercase start
- No period at end
- Concise single sentence

**Good:**
```rust
#[error("invalid X.509 certificate")]
#[error("connection timeout after {0}ms")]
```

**Bad:**
```rust
#[error("Invalid X.509 certificate.")]
#[error("The connection timed out.")]
```

**Application crates (lamco-rdp-portal-server, lamco-rdp-vdi-server):**
```rust
use anyhow::{Context, Result};

fn main() -> anyhow::Result<()> {
    do_thing().context("failed to do thing")?;
    Ok(())
}
```

**Rationale:** Library errors must be composable and typed. Application errors can be catch-all.

---

### 1.6 Testing Requirements

**Minimum requirements:**
- Unit tests for core logic
- Integration tests for public APIs
- Examples that compile and run

**Test organization:**
```
crate-name/
├── src/
│   ├── lib.rs
│   └── module.rs  // #[cfg(test)] mod tests { }
├── tests/
│   └── integration_test.rs
└── examples/
    └── basic_usage.rs
```

**Testing pattern:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        let result = function();
        assert_eq!(result, expected);
    }
}
```

**CI requirement:**
- `cargo test` must pass
- Examples must compile: `cargo build --examples`
- No ignored tests without comment explaining why

**Coverage target:** Not enforced, but aim for >60% of core logic.

**Rationale:** Basic testing ensures quality, examples ensure usability.

---

### 1.7 Dependencies

**Policy:**
- Minimize dependencies (fewer deps = faster compile, less breakage)
- Use well-maintained crates only
- Prefer standard ecosystem crates (tokio, serde, etc.)
- Document why each dependency is needed

**Version specification:**
- Use caret requirements: `"1.0"` (not `">=1.0"` or `"*"`)
- Don't specify patch versions unless required
- Use `workspace.dependencies` for shared deps

**Example:**
```toml
[dependencies]
tokio = { version = "1.35", features = ["rt", "sync"] }  # Async runtime
serde = { version = "1.0", features = ["derive"] }       # Serialization
tracing = "0.1"                                          # Logging
```

**Feature flags:**
- Optional dependencies should be feature-gated
- Feature names match dependency names
```toml
[dependencies]
serde = { version = "1.0", optional = true }

[features]
default = []
serde = ["dep:serde"]
```

**Rationale:** Clear dependency management reduces supply chain risk and compile time.

---

### 1.8 Public API Design

**Principles:**
- Minimize public surface (only export what's needed)
- Use semantic versioning strictly
- Breaking changes require major version bump
- Document all public items

**Visibility:**
```rust
// Public API
pub struct PublicType { }
pub fn public_function() { }

// Internal (crate-private)
pub(crate) struct InternalType { }
pub(crate) fn internal_function() { }
```

**Re-exports:**
```rust
// In lib.rs
pub use crate::module::PublicType;

// NOT THIS (don't expose internal structure)
pub mod module { pub struct PublicType; }
```

**Traits for extension:**
```rust
pub trait Backend {
    fn required_method(&self) -> Result<()>;

    // Optional with default
    fn optional_method(&self) -> Result<()> {
        Ok(())
    }
}
```

**Rationale:** Clean public APIs reduce breaking changes, improve usability.

---

## 2. DOCUMENTATION STANDARDS

### 2.1 README.md Requirements

**MUST HAVE sections:**

```markdown
# crate-name

Brief one-line description.

## Overview

2-3 paragraph explanation of what the crate does and why it exists.

## Features

- Feature 1
- Feature 2
- Feature 3

## Usage

Basic code example showing most common use case.

```rust
use crate_name::Thing;

fn main() {
    let thing = Thing::new();
    thing.do_something();
}
```

## Examples

See `examples/` directory for complete examples.

## About Lamco

This crate is part of the Lamco RDP Server project. Lamco develops RDP
server solutions for Wayland/Linux.

**Open source foundation:** Portal integration, protocol components
**Commercial products:** Lamco RDP Server, Lamco VDI

Learn more: https://lamco.io

## License

Licensed under either of Apache License, Version 2.0 or MIT license at
your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you shall be dual licensed as
above, without any additional terms or conditions.
```

**Keep it:**
- Concise (< 200 lines)
- Practical (working examples)
- Clear about Lamco's commercial nature
- Professional (no emojis, no marketing speak)

**Rationale:** Users need quick understanding, working examples, and clear licensing.

---

### 2.2 Rustdoc Requirements

**MUST document:**
- All `pub` items (functions, structs, enums, traits)
- Module-level docs (`//!` at top of each module)
- Examples in doc comments where helpful

**Style (from IronRDP):**
```rust
/// Brief one-line description.
///
/// Longer explanation if needed. Can be multiple paragraphs.
///
/// # Examples
///
/// ```
/// use crate_name::Thing;
/// let thing = Thing::new();
/// ```
///
/// # Errors
///
/// Returns `Error::InvalidConfig` if configuration is invalid.
pub fn function() -> Result<()> {
    Ok(())
}
```

**Module docs:**
```rust
//! Brief module description.
//!
//! Longer explanation of what this module provides and how to use it.
```

**Link to specs (when relevant):**
```rust
/// [2.2.3.1] Format List PDU
///
/// Sent by either client or server when clipboard is updated.
///
/// [2.2.3.1]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpeclip/...
pub struct FormatList { }
```

**Enforcement:**
- Warning for missing docs (initially)
- Error for missing docs (after v1.0)
- CI checks doc generation succeeds

---

### 2.3 Examples

**Requirements:**
- Minimum 1 example per crate showing basic usage
- Examples MUST compile
- Examples MUST be realistic (not toy code)
- Keep examples concise (< 100 lines each)

**Location:** `examples/basic.rs`, `examples/advanced.rs`

**Testing:** CI runs `cargo build --examples`

---

### 2.4 CHANGELOG.md

**Required:** Every published crate maintains CHANGELOG.md

**Format:** Keep a Changelog standard (https://keepachangelog.com)

**Template:**
```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- New feature

### Changed
- Modified behavior

### Fixed
- Bug fix

## [0.1.0] - 2025-12-15

### Added
- Initial release
- Basic functionality
```

**Maintenance:** Update BEFORE each release

---

## 3. PUBLICATION STANDARDS

### 3.1 Version Numbering

**Strategy:** Semantic Versioning 2.0.0

- **Major (X.0.0):** Breaking changes
- **Minor (0.X.0):** New features, backward compatible
- **Patch (0.0.X):** Bug fixes, backward compatible

**Pre-1.0 rules:**
- 0.1.x → 0.2.0: Can have breaking changes (API unstable)
- After 1.0.0: Strict semver (breaking = major bump)

**Version synchronization:**
- Lamco crates do NOT need matching versions
- Each crate versions independently
- Document inter-crate compatibility in README

---

### 3.2 Cargo.toml Metadata

**Required fields:**
```toml
[package]
name = "lamco-crate-name"
version = "0.1.0"
edition = "2021"
authors = ["Lamco <contact@lamco.io>"]
description = "Brief one-line description (max 120 chars)"
license = "MIT OR Apache-2.0"
repository = "https://github.com/lamco-rdp/lamco"
homepage = "https://lamco.io"
documentation = "https://docs.rs/lamco-crate-name"
readme = "README.md"
keywords = ["rdp", "wayland", "portal", "category", "tech"]  # Max 5
categories = ["network-programming", "api-bindings"]          # Max 5
```

**Description rules:**
- Max 120 characters
- Single sentence, lowercase start (cargo convention)
- No "A crate for..." prefix (just state what it does)

**Good:**
```toml
description = "XDG Desktop Portal integration for Wayland applications"
description = "RDP clipboard protocol implementation with loop prevention"
```

**Bad:**
```toml
description = "A crate for integrating with XDG Desktop Portals."
description = "This is a RDP clipboard implementation."
```

---

### 3.3 License Files

**Required files in each crate:**
- `LICENSE-MIT` (full MIT license text)
- `LICENSE-APACHE` (full Apache 2.0 license text)

**Cargo.toml:**
```toml
license = "MIT OR Apache-2.0"
```

**README footer:**
```markdown
## License

Licensed under either of:
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.
```

**Rationale:** Dual license is Rust standard (maximizes usability, Apache provides patent protection).

---

### 3.4 CI/CD Requirements

**GitHub Actions workflow required:**

```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --all-features
      - run: cargo clippy --all-features -- -D warnings
      - run: cargo fmt --check
      - run: cargo build --examples
```

**MUST pass before publishing:**
- ✅ `cargo test` passes
- ✅ `cargo clippy -D warnings` passes
- ✅ `cargo fmt --check` passes
- ✅ `cargo build --examples` passes
- ✅ `cargo doc` builds without warnings

**Badge in README:**
```markdown
[![CI](https://github.com/lamco-rdp/lamco/actions/workflows/ci.yml/badge.svg)](https://github.com/lamco-rdp/lamco/actions)
```

---

### 3.5 Repository Structure

**For monorepo:**
```
lamco/
├── crates/
│   ├── lamco-portal/
│   │   ├── src/
│   │   ├── examples/
│   │   ├── tests/
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   ├── LICENSE-MIT
│   │   ├── LICENSE-APACHE
│   │   └── CHANGELOG.md
│   └── lamco-pipewire/
│       └── ...
├── Cargo.toml  # Workspace root
├── rustfmt.toml
├── .github/
│   └── workflows/
│       └── ci.yml
└── README.md
```

---

## 4. CONSISTENCY STANDARDS

### 4.1 Naming Conventions

**Crate naming:**
```
lamco-{category}-{component}

Platform Integration:
- lamco-portal
- lamco-pipewire
- lamco-video

RDP Protocol:
- lamco-rdp-clipboard
- lamco-rdp-input
- lamco-rdp-egfx

Products (proprietary):
- lamco-rdp-portal-server
- lamco-rdp-vdi-server
```

**Module naming:** Snake case
```rust
mod clipboard_manager;  // Good
mod ClipboardManager;   // Bad
```

**Type naming:** PascalCase
```rust
struct ClipboardManager { }  // Good
struct clipboard_manager { } // Bad
```

**Function naming:** Snake case
```rust
fn initialize_clipboard() { }  // Good
fn initializeClipboard() { }   // Bad
```

---

### 4.2 Code Comments

**Inline comments (from IronRDP STYLE.md):**
- Full sentences
- Start with capital
- End with period

```rust
// GOOD
// When building a library, `-` in the artifact name are replaced by `_`.
let name = package.replace('-', "_");

// BAD
// when building a library, `-` in the artifact name are replaced by `_`
let name = package.replace('-', "_");
```

**Exception:** Brief annotations don't need period
```rust
let x = 5; // VER
let y = 3; // RSV
```

---

### 4.3 Dependency Management

**Workspace shared dependencies:**
```toml
# Root Cargo.toml
[workspace.dependencies]
tokio = "1.35"
tracing = "0.1"
# etc.

# Per-crate Cargo.toml
[dependencies]
tokio = { workspace = true, features = ["rt", "sync"] }
tracing = { workspace = true }
```

**Rationale:** Version consistency across crates, easier updates.

---

## 5. PUBLICATION CHECKLIST

### 5.1 Pre-Publication Requirements

**Before `cargo publish`:**

- [ ] Version bumped in Cargo.toml
- [ ] CHANGELOG.md updated
- [ ] All tests pass (`cargo test`)
- [ ] Clippy clean (`cargo clippy -- -D warnings`)
- [ ] Formatted (`cargo fmt --check`)
- [ ] Examples compile (`cargo build --examples`)
- [ ] Docs build (`cargo doc --no-deps`)
- [ ] README.md up to date
- [ ] Git tag created (`v0.1.0`)
- [ ] Committed and pushed to GitHub

**Verification:**
```bash
cargo test --all-features
cargo clippy --all-features -- -D warnings
cargo fmt --check
cargo build --examples
cargo doc --no-deps
cargo package --list  # Review what will be published
cargo publish --dry-run
```

---

### 5.2 crates.io Publication

**First publication:**
```bash
cargo publish --dry-run  # Review
cargo publish           # Execute
```

**Subsequent publications:**
1. Update version in Cargo.toml
2. Update CHANGELOG.md
3. Commit changes
4. Create git tag: `git tag v0.2.0`
5. Push: `git push && git push --tags`
6. Publish: `cargo publish`

**Announcement:**
- Create GitHub release (same as git tag)
- Add release notes (copy from CHANGELOG.md)

---

### 5.3 Post-Publication

**Immediately after:**
- [ ] Verify crate appears on crates.io
- [ ] Check docs built on docs.rs
- [ ] Test installation: `cargo add crate-name`
- [ ] Announce (if significant release)

**Maintenance:**
- Monitor issues on GitHub
- Respond to questions within 48 hours
- Review PRs within 1 week
- Security updates: immediate

---

## 6. DOCUMENTATION CONTENT STANDARDS

### 6.1 README Structure (Detailed)

**Section 1: Title and Badges**
```markdown
# lamco-crate-name

[![Crates.io](https://img.shields.io/crates/v/lamco-crate-name.svg)](https://crates.io/crates/lamco-crate-name)
[![Documentation](https://docs.rs/lamco-crate-name/badge.svg)](https://docs.rs/lamco-crate-name)
[![CI](https://github.com/lamco-rdp/lamco/actions/workflows/ci.yml/badge.svg)](https://github.com/lamco-rdp/lamco/actions)

Brief one-line description.
```

**Section 2: Overview (2-3 paragraphs)**
- What problem does this solve?
- Who is this for?
- Key features/capabilities

**Section 3: Quick Start Example**
```markdown
## Quick Start

```rust
use lamco_crate::Thing;

fn main() -> Result<()> {
    let thing = Thing::new()?;
    thing.do_work()?;
    Ok(())
}
```
```

**Section 4: Features (optional if simple)**

**Section 5: Installation**
```markdown
## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
lamco-crate-name = "0.1"
```
```

**Section 6: Documentation**
```markdown
## Documentation

See [docs.rs/lamco-crate-name](https://docs.rs/lamco-crate-name) for full API documentation.
```

**Section 7: About Lamco**

**Section 8: License**

**Total length:** 100-300 lines (concise and practical)

---

### 6.2 Module Documentation

**Top of lib.rs:**
```rust
//! Brief crate description.
//!
//! Longer overview of what the crate provides. 2-3 paragraphs explaining
//! the problem space, approach, and key abstractions.
//!
//! # Examples
//!
//! ```
//! use crate_name::Thing;
//! let thing = Thing::new();
//! ```
//!
//! # Features
//!
//! - Feature 1
//! - Feature 2

#![deny(missing_docs)]  // After reaching API stability
```

**Top of each module:**
```rust
//! Module purpose and responsibilities.
//!
//! Brief explanation of what this module provides.
```

---

### 6.3 API Documentation

**Functions:**
```rust
/// Brief description of what this function does.
///
/// Longer explanation if behavior is non-obvious.
///
/// # Arguments
///
/// * `param` - Description of parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// Returns `Error::Thing` if condition occurs
///
/// # Examples
///
/// ```
/// let result = function(42)?;
/// ```
pub fn function(param: u32) -> Result<String> {
    Ok(String::new())
}
```

**Types:**
```rust
/// Brief description of this type.
///
/// Longer explanation of purpose, invariants, usage patterns.
#[derive(Debug, Clone)]
pub struct Thing {
    /// Description of this field
    pub field: String,
}
```

---

## 7. CRATE-SPECIFIC STANDARDS

### 7.1 Feature Flags

**When to use:**
- Optional functionality
- Optional dependencies
- Platform-specific code
- Different async runtimes

**Naming:**
- Match dependency name when possible
- Use kebab-case
- Be descriptive

**Example:**
```toml
[features]
default = ["tokio-runtime"]
tokio-runtime = ["dep:tokio"]
async-std-runtime = ["dep:async-std"]
serde = ["dep:serde"]
```

**Documentation:**
```markdown
## Features

- `tokio-runtime` (default) - Use Tokio async runtime
- `async-std-runtime` - Use async-std runtime
- `serde` - Enable serde serialization support
```

---

### 7.2 Platform Support

**Target:**
- Primary: Linux (x86_64, aarch64)
- Secondary: macOS (best-effort)
- Windows: Only if naturally portable

**Platform-specific code:**
```rust
#[cfg(target_os = "linux")]
mod linux_impl;

#[cfg(not(target_os = "linux"))]
compile_error!("This crate only supports Linux");
```

**Document platform requirements in README:**
```markdown
## Platform Support

- **Linux:** Full support (Wayland compositors)
- **macOS/Windows:** Not supported (Wayland-specific)
```

---

## 8. MAINTENANCE STANDARDS

### 8.1 Issue Response

**Timeframes:**
- Security issues: Within 24 hours
- Bugs: Within 48 hours (acknowledgment)
- Feature requests: Within 1 week
- Questions: Within 48 hours

**Labels:**
- `bug` - Something is broken
- `enhancement` - Feature request
- `question` - Help needed
- `good first issue` - For new contributors
- `help wanted` - Community can help

---

### 8.2 Pull Request Review

**Requirements:**
- Review within 1 week
- Run CI before merging
- Require passing tests
- Request changes if style violations
- Thank contributors

**Merge criteria:**
- Tests pass
- No clippy warnings
- Formatted correctly
- Documented if adding public API

---

### 8.3 Release Cadence

**Not time-based** - Release when ready:
- Bug fixes: Release quickly (within days)
- Features: Batch into releases (weeks)
- Breaking changes: Plan carefully (months)

**Process:**
1. Update CHANGELOG.md
2. Bump version
3. `cargo publish`
4. Create GitHub release
5. Tag version
6. Announce (if significant)

---

## 9. TOOLING STANDARDS

### 9.1 Required Tools

**For development:**
- `cargo` (latest stable)
- `rustfmt`
- `clippy`
- `cargo-deny` (check licenses and security)
- `cargo-audit` (security advisories)

**Installation:**
```bash
rustup component add rustfmt clippy
cargo install cargo-deny cargo-audit
```

---

### 9.2 Pre-Commit Checks

**Recommended git hook** (`.git/hooks/pre-commit`):
```bash
#!/bin/bash
cargo fmt --check || exit 1
cargo clippy --all-features -- -D warnings || exit 1
cargo test || exit 1
```

**Optional but recommended:**
```bash
cargo install cargo-husky
```

---

## 10. INTER-CRATE CONSISTENCY

### 10.1 Error Types

**Pattern:** Each crate defines its own error type

```rust
// In lamco-portal
#[derive(Error, Debug)]
pub enum PortalError { }
pub type Result<T> = std::result::Result<T, PortalError>;

// In lamco-pipewire
#[derive(Error, Debug)]
pub enum PipewireError { }
pub type Result<T> = std::result::Result<T, PipewireError>;
```

**NO shared error crate** (each crate is independent)

**Error conversion:**
```rust
// If crate A uses crate B
#[error("portal error")]
PortalFailed(#[from] lamco_portal::PortalError),
```

---

### 10.2 Async Runtime

**Standard:** Tokio for all async crates

**Rationale:**
- Ecosystem standard
- Your wrd-server uses Tokio
- Better to be consistent

**Optional runtime feature** (if needed):
```toml
[features]
default = ["tokio"]
tokio = ["dep:tokio"]
async-std = ["dep:async-std"]
```

---

## 11. QUALITY GATES

### 11.1 Before Extracting Crate

- [ ] Code compiles in isolation
- [ ] All `pub` items documented
- [ ] Error types defined
- [ ] Basic tests exist
- [ ] Examples work

### 11.2 Before First Publication (v0.1.0)

- [ ] README.md complete
- [ ] CHANGELOG.md created
- [ ] LICENSE files present
- [ ] Cargo.toml metadata complete
- [ ] CI passing
- [ ] Examples demonstrate real usage
- [ ] Docs build on docs.rs (test with `cargo doc`)

### 11.3 Before Stable Release (v1.0.0)

- [ ] API reviewed and stable
- [ ] Comprehensive tests (>60% coverage)
- [ ] Real-world usage (dogfooded in wrd-server)
- [ ] No known major bugs
- [ ] Documentation comprehensive
- [ ] Breaking changes considered

---

## 12. SPECIAL CONSIDERATIONS

### 12.1 IronRDP Dependency Handling

**For crates depending on IronRDP fork:**

**During development:**
```toml
[dependencies]
ironrdp-cliprdr = { git = "https://github.com/glamberson/IronRDP", branch = "fix/server-clipboard-announce" }
```

**After upstream accepts patch (if they do):**
```toml
[dependencies]
ironrdp-cliprdr = "0.4"  # Use crates.io version
```

**Document in README if using fork:**
```markdown
## Note on IronRDP Dependency

This crate currently depends on a fork of IronRDP with server clipboard
support. We are working with upstream to merge this functionality. Once
merged, we will switch to the official IronRDP crates.io releases.
```

---

### 12.2 Breaking Change Policy

**Pre-1.0:**
- Breaking changes allowed in minor versions
- Document in CHANGELOG.md
- Try to minimize churn

**Post-1.0:**
- Breaking changes ONLY in major versions
- Deprecate before removing (one major version warning)
- Provide migration guide

**Deprecation example:**
```rust
#[deprecated(since = "1.2.0", note = "use new_function instead")]
pub fn old_function() { }

pub fn new_function() { }
```

---

## SUMMARY CHECKLIST

Every Lamco crate MUST have:

**Code Quality:**
- [ ] Passes `cargo clippy -- -D warnings`
- [ ] Passes `cargo fmt --check`
- [ ] Uses `tracing` for logging (structured fields)
- [ ] Uses `thiserror` for errors
- [ ] Has tests (`cargo test` passes)

**Documentation:**
- [ ] README.md (with Lamco section)
- [ ] Module docs (`//!`)
- [ ] Public API docs (`///`)
- [ ] Examples in `examples/`
- [ ] CHANGELOG.md

**Publication:**
- [ ] Cargo.toml metadata complete
- [ ] LICENSE-MIT and LICENSE-APACHE files
- [ ] GitHub repository
- [ ] CI/CD workflow
- [ ] Version follows semver

**Consistency:**
- [ ] Follows Lamco naming conventions
- [ ] Error patterns consistent
- [ ] Logging patterns consistent
- [ ] Matches IronRDP quality where applicable

---

**END OF STANDARDS DOCUMENT**

Next: Create extraction and publication pipeline using these standards.
