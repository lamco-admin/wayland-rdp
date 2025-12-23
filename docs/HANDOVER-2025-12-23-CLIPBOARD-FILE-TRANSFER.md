# Session Handover: Clipboard File Transfer Implementation
## Date: 2025-12-23

This document provides a comprehensive handover for continuing development of the lamco-rdp-server clipboard file transfer feature.

---

## Executive Summary

**Major Achievement**: Bidirectional clipboard file transfer is now working between Linux (server) and Windows (client).

| Direction | Status | Tested |
|-----------|--------|--------|
| Windows → Linux | Working | Copy file in Windows Explorer, paste in Dolphin |
| Linux → Windows | Working | Copy file in Dolphin, paste in Windows Explorer |

---

## Session Work Completed

### 1. IronRDP Patch: SendFileContentsResponse

**Repository**: https://github.com/glamberson/IronRDP
**Branch**: `cliprdr-request-file-contents`
**Commit**: `8462968b`

Added `SendFileContentsResponse` variant to enable servers to respond to client file content requests:

```rust
// ironrdp-cliprdr/src/backend.rs
pub enum ClipboardMessage {
    // ... existing variants ...

    /// Sent by clipboard backend when file contents are ready to be sent to the remote.
    ///
    /// Server implementation should send file contents response on `CLIPRDR` SVC when this
    /// message is received. This is used to respond to a FileContentsRequest from the client.
    SendFileContentsResponse(FileContentsResponse<'static>),
}
```

**Handler** in `ironrdp-server/src/server.rs:576`:
```rust
ClipboardMessage::SendFileContentsResponse(response) => {
    cliprdr.submit_file_contents(response)
}
```

### 2. lamco-clipboard-core: FileGroupDescriptorW Support

**Repository**: https://github.com/lamco-admin/lamco-rdp
**Commit**: `fc9646b`

Added complete FileGroupDescriptorW parsing/building:

- `FileDescriptor` struct - parses/builds 592-byte FILEDESCRIPTORW
- `FileDescriptorFlags` - flags indicating valid metadata fields
- `sanitize.rs` module - cross-platform filename sanitization
- `CF_FILEGROUPDESCRIPTORW` / `CF_FILECONTENTS` format constants

Key structures:
```rust
// FileGroupDescriptorW format:
// - 4 bytes: cItems (file count)
// - N * 592 bytes: FILEDESCRIPTORW structures

pub struct FileDescriptor {
    pub flags: FileDescriptorFlags,
    pub attributes: u32,
    pub creation_time: Option<u64>,
    pub access_time: Option<u64>,
    pub write_time: Option<u64>,
    pub size: Option<u64>,
    pub name: String,  // UTF-16 decoded
}
```

### 3. wrd-server-specs: Clipboard Manager Updates

**Repository**: https://github.com/lamco-admin/wayland-rdp
**Commit**: `cd418ac`

Updated `src/clipboard/manager.rs` to:

1. **Handle FileContentsRequest from Windows client** (~line 2299):
   - Parse stream_id, list_index, position, requested bytes
   - Look up file from pending transfer list
   - Return size (8 bytes) or data chunk (up to 65536 bytes)
   - Send response via `ServerEvent::Clipboard(ClipboardMessage::SendFileContentsResponse(...))`

2. **Build FileGroupDescriptorW for Linux→Windows** (~line 1800):
   - Parse `text/uri-list` from portal clipboard
   - Build file descriptors using `FileDescriptor::build()`
   - Store file paths for later content requests

---

## Architecture Overview

### File Transfer Flow: Windows → Linux

```
┌─────────────────┐                              ┌─────────────────┐
│  Windows Client │                              │  Linux Server   │
│  (mstsc.exe)    │                              │ (lamco-rdp)     │
└────────┬────────┘                              └────────┬────────┘
         │                                                │
         │  1. FormatListPDU (FileGroupDescriptorW)       │
         │────────────────────────────────────────────────>
         │                                                │
         │  2. FormatListResponse                         │
         │<────────────────────────────────────────────────
         │                                                │
         │  3. FormatDataRequest (FileGroupDescriptorW)   │
         │<────────────────────────────────────────────────
         │                                                │
         │  4. FormatDataResponse (file list metadata)    │
         │────────────────────────────────────────────────>
         │                                                │
         │  5. FileContentsRequest (SIZE, index=0)        │
         │<────────────────────────────────────────────────
         │                                                │
         │  6. FileContentsResponse (8 bytes, file size)  │
         │────────────────────────────────────────────────>
         │                                                │
         │  7. FileContentsRequest (DATA, pos=0, len=64K) │
         │<────────────────────────────────────────────────
         │                                                │
         │  8. FileContentsResponse (data chunk)          │
         │────────────────────────────────────────────────>
         │     ... repeat until complete ...              │
         │                                                │
```

### File Transfer Flow: Linux → Windows

```
┌─────────────────┐                              ┌─────────────────┐
│  Linux Server   │                              │  Windows Client │
│  (lamco-rdp)    │                              │  (mstsc.exe)    │
└────────┬────────┘                              └────────┬────────┘
         │                                                │
         │  1. FormatListPDU (FileGroupDescriptorW)       │
         │────────────────────────────────────────────────>
         │                                                │
         │  2. FormatListResponse                         │
         │<────────────────────────────────────────────────
         │                                                │
         │  3. FormatDataRequest (FileGroupDescriptorW)   │
         │<────────────────────────────────────────────────
         │                                                │
         │  4. FormatDataResponse (FileDescriptor list)   │
         │────────────────────────────────────────────────>
         │                                                │
         │  5. FileContentsRequest (SIZE, index=0)        │
         │<────────────────────────────────────────────────
         │                                                │
         │  6. FileContentsResponse (size in 8 bytes)     │
         │────────────────────────────────────────────────>
         │                                                │
         │  7. FileContentsRequest (DATA, pos=0, len=64K) │
         │<────────────────────────────────────────────────
         │                                                │
         │  8. FileContentsResponse (file data)           │
         │────────────────────────────────────────────────>
         │     ... repeat for each chunk ...              │
```

---

## Current Repository State

### IronRDP Fork (glamberson/IronRDP)

| Branch | Purpose | Status |
|--------|---------|--------|
| `cliprdr-request-file-contents` | CLIPRDR file transfer | Pushed, needs PR |
| `egfx-server-complete` | EGFX graphics | PR #1057 open |

**All patches on `cliprdr-request-file-contents`**:
1. `SendFileContentsRequest` - server requests file data from client
2. `SendFileContentsResponse` - server sends file data to client (NEW)
3. `reqwest` feature enabled in `ironrdp-server`

### lamco-rdp-workspace (lamco-admin/lamco-rdp)

| Crate | Changes |
|-------|---------|
| `lamco-clipboard-core` | FileDescriptor, sanitize.rs, format constants |

**Dependency**: Uses local IronRDP path for development (should change to git before publish)

### wrd-server-specs (lamco-admin/wayland-rdp)

This is the main server application. All clipboard file transfer logic is in:
- `src/clipboard/manager.rs` - ClipboardManager with file transfer state
- `src/clipboard/ironrdp_backend.rs` - IronRDP integration

---

## Pending Work

### High Priority: Create IronRDP PR

The `cliprdr-request-file-contents` branch needs to be submitted as a PR to Devolutions/IronRDP.

**PR Content**:
1. `SendFileContentsRequest` message variant (existing)
2. `SendFileContentsResponse` message variant (new)
3. `request_file_contents()` method
4. Handler in `ironrdp-server`

**Requirements** (per IronRDP STYLE.md):
- [ ] Doc comments with MS-RDPECLIP references
- [ ] Follow Core Tier rules (no I/O, no_std compatible)
- [ ] Add tests
- [ ] Pass CI

### Medium Priority: Publish lamco Crates

After IronRDP PR is merged:
1. Update lamco-rdp-workspace to use published IronRDP version
2. Publish lamco-clipboard-core with FileDescriptor support
3. Update lamco-rdp-server to use crates.io versions

### Low Priority: Minor Issues

**PipeWire Stride Mismatch** (transient, recovers):
```
WARN pipewire::stream: Failed to process frame: buffer stride mismatch
```
- Occurs briefly during resolution changes
- Self-recovers within 1-2 frames
- Not a blocking issue

**Graphics Queue Backpressure**:
```
WARN: Graphics queue full (100), dropping frame
```
- Intentional QoS mechanism
- Drops frames when client can't keep up
- Working as designed

**Duplicate RemoteCopy Events**:
```
🔔 RemoteCopy event: formats changed
📎 Advertised remote clipboard formats to client
🔔 RemoteCopy event: formats changed  <-- duplicate
```
- Minor logging noise
- Doesn't affect functionality
- Could deduplicate in future

---

## Testing the Implementation

### Prerequisites

1. Linux server with:
   - Wayland compositor (KDE Plasma tested)
   - PipeWire
   - Portal support (xdg-desktop-portal)

2. Windows client with:
   - mstsc.exe (built-in RDP client)
   - Clipboard redirection enabled

### Test: Windows → Linux

1. On Windows: Copy a file in Explorer (Ctrl+C)
2. On Linux: Open Dolphin file manager
3. On Linux: Paste (Ctrl+V)
4. Verify: File appears with correct content

### Test: Linux → Windows

1. On Linux: Copy a file in Dolphin (Ctrl+C)
2. On Windows: Open Explorer
3. On Windows: Paste (Ctrl+V)
4. Verify: File appears with correct content

### Logs to Watch

```bash
# Run server with debug logging
RUST_LOG=debug ./lamco-rdp-server

# Key log messages:
# Windows → Linux:
📋 Received FormatDataResponse with FileGroupDescriptorW
📂 Parsed N files from FileGroupDescriptorW
📥 FileContentsRequest: stream_id=X, index=Y, pos=Z, len=W
📤 Sending FileContentsResponse: size=X

# Linux → Windows:
🔔 RemoteCopy event: formats changed
📁 Building FileGroupDescriptorW for N files
📤 Sending FormatDataResponse with FileGroupDescriptorW
📥 Received FileContentsRequest
📤 Sending FileContentsResponse(stream=X, size=Y)
```

---

## Key Code References

### IronRDP Changes

| File | Line | Description |
|------|------|-------------|
| `ironrdp-cliprdr/src/backend.rs` | ~74 | `SendFileContentsResponse` variant |
| `ironrdp-server/src/server.rs` | ~576 | Handler for SendFileContentsResponse |
| `ironrdp-server/Cargo.toml` | ~42 | `reqwest` feature enabled |

### lamco-clipboard-core Changes

| File | Line | Description |
|------|------|-------------|
| `src/formats.rs` | ~525 | `FileDescriptor` struct |
| `src/formats.rs` | ~640 | `FileDescriptor::parse()` |
| `src/formats.rs` | ~720 | `FileDescriptor::build()` |
| `src/sanitize.rs` | ~72 | `sanitize_filename_for_windows()` |

### wrd-server-specs Changes

| File | Line | Description |
|------|------|-------------|
| `src/clipboard/manager.rs` | ~2299 | `handle_rdp_file_contents_request()` |
| `src/clipboard/manager.rs` | ~1800 | FileGroupDescriptorW building |
| `src/clipboard/manager.rs` | ~500 | `PendingFileTransfer` state |

---

## Dependency Configuration

### For Development (current)

```toml
# wrd-server-specs/Cargo.toml
[dependencies]
ironrdp = { path = "/home/greg/wayland/IronRDP/crates/ironrdp", ... }
ironrdp-server = { path = "/home/greg/wayland/IronRDP/crates/ironrdp-server" }
ironrdp-cliprdr = { path = "/home/greg/wayland/IronRDP/crates/ironrdp-cliprdr" }

[patch.crates-io]
ironrdp = { path = "/home/greg/wayland/IronRDP/crates/ironrdp" }
# ... all IronRDP crates
```

### For Release (after IronRDP PR merged)

```toml
# wrd-server-specs/Cargo.toml
[dependencies]
ironrdp = { version = "0.X", features = ["server"] }
ironrdp-server = "0.X"
ironrdp-cliprdr = "0.X"

# No [patch] section needed
```

---

## Important: Crypto Provider Configuration

The `ironrdp-tokio` crate must use the `reqwest` feature (NOT `reqwest-rustls-ring`):

```toml
# CORRECT - uses native TLS (OpenSSL)
ironrdp-tokio = { ..., features = ["reqwest"] }

# WRONG - causes rustls crypto provider conflict
ironrdp-tokio = { ..., features = ["reqwest-rustls-ring"] }
```

**Why**: `reqwest-rustls-ring` brings in the `ring` crypto backend, but `sspi` (a transitive dependency) brings in `aws-lc-rs`. Having both causes a panic:
```
Could not automatically determine the process-level CryptoProvider
```

The `reqwest` feature uses native TLS (OpenSSL on Linux) which avoids rustls entirely.

---

## Next Session Checklist

1. [ ] Create PR from `cliprdr-request-file-contents` branch to Devolutions/IronRDP
2. [ ] Wait for PR review/merge
3. [ ] Update lamco-rdp-workspace dependencies to use published IronRDP
4. [ ] Publish lamco-clipboard-core with FileDescriptor support
5. [ ] Update wrd-server-specs to use crates.io dependencies
6. [ ] Create clean lamco-rdp-server repo for commercial release
7. [ ] Address minor issues (stride mismatch, duplicate events)

---

## Contact & Resources

- **IronRDP Upstream**: https://github.com/Devolutions/IronRDP
- **IronRDP Fork**: https://github.com/glamberson/IronRDP
- **MS-RDPECLIP Spec**: [MS-RDPECLIP](https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpeclip/)
- **FileGroupDescriptorW**: Section 2.2.5.2.3.1 of MS-RDPECLIP

---

*Document generated: 2025-12-23*
*Session: Clipboard File Transfer Implementation*
