# Dependency Architecture
## Multi-Repository Development Configuration

**Last Updated:** 2025-12-23
**Status:** Production development configuration

---

## Overview

The lamco-rdp-server project requires coordinated dependency management across three repositories:

1. **wrd-server-specs** (this repo) - Commercial RDP server implementation
2. **lamco-rdp-workspace** - Open source RDP protocol crates
3. **IronRDP** (fork) - Forked RDP protocol with file transfer extensions

This document explains the dependency architecture and provides instructions for maintaining it correctly.

---

## Repository Relationships

```
┌────────────────────────────────────────────────────────────────────┐
│                        wrd-server-specs                             │
│                    (lamco-rdp-server v0.1.0)                        │
│                                                                     │
│  Dependencies:                                                      │
│  ├── Published crates.io:                                           │
│  │   ├── lamco-portal = "0.2.1"                                     │
│  │   ├── lamco-pipewire = "0.1.3"                                   │
│  │   ├── lamco-video = "0.1.2"                                      │
│  │   ├── lamco-clipboard-core = "0.3.0"                             │
│  │   └── lamco-wayland = "0.2.1"                                    │
│  │                                                                   │
│  ├── Local path (trait coupling):                                    │
│  │   └── lamco-rdp-clipboard (from lamco-rdp-workspace)             │
│  │                                                                   │
│  └── Local path (unpublished features):                              │
│      └── ironrdp-* (from IronRDP fork)                              │
└────────────────────────────────────────────────────────────────────┘
                              │
                              │ path dependency
                              ▼
┌────────────────────────────────────────────────────────────────────┐
│                      lamco-rdp-workspace                            │
│                   /home/greg/wayland/lamco-rdp-workspace            │
│                                                                     │
│  Published Crates:                                                  │
│  ├── lamco-clipboard-core v0.3.0                                    │
│  ├── lamco-rdp-clipboard v0.2.1                                     │
│  ├── lamco-rdp-input v0.1.1                                         │
│  └── lamco-rdp v0.3.0                                               │
│                                                                     │
│  Development Dependencies:                                           │
│  └── ironrdp-cliprdr (LOCAL PATH during development)                │
│      ↑ Must match wrd-server-specs to avoid trait conflicts         │
└────────────────────────────────────────────────────────────────────┘
                              │
                              │ path dependency (development)
                              ▼
┌────────────────────────────────────────────────────────────────────┐
│                         IronRDP Fork                                │
│                   /home/greg/wayland/IronRDP                        │
│                   Branch: pr3-file-contents-response-v2             │
│                                                                     │
│  Contains PRs pending upstream merge:                               │
│  ├── PR #1064: lock_clipboard() / unlock_clipboard()                │
│  ├── PR #1065: request_file_contents()                              │
│  └── PR #1066: SendFileContentsResponse                             │
│                                                                     │
│  These methods are REQUIRED for file transfer functionality.        │
└────────────────────────────────────────────────────────────────────┘
```

---

## Why This Architecture?

### Problem: Trait Identity in Rust

Rust's type system requires that traits come from the **exact same crate instance**. Two versions of the same crate (even with identical code) produce incompatible traits.

**Example of the problem:**
```
wrd-server-specs
├── ironrdp-cliprdr v0.5.0 (local fork)    ─┐
│   └── CliprdrBackend trait                │ Different trait identities!
│                                           │
└── lamco-rdp-clipboard (from crates.io)    │
    └── ironrdp-cliprdr v0.4.0 (crates.io) ─┘
        └── CliprdrBackend trait (INCOMPATIBLE)
```

### Solution: Unified Local Paths

All crates that interact with IronRDP traits must use the **same IronRDP source**:

```
wrd-server-specs
├── ironrdp-cliprdr (local fork)           ─┐
│   └── CliprdrBackend trait                │ Same trait!
│                                           │
└── lamco-rdp-clipboard (local path)        │
    └── ironrdp-cliprdr (local fork) ───────┘
        └── CliprdrBackend trait (SAME)
```

---

## Configuration Details

### wrd-server-specs/Cargo.toml

```toml
[dependencies]
# Published crates - no trait coupling
lamco-portal = { version = "0.2.1", features = ["dbus-clipboard"] }
lamco-pipewire = "0.1.3"
lamco-video = "0.1.2"
lamco-clipboard-core = { version = "0.3.0", features = ["image"] }

# Local path - implements CliprdrBackend trait
lamco-rdp-clipboard = { path = "../lamco-rdp-workspace/crates/lamco-rdp-clipboard" }

# Local fork - provides CliprdrBackend trait + file transfer methods
ironrdp-cliprdr = { path = "/home/greg/wayland/IronRDP/crates/ironrdp-cliprdr" }

[patch.crates-io]
# Override any transitive ironrdp dependencies from crates.io
ironrdp-cliprdr = { path = "/home/greg/wayland/IronRDP/crates/ironrdp-cliprdr" }
# ... other ironrdp crates
```

### lamco-rdp-workspace/Cargo.toml

```toml
[workspace.dependencies]
# DEVELOPMENT MODE: Use local IronRDP fork
# This MUST match wrd-server-specs to avoid trait conflicts
ironrdp-cliprdr = { path = "/home/greg/wayland/IronRDP/crates/ironrdp-cliprdr" }
ironrdp-core = { path = "/home/greg/wayland/IronRDP/crates/ironrdp-core" }

# PUBLICATION MODE (commented out during development):
# ironrdp-cliprdr = { version = "0.4", git = "https://github.com/glamberson/IronRDP", branch = "master" }
```

---

## Development Workflow

### Normal Development

1. Both repos use local IronRDP fork
2. Build from wrd-server-specs as normal
3. All traits resolve to same identity

### Before Publishing lamco-rdp-workspace

1. Edit `lamco-rdp-workspace/Cargo.toml`:
   ```toml
   # Switch to git dependency for publication
   ironrdp-cliprdr = { version = "0.4", git = "https://github.com/glamberson/IronRDP", branch = "master" }
   ```
2. Run `cargo publish` for affected crates
3. Switch back to local paths for continued development

### After IronRDP PRs Merge

When Devolutions merges PRs #1064-1066 and publishes a new version:

1. Update wrd-server-specs to use crates.io version:
   ```toml
   ironrdp-cliprdr = "0.6"  # or whatever version includes the PRs
   ```
2. Remove the `[patch.crates-io]` section
3. Update lamco-rdp-workspace to use new version
4. Re-publish lamco-rdp-clipboard if needed

---

## Crate Categories

### Category 1: Published to crates.io (No Trait Coupling)

These crates don't implement or depend on IronRDP traits:

| Crate | Version | Source |
|-------|---------|--------|
| lamco-portal | 0.2.1 | crates.io |
| lamco-pipewire | 0.1.3 | crates.io |
| lamco-video | 0.1.2 | crates.io |
| lamco-wayland | 0.2.1 | crates.io |
| lamco-clipboard-core | 0.3.0 | crates.io |
| lamco-rdp | 0.3.0 | crates.io |
| lamco-rdp-input | 0.1.1 | crates.io |

### Category 2: Local Path Required (Trait Coupling)

These crates implement IronRDP traits and must use the same source:

| Crate | Reason |
|-------|--------|
| lamco-rdp-clipboard | Implements `CliprdrBackend` trait |
| ironrdp-* | Defines `CliprdrBackend` trait |

### Category 3: IronRDP Fork (Unpublished Features)

These features are not yet available in upstream IronRDP:

| Feature | PR | Method |
|---------|-----|--------|
| Clipboard locking | #1064 | `lock_clipboard()`, `unlock_clipboard()` |
| File contents request | #1065 | `request_file_contents()` |
| File contents response | #1066 | `SendFileContentsResponse` |

---

## Troubleshooting

### Error: "trait `CliprdrBackend` is not implemented"

**Cause:** Multiple versions of ironrdp-cliprdr in dependency tree.

**Solution:**
1. Run `cargo tree -i ironrdp-cliprdr` to see versions
2. Ensure both wrd-server-specs and lamco-rdp-workspace use same source
3. Clear cargo caches: `rm -rf target Cargo.lock ~/.cargo/git/checkouts/ironrdp-*`

### Error: "patch was not used in the crate graph"

**Cause:** Patch version doesn't match dependency version (e.g., patching v0.5 onto v0.4 requirement).

**Solution:** Use local path directly instead of patch, or update version requirement.

### Error: "multiple `ironrdp-cliprdr` packages"

**Cause:** One repo using git, another using local path.

**Solution:** Both repos must use the same dependency source.

---

## IronRDP Version History

| Version | Source | Notes |
|---------|--------|-------|
| 0.4.0 | crates.io | Last published version, used by published lamco-rdp-clipboard |
| 0.5.0 | glamberson/IronRDP fork | Local development version with file transfer |
| TBD | Devolutions/IronRDP | When PRs merge and new version is published |

---

## File Locations

| Item | Path |
|------|------|
| wrd-server-specs | `/home/greg/wayland/wrd-server-specs` |
| lamco-rdp-workspace | `/home/greg/wayland/lamco-rdp-workspace` |
| IronRDP fork | `/home/greg/wayland/IronRDP` |
| IronRDP branch | `pr3-file-contents-response-v2` |

---

## Checklist: Verifying Configuration

```bash
# 1. Check only one ironrdp-cliprdr version
cargo tree -i ironrdp-cliprdr
# Should show ONLY v0.5.0 from local path

# 2. Verify build succeeds
cargo build --release

# 3. Test deployment
./test-gnome.sh deploy
# Verify video streaming and text clipboard work
```

---

**Document maintained by:** Development team
**Related documents:**
- `HANDOVER-2025-12-23-CLIPBOARD-PUBLICATION.md`
- `STATUS-2025-12-23-DEVELOPMENT-DIRECTIONS.md`
