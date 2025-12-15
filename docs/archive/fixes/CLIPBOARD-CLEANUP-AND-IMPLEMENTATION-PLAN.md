# Clipboard Cleanup and Complete Implementation Plan

**Date:** 2025-11-19
**Status:** COMPREHENSIVE CLEANUP + CORRECT IMPLEMENTATION
**Based On:** Complete spec review, API research, handover document analysis

---

## Executive Summary

After thorough analysis of:
- All architecture specifications
- ashpd Clipboard API official documentation
- Session handover document
- Current codebase state

This plan provides systematic cleanup of old/incorrect implementations and complete replacement with correct Portal Clipboard delayed rendering architecture.

---

## CURRENT STATE ANALYSIS

### ✅ KEEP - Correct and Production-Ready

| File | Lines | Status | Purpose |
|------|-------|--------|---------|
| `src/clipboard/formats.rs` | 936 | ✅ KEEP | Complete format conversion (RDP↔MIME, UTF-8↔UTF-16, DIB↔PNG, HDROP↔URI) |
| `src/clipboard/sync.rs` | 716 | ✅ KEEP | Loop detection, state machine, operation history |
| `src/clipboard/transfer.rs` | 601 | ✅ KEEP | Chunked transfer engine, progress tracking for files |
| `src/clipboard/error.rs` | 446 | ✅ KEEP | Comprehensive error handling, retry logic |
| `src/clipboard/mod.rs` | 126 | ✅ KEEP | Module exports |

**Rationale**: These files implement production-quality infrastructure (loop prevention, format conversion, chunked transfer, error handling) that Portal Clipboard API requires but doesn't provide.

### ❌ REMOVE - Wrong Solution

| File | Location | Issue | Action |
|------|----------|-------|--------|
| Cargo.toml | Line 78 | `wl-clipboard-rs = "0.8"` dependency | DELETE |
| manager.rs | Line 419-441 | wl-clipboard-rs usage | REPLACE with Portal API |

**Rationale**: wl-clipboard-rs uses `zwlr_data_control_manager_v1` protocol which is wlroots-specific. Fails on GNOME/KDE. Portal Clipboard API is the universal solution.

### 🔧 FIX - Incomplete/Incorrect Implementation

| File | Issue | Fix Required |
|------|-------|--------------|
| `src/clipboard/manager.rs` | Portal parameters cloned at event processor spawn (too early, captures None values) | Wrap in Arc<RwLock<Option<>>> for dynamic update |
| `src/portal/clipboard.rs` | Missing SelectionTransfer listener implementation | Add complete listener with channel-based architecture |
| `src/portal/clipboard.rs` | Missing SelectionOwnerChanged listener | Add listener for Linux→Windows flow |
| `src/portal/clipboard.rs` | Old `provide_data()` method doesn't use proper workflow | Replace with `write_selection_data()` using serial |
| `src/clipboard/manager.rs` | handle_rdp_data_response() uses wl-clipboard-rs | Replace with Portal SelectionWrite workflow |
| `src/clipboard/ironrdp_backend.rs` | Missing correlation between SelectionTransfer serial and RDP request | Add pending_transfers tracking |

### 🚧 MISSING - Not Yet Implemented

| Feature | Location | Complexity | Priority |
|---------|----------|------------|----------|
| SelectionTransfer → RDP request flow | manager.rs | Medium | P0 |
| SelectionWrite data delivery | manager.rs | Medium | P0 |
| SelectionOwnerChanged listener | portal/clipboard.rs | Medium | P1 |
| SelectionRead for Linux→Windows | manager.rs | Medium | P1 |
| Serial correlation system | manager.rs | Medium | P0 |
| Image clipboard via Portal | manager.rs | Low | P2 |
| File transfer via Portal | manager.rs | High | P3 |

---

## IMPLEMENTATION PLAN - SYSTEMATIC APPROACH

### Phase 1: Fix Parameter Passing (30 min)

**Problem**: Event processor clones portal_clipboard and portal_session at spawn time, capturing None values.

**Root Cause**:
```rust
// manager.rs line 181-187 (CURRENT - WRONG)
fn start_event_processor(&mut self, ...) {
    let portal_clipboard = self.portal_clipboard.clone(); // None at this time!
    let portal_session = self.portal_session.clone();     // None at this time!

    tokio::spawn(async move {
        // portal_clipboard and portal_session are ALWAYS None here
    });
}
```

**Solution**:
```rust
// Wrap in Arc<RwLock<Option<>>> for dynamic access
portal_clipboard: Arc<RwLock<Option<Arc<PortalClipboardManager>>>>
portal_session: Arc<RwLock<Option<Arc<Mutex<Session>>>>>

// Event processor clones the Arc wrappers
let portal_clipboard = Arc::clone(&self.portal_clipboard);

// Handlers read dynamically
let portal_opt = portal_clipboard.read().await.clone();
```

**Files to Modify**:
- `src/clipboard/manager.rs` (field types, initialization, all event handlers)
- `src/server/mod.rs` (make set_portal_clipboard() async)

---

### Phase 2: Remove wl-clipboard-rs Completely (15 min)

**Current Problem**:
- manager.rs:419-441 uses wl-clipboard-rs
- Fails with "zwlr_data_control_manager_v1 not supported" on GNOME/KDE
- Temporary fallback that never worked

**Action**:
1. Remove Cargo.toml dependency
2. Remove use statement in manager.rs
3. Replace implementation with Portal API (done in Phase 3)

**Success Criteria**: Zero references to wl_clipboard_rs in codebase

---

### Phase 3: Implement Windows→Linux Flow (2-3 hours)

**Required Workflow** (per XDG spec + ashpd API):

```
Windows user copies
  ↓
RDP FormatList PDU
  ↓
on_remote_copy() callback
  ↓
Portal.set_selection(session, mime_types) - Announce WITHOUT data
  ↓
Linux user pastes
  ↓
Portal SelectionTransfer signal (session, mime_type, serial)
  ↓
Request data from RDP via ServerEvent::Clipboard(SendInitiatePaste)
  ↓
on_format_data_response() receives data
  ↓
Convert UTF-16→UTF-8
  ↓
Portal.selection_write(session, serial) → returns FD
  ↓
Write data to FD
  ↓
Portal.selection_write_done(session, serial, true)
  ↓
Linux app receives clipboard data ✅
```

**Implementation Tasks**:

1. **portal/clipboard.rs**: Add SelectionTransfer listener
   ```rust
   pub async fn start_selection_transfer_listener(
       &self,
       event_tx: mpsc::UnboundedSender<(String, u32)>
   ) -> Result<()>
   ```

2. **manager.rs**: Wire SelectionTransfer to RDP request
   ```rust
   // In set_portal_clipboard():
   // Start listener, spawn handler task
   // On SelectionTransfer:
   //   1. Track (mime_type, serial)
   //   2. Convert MIME → format_id
   //   3. Send ServerEvent::Clipboard(SendInitiatePaste(format_id))
   //   4. Store serial for correlation
   ```

3. **manager.rs**: Complete handle_rdp_data_response()
   ```rust
   // When FormatDataResponse arrives:
   //   1. Get pending serial (from step 2.4)
   //   2. Convert UTF-16→UTF-8
   //   3. Call portal.write_selection_data(session, serial, data)
   //   4. Clear pending request
   ```

4. **portal/clipboard.rs**: Implement write_selection_data()
   ```rust
   pub async fn write_selection_data(
       &self,
       session: &Session<'_, RemoteDesktop<'_>>,
       serial: u32,
       data: Vec<u8>
   ) -> Result<()> {
       let fd = self.clipboard.selection_write(session, serial).await?;
       // Write to FD using tokio AsyncWriteExt
       // Call selection_write_done(session, serial, true)
   }
   ```

**Success Criteria**: Windows→Linux text clipboard working end-to-end

---

### Phase 4: Implement Linux→Windows Flow (2-3 hours)

**Required Workflow**:

```
Linux user copies
  ↓
Portal SelectionOwnerChanged signal (session, mime_types, session_is_owner=false)
  ↓
Detect new clipboard owner (not us)
  ↓
Convert MIME types → RDP formats
  ↓
Send FormatList PDU to RDP client
  ↓
Windows shows "clipboard available"
  ↓
Windows user pastes
  ↓
RDP FormatDataRequest PDU
  ↓
on_format_data_request() callback
  ↓
Convert format_id → MIME type
  ↓
Portal.selection_read(session, mime_type) → returns FD
  ↓
Read data from FD
  ↓
Convert UTF-8→UTF-16
  ↓
Send FormatDataResponse PDU to RDP client
  ↓
Windows app receives data ✅
```

**Implementation Tasks**:

1. **portal/clipboard.rs**: Add SelectionOwnerChanged listener
   ```rust
   pub async fn start_owner_changed_listener(
       &self,
       event_tx: mpsc::UnboundedSender<Vec<String>>
   ) -> Result<()>
   ```

2. **manager.rs**: Wire SelectionOwnerChanged to FormatList
   ```rust
   // In set_portal_clipboard():
   // Start listener, spawn handler task
   // On SelectionOwnerChanged (if not session_is_owner):
   //   1. Convert MIME types → RDP formats
   //   2. Send FormatList PDU to RDP client
   //      (need ServerEvent mechanism for this - research needed)
   ```

3. **ironrdp_backend.rs**: Complete on_format_data_request()
   ```rust
   // Currently just logs
   // NEEDS:
   //   1. Convert format_id → MIME type
   //   2. Call manager.handle_format_data_request(format_id)
   //   3. Send FormatDataResponse PDU back
   //      (need access to Cliprdr server or message proxy)
   ```

4. **manager.rs**: Implement handle_format_data_request()
   ```rust
   pub async fn handle_format_data_request(&self, format_id: u32) -> Result<Vec<u8>> {
       // 1. Portal.selection_read(session, mime_type) → FD
       // 2. Read from FD
       // 3. Convert UTF-8→UTF-16
       // 4. Return data
   }
   ```

**Success Criteria**: Linux→Windows text clipboard working end-to-end

---

### Phase 5: Add Image Support (1-2 hours)

**Formats to Support**:
- CF_DIB (8) ↔ image/png
- CF_PNG (0xD011) ↔ image/png
- CF_JPEG (0xD012) ↔ image/jpeg
- CF_BITMAP (2) ↔ image/bmp

**Implementation**:
- Format conversion already exists in formats.rs
- Just extend MIME type list in set_selection()
- Test screenshot copy/paste both directions

---

### Phase 6: Add File Transfer (4-6 hours)

**Formats**:
- CF_HDROP (15) ↔ text/uri-list

**Workflow**:
- FormatList with CF_HDROP
- FileContentsRequest/Response for chunked transfer
- Build URI list: `file:///tmp/wrd-clipboard/file_0\nfile:///tmp/.../file_1`
- Use transfer.rs engine for progress tracking

---

### Phase 7: Final Cleanup (1 hour)

1. Remove all unused imports
2. Remove placeholder/TODO comments
3. Clean up debug logging
4. Add production logging
5. Run cargo clippy and fix all warnings
6. Update documentation

---

## DETAILED IMPLEMENTATION GUIDE

### CRITICAL: Portal Parameter Passing Fix

**Current Bug** (from handover):
```
17:54:12 WARN: Portal clipboard or session not available ← PROBLEM!
```

**Analysis**:
```rust
// src/clipboard/manager.rs

// Line 146-155: Constructor creates manager
pub async fn new(config: ClipboardConfig) -> Result<Self> {
    let mut manager = Self {
        portal_clipboard: None,  // ← Set to None
        portal_session: None,    // ← Set to None
        ...
    };

    // Line 158: Start event processor IMMEDIATELY
    manager.start_event_processor(event_rx);  // ← Clones None values!

    return manager;  // Returns manager without Portal set
}

// Line 181-187: Event processor clones at spawn time
fn start_event_processor(&mut self, ...) {
    let portal_clipboard = self.portal_clipboard.clone();  // ← Clones None!
    let portal_session = self.portal_session.clone();      // ← Clones None!

    tokio::spawn(async move {
        // portal_clipboard is ALWAYS None in this closure
        // portal_session is ALWAYS None in this closure
    });
}

// Line 170-178: Set Portal AFTER event processor started
pub fn set_portal_clipboard(...) {
    self.portal_clipboard = Some(portal);  // ← Too late! Already cloned as None
    self.portal_session = Some(session);
}
```

**Calling sequence**:
```rust
// src/server/mod.rs

// Line 254: Create manager (event processor spawned here with None values)
let mut clipboard_mgr = ClipboardManager::new(config).await?;

// Line 260-264: Set Portal (but event processor already has None)
clipboard_mgr.set_portal_clipboard(portal_clip, session);
```

**Fix**:
```rust
// Option 1: Wrap in Arc<RwLock<Option<>>> for dynamic access
portal_clipboard: Arc<RwLock<Option<Arc<PortalClipboardManager>>>>

// Event processor clones the wrapper
let portal_clipboard = Arc::clone(&self.portal_clipboard);

// Handlers read dynamically
let portal_opt = portal_clipboard.read().await.clone();

// Option 2: Don't start processor in constructor, start it in set_portal_clipboard()
// Option 3: Pass Portal to constructor instead of setting later

// CHOSEN: Option 1 (most flexible, no API changes needed)
```

---

## STEP-BY-STEP IMPLEMENTATION

### Step 1: Fix Portal Parameter Passing

**File**: `src/clipboard/manager.rs`

**Changes**:
1. Change field types:
   ```rust
   portal_clipboard: Arc<RwLock<Option<Arc<crate::portal::clipboard::ClipboardManager>>>>
   portal_session: Arc<RwLock<Option<Arc<Mutex<Session>>>>>
   ```

2. Update constructor:
   ```rust
   portal_clipboard: Arc::new(RwLock::new(None))
   portal_session: Arc::new(RwLock::new(None))
   ```

3. Update start_event_processor():
   ```rust
   let portal_clipboard = Arc::clone(&self.portal_clipboard);
   let portal_session = Arc::clone(&self.portal_session);
   ```

4. Update set_portal_clipboard():
   ```rust
   pub async fn set_portal_clipboard(...) {
       *self.portal_clipboard.write().await = Some(portal);
       *self.portal_session.write().await = Some(session);
   }
   ```

5. Update all event handlers to read dynamically:
   ```rust
   let portal_opt = portal_clipboard.read().await.clone();
   let session_opt = portal_session.read().await.clone();
   ```

**File**: `src/server/mod.rs`

**Changes**:
1. Make set_portal_clipboard call async:
   ```rust
   clipboard_mgr.set_portal_clipboard(portal_clip, session).await;
   ```

**Test**: Build succeeds, Portal parameters now available in handlers

---

### Step 2: Implement SelectionTransfer Listener

**File**: `src/portal/clipboard.rs`

**Add**:
```rust
/// Selection transfer request from Portal
#[derive(Debug, Clone)]
pub struct SelectionTransferRequest {
    pub mime_type: String,
    pub serial: u32,
}

pub async fn start_selection_transfer_listener(
    &self,
    event_tx: mpsc::UnboundedSender<SelectionTransferRequest>
) -> Result<()> {
    let clipboard = Arc::clone(&self.clipboard);

    tokio::spawn(async move {
        let stream = clipboard.receive_selection_transfer().await;
        match stream {
            Ok(mut stream) => {
                while let Some((_, mime_type, serial)) = stream.next().await {
                    let req = SelectionTransferRequest { mime_type, serial };
                    if event_tx.send(req).is_err() {
                        break;
                    }
                }
            }
            Err(e) => error!("SelectionTransfer stream error: {}", e)
        }
    });

    Ok(())
}
```

---

### Step 3: Wire SelectionTransfer to RDP Request

**File**: `src/clipboard/manager.rs`

**Add field**:
```rust
/// Pending SelectionTransfer requests (serial → mime_type)
pending_portal_requests: Arc<RwLock<HashMap<u32, String>>>,
```

**In set_portal_clipboard()**:
```rust
pub async fn set_portal_clipboard(...) {
    // ... set portal and session ...

    // Start SelectionTransfer listener
    let (transfer_tx, mut transfer_rx) = mpsc::unbounded_channel();
    portal.start_selection_transfer_listener(transfer_tx).await?;

    // Spawn handler
    let pending = Arc::clone(&self.pending_portal_requests);
    let event_tx = self.event_tx.clone();

    tokio::spawn(async move {
        while let Some(req) = transfer_rx.recv().await {
            info!("SelectionTransfer: {} (serial {})", req.mime_type, req.serial);

            // Track this request
            pending.write().await.insert(req.serial, req.mime_type.clone());

            // Convert MIME → format_id
            let format_id = mime_to_format_id(&req.mime_type);

            // TODO: Send RDP data request
            // event_tx.send(ClipboardEvent::RequestFromRDP(format_id, req.serial))
        }
    });
}
```

---

### Step 4: Implement SelectionWrite Data Delivery

**File**: `src/portal/clipboard.rs`

**Replace old provide_data()**:
```rust
pub async fn write_selection_data(
    &self,
    session: &Session<'_, RemoteDesktop<'_>>,
    serial: u32,
    data: Vec<u8>
) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    let fd = self.clipboard.selection_write(session, serial).await
        .context("Failed to get write FD")?;

    let std_fd: std::os::fd::OwnedFd = fd.into();
    let std_file = std::fs::File::from(std_fd);
    let mut file = tokio::fs::File::from_std(std_file);

    match file.write_all(&data).await {
        Ok(()) => {
            file.flush().await?;
            drop(file);

            self.clipboard.selection_write_done(session, serial, true).await?;
            info!("✅ Wrote {} bytes (serial {})", data.len(), serial);
            Ok(())
        }
        Err(e) => {
            drop(file);
            let _ = self.clipboard.selection_write_done(session, serial, false).await;
            Err(e.into())
        }
    }
}
```

**File**: `src/clipboard/manager.rs`

**Replace handle_rdp_data_response()**:
```rust
async fn handle_rdp_data_response(
    data: Vec<u8>,
    ...,
    portal_clipboard: &Arc<RwLock<Option<Arc<...>>>>,
    portal_session: &Arc<RwLock<Option<Arc<...>>>>,
    pending_requests: &Arc<RwLock<HashMap<u32, String>>>,
) -> Result<()> {
    // Get Portal refs
    let portal_opt = portal_clipboard.read().await.clone();
    let session_opt = portal_session.read().await.clone();

    let (portal, session) = match (portal_opt, session_opt) {
        (Some(p), Some(s)) => (p, s),
        _ => {
            error!("Portal not available for data response");
            return Ok(());
        }
    };

    // Get pending serial
    let pending = pending_requests.read().await;
    let serial = pending.iter().next().map(|(k, _)| *k);
    drop(pending);

    let serial = match serial {
        Some(s) => s,
        None => {
            warn!("No pending Portal request for RDP data");
            return Ok(());
        }
    };

    // Convert UTF-16→UTF-8
    let utf16_data: Vec<u16> = data.chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&c| c != 0)
        .collect();

    let text = String::from_utf16(&utf16_data)
        .map_err(|e| ClipboardError::ConversionFailed(format!("UTF-16: {}", e)))?;

    let portal_data = text.into_bytes();

    // Write to Portal
    let session_guard = session.lock().await;
    portal.write_selection_data(&session_guard, serial, portal_data).await
        .map_err(|e| ClipboardError::PortalError(format!("Write: {}", e)))?;

    // Clear pending
    pending_requests.write().await.remove(&serial);

    info!("✅ Clipboard data delivered to Portal");
    Ok(())
}
```

**Success Criteria**: Paste in Linux shows Windows clipboard data

---

### Step 5: Implement SelectionOwnerChanged Listener

**File**: `src/portal/clipboard.rs`

**Add**:
```rust
pub async fn start_owner_changed_listener(
    &self,
    event_tx: mpsc::UnboundedSender<Vec<String>>
) -> Result<()> {
    let clipboard = Arc::clone(&self.clipboard);

    tokio::spawn(async move {
        let stream = clipboard.receive_selection_owner_changed().await;
        match stream {
            Ok(mut stream) => {
                while let Some((_, change)) = stream.next().await {
                    if !change.session_is_owner().unwrap_or(false) {
                        // Another app owns clipboard
                        let mime_types = change.mime_types();
                        if event_tx.send(mime_types).is_err() {
                            break;
                        }
                    }
                }
            }
            Err(e) => error!("SelectionOwnerChanged stream error: {}", e)
        }
    });

    Ok(())
}
```

**File**: `src/clipboard/manager.rs` or **ironrdp_backend.rs**

**Wire to FormatList sending**:
```rust
// When SelectionOwnerChanged received:
//   1. Convert MIME types → RDP formats
//   2. Send FormatList PDU to client
//      (need ServerEvent mechanism or message proxy)
```

**NOTE**: Sending FormatList requires access to send to client. Need to research if ServerEvent supports this or if we need message proxy access.

---

### Step 6: Implement SelectionRead for Linux→Windows

**File**: `src/clipboard/manager.rs`

**Complete handle_rdp_data_request()**:
```rust
async fn handle_rdp_data_request(
    format_id: u32,
    response_callback: Option<RdpResponseCallback>,
    ...,
    portal_clipboard: &Arc<RwLock<Option<Arc<...>>>>,
    portal_session: &Arc<RwLock<Option<Arc<...>>>>,
) -> Result<()> {
    let portal_opt = portal_clipboard.read().await.clone();
    let session_opt = portal_session.read().await.clone();

    let (portal, session) = match (portal_opt, session_opt) {
        (Some(p), Some(s)) => (p, s),
        _ => return Ok(()),
    };

    // Convert format_id → MIME
    let mime_type = converter.format_id_to_mime(format_id)?;

    // Read from Portal
    let session_guard = session.lock().await;
    let portal_data = portal.read_local_clipboard(&session_guard, &mime_type).await?;

    // Convert UTF-8→UTF-16
    let text = String::from_utf8(portal_data)
        .map_err(|e| ClipboardError::ConversionFailed(format!("UTF-8: {}", e)))?;

    let utf16: Vec<u16> = text.encode_utf16().collect();
    let mut rdp_data = Vec::with_capacity(utf16.len() * 2 + 2);
    for c in utf16 {
        rdp_data.extend_from_slice(&c.to_le_bytes());
    }
    rdp_data.extend_from_slice(&[0, 0]); // Null terminator

    // Send response
    if let Some(callback) = response_callback {
        callback(rdp_data);
    }

    Ok(())
}
```

**Success Criteria**: Windows can paste Linux clipboard data

---

## SYSTEMATIC EXECUTION PLAN

1. **Unstash incomplete work** (has some correct fixes)
2. **Review stashed changes** - keep correct parts, discard incomplete
3. **Apply Phase 1** - Fix parameter passing completely
4. **Test build**
5. **Apply Phase 2** - Remove wl-clipboard-rs
6. **Test build**
7. **Apply Phase 3** - Implement Windows→Linux flow
8. **Test functionality**
9. **Apply Phase 4** - Implement Linux→Windows flow
10. **Test functionality**
11. **Apply Phase 5-6** - Images and files
12. **Apply Phase 7** - Final cleanup
13. **Full integration test**

---

## SUCCESS CRITERIA

### Minimum (Text Clipboard):
- ✅ Windows→Linux text copy/paste working
- ✅ Linux→Windows text copy/paste working
- ✅ No crashes, no deadlocks
- ✅ Loop prevention functional
- ✅ Zero wl-clipboard-rs references

### Complete (All Features):
- ✅ Image clipboard both directions
- ✅ File transfer both directions
- ✅ Multiple formats supported
- ✅ Clean code, no TODOs
- ✅ All tests passing

---

**This plan provides systematic, complete implementation following official API documentation and architectural specifications.**
