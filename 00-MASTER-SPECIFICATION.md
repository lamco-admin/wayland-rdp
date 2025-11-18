# WAYLAND REMOTE DESKTOP SERVER - MASTER TECHNICAL SPECIFICATION
**Project Codename:** wrd-server
**Version:** 1.0
**Date:** 2025-01-18
**Status:** AUTHORITATIVE SPECIFICATION - DO NOT DEVIATE

---

## DOCUMENT PURPOSE

This is the **AUTHORITATIVE** master specification for the Wayland Remote Desktop Server project. All implementation work MUST conform to this specification and its associated documents. Any deviations require explicit approval and documentation updates.

## DOCUMENT STRUCTURE

This specification is organized into multiple documents:

### Core Specifications
1. **00-MASTER-SPECIFICATION.md** (this document) - Overall project overview
2. **01-ARCHITECTURE.md** - System architecture and design
3. **02-TECHNOLOGY-STACK.md** - Complete technology stack with exact versions
4. **03-PROJECT-STRUCTURE.md** - Directory layout and module organization
5. **04-DATA-STRUCTURES.md** - All data structures and type definitions
6. **05-PROTOCOL-SPECIFICATIONS.md** - RDP, Wayland, Portal protocol details

### Task Documents (Phase 1)
- **phase1-tasks/TASK-P1-01-FOUNDATION.md**
- **phase1-tasks/TASK-P1-02-SECURITY.md**
- **phase1-tasks/TASK-P1-03-PORTAL-INTEGRATION.md**
- **phase1-tasks/TASK-P1-04-PIPEWIRE.md**
- **phase1-tasks/TASK-P1-05-BITMAP-CONVERSION.md**
- **phase1-tasks/TASK-P1-06-IRONRDP-SERVER-INTEGRATION.md**
- **phase1-tasks/TASK-P1-07-INPUT-HANDLING.md**
- **phase1-tasks/TASK-P1-08-CLIPBOARD.md**
- **phase1-tasks/TASK-P1-09-MULTIMONITOR.md**
- **phase1-tasks/TASK-P1-10-TESTING-INTEGRATION.md**

### Reference Documents
- **reference/TESTING-SPECIFICATION.md**
- **reference/DEPLOYMENT-GUIDE.md**
- **reference/API-REFERENCE.md**
- **reference/PERFORMANCE-REQUIREMENTS.md**
- **reference/SECURITY-REQUIREMENTS.md**

---

## PROJECT OVERVIEW

### Mission Statement
Build a production-ready Wayland remote desktop server in Rust that enables Windows RDP clients to remotely control Wayland desktop sessions with high performance, low latency, and full feature parity with native desktop usage.

### Target Platform
- **Server:** Linux with Wayland compositor (GNOME 45+, KDE Plasma 6+, Sway 1.8+)
- **Client:** Windows 10/11 RDP client (mstsc.exe), FreeRDP 2.x+
- **Protocol:** RDP 10.x with RemoteFX codec support

### Key Features

#### Phase 1 (Core - 10 weeks)
- ✅ RDP server with TLS 1.3 encryption
- ✅ Network Level Authentication (NLA) with PAM
- ✅ Video streaming using RemoteFX codec
- ✅ Efficient bitmap compression and delta encoding
- ✅ Full input control (keyboard + mouse)
- ✅ Bidirectional clipboard sharing
- ✅ Multi-monitor support (up to 4 monitors)
- ✅ Damage tracking for efficiency
- ✅ Cursor metadata for reduced latency
- ✅ Portal-based architecture for maximum compositor compatibility

#### Phase 2 (Audio + Advanced - 4 weeks)
- ✅ Bidirectional audio streaming
- ✅ Opus audio codec
- ✅ Audio/video synchronization
- ✅ Volume control
- ✅ Performance optimizations
- ✅ Enhanced monitoring and metrics

---

## TECHNICAL APPROACH

### Architecture Philosophy
1. **Portal-First:** Use xdg-desktop-portal APIs for maximum compositor compatibility
2. **Rust-Native:** Pure Rust implementation using IronRDP library
3. **Efficient Processing:** RemoteFX codec with bitmap compression
4. **Built-in Encoding:** IronRDP handles all codec operations internally
5. **Standards-Compliant:** Follow RDP 10.x, Wayland, and Portal specifications exactly

### Technology Choices

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Language | Rust 1.75+ | Memory safety, performance, excellent async support |
| RDP Server | ironrdp-server 0.9.0 | Complete RDP server implementation in pure Rust |
| Video Codec | RemoteFX | Built into IronRDP, efficient bitmap compression |
| Portal Access | ashpd | Mature Rust bindings for xdg-desktop-portal |
| Media Transport | PipeWire | Modern Linux multimedia framework |
| Image Processing | image-rs | Pure Rust image manipulation library |
| Async Runtime | Tokio | Industry standard, excellent ecosystem |
| TLS | rustls (via IronRDP) | Pure Rust, modern TLS 1.3 implementation |
| Authentication | PAM | Standard Linux authentication |

### Critical Dependencies
- **Wayland Compositor:** GNOME 45+, KDE Plasma 6+, or Sway 1.8+
- **PipeWire:** 0.3.77+ (for screencasting)
- **xdg-desktop-portal:** 1.18+ (with compositor-specific backend)
- **System:** Linux kernel 6.0+, systemd

---

## PROJECT PHASES

### Phase 1: Core Functionality (Weeks 1-10)

**Milestones:**
1. **Foundation** (Week 1): Project setup, configuration, logging
2. **Security** (Week 2): TLS, certificates, NLA authentication via IronRDP
3. **Portal Integration** (Week 3): ScreenCast and RemoteDesktop portals
4. **PipeWire** (Week 4): Video stream capture and format conversion
5. **Bitmap Conversion** (Week 5): RGB to bitmap format for RemoteFX
6. **IronRDP Server** (Week 6): Complete server integration with RemoteFX
7. **Input** (Week 7): Keyboard and mouse control via IronRDP
8. **Clipboard** (Week 8): Bidirectional clipboard sync
9. **Multi-Monitor** (Week 9): Multiple display support
10. **Testing & Stabilization** (Week 10): Integration testing, bug fixes

**Deliverable:** Fully functional remote desktop server with video, input, clipboard, and multi-monitor support

### Phase 2: Audio & Polish (Weeks 11-14)

**Milestones:**
1. **Audio Capture** (Week 11): PipeWire audio, Opus encoding
2. **Audio Channels** (Week 12): RDP audio output/input via IronRDP, A/V sync
3. **Optimization** (Week 13): Performance tuning, profiling
4. **Documentation & Features** (Week 14): Complete documentation, metrics, monitoring

**Deliverable:** Production-ready v1.0 release with full audio support

---

## PERFORMANCE TARGETS

### Latency
- **Input latency:** < 30ms (target), < 50ms (maximum)
- **Encoding latency:** < 16ms @ 30 FPS (target), < 33ms (maximum)
- **End-to-end (LAN):** < 50ms (target), < 100ms (maximum)
- **End-to-end (WAN):** < 150ms (target), < 300ms (maximum)

### Throughput
- **1080p @ 30 FPS:** 5-8 Mbps (RemoteFX with compression)
- **1080p @ 60 FPS:** 10-15 Mbps (RemoteFX)
- **4K @ 30 FPS:** 20-30 Mbps (RemoteFX)

### Resource Usage
- **CPU (idle):** < 2%
- **CPU (1080p30, RemoteFX):** < 25%
- **CPU (4K30, RemoteFX):** < 40%
- **Memory:** < 300 MB typical, < 500 MB maximum
- **GPU:** N/A (RemoteFX is CPU-based)

---

## QUALITY REQUIREMENTS

### Code Quality
- **Test Coverage:** > 80% line coverage
- **Documentation:** 100% public API documented with rustdoc
- **Linting:** Zero clippy warnings
- **Formatting:** 100% rustfmt compliant
- **Security:** Pass cargo-audit, no unsafe code without justification

### Testing Requirements
- **Unit Tests:** Every module must have comprehensive unit tests
- **Integration Tests:** End-to-end connection, video, input, clipboard tests
- **Performance Tests:** Benchmarks for encoding and pipeline
- **Compatibility Tests:** Verified on GNOME, KDE, Sway with Intel/AMD GPUs
- **Client Tests:** Verified with Windows 10/11 mstsc and FreeRDP

### Documentation Requirements
- **User Guide:** Installation, configuration, troubleshooting
- **Deployment Guide:** Production deployment, systemd setup, security hardening
- **API Documentation:** Complete rustdoc for all public APIs
- **Architecture Documentation:** System design, data flows, protocol interactions

---

## SECURITY REQUIREMENTS

### Authentication & Encryption
- **TLS 1.3 Only:** No fallback to older TLS versions
- **NLA Required:** Network Level Authentication via CredSSP
- **PAM Integration:** System user authentication
- **Certificate Validation:** Proper certificate chain validation

### Access Control
- **Portal Permissions:** All compositor access via portals (user approval required)
- **No Direct Wayland Access:** No direct protocol access, only via portals
- **Clipboard Restrictions:** Configurable MIME type whitelist, size limits
- **Session Isolation:** Proper multi-user session isolation

### Auditing
- **Authentication Logging:** All auth attempts logged
- **Connection Logging:** Client connections, disconnections, failures
- **Permission Logging:** Portal permission grants/denials
- **Transfer Logging:** Clipboard data transfers

---

## COMPATIBILITY MATRIX

### Supported Compositors
| Compositor | Version | Status | Notes |
|------------|---------|--------|-------|
| GNOME | 45+ | ✅ Primary | Best tested |
| KDE Plasma | 6.0+ | ✅ Primary | Full support |
| Sway | 1.8+ | ✅ Secondary | wlroots-based |
| Hyprland | Latest | 🔶 Best Effort | Community tested |

### Supported Hardware
| Component | Requirement | Notes |
|-----------|-------------|-------|
| CPU | x86_64 with SSE4.2 | RemoteFX benefits from SIMD |
| Memory | 4GB minimum | 8GB+ recommended |
| GPU | Not required | RemoteFX is CPU-based |

### Supported Clients
| Client | Version | Status | Notes |
|--------|---------|--------|-------|
| Windows mstsc | 10/11 | ✅ Primary | Native client |
| FreeRDP | 2.11+ | ✅ Primary | Testing client |
| xrdp client | 0.9+ | 🔶 Best Effort | Not priority |

---

## BUILD AND RUNTIME REQUIREMENTS

### Build Requirements
- **Rust:** 1.75.0 or newer
- **Cargo:** Latest stable
- **System Libraries:** See 02-TECHNOLOGY-STACK.md
- **Build Tools:** pkg-config, cmake, clang

### Runtime Requirements
- **Linux Kernel:** 6.0+
- **Wayland Compositor:** Running and active
- **PipeWire:** 0.3.77+ running
- **xdg-desktop-portal:** 1.18+ with compositor backend running
- **PAM:** For authentication

---

## FILE ORGANIZATION

### Repository Structure
```
wrd-server/
├── Cargo.toml              # Rust project manifest
├── Cargo.lock              # Dependency lock file
├── src/                    # Source code
│   ├── main.rs
│   ├── lib.rs
│   ├── config/             # Configuration module
│   ├── server/             # IronRDP server integration
│   ├── portal/             # Portal integration
│   ├── pipewire/           # PipeWire integration
│   ├── bitmap/             # Bitmap conversion for RemoteFX
│   ├── input/              # Input handling
│   ├── clipboard/          # Clipboard management
│   ├── multimon/           # Multi-monitor support
│   ├── security/           # Security (TLS, auth)
│   └── utils/              # Utilities
├── tests/                  # Integration tests
├── benches/                # Performance benchmarks
├── config/                 # Default configuration files
├── scripts/                # Installation and setup scripts
└── docs/                   # User documentation
```

### Specification Documents Structure
```
wrd-server-specs/
├── 00-MASTER-SPECIFICATION.md          # This document
├── 01-ARCHITECTURE.md                   # System architecture
├── 02-TECHNOLOGY-STACK.md               # Tech stack details
├── 03-PROJECT-STRUCTURE.md              # Directory layout
├── 04-DATA-STRUCTURES.md                # Type definitions
├── 05-PROTOCOL-SPECIFICATIONS.md        # Protocol details
├── IRONRDP-INTEGRATION-GUIDE.md        # IronRDP integration guide
├── phase1-tasks/                        # Individual task specs
│   ├── TASK-P1-01-FOUNDATION.md
│   ├── TASK-P1-02-SECURITY.md
│   ├── TASK-P1-03-PORTAL-INTEGRATION.md
│   ├── TASK-P1-04-PIPEWIRE.md
│   ├── TASK-P1-05-BITMAP-CONVERSION.md
│   ├── TASK-P1-06-IRONRDP-SERVER-INTEGRATION.md
│   ├── TASK-P1-07-INPUT-HANDLING.md
│   ├── TASK-P1-08-CLIPBOARD.md
│   ├── TASK-P1-09-MULTIMONITOR.md
│   └── TASK-P1-10-TESTING-INTEGRATION.md
└── reference/                           # Reference docs
    ├── TESTING-SPECIFICATION.md
    ├── DEPLOYMENT-GUIDE.md
    ├── API-REFERENCE.md
    ├── PERFORMANCE-REQUIREMENTS.md
    └── SECURITY-REQUIREMENTS.md
```

---

## IMPLEMENTATION RULES

### Mandatory Rules (MUST)
1. **MUST** follow Rust 2021 edition conventions
2. **MUST** use exact dependency versions from Cargo.toml
3. **MUST** write unit tests for every public function
4. **MUST** document all public APIs with rustdoc
5. **MUST** handle all errors (no unwrap/expect in production code)
6. **MUST** use structured logging (tracing crate)
7. **MUST** follow the exact module structure defined in specifications
8. **MUST** implement graceful shutdown on SIGTERM/SIGINT
9. **MUST** validate all configuration at startup
10. **MUST** implement proper resource cleanup

### Recommended Rules (SHOULD)
1. **SHOULD** prefer async/await over manual Future implementations
2. **SHOULD** use type-safe builders for complex configurations
3. **SHOULD** use newtype pattern for domain-specific types
4. **SHOULD** limit function complexity (cyclomatic complexity < 10)
5. **SHOULD** keep functions under 50 lines
6. **SHOULD** use const for configuration constants
7. **SHOULD** implement Display and Debug for custom types
8. **SHOULD** use thiserror for error types
9. **SHOULD** use anyhow for application-level errors
10. **SHOULD** profile before optimizing

### Forbidden Rules (MUST NOT)
1. **MUST NOT** use unsafe code without explicit justification and review
2. **MUST NOT** use unwrap/expect in production code paths
3. **MUST NOT** ignore errors (use ? or explicit handling)
4. **MUST NOT** use println/eprintln (use tracing instead)
5. **MUST NOT** hardcode paths or configuration
6. **MUST NOT** use global mutable state without proper synchronization
7. **MUST NOT** block the async runtime with sync operations
8. **MUST NOT** leak resources (file descriptors, memory, threads)
9. **MUST NOT** deviate from the protocol specifications
10. **MUST NOT** commit secrets or credentials to repository

---

## VERSIONING AND RELEASES

### Version Scheme
Follow Semantic Versioning 2.0.0:
- **MAJOR:** Incompatible API changes
- **MINOR:** Backwards-compatible functionality additions
- **PATCH:** Backwards-compatible bug fixes

### Release Schedule
- **v0.1.0:** Phase 1 Milestone 6 complete (basic video streaming)
- **v0.5.0:** Phase 1 complete (all core features)
- **v0.9.0:** Phase 2 feature complete (audio working)
- **v1.0.0:** Production release (all tests passing, documented)

### Release Criteria for v1.0.0
- ✅ All Phase 1 features working
- ✅ All Phase 2 features working
- ✅ All integration tests passing
- ✅ Performance targets met
- ✅ Documentation complete
- ✅ Security audit passed
- ✅ Tested on 3+ compositors
- ✅ Tested with Windows 10 and 11 clients
- ✅ Zero known critical bugs

---

## SUPPORT AND MAINTENANCE

### Supported Configurations
Only configurations explicitly listed in this specification and tested in the compatibility matrix are officially supported.

### Issue Reporting
Issues must include:
1. wrd-server version
2. Linux distribution and version
3. Wayland compositor and version
4. GPU vendor and driver version
5. Client OS and RDP client version
6. Complete logs with debug level enabled
7. Steps to reproduce

### Bug Priority Levels
- **Critical:** Crashes, data loss, security vulnerabilities
- **High:** Core features not working, severe performance degradation
- **Medium:** Minor features not working, moderate issues
- **Low:** Cosmetic issues, enhancement requests

---

## CHANGE CONTROL

### Specification Updates
- **Minor Changes:** Typos, clarifications - no approval needed
- **Major Changes:** Architecture, API changes - requires team approval
- **Version Control:** All specification changes tracked in git

### Code Changes
- **All code changes must reference specification document sections**
- **Specification must be updated before or with code changes**
- **Breaking changes require specification version bump**

---

## SUCCESS CRITERIA

### Phase 1 Success
- ✅ Windows mstsc connects and displays desktop
- ✅ Keyboard and mouse control works
- ✅ Clipboard copy/paste works both directions
- ✅ Multi-monitor displays correctly
- ✅ Frame rate stable at 30 FPS
- ✅ Latency < 100ms on LAN
- ✅ All integration tests pass
- ✅ Works on GNOME, KDE, and Sway

### Phase 2 Success
- ✅ Audio plays on client
- ✅ Microphone works from client
- ✅ Audio/video synchronized
- ✅ Performance targets exceeded
- ✅ Complete documentation

### Overall Project Success
- ✅ Production deployment at 10+ sites
- ✅ Zero critical bugs in 30 days
- ✅ Positive user feedback
- ✅ Community adoption

---

## NEXT STEPS

### For Implementors
1. Read **01-ARCHITECTURE.md** for system design understanding
2. Read **02-TECHNOLOGY-STACK.md** for dependency setup
3. Read **IRONRDP-INTEGRATION-GUIDE.md** for IronRDP usage
4. Select a task from **phase1-tasks/** directory (P1-01 through P1-10)
5. Read the specific task specification completely
6. Implement exactly as specified
7. Write tests as specified
8. Submit for review with reference to specification section

### For Reviewers
1. Verify implementation matches specification exactly
2. Check test coverage meets requirements
3. Run integration tests
4. Verify performance meets targets
5. Approve or request changes with specification references

### For Project Managers
1. Track milestone completion against Phase 1/2 specifications
2. Monitor performance metrics against targets
3. Ensure all deliverables are completed
4. Coordinate between task implementors
5. Manage specification change requests

---

## APPENDIX: ACRONYMS AND TERMINOLOGY

- **DMA-BUF:** Direct Memory Access Buffer
- **FPS:** Frames Per Second
- **NLA:** Network Level Authentication
- **PAM:** Pluggable Authentication Modules
- **PDU:** Protocol Data Unit
- **PTS:** Presentation Timestamp
- **RDP:** Remote Desktop Protocol
- **RemoteFX:** Microsoft's RDP codec for bitmap compression
- **RFB:** Remote Framebuffer (VNC protocol)
- **RGB:** Red Green Blue color model
- **TLS:** Transport Layer Security
- **VNC:** Virtual Network Computing

---

## DOCUMENT REVISION HISTORY

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-01-18 | System | Initial authoritative specification |
| 2.0 | 2025-11-18 | System | Major update: Replaced H.264 with RemoteFX, integrated IronRDP server, reduced timeline from 18 to 14 weeks |

---

**END OF MASTER SPECIFICATION**

Read all associated specification documents before beginning implementation.
