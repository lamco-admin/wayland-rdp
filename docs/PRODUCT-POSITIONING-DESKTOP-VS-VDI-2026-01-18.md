# Product Positioning: lamco-rdp-server vs lamco-VDI
**Date:** 2026-01-18
**Purpose:** Technical differentiation between desktop sharing and headless VDI products
**Status:** Strategic positioning document

---

## Executive Summary

**lamco-rdp-server** and **lamco-VDI** (future product) share the same core RDP protocol stack but target fundamentally different use cases with different technical architectures.

**lamco-rdp-server (Current Product):**
- **What it does:** Share your existing Linux desktop session via RDP
- **Requirement:** Desktop environment (GNOME/KDE/wlroots) must be running
- **Use case:** Remote access to your workstation, screen sharing, remote support
- **Architecture:** Portal client consuming from existing compositor
- **Target:** Desktop users, developers, remote workers

**lamco-VDI (Future Product):**
- **What it does:** Multi-user headless VDI server (no desktop environment required)
- **Requirement:** Headless Linux server (cloud VM, bare metal)
- **Use case:** Enterprise VDI, cloud workspaces, multi-tenant hosting
- **Architecture:** Integrated Smithay compositor + embedded Portal + session management
- **Target:** Enterprise, service providers, cloud deployments

---

## Technical Architecture Comparison

### lamco-rdp-server: Desktop Sharing Architecture

```
┌─────────────────────────────────────────────────┐
│  Physical/Virtual Machine with Display          │
│  • User logged in to GNOME/KDE/wlroots          │
│  • Desktop environment fully running            │
│  • Applications launched by user                │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│  Existing Compositor (GNOME Mutter/KWin/Sway)   │
│  • Manages application windows                  │
│  • Renders to physical/virtual display          │
│  • Handles local user input                     │
│  • Provides Wayland protocols                   │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│  XDG Desktop Portal (External Process)           │
│  • xdg-desktop-portal-gnome / -kde / -wlr       │
│  • Presents permission dialogs to local user    │
│  • Bridges between apps and compositor          │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│  PipeWire (System Service)                       │
│  • Captures frames from compositor              │
│  • Provides video stream                        │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│  lamco-rdp-server (Portal Client)                │
│  • Connects to Portal via D-Bus                 │
│  • Consumes PipeWire video stream               │
│  • Encodes H.264 and streams via RDP            │
│  • Injects input via Portal RemoteDesktop       │
└─────────────────────────────────────────────────┘
                  │
                  ▼
         RDP Client (Windows/macOS/Linux)
```

**Key Characteristics:**
- **Dependency:** Requires existing desktop session
- **Session Model:** Single user (the logged-in user's desktop)
- **Permission Model:** Portal-based (dialogs required unless using direct APIs)
- **Resource Overhead:** Full DE (~800MB+ GNOME, ~400MB+ KDE)
- **Setup:** User must be logged in (GUI login or auto-login)
- **Scaling:** One session per machine (shares one user's desktop)

---

### lamco-VDI: Headless Multi-Session Architecture

```
┌─────────────────────────────────────────────────┐
│  Headless Linux Server (No Display)              │
│  • No desktop environment installed             │
│  • No physical display attached                 │
│  • Minimal OS (Ubuntu Server, Debian)           │
│  • Multiple users can connect simultaneously    │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│  lamco-VDI Session Manager                       │
│  • PAM authentication per user                  │
│  • Creates isolated Wayland compositor per user │
│  • Resource limits (cgroup, memory, CPU)        │
│  • Session lifecycle (create, suspend, destroy) │
└──────────┬──────────────┬────────────┬──────────┘
           │              │            │
    User 1 │       User 2 │     User 3 │
           ▼              ▼            ▼
    ┌──────────┐   ┌──────────┐  ┌──────────┐
    │ Smithay  │   │ Smithay  │  │ Smithay  │
    │Compositor│   │Compositor│  │Compositor│
    │ (virtual)│   │ (virtual)│  │ (virtual)│
    │ Per-user │   │ Per-user │  │ Per-user │
    └────┬─────┘   └────┬─────┘  └────┬─────┘
         │              │            │
         ▼              ▼            ▼
    ┌─────────────────────────────────────┐
    │   Embedded Portal Backend           │
    │   • Auto-grants permissions         │
    │   • No dialogs (headless)           │
    │   • Integrated into lamco-VDI       │
    └─────────────┬───────────────────────┘
                  │
                  ▼
    ┌─────────────────────────────────────┐
    │   PipeWire Producer (Integrated)    │
    │   • Generates streams from Smithay  │
    │   • Per-user isolated streams       │
    └─────────────┬───────────────────────┘
                  │
                  ▼
    ┌─────────────────────────────────────┐
    │   RDP Server (Shared IronRDP Core)  │
    │   • H.264 encoding per session      │
    │   • TLS encryption                  │
    │   • Multi-client support            │
    └─────────────┬───────────────────────┘
                  │
                  ▼
         RDP Clients (Multiple Users)
```

**Key Characteristics:**
- **Independence:** No desktop environment required
- **Session Model:** Multi-user, concurrent, isolated sessions
- **Permission Model:** Embedded Portal auto-grants (no dialogs)
- **Resource Overhead:** Minimal (~256MB per user session)
- **Setup:** Deploy as systemd service, users connect via RDP
- **Scaling:** 10-50+ concurrent users per server instance

---

## Feature Comparison Matrix

| Feature | lamco-rdp-server | lamco-VDI |
|---------|------------------|-----------|
| **Session Type** | Share existing desktop | Create isolated sessions |
| **User Model** | Single user (logged-in) | Multi-user concurrent |
| **Compositor** | External (GNOME/KDE/wlroots) | Integrated Smithay |
| **Desktop Environment** | Required (GNOME/KDE/etc.) | None (headless) |
| **Permission Dialogs** | Yes (unless direct API) | No (embedded Portal) |
| **Resource per Session** | ~800MB+ (full DE) | ~256MB (compositor only) |
| **Setup Complexity** | Low (install, run) | Medium (systemd service, multi-user config) |
| **Deployment** | Desktop workstation | Headless server (cloud/VPS) |
| **Scaling Model** | 1 session per machine | 10-50+ sessions per server |
| **Authentication** | Optional (or local user) | Required (PAM per user) |
| **Session Isolation** | N/A (single user) | cgroup resource limits |
| **Binary Size** | ~15MB | ~20MB (includes compositor) |
| **External Dependencies** | Portal, PipeWire, Compositor | Minimal (PipeWire optional) |
| **Use Case** | Remote workstation access | Enterprise VDI platform |

---

## Use Case Differentiation

### lamco-rdp-server Use Cases

**1. Remote Workstation Access**
- Developer working from home accessing office workstation
- System administrator remotely managing servers with GUI
- Designer accessing powerful workstation from laptop

**Requirement:** Existing desktop session (GNOME/KDE running)

---

**2. Screen Sharing / Remote Support**
- Technical support accessing customer's desktop
- Pair programming / code review sessions
- Demonstrating software to remote users

**Requirement:** User logged in locally, sharing their active desktop

---

**3. Virtual Machine Desktop Access**
- VM with desktop environment for development
- Test environment access
- Isolated GUI applications

**Requirement:** VM has desktop environment installed and running

---

**4. Single-User Server Desktop**
- Home server with occasional GUI access
- Media server with web browser access
- NAS with management GUI

**Requirement:** Desktop session running (can be auto-login)

---

### lamco-VDI Use Cases

**1. Enterprise Virtual Desktop Infrastructure**
- 100+ employees accessing virtual desktops
- Centralized desktop management
- Standard desktop environment for all users
- Cost savings vs Citrix/VMware

**Requirement:** Headless server cluster, no desktop environment

---

**2. Cloud Workspaces (AWS/Azure/GCP)**
- On-demand Linux development environments
- Scaling from 0 to 100+ users automatically
- Pay-per-use compute resources
- Containerized deployment (Kubernetes)

**Requirement:** Cloud VMs, container orchestration

---

**3. Thin Client Infrastructure**
- Corporate environment with thin clients at desks
- Centralized computing, zero data on clients
- Legacy hardware reuse
- Simplified IT management

**Requirement:** Central server(s), network infrastructure

---

**4. Multi-Tenant SaaS Platform**
- Provide Linux desktops as a service
- Per-user isolated environments
- Billing per user/hour
- Horizontal scaling

**Requirement:** Multi-server infrastructure, load balancing

---

**5. CI/CD with GUI Testing**
- Automated browser testing
- Visual regression testing
- Screenshot generation
- Headless Selenium/Playwright

**Requirement:** Headless CI infrastructure (GitHub Actions, GitLab CI)

---

## Technical Limitations Comparison

### What lamco-rdp-server CANNOT Do

❌ **Multi-User Concurrent Sessions**
- Architecture: Shares ONE user's desktop
- Limitation: Cannot create separate sessions for different users simultaneously
- Why: Portal/compositor belongs to single logged-in user
- VDI Solution: Creates per-user isolated compositor instances

---

❌ **True Headless Operation (No Desktop Environment)**
- Architecture: Requires existing compositor (GNOME/KDE/wlroots)
- Limitation: Must install full desktop environment
- Resource Cost: ~800MB+ RAM for GNOME, ~400MB+ for KDE
- VDI Solution: Minimal Smithay compositor (~50MB RAM)

---

❌ **Zero-Dialog Unattended Access (Flatpak Deployment on GNOME)**
- Architecture: Portal requires user permission
- Limitation: GNOME Portal rejects session persistence for RemoteDesktop
- Impact: Dialog appears on every server restart (Flatpak deployment)
- Workaround: Native package + Mutter Direct API (untested)
- VDI Solution: Embedded Portal auto-grants permissions (no dialogs)

---

❌ **Session Management & Lifecycle**
- Architecture: Shares user's existing session
- Limitation: Cannot create/destroy sessions programmatically
- Impact: No session pooling, no resource management per user
- VDI Solution: Full session manager with create/suspend/destroy/reconnect

---

❌ **Resource Isolation Between Users**
- Architecture: Single user session
- Limitation: N/A (not multi-user)
- VDI Solution: Per-user cgroup limits (memory, CPU, network)

---

❌ **Authentication Required for Access**
- Architecture: Optional PAM authentication
- Limitation: Can run with no authentication (suitable for trusted networks)
- Impact: Not suitable for public-facing deployments
- VDI Solution: Mandatory PAM authentication per user

---

❌ **Container/Kubernetes Deployment**
- Architecture: Requires D-Bus session bus, Portal, compositor
- Limitation: Complex to containerize (systemd, Desktop Environment)
- Impact: Not cloud-native
- VDI Solution: Single binary, minimal deps, container-friendly

---

❌ **Auto-Scaling**
- Architecture: One instance per machine
- Limitation: Cannot scale horizontally (can't add users dynamically)
- Impact: Fixed capacity
- VDI Solution: Kubernetes horizontal pod autoscaling

---

### What lamco-rdp-server DOES Exceptionally Well

✅ **Share Existing Desktop Session**
- Perfect for: Remote access to your personal workstation
- Benefit: See exactly what's on your screen, control your applications
- Setup: Install and run (5 minutes)

✅ **Zero Configuration**
- Perfect for: Users who want "just works"
- Benefit: Service Registry auto-detects capabilities, selects optimal features
- Setup: No compositor configuration, no Portal setup

✅ **Use Your Existing Applications**
- Perfect for: Access your configured environment (Firefox profiles, VS Code extensions)
- Benefit: Everything is already set up how you like it
- Setup: Applications already installed in your DE

✅ **Desktop Environment Integration**
- Perfect for: Users who need GNOME/KDE features (notifications, tray icons, etc.)
- Benefit: Full desktop experience remotely
- Setup: Desktop environment already configured

✅ **Low Barrier to Entry**
- Perfect for: Quick testing, evaluation, personal use
- Benefit: Flatpak install in 30 seconds
- Setup: `flatpak install flathub io.lamco.rdp-server`

---

## lamco-VDI Architecture (Future Product)

### Core Design Principles

**1. Headless-First**
- No desktop environment required
- No physical display required
- Runs on minimal Linux server (Ubuntu Server, Debian)

**2. Multi-Session**
- Concurrent users, each with isolated Wayland compositor
- Per-user resource limits (cgroup)
- Session lifecycle management (create, suspend, reconnect, destroy)

**3. Integrated Components**
- Smithay compositor embedded in binary
- Portal backend embedded (auto-grants permissions)
- PipeWire producer integrated (generates streams from compositor)
- Single binary deployment

**4. Enterprise-Ready**
- PAM authentication required
- systemd-logind integration
- Audit logging
- Monitoring (Prometheus metrics)

---

### Component Architecture

#### 1. Smithay Compositor (Per-User Instance)

**What it is:** Minimal Wayland compositor in pure Rust

**Responsibilities:**
- Manage application windows for one user
- Render to virtual framebuffer (no physical display)
- Handle Wayland protocol (wl_compositor, xdg_shell, wl_seat, etc.)
- Provide input injection (keyboard, mouse)
- Clipboard management

**Resource Footprint:** ~50-80MB RAM per instance

**Code:** ~3,000-5,000 lines (compositor implementation)

**Per-User Isolation:**
```
User 1: Smithay instance on WAYLAND_DISPLAY=wayland-0 (Port 3389)
User 2: Smithay instance on WAYLAND_DISPLAY=wayland-1 (Port 3390)
User 3: Smithay instance on WAYLAND_DISPLAY=wayland-2 (Port 3391)
```

Each user gets:
- Separate Wayland socket
- Separate RDP port (or port + username routing)
- Separate XDG_RUNTIME_DIR
- Separate environment variables
- Isolated compositor state

---

#### 2. Embedded Portal Backend

**What it is:** D-Bus service implementing org.freedesktop.portal.ScreenCast and RemoteDesktop

**Why embedded:** External portal (xdg-desktop-portal-gnome) requires GUI for dialogs. Headless servers have no GUI.

**How it works:**
- Registers on D-Bus session bus as org.freedesktop.portal.Desktop
- lamco-rdp-server connects to it (same code as desktop mode)
- Portal backend auto-grants all permissions (no dialogs)
- Direct integration with Smithay compositor

**Auto-Grant Logic:**
```rust
async fn select_sources(
    &mut self,
    session_handle: ObjectPath<'_>,
    options: HashMap<String, zvariant::Value<'_>>,
) -> zbus::fdo::Result<OwnedObjectPath> {
    // Auto-grant: select all available sources
    let session = self.sessions.get_mut(&session_handle.into())?;

    // No user interaction needed - this IS the user's session
    session.select_all_sources()?;

    // Return success immediately (no dialog shown)
    Ok(request_path)
}
```

**Code:** ~1,500-2,000 lines (Portal implementation)

---

#### 3. PipeWire Producer

**What it is:** Generates PipeWire video streams from Smithay framebuffer

**Current (lamco-rdp-server):** Consumes existing PipeWire stream from GNOME/KDE

**Future (lamco-VDI):** Produces PipeWire stream from Smithay compositor

**How it works:**
```rust
pub struct PipeWireProducer {
    stream: pipewire::Stream,
    compositor: Arc<Mutex<SmithayCompositor>>,
}

impl PipeWireProducer {
    pub fn push_frame(&mut self) -> Result<()> {
        // 1. Render compositor to framebuffer
        let pixels = self.compositor.lock().unwrap().render_frame()?;

        // 2. Get PipeWire buffer
        let mut buffer = self.stream.dequeue_buffer()?;

        // 3. Copy pixels to buffer
        buffer.datas_mut()[0].data().copy_from_slice(&pixels);

        // 4. Queue buffer for consumption
        self.stream.queue_buffer(buffer)?;

        Ok(())
    }
}
```

**Frame Loop:** Compositor render cycle triggers PipeWire frame push (30-60 FPS)

**Code:** ~500-800 lines (PipeWire producer)

---

#### 4. Session Manager

**What it is:** Multi-user session lifecycle management

**Responsibilities:**
- Authenticate users via PAM
- Create systemd-logind sessions
- Spawn per-user Smithay compositor
- Set up per-user environment (XDG_RUNTIME_DIR, HOME, etc.)
- Enforce resource limits (cgroup)
- Track session state (active, disconnected, suspended)
- Handle session reconnection
- Clean up on logout

**Example:**
```rust
pub async fn create_session(
    &mut self,
    username: &str,
    password: &str,
) -> Result<UserSession> {
    // 1. Authenticate via PAM
    self.authenticate_user(username, password).await?;

    // 2. Get user info (UID, GID, home directory)
    let user = User::from_name(username)?;

    // 3. Create systemd-logind session
    let session_id = self.systemd.create_session(&user.name).await?;

    // 4. Create XDG_RUNTIME_DIR (/run/user/1234)
    let runtime_dir = format!("/run/user/{}", user.uid);
    std::fs::create_dir_all(&runtime_dir)?;

    // 5. Spawn Smithay compositor as user
    let compositor_pid = self.spawn_compositor_as_user(
        &user,
        &runtime_dir,
        session_id,
    ).await?;

    // 6. Set up cgroup limits
    let cgroup = CGroup::new(&format!("lamco-vdi-{}", session_id))?;
    cgroup.set_memory_limit(2 * 1024 * 1024 * 1024)?; // 2GB
    cgroup.add_process(compositor_pid)?;

    // 7. Return session handle
    Ok(UserSession {
        username: user.name,
        session_id,
        compositor_pid,
        wayland_display: format!("wayland-{}", session_id),
        created_at: SystemTime::now(),
    })
}
```

**Code:** ~2,000-3,000 lines (session management)

---

## Shared Core Components

Both products share the **same RDP protocol stack**:

### Shared Components (from lamco-rdp-server codebase)

| Component | Lines | Purpose | Shared? |
|-----------|-------|---------|---------|
| **IronRDP integration** | ~5,000 | RDP protocol, TLS, channels | ✅ 100% shared |
| **EGFX video pipeline** | ~3,500 | H.264 encoding, AVC444, color space | ✅ 100% shared |
| **Input injection** | ~1,200 | Keyboard/mouse event translation | ✅ 100% shared |
| **Clipboard sync** | ~2,500 | RDP clipboard, file transfer | ✅ 100% shared |
| **Damage detection** | ~800 | Tile-based dirty region tracking | ✅ 100% shared |
| **Service Registry** | ~1,300 | Capability detection, runtime adaptation | ✅ Shared with VDI extensions |
| **Portal client** | ~2,000 | D-Bus Portal interaction | ⚠️ Desktop only (VDI has embedded Portal) |
| **PipeWire consumer** | ~1,500 | Stream consumption, format negotiation | ⚠️ Desktop only (VDI produces streams) |

**Total Shared Code:** ~16,800 lines (~70% of codebase)

---

### VDI-Specific Components (New Development)

| Component | Estimated Lines | Purpose |
|-----------|----------------|---------|
| **Smithay compositor** | ~3,000-5,000 | Wayland compositor implementation |
| **Embedded Portal backend** | ~1,500-2,000 | D-Bus Portal service (auto-grant) |
| **PipeWire producer** | ~500-800 | Generate streams from compositor |
| **Session manager** | ~2,000-3,000 | Multi-user session lifecycle |
| **Application launcher** | ~800-1,200 | Auto-start applications per user |
| **Resource isolation** | ~500-800 | cgroup limits, quota enforcement |

**Total New Code:** ~8,300-13,800 lines (~30% new, 70% reuse)

---

## Deployment Comparison

### lamco-rdp-server Deployment

**Installation (Flatpak):**
```bash
flatpak install flathub io.lamco.rdp-server
flatpak run io.lamco.rdp-server
```

**Installation (Native):**
```bash
# Fedora/RHEL
sudo dnf install lamco-rdp-server

# Start manually
lamco-rdp-server --config ~/.config/lamco-rdp-server/config.toml
```

**Requirements:**
- Desktop environment running (GNOME/KDE/wlroots)
- User logged in (GUI login or auto-login)
- Portal and PipeWire services active

**Target Environment:** User's personal workstation or VM with DE

---

### lamco-VDI Deployment

**Installation (Future):**
```bash
# Install package
sudo apt install lamco-vdi  # or dnf/zypper

# Configure multi-user
sudo systemctl enable lamco-vdi@.service

# Start service
sudo systemctl start lamco-vdi.socket
```

**System Service Template:**
```ini
# /etc/systemd/system/lamco-vdi@.service
[Unit]
Description=lamco VDI Server for %I
After=network.target

[Service]
Type=notify
User=%i
Environment=XDG_RUNTIME_DIR=/run/user/%U
Environment=WAYLAND_DISPLAY=wayland-vdi-%i
ExecStart=/usr/bin/lamco-vdi --user %i --headless
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

**Requirements:**
- Headless Linux server (Ubuntu Server, Debian, RHEL)
- No desktop environment (minimal OS install)
- systemd for service management
- Optional: GPU for hardware acceleration

**Target Environment:** Cloud VMs, data center servers, thin client infrastructure

---

## Session Persistence Architecture Comparison

### lamco-rdp-server: Portal-Dependent Persistence

**Challenge:** Portal requires user permission for screen capture/input

**Solutions (Multi-Strategy):**

**Strategy 1: Portal + Session Tokens (KDE, wlroots with Portal v4+)**
- First connection: User clicks "Allow" on permission dialog
- Server saves encrypted session restore token
- Subsequent connections: Token automatically restores session (no dialog)
- **Limitation:** GNOME Portal rejects tokens for RemoteDesktop sessions

**Strategy 2: Mutter Direct API (GNOME native package)**
- Bypasses Portal entirely
- Uses GNOME Mutter D-Bus APIs directly
- Zero dialogs even on first connection
- **Limitation:** Requires native package (not Flatpak), GNOME-specific
- **Status:** Implemented, pending testing

**Strategy 3: wlr-direct Protocols (Sway/Hyprland native package)**
- Uses wlroots native Wayland protocols (wlr-screencopy, wlr-virtual-keyboard/pointer)
- Zero dialogs (native protocols don't require Portal)
- **Limitation:** Requires native package, wlroots-specific
- **Status:** Implemented (1,050 lines), pending testing

**Strategy 4: Portal + libei/EIS (wlroots Flatpak)**
- Portal ConnectToEIS interface for input injection
- One dialog first time, token restores session
- **Limitation:** Requires Portal backend support (xdg-desktop-portal-wlr PR #359)
- **Status:** Implemented (480 lines), waiting for Portal support

**Fallback: Basic Portal**
- Dialog appears on every server restart
- **Use:** Only when all other strategies unavailable

**Key Point:** Session persistence is **complex** in lamco-rdp-server due to Portal security model. Multiple strategies exist to work around this, but deployment method and compositor dictate which strategy is available.

---

### lamco-VDI: Guaranteed Zero-Dialog Persistence

**No Dialogs Required:** Embedded Portal auto-grants permissions

**Architecture:**
```
lamco-VDI Session Manager
    ↓
Creates per-user Smithay compositor
    ↓
Embedded Portal backend (in-process)
    ↓
Auto-grants ScreenCast/RemoteDesktop permissions
    ↓
No user interaction needed (this IS the user's session)
```

**Why This Works:**
- Portal is part of lamco-VDI, not external service
- User authenticated via PAM (username/password) at RDP connection time
- Compositor belongs to that specific user
- Permission grant is implicit (user logged in = permission granted)
- No GUI needed (headless server)

**Result:** Zero dialogs, always. No strategy selection needed. No deployment constraints.

---

## Product Positioning

### lamco-rdp-server Positioning

**Tagline:** "Remote access to your Linux desktop"

**Primary Message:**
Share your existing GNOME, KDE, or wlroots desktop session via RDP. Connect from Windows, macOS, or Linux using built-in Remote Desktop clients. Hardware-accelerated H.264 encoding for smooth video. Crystal-clear text with AVC444. Zero configuration required.

**Target Audience:**
- Remote workers accessing office workstations
- Developers with home/office server
- IT support providing remote assistance
- Users wanting to access their Linux desktop from anywhere

**Value Proposition:**
- Install in minutes
- Use your existing desktop (no new environment to configure)
- Works with standard RDP clients (Windows Remote Desktop)
- Free for personal use

**Competitive Position:**
- Better than VNC (H.264 encoding, standard protocol)
- Better than xrdp (native Wayland, no X11)
- Better than proprietary (open source components, BSL future-proof)

---

### lamco-VDI Positioning (Future)

**Tagline:** "Enterprise Linux VDI platform"

**Primary Message:**
Deploy hundreds of isolated Linux desktop sessions on headless servers. Multi-user concurrent access with per-user resource limits. Cloud-native architecture (Kubernetes-ready). 70-85% cost savings vs Citrix/VMware Horizon. Pure Rust implementation with embedded Smithay compositor.

**Target Audience:**
- Enterprise IT departments (VDI replacement)
- Cloud service providers (Linux Desktop-as-a-Service)
- Educational institutions (computer labs)
- Development shops (cloud workspaces)

**Value Proposition:**
- Massive cost savings ($35/user/month AWS WorkSpaces → $5-10/user/month lamco-VDI)
- True headless (no desktop environment overhead)
- Horizontal scaling (Kubernetes, cloud-native)
- Multi-tenant isolation (per-user resources)
- Open source foundation (Apache 2.0 after Dec 31, 2028)

**Competitive Position:**
- Linux alternative to Citrix/VMware (Windows-centric)
- Open source alternative to AWS WorkSpaces (proprietary)
- Headless alternative to lamco-rdp-server (desktop sharing)
- Cloud-native alternative to traditional VDI (appliance-based)

---

## Technical Relationship Between Products

### Shared Foundation

Both products use the **same core technology stack**:

**RDP Protocol:**
- IronRDP for protocol implementation
- EGFX for H.264 video streaming
- TLS 1.3 encryption
- Clipboard synchronization
- Input injection (keyboard/mouse)

**Video Encoding:**
- OpenH264 software encoder
- VA-API hardware acceleration (Intel/AMD)
- NVENC hardware acceleration (NVIDIA)
- AVC420 and AVC444 codec support
- Color space management (BT.709, sRGB)

**Performance Optimization:**
- Damage tracking (tile-based dirty region detection)
- Adaptive frame rate (5-60 FPS)
- Bandwidth optimization (90%+ savings)

---

### Divergent Components

**Where they differ:**

| Component | lamco-rdp-server | lamco-VDI |
|-----------|------------------|-----------|
| **Compositor** | External (GNOME/KDE/wlroots) | Embedded Smithay (per-user instances) |
| **Portal** | External (xdg-desktop-portal-*) | Embedded (auto-grant, in-process) |
| **PipeWire** | Consumer (reads from compositor) | Producer (writes from Smithay) |
| **Session Management** | None (shares existing session) | Full lifecycle (create/suspend/destroy) |
| **Authentication** | Optional (or local user) | Required (PAM per user) |
| **Resource Limits** | None (shared with desktop) | Per-user cgroup limits |
| **Deployment** | User-space (run as logged-in user) | System service (multi-user daemon) |

---

## Migration Path

### Evolution Strategy

**Phase 1 (Now):** lamco-rdp-server v0.9.0
- Desktop sharing works
- Portal-based architecture proven
- RDP protocol stack mature
- Service Registry operational

**Phase 2 (Months 1-3):** lamco-rdp-server v1.0
- Session persistence strategies tested and verified
- Mutter Direct API tested on Ubuntu 22.04/24.04
- wlr-direct tested on Sway/Hyprland
- KDE Plasma tested
- Production-ready status confirmed

**Phase 3 (Months 4-6):** lamco-VDI Alpha
- Smithay compositor integration
- Embedded Portal backend
- Single-user headless proof-of-concept
- Shared RDP core from lamco-rdp-server v1.0

**Phase 4 (Months 7-9):** lamco-VDI Beta
- Multi-user session management
- PAM authentication
- Resource isolation
- systemd integration
- 10+ concurrent users supported

**Phase 5 (Months 10-12):** lamco-VDI v1.0
- Production hardening
- Enterprise features (LDAP, monitoring, audit logging)
- 50+ concurrent users supported
- Commercial support available

---

## Code Reuse Strategy

**Approach:** lamco-VDI will be a **separate product** built on lamco-rdp-server's foundation.

**Codebase Relationship:**

**Option A: Shared Workspace**
```
~/lamco-rdp-workspace/
├── crates/
│   ├── lamco-rdp-core/       # Shared RDP/EGFX/encoding
│   ├── lamco-portal-client/  # Portal client (desktop mode)
│   ├── lamco-portal-server/  # Embedded Portal (VDI mode)
│   ├── lamco-pipewire-consumer/
│   ├── lamco-pipewire-producer/
│   └── lamco-smithay-compositor/
├── lamco-rdp-server/         # Desktop product
│   └── Cargo.toml (uses lamco-rdp-core, portal-client, pipewire-consumer)
└── lamco-vdi/                # VDI product
    └── Cargo.toml (uses lamco-rdp-core, portal-server, pipewire-producer, smithay-compositor)
```

**Option B: Feature Flags**
```
lamco-rdp-server/
├── Cargo.toml
│   [features]
│   default = ["desktop-mode"]
│   desktop-mode = ["portal-client", "pipewire-consumer"]
│   vdi-mode = ["smithay", "portal-server", "pipewire-producer", "session-manager"]
└── src/
    ├── main.rs (mode selection)
    ├── desktop/ (existing)
    └── vdi/ (new)
```

**Recommendation:** Option A (separate products, shared crates) for clearer product differentiation and licensing flexibility.

---

## Licensing Strategy

### lamco-rdp-server
- **License:** Business Source License 1.1
- **Free Tier:** Personal use, ≤3 employees OR <$1M revenue
- **Commercial:** $4.99-$2,999 depending on scale
- **Conversion:** Apache 2.0 on December 31, 2028

### lamco-VDI (Future)
- **License:** TBD (likely BSL 1.1 with different pricing)
- **Free Tier:** TBD (possibly development/testing only)
- **Commercial:** Enterprise pricing (per concurrent user or per server)
- **Conversion:** Likely Apache 2.0 after 4 years

**Rationale for Separate Licensing:**
- Different target markets (desktop users vs enterprise)
- Different value propositions (convenience vs cost savings)
- Different support requirements (community vs enterprise SLA)

---

## Summary

### lamco-rdp-server
**What it is:** Remote desktop **client** for your existing Linux desktop
**Architecture:** Portal-based desktop sharing
**Target:** Individual users, remote workers, small teams
**Strength:** Easy setup, works with existing desktop, low barrier to entry
**Limitation:** Single-user, requires desktop environment, session persistence complex

### lamco-VDI
**What it is:** Multi-user VDI **server** for headless deployment
**Architecture:** Integrated Smithay compositor + embedded Portal
**Target:** Enterprises, cloud providers, service providers
**Strength:** Multi-user, headless, scalable, cloud-native, zero dialogs
**Development:** Planned (6-9 months to v1.0)

### Relationship
- **70% code shared** (RDP core, encoding, clipboard, input)
- **30% VDI-specific** (compositor, embedded Portal, session management)
- **Complementary products** serving different markets
- **Progressive enhancement** (lamco-rdp-server proves RDP stack, lamco-VDI adds multi-user headless)

---

**END OF POSITIONING DOCUMENT**
