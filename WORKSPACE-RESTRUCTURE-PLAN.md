# Workspace Restructuring Plan

## GOAL

Transform monolithic `wrd-server` into clean workspace with:
- 3 reusable library crates
- 2 binary products
- Proper separation of concerns

---

## TARGET STRUCTURE

```
wayland-rdp/ (renamed from wrd-server-specs)
├─ Cargo.toml (workspace root)
│
├─ crates/
│  │
│  ├─ lamco-compositor/          (4,586 lines)
│  │  ├─ Cargo.toml
│  │  ├─ README.md
│  │  └─ src/
│  │     ├─ lib.rs
│  │     ├─ protocols/
│  │     ├─ backend/
│  │     └─ state.rs
│  │
│  ├─ wayland-rdp-core/          (12,000 lines)
│  │  ├─ Cargo.toml
│  │  ├─ README.md
│  │  └─ src/
│  │     ├─ lib.rs
│  │     ├─ server/
│  │     ├─ video/
│  │     ├─ input/
│  │     └─ rdp/
│  │
│  └─ wayland-rdp-clipboard/     (3,000 lines)
│     ├─ Cargo.toml
│     ├─ README.md
│     └─ src/
│        ├─ lib.rs
│        ├─ backend.rs (trait)
│        ├─ portal.rs
│        ├─ klipper.rs (TODO)
│        ├─ wlr.rs (TODO)
│        ├─ compositor.rs
│        └─ formats.rs
│
├─ wayland-rdp-server/           (Portal mode binary)
│  ├─ Cargo.toml
│  ├─ README.md
│  └─ src/
│     └─ main.rs (uses portal backend)
│
└─ lamco-vdi/                    (Compositor mode binary)
   ├─ Cargo.toml
   ├─ README.md
   └─ src/
      └─ main.rs (uses compositor backend)
```

---

## MIGRATION STEPS (1-2 days)

### Step 1: Create Workspace (1 hour)

```bash
# Root Cargo.toml
[workspace]
members = [
    "crates/lamco-compositor",
    "crates/wayland-rdp-core",
    "crates/wayland-rdp-clipboard",
    "wayland-rdp-server",
    "lamco-vdi",
]
resolver = "2"

[workspace.dependencies]
# Shared dependencies
tokio = { version = "1.35", features = ["full"] }
ironrdp-server = { git = "https://github.com/allan2/IronRDP", branch = "update-sspi" }
# etc...
```

### Step 2: Extract lamco-compositor (2 hours)

```bash
mkdir -p crates/lamco-compositor/src
mv src/compositor/* crates/lamco-compositor/src/

# Create crates/lamco-compositor/Cargo.toml
[package]
name = "lamco-compositor"
version = "0.1.0"
edition = "2021"
description = "Pure Rust Wayland compositor built on Smithay"
license = "MIT OR Apache-2.0"

[dependencies]
smithay = { workspace = true }
# etc...
```

### Step 3: Extract wayland-rdp-clipboard (2 hours)

```bash
mkdir -p crates/wayland-rdp-clipboard/src
mv src/clipboard/* crates/wayland-rdp-clipboard/src/

# Create trait-based API
pub trait ClipboardBackend {
    async fn monitor(...);
    async fn read(...);
    async fn write(...);
}
```

### Step 4: Create wayland-rdp-core (2 hours)

```bash
mkdir -p crates/wayland-rdp-core/src
mv src/{server,video,input,portal,rdp} crates/wayland-rdp-core/src/
```

### Step 5: Create Binaries (2 hours)

**wayland-rdp-server/src/main.rs**:
```rust
use wayland_rdp_core::Server;
use wayland_rdp_clipboard::PortalBackend;

#[tokio::main]
async fn main() {
    let server = Server::builder()
        .with_portal_mode()
        .with_clipboard(PortalBackend::new())
        .build()?;
    
    server.run().await?;
}
```

**lamco-vdi/src/main.rs**:
```rust
use lamco_compositor::Compositor;
use wayland_rdp_core::Server;

#[tokio::main]
async fn main() {
    let compositor = Compositor::new()?;
    let server = Server::builder()
        .with_compositor_mode(compositor)
        .build()?;
    
    server.run().await?;
}
```

---

## BENEFITS

**Clean Separation**:
- Library crates: Reusable by others
- Binary crates: Specific use cases
- Workspace: Shared dependencies

**Publishable**:
- Each crate on crates.io independently
- Other projects can use lamco-compositor
- Helps ecosystem

**Maintainable**:
- Clear boundaries
- Easy to test each piece
- Proper versioning

---

## TIMELINE

**Day 1**: Workspace setup + extract compositor
**Day 2**: Extract clipboard + core
**Day 3**: Create binaries + test
**Day 4**: Documentation + cleanup
**Day 5**: Publish to crates.io

**Total**: 1 week part-time

---

## AFTER RESTRUCTURE

**Then you can**:
- Publish crates individually
- Release binaries separately
- Clear value proposition per product
- Better for users AND for your credibility

---

Ready to restructure, or finish testing current monolithic version first?
