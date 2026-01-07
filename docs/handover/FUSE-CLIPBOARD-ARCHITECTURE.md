# FUSE-Based Clipboard File Transfer Architecture

## Problem Statement

The current clipboard file transfer implementation stages files to `~/Downloads` and provides `file://` URIs to the file manager. This approach has fundamental flaws:

1. **Wrong destination**: Files appear in Downloads, not the paste location
2. **Duplicates**: File manager copies from staging location, creating duplicates
3. **Disk usage**: Large files consume disk space before user even pastes
4. **Cleanup complexity**: Staged files need manual or timed cleanup

## Industry Standard: FUSE Virtual Filesystem

FreeRDP, RustDesk, and gnome-remote-desktop all use FUSE (Filesystem in Userspace) for clipboard file transfer:

- File manager sees real filesystem paths
- File contents are fetched **on-demand** when read() is called
- No pre-staging required
- Files appear at the actual paste destination

### References
- [FreeRDP FUSE clipboard issue #6727](https://github.com/FreeRDP/FreeRDP/issues/6727)
- [gnome-remote-desktop source](https://github.com/GNOME/gnome-remote-desktop)
- g-r-d uses `grd-rdp-fuse-clipboard.c` for FUSE-based clipboard

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           Windows RDP Client                             │
│  Copy file(s) → Clipboard → FileGroupDescriptorW + FileContents data    │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ RDP Protocol
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         wrd-server (Rust)                                │
│                                                                          │
│  ┌──────────────────┐     ┌──────────────────┐     ┌────────────────┐   │
│  │ ClipboardManager │────▶│ FuseClipboardFs  │────▶│ FUSE Mount     │   │
│  │                  │     │ (fuser crate)    │     │ /tmp/wrd-fuse/ │   │
│  └──────────────────┘     └──────────────────┘     └────────────────┘   │
│          │                        │                        │            │
│          │                        │                        │            │
│          ▼                        ▼                        ▼            │
│  ┌──────────────────┐     ┌──────────────────┐     ┌────────────────┐   │
│  │ Portal Clipboard │     │ RDP Request      │     │ Virtual Files  │   │
│  │ (SetSelection)   │     │ Channel          │     │ (on-demand)    │   │
│  └──────────────────┘     └──────────────────┘     └────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ XDG Portal
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Linux File Manager                                │
│                                                                          │
│  Paste → Read URIs → Read file from FUSE → Copy to destination          │
└─────────────────────────────────────────────────────────────────────────┘
```

## Data Flow: Windows → Linux File Paste

### Step 1: Windows Copies File(s)
```
Windows: User copies files
→ RDP Client: Sends FormatList PDU with FileGroupDescriptorW format
→ Server: ClipboardManager receives RdpFormatList event
```

### Step 2: Server Announces to Portal
```
Server: Announce x-special/gnome-copied-files + text/uri-list to Portal
→ Portal: SetSelection with MIME types
→ Linux: Clipboard indicator shows "content available"
```

### Step 3: User Pastes in File Manager
```
Linux: User pastes (Ctrl+V in Nautilus)
→ Portal: SelectionTransfer signal (requests gnome-copied-files)
→ Server: Request FileGroupDescriptorW from RDP client
→ RDP: FormatDataResponse with file descriptors
```

### Step 4: Create Virtual Files in FUSE
```
Server: Parse file descriptors (names, sizes)
Server: Create virtual file entries in FUSE filesystem
Server: Return URIs to Portal: "copy\nfile:///tmp/wrd-fuse/filename.txt\0"
→ Portal: Delivers URIs to file manager
```

### Step 5: File Manager Reads (On-Demand)
```
Nautilus: Opens file:///tmp/wrd-fuse/filename.txt
→ FUSE: read(inode, offset, size) called
→ Server: Send FileContentsRequest to RDP client
→ RDP: FileContentsResponse with data chunk
→ FUSE: Returns data to read() caller
Nautilus: Copies data to actual paste destination (e.g., Desktop)
```

## Key Components

### 1. FuseClipboardFs (src/clipboard/fuse.rs)

Implements `fuser::Filesystem` trait:

```rust
struct FuseClipboardFs {
    /// Virtual files indexed by inode
    files: Arc<RwLock<HashMap<u64, VirtualFile>>>,
    /// Next available inode
    next_inode: AtomicU64,
    /// Channel to send FileContentsRequest
    request_tx: mpsc::Sender<FileRequest>,
    /// Channel to receive FileContentsResponse
    response_rx: Mutex<mpsc::Receiver<FileResponse>>,
    /// Mount point
    mount_point: PathBuf,
}

struct VirtualFile {
    inode: u64,
    filename: String,
    size: u64,
    file_index: u32,  // Index in FileGroupDescriptorW list
    clip_data_id: u32, // For locking
}

impl Filesystem for FuseClipboardFs {
    fn lookup(&mut self, ...) { /* Find file by name */ }
    fn getattr(&mut self, ...) { /* Return file attributes */ }
    fn readdir(&mut self, ...) { /* List virtual files */ }
    fn open(&mut self, ...) { /* Open virtual file */ }
    fn read(&mut self, ...) { /* Fetch data from RDP on-demand */ }
}
```

### 2. FuseManager (src/clipboard/fuse.rs)

Manages FUSE lifecycle:

```rust
struct FuseManager {
    mount_handle: Option<BackgroundSession>,
    fs: Arc<FuseClipboardFs>,
    mount_point: PathBuf,
}

impl FuseManager {
    /// Mount FUSE filesystem
    fn mount(&mut self) -> Result<()>;

    /// Unmount FUSE filesystem
    fn unmount(&mut self) -> Result<()>;

    /// Update virtual files from file descriptors
    fn set_files(&self, descriptors: Vec<FileDescriptor>) -> Vec<PathBuf>;

    /// Clear all virtual files
    fn clear_files(&self);
}
```

### 3. ClipboardManager Integration

Updates to existing ClipboardManager:

```rust
impl ClipboardManager {
    /// FUSE manager for file clipboard
    fuse_manager: Arc<RwLock<Option<FuseManager>>>,

    /// Handle file paste: create virtual files, return URIs
    async fn handle_file_paste_request(
        &self,
        descriptors: Vec<FileDescriptor>,
    ) -> Result<String> {
        let fuse = self.fuse_manager.read().await;
        let paths = fuse.set_files(descriptors);

        // Build gnome-copied-files format URI list
        let uris = paths.iter()
            .map(|p| format!("file://{}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(format!("copy\n{}\0", uris))
    }
}
```

## Synchronization: Async RDP ↔ Sync FUSE

The key challenge is that FUSE read() is synchronous (called from kernel), but RDP communication is async.

### Solution: Tokio Channel Bridge

```rust
impl FuseClipboardFs {
    fn read(&mut self, _req: &Request, ino: u64, _fh: u64,
            offset: i64, size: u32, _flags: i32,
            _lock_owner: Option<u64>, reply: ReplyData) {

        let file = self.files.read().unwrap().get(&ino).cloned();
        if let Some(file) = file {
            // Send request to async RDP task
            let request = FileRequest {
                file_index: file.file_index,
                offset: offset as u64,
                size,
                response_tx: oneshot::channel(),
            };

            if self.request_tx.blocking_send(request).is_ok() {
                // Wait for response (blocking in FUSE thread)
                match request.response_tx.1.blocking_recv() {
                    Ok(FileResponse::Data(data)) => reply.data(&data),
                    Ok(FileResponse::Error) => reply.error(libc::EIO),
                    Err(_) => reply.error(libc::EIO),
                }
            } else {
                reply.error(libc::EIO);
            }
        } else {
            reply.error(libc::ENOENT);
        }
    }
}
```

### Async Side (in ClipboardManager):

```rust
async fn run_fuse_request_handler(
    mut request_rx: mpsc::Receiver<FileRequest>,
    rdp_sender: mpsc::UnboundedSender<ServerEvent>,
) {
    while let Some(request) = request_rx.recv().await {
        // Send FileContentsRequest to RDP
        let contents_req = FileContentsRequest {
            stream_id: allocate_stream_id(),
            index: request.file_index,
            flags: FileContentsFlags::DATA,
            position: request.offset,
            requested_size: request.size,
            data_id: Some(clip_data_id),
        };

        rdp_sender.send(ServerEvent::Clipboard(
            ClipboardMessage::SendFileContentsRequest(contents_req)
        )).ok();

        // Response will come through handle_rdp_file_contents_response
        // which will send to request.response_tx
    }
}
```

## Mount Point

Use XDG runtime directory for security and cleanup:

```rust
fn get_mount_point() -> PathBuf {
    let uid = unsafe { libc::getuid() };
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", uid));

    PathBuf::from(runtime_dir).join("wrd-clipboard-fuse")
}
```

## Lifecycle

### Session Start
1. Create mount point directory
2. Mount FUSE filesystem
3. Start request handler task

### Windows Copies Files
1. Receive FormatList with FileGroupDescriptorW
2. (Don't fetch file data yet - wait for paste)

### User Pastes
1. Portal sends SelectionTransfer
2. Request FileGroupDescriptorW from RDP
3. Parse file descriptors
4. Create virtual files in FUSE
5. Return URIs pointing to FUSE mount

### Session End
1. Unmount FUSE filesystem
2. Remove mount point directory
3. Clean up channels

## Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
fuser = "0.14"  # FUSE filesystem support
```

## Implementation Order

1. **Phase 1**: Basic FUSE scaffolding
   - Add fuser dependency
   - Create `src/clipboard/fuse.rs`
   - Implement basic Filesystem trait (getattr, lookup, readdir)
   - Mount/unmount lifecycle

2. **Phase 2**: Virtual file management
   - FileEntry struct
   - Add/remove files from FUSE
   - Return URIs for Portal

3. **Phase 3**: On-demand read
   - Channel bridge for sync/async
   - FileContentsRequest integration
   - Data delivery to read()

4. **Phase 4**: Polish
   - Error handling
   - Timeout handling for RDP requests
   - Clipboard locking support (clipDataId)
   - Multiple concurrent transfers

## Testing

1. **Unit tests**: FUSE operations with mock data
2. **Integration tests**: Full clipboard flow with test RDP client
3. **Manual tests**:
   - Single file paste
   - Multiple file paste
   - Large file paste (chunked)
   - Cancel mid-transfer
   - New copy while paste in progress
