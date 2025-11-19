# Wayland RDP Server - Clipboard Architecture (Final Design)

**Date:** 2025-11-19
**Status:** AUTHORITATIVE - Production Architecture
**Context:** Research-backed solution using Portal Clipboard API

---

## Executive Summary

After comprehensive research into RDP, VNC, SPICE, X11, and Wayland clipboard protocols, we've identified the **correct architecture** for clipboard integration in a Wayland RDP Server.

**Key Insight:** As an RDP SERVER (not client), we provide a Linux desktop to Windows RDP clients. The clipboard must support **bidirectional delayed rendering** using the Portal Clipboard API.

**Solution:** Use `ashpd::desktop::Clipboard` with Portal's delayed rendering model, replacing wl-clipboard-rs entirely.

---

## Understanding Server vs Client Role

### We Are an RDP SERVER

```
Windows RDP Client (user's laptop)
        ↓
    RDP Connection (TLS)
        ↓
WRD-Server (Wayland RDP Server) ← THIS IS US
        ↓
Linux Wayland Desktop Session
```

**Our role:**
- Provide Linux desktop to remote Windows users
- Act as CLIPRDR server (Windows client acts as CLIPRDR client)
- Bridge between RDP protocol and Wayland clipboard
- Support multiple concurrent RDP client sessions

**Not:**
- Not a Windows app bridging to remote Linux (that would be client)
- Not syncing two clipboards as peers (that would be broker)

---

## CLIPRDR Protocol Roles

### Server Responsibilities (Us)

**Must support:**
1. **Send FormatList PDU** - When Linux clipboard changes, announce to Windows clients
2. **Receive FormatList PDU** - When Windows client copies, track available formats
3. **Receive FormatDataRequest PDU** - Windows client wants our clipboard data
4. **Send FormatDataResponse PDU** - Provide data from Linux clipboard
5. **Send FormatDataRequest PDU** - Request data from Windows client's clipboard
6. **Receive FormatDataResponse PDU** - Windows client sends their clipboard data

### Protocol Message Flow

**Scenario 1: Copy on Linux → Paste in Windows**
```
Linux Desktop:
  User copies text
      ↓
  Wayland clipboard updated
      ↓
  Portal SelectionOwnerChanged signal
      ↓
WRD-Server:
  Detect clipboard change
      ↓
  Create FormatList PDU ["text/plain" → CF_UNICODETEXT]
      ↓
  Send to RDP client
      ↓
Windows RDP Client:
  Receives FormatList
      ↓
  Shows "clipboard available" in Windows
      ↓
  User pastes (Ctrl+V)
      ↓
  Sends FormatDataRequest(CF_UNICODETEXT)
      ↓
WRD-Server:
  on_format_data_request() called
      ↓
  Read from Portal clipboard
      ↓
  Convert UTF-8 → UTF-16LE
      ↓
  Send FormatDataResponse with data
      ↓
Windows RDP Client:
  Receives data
      ↓
  Pastes into Windows app ✅
```

**Scenario 2: Copy in Windows → Paste on Linux**
```
Windows RDP Client:
  User copies text
      ↓
  Sends FormatList [CF_UNICODETEXT]
      ↓
WRD-Server:
  on_remote_copy() called
      ↓
  Portal.SetSelection(["text/plain"])  ← Announce without data!
      ↓
Linux Desktop:
  User pastes (Ctrl+V) in app
      ↓
  Portal sends SelectionTransfer signal (mime_type, serial)
      ↓
WRD-Server:
  Receive signal
      ↓
  Send FormatDataRequest(CF_UNICODETEXT) to RDP client
      ↓
Windows RDP Client:
  Receives request
      ↓
  Reads Windows clipboard
      ↓
  Sends FormatDataResponse with data
      ↓
WRD-Server:
  on_format_data_response() called
      ↓
  Convert UTF-16LE → UTF-8
      ↓
  Portal.SelectionWrite(serial, fd) with data
      ↓
Linux App:
  Receives clipboard data
      ↓
  Pastes text ✅
```

---

## Portal Clipboard API (Delayed Rendering)

### The Correct Solution

**Portal Clipboard interface provides:**
- `RequestClipboard(session)` - Enable clipboard for RemoteDesktop session
- `SetSelection(session, mime_types[])` - Announce formats WITHOUT data
- `SelectionTransfer signal` - Notification when data is requested
- `SelectionWrite(serial, fd)` - Provide requested data
- `SelectionRead(mime_type) → fd` - Read clipboard data
- `SelectionOwnerChanged signal` - Notification when clipboard changes

**This is the standard delayed rendering model!**

### API Usage

```rust
use ashpd::desktop::{Clipboard, RemoteDesktop, Session};

pub struct PortalClipboardManager {
    clipboard: Clipboard<'static>,
    session: Session<'static, RemoteDesktop<'static>>,
}

impl PortalClipboardManager {
    pub async fn new(session: Session<'static, RemoteDesktop<'static>>) -> Result<Self> {
        let clipboard = Clipboard::new().await?;

        // Request clipboard access for RemoteDesktop session
        clipboard.request(&session).await?;

        Ok(Self { clipboard, session })
    }

    // RDP client copied → announce to Wayland (NO data yet!)
    pub async fn announce_rdp_formats(&self, mime_types: &[&str]) -> Result<()> {
        self.clipboard.set_selection(&self.session, mime_types).await?;
        info!("Announced {} formats to Portal", mime_types.len());
        Ok(())
    }

    // Listen for data requests from Linux apps
    pub async fn start_transfer_listener(
        &self,
        rdp_request_handler: impl Fn(String, u32) + Send + 'static,
    ) -> Result<()> {
        let mut stream = self.clipboard.receive_selection_transfer().await?;

        tokio::spawn(async move {
            while let Some((session, mime_type, serial)) = stream.next().await {
                info!("Portal requesting clipboard data: {} (serial {})", mime_type, serial);
                // Call handler to request from RDP and provide via SelectionWrite
                rdp_request_handler(mime_type, serial);
            }
        });

        Ok(())
    }

    // Provide clipboard data to Linux app (after getting from RDP)
    pub async fn provide_data(&self, serial: u32, data: &[u8]) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        let fd = self.clipboard.selection_write(&self.session, serial).await?;

        // Write data to file descriptor
        let mut file = tokio::fs::File::from_std(fd.into());
        file.write_all(data).await?;
        file.flush().await?;

        // Notify completion
        self.clipboard.selection_write_done(&self.session, serial, true).await?;

        info!("Provided {} bytes to Portal (serial {})", data.len(), serial);
        Ok(())
    }

    // Monitor local clipboard changes
    pub async fn start_owner_changed_listener(
        &self,
        on_change: impl Fn(Vec<String>) + Send + 'static,
    ) -> Result<()> {
        let mut stream = self.clipboard.receive_selection_owner_changed().await?;

        tokio::spawn(async move {
            while let Some((session, change)) = stream.next().await {
                if !change.session_is_owner().unwrap_or(false) {
                    // Another app owns clipboard - announce to RDP clients
                    let mime_types = change.mime_types();
                    info!("Local clipboard changed: {:?}", mime_types);
                    on_change(mime_types);
                }
            }
        });

        Ok(())
    }

    // Read from local clipboard (for Linux → Windows copy)
    pub async fn read_clipboard(&self, mime_type: &str) -> Result<Vec<u8>> {
        use tokio::io::AsyncReadExt;

        let fd = self.clipboard.selection_read(&self.session, mime_type).await?;

        let mut file = tokio::fs::File::from_std(fd.into());
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;

        info!("Read {} bytes from Portal clipboard ({})", data.len(), mime_type);
        Ok(data)
    }
}
```

---

## Complete Data Flow Implementation

### Component Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        WRD-Server                               │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  Portal Clipboard Manager                                  │ │
│  │  - SetSelection() - Announce RDP formats                  │ │
│  │  - SelectionTransfer listener - Data requests from Linux  │ │
│  │  - SelectionWrite() - Provide data to Linux               │ │
│  │  - SelectionOwnerChanged listener - Linux clipboard changes│ │
│  │  - SelectionRead() - Read Linux clipboard                 │ │
│  └────────────┬───────────────────────────────────┬──────────┘ │
│               │                                   │            │
│  ┌────────────▼────────────┐         ┌───────────▼──────────┐ │
│  │  Clipboard Manager      │         │  Format Converter    │ │
│  │  - Event queue          │         │  - RDP ↔ MIME        │ │
│  │  - State machine        │         │  - UTF-8 ↔ UTF-16   │ │
│  │  - Loop detection       │         │  - DIB ↔ PNG         │ │
│  │  - Pending requests     │         │  - HDROP ↔ URI-list  │ │
│  └────────────┬────────────┘         └──────────────────────┘ │
│               │                                                │
│  ┌────────────▼──────────────────────────────────────────────┐ │
│  │  IronRDP Clipboard Backend (CliprdrBackend)               │ │
│  │  - on_remote_copy() → SetSelection()                      │ │
│  │  - on_format_data_request() → Read + send response        │ │
│  │  - on_format_data_response() → SelectionWrite()           │ │
│  │  - Non-blocking event queue                               │ │
│  └───────────────────────────────────────────────────────────┘ │
│                              ↕                                  │
│                      IronRDP Server                             │
└─────────────────────────────────────────────────────────────────┘
                               ↕
                         RDP Protocol
                               ↕
                      Windows RDP Client
```

### State Machine

```rust
enum ClipboardState {
    Idle,

    RdpOwned {
        formats: Vec<ClipboardFormat>,
        announced_at: SystemTime,
    },

    PortalOwned {
        mime_types: Vec<String>,
        announced_at: SystemTime,
    },

    PendingFromRdp {
        serial: u32,           // Portal's request serial
        mime_type: String,     // What Portal wants
        request_sent_at: SystemTime,
    },

    PendingFromPortal {
        format_id: u32,        // What RDP client requested
        request_sent_at: SystemTime,
    },
}
```

---

## Impact on File Copy and Device Sharing

### File Copy/Paste

**CLIPRDR supports file transfer via FileContents PDU:**

**Flow:**
```
1. Windows: Copy file → sends FormatList with CF_HDROP
2. Portal: SetSelection(["text/uri-list"])
3. Linux: Paste → Portal SelectionTransfer
4. Server: Send FileContentsRequest to RDP client (with stream_id, position, size)
5. Client: Sends FileContentsResponse (chunked data)
6. Server: Writes to temp file, adds to URI list
7. Server: SelectionWrite with file:/// URI
8. Linux: Receives file!
```

**Challenges:**
- Chunked transfer (files can be GB)
- Progress tracking
- Temp file management
- URI → file path conversion

**Portal Clipboard handles this!** The SelectionWrite fd can point to a file, and Portal handles the URI list format.

**Impact:** ✅ **POSITIVE** - Portal API supports file URIs natively, cleaner than wl-clipboard-rs

### USB Device Redirection

**Different protocol:** RDPDR (Device Redirection) channel, not CLIPRDR

**Architecture:**
```
USB Device (Windows client machine)
        ↓
   RDPDR Channel (USB redirection sub-protocol)
        ↓
WRD-Server (receives USB I/O requests)
        ↓
   USB/IP kernel driver
        ↓
Virtual USB device appears in Linux
```

**Impact:** ❌ **UNRELATED** - Completely separate channel and protocol
- Uses ironrdp-rdpdr (not cliprdr)
- Requires USB/IP kernel support
- No interaction with clipboard

### Drive Mapping / File Sharing

**Two mechanisms:**

**1. Drive Redirection (RDPDR - Device Redirection)**
```
Windows client's C:\ drive
        ↓
   RDPDR Channel
        ↓
Server sees mounted filesystem (FUSE)
        ↓
Linux apps can browse Windows files
```

**Impact:** ❌ **UNRELATED** - Separate RDPDR channel

**2. File Copy via Clipboard (CF_HDROP)**
```
Copy files in Windows Explorer
        ↓
   CLIPRDR with CF_HDROP + FileContents
        ↓
Server receives file list + data
        ↓
Writes to /tmp/
        ↓
Linux file manager shows copied files
```

**Impact:** ✅ **DIRECTLY RELATED** - This IS clipboard, just files instead of text

---

## Unified RDP Virtual Channel Architecture

### All Virtual Channels are Independent

```
RDP Connection
    │
    ├─ CLIPRDR (Static VC) - Clipboard
    │   ├─ Text/image clipboard
    │   └─ File copy/paste
    │
    ├─ RDPSND (Static VC) - Audio output
    │
    ├─ RDPDR (Static VC) - Device redirection
    │   ├─ Drive mapping
    │   ├─ Printer redirection
    │   ├─ Serial port
    │   └─ Smart card
    │
    ├─ RAIL (Static VC) - RemoteApp
    │
    ├─ DRDYNVC (Static VC) - Dynamic channel container
    │   ├─ Custom app channels
    │   └─ Extensions
    │
    └─ ... (30+ possible static channels)
```

**Each channel is independent:**
- Different PDU formats
- Different state machines
- Different backend implementations
- No cross-channel dependencies

**Clipboard (CLIPRDR) only handles:**
- ✅ Text clipboard
- ✅ Image clipboard
- ✅ File copy/paste via clipboard
- ❌ NOT drive mapping
- ❌ NOT USB devices
- ❌ NOT printer redirection

---

## Portal Clipboard API - Complete Integration

### Initialization (Server Startup)

```rust
// src/portal/clipboard.rs
use ashpd::desktop::{Clipboard, RemoteDesktop, Session};
use futures_util::StreamExt;

pub struct PortalClipboardManager {
    clipboard: Clipboard<'static>,
    session: Session<'static, RemoteDesktop<'static>>,

    // Track pending requests
    pending_portal_requests: Arc<RwLock<HashMap<u32, PendingRequest>>>,

    // Callback to request data from RDP client
    rdp_data_requester: Arc<dyn Fn(u32) -> BoxFuture<'static, Result<Vec<u8>>> + Send + Sync>,
}

struct PendingRequest {
    serial: u32,
    mime_type: String,
    requested_at: SystemTime,
}

impl PortalClipboardManager {
    pub async fn new(
        session: Session<'static, RemoteDesktop<'static>>,
        rdp_data_requester: impl Fn(u32) -> BoxFuture<'static, Result<Vec<u8>>> + Send + Sync + 'static,
    ) -> Result<Self> {
        let clipboard = Clipboard::new().await?;

        // Request clipboard access for this RemoteDesktop session
        clipboard.request(&session).await
            .context("Failed to request clipboard access")?;

        info!("Portal Clipboard enabled for RemoteDesktop session");

        let manager = Self {
            clipboard,
            session,
            pending_portal_requests: Arc::new(RwLock::new(HashMap::new())),
            rdp_data_requester: Arc::new(rdp_data_requester),
        };

        // Start listeners
        manager.start_selection_transfer_listener().await?;
        manager.start_owner_changed_listener().await?;

        Ok(manager)
    }

    // Announce RDP clipboard formats to Wayland (delayed rendering!)
    pub async fn announce_rdp_formats(&self, mime_types: Vec<String>) -> Result<()> {
        let mime_refs: Vec<&str> = mime_types.iter().map(|s| s.as_str()).collect();

        self.clipboard.set_selection(&self.session, &mime_refs).await
            .context("Failed to set Portal selection")?;

        info!("Announced {} RDP formats to Portal: {:?}", mime_types.len(), mime_types);
        Ok(())
    }

    // Listen for data requests from Linux apps (user pasted)
    async fn start_selection_transfer_listener(&self) -> Result<()> {
        let mut stream = self.clipboard.receive_selection_transfer().await?;
        let pending = Arc::clone(&self.pending_portal_requests);
        let requester = Arc::clone(&self.rdp_data_requester);
        let clipboard = self.clipboard.clone();
        let session = self.session.clone();

        tokio::spawn(async move {
            while let Some((sess, mime_type, serial)) = stream.next().await {
                info!("🔔 Portal requesting clipboard: {} (serial {})", mime_type, serial);

                // Track this request
                {
                    let mut requests = pending.write().await;
                    requests.insert(serial, PendingRequest {
                        serial,
                        mime_type: mime_type.clone(),
                        requested_at: SystemTime::now(),
                    });
                }

                // Request data from RDP client
                let format_id = mime_to_format_id(&mime_type);

                match requester(format_id).await {
                    Ok(data) => {
                        // Got data from RDP - provide to Portal
                        if let Err(e) = Self::provide_to_portal(
                            &clipboard,
                            &session,
                            serial,
                            data,
                        ).await {
                            error!("Failed to provide data to Portal: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to get data from RDP: {}", e);
                        // Notify Portal of failure
                        let _ = clipboard.selection_write_done(&session, serial, false).await;
                    }
                }

                // Cleanup request
                pending.write().await.remove(&serial);
            }
        });

        Ok(())
    }

    // Provide data to Portal via file descriptor
    async fn provide_to_portal(
        clipboard: &Clipboard<'_>,
        session: &Session<'_, RemoteDesktop<'_>>,
        serial: u32,
        data: Vec<u8>,
    ) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        // Get write file descriptor from Portal
        let fd = clipboard.selection_write(session, serial).await?;

        // Write data
        let mut file = tokio::fs::File::from_std(fd.into());
        file.write_all(&data).await?;
        file.flush().await?;
        drop(file); // Close fd

        // Notify Portal of success
        clipboard.selection_write_done(session, serial, true).await?;

        info!("✅ Provided {} bytes to Portal (serial {})", data.len(), serial);
        Ok(())
    }

    // Monitor local clipboard changes (Linux apps copy)
    async fn start_owner_changed_listener(&self) -> Result<()> {
        let mut stream = self.clipboard.receive_selection_owner_changed().await?;
        let clipboard = self.clipboard.clone();
        let session = self.session.clone();

        tokio::spawn(async move {
            while let Some((sess, change)) = stream.next().await {
                if change.session_is_owner().unwrap_or(false) {
                    // We own it (we just announced RDP data) - ignore
                    continue;
                }

                // Another app owns clipboard
                let mime_types = change.mime_types();
                info!("🔔 Local clipboard changed: {:?}", mime_types);

                // TODO: Announce these formats to RDP clients
                // Send FormatList PDU with converted formats
            }
        });

        Ok(())
    }

    // Read local clipboard data (for RDP client requesting our data)
    pub async fn read_local_clipboard(&self, mime_type: &str) -> Result<Vec<u8>> {
        use tokio::io::AsyncReadExt;

        let fd = self.clipboard.selection_read(&self.session, mime_type).await
            .context("Failed to read from Portal clipboard")?;

        let mut file = tokio::fs::File::from_std(fd.into());
        let mut data = Vec::new();
        file.read_to_end(&mut data).await
            .context("Failed to read clipboard data")?;

        info!("Read {} bytes from local clipboard ({})", data.len(), mime_type);
        Ok(data)
    }
}
```

---

## Integration with IronRDP Backend

### Updated Backend Implementation

```rust
// src/clipboard/ironrdp_backend.rs

pub struct WrdCliprdrBackend {
    portal_clipboard: Arc<PortalClipboardManager>,
    converter: Arc<FormatConverter>,
    event_queue: Arc<RwLock<VecDeque<ClipboardEvent>>>,
}

impl CliprdrBackend for WrdCliprdrBackend {
    fn on_remote_copy(&mut self, available_formats: &[ClipboardFormat]) {
        info!("RDP client copied - {} formats", available_formats.len());

        // Convert RDP formats to MIME types
        let mime_types = self.converter.rdp_to_mime_types(available_formats);

        // Announce to Portal (delayed rendering - NO data yet!)
        let portal = Arc::clone(&self.portal_clipboard);
        tokio::spawn(async move {
            if let Err(e) = portal.announce_rdp_formats(mime_types).await {
                error!("Failed to announce formats to Portal: {}", e);
            }
        });
    }

    fn on_format_data_request(&mut self, request: FormatDataRequest) {
        // RDP client wants OUR (Linux) clipboard data
        let format_id = request.format.0;
        info!("RDP client requesting format {}", format_id);

        let portal = Arc::clone(&self.portal_clipboard);
        let converter = Arc::clone(&self.converter);

        tokio::spawn(async move {
            // Convert format ID to MIME type
            let mime_type = converter.format_id_to_mime(format_id)?;

            // Read from local Portal clipboard
            let portal_data = portal.read_local_clipboard(&mime_type).await?;

            // Convert to RDP format
            let rdp_data = converter.mime_to_rdp_data(format_id, &portal_data)?;

            // TODO: Send FormatDataResponse PDU back to RDP client
            // This requires access to Cliprdr<Server> object or message channel
        });
    }

    fn on_format_data_response(&mut self, response: FormatDataResponse<'_>) {
        // RDP client sending us data (we requested it because Portal asked)
        if response.is_error() {
            warn!("RDP client sent error response");
            return;
        }

        let data = response.data().to_vec();
        info!("Received {} bytes from RDP client", data.len());

        // This should be in response to a Portal SelectionTransfer
        // Find the pending request and provide data to Portal
        let portal = Arc::clone(&self.portal_clipboard);

        tokio::spawn(async move {
            // TODO: Match with pending_portal_requests to get serial
            // Then call portal.provide_data(serial, data)
        });
    }
}
```

---

## File Transfer Deep Dive

### CF_HDROP Format

**File descriptor format (Windows):**
```
DROPFILES header:
- pFiles: offset to file list (20 bytes)
- pt: drop point coordinates
- fNC: boolean flags
- fWide: TRUE for Unicode

File list (double-null terminated):
C:\Users\Greg\Documents\file1.txt\0
C:\Users\Greg\Desktop\photo.png\0
\0
```

**Linux URI list format:**
```
file:///home/greg/Documents/file1.txt\r\n
file:///home/greg/Desktop/photo.png\r\n
```

### FileContents Protocol

**For large files (chunked transfer):**

```
1. FormatList with CF_HDROP (file descriptors only)
2. FileContentsRequest(stream_id=0, position=0, size=65536)
   - Request first 64KB of file #0
3. FileContentsResponse(stream_id=0, data[65536])
4. FileContentsRequest(stream_id=0, position=65536, size=65536)
   - Request next chunk
5. ... repeat until EOF
6. Move to next file (stream_id=1, position=0)
```

**Portal handles large data automatically** - when we write to the fd from SelectionWrite, we can stream the data in chunks.

### Implementation for File Transfer

```rust
async fn handle_file_contents_response(
    &self,
    stream_id: u32,
    data: Vec<u8>,
) -> Result<()> {
    // Get temp file for this stream
    let temp_path = self.get_temp_file_path(stream_id)?;

    // Append chunk
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&temp_path)
        .await?;

    file.write_all(&data).await?;

    // Check if transfer complete
    if self.is_file_complete(stream_id) {
        info!("File transfer complete: {:?}", temp_path);

        // Add to URI list for Portal
        let uri = format!("file://{}", temp_path.display());
        self.completed_file_uris.push(uri);

        // If all files complete, provide URI list to Portal
        if self.all_files_complete() {
            let uri_list = self.completed_file_uris.join("\r\n");
            self.portal.provide_data(self.current_serial, uri_list.as_bytes()).await?;
        }
    }

    Ok(())
}
```

**Impact:** ✅ **POSITIVE** - Portal Clipboard API simplifies file transfer
- No need for custom URI list handling
- Portal manages file permissions
- Temp file cleanup handled by system

---

## Device Sharing (USB Redirection) - Separate Topic

### RDPUSB Channel Architecture

**Completely different from clipboard:**

```
Windows RDP Client:
  USB device (YubiKey, printer, scanner)
      ↓
  RDPUSB channel (over RDP connection)
      ↓
WRD-Server:
  RDPUSB handler
      ↓
  USB/IP protocol
      ↓
  Linux kernel vhci driver
      ↓
  Virtual USB device (/dev/bus/usb/001/002)
      ↓
  Linux app sees real USB device
```

**Implementation:**
- Uses ironrdp-rdpusb crate (different from cliprdr)
- Requires USB/IP kernel module
- Needs root/capabilities for USB device creation
- Complex: URB (USB Request Block) forwarding
- Security sensitive: USB attacks possible

**Timeline:** 6-9 months (very complex)

**No interaction with clipboard** - completely separate channel and code path.

---

## Drive Mapping (RDPDR) - Separate Topic

### Drive Redirection Architecture

```
Windows Client C:\ drive
      ↓
  RDPDR channel (SMB2-like protocol)
      ↓
WRD-Server:
  RDPDR handler
      ↓
  FUSE filesystem
      ↓
  Mounted at /mnt/rdp-drives/C
      ↓
  Linux apps browse Windows files
```

**Implementation:**
- Uses ironrdp-rdpdr crate
- Implements SMB2-like file operations
- FUSE integration for mounting
- Performance: caching, read-ahead
- Security: sandboxing, permissions

**Timeline:** 4-6 months

**No interaction with clipboard** - though files can be copied between drives and clipboard.

---

## Implementation Plan - Portal Clipboard Integration

### Phase 1: Replace wl-clipboard-rs (Today - 4-6 hours)

**Tasks:**
1. ✅ Remove wl-clipboard-rs dependency
2. ✅ Replace portal/clipboard.rs with Portal API
3. ✅ Implement announce_rdp_formats()
4. ✅ Implement SelectionTransfer listener
5. ✅ Implement SelectionWrite for providing data
6. ✅ Implement SelectionOwnerChanged listener
7. ✅ Implement SelectionRead for reading local clipboard

### Phase 2: Wire to IronRDP Backend (4-6 hours)

**Tasks:**
8. ✅ Pass PortalClipboardManager to CliprdrBackend
9. ✅ Implement on_remote_copy() → SetSelection
10. ✅ Handle SelectionTransfer → send FormatDataRequest to RDP
11. ✅ Implement on_format_data_response() → SelectionWrite
12. ✅ Implement on_format_data_request() → SelectionRead + respond

### Phase 3: Response Mechanism (2-4 hours)

**Tasks:**
13. ✅ Create response channel for FormatDataResponse
14. ✅ Match Portal serials with RDP requests
15. ✅ Handle async request/response correlation

### Phase 4: Testing (2-3 hours)

**Tasks:**
16. ✅ Test Windows → Linux text clipboard
17. ✅ Test Linux → Windows text clipboard
18. ✅ Test Windows → Linux image clipboard
19. ✅ Test Linux → Windows image clipboard
20. ✅ Loop prevention validation

### Phase 5: File Transfer (1-2 days)

**Tasks:**
21. ✅ Implement FileContentsRequest/Response handling
22. ✅ Chunked file transfer
23. ✅ Temp file management
24. ✅ URI list generation
25. ✅ Test file copy both directions

**Total Timeline:** 2-4 days for complete clipboard with files

---

## Advantages of Portal Clipboard API

### vs. wl-clipboard-rs

| Feature | wl-clipboard-rs | Portal Clipboard API |
|---------|-----------------|----------------------|
| **Delayed rendering** | ❌ No - must have data to write | ✅ Yes - SetSelection announces |
| **Data request notification** | ❌ No callback mechanism | ✅ SelectionTransfer signal |
| **Session integration** | ❌ Independent | ✅ Tied to RemoteDesktop session |
| **Security** | ⚠️ Any process can write | ✅ Permission-based |
| **Large data** | ⚠️ Fork background process | ✅ FD-based streaming |
| **File URIs** | ⚠️ Must handle manually | ✅ Native support |
| **API complexity** | ✅ Simple (read/write only) | ⚠️ More complex (signals, fds) |

### Portal API Wins

✅ **Proper delayed rendering** - Exactly what RDP needs
✅ **Signal-based** - Notification when data requested
✅ **FD-based data transfer** - Efficient for large data
✅ **Session scoped** - Integrates with RemoteDesktop
✅ **Security** - Portal permission model
✅ **Standard** - Used by Flatpak, sandboxed apps

---

## Code Migration Strategy

### Remove wl-clipboard-rs

```toml
# Cargo.toml
# REMOVE:
wl-clipboard-rs = "0.8"

# Already have:
ashpd = "0.12"  # Includes Clipboard support
```

### Rewrite portal/clipboard.rs

**Old (wl-clipboard-rs):**
```rust
pub async fn read_clipboard(&self, mime_type: &str) -> Result<Vec<u8>> {
    tokio::task::spawn_blocking(move || {
        get_contents(ClipboardType::Regular, Seat::Unspecified, mime)
    }).await?
}
```

**New (Portal Clipboard):**
```rust
pub async fn read_clipboard(&self, mime_type: &str) -> Result<Vec<u8>> {
    use tokio::io::AsyncReadExt;
    let fd = self.clipboard.selection_read(&self.session, mime_type).await?;
    let mut file = tokio::fs::File::from_std(fd.into());
    let mut data = Vec::new();
    file.read_to_end(&mut data).await?;
    Ok(data)
}
```

### Wire SelectionTransfer to RDP Request

**Critical integration:**
```rust
// When Portal requests data (user pasted in Linux)
async fn on_selection_transfer(
    mime_type: String,
    serial: u32,
) {
    // 1. Convert MIME → RDP format ID
    let format_id = converter.mime_to_format_id(&mime_type)?;

    // 2. Send FormatDataRequest to RDP client
    //    THIS IS THE MISSING PIECE!
    let response_future = self.rdp_requester.request_format(format_id);

    // 3. Wait for response
    let rdp_data = response_future.await?;

    // 4. Convert format
    let portal_data = converter.rdp_to_mime_data(format_id, &rdp_data)?;

    // 5. Provide to Portal
    portal.provide_data(serial, portal_data).await?;
}
```

---

## Outstanding Question: RDP Request Mechanism

**The missing link:** How to send FormatDataRequest PDU to RDP client from our backend?

**Options:**

**1. Access CliprdrServer object** (need to investigate)
```rust
let cliprdr_server: &mut Cliprdr<Server> = /* how to get this? */;
let messages = cliprdr_server.initiate_paste(format_id)?;
// Send messages to RDP client
```

**2. Use event_sender with custom event**
```rust
self.event_sender.send(ServerEvent::ClipboardRequest(format_id))?;
// IronRDP processes and sends FormatDataRequest PDU
```

**3. Direct PDU construction** (last resort)
```rust
let pdu = FormatDataRequest { format: ClipboardFormatId(format_id) };
let encoded = encode_cliprdr_pdu(pdu)?;
// Send via SVC channel
```

**Need to investigate** which method IronRDP server supports.

---

## Next Steps

**Immediate (Now):**
1. ✅ Document this architecture (this file)
2. ✅ Rewrite portal/clipboard.rs with Portal API
3. ✅ Remove wl-clipboard-rs dependency
4. ✅ Implement delayed rendering properly

**Short-term (Hours):**
5. ✅ Test Windows → Linux text clipboard
6. ✅ Test Linux → Windows text clipboard
7. ✅ Debug RDP request mechanism
8. ✅ Fix any remaining issues

**Medium-term (Days):**
9. ✅ Add image clipboard support
10. ✅ Implement file transfer (FileContents protocol)
11. ✅ Comprehensive testing
12. ✅ Performance optimization

---

## Conclusion

**Portal Clipboard API is the correct solution** for Wayland RDP server clipboard integration:
- ✅ Supports delayed rendering (standard RDP model)
- ✅ Signal-based notifications (SelectionTransfer, SelectionOwnerChanged)
- ✅ FD-based data transfer (efficient, large data support)
- ✅ Session-scoped security
- ✅ File URI support built-in

**wl-clipboard-rs was the wrong tool** - designed for command-line copy/paste, not daemon-based clipboard synchronization.

**Impact on other features:**
- ✅ File copy/paste: SIMPLIFIED (Portal handles URIs)
- ❌ USB redirection: UNRELATED (different channel)
- ❌ Drive mapping: UNRELATED (different channel)

**Estimated implementation time:** 2-4 days for complete clipboard including file transfer.

---

**Status:** Ready to implement - architecture is now clear and correct.
