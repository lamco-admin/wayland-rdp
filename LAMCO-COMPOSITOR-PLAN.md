# Lamco Compositor - Clipboard Integration Plan
**Date**: 2025-11-20
**Branch**: feature/lamco-compositor-clipboard
**Goal**: Integrate Smithay-based compositor clipboard with WRD-Server

---

## STRATEGIC DECISION

After deep research, the BEST solution for WRD-Server is:

**Use the existing 4,586-line compositor implementation** (from feature/headless-infrastructure)

**Why**:
1. ✅ Already has `SelectionHandler::new_selection()` - clipboard change detection BUILT-IN
2. ✅ Solves Linux→Windows clipboard immediately
3. ✅ Enables headless deployment simultaneously  
4. ✅ Pure Rust stack (Smithay + IronRDP)
5. ✅ No Portal dependency for headless mode
6. ✅ Full control over all protocols

---

## WHAT WE HAVE

**Existing Code** (feature/headless-infrastructure branch):
- src/compositor/ - 4,586 lines
- Complete Wayland protocols
- **Clipboard via data_device protocol** ✅
- Event system with `CompositorEvent::ClipboardChanged`
- RDP bridge architecture

**Status**: Doesn't compile (91 errors) - likely Smithay API version mismatch

---

## IMMEDIATE PLAN

### Phase 1: Fix Compilation (Days 1-3)
1. Update Smithay to 0.7.0
2. Fix API compatibility (delegate macros, handler traits)
3. Get clean build

### Phase 2: Clipboard Integration (Days 4-6)
1. Wire `new_selection()` → RDP Format List
2. Wire RDP data → `send_selection()`  
3. Test bidirectional clipboard

### Phase 3: Backend Selection (Days 7-10)
1. Choose X11 (Xvfb) or DRM backend
2. Implement framebuffer export for RDP
3. Test headless operation

---

## KEY INSIGHT

**The compositor ALREADY HAS clipboard monitoring!**

```rust
// From src/compositor/protocols/data_device.rs:
impl SelectionHandler for CompositorState {
    fn new_selection(&mut self, ty: SelectionTarget, source: Option<WlDataSource>) {
        match ty {
            SelectionTarget::Clipboard => {
                // THIS FIRES WHEN CLIPBOARD CHANGES! ✅
                debug!("New clipboard selection: {:?}", source);
                
                // TODO: Wire to RDP (read data, announce formats)
            }
        }
    }
}
```

**This is the solution!** No Portal, no polling, no backend bugs. Direct Wayland protocol.

---

## RESEARCH COMPLETE

**Smithay 0.7.0 Documentation**: Deep dive completed
**Clipboard API**: SelectionHandler fully understood
**Integration Pattern**: calloop + message passing documented
**Backend Options**: X11/DRM analyzed

**Ready to implement.**

---

END OF PLAN
