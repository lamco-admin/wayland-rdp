# Contributing to lamco-rdp-server

Thank you for your interest in contributing to `lamco-rdp-server`! This document provides guidelines for contributing to the project.

## License

This project is licensed under the **Business Source License 1.1 (BSL)**. By contributing, you agree that your contributions will be licensed under the same BSL 1.1 terms.

**Important points:**
- Contributions become BSL 1.1 licensed immediately
- The codebase automatically converts to Apache-2.0 on December 31, 2028
- You retain copyright to your contributions
- See [LICENSE](LICENSE) for complete terms

## Getting Started

### Prerequisites

- Rust 1.77 or later
- Linux with Wayland
- PipeWire 0.3.50+
- XDG Desktop Portal with ScreenCast support
- Basic familiarity with Rust async programming (tokio)

### Development Setup

```bash
# Clone repository
git clone https://github.com/lamco-admin/lamco-rdp-server.git
cd lamco-rdp-server

# Install system dependencies (Debian/Ubuntu)
sudo apt install -y \
    pkg-config \
    libpipewire-0.3-dev \
    libspa-0.2-dev \
    libasound2-dev \
    libssl-dev

# Build
cargo build

# Run tests
cargo test

# Run with development config
cargo run -- -c config.toml -vv
```

## How to Contribute

### 1. Open an Issue First

Before starting significant work:

1. Check existing issues for duplicates
2. Open a new issue describing your proposed change
3. Wait for feedback from maintainers
4. Discuss implementation approach

This prevents wasted effort on changes that won't be accepted.

### 2. Fork and Branch

```bash
# Fork the repository on GitHub
# Clone your fork
git clone https://github.com/YOUR-USERNAME/lamco-rdp-server.git
cd lamco-rdp-server

# Create a feature branch
git checkout -b feature/your-feature-name
```

### 3. Make Changes

- Write clean, readable code
- Follow Rust conventions and idioms
- Add tests for new functionality
- Update documentation as needed
- Keep commits focused and atomic

### 4. Test Your Changes

```bash
# Format code
cargo fmt

# Check with clippy
cargo clippy -- -D warnings

# Run tests
cargo test

# Test manually
cargo run -- -c config.toml -vv
```

### 5. Commit and Push

```bash
# Stage changes
git add .

# Commit with descriptive message
git commit -m "Add feature: brief description

Longer explanation of what this change does and why.
Fixes #123"

# Push to your fork
git push origin feature/your-feature-name
```

### 6. Submit Pull Request

1. Go to the original repository on GitHub
2. Click "New Pull Request"
3. Select your fork and branch
4. Fill out the PR template:
   - **Description**: What does this PR do?
   - **Motivation**: Why is this change needed?
   - **Testing**: How was this tested?
   - **Breaking Changes**: Any breaking changes?
5. Submit the PR

## Code Style

### Rust Style

Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/):

- Use `cargo fmt` for formatting
- Run `cargo clippy` and fix all warnings
- Use meaningful variable and function names
- Document public APIs with doc comments
- Prefer idiomatic Rust patterns

**Example:**

```rust
/// Captures a frame from the PipeWire stream
///
/// # Arguments
///
/// * `timeout` - Maximum time to wait for a frame
///
/// # Returns
///
/// Returns `Ok(Frame)` on success, or an error if capture fails
///
/// # Errors
///
/// Returns `Error::Timeout` if no frame received within timeout
pub async fn capture_frame(&mut self, timeout: Duration) -> Result<Frame> {
    // Implementation
}
```

### Async Code

- Use `tokio` for async runtime
- Prefer structured concurrency (tokio::spawn with proper cleanup)
- Handle cancellation gracefully
- Avoid blocking operations in async context

### Error Handling

- Use `Result<T, Error>` for fallible operations
- Use `anyhow::Error` for application errors
- Use `thiserror` for library errors
- Provide context with `.context()` or `.with_context()`

**Example:**

```rust
use anyhow::{Context, Result};

fn load_config(path: &Path) -> Result<Config> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    toml::from_str(&contents)
        .context("Failed to parse config file")
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let config = Config::from_str("...").unwrap();
        assert_eq!(config.port, 3389);
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests

Place integration tests in `tests/`:

```rust
// tests/integration_test.rs
use lamco_rdp_server::*;

#[tokio::test]
async fn test_server_lifecycle() {
    // Test server startup and shutdown
}
```

### Testing Guidelines

- Write tests for new features
- Maintain or improve test coverage
- Test both success and failure cases
- Use property-based testing for complex logic (proptest)
- Mock external dependencies where appropriate

## Documentation

### Code Documentation

- Document all public APIs with `///` doc comments
- Include examples in doc comments
- Document panics, errors, and safety requirements
- Keep docs up to date with code changes

### User Documentation

Update relevant files:
- `README.md` - User-facing features
- `INSTALL.md` - Installation steps
- `CONFIGURATION.md` - Config options
- `CHANGELOG.md` - Notable changes

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Test additions/changes
- `chore`: Build process, dependencies

**Examples:**

```
feat(clipboard): add image format support

Implement clipboard sync for PNG and JPEG images.
Uses lamco-clipboard-core for format conversion.

Closes #123
```

```
fix(video): prevent frame queue overflow

Add backpressure mechanism to limit queue depth.
Fixes crashes under high load.

Fixes #456
```

## Pull Request Guidelines

### Before Submitting

- [ ] Code compiles without errors
- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated
- [ ] CHANGELOG.md updated (for notable changes)

### PR Description Template

```markdown
## Description
Brief description of changes

## Motivation
Why is this change needed?

## Changes
- List of specific changes
- New files added
- Modified behavior

## Testing
How was this tested?
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed

## Breaking Changes
Any breaking changes? If yes, describe migration path.

## Related Issues
Closes #XXX
Fixes #YYY
```

## Review Process

1. Maintainer reviews PR
2. Automated CI checks run
3. Feedback provided (if changes needed)
4. Contributor addresses feedback
5. PR approved and merged

**Timeline:** Most PRs reviewed within 1-2 weeks. Complex changes may take longer.

## Areas Needing Contribution

### High Priority

- **Performance optimization**: Profile and optimize hot paths
- **Documentation**: Improve user guides and API docs
- **Testing**: Increase test coverage
- **Bug fixes**: Address open issues

### Medium Priority

- **Platform support**: Test on different Linux distributions
- **Features**: Multi-monitor improvements, touch input
- **Tools**: Better debugging tools, diagnostic utilities
- **Examples**: Usage examples and tutorials

### Low Priority

- **Packaging**: Flatpak, AppImage, distribution packages
- **Localization**: Internationalization support
- **UI**: Optional GUI for configuration

## Code of Conduct

### Our Standards

- Be respectful and constructive
- Welcome newcomers and help them learn
- Focus on what's best for the project
- Accept constructive criticism gracefully
- Show empathy toward other contributors

### Unacceptable Behavior

- Harassment or discriminatory language
- Personal attacks or insults
- Publishing others' private information
- Trolling or inflammatory comments
- Other conduct harmful to the community

### Enforcement

Violations may result in:
1. Warning
2. Temporary ban from project
3. Permanent ban from project

Contact: office@lamco.io

## Getting Help

- **Questions**: Open a GitHub Discussion
- **Bugs**: File a GitHub Issue
- **Security**: Email office@lamco.io (do not file public issue)
- **Chat**: [Coming Soon]

## Recognition

Contributors will be:
- Listed in CHANGELOG.md for their contributions
- Credited in release notes
- Added to CONTRIBUTORS file (if significant contribution)

## License Headers

Add this header to new Rust files:

```rust
// This file is part of lamco-rdp-server
// Licensed under the Business Source License 1.1
// See LICENSE file for details
```

## Release Process

Releases are managed by maintainers:

1. Version bump in Cargo.toml
2. Update CHANGELOG.md
3. Create git tag
4. Publish to crates.io
5. Create GitHub release

Contributors don't need to worry about releases.

## Questions?

Open a GitHub Discussion or email office@lamco.io

Thank you for contributing to lamco-rdp-server! 🚀
