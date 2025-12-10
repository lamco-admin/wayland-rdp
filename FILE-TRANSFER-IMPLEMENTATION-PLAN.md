# File Transfer Implementation Plan
## MS-RDPECLIP FileContents Protocol
## Date: 2025-12-10
## Estimated Time: 6-8 hours

---

## OVERVIEW

Implement bidirectional file copy/paste between Linux and Windows via RDP clipboard.

**Protocol:** MS-RDPECLIP FileContents (sections 2.2.5.2-2.2.5.4)
**Current Status:** PDUs exist in IronRDP, business logic needed in wrd-server
**Scope:** FileGroupDescriptorW + streaming + Portal integration

---

## ARCHITECTURE

### Data Flow: Linux → Windows

```text
User copies file in Linux
        ↓
Portal Clipboard SelectionOwnerChanged
        │ (with file:// URIs)
        ↓
wrd-server Clipboard Manager
        │
        ├─> Parse file:// URIs
        ├─> Extract file metadata (name, size, mtime)
        ├─> Build FileGroupDescriptorW structure
        │
        ▼
Send CB_FORMAT_LIST with CF_HDROP format
        ↓
Windows RDP client shows "files available"
        ↓
User pastes in Windows
        ↓
CB_FILECONTENTS_REQUEST received
        │ (stream_id, position, size)
        ├─> Read file chunk from Portal
        ├─> Build CB_FILECONTENTS_RESPONSE
        └─> Stream until complete
```

### Data Flow: Windows → Linux

```text
User copies file(s) in Windows
        ↓
CB_FORMAT_LIST received (includes CF_HDROP)
        │
        ├─> Contains FileGroupDescriptorW in format data
        ▼
wrd-server requests FileGroupDescriptorW
        ↓
Parse file descriptors (names, sizes, attributes)
        ↓
User pastes in Linux application
        ↓
Portal SelectionTransfer (file:// MIME requested)
        │
        ├─> For each file:
        │     ├─> Send CB_FILECONTENTS_REQUEST (range)
        │     ├─> Receive CB_FILECONTENTS_RESPONSE (data chunk)
        │     └─> Repeat until file complete
        │
        ├─> Write temporary files
        ├─> Return file:// URIs to Portal
        └─> Portal delivers to Linux app
```

---

## IMPLEMENTATION BREAKDOWN

### Module 1: File Descriptor Handling (2-3 hours)

**File:** `src/clipboard/file_descriptor.rs` (NEW)

**Structures:**

```rust
/// Windows FILETIME structure (100-nanosecond intervals since 1601-01-01)
#[repr(C)]
pub struct FileTime {
    pub low: u32,
    pub high: u32,
}

/// File descriptor attributes (MS-RDPECLIP 2.2.5.2.3)
#[repr(C)]
pub struct FileDescriptor {
    pub flags: u32,              // FD_ATTRIBUTES, FD_FILESIZE, etc.
    pub attributes: u32,          // FILE_ATTRIBUTE_*
    pub last_write_time: FileTime,
    pub file_size_high: u32,
    pub file_size_low: u32,
    pub file_name: [u16; 260],    // MAX_PATH (Unicode)
}

/// FileGroupDescriptorW wrapper
pub struct FileGroupDescriptor {
    pub count: u32,
    pub descriptors: Vec<FileDescriptor>,
}
```

**Functions:**

```rust
impl FileGroupDescriptor {
    /// Build from Portal file:// URIs
    pub fn from_uris(uris: &[String]) -> Result<Self> {
        // For each URI:
        // 1. Parse file path
        // 2. stat() for metadata
        // 3. Convert to Windows format
        // 4. Populate FileDescriptor
    }

    /// Serialize to bytes for CB_FORMAT_DATA
    pub fn to_bytes(&self) -> Vec<u8> {
        // Pack into binary format per MS-RDPECLIP spec
    }

    /// Parse from CB_FORMAT_DATA
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        // Unpack from Windows format
    }
}

/// Convert Unix timestamp to Windows FILETIME
fn unix_to_filetime(unix_time: i64) -> FileTime {
    const EPOCH_DIFF: u64 = 11644473600; // Seconds between 1601 and 1970
    let windows_time = (unix_time as u64 + EPOCH_DIFF) * 10_000_000;
    FileTime {
        low: (windows_time & 0xFFFFFFFF) as u32,
        high: (windows_time >> 32) as u32,
    }
}

/// Convert Windows filename to UTF-8
fn wide_string_to_utf8(wide: &[u16]) -> String {
    String::from_utf16_lossy(wide)
        .trim_end_matches('\0')
        .to_string()
}

/// Convert UTF-8 filename to Windows wide string
fn utf8_to_wide_string(s: &str) -> [u16; 260] {
    let mut wide = [0u16; 260];
    for (i, ch) in s.encode_utf16().take(259).enumerate() {
        wide[i] = ch;
    }
    wide
}
```

**Testing:**
- Unit test: Unix time → FILETIME conversion
- Unit test: Wide string ↔ UTF-8 conversion
- Unit test: Serialize → deserialize round-trip

### Module 2: File Content Streaming (3-4 hours)

**File:** `src/clipboard/file_streamer.rs` (NEW)

**Structures:**

```rust
/// File transfer session state
pub struct FileTransferSession {
    pub files: Vec<FileInfo>,
    pub current_file: Option<usize>,
    pub bytes_transferred: u64,
}

/// Individual file information
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub position: u64,  // Current read position
    pub stream_id: u32,
}

/// Request for file content chunk
pub struct FileContentsRequest {
    pub stream_id: u32,
    pub list_index: u32,
    pub position: u64,
    pub requested_size: u32,
    pub flags: u32,  // FILECONTENTS_SIZE or FILECONTENTS_RANGE
}
```

**Functions:**

```rust
impl FileTransferSession {
    /// Create new session from file URIs
    pub fn new(uris: &[String]) -> Result<Self> {
        // Parse URIs, validate files exist
    }

    /// Handle file contents request
    pub async fn handle_request(
        &mut self,
        request: FileContentsRequest,
    ) -> Result<Vec<u8>> {
        match request.flags {
            FILECONTENTS_SIZE => {
                // Return 8-byte size
                let file = &self.files[request.list_index as usize];
                Ok(file.size.to_le_bytes().to_vec())
            }
            FILECONTENTS_RANGE => {
                // Read and return file chunk
                let file = &mut self.files[request.list_index as usize];
                read_file_chunk(
                    &file.path,
                    request.position,
                    request.requested_size as usize,
                )
            }
            _ => Err(anyhow!("Unknown file contents flags: {}", request.flags)),
        }
    }
}

/// Read chunk from file
async fn read_file_chunk(
    path: &Path,
    position: u64,
    size: usize,
) -> Result<Vec<u8>> {
    use tokio::fs::File;
    use tokio::io::{AsyncReadExt, AsyncSeekExt};

    let mut file = File::open(path).await?;
    file.seek(std::io::SeekFrom::Start(position)).await?;

    let mut buffer = vec![0u8; size];
    let bytes_read = file.read(&mut buffer).await?;
    buffer.truncate(bytes_read);

    Ok(buffer)
}
```

**Error Handling:**
- File not found
- Permission denied
- File size changed during transfer
- Timeout on large files (implement progress tracking)

### Module 3: IronRDP Backend Integration (1-2 hours)

**File:** `src/clipboard/ironrdp_backend.rs`

**Current Stubs (lines 149-157):**
```rust
fn on_file_contents_request(&mut self, req: CliprdrBackendEvent) -> CliprdrBackendCommand {
    // TODO: Implement file transfer
    CliprdrBackendCommand::None
}

fn on_file_contents_response(&mut self, resp: CliprdrBackendEvent) -> CliprdrBackendCommand {
    // TODO: Implement file transfer
    CliprdrBackendCommand::None
}
```

**Implementation:**

```rust
fn on_file_contents_request(&mut self, req: CliprdrBackendEvent) -> CliprdrBackendCommand {
    // Extract FileContentsRequest from event
    let request = match req {
        CliprdrBackendEvent::FileContentsRequest(r) => r,
        _ => return CliprdrBackendCommand::None,
    };

    info!("📂 FileContents request: stream={}, list_index={}, pos={}, size={}",
          request.stream_id, request.list_index, request.position, request.cb_requested);

    // Spawn async task to read file chunk
    let manager = Arc::clone(&self.manager);
    let stream_id = request.stream_id;
    let list_index = request.list_index;
    let position = request.position;
    let size = request.cb_requested;

    tokio::spawn(async move {
        match manager.lock().await.handle_file_contents_request(
            stream_id, list_index, position, size
        ).await {
            Ok(data) => {
                // Send FileContentsResponse with data
                // manager.send_file_contents_response(stream_id, data)
            }
            Err(e) => {
                error!("Failed to read file contents: {}", e);
                // Send error response
            }
        }
    });

    CliprdrBackendCommand::None // Async operation, response sent later
}
```

### Module 4: Clipboard Manager Integration (1-2 hours)

**File:** `src/clipboard/manager.rs`

**Add Methods:**

```rust
impl ClipboardManager {
    /// Start file transfer session (Linux → Windows)
    pub async fn start_file_transfer(&mut self, uris: Vec<String>) -> Result<()> {
        use crate::clipboard::file_descriptor::FileGroupDescriptor;
        use crate::clipboard::file_streamer::FileTransferSession;

        info!("📂 Starting file transfer: {} files", uris.len());

        // Build file descriptor
        let descriptor = FileGroupDescriptor::from_uris(&uris)?;

        // Store transfer session
        self.file_transfer_session = Some(FileTransferSession::new(&uris)?);

        // Send format list with CF_HDROP
        let formats = vec![
            ClipboardFormat {
                id: 49161, // CF_HDROP
                name: String::new(),
            },
            ClipboardFormat {
                id: 49267, // FileGroupDescriptorW
                name: "FileGroupDescriptorW".to_string(),
            },
        ];

        self.send_format_list(formats).await?;
        Ok(())
    }

    /// Handle file contents request from Windows
    pub async fn handle_file_contents_request(
        &mut self,
        stream_id: u32,
        list_index: u32,
        position: u64,
        size: u32,
    ) -> Result<Vec<u8>> {
        let session = self.file_transfer_session
            .as_mut()
            .ok_or_else(|| anyhow!("No active file transfer session"))?;

        session.handle_request(FileContentsRequest {
            stream_id,
            list_index,
            position,
            requested_size: size,
            flags: FILECONTENTS_RANGE,
        }).await
    }

    /// Receive file from Windows (Windows → Linux)
    pub async fn receive_file_transfer(
        &mut self,
        descriptor: FileGroupDescriptor,
    ) -> Result<Vec<String>> {
        // For each file in descriptor:
        // 1. Create temp file
        // 2. Request chunks via CB_FILECONTENTS_REQUEST
        // 3. Write chunks to temp file
        // 4. Return file:// URIs to Portal

        let mut uris = Vec::new();

        for (index, file_desc) in descriptor.descriptors.iter().enumerate() {
            let filename = wide_string_to_utf8(&file_desc.file_name);
            let size = ((file_desc.file_size_high as u64) << 32) | (file_desc.file_size_low as u64);

            info!("📂 Receiving file: {} ({} bytes)", filename, size);

            // Create temp file
            let temp_path = self.create_temp_file(&filename)?;

            // Request file contents in chunks (64KB at a time)
            const CHUNK_SIZE: u32 = 65536;
            let mut position = 0u64;

            while position < size {
                let chunk_size = std::cmp::min(CHUNK_SIZE, (size - position) as u32);

                // Send FileContentsRequest
                let data = self.request_file_chunk(index as u32, position, chunk_size).await?;

                // Write to temp file
                self.write_file_chunk(&temp_path, &data).await?;

                position += data.len() as u64;
            }

            uris.push(format!("file://{}", temp_path.display()));
        }

        Ok(uris)
    }
}
```

---

## DETAILED IMPLEMENTATION STEPS

### Step 1: File Descriptor Module (2 hours)

**Create:** `src/clipboard/file_descriptor.rs`

**Tasks:**
- [ ] Define FileTime, FileDescriptor, FileGroupDescriptor structs
- [ ] Implement from_uris() - stat files and build descriptors
- [ ] Implement to_bytes() - serialize per MS-RDPECLIP spec
- [ ] Implement from_bytes() - parse Windows format
- [ ] Unix → FILETIME conversion
- [ ] UTF-8 ↔ UTF-16 filename conversion
- [ ] Unit tests for all conversions

**Spec Reference:**
- MS-RDPECLIP 2.2.5.2: CLIPRDR_FILELIST
- MS-RDPECLIP 2.2.5.2.3: FILEDESCRIPTOR structure
- Size: u32 count + (FILEDESCRIPTOR[count])
- FILEDESCRIPTOR: 592 bytes fixed size

**Validation:**
- Filename max 260 characters (Windows MAX_PATH)
- Handle Unicode properly (UTF-16LE)
- File size split into high/low u32
- Attributes bitmask (readonly, hidden, etc.)

### Step 2: File Streaming Module (3 hours)

**Create:** `src/clipboard/file_streamer.rs`

**Tasks:**
- [ ] Define FileTransferSession, FileInfo, FileContentsRequest
- [ ] Implement async file chunk reading
- [ ] Handle FILECONTENTS_SIZE requests (return 8-byte size)
- [ ] Handle FILECONTENTS_RANGE requests (return chunk)
- [ ] Implement progress tracking
- [ ] Add timeout handling (large files)
- [ ] Error recovery (file changed during transfer)
- [ ] Temp file management for Windows→Linux

**Constants:**
```rust
const FILECONTENTS_SIZE: u32 = 0x00000001;
const FILECONTENTS_RANGE: u32 = 0x00000002;
const FILE_CHUNK_SIZE: u32 = 65536; // 64KB chunks
const FILE_TRANSFER_TIMEOUT: Duration = Duration::from_secs(30);
```

**Edge Cases:**
- File deleted mid-transfer
- File modified mid-transfer (size changed)
- Disk space exhaustion
- Permission errors
- Path traversal attacks (sanitize filenames!)

**Security:**
```rust
fn sanitize_filename(name: &str) -> String {
    // Strip path separators
    name.replace('/', "_")
        .replace('\\', "_")
        .replace("..", "_")
}
```

### Step 3: IronRDP Backend Wiring (1 hour)

**Modify:** `src/clipboard/ironrdp_backend.rs`

**Tasks:**
- [ ] Implement on_file_contents_request() handler
- [ ] Implement on_file_contents_response() handler
- [ ] Add FileTransferSession to IronRdpClipboard struct
- [ ] Wire async file operations
- [ ] Add logging for file transfer flow

**Integration Points:**

```rust
// In IronRdpClipboard struct:
file_transfer_session: Option<Arc<Mutex<FileTransferSession>>>,

// In on_file_contents_request:
if let Some(ref session) = self.file_transfer_session {
    // Handle request via session
} else {
    warn!("FileContents request but no active transfer session");
    return CliprdrBackendCommand::None;
}
```

### Step 4: Portal Integration (1-2 hours)

**Modify:** `src/clipboard/manager.rs`

**Tasks:**
- [ ] Detect file:// URIs in SelectionOwnerChanged
- [ ] Extract file metadata for FileGroupDescriptor
- [ ] Handle file MIME types (application/x-file, text/uri-list)
- [ ] Request file data via SelectionTransfer
- [ ] Write temp files for Windows→Linux
- [ ] Clean up temp files after transfer

**Portal File MIME Types:**
```
text/uri-list         - List of file:// URIs (newline separated)
application/x-file    - Binary file data (some systems)
```

**Temp File Strategy:**
```rust
// Create in /tmp/wrd-server-clipboard/
let temp_dir = std::env::temp_dir().join("wrd-server-clipboard");
std::fs::create_dir_all(&temp_dir)?;

// Use random UUID for uniqueness
let temp_file = temp_dir.join(format!("{}-{}", uuid::Uuid::new_v4(), sanitized_filename));
```

**Cleanup:**
```rust
// On transfer complete or timeout:
tokio::spawn(async move {
    tokio::time::sleep(Duration::from_secs(300)).await; // 5 min retention
    let _ = tokio::fs::remove_file(temp_file).await;
});
```

---

## TESTING PLAN

### Unit Tests

**file_descriptor.rs:**
- [ ] Unix time → FILETIME conversion accuracy
- [ ] UTF-8 → UTF-16 → UTF-8 round-trip
- [ ] FileDescriptor serialization/deserialization
- [ ] Multiple files in FileGroupDescriptor
- [ ] Edge cases: very long filenames, special characters

**file_streamer.rs:**
- [ ] Chunk reading at various offsets
- [ ] Size request handling
- [ ] Range request handling
- [ ] File not found errors
- [ ] Partial read handling

### Integration Tests

**Linux → Windows:**
1. Copy single text file in Linux
2. Paste in Windows Notepad
3. Verify content matches
4. Test with binary file (image)
5. Test with multiple files

**Windows → Linux:**
1. Copy file(s) in Windows Explorer
2. Paste in Linux file manager
3. Verify files created
4. Check content integrity
5. Test large files (100MB+)

**Edge Cases:**
- Empty file (0 bytes)
- Very large file (1GB+)
- File with Unicode name
- File with spaces in name
- Multiple files at once
- File deleted mid-transfer

---

## MS-RDPECLIP SPECIFICATION DETAILS

### Format IDs

```rust
const CF_HDROP: u32 = 15; // File drag-drop format (predefined)
const CF_FILEGROUP_DESCRIPTOR_W: u32 = 49267; // FileGroupDescriptorW (registered)
```

### PDU Flow: Linux → Windows

```
1. CB_FORMAT_LIST
   Format 1: CF_HDROP (id=15, name="")
   Format 2: FileGroupDescriptorW (id=49267, name="FileGroupDescriptorW")

2. CB_FORMAT_LIST_RESPONSE (ACK)

3. CB_FORMAT_DATA_REQUEST (format_id=49267)

4. CB_FORMAT_DATA_RESPONSE
   Data: FileGroupDescriptorW binary structure

5. CB_FILECONTENTS_REQUEST (stream_id, list_index=0, position=0, size=?)
   Flags: FILECONTENTS_SIZE

6. CB_FILECONTENTS_RESPONSE
   Data: 8 bytes (u64 file size)

7. CB_FILECONTENTS_REQUEST (stream_id, list_index=0, position=0, size=65536)
   Flags: FILECONTENTS_RANGE

8. CB_FILECONTENTS_RESPONSE
   Data: First 64KB of file

9. Repeat step 7-8 with increasing position until file complete
```

### PDU Flow: Windows → Linux

```
1. CB_FORMAT_LIST (from Windows)
   Includes: CF_HDROP, FileGroupDescriptorW, FileContents...

2. CB_FORMAT_LIST_RESPONSE (ACK)

3. CB_FORMAT_DATA_REQUEST (format_id=FileGroupDescriptorW)

4. CB_FORMAT_DATA_RESPONSE (from Windows)
   Data: FileGroupDescriptorW with file list

5. We parse descriptor, user pastes in Linux

6. CB_FILECONTENTS_REQUEST (we send, stream_id, list_index, range)

7. CB_FILECONTENTS_RESPONSE (Windows sends chunk)

8. Repeat 6-7 until complete

9. Write to temp file, return file:// URI to Portal
```

---

## CURRENT IRONRDP SUPPORT

### What IronRDP Provides ✅

From `ironrdp-cliprdr` crate:

```rust
// PDU structures exist:
pub struct FileContentsRequest { ... }
pub struct FileContentsResponse { ... }
pub enum CliprdrBackendEvent::FileContentsRequest(...)
pub enum CliprdrBackendCommand::FileContentsResponse(...)
```

**IronRDP handles:**
- PDU encoding/decoding
- Network framing
- Clipboard channel routing

### What We Must Implement ❌

**Business logic:**
- FileGroupDescriptorW binary format (IronRDP just has raw bytes)
- File metadata extraction
- Chunk streaming state machine
- Temp file management
- Portal file:// URI handling

**Estimate:** IronRDP is ~30% of file transfer, we do the other 70%

---

## DEPENDENCIES

### Cargo.toml Additions

```toml
[dependencies]
uuid = { version = "1.6", features = ["v4"] }  # Temp file names
```

All other dependencies already present:
- tokio (async file I/O)
- ashpd (Portal file URIs)
- ironrdp-cliprdr (PDU structures)

---

## RISKS AND MITIGATION

### Risk 1: Large File Transfers

**Problem:** 1GB file = 15,625 chunks @ 64KB each
**Mitigation:**
- Progress tracking and logging
- Timeout on per-chunk basis (not total transfer)
- Allow cancellation (user feedback)

### Risk 2: Path Traversal Attacks

**Problem:** Windows sends "../../../etc/passwd" as filename
**Mitigation:**
- Sanitize all filenames (remove .., /, \)
- Use UUIDs in temp file names
- Restrict to temp directory only

### Risk 3: Disk Space Exhaustion

**Problem:** Multiple large file transfers fill disk
**Mitigation:**
- Check available disk space before starting
- Limit temp directory size (configurable)
- Auto-cleanup after timeout

### Risk 4: Concurrent Transfers

**Problem:** User starts second transfer before first completes
**Mitigation:**
- Cancel previous transfer
- OR queue transfers (single active at a time)
- Clear session state on new transfer

---

## SUCCESS CRITERIA

### Functional Requirements ✅
- [ ] Copy single file Linux → Windows
- [ ] Copy multiple files Linux → Windows
- [ ] Copy file Windows → Linux
- [ ] Files integrity verified (checksum match)
- [ ] Unicode filenames work correctly
- [ ] Large files (100MB+) transfer successfully

### Performance Requirements ✅
- [ ] Chunk size optimized (64KB = good balance)
- [ ] No blocking on file I/O (async tokio::fs)
- [ ] Timeout prevents hangs
- [ ] Progress visible in logs

### Security Requirements ✅
- [ ] No path traversal possible
- [ ] Filenames sanitized
- [ ] Temp files in restricted directory
- [ ] Auto-cleanup after timeout

---

## IMPLEMENTATION ORDER

This session (6-8 hours):
1. ✅ file_descriptor.rs (2 hours)
2. ✅ file_streamer.rs (3 hours)
3. ✅ ironrdp_backend.rs integration (1 hour)
4. ✅ manager.rs Portal integration (1-2 hours)
5. ✅ Testing and debugging (1 hour)

Next session:
- Comprehensive testing (multiple scenarios)
- Edge case handling refinement
- Performance optimization
- Documentation

---

## END OF PLAN
Ready to implement after periodic refresh testing!
