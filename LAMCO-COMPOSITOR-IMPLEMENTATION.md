# Lamco Compositor Implementation - Complete Guide
**Branch**: feature/lamco-compositor-clipboard
**Status**: Ready to implement
**Timeline**: 10-15 days to working clipboard

---

## EXECUTIVE SUMMARY - THE SOLUTION

**Problem**: Linux→Windows clipboard doesn't work (SelectionOwnerChanged signal broken)

**Solution**: Use Lamco compositor's built-in clipboard monitoring via Smithay data_device protocol

**How It Works**:
```
Wayland App Copies → wl_data_device.set_selection()
  → Smithay calls SelectionHandler::new_selection()
  → Read clipboard data via file descriptor
  → Convert MIME → RDP formats
  → Send CLIPRDR Format List to RDP client
  → Done! ✅
```

**No Portal. No polling. Direct Wayland protocol.**

---

## WHAT'S ALREADY BUILT

From feature/headless-infrastructure (now merged):

**Complete Implementation** (4,586 lines):
- ✅ src/compositor/protocols/data_device.rs - Clipboard protocol
- ✅ src/compositor/state.rs - Event system
- ✅ src/compositor/integration.rs - RDP bridge
- ✅ src/compositor/input.rs - Input injection
- ✅ All Wayland protocols (compositor, xdg_shell, seat, shm, output)

**The Critical Code**:
```rust
// src/compositor/protocols/data_device.rs:27
impl SelectionHandler for CompositorState {
    fn new_selection(
        &mut self,
        ty: SelectionTarget,
        source: Option<WlDataSource>,
    ) {
        match ty {
            SelectionTarget::Clipboard => {
                // 🎯 THIS IS THE CLIPBOARD MONITOR!
                // Fires every time clipboard changes
                debug!("New clipboard selection: {:?}", source);
                
                // Currently: Just logs
                // TODO: Read data, send to RDP
            }
        }
    }
}
```

---

## CURRENT STATUS

**Compilation**: ❌ 91 errors (Smithay API mismatch)
**Architecture**: ✅ Correct and complete
**Integration**: ⏳ Needs wiring to current RDP server

---

## IMPLEMENTATION ROADMAP

### Week 1: Fix Compilation

**Day 1**: Smithay Dependency Update
- Update Cargo.toml to Smithay 0.7.0
- Add required Smithay features
- Update delegate macros (0.3→0.7 breaking changes)

**Day 2**: Handler Trait Updates
- Fix CompositorHandler implementation
- Fix SeatHandler implementation
- Fix SelectionHandler implementation
- Fix DataDeviceHandler implementation

**Day 3**: Build System
- Resolve all 91 compile errors
- Add headless-compositor feature flag
- Get clean build

### Week 2: Clipboard Integration

**Day 4**: Wire new_selection to RDP
- Read data from WlDataSource
- Convert MIME types to RDP formats
- Send Format List via existing clipboard manager

**Day 5**: Wire RDP to send_selection
- Receive RDP clipboard data
- Write to file descriptor
- Handle MIME type conversion

**Day 6**: Testing
- Test Wayland app → RDP copy
- Test RDP → Wayland app paste
- Verify loop prevention

### Week 3: Backend & Integration

**Day 7-8**: Choose and Implement Backend
- Option A: X11 backend with Xvfb (easier)
- Option B: DRM backend with virtio-gpu (production)

**Day 9**: Framebuffer Export
- Wire compositor render to RDP encoder
- Test video streaming

**Day 10**: Full Integration Testing
- Video + Input + Clipboard all working
- Performance testing
- Headless deployment test

---

## TECHNICAL DETAILS

### Smithay 0.7.0 Requirements

**Cargo.toml Updates**:
```toml
[dependencies]
smithay = { version = "0.7", features = [
    "wayland_frontend",
    "desktop",
    "backend_x11",  # Or backend_drm
] }
calloop = "0.13"
wayland-server = "0.31"
```

### new_selection() Implementation

```rust
fn new_selection(
    &mut self,
    ty: SelectionTarget,
    source: Option<SelectionSource>,
    seat: Seat<Self>,
) {
    if let Some(source) = source {
        // Get available MIME types
        let mime_types = source.mime_types();
        
        // Create pipe for reading clipboard data
        let (read_fd, write_fd) = nix::unistd::pipe().unwrap();
        
        // Request data in preferred MIME type
        let mime = mime_types.first().unwrap_or(&"text/plain".to_string());
        
        request_data_device_client_selection(
            &seat,
            mime,
            write_fd,
        ).unwrap();
        
        // Spawn task to read and send to RDP
        let rdp_tx = self.rdp_clipboard_tx.clone();
        tokio::spawn(async move {
            let mut data = Vec::new();
            use std::io::Read;
            let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
            file.read_to_end(&mut data).unwrap();
            
            // Send to RDP via existing clipboard manager
            rdp_tx.send(ClipboardEvent::PortalFormatsAvailable(mime_types)).unwrap();
        });
    }
}
```

---

## INTEGRATION WITH CURRENT CODEBASE

### Clipboard Manager Bridge

**Current** (main branch):
```
src/clipboard/manager.rs
  ├─> Uses Portal clipboard
  ├─> Has polling fallback (disabled)
  └─> Has all RDP integration
```

**Compositor** (this branch):
```
src/compositor/protocols/data_device.rs
  ├─> Has SelectionHandler callbacks
  ├─> Needs wiring to RDP
  └─> Provides change events
```

**Integration Strategy**:
```rust
// Option A: Compositor sends events to existing clipboard manager
compositor.on_clipboard_change(|mime_types| {
    clipboard_manager.handle_event(
        ClipboardEvent::PortalFormatsAvailable(mime_types)
    );
});

// Option B: Direct RDP integration in compositor
compositor.new_selection() → read data → CLIPRDR Format List directly
```

**Recommendation**: Option A - reuse existing clipboard manager logic

---

## DEPLOYMENT MODES

### Mode 1: Desktop (Portal)
**Current working setup**
- Uses Portal for video/input
- Compositor NOT needed
- Clipboard via Portal (with polling)

### Mode 2: Headless (Compositor)
**Future capability**
- Uses Lamco compositor
- No Portal needed
- Clipboard via data_device protocol ✅
- Input injection via compositor
- Video via compositor framebuffer

### Mode 3: Hybrid
**Best of both**
- Portal for video/input (existing, stable)
- Compositor clipboard only (for monitoring)
- Gradual migration path

---

## NEXT SESSION START COMMANDS

```bash
# 1. Switch to branch
git checkout feature/lamco-compositor-clipboard

# 2. Check compositor code
ls -la src/compositor/

# 3. Try to build
cargo check --features headless-compositor

# 4. Analyze errors
cargo check --features headless-compositor 2>&1 | grep "^error" | wc -l

# 5. Start fixing
# Follow error list systematically
```

---

## SUCCESS CRITERIA

**Phase 1 Complete** when:
- ✅ Code compiles with Smithay 0.7.0
- ✅ All handler traits implemented correctly
- ✅ Clean build with headless-compositor feature

**Phase 2 Complete** when:
- ✅ Wayland app copy triggers RDP Format List
- ✅ RDP paste provides data to Wayland app
- ✅ Bidirectional clipboard working

**Phase 3 Complete** when:
- ✅ Compositor runs headless
- ✅ Video + Input + Clipboard all working
- ✅ Can deploy to cloud VM

---

END OF IMPLEMENTATION GUIDE
