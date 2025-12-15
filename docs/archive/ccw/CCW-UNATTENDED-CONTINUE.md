# CCW UNATTENDED SESSION - RESUME AND CONTINUE
**Repository:** https://github.com/lamco-admin/wayland-rdp
**Mode:** UNATTENDED (work while user sleeps)
**Objective:** Get back on track with v2.0 specifications and continue PipeWire implementation

---

## 🎯 SITUATION ASSESSMENT

### What's Complete
You (CCW) have already implemented:
- ✅ P1-01: Foundation (config, main, logging)
- ✅ P1-02: Security (TLS, certificates, PAM auth)
- ✅ P1-03: Portal Integration (ashpd, ScreenCast, RemoteDesktop)

**Code exists in:** src/config/, src/security/, src/portal/

### Critical Issue Identified
The **Cargo.toml is MISSING the IronRDP dependency** which is critical for the entire project.

### What Changed
The specifications were completely revised to v2.0 (production-grade). The architecture is now based on IronRDP v0.9.0 which provides the RDP server framework.

---

## 🔧 IMMEDIATE ACTIONS REQUIRED

### STEP 1: Update Cargo.toml (CRITICAL)

The current Cargo.toml is missing IronRDP. Update it to match the CORRECT specification from:
`02-TECHNOLOGY-STACK.md` (lines 30-200)

**Add these critical dependencies:**
```toml
# RDP Protocol (CRITICAL - Currently MISSING)
ironrdp-server = { version = "0.9.0", features = ["helper"] }

# PipeWire (for next task)
pipewire = { version = "0.9.2", features = ["v0_3_77"] }
libspa = "0.9.2"
libspa-sys = "0.9.2"

# Image processing (for bitmap conversion)
image = "0.25.0"

# Update these to correct versions:
rustls = "0.23.35"  # Currently 0.21
rustls-pemfile = "2.2.0"  # Currently 1.0  
tokio-rustls = "0.26.4"  # Currently 0.24
```

**Full correct Cargo.toml is in:** `02-TECHNOLOGY-STACK.md`

### STEP 2: Verify Existing Code Quality

Check if the existing portal/security/config code matches v2.0 specifications:

**Compare against:**
- `04-DATA-STRUCTURES.md` - Are data structures correct?
- `05-PROTOCOL-SPECIFICATIONS.md` - Are Portal APIs used correctly?
- `TASK-P1-03-PORTAL-INTEGRATION-REVISED.md` - Does implementation match?

**If mismatches found:** Document them but DON'T break working code.

### STEP 3: Begin P1-04 - PipeWire Integration

**Specification:** `phase1-tasks/TASK-P1-04-PIPEWIRE-COMPLETE.md`

This is a **73KB, 1500-line PRODUCTION-GRADE specification** with:
- Complete PipeWire C API implementation
- DMA-BUF zero-copy path
- All 21 format conversions
- Multi-stream handling
- 800+ lines of complete production code
- NO TODOs or placeholders

**Implement:**
```
src/pipewire/
├── mod.rs          # Main coordinator
├── stream.rs       # PipeWire stream connection
├── receiver.rs     # Frame reception
└── format.rs       # Format negotiation

examples/pipewire_frames.rs
tests/integration/pipewire_test.rs
```

---

## 📋 DETAILED TASKS FOR UNATTENDED SESSION

### Task 1: Fix Cargo.toml (30 minutes)
1. Read `02-TECHNOLOGY-STACK.md` lines 30-200
2. Update Cargo.toml with ALL correct dependencies
3. Remove obsolete dependencies
4. Add ironrdp-server = "0.9.0" (CRITICAL)
5. Add pipewire dependencies
6. Update rustls versions
7. Run `cargo check` to verify
8. Commit: "Update Cargo.toml to v2.0 specifications with IronRDP"

### Task 2: Code Quality Review (30 minutes)
1. Review src/portal/ against `05-PROTOCOL-SPECIFICATIONS.md`
2. Review src/security/ against `TASK-P1-02-SECURITY.md`
3. Review src/config/ against `04-DATA-STRUCTURES.md`
4. Document any discrepancies in CODE-REVIEW.md
5. If minor issues found, fix them
6. If major issues found, document for later
7. Commit: "Code quality review against v2.0 specifications"

### Task 3: Implement PipeWire Module (4-6 hours)
Follow `TASK-P1-04-PIPEWIRE-COMPLETE.md` EXACTLY:

1. **Create src/pipewire/mod.rs** (lines 100-300 of spec)
   - Module coordinator
   - Public API
   - Integration with Portal

2. **Create src/pipewire/stream.rs** (lines 302-600 of spec)
   - PipeWire context creation using portal FD
   - pw_context_connect_fd implementation
   - Stream creation and configuration
   - Event listener setup

3. **Create src/pipewire/format.rs** (lines 602-850 of spec)
   - SPA Pod construction for format negotiation
   - All 21 format specifications
   - DMA-BUF modifier handling
   - Format preference order

4. **Create src/pipewire/receiver.rs** (lines 852-1200 of spec)
   - Frame processing callback
   - Buffer dequeue/queue
   - VideoFrame struct population
   - Multi-stream coordination

5. **Create examples/pipewire_frames.rs** (lines 1300-1400 of spec)
   - Demonstrates PipeWire connection
   - Shows frame reception
   - Prints frame metadata

6. **Create tests/integration/pipewire_test.rs** (lines 1402-1500 of spec)
   - Tests PipeWire connection
   - Tests frame reception
   - Mark as #[ignore] for CI

7. **Test and verify:**
   - cargo build succeeds
   - cargo test passes
   - Example runs in Wayland session

8. Commit: "Implement P1-04: Complete PipeWire integration with DMA-BUF support"

---

## 🎯 SUCCESS CRITERIA FOR THIS SESSION

### Must Complete:
- [ ] Cargo.toml updated with IronRDP and all correct dependencies
- [ ] Code quality review completed
- [ ] src/pipewire/ module implemented (all 4 files)
- [ ] Example program created
- [ ] Integration test created
- [ ] cargo build --lib succeeds
- [ ] All code committed to branch

### Stretch Goals (if time permits):
- [ ] Start P1-05: Bitmap Conversion module
- [ ] Update README with current progress
- [ ] Run example program and verify frames received

---

## 📚 REFERENCE DOCUMENTS (In Order of Importance)

1. **SPECIFICATIONS-V2-COMPLETE.md** - Overview of v2.0 changes
2. **02-TECHNOLOGY-STACK.md** - Correct dependencies (READ FIRST for Cargo.toml)
3. **04-DATA-STRUCTURES.md** - All data structure definitions
4. **05-PROTOCOL-SPECIFICATIONS.md** - All protocol details
5. **TASK-P1-04-PIPEWIRE-COMPLETE.md** - COMPLETE implementation spec
6. **IRONRDP-INTEGRATION-GUIDE.md** - Why we use IronRDP

---

## ⚠️ CRITICAL NOTES

### About Cargo.toml
The existing Cargo.toml is based on v1.0 specs (wrong). It's MISSING:
- ironrdp-server (THE most important dependency!)
- pipewire crates
- Correct rustls versions

**MUST update Cargo.toml first** before implementing PipeWire.

### About Existing Code
The portal/security/config code was implemented against v1.0 specs but should mostly still be correct. The v2.0 changes are primarily about:
- Adding IronRDP (not removing portal/security)
- Changing video architecture (doesn't affect portal/security)
- Adding missing details (doesn't break existing code)

**Don't break what works** - just enhance and add PipeWire.

### About PipeWire Implementation
The spec includes COMPLETE code. You can:
- Copy the implementation from the spec
- Adapt for Rust idioms
- Use the spec's exact approach
- Follow the spec's structure

The spec has 800+ lines of production code ready to use.

---

## 🚀 WORKFLOW FOR UNATTENDED SESSION

### Phase 1: Setup (30 min)
```bash
# Clone repo if needed
git clone https://github.com/lamco-admin/wayland-rdp.git
cd wayland-rdp

# Create new branch
git checkout -b fix/cargo-and-pipewire

# Review current state
cargo check
```

### Phase 2: Fix Cargo.toml (30 min)
```bash
# Update Cargo.toml with correct dependencies
# Reference: 02-TECHNOLOGY-STACK.md

# Verify
cargo check

# Commit
git add Cargo.toml
git commit -m "Update Cargo.toml to v2.0 specs with IronRDP"
```

### Phase 3: Implement PipeWire (4-6 hours)
```bash
# Create module structure
mkdir -p src/pipewire

# Implement each file following spec
# src/pipewire/mod.rs
# src/pipewire/stream.rs
# src/pipewire/format.rs
# src/pipewire/receiver.rs

# Create example
# examples/pipewire_frames.rs

# Create test
# tests/integration/pipewire_test.rs

# Verify
cargo build --lib
cargo test

# Commit
git add src/pipewire/ examples/ tests/
git commit -m "Implement P1-04: Complete PipeWire integration"
```

### Phase 4: Document and Push (15 min)
```bash
# Create progress report
cat > PROGRESS-REPORT.md << EOF
# Progress Report - Unattended Session

## Completed:
- [x] Updated Cargo.toml to v2.0 specifications
- [x] Added ironrdp-server = "0.9.0"
- [x] Implemented src/pipewire/ module
- [x] Created example program
- [x] Created integration test
- [x] All builds passing

## Next:
- P1-05: Bitmap Conversion
EOF

# Push everything
git push -u origin fix/cargo-and-pipewire
```

---

## 💡 IF YOU ENCOUNTER ISSUES

### Issue: pipewire crate API different from spec
**Solution:** The spec shows C API. Use pipewire-rs Rust bindings. Core concepts are the same:
- Connect using FD
- Negotiate format
- Receive buffers
- Extract frame data

### Issue: DMA-BUF too complex
**Solution:** Implement memory buffer path first, DMA-BUF is optimization. Spec shows both paths.

### Issue: Format negotiation unclear
**Solution:** Spec lines 602-850 have complete format negotiation. Follow exactly.

### Issue: Build errors with dependencies
**Solution:** Check 02-TECHNOLOGY-STACK.md for correct versions. Run scripts/verify-dependencies.sh.

### Issue: Portal FD not working
**Solution:** Verify portal module returns valid FD. Check src/portal/session.rs implementation.

---

## 📊 EXPECTED OUTPUT AFTER SESSION

### Code Added
- Updated Cargo.toml (with IronRDP)
- src/pipewire/ (4 files, ~1000 lines)
- examples/pipewire_frames.rs (~100 lines)
- tests/integration/pipewire_test.rs (~150 lines)
- Total: ~1250 new lines

### Commits Made
- "Update Cargo.toml to v2.0 specifications with IronRDP"
- "Implement P1-04: Complete PipeWire integration"
- "Add PipeWire example and integration test"

### Branch Status
- Branch: fix/cargo-and-pipewire
- Pushed to: origin
- Ready for: Review and merge

---

## ✅ SESSION COMPLETION CRITERIA

Session is successful when:
1. ✅ Cargo.toml has ironrdp-server = "0.9.0"
2. ✅ Cargo.toml has correct pipewire dependencies
3. ✅ src/pipewire/ module implemented
4. ✅ cargo build --lib succeeds
5. ✅ All code committed and pushed
6. ✅ Progress report created

**Minimum acceptable:** Cargo.toml fixed + PipeWire module started
**Target:** PipeWire module complete and tested
**Stretch:** Start bitmap conversion module

---

## 🌙 GOOD NIGHT MESSAGE FOR USER

When you wake up, you should have:
- ✅ Cargo.toml corrected with IronRDP
- ✅ PipeWire module implemented (likely complete)
- ✅ Example program to test PipeWire
- ✅ All code in new branch ready for review
- ✅ Progress report showing what was done

**Next step:** Review the code, test the example, merge branch, continue with P1-05 (Bitmap Conversion).

---

**Start this session in CCW now. It will work while you sleep.** 🌙
