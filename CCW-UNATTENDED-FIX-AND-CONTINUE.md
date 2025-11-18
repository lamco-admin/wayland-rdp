# CCW UNATTENDED SESSION - CODE FIXES + CONTINUE
**Repository:** https://github.com/lamco-admin/wayland-rdp
**Mode:** UNATTENDED OVERNIGHT
**Status:** Code review complete - CRITICAL fixes needed
**Read First:** CODE-QUALITY-REVIEW.md (396 lines, complete review)

---

## 🔴 CRITICAL SITUATION

**Code Quality Review Completed:** 8 CRITICAL issues found, 12 HIGH priority issues

**Bottom Line:** The existing code is 30% complete but MISSING the IronRDP dependency entirely. It cannot function as an RDP server without fixing Cargo.toml FIRST.

---

## 📋 YOUR MISSION (UNATTENDED - WORK OVERNIGHT)

### PHASE 1: FIX CRITICAL ISSUES (2-3 hours) - MUST DO

#### Task 1.1: Fix Cargo.toml Dependencies (CRITICAL - 1 hour)

**Read:** `02-TECHNOLOGY-STACK.md` lines 30-230 for CORRECT Cargo.toml

**Fix these CRITICAL missing dependencies:**

```toml
# ADD - RDP Protocol (CURRENTLY COMPLETELY MISSING!)
ironrdp-server = { version = "0.9.0", features = ["helper"] }
ironrdp-pdu = "0.6.0"
ironrdp-graphics = "0.6.0"

# ADD - PipeWire (CURRENTLY MISSING!)
pipewire = { version = "0.9.2", features = ["v0_3_77"] }
libspa = "0.9.2"
libspa-sys = "0.9.2"

# ADD - Image Processing (CURRENTLY MISSING!)
image = "0.25.0"
yuv = "0.1.4"

# ADD - Missing utilities
async-trait = "0.1.85"

# UPDATE - Wrong versions
rustls = "0.23.35"  # Currently 0.21
rustls-pemfile = "2.2.0"  # Currently 1.0
tokio-rustls = "0.26.4"  # Currently 0.24
tokio = "1.48"  # Currently 1.35
thiserror = "2.0.17"  # Currently 1.0

# ADD - Video encoding (choose ONE for now)
openh264 = { version = "0.6.0", features = ["encoder"], optional = true }
# OR later: va = "0.7.0" for hardware encoding
```

**Verify after fix:**
```bash
cargo check
# Should succeed now!
```

**Commit:**
```
git commit -m "CRITICAL FIX: Add IronRDP and all missing dependencies to Cargo.toml

Fixes:
- Add ironrdp-server 0.9.0 (THE core dependency, was missing!)
- Add pipewire 0.9.2 for screen capture
- Add image processing crates
- Update rustls to 0.23.35
- Update tokio-rustls to 0.26.4
- Add all missing dependencies from 02-TECHNOLOGY-STACK.md

Without these dependencies the project cannot function.
Code review: CODE-QUALITY-REVIEW.md"
```

#### Task 1.2: Fix Portal Implementation Issues (1 hour)

**Issues from CODE-QUALITY-REVIEW.md:**

**Fix src/portal/screencast.rs** (currently only 4 lines - CRITICAL)
- It's just a placeholder stub!
- Must implement complete ScreenCastManager
- **Copy implementation from:** `TASK-P1-03-PORTAL-INTEGRATION-REVISED.md` lines 200-270
- ~130 lines of code needed

**Fix src/portal/clipboard.rs** (currently 48 lines - incomplete)
- Has placeholder comments (lines 25, 40)
- Must complete Portal clipboard integration
- **Reference:** `05-PROTOCOL-SPECIFICATIONS.md` lines 180-240

**Commit:**
```
git commit -m "Complete portal implementation: screencast and clipboard

- Implement full ScreenCastManager from spec
- Complete clipboard portal integration  
- Remove placeholder comments
- Add proper error handling"
```

#### Task 1.3: Fix Security Module Issues (30 min)

**Fix src/security/tls.rs line 132:**
- Remove `expect()` in accept() method
- Return Result instead
- **Reference:** `TASK-P1-02-SECURITY.md` lines 150-180

**Fix src/security/certificates.rs:**
- Currently empty stub (4 lines)
- Must implement complete CertificateGenerator
- **Copy from:** `TASK-P1-02-SECURITY.md` lines 250-370

**Commit:**
```
git commit -m "Fix security module: complete certificate generation and fix TLS error handling"
```

---

### PHASE 2: IMPLEMENT PIPEWIRE (4-6 hours) - PRIMARY GOAL

**After PHASE 1 fixes are committed**, implement PipeWire module.

**Specification:** `phase1-tasks/TASK-P1-04-PIPEWIRE-COMPLETE.md`

**This is 73KB, 1500 lines, 100% COMPLETE specification with 800+ lines of production code.**

**Implement:**

1. **src/pipewire/mod.rs** (~250 lines)
   - Module coordinator
   - PipeWire connection manager
   - Integration with Portal FD

2. **src/pipewire/stream.rs** (~350 lines)
   - PipeWire context creation using FD
   - Stream setup and configuration
   - Event listener registration

3. **src/pipewire/format.rs** (~200 lines)
   - SPA Pod construction
   - Format negotiation (21 formats)
   - DMA-BUF modifier handling

4. **src/pipewire/receiver.rs** (~400 lines)
   - Frame processing callback
   - Buffer dequeue/queue
   - VideoFrame extraction
   - Multi-stream coordination

5. **examples/pipewire_frames.rs** (~100 lines)
   - Demo program showing frame reception

6. **tests/integration/pipewire_test.rs** (~150 lines)
   - Integration test (#[ignore])

**Commit:**
```
git commit -m "Implement P1-04: Complete PipeWire integration with DMA-BUF support

Complete implementation per TASK-P1-04-PIPEWIRE-COMPLETE.md:
- PipeWire connection using portal FD
- Format negotiation with 21 formats
- DMA-BUF zero-copy path
- Multi-stream handling (up to 8 monitors)
- Complete error handling and recovery
- Integration tests and examples

Tested: cargo build --lib succeeds, tests pass"
```

---

### PHASE 3: CREATE PROGRESS REPORT (15 min)

**Create file:** `OVERNIGHT-PROGRESS-REPORT.md`

```markdown
# Overnight Unattended Session - Progress Report

## Session Duration: [START] to [END]

## PHASE 1: Critical Fixes ✅
- [x] Updated Cargo.toml with IronRDP and all dependencies
- [x] Fixed portal screencast implementation
- [x] Fixed portal clipboard implementation
- [x] Fixed security certificate generation
- [x] Fixed TLS error handling
- [x] cargo check now succeeds

## PHASE 2: PipeWire Implementation
- [x/partial/?] src/pipewire/ module (check what was completed)
- [x/partial/?] examples/pipewire_frames.rs
- [x/partial/?] tests/integration/pipewire_test.rs
- [?] cargo build --lib status
- [?] cargo test status

## Issues Encountered:
[List any blockers or problems]

## What's Ready:
- Cargo.toml: Fixed with all correct dependencies ✅
- Portal: Complete implementation ✅
- Security: Fixed ✅
- PipeWire: [Status - check what's done]

## Next Steps:
[If not complete: what remains]
[If complete: P1-05 Bitmap Conversion is next]

## Time Spent:
Phase 1 fixes: ~X hours
PipeWire implementation: ~Y hours
Total: ~Z hours
```

**Push everything:**
```bash
git push -u origin <your-branch-name>
```

---

## 📚 CRITICAL REFERENCE DOCUMENTS

**Must Read (In Order):**

1. **CODE-QUALITY-REVIEW.md** - What's wrong with existing code
2. **02-TECHNOLOGY-STACK.md** - Correct Cargo.toml (lines 30-230)
3. **TASK-P1-04-PIPEWIRE-COMPLETE.md** - Complete PipeWire implementation
4. **04-DATA-STRUCTURES.md** - VideoFrame and all structures  
5. **05-PROTOCOL-SPECIFICATIONS.md** - PipeWire and Portal protocols

---

## ⚠️ CRITICAL PRIORITIES

### MUST FIX (Before anything else works):
1. ✅ Cargo.toml - Add ironrdp-server (THE core dependency!)
2. ✅ Cargo.toml - Add pipewire crates
3. ✅ Cargo.toml - Update rustls versions
4. ✅ Portal screencast - Complete implementation (not stub)
5. ✅ Security certificates - Complete implementation (not stub)

### THEN IMPLEMENT:
6. ✅ PipeWire module - Full implementation from spec

### IF TIME PERMITS:
7. Start P1-05: Bitmap Conversion
8. Run examples and verify

---

## 🎯 MINIMUM SUCCESS CRITERIA

Session is successful if:
- [ ] Cargo.toml has ironrdp-server = "0.9.0" ✅
- [ ] Cargo.toml has all correct dependencies ✅
- [ ] Portal module complete (no stubs) ✅
- [ ] Security module complete (certificate generation) ✅
- [ ] cargo check succeeds ✅
- [ ] PipeWire module started (at minimum)

**Target:** PipeWire module completely implemented

**Stretch:** Start bitmap conversion

---

## 🔧 TROUBLESHOOTING

### If cargo check fails after dependency updates:
- Check version compatibility
- Consult 02-TECHNOLOGY-STACK.md for exact versions
- May need to run: cargo update

### If PipeWire API doesn't match spec:
- Spec shows C API as reference
- Use pipewire-rs Rust bindings
- Core concepts same: connect FD, negotiate format, receive buffers

### If time runs out:
- Prioritize Cargo.toml fixes
- Commit what you have
- Document what remains in progress report

---

## 📊 EXPECTED OUTCOME WHEN USER WAKES UP

### Minimum (If Time Constrained):
- Cargo.toml fixed with all dependencies ✅
- Portal stubs completed ✅
- Security certificate generation implemented ✅
- Code compiles with cargo check ✅
- Progress report documents status ✅

### Target (Normal Progress):
- All of minimum ✅
- PipeWire module 100% implemented ✅
- Examples and tests created ✅
- cargo build --lib succeeds ✅
- Ready for P1-05 (Bitmap Conversion) ✅

### Stretch (Fast Progress):
- All of target ✅
- Bitmap conversion module started ✅
- Integration test run successfully ✅

---

## 🌙 FINAL NOTES

The code review found the implementation is **structurally sound** but:
- **Missing critical dependencies** (can't work without IronRDP!)
- **Some modules are stubs** (screencast, clipboard, certificates)
- **Needs completion** per v2.0 specifications

**Good news:** The specifications are NOW 100% complete with full code examples. You can copy implementations directly from the specs.

**Work overnight. Fix the critical issues. Implement PipeWire. Document progress.**

**When user wakes up:** Code should be building, portal complete, PipeWire likely implemented, ready to continue!

🚀 **Good luck!**
