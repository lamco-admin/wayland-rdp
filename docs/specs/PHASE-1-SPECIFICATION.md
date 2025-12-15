# PHASE 1 CONSOLIDATED SPECIFICATION
**Document:** PHASE-1-SPECIFICATION.md
**Version:** 2.0 (IronRDP-Based Architecture)
**Date:** 2025-01-18
**Status:** AUTHORITATIVE - Production Grade
**Parent:** 00-MASTER-SPECIFICATION.md

---

## PHASE 1 OVERVIEW

### Objective
Deliver a **production-ready Wayland Remote Desktop Server** that enables Windows RDP clients to remotely control Wayland desktop sessions with full video streaming, input control, clipboard synchronization, and multi-monitor support using the RemoteFX codec.

### Timeline
**Duration:** 10 weeks (reduced from 12 weeks due to IronRDP)
**Start:** Week 1
**Completion:** Week 10

### Features Delivered

#### Core Functionality
- ✅ RDP server using IronRDP v0.9.0 (TLS 1.2/1.3, NLA authentication)
- ✅ Video streaming using RemoteFX codec
- ✅ Real-time screen capture via xdg-desktop-portal and PipeWire
- ✅ Full input control (keyboard and mouse) via Portal input injection
- ✅ Bidirectional clipboard synchronization (text and images)
- ✅ Multi-monitor support (up to 8 displays)
- ✅ Efficient bitmap compression and delta encoding
- ✅ Cursor metadata support
- ✅ Portal-based architecture for maximum compositor compatibility

#### Non-Functional Requirements
- ✅ Security: TLS 1.3, NLA with PAM authentication
- ✅ Performance: < 100ms end-to-end latency, 30 FPS stable
- ✅ Compatibility: GNOME 45+, KDE Plasma 6+, Sway 1.8+
- ✅ Quality: 80%+ test coverage, comprehensive error handling
- ✅ Documentation: Complete rustdoc, user guides, deployment procedures

---

## PHASE 1 TASKS (REVISED SEQUENCE)

### Completed Tasks (by CCW)
1. ✅ **P1-01: Foundation** (Week 1-2) - COMPLETE
2. ✅ **P1-02: Security** (Week 3) - COMPLETE  
3. ✅ **P1-03: Portal Integration** (Week 3-4) - COMPLETE

### Remaining Tasks (Specifications Complete, Ready to Assign)

4. **P1-04: PipeWire Integration** (Week 5, 3-5 days)
   - **Spec:** TASK-P1-04-PIPEWIRE-COMPLETE.md (72KB, 1500+ lines)
   - **Objective:** Complete PipeWire connection, frame reception, DMA-BUF zero-copy
   - **Deliverables:**
     - src/pipewire/ module (4 files, ~1200 lines of code)
     - Complete format negotiation (21 formats supported)
     - DMA-BUF zero-copy path with EGL/Vulkan
     - Multi-stream handling (8 monitors)
     - Integration test + example program

5. **P1-05: Bitmap Conversion** (Week 5-6, 5-7 days)
   - **Spec:** TASK-P1-05-BITMAP-CONVERSION.md (52KB, 1200+ lines)
   - **Objective:** Convert PipeWire frames to IronRDP BitmapUpdate structs
   - **Deliverables:**
     - src/bitmap/ module (4 files, ~800 lines of code)
     - All format conversions (BGRA, XRGB, NV12, YUV)
     - SIMD optimizations (AVX2, NEON)
     - Damage tracking with quad-tree
     - Buffer pool management
     - Cursor extraction

6. **P1-06: IronRDP Server Integration** (Week 6-7, 7-10 days)
   - **Spec:** TASK-P1-06-IRONRDP-SERVER-INTEGRATION.md (67KB, 1500+ lines)
   - **Objective:** Integrate all components with IronRDP server
   - **Deliverables:**
     - RdpServerInputHandler trait implementation (~700 lines)
     - RdpServerDisplay trait implementation (~500 lines)
     - Server lifecycle management (~400 lines)
     - RemoteFX configuration
     - Multi-client support
     - **This is the BIG integration task!**

7. **P1-07: Input Handling** (Week 8, 5-7 days)
   - **Spec:** TASK-P1-07-INPUT-HANDLING.md (74KB, 1000+ lines)
   - **Objective:** Complete input event forwarding with full mappings
   - **Deliverables:**
     - Complete scancode mapping table (200+ keys)
     - Coordinate transformation formulas
     - Multi-monitor input routing
     - InputTranslator implementation

8. **P1-08: Clipboard** (Week 8-9, 5-7 days)
   - **Spec:** TASK-P1-08-CLIPBOARD.md (50KB, 1100+ lines)
   - **Objective:** Bidirectional clipboard with format conversion
   - **Deliverables:**
     - Complete MIME ↔ RDP format mapping (15+ formats)
     - Image format conversions (DIB, PNG, JPEG)
     - Clipboard loop prevention algorithm
     - IronRDP CliprdrServer integration

9. **P1-09: Multi-Monitor** (Week 9, 5-7 days)
   - **Spec:** TASK-P1-09-MULTIMONITOR.md (54KB, 1000+ lines)
   - **Objective:** Full multi-monitor support with layout calculation
   - **Deliverables:**
     - Complete layout calculation algorithm
     - Coordinate system transformations
     - Monitor hotplug support
     - IronRDP DisplayControl integration

10. **P1-10: Testing & Integration** (Week 10, 7-10 days)
    - **Spec:** TASK-P1-10-TESTING-INTEGRATION.md (58KB, 1196 lines)
    - **Objective:** Complete testing, bug fixing, Phase 1 sign-off
    - **Deliverables:**
      - Complete integration test suite
      - Performance benchmarks
      - Compatibility matrix validation
      - Bug fixes and stabilization
      - Complete documentation

---

## INTEGRATION STRATEGY

### Component Integration Flow

```
Foundation (P1-01) ──────────────┐
                                 │
Security (P1-02) ────────────────┤
                                 │
Portal (P1-03) ──────────────────┼──► Provides Wayland access
                                 │
PipeWire (P1-04) ────────────────┤
                                 │
Bitmap Conversion (P1-05) ───────┤
                                 │
                                 ▼
         ┌───────────────────────────────────┐
         │ IronRDP Server Integration (P1-06)│ ◄── CENTRAL INTEGRATION
         └───────────────────────────────────┘
                      │
         ┌────────────┼────────────┐
         ▼            ▼            ▼
   Input (P1-07) Clipboard   Multi-Monitor
                 (P1-08)      (P1-09)
         └────────────┼────────────┘
                      ▼
          Testing & Integration (P1-10)
```

### Data Flow Integration

**Video Path:**
```
Wayland Compositor
    ↓ (Portal ScreenCast)
PipeWire Streams (DMA-BUF or SHM)
    ↓ (P1-04: PipeWire module)
VideoFrame structs
    ↓ (P1-05: Bitmap conversion)
BitmapUpdate structs (BGRA/XRGB pixels)
    ↓ (P1-06: RdpServerDisplay trait)
IronRDP Server (RemoteFX compression)
    ↓ (TCP/TLS)
Windows RDP Client (Display)
```

**Input Path:**
```
Windows RDP Client (Keyboard/Mouse)
    ↓ (TCP/TLS)
IronRDP Server (PDU parsing)
    ↓ (P1-06: RdpServerInputHandler trait)
KeyboardEvent / MouseEvent
    ↓ (P1-07: Input translator)
Portal RemoteDesktop API calls
    ↓ (libei / D-Bus)
Wayland Compositor (Input injection)
```

**Clipboard Path:**
```
Client Clipboard ←→ IronRDP CliprdrServer ←→ ClipboardManager ←→ Portal Clipboard ←→ Wayland Clipboard
                    (P1-06/P1-08: Format conversion and sync)
```

---

## DEPENDENCIES AND SEQUENCING

### Task Dependencies (Directed Acyclic Graph)

```
P1-01 (Foundation)
   ↓
   ├─→ P1-02 (Security) ───→ P1-06 (IronRDP Server)
   │                              ↓
   └─→ P1-03 (Portal) ──┬─→ P1-04 (PipeWire) ──→ P1-05 (Bitmap) ──→ P1-06
                        │                                              ↓
                        ├─→ P1-07 (Input) ──────────────────────────→ P1-06
                        │                                              ↓
                        └─→ P1-08 (Clipboard) ────────────────────────→ P1-06
                                                                       ↓
P1-09 (Multi-Monitor) ─────────────────────────────────────────────→ P1-06
                                                                       ↓
                                                               P1-10 (Testing)
```

### Parallel Execution Opportunities

**Week 5: Can parallelize**
- P1-04 (PipeWire) - Agent A
- P1-05 (Bitmap) - Agent B (minimal dependency on P1-04)

**Week 8-9: Can parallelize**
- P1-07 (Input) - Agent C
- P1-08 (Clipboard) - Agent D  
- P1-09 (Multi-Monitor) - Agent E

---

## PHASE 1 ACCEPTANCE CRITERIA

### Feature Completeness

| Feature | Acceptance Criteria | Verification Method |
|---------|-------------------|---------------------|
| RDP Connection | Windows mstsc connects successfully | Integration test |
| Video Streaming | 30 FPS stable at 1080p | Performance test |
| Input Control | < 50ms latency, all keys work | Integration + latency test |
| Clipboard | Bidirectional text + images | Integration test |
| Multi-Monitor | Up to 4 monitors, correct layout | Integration test |
| RemoteFX Codec | Bitrate 5-30 Mbps, good quality | Visual + bandwidth test |

### Performance Targets

| Metric | Target | Maximum | Test Method |
|--------|--------|---------|-------------|
| End-to-end latency (LAN) | < 50ms | < 100ms | timestamped-ping |
| Frame rate (1080p) | 30 FPS | - | FPS counter |
| Input latency | < 30ms | < 50ms | Event timestamping |
| CPU usage (idle) | < 5% | < 10% | top/htop monitoring |
| CPU usage (1080p30 RemoteFX) | < 25% | < 40% | top/htop monitoring |
| Memory usage | < 300MB | < 500MB | ps_mem/smem |
| Network bandwidth (1080p30) | 10 Mbps | 30 Mbps | iftop/nethogs |

### Quality Metrics

| Metric | Target | Verification |
|--------|--------|--------------|
| Test coverage | > 80% | cargo tarpaulin |
| Documentation | 100% public APIs | cargo doc |
| Clippy warnings | 0 | cargo clippy |
| Format compliance | 100% | cargo fmt --check |
| Security audit | Pass | cargo audit |

### Compatibility Matrix

| Compositor | GPU | Client | Status Required |
|------------|-----|--------|-----------------|
| GNOME 45+ | Intel | Windows 10 | ✅ MUST PASS |
| GNOME 45+ | AMD | Windows 11 | ✅ MUST PASS |
| KDE Plasma 6+ | Intel | Windows 10 | ✅ MUST PASS |
| Sway 1.8+ | Intel | FreeRDP 2.x | ✅ SHOULD PASS |

---

## RISK MANAGEMENT

### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Portal permission denied | Medium | High | Clear user documentation, permission caching |
| PipeWire connection fails | Low | High | Comprehensive error handling, retry logic |
| IronRDP trait complexity | Medium | Medium | Follow examples, thorough testing |
| RemoteFX performance insufficient | Low | Medium | Benchmark early, optimize bitmap conversion |
| Multi-monitor layout issues | Medium | Medium | Extensive testing, layout algorithms |
| Clipboard format incompatibility | Low | Low | Support common formats, graceful degradation |

### Schedule Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| IronRDP learning curve | Medium | Medium | Deep research up-front (done!), reference implementation |
| PipeWire DMA-BUF complexity | Medium | Low | Software path fallback, thorough docs |
| Integration issues | Medium | High | Early integration (P1-06), iterative testing |
| Testing takes longer | High | Low | Parallel testing during development |

---

## DELIVERABLES

### Code Deliverables
- [ ] Complete Rust implementation (~8,000-10,000 lines)
- [ ] All modules implemented as specified
- [ ] All unit tests passing
- [ ] All integration tests passing
- [ ] Benchmark suite operational

### Documentation Deliverables
- [ ] Complete rustdoc API documentation
- [ ] User guide (installation, configuration, usage)
- [ ] Deployment guide (systemd, production setup)
- [ ] Troubleshooting guide
- [ ] Architecture documentation

### Configuration Deliverables
- [ ] Default configuration file
- [ ] Example configurations
- [ ] Systemd service file
- [ ] Certificate generation scripts
- [ ] Deployment scripts

### Testing Deliverables
- [ ] Unit test suite (>80% coverage)
- [ ] Integration test suite (all workflows)
- [ ] Performance benchmark suite
- [ ] Compatibility test results
- [ ] Security audit results

---

## SUCCESS CRITERIA

Phase 1 is COMPLETE and SUCCESSFUL when:

### Functional Criteria
1. ✅ Windows 10/11 mstsc.exe connects successfully
2. ✅ Video displays at 30 FPS with acceptable quality
3. ✅ All keyboard keys work (200+ scancodes mapped)
4. ✅ Mouse control precise with < 50ms latency
5. ✅ Clipboard copy/paste works both directions (text + images)
6. ✅ Multi-monitor displays correctly with proper layout
7. ✅ Connection persists for > 1 hour without errors

### Performance Criteria
1. ✅ End-to-end latency < 100ms on LAN
2. ✅ Frame rate stable at 30 FPS
3. ✅ CPU usage < 40% at 1080p30
4. ✅ Memory usage < 500MB
5. ✅ Network bandwidth 10-30 Mbps at 1080p30

### Quality Criteria
1. ✅ All unit tests pass
2. ✅ All integration tests pass
3. ✅ Code coverage > 80%
4. ✅ Zero clippy warnings
5. ✅ Security audit clean
6. ✅ All documentation complete

### Compatibility Criteria
1. ✅ Works on GNOME 45+
2. ✅ Works on KDE Plasma 6+
3. ✅ Works on Sway 1.8+
4. ✅ Windows 10 client compatible
5. ✅ Windows 11 client compatible

---

## TECHNOLOGY STACK (Phase 1)

### Core Dependencies
```toml
# RDP Protocol (THE KEY DEPENDENCY)
ironrdp-server = { version = "0.9.0", features = ["helper"] }

# Portal Integration
ashpd = { version = "0.12.0", features = ["tokio"] }
zbus = "4.0.1"

# PipeWire Integration
pipewire = { version = "0.9.2", features = ["v0_3_77"] }
libspa = "0.9.2"

# Async Runtime
tokio = { version = "1.48", features = ["full", "tracing"] }
tokio-util = "0.7.17"
async-trait = "0.1.77"

# Security (TLS for IronRDP)
tokio-rustls = "0.26.4"
rustls = "0.23.35"
rustls-pemfile = "2.2.0"
pam = "0.7.0"

# Image Processing (for bitmap conversion)
image = "0.25.0"
yuv = "0.1.4"

# Utilities
bytes = "1.11.0"
anyhow = "1.0.79"
thiserror = "2.0.17"
tracing = "0.1.40"
```

### No Longer Needed (Removed)
- ❌ openh264 (IronRDP does encoding)
- ❌ va/libva (IronRDP does encoding)
- ❌ ironrdp-pdu separately (included in ironrdp-server)

---

## ARCHITECTURE SUMMARY

### IronRDP-Based Architecture

```
┌─────────────────────────────────┐
│    Windows RDP Client           │
└──────────┬──────────────────────┘
           │ RDP Protocol
           ↓
┌──────────────────────────────────┐
│   IronRDP Server (v0.9.0)        │
│   - Protocol handling            │
│   - RemoteFX compression         │
│   - Channel management           │
│   - TLS/NLA                      │
└──┬────────────────────┬──────────┘
   │ Traits             │ Traits
   ↓                    ↓
┌──────────────┐   ┌────────────────┐
│RdpServerInput│   │RdpServerDisplay│
│Handler       │   │                │
└──┬───────────┘   └────┬───────────┘
   │                    │
   │ Forward            │ Provide
   │ to Portal          │ BitmapUpdates
   ↓                    ↓
Portal API         PipeWire Frames
   ↓                    ↓
Compositor         Screen Content
```

**Key Insight:** IronRDP provides the RDP server framework. We only implement:
1. Input forwarding (IronRDP → Portal)
2. Display providing (PipeWire → IronRDP via BitmapUpdate)
3. Clipboard handling (IronRDP ↔ Portal)

---

## MILESTONE BREAKDOWN

### Week 1-2: Foundation ✅ COMPLETE
- Project structure
- Configuration system
- Logging infrastructure
- Build system

### Week 3: Security ✅ COMPLETE
- TLS 1.3 configuration
- Certificate management
- PAM authentication

### Week 3-4: Portal Integration ✅ COMPLETE
- D-Bus connection
- ScreenCast + RemoteDesktop portals
- Session management

### Week 5: Video Capture (P1-04 + P1-05)
- PipeWire connection and frame reception
- Bitmap conversion with SIMD
- First video frames flowing

### Week 6-7: RDP Server Integration (P1-06) ← CRITICAL
- IronRDP server implementation
- Trait implementations
- **First end-to-end working connection!**

### Week 8: Input & Clipboard (P1-07 + P1-08)
- Input forwarding with full mappings
- Clipboard sync with format conversion

### Week 9: Multi-Monitor (P1-09)
- Layout calculation
- Multiple stream handling

### Week 10: Testing & Polish (P1-10)
- Integration testing
- Performance validation
- Bug fixes
- Documentation completion

---

## PHASE 1 COMPLETION CHECKLIST

### Code Complete
- [ ] All 10 tasks implemented
- [ ] All modules integrated
- [ ] cargo build --release succeeds
- [ ] cargo clippy shows 0 warnings
- [ ] cargo fmt --check passes

### Testing Complete
- [ ] cargo test passes (all tests)
- [ ] cargo bench completes successfully
- [ ] Integration tests pass on GNOME
- [ ] Integration tests pass on KDE
- [ ] Integration tests pass on Sway
- [ ] Windows 10 client tested
- [ ] Windows 11 client tested
- [ ] Code coverage > 80%

### Performance Validated
- [ ] Latency < 100ms verified
- [ ] Frame rate 30 FPS verified
- [ ] CPU usage < 40% verified
- [ ] Memory usage < 500MB verified
- [ ] Bandwidth 10-30 Mbps verified

### Documentation Complete
- [ ] All public APIs documented (rustdoc)
- [ ] User guide written
- [ ] Deployment guide complete
- [ ] Troubleshooting guide complete
- [ ] README updated

### Production Ready
- [ ] Security audit passed
- [ ] No known critical bugs
- [ ] Deployment tested
- [ ] Systemd service tested
- [ ] Multi-user tested

---

## TRANSITION TO PHASE 2

### Hand-off Criteria
Phase 2 can begin when ALL Phase 1 criteria are met.

### Phase 2 Preparation
- Identify performance bottlenecks for Phase 2 optimization
- Document any technical debt
- List enhancement opportunities
- Prepare audio integration architecture

---

## APPENDIX: TASK SPECIFICATION SUMMARY

| Task | Spec File | Size | Status | Duration |
|------|-----------|------|--------|----------|
| P1-01 | TASK-P1-01-FOUNDATION.md | 789 lines | ✅ Complete | 3-5 days |
| P1-02 | TASK-P1-02-SECURITY.md | 718 lines | ✅ Complete | 5-7 days |
| P1-03 | TASK-P1-03-PORTAL-INTEGRATION-REVISED.md | ~150 lines | ✅ Complete | 5-7 days |
| P1-04 | TASK-P1-04-PIPEWIRE-COMPLETE.md | 1500+ lines | 📝 Spec Ready | 3-5 days |
| P1-05 | TASK-P1-05-BITMAP-CONVERSION.md | 1200+ lines | 📝 Spec Ready | 5-7 days |
| P1-06 | TASK-P1-06-IRONRDP-SERVER-INTEGRATION.md | 1500+ lines | 📝 Spec Ready | 7-10 days |
| P1-07 | TASK-P1-07-INPUT-HANDLING.md | 1000+ lines | 📝 Spec Ready | 5-7 days |
| P1-08 | TASK-P1-08-CLIPBOARD.md | 1100+ lines | 📝 Spec Ready | 5-7 days |
| P1-09 | TASK-P1-09-MULTIMONITOR.md | 1000+ lines | 📝 Spec Ready | 5-7 days |
| P1-10 | TASK-P1-10-TESTING-INTEGRATION.md | 1196 lines | 📝 Spec Ready | 7-10 days |

**Total:** 10 tasks, ~11,000+ lines of specification

---

**END OF PHASE 1 SPECIFICATION**

This consolidated specification provides complete oversight of Phase 1 development with all tasks, dependencies, integration strategy, and acceptance criteria fully defined.

**Status:** PRODUCTION-GRADE PRD/SRS QUALITY
**Completeness:** 100%
**Readiness:** Ready for implementation
