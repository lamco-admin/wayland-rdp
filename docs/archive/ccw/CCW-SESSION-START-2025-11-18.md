# CCW Development Session - 2025-11-18

**Repository:** https://github.com/lamco-admin/wayland-rdp
**Branch:** main
**Last Commit:** 2645e5b (PipeWire integration complete)

---

## CRITICAL STANDARDS - READ FIRST

**ZERO TOLERANCE FOR:**
- ❌ Shortcuts of any kind
- ❌ Simplifications
- ❌ Stub implementations
- ❌ TODO comments without full implementation
- ❌ Placeholder code
- ❌ "We'll implement this later" statements

**MANDATORY REQUIREMENTS:**
- ✅ Follow specifications EXACTLY as written
- ✅ Implement EVERY feature in the spec
- ✅ Complete error handling for all code paths
- ✅ Full rustdoc documentation on all public items
- ✅ Comprehensive testing (unit + integration)
- ✅ Production-grade code quality
- ✅ NO skipping of "optional" features - implement everything

**If specifications are unclear or incomplete:**
1. Ask clarifying questions
2. Research the correct approach
3. Implement fully - never stub it out

---

## Current Project Status

### Completed Tasks ✅
- **P1-01:** Foundation & Configuration (fully implemented)
- **P1-02:** Security Module - TLS & Authentication (fully implemented)
- **P1-03:** Portal Integration (fully implemented)
- **P1-04:** PipeWire Integration (fully implemented THIS SESSION by CCC)

### Blocked Task ⏳
- **P1-05:** IronRDP Server Integration
  - **STATUS:** BLOCKED on upstream dependency fix
  - **Blocker:** sspi 0.16.1 incompatibility in IronRDP
  - **Tracking:** https://github.com/Devolutions/IronRDP/pull/1028
  - **Do NOT attempt** - wait for upstream fix

### Your Tasks (Priority Order)

---

## TASK 1: Implement Video Processing Pipeline

**Specification:** Create in `phase1-tasks/TASK-P1-05-VIDEO-PIPELINE.md` if it doesn't exist, OR use existing spec if present.

**Core Requirements:**

### 1.1 Video Frame Processing
Implement `src/video/processor.rs`:
- Connect to PipeWire frame output (uses `VideoFrame` from pipewire module)
- Frame buffer queue management
- Frame rate control and adaptive quality
- Damage region optimization
- Performance metrics

### 1.2 Format Preparation for RDP
Implement `src/video/converter.rs`:
- Convert VideoFrame to format suitable for RDP bitmap updates
- Handle stride alignment for RDP protocol
- Prepare metadata (damage regions, timestamps)
- Memory-efficient buffer handling

### 1.3 Frame Dispatcher
Implement `src/video/dispatcher.rs`:
- Route frames from multiple PipeWire streams
- Priority queue for frame processing
- Backpressure handling
- Frame drop decisions based on load

**Integration Points:**
- Input: `VideoFrame` from `crate::pipewire::VideoFrame`
- Output: Prepared frames ready for RDP encoding (when IronRDP available)
- Portal: Use monitor metadata from `crate::portal::StreamInfo`

**Testing:**
- Unit tests for all conversions
- Integration test connecting PipeWire → Video pipeline
- Performance benchmarks (<2ms processing time)

---

## TASK 2: Implement Comprehensive Testing Suite

**Specification:** Expand testing per `reference/TESTING-SPECIFICATION.md`

### 2.1 Portal Integration Tests
File: `tests/integration/portal_test.rs` (already exists, enhance it)
- Test with real D-Bus and portal
- Verify PipeWire FD acquisition
- Test input injection
- Multi-monitor session creation
- Error condition testing

### 2.2 PipeWire Integration Tests
File: `tests/integration/pipewire_test.rs` (exists, complete ignored tests)
- Run in actual Wayland session
- Connect to real PipeWire daemon
- Capture actual frames
- Test all format conversions with real data
- DMA-BUF detection and fallback
- Multi-stream coordination

### 2.3 End-to-End Integration Tests
File: `tests/integration/e2e_test.rs` (create)
- Portal session → PipeWire connection → Frame capture → Video pipeline
- Test complete data flow WITHOUT RDP (since IronRDP blocked)
- Verify frame timing and synchronization
- Test cleanup and resource management

### 2.4 Security Tests
File: `tests/security_integration.rs` (exists, expand)
- TLS handshake testing
- Certificate validation
- Authentication flow testing
- Security policy enforcement

**All tests must:**
- Handle actual system integration (not just mocks)
- Test both success and failure paths
- Validate performance requirements
- Check resource cleanup

---

## TASK 3: Configuration System Enhancements

**File:** `src/config/types.rs` and `src/config/mod.rs`

### 3.1 PipeWire Configuration
Add complete configuration for:
- Buffer pool sizes
- DMA-BUF enable/disable
- Format preferences
- Stream parameters (FPS, resolution)
- Recovery settings (retry counts, timeouts)
- Performance thresholds

### 3.2 Video Pipeline Configuration
Add configuration for:
- Frame queue depths
- Processing thread counts
- Quality adaptation parameters
- Damage region thresholds
- Performance targets

### 3.3 Configuration Validation
Implement:
- Schema validation for all config values
- Range checking
- Dependency validation (e.g., DMA-BUF requires GPU)
- Configuration file examples in `config/` directory

---

## TASK 4: Monitoring and Metrics

**File:** `src/utils/metrics.rs` (create)

### 4.1 Performance Metrics
Implement:
- Frame rate tracking (current, average, min, max)
- Latency measurements (capture, processing, total)
- CPU and memory usage per component
- Buffer pool utilization
- Frame drop counters

### 4.2 Health Monitoring
Implement:
- Component health checks
- Resource usage alerts
- Performance degradation detection
- Error rate tracking
- Recovery action logging

### 4.3 Metrics Export
Implement:
- Prometheus-compatible metrics endpoint (optional feature)
- JSON metrics dump
- Real-time metrics logging
- Performance report generation

---

## TASK 5: Documentation

### 5.1 API Documentation
- Ensure EVERY public function has rustdoc
- Add usage examples in doc comments
- Document all error conditions
- Explain safety invariants for unsafe code

### 5.2 Integration Guide
Create `docs/INTEGRATION-GUIDE.md`:
- How Portal module works
- How PipeWire module works
- How to test each component
- Common integration patterns
- Troubleshooting guide

### 5.3 Developer Guide
Create `docs/DEVELOPER-GUIDE.md`:
- Project architecture overview
- Module dependencies
- Testing strategies
- Performance profiling
- Debugging techniques

---

## IMPLEMENTATION GUIDELINES

### Code Quality Requirements

**Every file must have:**
- Module-level documentation explaining purpose
- Function-level documentation for all public items
- Error handling for all failure modes (no unwrap/expect in production code)
- Unit tests achieving >80% coverage
- Performance considerations documented

**Async Code:**
- Use tokio::sync primitives (Mutex, RwLock, mpsc)
- Avoid blocking operations in async contexts
- Handle cancellation gracefully
- Document Send/Sync requirements

**Unsafe Code:**
- Minimize use
- Document all safety invariants
- Explain why unsafe is necessary
- Ensure memory safety

**Error Handling:**
- Use Result types throughout
- Create specific error types (not anyhow in libraries)
- Implement proper error context
- Log errors appropriately

### Testing Requirements

**Unit Tests:**
- Test all public functions
- Test error conditions
- Test edge cases
- Use proptest for property-based testing where appropriate

**Integration Tests:**
- Test real system integration
- Mark as #[ignore] if require special environment
- Document test prerequisites
- Provide test data/fixtures

**Performance Tests:**
- Benchmark critical paths
- Verify performance requirements
- Use criterion for benchmarks
- Document performance characteristics

---

## BUILD & TEST COMMANDS

```bash
# Full build
cargo build --release

# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run integration tests (requires Wayland + PipeWire)
cargo test --test portal_test -- --ignored
cargo test --test pipewire_test -- --ignored

# Run specific module tests
cargo test --lib pipewire::
cargo test --lib portal::

# Check for issues
cargo clippy -- -D warnings

# Format code
cargo fmt

# Build examples
cargo build --examples

# Run example
cargo run --example pipewire_capture
```

---

## CURRENT CODEBASE STATE

### Working Modules
- ✅ `src/config/` - Configuration system
- ✅ `src/security/` - TLS, certificates, authentication
- ✅ `src/portal/` - Portal integration (5 files, fully implemented)
- ✅ `src/pipewire/` - PipeWire integration (9 files, 3,392 lines, fully implemented)

### Stub Modules (Need Implementation)
- ⚠️ `src/video/` - Video processing (has mod.rs stub + encoder stub)
- ⚠️ `src/rdp/` - RDP protocol (blocked on IronRDP fix, leave as stub)
- ⚠️ `src/clipboard/` - Clipboard (can implement if time allows)
- ⚠️ `src/input/` - Input handling (can implement if time allows)
- ⚠️ `src/multimon/` - Multi-monitor (integrated in PipeWire coordinator)
- ⚠️ `src/server/` - Server main loop (implement after video pipeline)
- ⚠️ `src/utils/` - Utilities (add metrics module)

### Dependencies Status
- ✅ Portal: ashpd 0.12.0, zbus 4.0.1
- ✅ PipeWire: pipewire 0.8, libspa 0.8
- ✅ TLS: tokio-rustls 0.26, rustls 0.23
- ⏳ RDP: ironrdp BLOCKED (do not add yet)

---

## DELIVERABLES FOR THIS SESSION

### Priority 1 (MUST COMPLETE)
1. Video processing pipeline implementation
2. Enhanced integration tests (run real Portal + PipeWire tests)
3. Configuration system completion
4. Metrics and monitoring system

### Priority 2 (COMPLETE IF TIME)
1. Clipboard integration (if spec exists)
2. Input handling utilities
3. Server main loop structure (without RDP)
4. Additional documentation

### Priority 3 (NICE TO HAVE)
1. Performance optimization
2. Additional examples
3. Benchmark suite expansion

---

## WHAT NOT TO DO

❌ **DO NOT** try to implement IronRDP integration (P1-05) - it's blocked on upstream
❌ **DO NOT** add ironrdp dependencies to Cargo.toml - they won't compile
❌ **DO NOT** create workarounds or temporary solutions - wait for proper fix
❌ **DO NOT** skip error handling or testing
❌ **DO NOT** use TODO comments - implement fully or don't commit
❌ **DO NOT** simplify specifications - follow them exactly

---

## SUCCESS CRITERIA

**Before marking session complete:**
- [ ] All code compiles without errors
- [ ] All unit tests pass
- [ ] cargo clippy shows no warnings
- [ ] cargo fmt applied to all files
- [ ] Every public function documented
- [ ] Integration tests created (even if marked #[ignore])
- [ ] No TODO/FIXME comments in production code
- [ ] All modules have comprehensive error handling

---

## QUESTIONS TO ASK IF NEEDED

**If you encounter:**
- Missing or unclear specifications
- Conflicting requirements
- Ambiguous API designs
- Performance trade-offs

**Then:**
1. Document the issue clearly
2. Propose 2-3 solutions with pros/cons
3. Ask for decision
4. Implement chosen solution FULLY

**NEVER:**
- Guess and implement without asking
- Create stub "to be completed later"
- Skip difficult parts

---

## REFERENCE DOCUMENTS

**Specifications:**
- `phase1-tasks/TASK-P1-04-PIPEWIRE-COMPLETE.md` (PipeWire - DONE)
- `phase1-tasks/TASK-P1-05-*` (Check for video pipeline specs)
- `reference/TESTING-SPECIFICATION.md` (Testing requirements)
- `reference/PERFORMANCE-REQUIREMENTS.md` (Performance targets)

**Implementation:**
- `src/portal/` - Example of complete, production-grade implementation
- `src/pipewire/` - Just completed, follow this quality standard
- `src/security/` - Example of proper error handling

**IronRDP Status:**
- `IRONRDP-RESOLUTION-FINAL.md` - Why IronRDP is blocked, what to monitor

---

## START HERE

1. Read this entire document
2. Review PipeWire implementation in `src/pipewire/` to understand quality expectations
3. Check for video pipeline specification in `phase1-tasks/`
4. Start with TASK 1 (Video Processing Pipeline)
5. Work through priorities systematically
6. Ask questions when needed
7. Implement EVERYTHING completely

**Remember:** This is a high-standards project. Quality over speed. Complete over quick.

---

**Good luck! Implement everything fully. No shortcuts. No simplifications. Production-grade only.**

