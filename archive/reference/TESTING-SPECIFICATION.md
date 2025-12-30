# TESTING SPECIFICATION
**Document:** TESTING-SPECIFICATION.md
**Version:** 1.0
**Parent:** 00-MASTER-SPECIFICATION.md

## OVERVIEW
This document specifies all testing requirements, strategies, and procedures for WRD-Server.

## TEST COVERAGE REQUIREMENTS

### Minimum Coverage
- **Unit Tests:** > 80% line coverage
- **Integration Tests:** All major workflows
- **Performance Tests:** All critical paths
- **Compatibility Tests:** 3+ compositors, 2+ GPUs

## UNIT TESTING

### Test Structure
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Arrange
        let input = setup_test_data();

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected_value);
    }
}
```

### Required Unit Tests Per Module

**Config Module:**
- Default config creation
- Config file loading
- Validation (all error paths)
- CLI argument overrides
- Invalid config rejection

**Security Module:**
- TLS config loading
- Certificate generation
- PAM authentication (mocked)
- Session token generation
- Username validation

**RDP Module:**
- State transitions
- Capability negotiation
- PDU parsing
- Channel management

**Portal Module:**
- Session creation (integration-style)
- Error handling

**Video Module:**
- Encoder initialization
- Frame encoding
- Format conversion
- Damage tracking

**Input Module:**
- Scancode translation
- Coordinate transformation
- Event validation

**Clipboard Module:**
- Format conversion
- Size validation
- Type filtering

## INTEGRATION TESTING

### Test Suite Location
`tests/integration/`

### Required Integration Tests

**1. Connection Flow (`tests/integration/connection_test.rs`)**
```rust
#[tokio::test]
async fn test_basic_rdp_connection() {
    let config = Config::default_config().unwrap();
    let server = Server::new(config).await.unwrap();

    // Spawn server
    tokio::spawn(async move {
        server.run().await.unwrap();
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client (using FreeRDP library or TCP test)
    let stream = TcpStream::connect("127.0.0.1:3389").await.unwrap();
    assert!(stream.peer_addr().is_ok());
}
```

**2. Video Streaming (`tests/integration/video_test.rs`)**
- Portal session creation
- PipeWire connection
- Frame reception
- Encoding
- RDP transmission

**3. Input Injection (`tests/integration/input_test.rs`)**
- RDP input events received
- Translation to Wayland events
- Portal injection

**4. Clipboard Sync (`tests/integration/clipboard_test.rs`)**
- Client → Server
- Server → Client
- Format conversion
- Size limits

**5. Multi-Monitor (`tests/integration/multimon_test.rs`)**
- Monitor detection
- Multiple streams
- Layout calculation

## PERFORMANCE TESTING

### Benchmarks Location
`benches/`

### Required Benchmarks

**1. Encoding Performance (`benches/encoding.rs`)**
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_openh264_1080p(c: &mut Criterion) {
    let frame = create_test_frame(1920, 1080);
    let mut encoder = OpenH264Encoder::new(...).unwrap();

    c.bench_function("openh264_encode_1080p", |b| {
        b.iter(|| encoder.encode(black_box(&frame)))
    });
}

fn bench_vaapi_1080p(c: &mut Criterion) {
    if !VaApiEncoder::is_available(&config) {
        return;
    }

    let frame = create_test_frame(1920, 1080);
    let mut encoder = VaApiEncoder::new(...).unwrap();

    c.bench_function("vaapi_encode_1080p", |b| {
        b.iter(|| encoder.encode(black_box(&frame)))
    });
}

criterion_group!(benches, bench_openh264_1080p, bench_vaapi_1080p);
criterion_main!(benches);
```

**2. Pipeline Performance (`benches/pipeline.rs`)**
- End-to-end frame processing
- Damage tracking overhead
- Format conversion performance

### Performance Targets

| Metric | Target | Maximum |
|--------|--------|---------|
| Encoding latency (1080p, VA-API) | < 16ms | < 33ms |
| Encoding latency (1080p, OpenH264) | < 50ms | < 100ms |
| Frame rate | 30 FPS | - |
| Input latency | < 30ms | < 50ms |
| End-to-end latency (LAN) | < 50ms | < 100ms |

## COMPATIBILITY TESTING

### Test Matrix

| Compositor | Version | GPU | Encoder | Priority |
|------------|---------|-----|---------|----------|
| GNOME | 45+ | Intel | VA-API | HIGH |
| GNOME | 45+ | AMD | VA-API | HIGH |
| GNOME | 45+ | Any | OpenH264 | HIGH |
| KDE Plasma | 6.0+ | Intel | VA-API | HIGH |
| Sway | 1.8+ | Intel | VA-API | MEDIUM |
| Hyprland | Latest | Intel | VA-API | LOW |

### Client Compatibility

| Client | Version | OS | Priority |
|--------|---------|-----|----------|
| mstsc.exe | Windows 10 | Windows 10 | HIGH |
| mstsc.exe | Windows 11 | Windows 11 | HIGH |
| FreeRDP | 2.11+ | Linux | HIGH |
| rdesktop | Latest | Linux | LOW |

### Test Procedure

For each combination:
1. Start wrd-server
2. Connect client
3. Verify video displays
4. Test keyboard input
5. Test mouse input
6. Test clipboard (both directions)
7. Measure latency
8. Measure frame rate
9. Run for 10 minutes (stability)
10. Document results

## STRESS TESTING

### Load Tests

**1. Maximum Connections**
- Start server with max_connections = 10
- Connect 10 concurrent clients
- Verify all connect successfully
- Verify 11th connection rejected

**2. Long-Running Sessions**
- Connect client
- Run for 24 hours
- Monitor memory usage (should be stable)
- Monitor CPU usage
- Check for leaks

**3. Network Interruptions**
- Connect client
- Simulate network issues:
  - Packet loss (1%, 5%, 10%)
  - Latency spikes (100ms, 500ms)
  - Temporary disconnection
- Verify graceful handling

**4. High Frame Rate**
- Connect client
- Generate 60 FPS content
- Verify encoding keeps up
- Monitor dropped frames

## SECURITY TESTING

### Security Test Cases

**1. TLS Validation**
- Verify only TLS 1.3 accepted
- Verify older TLS versions rejected
- Verify certificate validation
- Verify cipher suites

**2. Authentication**
- Valid credentials accepted
- Invalid credentials rejected
- Brute force protection (rate limiting)
- Session timeout enforcement

**3. Input Validation**
- Malformed RDP PDUs rejected
- Oversized PDUs rejected
- Invalid coordinates rejected
- XSS-style attacks blocked

**4. Resource Limits**
- Clipboard size limit enforced
- Connection limit enforced
- Memory limits enforced

## AUTOMATED TESTING

### CI/CD Pipeline
```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libwayland-dev libpipewire-0.3-dev \
            libva-dev libpam0g-dev
      - name: Run tests
        run: cargo test --all-features
      - name: Run clippy
        run: cargo clippy -- -D warnings
      - name: Check formatting
        run: cargo fmt -- --check

  bench:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run benchmarks
        run: cargo bench --no-fail-fast
```

### Test Commands

```bash
# Run all tests
cargo test --all-features

# Run unit tests only
cargo test --lib

# Run integration tests only
cargo test --test '*'

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run benchmarks
cargo bench

# Test coverage
cargo tarpaulin --all-features --workspace --timeout 120

# Memory leak detection
valgrind --leak-check=full target/debug/wrd-server
```

## TEST DATA

### Test Fixtures
`tests/fixtures/`
- Sample video frames (various formats)
- Test certificates
- Sample RDP PDUs
- Configuration files

### Mock Services
- Mock portal responses
- Mock PipeWire streams
- Mock RDP clients

## REGRESSION TESTING

### Regression Test Suite
When bugs are fixed, add regression tests:

```rust
#[test]
fn regression_issue_123_clipboard_crash() {
    // Test for issue #123: clipboard crashes on large data
    let data = vec![0u8; 20_000_000]; // 20MB
    let result = clipboard_manager.sync_to_server(data, CF_TEXT);
    assert!(result.is_ok());
}
```

## DOCUMENTATION TESTING

### Doc Tests
```rust
/// Encodes a video frame to H.264.
///
/// # Example
/// ```
/// use wrd_server::video::encoder::OpenH264Encoder;
///
/// let encoder = OpenH264Encoder::new(config, 1920, 1080).unwrap();
/// let frame = VideoFrame::new(...);
/// let encoded = encoder.encode(&frame).await.unwrap();
/// ```
pub async fn encode(&mut self, frame: &VideoFrame) -> Result<EncodedFrame>;
```

Run doc tests:
```bash
cargo test --doc
```

## TEST REPORTING

### Test Report Format
```
Test Results for wrd-server v0.1.0
===================================

Unit Tests:        ✓ 145 passed, 0 failed
Integration Tests: ✓ 12 passed, 0 failed
Doc Tests:         ✓ 23 passed, 0 failed
Coverage:          84.2% (target: 80%)

Performance Tests:
- Encoding (VA-API 1080p):  12.3ms (target: < 16ms) ✓
- Encoding (OpenH264 1080p): 45.2ms (target: < 50ms) ✓
- Input latency:            28.1ms (target: < 30ms) ✓

Compatibility Tests:
- GNOME 45 + Intel: ✓ PASS
- KDE 6 + Intel:    ✓ PASS
- Sway 1.8 + Intel: ✓ PASS

Security Tests:     ✓ All passed

Status: READY FOR RELEASE
```

## END OF TESTING SPECIFICATION
