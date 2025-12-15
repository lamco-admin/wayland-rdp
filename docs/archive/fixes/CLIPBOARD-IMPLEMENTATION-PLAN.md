# Clipboard Implementation Plan - Complete File Transfer & Copy/Paste

**Status:** Format conversions ✅ DONE, Backend integration ❌ NEEDS IMPLEMENTATION
**Priority:** HIGH - Last major feature for v1.0
**Effort:** 3-5 days

---

## Current State

### ✅ What Exists (Complete)

1. **Format Conversion Functions** (`src/clipboard/formats.rs`)
   - ✅ `convert_unicode_text_to_utf8()` / reverse
   - ✅ `convert_uri_list_to_hdrop()` / reverse
   - ✅ `convert_dib_to_png()` / reverse
   - ✅ Image conversions for JPEG, BMP
   - ✅ Format mapping tables
   - ✅ All 936 lines implemented and tested!

2. **Clipboard Manager** (`src/clipboard/manager.rs`)
   - ✅ ClipboardManager struct
   - ✅ Loop detection logic
   - ✅ State machine
   - ✅ Event handling

3. **Backend Factory** (`src/clipboard/ironrdp_backend.rs`)
   - ✅ WrdCliprdrFactory
   - ✅ WrdCliprdrBackend structure
   - ✅ Trait implementations (skeleton)

4. **Portal Integration** (`src/portal/clipboard.rs`)
   - ✅ Portal clipboard module
   - ✅ Read/write methods

### ❌ What's Missing (Stubs)

The backend methods in `ironrdp_backend.rs` are stubs that just log:
- `on_format_data_request()` - Line 146
- `on_format_data_response()` - Line 157
- `on_file_contents_request()` - Line 167
- `on_file_contents_response()` - Line 174

These need REAL implementations to make clipboard work!

---

## Implementation Tasks

### Task 1: Implement on_remote_copy() - Client Copies Data

**File:** `src/clipboard/ironrdp_backend.rs` line 128

**Purpose:** When RDP client copies something (Ctrl+C), announce it to Portal

**Current:**
```rust
fn on_remote_copy(&mut self, available_formats: &[ClipboardFormat]) {
    info!("Remote copy announced with {} formats", available_formats.len());
    // Just logs, does nothing
}
```

**Implement:**
```rust
fn on_remote_copy(&mut self, available_formats: &[ClipboardFormat]) {
    info!("Remote copy announced with {} formats", available_formats.len());

    // Log formats for debugging
    for (idx, format) in available_formats.iter().enumerate() {
        debug!("  Format {}: ID={}, Name={:?}", idx, format.format_id, format.format_name);
    }

    // Spawn async task to handle Portal clipboard update
    let clipboard_manager = Arc::clone(&self.clipboard_manager);
    let formats = available_formats.to_vec();

    tokio::spawn(async move {
        let mut manager = clipboard_manager.lock().await;

        // Convert RDP formats to MIME types
        match manager.handle_remote_copy(formats).await {
            Ok(_) => debug!("Remote clipboard announced to Portal"),
            Err(e) => error!("Failed to announce remote clipboard: {}", e),
        }
    });
}
```

### Task 2: Implement on_format_data_request() - Portal Requests Data

**File:** `src/clipboard/ironrdp_backend.rs` line 146

**Purpose:** When user pastes in Linux, Portal asks for the actual clipboard data

**Implement:**
```rust
fn on_format_data_request(&mut self, request: FormatDataRequest) {
    debug!("Format data requested: format_id={}", request.format_id());

    let clipboard_manager = Arc::clone(&self.clipboard_manager);
    let format_id = request.format_id();

    tokio::spawn(async move {
        let manager = clipboard_manager.lock().await;

        // Fetch data from RDP client and convert to Portal format
        match manager.handle_format_data_request(format_id).await {
            Ok(data) => {
                debug!("Sending {} bytes for format {}", data.len(), format_id);
                // Send FormatDataResponse via CliprdrServer message proxy
                // (Would need access to message sender here)
            }
            Err(e) => error!("Failed to get format data: {}", e),
        }
    });
}
```

**Note:** This needs access to the IronRDP message sender to send responses. The backend would need to store an `Arc<Mutex<MessageProxy>>` or similar.

### Task 3: Implement on_format_data_response() - Client Sends Data

**File:** `src/clipboard/ironrdp_backend.rs` line 157

**Purpose:** RDP client provides clipboard data that was requested

**Implement:**
```rust
fn on_format_data_response(&mut self, response: FormatDataResponse<'_>) {
    debug!("Format data response: {} bytes, format={}",
        response.data().len(),
        response.msg_flags());

    let clipboard_manager = Arc::clone(&self.clipboard_manager);
    let data = response.data().to_vec();

    tokio::spawn(async move {
        let mut manager = clipboard_manager.lock().await;

        // Convert from RDP format and set to Portal clipboard
        match manager.handle_format_data_response(data).await {
            Ok(_) => debug!("Clipboard data set to Portal"),
            Err(e) => error!("Failed to set clipboard data: {}", e),
        }
    });
}
```

### Task 4: Implement on_file_contents_request() - Client Wants File Data

**File:** `src/clipboard/ironrdp_backend.rs` line 167

**Purpose:** Client is pasting files and needs actual file contents

**Implement:**
```rust
fn on_file_contents_request(&mut self, request: FileContentsRequest) {
    debug!("File contents requested: stream_id={}, position={}, size={}",
        request.stream_id(),
        request.position(),
        request.requested_size());

    let clipboard_manager = Arc::clone(&self.clipboard_manager);
    let stream_id = request.stream_id();
    let position = request.position();
    let size = request.requested_size();

    tokio::spawn(async move {
        let manager = clipboard_manager.lock().await;

        // Read file contents from local filesystem
        match manager.handle_file_contents_request(stream_id, position, size).await {
            Ok(data) => {
                debug!("Sending {} bytes of file data", data.len());
                // Send FileContentsResponse via CliprdrServer
            }
            Err(e) => error!("Failed to read file contents: {}", e),
        }
    });
}
```

### Task 5: Implement on_file_contents_response() - Client Sends File Data

**File:** `src/clipboard/ironrdp_backend.rs` line 174

**Purpose:** Client sending file data that was requested

**Implement:**
```rust
fn on_file_contents_response(&mut self, response: FileContentsResponse<'_>) {
    debug!("File contents response: stream_id={}, {} bytes",
        response.stream_id(),
        response.data().len());

    let clipboard_manager = Arc::clone(&self.clipboard_manager);
    let stream_id = response.stream_id();
    let data = response.data().to_vec();

    tokio::spawn(async move {
        let mut manager = clipboard_manager.lock().await;

        // Write file contents to temporary storage
        match manager.handle_file_contents_response(stream_id, data).await {
            Ok(path) => debug!("File written to: {:?}", path),
            Err(e) => error!("Failed to write file: {}", e),
        }
    });
}
```

---

## ClipboardManager Methods Needed

The backend calls methods on ClipboardManager that need to be implemented:

**File:** `src/clipboard/manager.rs`

### Method 1: handle_remote_copy()

```rust
pub async fn handle_remote_copy(&mut self, formats: Vec<ClipboardFormat>) -> Result<()> {
    // 1. Store available formats
    self.available_formats = formats.clone();

    // 2. Check loop detection
    if self.loop_detector.should_ignore_update() {
        debug!("Ignoring remote copy (loop prevention)");
        return Ok(());
    }

    // 3. Convert RDP formats to MIME types
    let mime_types = self.converter.rdp_to_mime_types(&formats)?;

    // 4. Announce to Portal clipboard
    self.portal_clipboard.set_available_formats(&mime_types).await?;

    // 5. Update state
    self.state = ClipboardState::RdpOwned;
    self.loop_detector.record_operation(SyncDirection::RdpToPortal);

    Ok(())
}
```

### Method 2: handle_format_data_request()

```rust
pub async fn handle_format_data_request(&self, format_id: u32) -> Result<Vec<u8>> {
    // 1. Read data from Portal clipboard
    let mime_type = self.converter.format_id_to_mime(format_id)?;
    let portal_data = self.portal_clipboard.read_data(&mime_type).await?;

    // 2. Convert from Portal format to RDP format
    let rdp_data = self.converter.convert_mime_to_rdp(&mime_type, format_id, &portal_data).await?;

    Ok(rdp_data)
}
```

### Method 3: handle_format_data_response()

```rust
pub async fn handle_format_data_response(&mut self, data: Vec<u8>) -> Result<()> {
    // 1. Determine format from current request context
    let format_id = self.current_format_request.unwrap_or(CF_UNICODETEXT);

    // 2. Convert RDP data to Portal format
    let mime_type = self.converter.format_id_to_mime(format_id)?;
    let portal_data = self.converter.convert_rdp_to_mime(format_id, &mime_type, &data).await?;

    // 3. Set to Portal clipboard
    self.portal_clipboard.write_data(&mime_type, &portal_data).await?;

    // 4. Update state
    self.state = ClipboardState::PortalOwned;
    self.loop_detector.record_operation(SyncDirection::PortalToRdp);

    Ok(())
}
```

### Method 4: handle_file_contents_request()

```rust
pub async fn handle_file_contents_request(
    &self,
    stream_id: u32,
    position: u64,
    size: u32,
) -> Result<Vec<u8>> {
    // 1. Get file path from stream_id
    let file_path = self.file_streams.get(&stream_id)
        .ok_or_else(|| ClipboardError::InvalidStreamId(stream_id))?;

    // 2. Open and read file chunk
    let mut file = tokio::fs::File::open(file_path).await?;
    file.seek(SeekFrom::Start(position)).await?;

    let mut buffer = vec![0u8; size as usize];
    file.read_exact(&mut buffer).await?;

    Ok(buffer)
}
```

### Method 5: handle_file_contents_response()

```rust
pub async fn handle_file_contents_response(
    &mut self,
    stream_id: u32,
    data: Vec<u8>,
) -> Result<PathBuf> {
    // 1. Get or create temp file for this stream
    let temp_path = self.get_temp_file_path(stream_id)?;

    // 2. Append data to file
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&temp_path)
        .await?;

    file.write_all(&data).await?;

    // 3. Check if transfer complete
    if self.is_transfer_complete(stream_id) {
        debug!("File transfer complete: {:?}", temp_path);
        // Add to URI list for Portal
        self.completed_files.push(format!("file://{}", temp_path.display()));
    }

    Ok(temp_path)
}
```

---

## Architecture Issue - Message Proxy Access

The backend methods are **callbacks** - they can't directly send messages back to IronRDP. They need access to a message sender/proxy.

### Solution: Store Message Proxy in Backend

**Modify WrdCliprdrBackend:**

```rust
struct WrdCliprdrBackend {
    clipboard_manager: Arc<Mutex<ClipboardManager>>,
    capabilities: ClipboardGeneralCapabilityFlags,
    temporary_directory: String,
    /// Message proxy for sending responses (set after backend creation)
    message_proxy: Arc<Mutex<Option<CliprdrMessageProxy>>>,
}
```

The IronRDP library likely provides a way to get the message proxy after backend creation. Need to check IronRDP source for `CliprdrServer` or `CliprdrBackend` documentation.

---

## Simplified Implementation Strategy

Given complexity, consider **simplified approach for MVP:**

### MVP Clipboard (Text Only) - 1 day

1. **Text Windows → Linux**
   - Implement `on_format_data_response()` for CF_UNICODETEXT
   - Convert UTF-16 → UTF-8
   - Write to Portal clipboard

2. **Text Linux → Windows**
   - Implement `on_format_data_request()` for CF_UNICODETEXT
   - Read from Portal clipboard
   - Convert UTF-8 → UTF-16
   - Return data

**Test:** Copy/paste text both directions

### Full Clipboard (All Formats) - 3-4 days

3. **Add Image Support**
   - Use existing DIB/PNG conversions
   - Test screenshots

4. **Add File Transfer**
   - Implement FileContents handlers
   - Use existing HDROP/URI-list conversions
   - Test file copy/paste

---

## Key Files Reference

### Already Implemented (Use These!)

1. **src/clipboard/formats.rs** (936 lines)
   - Complete FormatConverter with all conversions
   - Text, image, file format functions
   - Well-tested

2. **src/clipboard/manager.rs** (16,760 lines)
   - ClipboardManager with state machine
   - Loop detection
   - Event handling

3. **src/clipboard/sync.rs** (21,300 lines)
   - Loop prevention logic
   - State machine implementation

4. **src/clipboard/transfer.rs** (17,690 lines)
   - File transfer engine
   - Progress tracking
   - Chunked transfer support

### Needs Implementation

5. **src/clipboard/ironrdp_backend.rs** (209 lines)
   - Methods 146, 157, 167, 174 are stubs
   - Need to call ClipboardManager methods
   - Need message proxy integration

---

## Testing Plan

### Text Clipboard (15 minutes)
1. Start server with clipboard enabled
2. Windows: Copy text (Ctrl+C)
3. Linux: Paste (Ctrl+V) - should appear
4. Linux: Copy text
5. Windows: Paste - should appear

### Image Clipboard (15 minutes)
1. Windows: Take screenshot (Win+Shift+S)
2. Windows: Copy to clipboard
3. Linux: Paste into image editor
4. Verify image appears correctly
5. Linux: Copy image
6. Windows: Paste - should work

### File Transfer (30 minutes)
1. Windows: Copy a file (Ctrl+C in Explorer)
2. Linux: Paste (Ctrl+V in file manager)
3. Verify file appears with correct name/size
4. Linux: Copy multiple files
5. Windows: Paste - should copy all files
6. Test large file (> 10 MB)
7. Test special characters in filenames

---

## Quick Win: Text-Only Clipboard

If time is limited, implement TEXT ONLY first:

**Changes needed:**
1. Modify `on_format_data_response()` - 10 lines of code
2. Modify `on_format_data_request()` - 10 lines of code
3. Wire up to Portal - 5 lines

**Total:** ~25 lines of actual code
**Benefit:** Copy/paste text working immediately
**Time:** 1-2 hours

Then add images and files incrementally.

---

## Message Proxy Challenge

The biggest blocker is getting access to send responses back to IronRDP from the callback methods.

### Research Needed

Check IronRDP source:
- `/tmp/IronRDP-fork-check/crates/ironrdp-cliprdr/`
- Look for CliprdrServer, CliprdrBackend examples
- Find how to send FormatDataResponse
- Find how to send FileContentsResponse

### Possible Solutions

1. **Store sender in backend** (likely correct)
2. **Use global/shared sender** (less elegant)
3. **Return responses from methods** (check if trait allows)

---

## Recommendation

**For next session:**

1. **Start with text clipboard** (highest value, lowest effort)
2. **Research IronRDP message sending** (check examples)
3. **Implement incrementally** (text, then images, then files)
4. **Test each step** (verify before moving on)

**Estimated timeline:**
- Text clipboard: 2-4 hours
- Image clipboard: 1 day
- File transfer: 2-3 days
- **Total: 3-5 days for complete clipboard**

---

## Success Criteria

When complete, users should be able to:

✅ Copy/paste text Windows ↔ Linux
✅ Copy/paste images Windows ↔ Linux
✅ Copy/paste files Windows ↔ Linux
✅ Drag/drop files into RDP window
✅ Multiple files at once
✅ Large files (chunked transfer)
✅ Special characters in filenames
✅ No clipboard loops
✅ Graceful error handling

---

**This will make the RDP server feature-complete for v1.0!**
