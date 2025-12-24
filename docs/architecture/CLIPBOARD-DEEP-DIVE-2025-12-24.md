# Clipboard Architecture Deep Dive

**Date:** 2025-12-24
**Status:** Research Complete
**Purpose:** Comprehensive analysis of MS-RDPECLIP protocol, XDG Portal Clipboard API, and current implementation to verify architecture boundaries and identify future development opportunities.

## Executive Summary

The lamco-rdp-server clipboard implementation is architecturally sound, with clean separation between:
- **Protocol-agnostic logic** (`lamco-clipboard-core`) - format conversion, sanitization, file descriptors
- **IronRDP integration** (`lamco-rdp-clipboard`) - CliprdrBackend trait, event bridging
- **Portal integration** (`lamco-portal`) - D-Bus clipboard, delayed rendering
- **Server orchestration** (`wrd-server-specs/src/clipboard/`) - state machine, coordination

Current implementation supports bidirectional clipboard for:
- ✅ Text (UTF-8, all encodings)
- ✅ Images (DIB ↔ PNG/JPEG/BMP)
- ✅ Files (>64MB supported with chunked transfer)
- ✅ HTML content

---

## 1. MS-RDPECLIP Protocol Analysis

### 1.1 Protocol Overview

MS-RDPECLIP is a Static Virtual Channel (SVC) extension for RDP clipboard sharing. The implementation follows the [MS-RDPECLIP] specification from Microsoft.

### 1.2 Capability Flags

| Flag | Value | Our Implementation | Notes |
|------|-------|-------------------|-------|
| CB_USE_LONG_FORMAT_NAMES | 0x0002 | ✅ Enabled | Required for custom format names |
| CB_STREAM_FILECLIP_ENABLED | 0x0004 | ✅ Enabled | File streaming via FileContents PDUs |
| CB_FILECLIP_NO_FILE_PATHS | 0x0008 | ❌ Not set | We support file paths |
| CB_CAN_LOCK_CLIPDATA | 0x0010 | ✅ Enabled | Data locking for large transfers |
| CB_HUGE_FILE_SUPPORT_ENABLED | 0x0020 | ⚠️ Implicit | 64-bit file sizes via chunk continuation |

### 1.3 PDU Flow (Implemented)

```
Server                          Client (Windows)
   │                                  │
   ├──── Capabilities ───────────────►│
   │◄─── Capabilities ─────────────────┤
   │                                  │
   │◄─── Format List ──────────────────┤  (Windows copies)
   ├──── Format List Response ────────►│
   │                                  │
   ├──── Format Data Request ─────────►│  (Linux pastes)
   │◄─── Format Data Response ─────────┤
   │                                  │
   │◄─── FileContentsRequest ──────────┤  (Linux → Windows file)
   ├──── FileContentsResponse ────────►│
   │                                  │
   ├──── Lock ────────────────────────►│  (Multi-file transfer)
   ├──── Unlock ──────────────────────►│
```

### 1.4 FileContentsRequest/Response (Critical for File Transfer)

**FILECONTENTS_SIZE (dwFlags = 0x0001)**
- Client requests file size only
- Response: 8 bytes (UINT64 file size)

**FILECONTENTS_RANGE (dwFlags = 0x0002)**
- Client requests file data chunk
- Request specifies: stream_id, list_index, position, requested_size
- Response: actual file bytes (up to 65535 bytes per chunk)

**Our Implementation:**
```rust
// manager.rs:114-122
RdpFileContentsRequest {
    stream_id: u32,
    list_index: u32,
    position: u64,
    size: u32,
    is_size_request: bool,
}
```

### 1.5 CLIPRDR_FILEDESCRIPTOR Structure

592-byte structure per file:
| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| 0 | 4 | dwFlags | FD_ATTRIBUTES, FD_FILESIZE, etc. |
| 4 | 32 | reserved | Reserved |
| 36 | 4 | dwFileAttributes | FILE_ATTRIBUTE_* flags |
| ... | ... | timestamps | Create/Access/Write times |
| 76 | 4 | nFileSizeHigh | High 32-bits of size |
| 80 | 4 | nFileSizeLow | Low 32-bits of size |
| 84 | 520 | cFileName | UTF-16LE filename (260 chars) |

**Our Implementation:** `lamco-clipboard-core/src/formats.rs:215-280`

---

## 2. XDG Desktop Portal Clipboard API Analysis

### 2.1 Interface Overview

D-Bus interface: `org.freedesktop.portal.Clipboard`

### 2.2 Methods

| Method | Parameters | Return | Our Usage |
|--------|------------|--------|-----------|
| RequestClipboard | session_handle | - | ✅ `enable_for_session()` |
| SetSelection | session, mime_types | - | ✅ `announce_rdp_formats()` |
| SelectionWrite | session, serial | fd | ✅ `write_selection_data()` |
| SelectionWriteDone | session, serial, success | - | ✅ Called after write |
| SelectionRead | session, mime_type | fd | ✅ `read_local_clipboard()` |

### 2.3 Signals

| Signal | Parameters | Our Handling |
|--------|------------|--------------|
| SelectionOwnerChanged | session, options | ⚠️ Works on Mutter, NOT on GNOME |
| SelectionTransfer | session, mime_type, serial | ✅ `start_selection_transfer_listener()` |

### 2.4 Critical Discovery: GNOME Portal Gap

**Problem:** GNOME's Portal implementation does NOT emit `SelectionOwnerChanged` signals.

**Solution:** GNOME Shell Extension (`extension/extension.js`) that:
1. Polls St.Clipboard at 500ms intervals
2. Uses `get_mimetypes()` for actual MIME types (fixed 2025-12-23)
3. Emits D-Bus signal `org.wayland_rdp.Clipboard.ClipboardChanged`
4. Server listens via `DbusClipboardBridge`

### 2.5 Non-Blocking FD Issue (Fixed 2025-12-24)

Portal's `SelectionRead()` returns non-blocking pipe FDs. Direct read fails with EAGAIN.

**Fix Applied:**
```rust
// lamco-portal/src/clipboard.rs:299-307
let raw_fd = std_file.as_raw_fd();
unsafe {
    let flags = libc::fcntl(raw_fd, libc::F_GETFL);
    if flags != -1 {
        libc::fcntl(raw_fd, libc::F_SETFL, flags & !libc::O_NONBLOCK);
    }
}
```

---

## 3. Architecture Boundary Analysis

### 3.1 Crate Responsibility Map

```
┌─────────────────────────────────────────────────────────────────────┐
│                     wrd-server-specs (Application)                   │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │ src/clipboard/                                                   ││
│  │   manager.rs    - Orchestration, state machine                   ││
│  │   sync.rs       - ClipboardState enum, SyncManager               ││
│  │   dbus.rs       - GNOME extension bridge                         ││
│  │   mod.rs        - Format conversion extension traits             ││
│  └─────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
                                   │
         ┌─────────────────────────┼─────────────────────────┐
         ▼                         ▼                         ▼
┌─────────────────┐      ┌─────────────────┐      ┌─────────────────┐
│ lamco-clipboard │      │ lamco-rdp-      │      │ lamco-portal    │
│ -core           │      │ clipboard       │      │                 │
│                 │      │                 │      │                 │
│ • FormatConvert │      │ • CliprdrBackend│      │ • Portal D-Bus  │
│ • TransferEngine│      │   trait impl    │      │ • Selection API │
│ • FileDescriptor│      │ • Event bridge  │      │ • Clipboard mgr │
│ • Sanitization  │      │ • Factory       │      │ • FD handling   │
│ • DIB/PNG       │      │                 │      │                 │
└─────────────────┘      └─────────────────┘      └─────────────────┘
         │                         │
         └──────────┬──────────────┘
                    ▼
          ┌─────────────────┐
          │ ironrdp-cliprdr │
          │ (upstream)      │
          │                 │
          │ • CLIPRDR SVC   │
          │ • PDU types     │
          │ • Backend trait │
          └─────────────────┘
```

### 3.2 Boundary Violations: NONE

All boundaries are properly maintained:

| Boundary | Expected | Actual | Status |
|----------|----------|--------|--------|
| lamco-clipboard-core | No RDP knowledge | ✅ Only format conversion, no ironrdp imports | ✅ |
| lamco-rdp-clipboard | Only ironrdp bridge | ✅ CliprdrBackend + events only | ✅ |
| lamco-portal | Only Portal D-Bus | ✅ No RDP knowledge, just FD/selection | ✅ |
| wrd-server-specs | Orchestration only | ✅ Coordinates between crates | ✅ |

### 3.3 Dependency Direction

```
wrd-server-specs
    ├── lamco-clipboard-core
    ├── lamco-rdp-clipboard ──► ironrdp-cliprdr (patched)
    └── lamco-portal ──► ashpd, zbus
```

**Note:** `lamco-rdp-clipboard` MUST use local path because it implements `CliprdrBackend` from patched ironrdp-cliprdr. If using crates.io version, trait would mismatch.

---

## 4. Current Feature Status

### 4.1 Fully Implemented

| Feature | Direction | Max Size | Notes |
|---------|-----------|----------|-------|
| Text (UTF-8) | Bidirectional | 16MB | UTF-16LE ↔ UTF-8 conversion |
| Text (other) | Bidirectional | 16MB | CF_TEXT, STRING, etc. |
| Images (PNG) | Bidirectional | 16MB | DIB ↔ PNG conversion |
| Images (JPEG) | Bidirectional | 16MB | DIB ↔ JPEG conversion |
| Images (BMP) | Bidirectional | 16MB | Direct DIB support |
| HTML | Bidirectional | 16MB | HTML Fragment format |
| Files (single) | Bidirectional | >64MB | Chunked transfer |
| Files (multiple) | Bidirectional | >64MB | FileGroupDescriptorW |

### 4.2 Known Limitations

1. **RTF Format**: Declared but not implemented (`enable_rtf` config exists but unused)
2. **Primary Selection**: GNOME extension supports it, but RDP has no equivalent
3. **Custom Formats**: Framework exists but no custom format registration

---

## 5. Protocol Gaps and Future Opportunities

### 5.1 Features in MS-RDPECLIP Not Yet Implemented

| Feature | Spec Section | Complexity | Value |
|---------|--------------|------------|-------|
| **CB_FILECLIP_NO_FILE_PATHS** | 2.2.2.1 | Low | Privacy - don't leak paths |
| **CLIPRDR_TEMP_DIRECTORY** | 2.2.2.3 | Medium | Custom temp dir negotiation |
| **Synthesized Formats** | 2.2.3.1 | High | Auto-convert CF_DIB → CF_DIBV5 |
| **Palette Data** | 2.2.3.2 | Low | Legacy 8-bit color support |
| **Metafile** | 2.2.5 | Medium | EMF/WMF graphics format |
| **CLIPRDR_LOCK/UNLOCK** | 2.2.6 | ✅ Done | Multi-file atomic transfer |

### 5.2 Potential New Features

#### 5.2.1 Rich Text Format (RTF)
- **Effort:** Medium
- **Value:** High for office document interop
- **Implementation:** Add RTF ↔ HTML bidirectional conversion

#### 5.2.2 Clipboard History/Manager
- **Effort:** Medium
- **Value:** Medium (UX improvement)
- **Implementation:** Server-side clipboard ring buffer, expose via D-Bus

#### 5.2.3 Format Filtering by Policy
- **Effort:** Low
- **Value:** High for enterprise security
- **Implementation:** Config-based format whitelist/blacklist

#### 5.2.4 Compression for Large Data
- **Effort:** Medium
- **Value:** Medium (performance)
- **Implementation:** LZ4/zstd compression for >1MB payloads

#### 5.2.5 Clipboard Audit Logging
- **Effort:** Low
- **Value:** High for compliance
- **Implementation:** Structured logging of all clipboard operations

### 5.3 Portal API Opportunities

| Portal Method | Current Usage | Potential Enhancement |
|--------------|---------------|----------------------|
| SelectionRead | ✅ Full | Could batch-read multiple MIME types |
| SelectionWrite | ✅ Full | - |
| Request | ✅ Full | - |

---

## 6. Recommendations

### 6.1 Short-Term (Next Sprint)

1. **Add RTF Support**: Complete the `enable_rtf` feature path
2. **Add Format Filtering**: Enterprise security requirement
3. **Audit Logging**: Add structured tracing for clipboard ops

### 6.2 Medium-Term (Q1 2025)

1. **Optimize Multi-File Transfer**: Batch FileContentsRequest
2. **Add Compression**: For large clipboard payloads
3. **Clipboard History**: D-Bus interface for history access

### 6.3 Long-Term

1. **Enhanced Graphics Formats**: EMF/WMF support for vector graphics
2. **Custom Format Registration**: Allow apps to register custom formats
3. **Multi-Monitor Clipboard Contexts**: Different clipboards per display

---

## 7. Test Coverage Gaps

| Component | Unit Tests | Integration Tests | Recommendation |
|-----------|------------|-------------------|----------------|
| FormatConverter | ✅ Good | ⚠️ Limited | Add cross-format roundtrip tests |
| TransferEngine | ✅ Good | ⚠️ Limited | Add >64MB file transfer tests |
| DIB conversion | ✅ Good | ⚠️ Limited | Add 1-bit, 4-bit, 8-bit palette tests |
| ClipboardManager | ⚠️ Limited | ⚠️ Limited | Add state machine tests |
| Portal integration | ❌ None | ⚠️ Manual | Consider mock Portal for CI |

---

## 8. Conclusion

The clipboard implementation is architecturally clean and functionally complete for core use cases. The crate boundaries are well-maintained with proper separation of concerns.

**Key Strengths:**
- Clean abstraction layers
- Comprehensive format support
- Robust file transfer with >64MB support
- GNOME extension workaround for Portal limitations

**Areas for Growth:**
- RTF format support
- Compression for performance
- Format filtering for enterprise security
- Audit logging for compliance

The foundation is solid for incremental feature additions without architectural changes.
