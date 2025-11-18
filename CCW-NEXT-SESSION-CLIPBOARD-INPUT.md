# CCW Development Session - Clipboard & Input Implementation

**Repository:** https://github.com/lamco-admin/wayland-rdp
**Branch:** main
**Current Status:** P1-01 through P1-04 complete, Video pipeline complete, IronRDP blocked

---

## CRITICAL: ZERO TOLERANCE POLICY

**This project has HIGHEST standards. Read carefully:**

❌ **ABSOLUTELY FORBIDDEN:**
- Shortcuts of ANY kind
- Simplifications or "simplified versions"
- Stub implementations with TODO comments
- "We'll implement this later" statements
- Skipping error handling
- Skipping tests
- Incomplete implementations
- Mock/fake implementations (except for explicitly blocked IronRDP)

✅ **MANDATORY FOR EVERY LINE OF CODE:**
- Follow specification EXACTLY as written
- Implement EVERY feature mentioned in spec
- Complete error handling with proper Result types
- Full rustdoc documentation on all public items
- Comprehensive unit tests (>80% coverage)
- Integration tests where applicable
- Production-grade quality only

**If specification is unclear:** ASK - never guess or simplify.
**If feature seems optional:** It's NOT - implement it fully.
**If unsure about approach:** Research and ask - never stub it.

---

## PROJECT STATUS

### Completed Modules ✅
- **P1-01:** Foundation & Configuration (complete)
- **P1-02:** Security (TLS, certs, auth - complete)
- **P1-03:** Portal Integration (5 files, 600 lines - complete)
- **P1-04:** PipeWire Integration (9 files, 3,392 lines - complete)
- **P1-05:** Video Processing Pipeline (3 files, 1,735 lines - complete)

### Blocked Module ⏳
- **P1-06:** IronRDP Server Integration
  - **DO NOT ATTEMPT** - upstream dependency broken
  - Tracking: https://github.com/Devolutions/IronRDP/pull/1028
  - Status: Still waiting on picky-rs 7.0.0 final release

### Your Tasks (Implement Fully)

---

## TASK 1: CLIPBOARD INTEGRATION (PRIORITY 1)

**Specification:** `phase1-tasks/TASK-P1-08-CLIPBOARD.md` (1,532 lines)
**Current State:** Stub only (`src/clipboard/mod.rs`)
**Portal Integration:** Already complete (`src/portal/clipboard.rs`)

### What to Implement

#### File Structure:
```
src/clipboard/
├── mod.rs         - Module coordinator, public API
├── manager.rs     - Clipboard manager, lifecycle
├── formats.rs     - Format negotiation and conversion
├── transfer.rs    - Data transfer handling
├── sync.rs        - Bidirectional synchronization
└── error.rs       - Error types specific to clipboard
```

#### Core Requirements:

**1. Clipboard Manager (manager.rs)**
- Initialize clipboard synchronization
- Connect to Portal clipboard interface (use `crate::portal::ClipboardManager`)
- Monitor clipboard changes in both directions
- Coordinate format negotiations
- Handle transfer lifecycle

**2. Format Handling (formats.rs)**
- Text formats: UTF-8, UTF-16, RTF, HTML
- Image formats: PNG, JPEG, BMP, TIFF, DIB
- File lists: File descriptors, URI lists
- Format conversion and negotiation
- MIME type mapping

**3. Transfer Engine (transfer.rs)**
- Chunked transfer for large data
- Progress tracking and cancellation
- Timeout handling
- Memory-efficient streaming
- Integrity verification

**4. Bidirectional Sync (sync.rs)**
- Wayland → RDP client sync
- RDP client → Wayland sync
- Conflict resolution
- Change detection
- Ownership tracking

**5. Integration:**
- Use existing `src/portal/clipboard.rs` for Wayland side
- Prepare RDP side (mock for now since IronRDP blocked)
- Complete event loop
- Full error recovery

### Deliverables:
- [ ] All 5 source files implemented completely
- [ ] Full test coverage in tests/integration/clipboard_test.rs
- [ ] Example program: examples/clipboard_sync.rs
- [ ] All formats supported (text, images, files)
- [ ] Comprehensive error handling
- [ ] Complete rustdoc documentation

---

## TASK 2: INPUT HANDLING (PRIORITY 1)

**Specification:** `phase1-tasks/TASK-P1-07-INPUT-HANDLING.md` (2,028 lines)
**Current State:** Stub only (`src/input/mod.rs`)
**Portal Integration:** Already complete (`src/portal/remote_desktop.rs`)

### What to Implement

#### File Structure:
```
src/input/
├── mod.rs           - Module coordinator
├── keyboard.rs      - Keyboard event handling & scancode mapping
├── mouse.rs         - Mouse event handling & coordinate transform
├── translator.rs    - RDP → Linux evdev translation
├── mapper.rs        - Complete scancode mapping tables (200+ keys)
├── coordinates.rs   - Multi-monitor coordinate transformation
└── error.rs         - Input-specific errors
```

#### Core Requirements:

**1. Keyboard Translation (keyboard.rs + mapper.rs)**
- RDP scancode → Linux evdev keycode mapping
- Complete mapping table for 200+ keys (per spec)
- All standard keys (letters, numbers, symbols)
- All special keys (F1-F24, media keys, etc.)
- Modifier key handling (Shift, Ctrl, Alt, Meta)
- Key press/release/repeat handling
- Layout-aware processing

**2. Mouse Translation (mouse.rs + coordinates.rs)**
- RDP coordinate → Wayland coordinate transformation
- Multi-monitor boundary handling
- Relative vs absolute movement
- Button mapping (left, right, middle, extended)
- Scroll wheel (vertical + horizontal)
- High-precision scrolling
- Coordinate clamping and validation

**3. Integration:**
- Receive events from RDP protocol (mock for now)
- Forward to Portal RemoteDesktop API (use existing `src/portal/remote_desktop.rs`)
- Event ordering preservation
- Sub-20ms latency path
- Input injection via portal's `notify_*` methods

**4. Multi-Monitor Support:**
- Coordinate space transformation
- Monitor boundary detection
- Proper routing to correct monitor
- Edge case handling (gaps, overlaps)

### Deliverables:
- [ ] Complete scancode mapping table (all 200+ keys)
- [ ] All input event types handled
- [ ] Multi-monitor coordinate math correct
- [ ] Full test coverage with mock events
- [ ] Integration test: tests/integration/input_test.rs
- [ ] Example: examples/input_injection.rs
- [ ] Performance: <20ms latency verified

---

## TASK 3: MULTI-MONITOR COORDINATION (PRIORITY 2)

**Specification:** `phase1-tasks/TASK-P1-09-MULTIMONITOR.md` (1,653 lines)
**Current State:** Stub (`src/multimon/mod.rs`)
**Note:** Some multi-monitor work already in PipeWire coordinator

### What to Implement

**Enhancement to existing work:**
- Review `src/pipewire/coordinator.rs` (already has multi-stream)
- Enhance with geometry management from spec
- Add monitor layout negotiation
- Coordinate space management
- Dynamic reconfiguration

**New implementation in src/multimon/:**
- Monitor configuration management
- Layout calculation and validation
- Coordinate mapping utilities
- RDP DisplayControl protocol structures (for when IronRDP ready)

### Deliverables:
- [ ] Monitor layout manager
- [ ] Coordinate transformation utilities
- [ ] Layout validation
- [ ] Tests for all coordinate edge cases
- [ ] Integration with PipeWire and input modules

---

## TASK 4: FIX TEST COMPILATION ISSUES

**Current Issue:** Video processor tests have Send bound error

### What to Fix:
1. Review `src/video/processor.rs` test at line ~171
2. Fix Future Send bound issue in tokio::spawn
3. Ensure all tests compile and run
4. Fix any remaining warnings

### Test Validation:
```bash
cargo test --lib          # Must pass
cargo test --all          # Must pass (except #[ignore])
cargo clippy -- -D warnings  # Must be clean
cargo fmt --check         # Must be formatted
```

---

## TASK 5: END-TO-END INTEGRATION TESTING

**File:** `tests/integration/e2e_test.rs` (create)

### What to Test:

**Full Pipeline (Without RDP):**
1. Portal session creation
2. PipeWire stream establishment
3. Frame capture
4. Video processing pipeline
5. Metrics collection
6. Resource cleanup

**Test Coverage:**
- [ ] Portal → PipeWire connection
- [ ] Frame flow through video pipeline
- [ ] Multi-monitor scenarios
- [ ] Format conversions
- [ ] Error recovery
- [ ] Resource leak detection
- [ ] Performance under load

**Requirements:**
- Mark as #[ignore] since requires Wayland
- Provide clear instructions for manual testing
- Test both success and failure paths
- Validate all performance metrics

---

## SPECIFICATIONS REFERENCE

**ALL specifications are in the MAIN branch:**

```
Repository: https://github.com/lamco-admin/wayland-rdp
Branch: main
```

**Task Specifications:**
- `phase1-tasks/TASK-P1-07-INPUT-HANDLING.md` (2,028 lines)
- `phase1-tasks/TASK-P1-08-CLIPBOARD.md` (1,532 lines)
- `phase1-tasks/TASK-P1-09-MULTIMONITOR.md` (1,653 lines)
- `phase1-tasks/TASK-P1-10-TESTING-INTEGRATION.md` (testing requirements)

**Reference Specs:**
- `reference/TESTING-SPECIFICATION.md` (testing standards)
- `reference/PERFORMANCE-REQUIREMENTS.md` (performance targets)

**Implemented Examples (Quality Reference):**
- `src/pipewire/` - 3,392 lines, 9 files, 54 tests (study this for quality expectations)
- `src/portal/` - 600 lines, 5 files (complete implementation)
- `src/video/` - 1,735 lines, 3 files (complete pipeline)

---

## IMPLEMENTATION ORDER

**Recommended sequence:**

1. **TASK 1: Clipboard** (3-4 days)
   - Most self-contained
   - Clear spec
   - Portal integration ready
   - Can be tested independently

2. **TASK 2: Input Handling** (4-5 days)
   - Large scancode mapping table
   - Complex coordinate math
   - Critical for functionality
   - Portal integration ready

3. **TASK 4: Fix Tests** (1 day)
   - Fix Send bound issue
   - Ensure all tests pass
   - Clean up warnings

4. **TASK 3: Multi-Monitor** (2-3 days)
   - Enhance existing coordinator
   - Add layout management
   - Coordinate utilities

5. **TASK 5: E2E Testing** (2-3 days)
   - Comprehensive integration tests
   - Performance validation
   - Load testing

---

## CODE QUALITY CHECKLIST

**Before committing ANY code:**

- [ ] Follows specification exactly (no deviations)
- [ ] All public items have rustdoc documentation
- [ ] All error paths return proper Result types
- [ ] No unwrap() or expect() in production code (only in tests)
- [ ] Unit tests for all functions
- [ ] Integration tests where applicable
- [ ] cargo build --release succeeds
- [ ] cargo test --lib passes
- [ ] cargo clippy -- -D warnings passes
- [ ] cargo fmt applied
- [ ] No TODO/FIXME in production code
- [ ] Performance requirements met and tested

---

## EXAMPLE: EXPECTED QUALITY LEVEL

**Look at `src/pipewire/format.rs`** as reference:
- Complete implementation of all format conversions
- Proper error handling throughout
- Comprehensive tests for each conversion
- Performance-optimized row-by-row processing
- Full documentation

**Your code must match this quality standard.**

---

## WHAT NOT TO DO

❌ Add ironrdp dependencies (they're broken upstream)
❌ Try to implement RDP protocol layer (blocked)
❌ Create temporary/mock RDP implementations (wait for real fix)
❌ Skip "advanced" features in specs (implement everything)
❌ Use unwrap/expect outside of tests
❌ Leave TODO comments
❌ Skip tests because "it's hard to test"
❌ Implement "basic version first" (implement complete version only)

---

## BUILD & TEST COMMANDS

```bash
# Development workflow
cargo build --lib                # Must succeed
cargo test --lib                 # Must pass
cargo clippy -- -D warnings      # Must be clean
cargo fmt                        # Apply before commit

# Integration testing (requires Wayland)
cargo test --test clipboard_test -- --ignored
cargo test --test input_test -- --ignored
cargo test --test e2e_test -- --ignored

# Examples
cargo run --example clipboard_sync
cargo run --example input_injection
```

---

## SESSION DELIVERABLES

**Expected by end of session:**

### Minimum (Must Complete):
1. ✅ Clipboard module fully implemented (all 5 files)
2. ✅ Input module fully implemented (all 6 files)
3. ✅ All tests passing
4. ✅ Test compilation fixed
5. ✅ Complete documentation

### Stretch Goals:
1. Multi-monitor enhancements
2. E2E integration tests
3. Additional examples
4. Performance optimizations

---

## QUALITY STANDARDS REMINDER

**Every single line of code must be:**
- Specification-compliant (exact adherence)
- Production-ready (no prototypes)
- Fully tested (unit + integration)
- Completely documented (rustdoc)
- Error-handled (no panics)
- Performance-optimized (meet spec requirements)

**Reference the PipeWire implementation (`src/pipewire/`) for quality expectations.**

---

## START CHECKLIST

Before writing code:
- [ ] Read TASK-P1-08-CLIPBOARD.md completely
- [ ] Read TASK-P1-07-INPUT-HANDLING.md completely
- [ ] Study `src/pipewire/` for quality reference
- [ ] Study `src/portal/` for integration patterns
- [ ] Understand existing clipboard portal integration
- [ ] Understand existing remote desktop portal integration
- [ ] Plan file structure
- [ ] Identify all integration points

Then:
- [ ] Implement clipboard completely
- [ ] Implement input handling completely
- [ ] Fix test issues
- [ ] Write comprehensive tests
- [ ] Document everything
- [ ] Verify build and tests
- [ ] Format and lint

---

**Remember: Production-grade only. Zero shortcuts. Follow specs exactly.**

**START WITH:** Reading both complete specifications, then implement clipboard first.

