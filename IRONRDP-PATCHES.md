# IronRDP Patches Tracker

This document tracks all patches made to the local IronRDP fork that need to be submitted as PRs to Devolutions/IronRDP.

## Fork Repository
- **Fork**: https://github.com/glamberson/IronRDP
- **Upstream**: https://github.com/Devolutions/IronRDP

---

## Patch 1: cliprdr-request-file-contents

**Branch**: `cliprdr-request-file-contents`
**Status**: Pushed to fork, NOT YET PR'd
**Crates**: `ironrdp-cliprdr` (Core Tier), `ironrdp-server` (Community Tier)

### Changes

#### ironrdp-cliprdr/src/lib.rs
- Added import of `FileContentsRequest` and `FileContentsFlags` to pdu use statement
- Added `request_file_contents()` method to `Cliprdr<R>` struct

#### ironrdp-cliprdr/src/backend.rs
- Added `SendFileContentsRequest` variant to `ClipboardMessage` enum
- Carries all parameters needed for file contents request (stream_id, index, position, requested_size, is_size_request, data_id)

#### ironrdp-server/src/server.rs
- Added handler for `ClipboardMessage::SendFileContentsRequest` variant
- Calls `cliprdr.request_file_contents()` with extracted parameters

### Method Signature
```rust
pub fn request_file_contents(
    &self,
    stream_id: u32,
    index: u32,
    position: u64,
    requested_size: u32,
    is_size_request: bool,
    data_id: Option<u32>,
) -> PduResult<CliprdrSvcMessages<R>>
```

### Message Variant
```rust
ClipboardMessage::SendFileContentsRequest {
    stream_id: u32,
    index: u32,
    position: u64,
    requested_size: u32,
    is_size_request: bool,
    data_id: Option<u32>,
}
```

### Purpose
Enables servers to request file contents from clients during clipboard file transfers. This was the missing piece for server-side file paste operations.

### Commits
```
f2bc3659 feat(cliprdr): add request_file_contents method for server-side file transfer
b92d0206 feat(cliprdr): add SendFileContentsRequest message variant
```

---

## Patch 2: egfx-server-complete (Existing)

**Branch**: `egfx-server-complete`
**Status**: PR #1057 open
**Crates**: Multiple EGFX-related crates

### Purpose
Server-side EGFX (graphics pipeline) support.

---

## Using the Fork Branch

Since the patches affect multiple crates that depend on each other, ALL IronRDP
dependencies should use the same fork branch to avoid trait conflicts:

```toml
[dependencies]
# All IronRDP crates from fork branch
ironrdp = { git = "https://github.com/glamberson/IronRDP", branch = "cliprdr-request-file-contents", ... }
ironrdp-server = { git = "https://github.com/glamberson/IronRDP", branch = "cliprdr-request-file-contents" }
ironrdp-cliprdr = { git = "https://github.com/glamberson/IronRDP", branch = "cliprdr-request-file-contents" }
# ... same for all other ironrdp-* crates
```

Note: `[patch.crates-io]` only patches crates.io dependencies, not git dependencies.
If you use git dependencies directly, they must all point to the same source.

---

## PR Checklist (for when submitting)

Each PR must:
- [ ] Be in its own separate branch
- [ ] Follow IronRDP STYLE.md guidelines
- [ ] Include doc comments linking to MS specifications
- [ ] Pass `cargo check` and `cargo test`
- [ ] Be reviewed against ARCHITECTURE.md tier requirements
  - Core Tier (ironrdp-cliprdr): no I/O, must be fuzzed, no_std compatible

---

## Notes

- Each patch should be a focused, single-purpose change
- PRs to be created in clean sessions after testing confirms patches work
- Follow lamco-admin staging standards for publication workflow
