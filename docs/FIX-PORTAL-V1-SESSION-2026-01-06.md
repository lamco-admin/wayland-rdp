# Portal v1 Session Access Fix

**Date:** 2026-01-06
**Issue:** Crash on Portal v1 (RHEL 9) due to missing session access
**Status:** Deployed to RHEL 9, ready for testing

## Deployment Status

- **Local build:** Success (296/296 tests passed)
- **RHEL 9 build:** Success (2m 27s)
- **Files deployed:**
  - `src/session/strategy.rs`
  - `src/session/strategies/portal_token.rs`
  - `src/server/mod.rs`

---

## Problem Description

### Symptoms
- Server crashes immediately on RHEL 9 GNOME 40
- Works fine on Ubuntu 24.04 GNOME 46 (Portal v2+)
- Crash occurs during startup, before RDP connection

### Root Cause

**Current problematic code flow:**

1. `PortalTokenStrategy::create_session()` creates a Portal session
   - On Portal v1: `clipboard_manager = None` (no clipboard support)
   - On Portal v2+: `clipboard_manager = Some(Arc<ClipboardManager>)`

2. `PortalSessionHandleImpl::portal_clipboard()` returns:
   ```rust
   // Returns None when clipboard_manager is None (Portal v1)
   self.clipboard_manager.as_ref().map(|mgr| {
       ClipboardComponents {
           manager: Arc::clone(mgr),
           session: Arc::clone(&self.session),
       }
   })
   ```

3. `server/mod.rs:325-327` tries to get session:
   ```rust
   let session = clipboard_components
       .map(|c| c.session)
       .unwrap_or_else(|| Arc::new(Mutex::new(unsafe { std::mem::MaybeUninit::uninit().assume_init() })));
   ```
   This creates UNINITIALIZED memory when `clipboard_components` is `None` → undefined behavior → crash

### Why This Happened

The design assumed clipboard and session are always paired. But on Portal v1:
- Session exists (for video + input)
- Clipboard manager doesn't exist (Portal v1 doesn't support clipboard)

The code conflated "no clipboard" with "no session".

---

## Fix Design

### Principle
Separate the concepts: session is always available, clipboard manager is optional.

### Changes Required

#### 1. strategy.rs - ClipboardComponents struct (lines 17-22)

**Before:**
```rust
pub struct ClipboardComponents {
    pub manager: Arc<lamco_portal::ClipboardManager>,
    pub session: Arc<Mutex<Session<...>>>,
}
```

**After:**
```rust
pub struct ClipboardComponents {
    /// Portal clipboard manager - None on Portal v1 (no clipboard support)
    pub manager: Option<Arc<lamco_portal::ClipboardManager>>,
    /// Portal session - always present
    pub session: Arc<Mutex<Session<...>>>,
}
```

#### 2. portal_token.rs - portal_clipboard() method (lines 99-108)

**Before:**
```rust
fn portal_clipboard(&self) -> Option<ClipboardComponents> {
    self.clipboard_manager.as_ref().map(|mgr| {
        ClipboardComponents {
            manager: Arc::clone(mgr),
            session: Arc::clone(&self.session),
        }
    })
}
```

**After:**
```rust
fn portal_clipboard(&self) -> Option<ClipboardComponents> {
    // Always return Some for Portal strategy - session is always available
    // Manager may be None on Portal v1 (no clipboard support)
    Some(ClipboardComponents {
        manager: self.clipboard_manager.clone(),
        session: Arc::clone(&self.session),
    })
}
```

#### 3. server/mod.rs - session extraction (lines 320-329)

**Before:**
```rust
let clipboard_components = session_handle.portal_clipboard();
let clipboard_mgr = clipboard_components.as_ref().map(|c| c.manager.clone());

let session = clipboard_components
    .map(|c| c.session)
    .unwrap_or_else(|| Arc::new(Mutex::new(unsafe { std::mem::MaybeUninit::uninit().assume_init() })));
```

**After:**
```rust
let clipboard_components = session_handle.portal_clipboard()
    .expect("Portal strategy always provides ClipboardComponents");

let clipboard_mgr = clipboard_components.manager.clone();
let session = clipboard_components.session;
```

---

## Testing Plan

1. **Build locally:** `cargo build --release && cargo test`
2. **Deploy to RHEL 9:** Transfer fixed source and rebuild
3. **Test connection:** Should see ONE permission dialog
4. **Verify:**
   - Video displays
   - Keyboard input works
   - Mouse input works (check alignment)
   - No crashes

---

## Rollback Plan

If fix doesn't work, reset to commit c572605:
```bash
git reset --hard c572605
```

This commit has the two-dialog bug but doesn't crash.

---

## Files Modified

1. `src/session/strategy.rs` - ClipboardComponents.manager becomes Option
2. `src/session/strategies/portal_token.rs` - portal_clipboard() always returns Some
3. `src/server/mod.rs` - Remove unsafe uninitialized memory hack
