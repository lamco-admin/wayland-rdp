# CLAUDE CODE WEB - NEXT SESSION PROMPT
**Task:** TASK-P1-03 - Portal Integration
**Repository:** https://github.com/lamco-admin/wayland-rdp
**Branch:** Create new branch `task/p1-03-portal`
**Duration:** 5-7 days estimated

---

## SESSION START PROMPT

```
I'm continuing development of the Wayland Remote Desktop Server (wrd-server) project.

Repository: https://github.com/lamco-admin/wayland-rdp
Previous work completed: Foundation (P1-01) and Security (P1-02) modules

TASK ASSIGNMENT: Implement Portal Integration (TASK-P1-03-REVISED)

Please read and implement the complete specification in:
https://github.com/lamco-admin/wayland-rdp/blob/main/phase1-tasks/TASK-P1-03-PORTAL-INTEGRATION-REVISED.md

Key requirements:
1. Create src/portal/ module with all submodules
2. Implement PortalManager using ashpd crate
3. Implement ScreenCast, RemoteDesktop, and Clipboard portal managers
4. Create PortalSessionHandle for session management
5. Write integration test that creates portal session
6. Create example program (examples/portal_info.rs) that demonstrates portal usage

Technical notes:
- Use ashpd = "0.12.0" (already in Cargo.toml)
- Connect to D-Bus session bus
- Create RemoteDesktop session (includes screen capture)
- Request Keyboard + Pointer devices
- Return PipeWire file descriptor and stream info
- All portal methods should be async

Success criteria:
- Portal session creates successfully
- User permission dialog appears
- PipeWire FD obtained (positive integer)
- Stream metadata correct
- Input injection methods callable
- All tests pass

Follow the specification exactly. Ask clarifying questions if the ashpd API has changed from what's specified.
```

---

## CONTEXT FOR CCW

### What's Already Complete
- ✅ Project structure (Cargo.toml, directory layout)
- ✅ Configuration system (src/config/)
- ✅ Main entry point (src/main.rs)
- ✅ Security module (src/security/) with TLS and auth
- ✅ Logging infrastructure

### What This Task Adds
- New module: `src/portal/` with 5 files
- Portal integration using ashpd
- D-Bus connection management
- Session lifecycle
- Input injection API wrapper
- Integration test
- Example program

### What Comes After This Task
- P1-04: PipeWire integration (connect to stream using FD from this task)
- P1-05: IronRDP server integration (use portal for input/output)

### Dependencies Available
From Cargo.toml (already configured):
```toml
ashpd = { version = "0.12.0", features = ["tokio"] }
zbus = "4.0.1"
async-trait = "0.1.77"
tokio = { version = "1.35", features = ["full"] }
```

---

## FILE STRUCTURE TO CREATE

```
src/portal/
├── mod.rs              # PortalManager coordinator
├── screencast.rs       # ScreenCast portal wrapper
├── remote_desktop.rs   # RemoteDesktop portal wrapper
├── clipboard.rs        # Clipboard portal wrapper
└── session.rs          # PortalSessionHandle and StreamInfo types

tests/integration/
└── portal_test.rs      # Integration test

examples/
└── portal_info.rs      # Demo program
```

---

## TESTING REQUIREMENTS

### Unit Tests
Add to each module in `#[cfg(test)]` blocks.

### Integration Test
`tests/integration/portal_test.rs` must:
- Create portal manager
- Create session (triggers permission dialog)
- Verify PipeWire FD is valid
- Verify streams detected
- Test input injection
- Close session cleanly

**Mark as #[ignore]** since it requires Wayland session.

### Manual Testing
Run in actual Wayland session:
```bash
cargo test --test portal_test -- --ignored --nocapture
cargo run --example portal_info
```

---

## EXPECTED BEHAVIOR

When running the example:
1. Program starts
2. Connects to D-Bus
3. Creates portal session
4. **Permission dialog appears on desktop** asking to share screen and control input
5. User clicks "Allow"
6. Program receives PipeWire FD (e.g., FD=3 or similar)
7. Program receives stream info (resolution, position, etc.)
8. Program can call input injection methods
9. Mouse cursor moves when injection methods called
10. Program exits cleanly

---

## COMMON ISSUES TO HANDLE

### If ashpd API differs from spec:
- Check ashpd docs: https://docs.rs/ashpd/0.12.0/
- Adapt implementation to actual API
- Document the difference in code comments

### If permission dialog doesn't appear:
- Check xdg-desktop-portal is running
- Check compositor-specific backend is installed
- Add debug logging

### If PipeWire FD is invalid:
- Verify PipeWire service is running
- Check portal response structure
- Ensure FD isn't being closed prematurely

---

## DELIVERABLES CHECKLIST

Before marking task complete:

- [ ] src/portal/mod.rs implemented
- [ ] src/portal/screencast.rs implemented
- [ ] src/portal/remote_desktop.rs implemented
- [ ] src/portal/clipboard.rs implemented
- [ ] src/portal/session.rs implemented
- [ ] tests/integration/portal_test.rs created
- [ ] examples/portal_info.rs created
- [ ] cargo build succeeds
- [ ] cargo test passes (unit tests)
- [ ] cargo run --example portal_info works (manual test in Wayland)
- [ ] All rustdoc comments added
- [ ] Code formatted (cargo fmt)
- [ ] No clippy warnings (cargo clippy)

---

## REFERENCE

**Specification:** TASK-P1-03-PORTAL-INTEGRATION-REVISED.md (in repo)
**IronRDP Guide:** IRONRDP-INTEGRATION-GUIDE.md (in repo)
**ashpd docs:** https://docs.rs/ashpd/0.12.0/

---

**Ready to start implementation!**
