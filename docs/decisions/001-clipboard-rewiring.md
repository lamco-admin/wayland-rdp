# ADR-001: Clipboard Module Rewiring

**Date**: 2025-12-17
**Status**: Approved
**Deciders**: Greg (product owner)

## Context

The lamco-rdp-server clipboard module contains ~5,700 LOC that duplicates functionality from the recently-published lamco crate ecosystem:

- `lamco-clipboard-core` - Protocol-agnostic clipboard utilities
- `lamco-portal` - XDG Desktop Portal integration (with D-Bus bridge)
- `lamco-rdp-clipboard` - IronRDP clipboard backend

The goal is to reduce server clipboard code to thin orchestration glue (~600 LOC) by reusing the published crates.

## Decision

### Files to REPLACE with Library Imports

| Server File | LOC | Replacement | Rationale |
|-------------|-----|-------------|-----------|
| `formats.rs` | 980 | `lamco-clipboard-core::formats` | Identical functionality - MIME↔RDP format conversion, ClipboardFormat struct, format constants |
| `transfer.rs` | 608 | `lamco-clipboard-core::transfer` | Identical functionality - TransferEngine, chunked transfer, progress tracking, SHA256 verification |
| `dbus_bridge.rs` | 346 | `lamco-portal::dbus_clipboard` | Identical functionality - DbusClipboardBridge, ClipboardChanged signal handling |
| `ironrdp_backend.rs` | 435 | `lamco-rdp-clipboard::backend` | Mostly identical - RdpCliprdrBackend, non-blocking event queue |

**Total Removed**: ~2,369 LOC

### Files to PARTIALLY REFACTOR

| Server File | LOC | Keep | Replace | Rationale |
|-------------|-----|------|---------|-----------|
| `sync.rs` | 818 | `SyncManager`, `ClipboardState` enum | `LoopDetector` | Server's SyncManager has sophisticated state machine (Idle, RdpOwned, PortalOwned, Syncing) with echo protection. LoopDetector from library provides core hashing primitives. |
| `error.rs` | 446 | `ErrorContext`, `RecoveryAction`, `RetryConfig`, `recovery_action()` | Base `ClipboardError` variants | Error recovery policy is server-specific. Base error types come from library. |

**LOC After Refactor**: ~400 LOC (down from ~1,264)

### Files to REFACTOR (Keep but Simplify)

| Server File | LOC | Changes | Target LOC |
|-------------|-----|---------|------------|
| `manager.rs` | 1,954 | Update imports to use library types. Remove duplicate type definitions. Orchestration logic stays. | ~800 |
| `mod.rs` | 129 | Update re-exports to point to library types | ~30 |

### Server-Specific Code (MUST STAY)

1. **SyncManager State Machine** (`sync.rs`)
   - `ClipboardState` enum: `Idle`, `RdpOwned(timestamp)`, `PortalOwned`, `Syncing`
   - Echo protection: 2-second blocking window after RDP ownership changes
   - State-based blocking decisions that go beyond simple loop detection
   - This is orchestration policy, not a reusable primitive

2. **Error Recovery Policy** (`error.rs`)
   - `ErrorContext` - Server-specific context for error recovery decisions
   - `RecoveryAction` enum - Server policy: Retry, Skip, RequestNewSession, etc.
   - `recovery_action()` - Policy function mapping errors to recovery strategies
   - `RetryConfig` with backoff - Server timeout/retry configuration

3. **ClipboardConfig** (`manager.rs`)
   - Server-specific configuration (max sizes, timeouts, feature toggles)
   - Can't be generalized to library level

4. **WrdCliprdrFactory** (`ironrdp_backend.rs`)
   - Server-specific factory that wraps `RdpCliprdrFactory`
   - Integrates with server's `ClipboardManager`
   - Handles server-specific event routing

## Architectural Diagram

```
BEFORE (5,700 LOC):
┌─────────────────────────────────────────────────────────┐
│                  wrd-server clipboard/                   │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌─────────┐  │
│  │ formats   │ │ transfer  │ │dbus_bridge│ │ backend │  │
│  │  980 LOC  │ │  608 LOC  │ │  346 LOC  │ │ 435 LOC │  │
│  └───────────┘ └───────────┘ └───────────┘ └─────────┘  │
│  ┌───────────┐ ┌───────────┐ ┌─────────────────────────┐│
│  │  error    │ │   sync    │ │        manager          ││
│  │  446 LOC  │ │  818 LOC  │ │       1,954 LOC         ││
│  └───────────┘ └───────────┘ └─────────────────────────┘│
└─────────────────────────────────────────────────────────┘

AFTER (~600 LOC):
┌─────────────────────────────────────────────────────────┐
│                  wrd-server clipboard/                   │
│  ┌─────────────────────────────────────────────────────┐│
│  │              manager.rs (~800 LOC)                  ││
│  │   - ClipboardManager orchestration                  ││
│  │   - ClipboardConfig                                 ││
│  │   - Event routing between Portal and RDP           ││
│  └─────────────────────────────────────────────────────┘│
│  ┌─────────────────────────┐ ┌─────────────────────────┐│
│  │    sync.rs (~400 LOC)   │ │  error.rs (~200 LOC)    ││
│  │ - SyncManager state     │ │ - ErrorContext          ││
│  │ - ClipboardState        │ │ - RecoveryAction        ││
│  │ - Echo protection       │ │ - Server error policy   ││
│  └─────────────────────────┘ └─────────────────────────┘│
│                        imports                          │
│  ┌─────────────────────────────────────────────────────┐│
│  │ lamco-clipboard-core                                ││
│  │   FormatConverter, ClipboardFormat, LoopDetector,   ││
│  │   TransferEngine, ClipboardError                    ││
│  ├─────────────────────────────────────────────────────┤│
│  │ lamco-portal                                        ││
│  │   DbusClipboardBridge, DbusClipboardEvent           ││
│  ├─────────────────────────────────────────────────────┤│
│  │ lamco-rdp-clipboard                                 ││
│  │   RdpCliprdrBackend, RdpCliprdrFactory,             ││
│  │   ClipboardEvent, ClipboardEventReceiver            ││
│  └─────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
```

## Import Structure After Rewiring

```rust
// manager.rs imports

// From lamco-clipboard-core
use lamco_clipboard_core::{
    ClipboardFormat,
    FormatConverter,
    LoopDetector,
    LoopDetectionConfig,
    TransferEngine,
    TransferConfig,
    ClipboardError as CoreClipboardError,
};

// From lamco-portal
use lamco_portal::dbus_clipboard::{DbusClipboardBridge, DbusClipboardEvent};

// From lamco-rdp-clipboard
use lamco_rdp_clipboard::{
    RdpCliprdrBackend,
    RdpCliprdrFactory,
    ClipboardEvent,
    ClipboardEventReceiver,
    ClipboardGeneralCapabilityFlags,
};

// Server-specific (from this crate)
use crate::clipboard::sync::{SyncManager, ClipboardState};
use crate::clipboard::error::{ErrorContext, RecoveryAction, recovery_action};
```

## Data Flow (Unchanged)

### Windows → Linux (Paste)

```
RDP Client copies
    ↓
IronRDP CLIPRDR FormatList PDU
    ↓
RdpCliprdrBackend.on_remote_copy() → ClipboardEvent::RemoteCopy
    ↓
Server manager receives event
    ↓
SyncManager.set_rdp_owned() - Track state
    ↓
LoopDetector.record_operation() - Record for loop detection
    ↓
FormatConverter.rdp_to_mime_types()
    ↓
Portal SetSelection (delayed rendering - announce formats only)
    ↓
User pastes in Linux app → SelectionTransfer signal
    ↓
Server requests data from RDP via FormatDataRequest
    ↓
RdpCliprdrBackend.on_format_data_response()
    ↓
FormatConverter.convert_from_rdp()
    ↓
Portal SelectionWrite (provide data)
```

### Linux → Windows (Copy)

```
Wayland clipboard change
    ↓
DbusClipboardBridge.subscribe() receives DbusClipboardEvent
    ↓
SyncManager.check_echo_protection() - Filter echoes
    ↓
LoopDetector.would_cause_loop() check
    ↓
If not loop: FormatConverter.mime_to_rdp_formats()
    ↓
SyncManager.set_portal_owned() - Track state
    ↓
Send FormatListPDU via IronRDP ServerEvent channel
    ↓
RDP client pastes → FormatDataRequest
    ↓
RdpCliprdrBackend.on_format_data_request()
    ↓
Server fetches from Portal via selection_read()
    ↓
FormatConverter.convert_to_rdp()
    ↓
Send FormatDataResponse via IronRDP
```

## IronRDP Dependency

The libraries currently use IronRDP from git branch `master` (commit ~b50b6483 as of 2025-12-16). When IronRDP 0.5+ publishes to crates.io, we'll switch to published versions.

```toml
# Current (lamco-rdp-clipboard Cargo.toml)
[dependencies]
ironrdp-cliprdr = { git = "https://github.com/Devolutions/IronRDP", branch = "master" }
ironrdp-core = { git = "https://github.com/Devolutions/IronRDP", branch = "master" }

# Server Cargo.toml must use same git refs for compatibility
```

## Implementation Steps

1. **Update Cargo.toml** - Add lamco-* dependencies with required features
2. **Delete replaced files** - `formats.rs`, `transfer.rs`, `dbus_bridge.rs`
3. **Refactor sync.rs** - Keep SyncManager, import LoopDetector from library
4. **Refactor error.rs** - Keep server policy types, import base errors
5. **Refactor ironrdp_backend.rs** - Use library backend, add server wrappers
6. **Refactor manager.rs** - Update imports, remove duplicated code
7. **Update mod.rs** - Re-export from libraries instead of local modules
8. **Test** - Full bidirectional clipboard testing

## Testing Requirements

1. **Text sync**: UTF-8 ↔ UTF-16LE, HTML CF_HTML format
2. **Image sync**: PNG, JPEG, BMP ↔ DIB conversion
3. **Loop detection**: Verify no infinite loops on rapid clipboard changes
4. **Echo protection**: Verify D-Bus signals within echo window are ignored
5. **Error recovery**: Verify retry/fallback behavior on transient errors
6. **Large data**: Chunked transfer for >1MB clipboard data

## Consequences

### Positive

- **~80% code reduction** in clipboard module (5,700 → ~600 LOC)
- **Reusable libraries** - Other RDP projects can use lamco-* crates
- **Maintainability** - Bug fixes in libraries benefit all consumers
- **Clean separation** - Library primitives vs. server policy clearly separated

### Negative

- **External dependencies** - Server now depends on published crates
- **API coupling** - Changes to library APIs require server updates
- **IronRDP git ref** - Must coordinate git refs until IronRDP 0.5 publishes

### Risks

- **Tomorrow's republish** - Libraries must be republished before server can pull them
- **Feature flags** - Must enable correct features (`image`, `dbus-clipboard`, etc.)
- **IronRDP compatibility** - Server and libraries must use same IronRDP git ref

## Notes

- Server's `WrdCliprdrFactory` stays as wrapper around library's `RdpCliprdrFactory` to integrate with server-specific `ClipboardManager`
- Error types are EXTENDED (server adds `RecoveryAction`), not replaced
- SyncManager uses library's `LoopDetector` internally, but adds state tracking layer on top
