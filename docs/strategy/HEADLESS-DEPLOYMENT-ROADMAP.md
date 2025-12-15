# WRD-Server Headless Deployment - Complete Roadmap & Architecture

**Document Version:** 1.0
**Date:** 2025-11-19
**Status:** APPROVED FOR IMPLEMENTATION
**Primary Focus:** Rust-native implementation with Smithay compositor
**Target Timeline:** 6-9 months to production-ready headless deployment

---

## EXECUTIVE SUMMARY

### The Opportunity

**Market Reality:**
- 95% of enterprise RDP servers run headless (no physical display)
- Cloud VMs with GPU cost $100-500 more per instance
- AWS WorkSpaces charges $35-75/user/month
- Open-source Linux VDI market is grossly underserved ($10B+ TAM)

**Current Limitation:**
WRD-Server requires full GNOME/KDE desktop environment, making it unsuitable for:
- Cloud/VPS deployment (headless servers)
- Enterprise VDI (thousands of concurrent users)
- Container-based deployment (Kubernetes, Docker)
- Cost-effective thin client infrastructure
- CI/CD with GUI testing

**Strategic Value:**
Headless deployment is the **#1 feature** for enterprise adoption. Without it, WRD-Server remains a desktop screen-sharing tool. With it, it becomes a **cloud-native VDI platform** competing with Citrix, VMware Horizon, and Microsoft RDS.

**Cost Comparison:**
```
Traditional VDI:
- Citrix Virtual Apps: $200-400/user/year licensing
- VMware Horizon: $150-300/user/year
- AWS WorkSpaces: $420-900/user/year
- Physical thin clients: $300-500 hardware + $100/year support

WRD-Server Headless:
- Software: $0 (open source)
- VPS hosting: $60-180/user/year (512MB RAM + 1 vCPU)
- Support: Optional commercial support model

Savings: 70-85% reduction in total cost of ownership
```

---

## ARCHITECTURE OVERVIEW

### Current Architecture (Desktop Mode)

```
┌─────────────────────────────────────────────────┐
│  Physical Linux Workstation                      │
│  • Monitor connected                            │
│  • Full GNOME/KDE desktop environment           │
│  • User logged in locally                       │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│  GNOME Mutter / KDE KWin (Compositor)            │
│  • Renders to physical display                  │
│  • Manages application windows                  │
│  • Handles user input from local devices        │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│  xdg-desktop-portal-gnome (External Process)     │
│  • D-Bus service providing Portal APIs          │
│  • Shows permission dialogs to user             │
│  • Requires GUI for user interaction            │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│  PipeWire (System Service)                       │
│  • Captures frames from compositor              │
│  • Provides video stream to consumers           │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│  WRD-Server                                      │
│  • Connects to Portal via D-Bus                 │
│  • Consumes PipeWire video stream               │
│  • Encodes and streams via RDP                  │
└─────────────────────────────────────────────────┘
```

**Limitations:**
- Requires physical or virtual GPU for compositor
- Needs full desktop environment (GNOME: ~800MB RAM minimum)
- Portal requires GUI for permission dialogs
- Cannot run in cloud VMs without X11 forwarding/VNC
- Not suitable for containerization
- High resource overhead per session

---

### Target Architecture (Headless Mode)

```
┌─────────────────────────────────────────────────┐
│  Headless Linux Server (Cloud VM / Bare Metal)   │
│  • No physical display attached                 │
│  • No desktop environment installed             │
│  • Minimal OS (Ubuntu Server / Debian)          │
│  • Optional: Software GPU (llvmpipe)            │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│  WRD Headless Compositor (Smithay-based)         │
│  • Minimal Wayland compositor in Rust           │
│  • Headless backend (no physical outputs)       │
│  • Virtual display rendering                    │
│  • Software rendering (Pixman/llvmpipe)         │
│  • OR hardware rendering (DRM/KMS + GPU)        │
│  • Integrated directly into wrd-server binary   │
│  • Memory footprint: < 50MB per session         │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│  Embedded Portal Backend (In-Process)            │
│  • D-Bus service embedded in wrd-server         │
│  • Implements ScreenCast + RemoteDesktop APIs   │
│  • Auto-grants permissions (no UI dialogs)      │
│  • Direct integration with compositor           │
│  • No external portal-gnome dependency          │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│  PipeWire Producer (In-Process)                  │
│  • Creates PipeWire stream from compositor      │
│  • Pushes frames from compositor framebuffer    │
│  • DMA-BUF or SHM based on GPU availability     │
│  • Integrated into wrd-server binary            │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│  WRD-Server Core (Existing)                      │
│  • RDP protocol handling (IronRDP)              │
│  • Video encoding (RemoteFX, future: H.264)     │
│  • Input injection (keyboard, mouse)            │
│  • Clipboard integration                        │
└─────────────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│  RDP Client (Windows/Linux/macOS)                │
└─────────────────────────────────────────────────┘
```

**Benefits:**
- Single binary deployment (all components integrated)
- Minimal resource footprint (256MB RAM + 256MB per session)
- No GUI required (runs on headless servers)
- Container-friendly (Docker, Kubernetes ready)
- Auto-scaling capable (cloud-native)
- Fast startup (< 2 seconds to ready)
- Cost-effective ($5-15/month VPS per user)

---

## TECHNOLOGY STACK ANALYSIS

### Component 1: Wayland Compositor

**Requirement:** Provide a Wayland compositor that renders to virtual displays instead of physical outputs.

#### **Option A: Smithay (Pure Rust) - RECOMMENDED**

**Repository:** https://github.com/Smithay/smithay
**Version:** 0.3.x (actively maintained)
**Language:** Pure Rust
**License:** MIT

**Architecture:**
```rust
use smithay::{
    backend::renderer::{
        gles::GlesRenderer,
        Frame, ImportDma, ImportShm, Renderer,
    },
    delegate_compositor, delegate_data_device, delegate_output,
    delegate_seat, delegate_shm, delegate_xdg_shell,
    desktop::{Space, Window},
    input::{Seat, SeatState},
    reexports::{
        calloop::EventLoop,
        wayland_server::{Display, DisplayHandle},
    },
    utils::{Physical, Size},
    wayland::{
        compositor::CompositorState,
        data_device::DataDeviceState,
        output::OutputManagerState,
        shell::xdg::XdgShellState,
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};

pub struct WrdCompositor {
    // Event loop
    event_loop: EventLoop<'static, WrdCompositorData>,

    // Wayland display
    display: Display<WrdCompositorData>,

    // Compositor state
    compositor_state: CompositorState,
    xdg_shell_state: XdgShellState,
    shm_state: ShmState,
    data_device_state: DataDeviceState,
    seat_state: SeatState<WrdCompositorData>,

    // Desktop management
    space: Space<Window>,

    // Rendering
    renderer: GlesRenderer,

    // Virtual output
    output_size: Size<i32, Physical>,

    // Integration with RDP
    frame_callback: Arc<Mutex<Option<Box<dyn Fn(Vec<u8>) + Send>>>>,
}
```

**Pros:**
- ✅ Pure Rust (memory safety, no FFI overhead)
- ✅ Modular architecture (use only what you need)
- ✅ Active development (frequent updates)
- ✅ Good documentation and examples
- ✅ Used by production compositors (Cosmic, Niri)
- ✅ Supports both software and hardware rendering
- ✅ Can integrate directly into wrd-server binary
- ✅ Full control over compositor behavior
- ✅ Excellent for RDP-specific optimizations

**Cons:**
- ⚠️ Relatively new (less battle-tested than wlroots)
- ⚠️ Steeper learning curve (compositor development is complex)
- ⚠️ Need to implement all Wayland protocols ourselves
- ⚠️ More code to write initially

**Estimated Effort:** 10-12 weeks for full implementation

**Code Structure:**
```
src/headless/
  ├── compositor/
  │   ├── mod.rs              - Main compositor struct
  │   ├── backend.rs          - Headless backend (no physical output)
  │   ├── renderer.rs         - Rendering (GLES2 or Pixman)
  │   ├── protocols.rs        - Wayland protocol implementations
  │   ├── shell.rs            - XDG shell (window management)
  │   ├── seat.rs             - Virtual seat (keyboard/mouse)
  │   ├── output.rs           - Virtual output management
  │   └── data_device.rs      - Clipboard/DND support
  ├── window_manager.rs       - Window layout and management
  ├── surface_manager.rs      - Surface tracking and composition
  ├── frame_capture.rs        - Capture framebuffer for PipeWire
  └── config.rs               - Compositor configuration
```

---

#### **Option B: wlroots (C library with Rust bindings)**

**Repository:** https://gitlab.freedesktop.org/wlroots/wlroots
**Bindings:** https://github.com/swaywm/wlroots-rs (ARCHIVED!)
**Alternative:** https://github.com/psychon/wlroots-rs
**Version:** wlroots 0.17.x
**Language:** C with Rust FFI bindings
**License:** MIT

**Architecture:**
```rust
use wlroots::{
    Backend, Compositor, Renderer, Output, Seat,
    HeadlessBackend, GlRenderer,
};

pub struct WlrootsCompositor {
    backend: HeadlessBackend,
    compositor: Compositor,
    renderer: GlRenderer,
    output: Output,
    seat: Seat,
}

impl WlrootsCompositor {
    pub fn new_headless(width: u32, height: u32) -> Result<Self> {
        // Initialize headless backend
        let backend = HeadlessBackend::new()?;

        // Add virtual output
        let output = backend.add_output(width, height)?;

        // Create renderer
        let renderer = GlRenderer::new(&backend)?;

        // Initialize compositor
        let compositor = Compositor::new(&backend, &renderer)?;

        Ok(Self { backend, compositor, renderer, output, seat })
    }

    pub fn render_frame(&mut self) -> Result<Vec<u8>> {
        // Render to framebuffer and return pixels
    }
}
```

**Pros:**
- ✅ Battle-tested (used by Sway, Wayfire, River)
- ✅ Full-featured (all protocols implemented)
- ✅ Headless backend built-in
- ✅ Well-documented (C library docs)
- ✅ Mature and stable

**Cons:**
- ❌ C FFI overhead and unsafe code
- ❌ Rust bindings are unmaintained (wlroots-rs archived in 2023!)
- ❌ Would need to maintain own Rust bindings
- ❌ Harder to integrate into single binary
- ❌ External dependency management
- ❌ Less control over compositor internals
- ❌ Memory safety relies on correct FFI usage

**Estimated Effort:** 8-10 weeks (if bindings work), 16+ weeks (if need to create/maintain bindings)

**Risk Assessment:** **HIGH** - Dependency on unmaintained Rust bindings is a serious concern.

---

#### **Option C: Weston (External Process)**

**Repository:** https://gitlab.freedesktop.org/wayland/weston
**Version:** 13.0+
**Language:** C (separate process)
**License:** MIT

**Architecture:**
```bash
# Launch Weston in headless mode
weston \
  --backend=headless-backend.so \
  --width=1920 \
  --height=1080 \
  --use-pixman \
  --socket=wrd-wayland-0 \
  &

# WRD-Server connects to Weston's socket
WAYLAND_DISPLAY=wrd-wayland-0 wrd-server -c headless.toml
```

**Integration:**
```rust
// src/headless/weston_launcher.rs
use std::process::{Command, Child};

pub struct WestonLauncher {
    process: Child,
    socket_name: String,
}

impl WestonLauncher {
    pub fn start(width: u32, height: u32) -> Result<Self> {
        let socket_name = format!("wrd-wayland-{}", std::process::id());

        let process = Command::new("weston")
            .arg("--backend=headless-backend.so")
            .arg(format!("--width={}", width))
            .arg(format!("--height={}", height))
            .arg("--use-pixman")
            .arg(format!("--socket={}", socket_name))
            .env("XDG_RUNTIME_DIR", "/run/user/1000")
            .spawn()?;

        // Wait for socket to be ready
        Self::wait_for_socket(&socket_name)?;

        Ok(Self { process, socket_name })
    }

    pub fn socket_name(&self) -> &str {
        &self.socket_name
    }
}

impl Drop for WestonLauncher {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}
```

**Pros:**
- ✅ Quickest to implement (2-3 weeks)
- ✅ Extremely mature and stable
- ✅ Reference Wayland compositor
- ✅ Headless backend well-tested
- ✅ No compositor code to write

**Cons:**
- ❌ External process (separate lifecycle)
- ❌ Higher resource usage (~100MB RAM)
- ❌ Not integrated into wrd-server binary
- ❌ Dependency on system package
- ❌ Less control over rendering
- ❌ No RDP-specific optimizations

**Estimated Effort:** 2-3 weeks for integration

**Use Case:** Good for **proof-of-concept** or **initial release**, but Smithay is better long-term.

---

#### **Recommendation: Phased Approach**

**Phase 1 (Months 1-2): Weston Prototype**
- Use Weston headless backend
- Validate architecture works
- Test with real workloads
- Get user feedback

**Phase 2 (Months 3-6): Smithay Implementation**
- Implement Smithay-based compositor
- Integrate into wrd-server binary
- Optimize for RDP use case
- Maintain Weston as fallback option

**Configuration:**
```toml
[headless]
compositor = "smithay"  # or "weston" for fallback
```

This approach reduces risk while building toward optimal solution.

---

### Component 2: Portal Backend

**Requirement:** Provide ScreenCast and RemoteDesktop Portal D-Bus interfaces without external xdg-desktop-portal-gnome dependency.

#### **Option A: Embedded D-Bus Service (In-Process) - RECOMMENDED**

**Technology:** `zbus` crate for D-Bus service implementation
**Repository:** https://github.com/dbus2/zbus
**Version:** zbus 4.x

**Architecture:**
```rust
use zbus::{dbus_interface, ConnectionBuilder, SignalContext};
use zbus::zvariant::{ObjectPath, OwnedObjectPath};

// Implement org.freedesktop.portal.ScreenCast
pub struct ScreenCastPortal {
    compositor: Arc<Mutex<WrdCompositor>>,
    sessions: HashMap<OwnedObjectPath, ScreenCastSession>,
}

#[dbus_interface(name = "org.freedesktop.portal.ScreenCast")]
impl ScreenCastPortal {
    async fn create_session(
        &mut self,
        #[zbus(signal_context)] ctx: SignalContext<'_>,
        options: HashMap<String, zvariant::Value<'_>>,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        // Create new session
        let session_path = format!(
            "/org/freedesktop/portal/desktop/session/wrd_{}",
            uuid::Uuid::new_v4()
        );

        let session = ScreenCastSession::new(
            Arc::clone(&self.compositor),
            options,
        );

        self.sessions.insert(session_path.clone().into(), session);

        Ok(session_path.into())
    }

    async fn select_sources(
        &mut self,
        session_handle: ObjectPath<'_>,
        options: HashMap<String, zvariant::Value<'_>>,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        // Auto-grant: select all available sources
        let session = self.sessions.get_mut(&session_handle.into())
            .ok_or_else(|| zbus::fdo::Error::Failed("Invalid session".into()))?;

        session.select_all_sources()?;

        // Return request path
        Ok("/org/freedesktop/portal/desktop/request/wrd_1".into())
    }

    async fn start(
        &mut self,
        session_handle: ObjectPath<'_>,
        parent_window: &str,
        options: HashMap<String, zvariant::Value<'_>>,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        // Start streaming
        let session = self.sessions.get_mut(&session_handle.into())
            .ok_or_else(|| zbus::fdo::Error::Failed("Invalid session".into()))?;

        session.start_streaming()?;

        Ok("/org/freedesktop/portal/desktop/request/wrd_2".into())
    }
}

// Similarly implement org.freedesktop.portal.RemoteDesktop
pub struct RemoteDesktopPortal {
    compositor: Arc<Mutex<WrdCompositor>>,
}

#[dbus_interface(name = "org.freedesktop.portal.RemoteDesktop")]
impl RemoteDesktopPortal {
    async fn notify_pointer_motion(
        &mut self,
        session_handle: ObjectPath<'_>,
        options: HashMap<String, zvariant::Value<'_>>,
        dx: f64,
        dy: f64,
    ) -> zbus::fdo::Result<()> {
        // Inject into compositor
        let mut compositor = self.compositor.lock().await;
        compositor.inject_pointer_motion(dx, dy)?;
        Ok(())
    }

    // ... other methods
}
```

**Service Registration:**
```rust
// src/headless/portal_service.rs
pub async fn start_portal_service(
    compositor: Arc<Mutex<WrdCompositor>>,
) -> Result<()> {
    // Build D-Bus connection
    let connection = ConnectionBuilder::session()?
        .name("org.freedesktop.portal.Desktop")?
        .serve_at(
            "/org/freedesktop/portal/desktop",
            ScreenCastPortal::new(Arc::clone(&compositor)),
        )?
        .serve_at(
            "/org/freedesktop/portal/desktop",
            RemoteDesktopPortal::new(Arc::clone(&compositor)),
        )?
        .build()
        .await?;

    // Keep service running
    loop {
        connection.executor().tick().await;
    }
}
```

**Pros:**
- ✅ Fully integrated into wrd-server
- ✅ Single binary deployment
- ✅ No external dependencies
- ✅ Auto-grant permissions (no UI)
- ✅ Direct compositor integration
- ✅ Full control over Portal behavior

**Cons:**
- ⚠️ Complex D-Bus interface implementation
- ⚠️ Must implement all Portal APIs correctly
- ⚠️ Need to handle D-Bus session bus registration

**Estimated Effort:** 4-6 weeks

---

#### **Option B: Custom Portal Backend (Separate Process)**

**Technology:** Fork xdg-desktop-portal-wlr and modify
**Repository:** https://github.com/emersion/xdg-desktop-portal-wlr

**Architecture:**
```
wrd-portal-backend (separate binary)
  ↓
Communicates with wrd-compositor via custom protocol
  ↓
Provides Portal D-Bus interfaces
  ↓
wrd-server connects as normal Portal client
```

**Pros:**
- ✅ Follows standard Portal architecture
- ✅ Can leverage existing xdg-desktop-portal-wlr code
- ✅ Separation of concerns

**Cons:**
- ❌ Extra process to manage
- ❌ More complex deployment
- ❌ Need to maintain forked portal code
- ❌ Still need custom protocol between compositor and portal

**Estimated Effort:** 6-8 weeks

**Recommendation:** Not worth it - Option A (embedded) is better.

---

#### **Option C: Use xdg-desktop-portal-wlr (External)**

**Technology:** Use existing xdg-desktop-portal-wlr with configuration

**Configuration:**
```ini
# ~/.config/xdg-desktop-portal-wlr/config
[screencast]
chooser_type = simple
chooser_cmd = /bin/true  # Auto-approve

[remote-desktop]
# Auto-grant all permissions
```

**Pros:**
- ✅ No code to write
- ✅ Proven and tested

**Cons:**
- ❌ External dependency
- ❌ May not auto-grant properly
- ❌ Designed for wlroots compositors
- ❌ Won't work well with Smithay

**Recommendation:** Not suitable for production.

---

**Final Recommendation:** **Option A (Embedded D-Bus Service)**

This provides the cleanest architecture and best integration.

---

### Component 3: PipeWire Integration

**Requirement:** Create PipeWire video streams from compositor output instead of consuming from external compositor.

#### **Current Architecture (Consumer)**

```rust
// Current: wrd-server consumes PipeWire stream from GNOME
let stream = pipewire::stream::Stream::new(
    &core,
    "wrd-capture",
    properties,
)?;

stream.connect(
    pipewire::spa::Direction::Input,  // We consume
    node_id,  // GNOME's PipeWire node
)?;

// Receive frames
stream.add_listener_local()
    .on_process(|stream| {
        let buffer = stream.dequeue_buffer()?;
        // Process frame
    })
    .register()?;
```

---

#### **Target Architecture (Producer)**

```rust
// New: wrd-server produces PipeWire stream from compositor
use pipewire::{
    prelude::*,
    stream::{Stream, StreamFlags, StreamListener},
    spa::{
        param::video::VideoInfoRaw,
        pod::Pod,
        utils::Direction,
    },
};

pub struct PipeWireProducer {
    stream: Stream,
    compositor: Arc<Mutex<WrdCompositor>>,
}

impl PipeWireProducer {
    pub fn new(
        context: &pipewire::context::Context,
        compositor: Arc<Mutex<WrdCompositor>>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        // Create output stream
        let stream = Stream::new(
            context,
            "wrd-compositor-output",
            properties! {
                *pipewire::keys::MEDIA_TYPE => "Video",
                *pipewire::keys::MEDIA_CATEGORY => "Capture",
                *pipewire::keys::MEDIA_ROLE => "Screen",
            },
        )?;

        // Configure video format
        let video_info = VideoInfoRaw::new()
            .format(pipewire::spa::param::video::VideoFormat::BGRx)
            .size(width, height)
            .framerate(pipewire::spa::utils::Fraction::new(60, 1))
            .build();

        // Build params
        let params = [Pod::from_bytes(&video_info.to_pod_bytes())?.into()];

        // Connect as source
        stream.connect(
            Direction::Output,  // We produce
            None,  // No specific target node
            StreamFlags::DRIVER | StreamFlags::ALLOC_BUFFERS,
            &params,
        )?;

        Ok(Self { stream, compositor })
    }

    pub fn push_frame(&mut self) -> Result<()> {
        // Get buffer from PipeWire
        let mut buffer = self.stream.dequeue_buffer()?;

        // Render compositor frame
        let compositor_frame = {
            let mut compositor = self.compositor.lock().unwrap();
            compositor.render_frame()?
        };

        // Copy to PipeWire buffer
        let data = buffer.datas_mut()[0].data();
        data.copy_from_slice(&compositor_frame);

        // Queue buffer
        self.stream.queue_buffer(buffer)?;

        Ok(())
    }
}
```

**Integration with Compositor:**
```rust
impl WrdCompositor {
    pub fn render_frame(&mut self) -> Result<Vec<u8>> {
        // Render all windows to framebuffer
        let mut frame = self.renderer.render(
            self.output_size,
            Transform::Normal,
        )?;

        // Draw each window
        for window in &self.space.windows() {
            window.draw(
                &mut frame,
                &mut self.renderer,
                1.0,  // Scale
            )?;
        }

        // Read pixels from framebuffer
        let pixels = frame.read_pixels(
            self.output_size,
            PixelFormat::Bgrx8888,
        )?;

        Ok(pixels)
    }
}
```

**Frame Timing:**
```rust
// Sync with compositor refresh rate
pub async fn run_frame_loop(
    mut producer: PipeWireProducer,
    target_fps: u32,
) {
    let frame_duration = Duration::from_micros(1_000_000 / target_fps as u64);
    let mut interval = tokio::time::interval(frame_duration);

    loop {
        interval.tick().await;

        if let Err(e) = producer.push_frame() {
            error!("Failed to push frame: {}", e);
        }
    }
}
```

**Estimated Effort:** 3-4 weeks

---

### Component 4: Session Management

**Requirement:** Multi-user session isolation, authentication, resource management.

#### **Architecture**

```rust
// src/session/manager.rs
use nix::unistd::{User, Uid, Gid};
use std::collections::HashMap;

pub struct SessionManager {
    sessions: HashMap<String, UserSession>,
    systemd: SystemdLogind,
}

pub struct UserSession {
    username: String,
    uid: Uid,
    gid: Gid,
    home_dir: PathBuf,

    // Wayland display
    wayland_display: String,
    compositor_pid: u32,

    // WRD server instance
    wrd_server_pid: u32,

    // Resource limits
    cgroup: CGroup,

    // Session state
    state: SessionState,
    created_at: SystemTime,
    last_activity: SystemTime,
}

#[derive(Debug, Clone, Copy)]
pub enum SessionState {
    Starting,
    Active,
    Disconnected,
    Terminating,
}

impl SessionManager {
    pub async fn create_session(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<UserSession> {
        // 1. Authenticate via PAM
        self.authenticate_user(username, password).await?;

        // 2. Get user info
        let user = User::from_name(username)?
            .ok_or_else(|| anyhow!("User not found"))?;

        // 3. Create systemd-logind session
        let session_id = self.systemd
            .create_session(&user.name, SessionType::User)
            .await?;

        // 4. Set up environment
        let runtime_dir = format!("/run/user/{}", user.uid);
        let wayland_display = format!("wayland-wrd-{}", session_id);

        std::fs::create_dir_all(&runtime_dir)?;

        let env = vec![
            ("USER", user.name.clone()),
            ("HOME", user.dir.to_string_lossy().to_string()),
            ("SHELL", user.shell.to_string_lossy().to_string()),
            ("XDG_RUNTIME_DIR", runtime_dir.clone()),
            ("WAYLAND_DISPLAY", wayland_display.clone()),
            ("XDG_SESSION_TYPE", "wayland".to_string()),
        ];

        // 5. Create cgroup for resource limits
        let cgroup = CGroup::new(&format!("wrd-session-{}", session_id))?;
        cgroup.set_memory_limit(2 * 1024 * 1024 * 1024)?; // 2GB
        cgroup.set_cpu_shares(1024)?;

        // 6. Spawn compositor as user
        let compositor_pid = self.spawn_compositor_as_user(
            &user,
            &env,
            &cgroup,
        ).await?;

        // 7. Wait for compositor ready
        self.wait_for_wayland_socket(&runtime_dir, &wayland_display).await?;

        // 8. Spawn wrd-server instance
        let wrd_server_pid = self.spawn_wrd_server_as_user(
            &user,
            &env,
            &cgroup,
        ).await?;

        // 9. Create session object
        let session = UserSession {
            username: user.name,
            uid: user.uid,
            gid: user.gid,
            home_dir: user.dir,
            wayland_display,
            compositor_pid,
            wrd_server_pid,
            cgroup,
            state: SessionState::Active,
            created_at: SystemTime::now(),
            last_activity: SystemTime::now(),
        };

        self.sessions.insert(session_id, session.clone());

        Ok(session)
    }

    async fn spawn_compositor_as_user(
        &self,
        user: &User,
        env: &[(impl AsRef<str>, impl AsRef<str>)],
        cgroup: &CGroup,
    ) -> Result<u32> {
        use nix::unistd::{fork, setuid, setgid, ForkResult};

        match unsafe { fork()? } {
            ForkResult::Parent { child } => {
                // Add child to cgroup
                cgroup.add_process(child.as_raw() as u32)?;
                Ok(child.as_raw() as u32)
            }
            ForkResult::Child => {
                // Drop privileges
                setgid(user.gid)?;
                setuid(user.uid)?;

                // Set environment
                for (key, value) in env {
                    std::env::set_var(key.as_ref(), value.as_ref());
                }

                // Exec compositor (integrated mode)
                // In practice, this would be a function call, not exec
                // since compositor is integrated into wrd-server

                std::process::exit(0);
            }
        }
    }
}
```

**PAM Authentication:**
```rust
// src/session/auth.rs
use pam::{Authenticator, PamReturnCode};

pub async fn authenticate_user(
    username: &str,
    password: &str,
) -> Result<()> {
    let mut auth = Authenticator::with_password("wrd-server")?;

    auth.get_handler().set_credentials(username, password);

    match auth.authenticate() {
        Ok(()) => Ok(()),
        Err(e) => Err(anyhow!("Authentication failed: {:?}", e)),
    }
}
```

**systemd-logind Integration:**
```rust
// src/session/systemd_logind.rs
use zbus::Connection;

pub struct SystemdLogind {
    connection: Connection,
}

impl SystemdLogind {
    pub async fn new() -> Result<Self> {
        let connection = Connection::system().await?;
        Ok(Self { connection })
    }

    pub async fn create_session(
        &self,
        username: &str,
        session_type: SessionType,
    ) -> Result<String> {
        let proxy = systemd_logind::ManagerProxy::new(&self.connection).await?;

        let (session_id, _object_path) = proxy.create_session(
            "",  // Auto-assign UID
            0,   // Auto-assign PID
            username,
            "",  // Seat (empty for headless)
            0,   // VT number (0 for headless)
            "",  // Display (empty)
            false, // Remote
            "",  // Remote user
            "",  // Remote host
            &[],  // Properties
        ).await?;

        Ok(session_id)
    }
}
```

**Estimated Effort:** 6-8 weeks for complete session management

---

### Component 5: Application Launcher

**Requirement:** Auto-start applications in user sessions.

```rust
// src/apps/launcher.rs
use tokio::process::Command;

pub struct AppLauncher {
    compositor: Arc<Mutex<WrdCompositor>>,
    apps: Vec<AppConfig>,
    running_apps: HashMap<String, AppInstance>,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    name: String,
    command: String,
    args: Vec<String>,
    restart_on_crash: bool,
    environment: HashMap<String, String>,
}

pub struct AppInstance {
    config: AppConfig,
    pid: u32,
    started_at: SystemTime,
    restart_count: u32,
}

impl AppLauncher {
    pub async fn launch_app(&mut self, config: &AppConfig) -> Result<u32> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);

        // Set environment
        for (key, value) in &config.environment {
            cmd.env(key, value);
        }

        // Ensure WAYLAND_DISPLAY is set
        if !config.environment.contains_key("WAYLAND_DISPLAY") {
            cmd.env("WAYLAND_DISPLAY", self.get_wayland_display());
        }

        // Spawn process
        let child = cmd.spawn()?;
        let pid = child.id().ok_or_else(|| anyhow!("No PID"))?;

        // Track instance
        let instance = AppInstance {
            config: config.clone(),
            pid,
            started_at: SystemTime::now(),
            restart_count: 0,
        };

        self.running_apps.insert(config.name.clone(), instance);

        // Monitor for crashes if restart enabled
        if config.restart_on_crash {
            self.monitor_app(config.name.clone(), child);
        }

        Ok(pid)
    }

    async fn monitor_app(&self, name: String, mut child: tokio::process::Child) {
        tokio::spawn(async move {
            match child.wait().await {
                Ok(status) if !status.success() => {
                    warn!("App {} exited with status: {}", name, status);
                    // Restart logic here
                }
                Err(e) => {
                    error!("Failed to wait for app {}: {}", name, e);
                }
                _ => {}
            }
        });
    }

    pub async fn launch_all(&mut self) -> Result<()> {
        for config in &self.apps.clone() {
            if let Err(e) = self.launch_app(config).await {
                error!("Failed to launch {}: {}", config.name, e);
            }
        }
        Ok(())
    }
}
```

**Configuration:**
```toml
[[headless.apps]]
name = "terminal"
command = "/usr/bin/gnome-terminal"
args = []
restart_on_crash = true

[[headless.apps]]
name = "browser"
command = "/usr/bin/firefox"
args = ["--new-window"]
restart_on_crash = false

[[headless.apps]]
name = "vscode"
command = "/usr/bin/code"
args = []
restart_on_crash = true
```

**Estimated Effort:** 2-3 weeks

---

## IMPLEMENTATION ROADMAP

### **Phase 1: Proof of Concept (Weeks 1-4)**

**Goal:** Validate headless architecture with Weston

**Tasks:**
1. **Week 1: Weston Integration**
   - Implement WestonLauncher process manager
   - Test Weston headless backend launch
   - Verify Wayland socket creation
   - Connect wrd-server to Weston socket

2. **Week 2: Portal Integration**
   - Configure xdg-desktop-portal-wlr for auto-grant
   - Test Portal APIs with Weston
   - Verify screen capture works headless
   - Test input injection

3. **Week 3: Basic Session Management**
   - Single-user session creation
   - Environment setup (XDG_RUNTIME_DIR, etc.)
   - Application launcher (launch terminal, browser)
   - Test end-to-end RDP connection

4. **Week 4: Testing & Validation**
   - Deploy to headless VPS
   - Test from Windows RDP client
   - Measure performance (CPU, memory, latency)
   - Document findings and issues

**Deliverable:** Working proof-of-concept with Weston

**Success Criteria:**
- ✅ Connect to headless server via RDP
- ✅ See applications running (terminal, browser)
- ✅ Mouse and keyboard work
- ✅ Resource usage < 512MB RAM
- ✅ Startup time < 30 seconds

---

### **Phase 2: Smithay Compositor (Weeks 5-14)**

**Goal:** Replace Weston with integrated Smithay compositor

**Tasks:**

**Weeks 5-6: Smithay Foundation**
- Learn Smithay architecture
- Create minimal compositor skeleton
- Implement headless backend
- Test basic rendering

**Weeks 7-8: Wayland Protocols**
- Implement XDG shell protocol
- Implement compositor protocol
- Implement shm (shared memory) protocol
- Implement data device (clipboard/DND)

**Weeks 9-10: Window Management**
- Surface tracking and management
- Window focus and stacking
- Input event routing to windows
- Damage tracking

**Weeks 11-12: Rendering Pipeline**
- Implement software renderer (Pixman)
- Framebuffer capture for PipeWire
- Test rendering performance
- Optional: Add GLES2 hardware rendering

**Weeks 13-14: Integration & Testing**
- Integrate compositor into wrd-server binary
- Replace Weston with Smithay
- Test all functionality
- Performance tuning

**Deliverable:** Smithay-based integrated compositor

**Success Criteria:**
- ✅ Single binary deployment
- ✅ All features work (video, input, clipboard)
- ✅ Memory usage < 256MB for compositor
- ✅ Startup time < 5 seconds
- ✅ Comparable or better performance than Weston

---

### **Phase 3: Embedded Portal (Weeks 15-20)**

**Goal:** Replace xdg-desktop-portal-wlr with embedded D-Bus service

**Tasks:**

**Weeks 15-16: D-Bus Service**
- Implement D-Bus service with zbus
- Register org.freedesktop.portal.Desktop
- Implement basic session management
- Test D-Bus interface from wrd-server

**Weeks 17-18: ScreenCast Portal**
- Implement CreateSession
- Implement SelectSources (auto-grant)
- Implement Start
- Direct integration with compositor

**Week 19: RemoteDesktop Portal**
- Implement NotifyPointerMotion
- Implement NotifyPointerButton
- Implement NotifyKeyboardKeycode
- Direct injection into compositor

**Week 20: Integration & Testing**
- Replace external portal with embedded
- Test all Portal APIs
- Verify auto-grant works
- End-to-end testing

**Deliverable:** Embedded Portal backend

**Success Criteria:**
- ✅ No external portal dependency
- ✅ Auto-grant permissions work
- ✅ All Portal APIs functional
- ✅ Direct compositor integration
- ✅ Single binary deployment maintained

---

### **Phase 4: PipeWire Producer (Weeks 21-24)**

**Goal:** Produce PipeWire streams from compositor instead of consuming

**Tasks:**

**Week 21: PipeWire Producer**
- Create PipeWire output stream
- Configure video format params
- Test stream creation

**Week 22: Frame Pushing**
- Render compositor frame to buffer
- Push frames to PipeWire
- Implement frame timing
- Test stream consumption by wrd-server

**Week 23: Format Negotiation**
- Support multiple pixel formats
- DMA-BUF export (if GPU available)
- SHM fallback
- Format conversion if needed

**Week 24: Integration & Optimization**
- Integrate into compositor render loop
- Optimize frame timing
- Reduce latency
- Performance testing

**Deliverable:** Compositor-integrated PipeWire producer

**Success Criteria:**
- ✅ Frames flow from compositor to wrd-server
- ✅ No external PipeWire dependency
- ✅ Low latency (< 16ms added)
- ✅ Supports DMA-BUF and SHM
- ✅ Clean integration

---

### **Phase 5: Multi-User & Production (Weeks 25-36)**

**Goal:** Enterprise-ready multi-user session management

**Tasks:**

**Weeks 25-27: Session Management**
- PAM authentication integration
- systemd-logind session creation
- User environment setup
- Per-user compositor instances
- Session isolation

**Weeks 28-29: Resource Management**
- cgroup integration
- Memory limits per session
- CPU share allocation
- Network bandwidth limits
- Enforce limits

**Weeks 30-31: Session Lifecycle**
- Session creation
- Session suspension
- Session reconnection
- Session termination
- Cleanup on exit

**Weeks 32-33: systemd Integration**
- systemd service units
- Socket activation
- User session templates
- Auto-start on boot
- Service management

**Weeks 34-35: Monitoring & Management**
- Health check endpoints
- Prometheus metrics
- Structured logging
- Session status API
- Admin tools

**Week 36: Production Hardening**
- Security audit
- Performance tuning
- Load testing
- Documentation
- Release preparation

**Deliverable:** Production-ready multi-user headless deployment

**Success Criteria:**
- ✅ Support 10+ concurrent users
- ✅ PAM authentication works
- ✅ Resource limits enforced
- ✅ Session reconnection works
- ✅ Auto-start on boot
- ✅ Monitoring integrated
- ✅ Ready for enterprise deployment

---

## DEPENDENCIES

### **Rust Crates Required**

```toml
[dependencies]
# Existing dependencies
ironrdp-server = { git = "https://github.com/allan2/IronRDP", branch = "update-sspi" }
ashpd = "0.12"
pipewire = "0.8"
tokio = { version = "1.35", features = ["full"] }
anyhow = "1.0"
tracing = "0.1"

# Compositor (Smithay)
smithay = "0.3"
smithay-client-toolkit = "0.18"
drm = "0.12"
gbm = "0.15"
udev = "0.8"
input = "0.9"
xkbcommon = "0.7"

# Rendering
pixman = "0.1"  # Software rendering
glow = "0.13"    # OpenGL bindings (for GLES2)

# Portal backend
zbus = "4.0"
zvariant = "4.0"
futures = "0.3"

# Session management
pam = "0.7"
nix = { version = "0.29", features = ["user", "process", "signal"] }
caps = "0.5"
procfs = "0.16"

# Configuration
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"

# Logging & monitoring
tracing-subscriber = "0.3"
tracing-appender = "0.2"
prometheus = "0.13"  # Optional

# Utilities
uuid = { version = "1.0", features = ["v4"] }
libc = "0.2"
```

### **System Dependencies**

```bash
# Build dependencies
sudo apt-get install \
    libpam0g-dev \
    libsystemd-dev \
    libudev-dev \
    libinput-dev \
    libxkbcommon-dev \
    libdrm-dev \
    libgbm-dev \
    libgl1-mesa-dev \
    libgles2-mesa-dev \
    libpixman-1-dev \
    libclang-dev

# Runtime dependencies
sudo apt-get install \
    libpam0g \
    libsystemd0 \
    libudev1 \
    libinput10 \
    libxkbcommon0 \
    libdrm2 \
    libgbm1 \
    libgl1 \
    libgles2 \
    libpixman-1-0 \
    pipewire \
    mesa-utils  # For llvmpipe software rendering
```

---

## ALTERNATIVE APPROACHES

### **Hybrid Approach: Smithay + External Portal**

**Architecture:**
- Use Smithay compositor (integrated)
- Keep xdg-desktop-portal-wlr (external)
- Configure auto-grant via config file

**Pros:**
- Simpler Portal implementation
- Leverage existing tested Portal backend
- Focus effort on compositor quality

**Cons:**
- Still have external dependency
- Auto-grant may not work reliably
- Harder to deploy in containers

**Recommendation:** Good intermediate step if embedded Portal proves too complex.

---

### **Container-First Approach**

**Architecture:**
- Package everything in Docker container
- Use init system (s6, tini) for process management
- Pre-configure all dependencies

**Dockerfile:**
```dockerfile
FROM ubuntu:24.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    libpam0g libsystemd0 pipewire \
    mesa-utils  # llvmpipe

# Copy wrd-server binary
COPY target/release/wrd-server /usr/bin/
COPY config.toml /etc/wrd-server/

# Create runtime directory
RUN mkdir -p /run/user/1000
ENV XDG_RUNTIME_DIR=/run/user/1000

# Expose RDP port
EXPOSE 3389

# Run wrd-server
CMD ["/usr/bin/wrd-server", "-c", "/etc/wrd-server/config.toml"]
```

**Kubernetes Deployment:**
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: wrd-server
spec:
  replicas: 5
  selector:
    matchLabels:
      app: wrd-server
  template:
    metadata:
      labels:
        app: wrd-server
    spec:
      containers:
      - name: wrd-server
        image: ghcr.io/lamco-admin/wrd-server:headless
        ports:
        - containerPort: 3389
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
---
apiVersion: v1
kind: Service
metadata:
  name: wrd-server-lb
spec:
  type: LoadBalancer
  selector:
    app: wrd-server
  ports:
  - protocol: TCP
    port: 3389
    targetPort: 3389
```

**Pros:**
- Cloud-native deployment
- Easy scaling (horizontal)
- Isolation by default
- Standard tooling

**Cons:**
- Container overhead
- Complexity for simple deployments
- Need container registry

**Recommendation:** Target this for Phase 6 (post-MVP) for cloud deployments.

---

## PERFORMANCE TARGETS

### **Resource Usage**

| Metric | Target | Stretch Goal |
|--------|--------|--------------|
| Base memory (no sessions) | < 128 MB | < 64 MB |
| Memory per session | < 256 MB | < 128 MB |
| CPU idle (no clients) | < 5% | < 2% |
| CPU per session (active) | < 25% | < 15% |
| Startup time | < 5 sec | < 2 sec |
| Frame latency | < 50 ms | < 30 ms |

### **Scalability**

| Metric | Target | Stretch Goal |
|--------|--------|--------------|
| Concurrent users (4 vCPU) | 10 users | 20 users |
| Concurrent users (8 vCPU) | 25 users | 50 users |
| Sessions per GB RAM | 3-4 sessions | 6-8 sessions |

### **Deployment Cost**

| Configuration | Monthly Cost | Users Supported |
|---------------|--------------|-----------------|
| Small VPS (2 vCPU, 4GB RAM) | $15-20 | 8-10 users |
| Medium VPS (4 vCPU, 8GB RAM) | $40-60 | 20-25 users |
| Large VPS (8 vCPU, 16GB RAM) | $80-120 | 50-60 users |

**Comparison:** AWS WorkSpaces = $35-75 per user/month

---

## RISKS & MITIGATION

### **Technical Risks**

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Smithay learning curve steeper than expected | High | High | Start with Weston PoC first; allocate extra time for learning |
| Portal auto-grant doesn't work | Medium | High | Implement embedded Portal from start; test early |
| PipeWire producer complex | Medium | Medium | Leverage existing PipeWire examples; start simple |
| Session isolation issues | Medium | High | Use proven Linux tools (cgroups, namespaces); test thoroughly |
| Performance worse than desktop | Low | Medium | Profile early; optimize hot paths; consider hardware acceleration |

### **Resource Risks**

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Development takes longer than 9 months | Medium | Medium | Phased approach allows early releases; prioritize MVP |
| Insufficient testing resources | Medium | High | Automate testing; use CI/CD; recruit beta testers |
| Documentation lags behind code | High | Medium | Document as you build; use rustdoc extensively |

### **Market Risks**

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Low adoption despite headless | Low | High | Market validation shows clear demand; focus on use cases |
| Competitors release similar | Low | Medium | Open source advantage; community building; rapid iteration |
| Enterprise reluctant to adopt | Medium | High | Provide commercial support option; security audits; compliance certifications |

---

## SUCCESS METRICS

### **Technical Metrics**

**Phase 1 (PoC) Success:**
- ✅ RDP connection works headless
- ✅ Resource usage < 512MB RAM total
- ✅ All core features work (video, input, clipboard)
- ✅ Can run on $15/month VPS

**Phase 2-4 (Integrated) Success:**
- ✅ Single binary deployment
- ✅ No external dependencies (except system libs)
- ✅ Memory < 256MB per session
- ✅ Startup < 5 seconds

**Phase 5 (Production) Success:**
- ✅ 10+ concurrent users supported
- ✅ Auto-start on boot working
- ✅ Session reconnection working
- ✅ Resource limits enforced
- ✅ Security audit passed

### **Adoption Metrics**

**6 Months:**
- 100+ GitHub stars
- 10+ production deployments
- 5+ community contributors
- 1+ enterprise pilot

**12 Months:**
- 500+ GitHub stars
- 50+ production deployments
- 20+ community contributors
- 5+ enterprise customers

### **Business Metrics**

**Proof of Market:**
- 10+ requests for commercial support
- 3+ companies willing to sponsor development
- 1+ partnership with Linux vendor (Canonical, Red Hat)
- 1+ cloud provider integration (marketplace listing)

---

## FUTURE ENHANCEMENTS

### **Phase 6: Advanced Features (Months 10-18)**

**Direct Login System:**
- RDP acts as display manager
- Pre-authentication RDP server
- Session transfer after login
- No local login needed

**Hardware Acceleration:**
- VAAPI video encoding (H.264/H.265)
- Zero-copy DMA-BUF path
- GPU rendering support
- 70% CPU reduction

**Load Balancing:**
- Multiple server instances
- Client routing
- Session migration
- High availability

### **Phase 7: Enterprise Integration (Months 19-24)**

**Directory Services:**
- LDAP/Active Directory integration
- Kerberos authentication
- Group policy support
- SSO integration

**Compliance:**
- Audit logging (all sessions)
- RBAC (role-based access)
- Encryption at rest
- FIPS compliance

**Management:**
- Web-based admin console
- REST API for automation
- Monitoring dashboards
- Alerting integration

---

## DEPLOYMENT SCENARIOS

### **Scenario 1: Small Office (5-10 Users)**

**Hardware:**
- Single server: 4 vCPU, 8GB RAM
- Software rendering (llvmpipe)
- Local network (Gigabit)

**Configuration:**
```toml
[headless]
compositor = "smithay"
renderer = "pixman"
max_sessions = 10

[headless.resources]
memory_per_session = "768MB"
cpu_shares = 1024
```

**Cost:** $0/month (self-hosted) + hardware ($500 one-time)

---

### **Scenario 2: Cloud Workstations (20-50 Users)**

**Infrastructure:**
- 3x VPS instances: 8 vCPU, 16GB RAM each
- Load balancer
- Managed PipeWire
- Cloud storage for user data

**Configuration:**
```toml
[headless]
compositor = "smithay"
renderer = "llvmpipe"
max_sessions = 20

[headless.resources]
memory_per_session = "512MB"
cpu_shares = 512

[loadbalancing]
mode = "round-robin"
health_check_interval = 30
```

**Cost:** $240-360/month (3x $80-120 VPS) for 50-60 users
**Savings vs AWS WorkSpaces:** $2,100-4,500/month (85% reduction!)

---

### **Scenario 3: Enterprise VDI (500+ Users)**

**Infrastructure:**
- Kubernetes cluster (10+ nodes)
- Hardware GPU nodes for power users
- Software rendering for standard users
- Persistent storage (NFS/Ceph)
- Load balancer with session affinity
- Monitoring stack (Prometheus, Grafana)

**Deployment:**
```yaml
# Standard user pool
apiVersion: apps/v1
kind: Deployment
metadata:
  name: wrd-server-standard
spec:
  replicas: 20
  template:
    spec:
      containers:
      - name: wrd-server
        resources:
          requests:
            memory: "4Gi"
            cpu: "2"
          limits:
            memory: "8Gi"
            cpu: "4"

# Power user pool (GPU)
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: wrd-server-gpu
spec:
  replicas: 5
  template:
    spec:
      containers:
      - name: wrd-server
        resources:
          requests:
            nvidia.com/gpu: 1
```

**Cost:** ~$2,000-3,000/month (cloud) or ~$50,000 hardware + datacenter
**Savings vs Commercial VDI:** $100,000-200,000/year

---

## CONCLUSION

### **Strategic Recommendation**

**Headless deployment is THE critical feature for enterprise adoption.** Without it, WRD-Server is limited to desktop screen sharing. With it, it becomes a **cloud-native VDI platform** that can compete with multi-billion dollar enterprise solutions.

**Phased Approach:**
1. **Prove concept** with Weston (4 weeks)
2. **Build foundation** with Smithay (10 weeks)
3. **Integrate components** (embedded Portal, PipeWire) (10 weeks)
4. **Production harden** (multi-user, systemd) (12 weeks)

**Total Timeline:** 36 weeks (9 months) to production-ready

**Technology Choice:** **Smithay** is the right long-term choice:
- Pure Rust (aligns with project)
- Full control and integration
- Single binary deployment
- Optimal for RDP use case
- Strong ecosystem support

**Market Opportunity:** Addressing a $10B+ underserved market (Linux VDI) with 70-85% cost savings vs commercial solutions.

**Risk Level:** Medium - significant engineering effort but clear path forward, proven technologies, phased approach reduces risk.

**Return on Investment:** Extremely high - enables enterprise market, cloud deployments, scaling to thousands of users, potential for commercial support business.

### **Next Steps**

1. **Validate with PoC:** Build Weston-based prototype (Month 1)
2. **Community feedback:** Share roadmap, gather input
3. **Resource planning:** Allocate development time
4. **Begin Phase 1:** Start Smithay integration (Month 2)
5. **Continuous delivery:** Release incremental improvements

---

**Document Status:** APPROVED FOR IMPLEMENTATION
**Primary Owner:** WRD-Server Core Team
**Review Cycle:** Monthly during implementation
**Success Definition:** Production-ready headless deployment supporting 10+ concurrent users with <256MB RAM per session

---

## APPENDIX A: CODE STRUCTURE

### **Proposed Directory Layout**

```
src/
├── headless/
│   ├── mod.rs                      - Headless mode orchestration
│   ├── compositor/
│   │   ├── mod.rs                  - Smithay compositor main
│   │   ├── backend.rs              - Headless backend
│   │   ├── renderer.rs             - Pixman/GLES renderer
│   │   ├── protocols/
│   │   │   ├── compositor.rs       - wl_compositor protocol
│   │   │   ├── xdg_shell.rs        - xdg_shell protocol
│   │   │   ├── shm.rs              - wl_shm protocol
│   │   │   └── data_device.rs      - wl_data_device protocol
│   │   ├── window_manager.rs       - Window management
│   │   ├── input.rs                - Virtual seat handling
│   │   └── output.rs               - Virtual output
│   ├── portal/
│   │   ├── mod.rs                  - Portal service main
│   │   ├── screencast.rs           - ScreenCast D-Bus interface
│   │   ├── remote_desktop.rs       - RemoteDesktop D-Bus interface
│   │   ├── session.rs              - Session management
│   │   └── auto_grant.rs           - Auto-grant logic
│   ├── pipewire/
│   │   ├── producer.rs             - PipeWire stream producer
│   │   ├── stream_factory.rs       - Stream creation
│   │   └── format_negotiation.rs   - Format params
│   ├── session/
│   │   ├── manager.rs              - Session lifecycle
│   │   ├── auth.rs                 - PAM authentication
│   │   ├── isolation.rs            - Resource isolation
│   │   ├── systemd_logind.rs       - systemd integration
│   │   └── user_session.rs         - Per-user state
│   ├── apps/
│   │   ├── launcher.rs             - Application spawning
│   │   ├── autostart.rs            - Auto-start config
│   │   ├── lifecycle.rs            - Process monitoring
│   │   └── environment.rs          - Environment setup
│   └── config.rs                   - Headless configuration
├── server/                         - Existing RDP server
├── portal/                         - Existing Portal client
├── pipewire/                       - Existing PipeWire consumer
├── clipboard/                      - Existing clipboard
└── ... (other existing modules)
```

### **Integration Points**

**Main Entry Point:**
```rust
// src/main.rs
#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load("config.toml")?;

    if config.headless.enabled {
        // Headless mode
        headless::run(config).await?;
    } else {
        // Desktop mode (existing)
        server::run(config).await?;
    }

    Ok(())
}
```

**Headless Orchestration:**
```rust
// src/headless/mod.rs
pub async fn run(config: Config) -> Result<()> {
    // 1. Initialize compositor
    let compositor = compositor::WrdCompositor::new(
        config.headless.width,
        config.headless.height,
        config.headless.renderer,
    )?;

    // 2. Start Portal service
    let portal_handle = portal::start_service(
        Arc::clone(&compositor),
    ).await?;

    // 3. Start PipeWire producer
    let pipewire_producer = pipewire::Producer::new(
        Arc::clone(&compositor),
    )?;

    // 4. Launch applications
    let app_launcher = apps::Launcher::new(config.headless.apps);
    app_launcher.launch_all().await?;

    // 5. Start RDP server
    let rdp_server = server::WrdServer::new(config.server)?;
    rdp_server.run().await?;

    Ok(())
}
```

---

## APPENDIX B: TESTING STRATEGY

### **Unit Tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compositor_initialization() {
        let compositor = WrdCompositor::new(1920, 1080, RendererType::Pixman);
        assert!(compositor.is_ok());
    }

    #[test]
    fn test_portal_session_creation() {
        let portal = ScreenCastPortal::new();
        let session = portal.create_session(HashMap::new());
        assert!(session.is_ok());
    }

    #[tokio::test]
    async fn test_pipewire_stream_creation() {
        let producer = PipeWireProducer::new(1920, 1080).await;
        assert!(producer.is_ok());
    }
}
```

### **Integration Tests**

```rust
// tests/headless_integration.rs
#[tokio::test]
async fn test_full_headless_stack() {
    // Start compositor
    let compositor = WrdCompositor::new(1920, 1080, RendererType::Pixman)?;

    // Start Portal
    let portal = start_portal_service(Arc::clone(&compositor)).await?;

    // Create PipeWire stream
    let producer = PipeWireProducer::new(Arc::clone(&compositor))?;

    // Render frame
    producer.push_frame()?;

    // Verify frame received by consumer
    // ...
}
```

### **System Tests**

```bash
#!/bin/bash
# tests/system/headless_rdp_test.sh

# Start wrd-server in headless mode
./target/release/wrd-server -c tests/headless.toml &
WRD_PID=$!

# Wait for server ready
sleep 5

# Connect with RDP client
xfreerdp /v:localhost:3389 /u:test /p:test /cert:ignore &
RDP_PID=$!

# Wait for connection
sleep 10

# Take screenshot
import -window root screenshot.png

# Verify screenshot shows desktop
# ...

# Cleanup
kill $RDP_PID $WRD_PID
```

### **Performance Tests**

```rust
// benches/headless_performance.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_frame_render(c: &mut Criterion) {
    let mut compositor = WrdCompositor::new(1920, 1080, RendererType::Pixman).unwrap();

    c.bench_function("render_frame", |b| {
        b.iter(|| {
            compositor.render_frame()
        })
    });
}

criterion_group!(benches, benchmark_frame_render);
criterion_main!(benches);
```

---

**END OF DOCUMENT**
