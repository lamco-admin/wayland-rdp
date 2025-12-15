# ASHPD Clipboard API - Complete Research Document

## Executive Summary

The `ashpd` library (v0.12.0) provides Rust bindings for the XDG Desktop Portal Clipboard API, which implements delayed rendering clipboard integration primarily for RemoteDesktop sessions. This document provides comprehensive documentation of the API, workflow, and implementation details for production Wayland RDP server integration.

**Critical Note**: The Clipboard portal does NOT create its own session - it extends existing RemoteDesktop sessions with clipboard functionality.

---

## Table of Contents

1. [API Overview](#api-overview)
2. [Complete Method Signatures](#complete-method-signatures)
3. [Delayed Rendering Workflow](#delayed-rendering-workflow)
4. [Session and Lifetime Management](#session-and-lifetime-management)
5. [File Descriptor Handling](#file-descriptor-handling)
6. [XDG Portal Specification](#xdg-portal-specification)
7. [Implementation Examples](#implementation-examples)
8. [Important Notes and Pitfalls](#important-notes-and-pitfalls)
9. [References](#references)

---

## API Overview

### Module Information

- **Crate**: `ashpd` v0.12.0
- **Module Path**: `ashpd::desktop::clipboard`
- **DBus Interface**: `org.freedesktop.portal.Clipboard`
- **Purpose**: Clipboard integration for RemoteDesktop sessions using delayed rendering

### Primary Structs

#### `Clipboard<'a>`

Wrapper around the DBus interface for clipboard operations. Contains a `zbus::Proxy<'a>` internally.

```rust
pub struct Clipboard<'a>(/* private fields */);
```

**Trait Implementations**:
- `Deref` → `Proxy<'a>` (provides access to underlying zbus proxy)
- `Send`, `Sync`, `Unpin`

#### `SelectionOwnerChanged`

Represents details of a new clipboard selection event.

```rust
pub struct SelectionOwnerChanged {
    // private fields
}
```

**Methods**:
- `session_is_owner() -> Option<bool>` - Whether the session owns the clipboard
- `mime_types() -> Vec<String>` - List of available MIME types

**Trait Implementations**:
- `Debug`
- `Deserialize<'de>` (serde)
- `Type` (zvariant)
- `Send`, `Sync`, `Unpin`, `UnwindSafe`

---

## Complete Method Signatures

### Instance Creation

```rust
pub async fn new() -> Result<Clipboard<'a>>
```

Creates a new Clipboard instance by connecting to the DBus interface.

**Returns**: `Result<Clipboard<'a>>` - A new clipboard proxy instance
**Errors**: Returns error if DBus connection fails

---

### Request Clipboard Access

```rust
pub async fn request(
    &self,
    session: &Session<'_, RemoteDesktop<'_>>
) -> Result<()>
```

Requests clipboard access for a portal session. **Must be called before the session starts**.

**Parameters**:
- `session` - Reference to an active RemoteDesktop session

**Returns**: `Result<()>`
**DBus Method**: `RequestClipboard`
**Specification**: Must be called before `RemoteDesktop::start()`

---

### Set Available MIME Types

```rust
pub async fn set_selection(
    &self,
    session: &Session<'_, RemoteDesktop<'_>>,
    mime_types: &[&str]
) -> Result<()>
```

Advertises which MIME types the session can provide clipboard content for. Sets the session as the owner of the clipboard for the specified formats.

**Parameters**:
- `session` - Reference to an active RemoteDesktop session
- `mime_types` - Slice of MIME type strings (e.g., `["text/plain", "text/html"]`)

**Returns**: `Result<()>`
**DBus Method**: `SetSelection`
**Requirements**:
- Session must already be started
- Clipboard access must have been granted via `request()`

---

### Write Selection Data (Answer to SelectionTransfer)

```rust
pub async fn selection_write(
    &self,
    session: &Session<'_, RemoteDesktop<'_>>,
    serial: u32
) -> Result<OwnedFd>
```

Transfers clipboard content for a given serial number. This is the response to a `SelectionTransfer` signal.

**Parameters**:
- `session` - Reference to an active RemoteDesktop session
- `serial` - Serial number from the `SelectionTransfer` signal

**Returns**: `Result<OwnedFd>` - File descriptor created by the callee for writing data
**DBus Method**: `SelectionWrite`
**File Descriptor**: The portal creates the FD; the session writes clipboard data to it
**Type**: `OwnedFd` from `zvariant` (wraps `std::os::fd::OwnedFd`)

**Important**: After writing data, you MUST call `selection_write_done()` with the same serial.

---

### Signal Write Completion

```rust
pub async fn selection_write_done(
    &self,
    session: &Session<'_, RemoteDesktop<'_>>,
    serial: u32,
    success: bool
) -> Result<()>
```

Notifies that clipboard data transfer has completed (successfully or failed).

**Parameters**:
- `session` - Reference to an active RemoteDesktop session
- `serial` - Serial number from the original `SelectionTransfer` signal
- `success` - `true` if transfer succeeded, `false` if it failed

**Returns**: `Result<()>`
**DBus Method**: `SelectionWriteDone`
**Requirements**: Must be called after handling each `SelectionTransfer` request

---

### Read Selection Data

```rust
pub async fn selection_read(
    &self,
    session: &Session<'_, RemoteDesktop<'_>>,
    mime_type: &str
) -> Result<OwnedFd>
```

Reads clipboard content for a specific MIME type from the system clipboard.

**Parameters**:
- `session` - Reference to an active RemoteDesktop session
- `mime_type` - The MIME type to read (e.g., `"text/plain"`)

**Returns**: `Result<OwnedFd>` - File descriptor for reading clipboard data
**DBus Method**: `SelectionRead`
**File Descriptor**: The caller creates the FD and receives data from it

---

### Receive Selection Owner Changed Events

```rust
pub async fn receive_selection_owner_changed(
    &self
) -> Result<impl Stream<Item = (Session<'_, RemoteDesktop<'_>>, SelectionOwnerChanged)>>
```

Creates a stream that emits events when clipboard ownership changes.

**Returns**: `Result<impl Stream<Item = (Session, SelectionOwnerChanged)>>`
**DBus Signal**: `SelectionOwnerChanged`
**Stream Items**: Tuple of:
1. The session handle
2. `SelectionOwnerChanged` struct with:
   - Available MIME types
   - Whether this session is the owner

**Usage**: Poll this stream to detect when clipboard content changes or ownership transfers.

---

### Receive Selection Transfer Requests

```rust
pub async fn receive_selection_transfer(
    &self
) -> Result<impl Stream<Item = (Session<'_, RemoteDesktop<'_>>, String, u32)>>
```

Creates a stream that emits clipboard content requests. This is the core of delayed rendering.

**Returns**: `Result<impl Stream<Item = (Session, String, u32)>>`
**DBus Signal**: `SelectionTransfer`
**Stream Items**: Tuple of:
1. The session handle
2. MIME type being requested (e.g., `"text/plain"`)
3. Serial number for tracking this request

**Usage**: When an item is received, call `selection_write()` with the serial to provide data, then call `selection_write_done()`.

---

## Delayed Rendering Workflow

### Overview

Delayed rendering (also called "lazy clipboard") means clipboard data is only transferred when actually pasted, not when copied. This is critical for:
- Large data transfers
- Multiple format support
- Network efficiency in RDP scenarios

### Complete Workflow Diagram

```
┌─────────────────┐
│ Create Session  │
│  RemoteDesktop  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Request         │
│ Clipboard Access│  ← clipboard.request(&session)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Start Session   │  ← remote_desktop.start(&session)
└────────┬────────┘
         │
         ▼
    ┌────────────────────────────────────────┐
    │       USER COPIES DATA                 │
    └────────────────────────────────────────┘
         │
         ▼
┌─────────────────┐
│ Set Selection   │  ← clipboard.set_selection(&session, &["text/plain", "text/html"])
│ (Advertise MIME)│
└────────┬────────┘
         │
         │  emit: SelectionOwnerChanged signal
         │        ↓ receive_selection_owner_changed()
         │
    ┌────────────────────────────────────────┐
    │       USER PASTES DATA                 │
    └────────────────────────────────────────┘
         │
         │  emit: SelectionTransfer signal
         │        ↓ receive_selection_transfer()
         │        returns: (session, "text/plain", serial: 42)
         │
         ▼
┌─────────────────┐
│ Selection Write │  ← clipboard.selection_write(&session, 42)
│ Get FD          │  → returns OwnedFd
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Write Data to FD│  ← Write clipboard content to file descriptor
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Write Done      │  ← clipboard.selection_write_done(&session, 42, true)
└─────────────────┘
```

### Reading from System Clipboard

```
┌─────────────────┐
│ User Pastes     │
│ into RDP client │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Selection Read  │  ← clipboard.selection_read(&session, "text/plain")
│ Get FD          │  → returns OwnedFd
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Read Data from  │
│ File Descriptor │
└─────────────────┘
```

---

## Session and Lifetime Management

### Session Type Structure

```rust
Session<'a, T: SessionPortal>
```

The Session struct is generic over:
- **Lifetime `'a`**: Bounds the session lifetime
- **Type `T`**: Must implement `SessionPortal` trait (e.g., `RemoteDesktop`)

### Clipboard Session Relationship

All clipboard methods require a session parameter typed as:

```rust
session: &Session<'_, RemoteDesktop<'_>>
```

This enforces at compile-time that:
1. Clipboard operations ONLY work with RemoteDesktop sessions
2. The session cannot outlive the RemoteDesktop instance
3. Clipboard proxy cannot outlive the session

### Typical Session Lifecycle

```rust
use ashpd::desktop::{
    remote_desktop::{DeviceType, RemoteDesktop},
    clipboard::Clipboard,
    PersistMode,
};

async fn example() -> ashpd::Result<()> {
    // 1. Create RemoteDesktop proxy
    let remote_desktop = RemoteDesktop::new().await?;

    // 2. Create session (tied to remote_desktop lifetime)
    let session = remote_desktop.create_session().await?;

    // 3. Select devices
    remote_desktop.select_devices(
        &session,
        DeviceType::Keyboard | DeviceType::Pointer,
        None,
        PersistMode::DoNot,
    ).await?;

    // 4. Request clipboard BEFORE starting session
    let clipboard = Clipboard::new().await?;
    clipboard.request(&session).await?;

    // 5. Start the session (shows dialog to user)
    let response = remote_desktop.start(&session, None).await?.response()?;

    // 6. Now clipboard is active and can be used
    // Session stays alive until dropped or explicitly closed

    Ok(())
}
```

### Session Methods

```rust
// Close session explicitly
session.close().await?;

// Receive close signal
let mut close_stream = session.receive_closed().await?;
while let Some(_) = close_stream.next().await {
    // Session was closed
    break;
}
```

---

## File Descriptor Handling

### OwnedFd Overview

`OwnedFd` is from the `zvariant` crate, which wraps `std::os::fd::OwnedFd`:

```rust
pub struct OwnedFd { /* private */ }
```

**Characteristics**:
- Owns the file descriptor
- Automatically closes FD when dropped (RAII)
- Can be converted to raw FD to prevent auto-close
- DBus type signature: `'h'`

### Trait Implementations

- `AsFd` - Borrow the FD
- `AsRawFd` - Get raw integer FD
- `IntoRawFd` - Take ownership and prevent close-on-drop
- `From<std::os::fd::OwnedFd>` - Conversion from std library
- `Send`, `Sync`, `Unpin` - Safe for async/concurrent use

### Reading from OwnedFd

```rust
use std::os::unix::io::{FromRawFd, AsRawFd};
use std::fs::File;
use std::io::Read;

async fn read_clipboard_data(owned_fd: OwnedFd) -> std::io::Result<Vec<u8>> {
    // Convert to File for reading (takes ownership)
    let raw_fd = owned_fd.as_raw_fd();
    let mut file = unsafe { File::from_raw_fd(raw_fd) };

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // File will be closed when dropped
    Ok(buffer)
}
```

**Safety Note**: `from_raw_fd()` is unsafe because:
- No guarantee the FD is valid
- Unclear who owns the FD
- Must ensure FD isn't closed elsewhere

### Writing to OwnedFd

```rust
use std::os::unix::io::{FromRawFd, AsRawFd};
use std::fs::File;
use std::io::Write;

async fn write_clipboard_data(owned_fd: OwnedFd, data: &[u8]) -> std::io::Result<()> {
    let raw_fd = owned_fd.as_raw_fd();
    let mut file = unsafe { File::from_raw_fd(raw_fd) };

    file.write_all(data)?;
    file.flush()?;

    // FD automatically closed when file is dropped
    Ok(())
}
```

### Async File Descriptor Operations with Tokio

For production systems, use async I/O:

```rust
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::os::unix::io::FromRawFd;

async fn async_read_clipboard(owned_fd: OwnedFd) -> tokio::io::Result<Vec<u8>> {
    let raw_fd = owned_fd.as_raw_fd();

    // Convert to tokio File for async operations
    let std_file = unsafe { std::fs::File::from_raw_fd(raw_fd) };
    let mut tokio_file = File::from_std(std_file);

    let mut buffer = Vec::new();
    tokio_file.read_to_end(&mut buffer).await?;

    Ok(buffer)
}

async fn async_write_clipboard(owned_fd: OwnedFd, data: &[u8]) -> tokio::io::Result<()> {
    let raw_fd = owned_fd.as_raw_fd();

    let std_file = unsafe { std::fs::File::from_raw_fd(raw_fd) };
    let mut tokio_file = File::from_std(std_file);

    tokio_file.write_all(data).await?;
    tokio_file.flush().await?;

    Ok(())
}
```

### Using with Streams

```rust
use tokio_util::io::ReaderStream;
use futures::StreamExt;

async fn stream_read_clipboard(owned_fd: OwnedFd) -> tokio::io::Result<Vec<u8>> {
    let raw_fd = owned_fd.as_raw_fd();
    let std_file = unsafe { std::fs::File::from_raw_fd(raw_fd) };
    let tokio_file = tokio::fs::File::from_std(std_file);

    let mut stream = ReaderStream::new(tokio_file);
    let mut chunks = Vec::new();

    while let Some(chunk) = stream.next().await {
        chunks.extend_from_slice(&chunk?);
    }

    Ok(chunks)
}
```

---

## XDG Portal Specification

### DBus Interface XML

```xml
<?xml version="1.0"?>
<node name="/" xmlns:doc="http://www.freedesktop.org/dbus/1.0/doc.dtd">
  <interface name="org.freedesktop.portal.Clipboard">

    <method name="RequestClipboard">
      <arg type="o" name="session_handle" direction="in"/>
      <arg type="a{sv}" name="options" direction="in"/>
    </method>

    <method name="SetSelection">
      <arg type="o" name="session_handle" direction="in"/>
      <arg type="a{sv}" name="options" direction="in"/>
    </method>

    <method name="SelectionWrite">
      <annotation name="org.gtk.GDBus.C.UnixFD" value="true"/>
      <arg type="o" name="session_handle" direction="in"/>
      <arg type="u" name="serial" direction="in"/>
      <arg type="h" name="fd" direction="out"/>
    </method>

    <method name="SelectionWriteDone">
      <arg type="o" name="session_handle" direction="in"/>
      <arg type="u" name="serial" direction="in"/>
      <arg type="b" name="success" direction="in"/>
    </method>

    <method name="SelectionRead">
      <annotation name="org.gtk.GDBus.C.UnixFD" value="true"/>
      <arg type="o" name="session_handle" direction="in"/>
      <arg type="s" name="mime_type" direction="in"/>
      <arg type="h" name="fd" direction="out"/>
    </method>

    <signal name="SelectionOwnerChanged">
      <arg type="o" name="session_handle" direction="out"/>
      <arg type="a{sv}" name="options" direction="out"/>
    </signal>

    <signal name="SelectionTransfer">
      <arg type="o" name="session_handle" direction="out"/>
      <arg type="s" name="mime_type" direction="out"/>
      <arg type="u" name="serial" direction="out"/>
    </signal>

    <property name="version" type="u" access="read"/>
  </interface>
</node>
```

### DBus Type Reference

- `o` - Object path (session handle)
- `a{sv}` - Dictionary of string keys to variant values (options)
- `u` - Unsigned 32-bit integer (serial)
- `h` - Unix file descriptor (passed via SCM_RIGHTS)
- `s` - String (MIME type)
- `b` - Boolean (success flag)

### SetSelection Options

The `options` dictionary for `SetSelection` contains:

```
{
  "mime_types": ["text/plain", "text/html", "image/png", ...]
}
```

### SelectionOwnerChanged Options

The `options` dictionary for the signal contains:

```
{
  "mime_types": ["text/plain", ...],
  "session_is_owner": true/false
}
```

---

## Implementation Examples

### Basic RemoteDesktop Session Setup

```rust
use ashpd::desktop::{
    remote_desktop::{DeviceType, RemoteDesktop},
    clipboard::Clipboard,
    PersistMode,
};
use ashpd::WindowIdentifier;

#[tokio::main]
async fn main() -> ashpd::Result<()> {
    // Create proxies
    let remote_desktop = RemoteDesktop::new().await?;
    let clipboard = Clipboard::new().await?;

    // Create session
    let session = remote_desktop.create_session().await?;

    // Select input devices
    remote_desktop.select_devices(
        &session,
        DeviceType::Keyboard | DeviceType::Pointer,
        None,
        PersistMode::DoNot,
    ).await?;

    // Request clipboard access BEFORE starting
    clipboard.request(&session).await?;

    // Start session (shows permission dialog)
    let response = remote_desktop
        .start(&session, WindowIdentifier::default())
        .await?
        .response()?;

    println!("Session started with devices: {:?}", response.devices());

    // Session is now active with clipboard support

    Ok(())
}
```

### Handling Clipboard Copy (Setting Selection)

```rust
use ashpd::desktop::clipboard::Clipboard;
use ashpd::desktop::remote_desktop::RemoteDesktop;

async fn handle_clipboard_copy(
    clipboard: &Clipboard<'_>,
    session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
    mime_types: &[&str],
) -> ashpd::Result<()> {
    // Advertise available MIME types
    clipboard.set_selection(session, mime_types).await?;

    println!("Clipboard selection set with MIME types: {:?}", mime_types);

    Ok(())
}
```

### Handling Delayed Rendering (SelectionTransfer)

```rust
use ashpd::desktop::clipboard::Clipboard;
use futures::StreamExt;
use std::collections::HashMap;

async fn handle_selection_transfers(
    clipboard: &Clipboard<'_>,
    session: &ashpd::desktop::Session<'_, ashpd::desktop::remote_desktop::RemoteDesktop<'_>>,
) -> ashpd::Result<()> {
    // Create stream for transfer requests
    let mut transfer_stream = clipboard.receive_selection_transfer().await?;

    // Mock clipboard data store
    let mut clipboard_data: HashMap<String, Vec<u8>> = HashMap::new();
    clipboard_data.insert(
        "text/plain".to_string(),
        b"Hello from clipboard!".to_vec()
    );

    // Handle incoming transfer requests
    while let Some((transfer_session, mime_type, serial)) = transfer_stream.next().await {
        println!("Transfer requested - MIME: {}, Serial: {}", mime_type, serial);

        // Get file descriptor for writing
        match clipboard.selection_write(&transfer_session, serial).await {
            Ok(fd) => {
                // Write clipboard data to the file descriptor
                match write_clipboard_data(fd, &clipboard_data, &mime_type).await {
                    Ok(_) => {
                        // Signal successful completion
                        clipboard.selection_write_done(&transfer_session, serial, true).await?;
                        println!("Transfer completed successfully");
                    }
                    Err(e) => {
                        // Signal failure
                        clipboard.selection_write_done(&transfer_session, serial, false).await?;
                        eprintln!("Transfer failed: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to get FD for transfer: {}", e);
            }
        }
    }

    Ok(())
}

async fn write_clipboard_data(
    owned_fd: ashpd::zvariant::OwnedFd,
    data_store: &HashMap<String, Vec<u8>>,
    mime_type: &str,
) -> std::io::Result<()> {
    use std::os::unix::io::{AsRawFd, FromRawFd};
    use tokio::io::AsyncWriteExt;

    // Get clipboard data for the requested MIME type
    let data = data_store.get(mime_type)
        .ok_or_else(|| std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "MIME type not found"
        ))?;

    // Convert to tokio file for async writing
    let raw_fd = owned_fd.as_raw_fd();
    let std_file = unsafe { std::fs::File::from_raw_fd(raw_fd) };
    let mut tokio_file = tokio::fs::File::from_std(std_file);

    // Write data
    tokio_file.write_all(data).await?;
    tokio_file.flush().await?;

    Ok(())
}
```

### Monitoring Clipboard Ownership Changes

```rust
use ashpd::desktop::clipboard::Clipboard;
use futures::StreamExt;

async fn monitor_clipboard_changes(
    clipboard: &Clipboard<'_>,
) -> ashpd::Result<()> {
    let mut owner_stream = clipboard.receive_selection_owner_changed().await?;

    while let Some((session, change)) = owner_stream.next().await {
        println!("Clipboard ownership changed!");
        println!("  Session is owner: {:?}", change.session_is_owner());
        println!("  Available MIME types: {:?}", change.mime_types());

        // React to ownership change
        if change.session_is_owner() == Some(false) {
            println!("Another application owns the clipboard now");
        }
    }

    Ok(())
}
```

### Reading from System Clipboard

```rust
use ashpd::desktop::clipboard::Clipboard;
use std::os::unix::io::{AsRawFd, FromRawFd};
use tokio::io::AsyncReadExt;

async fn read_system_clipboard(
    clipboard: &Clipboard<'_>,
    session: &ashpd::desktop::Session<'_, ashpd::desktop::remote_desktop::RemoteDesktop<'_>>,
    mime_type: &str,
) -> ashpd::Result<Vec<u8>> {
    // Request clipboard data
    let fd = clipboard.selection_read(session, mime_type).await?;

    // Convert to tokio file
    let raw_fd = fd.as_raw_fd();
    let std_file = unsafe { std::fs::File::from_raw_fd(raw_fd) };
    let mut tokio_file = tokio::fs::File::from_std(std_file);

    // Read all data
    let mut buffer = Vec::new();
    tokio_file.read_to_end(&mut buffer).await?;

    Ok(buffer)
}
```

### Complete Production Example

```rust
use ashpd::desktop::{
    remote_desktop::{DeviceType, RemoteDesktop},
    clipboard::Clipboard,
    Session, PersistMode,
};
use ashpd::WindowIdentifier;
use futures::StreamExt;
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

struct ClipboardManager {
    clipboard: Clipboard<'static>,
    remote_desktop: RemoteDesktop<'static>,
    session: Session<'static, RemoteDesktop<'static>>,
    clipboard_data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl ClipboardManager {
    async fn new() -> ashpd::Result<Self> {
        let remote_desktop = RemoteDesktop::new().await?;
        let clipboard = Clipboard::new().await?;

        let session = remote_desktop.create_session().await?;

        // Configure session
        remote_desktop.select_devices(
            &session,
            DeviceType::Keyboard | DeviceType::Pointer,
            None,
            PersistMode::DoNot,
        ).await?;

        clipboard.request(&session).await?;

        let _response = remote_desktop
            .start(&session, WindowIdentifier::default())
            .await?
            .response()?;

        Ok(Self {
            clipboard,
            remote_desktop,
            session,
            clipboard_data: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    async fn set_clipboard_text(&self, text: String) -> ashpd::Result<()> {
        // Store data
        let mut data = self.clipboard_data.write().await;
        data.insert("text/plain".to_string(), text.into_bytes());
        data.insert("text/plain;charset=utf-8".to_string(), text.into_bytes());
        drop(data);

        // Advertise to system
        self.clipboard.set_selection(
            &self.session,
            &["text/plain", "text/plain;charset=utf-8"]
        ).await?;

        Ok(())
    }

    async fn get_clipboard_text(&self) -> ashpd::Result<String> {
        let fd = self.clipboard.selection_read(&self.session, "text/plain").await?;

        use std::os::unix::io::{AsRawFd, FromRawFd};
        use tokio::io::AsyncReadExt;

        let raw_fd = fd.as_raw_fd();
        let std_file = unsafe { std::fs::File::from_raw_fd(raw_fd) };
        let mut tokio_file = tokio::fs::File::from_std(std_file);

        let mut buffer = Vec::new();
        tokio_file.read_to_end(&mut buffer).await?;

        String::from_utf8(buffer)
            .map_err(|e| ashpd::Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e
            )))
    }

    async fn run_transfer_handler(&self) -> ashpd::Result<()> {
        let mut transfer_stream = self.clipboard.receive_selection_transfer().await?;

        while let Some((_session, mime_type, serial)) = transfer_stream.next().await {
            let data = self.clipboard_data.read().await;

            if let Some(content) = data.get(&mime_type) {
                match self.clipboard.selection_write(&self.session, serial).await {
                    Ok(fd) => {
                        let content_clone = content.clone();

                        tokio::spawn(async move {
                            if let Err(e) = Self::write_fd(fd, &content_clone).await {
                                eprintln!("Failed to write clipboard data: {}", e);
                            }
                        });

                        self.clipboard.selection_write_done(&self.session, serial, true).await?;
                    }
                    Err(e) => {
                        eprintln!("Failed to get FD: {}", e);
                        self.clipboard.selection_write_done(&self.session, serial, false).await?;
                    }
                }
            } else {
                self.clipboard.selection_write_done(&self.session, serial, false).await?;
            }
        }

        Ok(())
    }

    async fn write_fd(fd: ashpd::zvariant::OwnedFd, data: &[u8]) -> tokio::io::Result<()> {
        use std::os::unix::io::{AsRawFd, FromRawFd};
        use tokio::io::AsyncWriteExt;

        let raw_fd = fd.as_raw_fd();
        let std_file = unsafe { std::fs::File::from_raw_fd(raw_fd) };
        let mut tokio_file = tokio::fs::File::from_std(std_file);

        tokio_file.write_all(data).await?;
        tokio_file.flush().await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> ashpd::Result<()> {
    let manager = ClipboardManager::new().await?;

    // Spawn transfer handler
    let manager_clone = Arc::new(manager);
    let handler = tokio::spawn({
        let manager = manager_clone.clone();
        async move {
            manager.run_transfer_handler().await
        }
    });

    // Set clipboard
    manager_clone.set_clipboard_text("Test data".to_string()).await?;

    // Wait a bit then read
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    let text = manager_clone.get_clipboard_text().await?;
    println!("Read from clipboard: {}", text);

    handler.await??;

    Ok(())
}
```

---

## Important Notes and Pitfalls

### Critical Requirements

1. **Request BEFORE Start**: `clipboard.request()` MUST be called BEFORE `remote_desktop.start()`
   - Failing to do this means clipboard access will be denied
   - No runtime error - clipboard just won't work

2. **Always Call selection_write_done()**: After each `SelectionTransfer` signal
   - Track the serial number from the signal
   - Call `selection_write_done()` with same serial and success status
   - Failure to do this can cause portal to hang

3. **RemoteDesktop Dependency**: Clipboard ONLY works with RemoteDesktop sessions
   - Cannot use clipboard portal standalone
   - Session type must be `Session<'_, RemoteDesktop<'_>>`

### Lifetime Constraints

```rust
// This will NOT compile:
async fn broken_example() {
    let clipboard = Clipboard::new().await.unwrap();
    let session = {
        let rd = RemoteDesktop::new().await.unwrap();
        rd.create_session().await.unwrap()
    }; // rd dropped here

    // ERROR: session references dropped RemoteDesktop
    clipboard.request(&session).await.unwrap();
}

// Correct:
async fn correct_example() {
    let clipboard = Clipboard::new().await.unwrap();
    let rd = RemoteDesktop::new().await.unwrap(); // Keep alive
    let session = rd.create_session().await.unwrap();

    clipboard.request(&session).await.unwrap(); // OK
}
```

### File Descriptor Safety

1. **from_raw_fd is unsafe**: Always ensure FD validity
2. **FD closed on drop**: OwnedFd/File close FD automatically
3. **Don't duplicate close**: Only one owner should close the FD
4. **Use IntoRawFd to prevent close**: If you need to pass ownership elsewhere

### Known Issues

1. **MIME Type Filtering**: Some backends only request "text/plain" in SelectionTransfer regardless of what was set in SetSelection
   - Bug report: https://bugs.kde.org/show_bug.cgi?id=512075
   - Workaround: Always support "text/plain" as a fallback

2. **Multiple Requests**: The API may send multiple SelectionTransfer signals for the same paste operation
   - Each has a unique serial number
   - Handle them independently
   - Consider optimizing with batch operations in the future

3. **File Transfer MIME Type**: Special MIME type `application/vnd.portal.filetransfer`
   - Used for file transfers through the portal
   - Requires additional File Transfer portal integration

### Performance Considerations

1. **Async Everything**: All operations are async - use tokio/async-std
2. **Stream Processing**: Both signal streams should run in background tasks
3. **Large Data**: Use streaming for large clipboard data to avoid memory spikes
4. **Serial Tracking**: Maintain a map of serial → pending transfers for complex apps

### Security Considerations

1. **User Permission Required**: Portal shows permission dialog to user
2. **Session Scope**: Clipboard access limited to the session
3. **Focus Requirements**: Some compositors limit clipboard to focused app
4. **Sandbox Escape**: Clipboard is intentionally a sandbox escape mechanism

---

## References

### Official Documentation

- **ashpd Crate**: https://docs.rs/ashpd/0.12.0/ashpd/
- **Clipboard Module**: https://docs.rs/ashpd/0.12.0/ashpd/desktop/clipboard/
- **GitHub Repository**: https://github.com/bilelmoussaoui/ashpd
- **Author's Blog**: https://belmoussaoui.com/blog/20-million-portals/

### XDG Desktop Portal Specs

- **Clipboard Portal Docs**: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Clipboard.html
- **XML Specification**: https://github.com/flatpak/xdg-desktop-portal/blob/main/data/org.freedesktop.portal.Clipboard.xml
- **RemoteDesktop Portal**: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html
- **Original PR**: https://github.com/flatpak/xdg-desktop-portal/pull/852

### Rust Dependencies

- **zbus**: https://docs.rs/zbus/latest/zbus/
- **zvariant**: https://docs.rs/zvariant/latest/zvariant/
- **zvariant::OwnedFd**: https://docs.rs/zvariant/latest/zvariant/struct.OwnedFd.html
- **std::os::fd**: https://doc.rust-lang.org/std/os/fd/

### Related Projects

- **RustDesk**: Remote desktop in Rust (uses clipboard)
- **IronRDP**: RDP implementation in Rust
- **Teleport RFD**: Desktop clipboard design doc

---

## Version Information

- **Document Version**: 1.0
- **ashpd Version**: 0.12.0
- **Research Date**: 2025-11-19
- **XDG Portal Version**: 1.18+ (Clipboard portal added in 1.18)

---

## Appendix: Quick Reference

### Import Statements

```rust
use ashpd::desktop::{
    clipboard::{Clipboard, SelectionOwnerChanged},
    remote_desktop::{DeviceType, RemoteDesktop},
    Session, PersistMode,
};
use ashpd::{WindowIdentifier, zvariant::OwnedFd};
use futures::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::os::unix::io::{AsRawFd, FromRawFd};
```

### Cargo Dependencies

```toml
[dependencies]
ashpd = "0.12.0"
tokio = { version = "1", features = ["full"] }
futures = "0.3"
```

### Method Call Order

```
1. RemoteDesktop::new()
2. RemoteDesktop::create_session()
3. RemoteDesktop::select_devices()
4. Clipboard::new()
5. Clipboard::request()              ← BEFORE start!
6. RemoteDesktop::start()
7. Clipboard::set_selection()         ← When copying
8. Handle SelectionTransfer stream    ← When pasting (delayed)
9. Clipboard::selection_read()        ← Read from system
```

### Error Handling

```rust
match clipboard.request(&session).await {
    Ok(_) => println!("Clipboard access granted"),
    Err(ashpd::Error::Response(err)) => {
        // User denied permission
        eprintln!("User denied clipboard access: {:?}", err);
    }
    Err(e) => {
        // Other error (DBus, connection, etc.)
        eprintln!("Clipboard error: {}", e);
    }
}
```

---

**End of Document**
