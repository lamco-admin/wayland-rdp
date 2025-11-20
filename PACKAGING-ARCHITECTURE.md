# WRD-Server Packaging Architecture

## YOU'RE EXACTLY RIGHT

You want **TWO separate products** that can share code:

---

## PRODUCT 1: Universal Wayland RDP Server

**What**: Connect to ANY existing Wayland desktop
**Use case**: Screen sharing, remote access to physical workstation
**User sees**: Their actual GNOME/KDE/Sway desktop

**Capabilities**:
```
✅ Video: Portal (all desktops)
✅ Input: Portal (all desktops)
✅ Clipboard Windows→Linux: Portal (all desktops)
⚠️ Clipboard Linux→Windows: 
    - KDE: YES (Klipper DBus)
    - Sway: YES (wlr-data-control)
    - GNOME: NO (limitation)
```

**Package**: `wayland-rdp-server`
**Binary**: `wayland-rdp`
**Crate**: `wayland-rdp` (uses Portal APIs)

---

## PRODUCT 2: Lamco Compositor VDI

**What**: Self-contained virtual desktop
**Use case**: Headless cloud VDI, separate desktop environment
**User sees**: NEW Lamco desktop (not their GNOME/KDE)

**Capabilities**:
```
✅ Video: Compositor framebuffer
✅ Input: Compositor injection
✅ Clipboard: Full bidirectional (all directions)
✅ Headless: Runs with Xvfb (no GPU)
✅ Multi-tenant: Isolated per user
```

**Package**: `lamco-compositor` + `wayland-rdp`
**Binary**: `lamco-vdi`
**Crate**: `lamco-compositor` (reusable Wayland compositor)

---

## SHARED CODE (CRATES)

### Crate 1: `lamco-compositor`

**What**: Reusable Wayland compositor library
```
src/compositor/ (4,586 lines)
├─ Wayland protocols (compositor, xdg_shell, seat, etc.)
├─ Clipboard (data_device protocol)
├─ Input management
├─ Software renderer
└─ Backend abstraction (X11, DRM, Pixman)
```

**Used by**:
- Lamco VDI product
- Could be used by other Rust Wayland projects
- Standalone crate on crates.io

### Crate 2: `wayland-rdp`

**What**: RDP server for Wayland systems
```
src/server/, src/rdp/, src/video/, src/input/
├─ IronRDP integration
├─ Portal client (ashpd)
├─ PipeWire capture
├─ Input translation
└─ Backend abstraction (Portal OR Compositor)
```

**Used by**:
- Universal RDP server product
- Lamco VDI product
- Reusable for other RDP projects

### Crate 3: `wayland-rdp-clipboard`

**What**: Adaptive clipboard sync library
```
src/clipboard/
├─ Backend trait
├─ Portal backend (delayed rendering)
├─ Klipper backend (KDE DBus)
├─ wlr-data-control backend (Sway)
├─ Compositor backend (Lamco)
└─ Format conversion (RDP ↔ MIME)
```

**Used by**:
- Both products
- Could help other projects (Barrier, Deskflow)

---

## WORKSPACE STRUCTURE

```
wayland-rdp/  (workspace root)
│
├─ Cargo.toml  (workspace)
│
├─ crates/
│  ├─ lamco-compositor/     (NEW: Compositor library)
│  │  ├─ Cargo.toml
│  │  └─ src/
│  │     ├─ protocols/
│  │     ├─ backend/
│  │     └─ state.rs
│  │
│  ├─ wayland-rdp/          (RENAME: Core RDP server)
│  │  ├─ Cargo.toml
│  │  └─ src/
│  │     ├─ server/
│  │     ├─ portal/
│  │     └─ video/
│  │
│  └─ wayland-rdp-clipboard/ (NEW: Clipboard library)
│     ├─ Cargo.toml
│     └─ src/
│        ├─ backend_trait.rs
│        ├─ klipper.rs
│        ├─ wlr_data_control.rs
│        └─ portal.rs
│
├─ wayland-rdp-server/      (Binary 1: Portal mode)
│  ├─ Cargo.toml
│  └─ src/
│     └─ main.rs  (Portal mode only)
│
└─ lamco-vdi/               (Binary 2: Compositor mode)
   ├─ Cargo.toml
   └─ src/
      └─ main.rs  (Compositor + RDP)
```

---

## PRODUCT COMPARISON

### Universal Wayland RDP Server

**For**: Existing desktop users
**Connects to**: User's GNOME/KDE/Sway
**Shows**: Their actual desktop
**Clipboard**: Windows→Linux + (KDE/Sway: Linux→Windows)
**Install**: Single package, uses system compositor
**Use**: `wayland-rdp --port 3389`

### Lamco VDI

**For**: Cloud/headless/multi-tenant
**Creates**: New virtual desktop
**Shows**: Lamco environment (separate from host desktop)
**Clipboard**: Full bidirectional on all systems
**Install**: Compositor + RDP + Xvfb
**Use**: `lamco-vdi --port 3389`

---

## EVEN FOR WORKSTATIONS

**On a physical workstation with GNOME**:

**Scenario 1**: User wants to share their actual GNOME desktop
```bash
wayland-rdp  # Portal mode, connects to their GNOME
# They see their real desktop, running apps, etc.
# Clipboard: Windows→Linux only (GNOME limitation)
```

**Scenario 2**: User wants full clipboard in separate session
```bash
lamco-vdi --display :1  # Separate Lamco desktop on :1
# Different desktop from their GNOME session
# Full bidirectional clipboard
# Isolated environment
```

**Both can run SIMULTANEOUSLY** (different displays)!

---

## YOU'RE RIGHT ABOUT SEPARATION

**UI Product** (`wayland-rdp-server`):
- Binary that connects to existing UI
- Portal-based
- Works everywhere
- Limitation: GNOME clipboard one-way

**Compositor Product** (`lamco-vdi`):
- Binary with embedded compositor
- Creates its own UI
- Works everywhere
- Full clipboard always

**Shared Crates**:
- `lamco-compositor` - Reusable compositor
- `wayland-rdp` - RDP server core
- `wayland-rdp-clipboard` - Adaptive clipboard

---

## CURRENT CODE ORGANIZATION

**What you have now**:
```
wrd-server/ (monolithic)
  └─ Everything in one binary
     ├─ Portal mode code
     └─ Compositor code (feature flag)
```

**What you should have**:
```
Workspace with multiple crates:
  ├─ lamco-compositor (library)
  ├─ wayland-rdp (library)
  ├─ wayland-rdp-clipboard (library)
  ├─ wayland-rdp-server (binary)
  └─ lamco-vdi (binary)
```

---

## MIGRATION PATH

### Step 1: Extract Compositor (1-2 days)
- Move `src/compositor/` → `crates/lamco-compositor/src/`
- Publish as separate crate
- Make it reusable

### Step 2: Extract Clipboard (1 day)
- Move clipboard backends → `crates/wayland-rdp-clipboard/src/`
- Backend trait + implementations
- Reusable library

### Step 3: Create Binaries (1 day)
- `wayland-rdp-server`: Portal mode only
- `lamco-vdi`: Compositor mode only
- Clean separation

---

## IS THIS RIGHT?

**Yes!** You understand it perfectly:

✅ **Universal RDP server** = Connect to existing desktops
✅ **Lamco VDI** = Separate virtual desktop  
✅ **Shared crates** = Code reuse
✅ **Both run on same machine** = Different purposes

**Different products for different use cases, sharing battle-tested code.**

---

Ready to restructure into workspace with separate crates?
