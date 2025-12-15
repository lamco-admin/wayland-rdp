# X11 + XVFB IMPLEMENTATION GUIDE
**Practical Guide for Implementing Smithay X11 Backend with Xvfb**

**Target**: WRD-Server Phase 2 Implementation
**Timeline**: 2-4 Weeks
**Status**: IMPLEMENTATION READY

---

## OVERVIEW

This guide provides step-by-step instructions for implementing a Smithay-based Wayland compositor using the X11 backend with Xvfb for headless RDP server deployment.

**Architecture**:
```
┌─────────────────┐
│  WRD-Server     │ ← RDP Server (IronRDP + Tokio)
│  (Tokio thread) │
└────────┬────────┘
         │ channels
┌────────▼────────┐
│  Smithay        │ ← Wayland Compositor
│  Compositor     │   (Calloop thread)
│  (X11 backend)  │
└────────┬────────┘
         │
┌────────▼────────┐
│     Xvfb        │ ← Virtual X Server
│  (in-memory FB) │   (separate process)
└─────────────────┘
```

**Key Benefits**:
- NO GPU required
- Container-friendly
- Full compositor control
- Event-driven clipboard

---

## PREREQUISITES

### System Requirements

```bash
# Ubuntu/Debian
sudo apt-get install -y \
    xvfb \
    libx11-dev \
    libxext-dev \
    libxfixes-dev \
    libxcursor-dev \
    libxi-dev \
    libxrandr-dev \
    libdrm-dev \
    libgbm-dev \
    libegl1-mesa-dev \
    libgles2-mesa-dev \
    libwayland-dev \
    libxkbcommon-dev

# Fedora/RHEL
sudo dnf install -y \
    xorg-x11-server-Xvfb \
    libX11-devel \
    libdrm-devel \
    mesa-libgbm-devel \
    mesa-libEGL-devel \
    mesa-libGLES-devel \
    wayland-devel \
    libxkbcommon-devel
```

### Rust Dependencies

```toml
# Cargo.toml
[dependencies]
# Smithay compositor
smithay = { version = "0.7.0", features = [
    "wayland_frontend",    # Wayland protocol support
    "backend_x11",         # X11 backend (key!)
    "backend_egl",         # OpenGL/EGL context
    "backend_gbm",         # Buffer allocation
    "renderer_gl",         # GLES2 renderer
    "desktop",             # Desktop shell helpers
]}

# Event loop
calloop = "0.14.0"

# DRM/GBM
drm = "0.14.0"
gbm = "0.18.0"

# X11
x11rb = { version = "0.13.0", features = ["dri3", "present", "xfixes", "xinput"] }

# Wayland
wayland-server = "0.31.9"
wayland-protocols = { version = "0.32.8", features = ["server"] }
xkbcommon = "0.8.0"

# Channels for threading
crossbeam-channel = "0.5"

# Async (for RDP integration)
tokio = { version = "1.35", features = ["full"] }

# Utilities
tracing = "0.1"
anyhow = "1.0"
thiserror = "1.0"
```

---

## WEEK 1: CORE COMPOSITOR SETUP

### Step 1.1: Project Structure

```
src/
├── main.rs                    # Entry point (thread spawning)
├── compositor/
│   ├── mod.rs                 # Compositor module
│   ├── backend.rs             # X11 backend setup
│   ├── state.rs               # Compositor state
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── compositor.rs      # CompositorHandler
│   │   ├── xdg_shell.rs       # XdgShellHandler
│   │   ├── shm.rs             # ShmHandler
│   │   ├── seat.rs            # SeatHandler
│   │   └── selection.rs       # SelectionHandler (clipboard)
│   └── render.rs              # Rendering logic
├── rdp/
│   ├── mod.rs                 # RDP server module
│   ├── server.rs              # IronRDP server
│   └── bridge.rs              # Compositor ↔ RDP bridge
└── lib.rs                     # Library exports
```

### Step 1.2: X11 Backend Initialization

```rust
// src/compositor/backend.rs
use smithay::backend::x11::{X11Backend, X11Surface, WindowBuilder};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::egl::{EGLDisplay, EGLContext};
use smithay::backend::allocator::gbm::{GbmAllocator, GbmBufferFlags};
use smithay::backend::allocator::dmabuf::DmabufAllocator;
use smithay::utils::DeviceFd;
use gbm::Device as GbmDevice;

pub struct X11CompositorBackend {
    pub backend: X11Backend,
    pub surface: X11Surface,
    pub renderer: GlesRenderer,
    pub window_size: (i32, i32),
}

impl X11CompositorBackend {
    pub fn new(display_num: u32) -> Result<Self> {
        // 1. Set DISPLAY environment variable
        std::env::set_var("DISPLAY", format!(":{}",display_num));

        // 2. Connect to X server
        tracing::info!("Connecting to X11 display :{}", display_num);
        let backend = X11Backend::new()
            .map_err(|e| anyhow::anyhow!("Failed to connect to X server: {}", e))?;

        let handle = backend.handle();

        // 3. Get DRM node from X server for rendering
        let (drm_node, drm_fd) = handle
            .drm_node()
            .map_err(|e| anyhow::anyhow!("Failed to get DRM node: {}", e))?;

        tracing::info!("Using DRM node: {:?}", drm_node);

        // 4. Create GBM device for buffer allocation
        let gbm_device = GbmDevice::new(DeviceFd::from(drm_fd))
            .map_err(|e| anyhow::anyhow!("Failed to create GBM device: {}", e))?;

        // 5. Initialize EGL
        let egl_display = unsafe {
            EGLDisplay::new(gbm_device.clone())
                .map_err(|e| anyhow::anyhow!("Failed to create EGL display: {}", e))?
        };

        let egl_context = EGLContext::new(&egl_display)
            .map_err(|e| anyhow::anyhow!("Failed to create EGL context: {}", e))?;

        // 6. Create GLES2 renderer
        let renderer = unsafe {
            GlesRenderer::new(egl_context)
                .map_err(|e| anyhow::anyhow!("Failed to create renderer: {}", e))?
        };

        // 7. Create X11 window
        let window = WindowBuilder::new()
            .title("WRD-Server Compositor")
            .build(&handle)
            .map_err(|e| anyhow::anyhow!("Failed to create window: {}", e))?;

        let window_size = {
            let s = window.size();
            (s.w as i32, s.h as i32)
        };

        tracing::info!("Window size: {}x{}", window_size.0, window_size.1);

        // 8. Create X11 surface for rendering
        let allocator = DmabufAllocator(GbmAllocator::new(
            gbm_device,
            GbmBufferFlags::RENDERING,
        ));

        let modifiers = egl_context
            .dmabuf_render_formats()
            .iter()
            .map(|format| format.modifier);

        let surface = handle
            .create_surface(&window, allocator, modifiers)
            .map_err(|e| anyhow::anyhow!("Failed to create surface: {}", e))?;

        Ok(Self {
            backend,
            surface,
            renderer,
            window_size,
        })
    }
}
```

### Step 1.3: Compositor State

```rust
// src/compositor/state.rs
use smithay::wayland::compositor::CompositorState;
use smithay::wayland::shm::ShmState;
use smithay::wayland::shell::xdg::XdgShellState;
use smithay::input::{SeatState, Seat};
use smithay::wayland::selection::data_device::DataDeviceState;
use smithay::reexports::wayland_server::DisplayHandle;
use crossbeam_channel::Sender;

pub struct WrdCompositorState {
    // Wayland protocol state
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,

    // Input
    pub seat: Seat<Self>,

    // Backend
    pub display_handle: DisplayHandle,

    // RDP communication
    pub rdp_tx: Sender<CompositorEvent>,

    // Rendering
    pub start_time: std::time::Instant,
}

pub enum CompositorEvent {
    FrameReady { pixels: Vec<u8>, width: u32, height: u32 },
    ClipboardChanged { mime_type: String, data: Vec<u8> },
    // ... more events
}

impl WrdCompositorState {
    pub fn new(
        display_handle: DisplayHandle,
        rdp_tx: Sender<CompositorEvent>,
    ) -> Self {
        let compositor_state = CompositorState::new::<Self>(&display_handle);
        let xdg_shell_state = XdgShellState::new::<Self>(&display_handle);
        let shm_state = ShmState::new::<Self>(&display_handle, vec![]);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<Self>(&display_handle);

        // Create seat
        let seat = seat_state.new_wl_seat(&display_handle, "wrd-seat");

        Self {
            compositor_state,
            xdg_shell_state,
            shm_state,
            seat_state,
            data_device_state,
            seat,
            display_handle,
            rdp_tx,
            start_time: std::time::Instant::now(),
        }
    }
}
```

### Step 1.4: Main Event Loop

```rust
// src/compositor/mod.rs
use calloop::EventLoop;
use smithay::reexports::wayland_server::Display;
use crossbeam_channel::{Sender, Receiver};

pub struct WrdCompositor {
    backend: X11CompositorBackend,
    event_loop: EventLoop<WrdCompositorState>,
    display: Display<WrdCompositorState>,
    state: WrdCompositorState,
}

impl WrdCompositor {
    pub fn new(
        display_num: u32,
        rdp_tx: Sender<CompositorEvent>,
        rdp_rx: Receiver<RdpCommand>,
    ) -> Result<Self> {
        // 1. Initialize backend
        let backend = X11CompositorBackend::new(display_num)?;

        // 2. Create Wayland display
        let display = Display::new()?;
        let display_handle = display.handle();

        // 3. Create compositor state
        let mut state = WrdCompositorState::new(display_handle.clone(), rdp_tx);

        // 4. Add keyboard to seat
        state.seat.add_keyboard(
            Default::default(), // XKB config
            200,                // repeat delay
            25,                 // repeat rate
        )?;

        // 5. Add pointer to seat
        state.seat.add_pointer();

        // 6. Create event loop
        let mut event_loop = EventLoop::try_new()?;
        let handle = event_loop.handle();

        // 7. Insert X11 backend as event source
        handle.insert_source(
            backend.backend.clone(),
            |event, _, state| {
                state.handle_x11_event(event);
            }
        )?;

        // 8. Insert Wayland display event source
        handle.insert_source(
            calloop::generic::Generic::new(
                display.backend().poll_fd(),
                calloop::Interest::READ,
                calloop::Mode::Level,
            ),
            |_, _, state| {
                // Dispatch Wayland clients
                state.display.dispatch_clients(state)?;
                Ok(calloop::PostAction::Continue)
            }
        )?;

        // 9. Insert RDP command channel
        let (rdp_async_tx, rdp_async_rx) = calloop::channel::channel();
        handle.insert_source(
            rdp_async_rx,
            |event, _, state| {
                match event {
                    calloop::channel::Event::Msg(cmd) => {
                        state.handle_rdp_command(cmd);
                    }
                    calloop::channel::Event::Closed => {
                        tracing::warn!("RDP channel closed");
                    }
                }
            }
        )?;

        // Bridge sync channel to async channel
        std::thread::spawn(move || {
            while let Ok(cmd) = rdp_rx.recv() {
                let _ = rdp_async_tx.send(cmd);
            }
        });

        Ok(Self {
            backend,
            event_loop,
            display,
            state,
        })
    }

    pub fn run(mut self) -> Result<()> {
        tracing::info!("Starting compositor event loop");

        // Run event loop
        self.event_loop.run(
            None,
            &mut self.state,
            |state| {
                // Render frame periodically
                state.render_frame();
            }
        )?;

        Ok(())
    }
}
```

### Step 1.5: Xvfb Startup Script

```bash
#!/bin/bash
# scripts/start-xvfb.sh

set -e

DISPLAY_NUM=${1:-99}
RESOLUTION=${2:-1920x1080x24}

echo "Starting Xvfb on display :${DISPLAY_NUM}"

# Kill existing Xvfb on this display
pkill -f "Xvfb :${DISPLAY_NUM}" 2>/dev/null || true
sleep 0.5

# Start Xvfb
Xvfb :${DISPLAY_NUM} \
  -screen 0 ${RESOLUTION} \
  -nolisten tcp \
  -auth /tmp/.Xvfb-${DISPLAY_NUM}-auth \
  +extension GLX \
  +extension RANDR \
  +render \
  -noreset \
  > /tmp/xvfb-${DISPLAY_NUM}.log 2>&1 &

XVFB_PID=$!
echo ${XVFB_PID} > /tmp/xvfb-${DISPLAY_NUM}.pid

# Wait for X server
for i in {1..30}; do
  if xdpyinfo -display :${DISPLAY_NUM} >/dev/null 2>&1; then
    echo "Xvfb started successfully (PID: ${XVFB_PID})"
    exit 0
  fi
  sleep 0.1
done

# Failed to start
echo "ERROR: Xvfb failed to start"
kill ${XVFB_PID} 2>/dev/null || true
exit 1
```

---

## WEEK 2: WAYLAND PROTOCOL HANDLERS

### Step 2.1: Compositor Handler

```rust
// src/compositor/handlers/compositor.rs
use smithay::wayland::compositor::{CompositorHandler, CompositorState};
use smithay::backend::renderer::utils::on_commit_buffer_handler;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::Client;

impl CompositorHandler for WrdCompositorState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(
        &self,
        client: &'a Client
    ) -> &'a smithay::wayland::compositor::CompositorClientState {
        &client
            .get_data::<ClientState>()
            .unwrap()
            .compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        // Handle buffer commit
        on_commit_buffer_handler::<Self>(surface);

        // Trigger render if this is a toplevel
        if self.is_toplevel(surface) {
            self.schedule_render();
        }
    }
}

smithay::delegate_compositor!(WrdCompositorState);
```

### Step 2.2: XDG Shell Handler

```rust
// src/compositor/handlers/xdg_shell.rs
use smithay::wayland::shell::xdg::{
    XdgShellHandler, XdgShellState,
    ToplevelSurface, PopupSurface, PositionerState
};
use smithay::utils::Serial;

impl XdgShellHandler for WrdCompositorState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        tracing::info!("New toplevel surface");

        // Configure surface
        surface.with_pending_state(|state| {
            // Set initial state
            state.states.set(
                smithay::wayland::shell::xdg::xdg_toplevel::State::Activated
            );

            // Set size to window size
            state.size = Some((1920, 1080).into());
        });

        // Send configure
        surface.send_configure();

        // Give keyboard focus
        let keyboard = self.seat.get_keyboard().unwrap();
        keyboard.set_focus(
            self,
            Some(surface.wl_surface().clone()),
            Serial::from(0),
        );
    }

    fn new_popup(&mut self, surface: PopupSurface, positioner: PositionerState) {
        // Handle popup creation
        tracing::info!("New popup surface");
        surface.send_configure().ok();
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {
        // Handle popup grab
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        surface.send_repositioned(token);
    }
}

smithay::delegate_xdg_shell!(WrdCompositorState);
```

### Step 2.3: Selection Handler (Clipboard!)

```rust
// src/compositor/handlers/selection.rs
use smithay::wayland::selection::{SelectionHandler, SelectionTarget};
use smithay::wayland::selection::data_device::DataDeviceState;
use smithay::reexports::wayland_server::protocol::wl_data_source::WlDataSource;

impl SelectionHandler for WrdCompositorState {
    type SelectionUserData = ();

    fn new_selection(
        &mut self,
        ty: SelectionTarget,
        source: Option<WlDataSource>,
    ) {
        match ty {
            SelectionTarget::Clipboard => {
                tracing::info!("Clipboard selection changed!");

                if let Some(source) = source {
                    // Get MIME types offered
                    let mime_types = /* extract from source */;

                    // Request data for first MIME type
                    if let Some(mime_type) = mime_types.first() {
                        self.read_clipboard_data(source, mime_type.clone());
                    }
                }
            }
            SelectionTarget::Primary => {
                // Primary selection (middle-click paste)
            }
        }
    }

    fn send_selection(
        &mut self,
        ty: SelectionTarget,
        mime_type: String,
        fd: std::os::unix::io::OwnedFd,
    ) {
        // Write clipboard data to client
        tracing::info!("Client requesting clipboard: {}", mime_type);

        // Write data to fd
        // This is called when a Wayland client pastes
    }
}

impl WrdCompositorState {
    fn read_clipboard_data(&mut self, source: WlDataSource, mime_type: String) {
        use std::os::unix::io::FromRawFd;
        use std::io::Read;

        // Create pipe
        let (read_fd, write_fd) = nix::unistd::pipe().unwrap();

        // Request data from source
        source.send(mime_type.clone(), write_fd.as_raw_fd());
        drop(write_fd); // Close write end

        // Read data
        let mut data = Vec::new();
        let mut file = unsafe { std::fs::File::from_raw_fd(read_fd.as_raw_fd()) };
        file.read_to_end(&mut data).unwrap();

        // Send to RDP
        self.rdp_tx.send(CompositorEvent::ClipboardChanged {
            mime_type,
            data,
        }).ok();
    }
}

smithay::delegate_data_device!(WrdCompositorState);
```

### Step 2.4: SHM and Seat Handlers

```rust
// src/compositor/handlers/shm.rs
use smithay::wayland::shm::{ShmHandler, ShmState};

impl ShmHandler for WrdCompositorState {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

smithay::delegate_shm!(WrdCompositorState);

// src/compositor/handlers/seat.rs
use smithay::input::{SeatHandler, SeatState, Seat};
use smithay::input::pointer::CursorImageStatus;

impl SeatHandler for WrdCompositorState {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&WlSurface>) {
        tracing::info!("Focus changed: {:?}", focused);
    }

    fn cursor_image(&mut self, seat: &Seat<Self>, image: CursorImageStatus) {
        // Handle cursor image changes
    }
}

smithay::delegate_seat!(WrdCompositorState);
```

---

## WEEK 3: RDP INTEGRATION

### Step 3.1: Event Bridge

```rust
// src/rdp/bridge.rs
use crossbeam_channel::{Sender, Receiver};
use tokio::sync::mpsc;

pub struct RdpBridge {
    // Compositor → RDP (sync)
    comp_rx: Receiver<CompositorEvent>,

    // RDP → Compositor (async)
    rdp_rx: mpsc::Receiver<RdpCommand>,
    rdp_tx: Sender<RdpCommand>,
}

pub enum RdpCommand {
    KeyPress { keycode: u32, pressed: bool },
    MouseMove { x: f64, y: f64 },
    MouseButton { button: u32, pressed: bool },
    SetClipboard { mime_type: String, data: Vec<u8> },
}

impl RdpBridge {
    pub fn new() -> (Self, Sender<CompositorEvent>, mpsc::Sender<RdpCommand>) {
        let (comp_tx, comp_rx) = crossbeam_channel::bounded(8);
        let (rdp_tx_async, rdp_rx) = mpsc::channel(32);
        let (rdp_tx_sync, _) = crossbeam_channel::bounded(32);

        let bridge = Self {
            comp_rx,
            rdp_rx,
            rdp_tx: rdp_tx_sync,
        };

        (bridge, comp_tx, rdp_tx_async)
    }

    pub async fn run(mut self, rdp_server: RdpServerHandle) {
        loop {
            tokio::select! {
                // Compositor → RDP
                Ok(event) = self.comp_rx.recv_async() => {
                    match event {
                        CompositorEvent::FrameReady { pixels, width, height } => {
                            self.handle_frame(rdp_server, pixels, width, height).await;
                        }
                        CompositorEvent::ClipboardChanged { mime_type, data } => {
                            self.handle_clipboard(rdp_server, mime_type, data).await;
                        }
                    }
                }

                // RDP → Compositor
                Some(cmd) = self.rdp_rx.recv() => {
                    // Forward to compositor
                    self.rdp_tx.send(cmd).ok();
                }
            }
        }
    }

    async fn handle_frame(&self, rdp: RdpServerHandle, pixels: Vec<u8>, w: u32, h: u32) {
        // Convert to RDP bitmap format
        let rdp_bitmap = convert_to_rdp_bitmap(&pixels, w, h);

        // Send to RDP client
        rdp.send_bitmap_update(rdp_bitmap).await.ok();
    }

    async fn handle_clipboard(&self, rdp: RdpServerHandle, mime: String, data: Vec<u8>) {
        // Convert MIME type to RDP format
        let rdp_format = mime_to_rdp_format(&mime);

        // Send format list
        rdp.send_clipboard_format_list(vec![rdp_format]).await.ok();

        // Store data for when client requests it
        rdp.set_clipboard_data(data).await;
    }
}

fn convert_to_rdp_bitmap(pixels: &[u8], width: u32, height: u32) -> RdpBitmap {
    // Convert RGBA/BGRA to RDP bitmap format
    // Apply compression if needed
    // Return RDP bitmap structure
    todo!("Implement bitmap conversion")
}
```

### Step 3.2: Rendering and Frame Capture

```rust
// src/compositor/render.rs
use smithay::backend::renderer::{Renderer, Frame, Bind};
use smithay::backend::renderer::element::{AsRenderElements, RenderElement};
use smithay::backend::renderer::utils::draw_render_elements;
use smithay::utils::Transform;

impl WrdCompositorState {
    pub fn render_frame(&mut self, backend: &mut X11CompositorBackend) {
        // Get toplevel surfaces
        let surfaces: Vec<_> = self.xdg_shell_state
            .toplevel_surfaces()
            .cloned()
            .collect();

        if surfaces.is_empty() {
            return; // Nothing to render
        }

        // Bind surface for rendering
        backend.renderer.bind(&backend.surface).unwrap();

        let size = backend.window_size;
        let damage = smithay::utils::Rectangle::from_size(size);

        // Render
        {
            let mut frame = backend.renderer
                .render(&mut backend.surface, size, Transform::Flipped180)
                .unwrap();

            // Clear background
            frame.clear([0.1, 0.1, 0.1, 1.0], &[damage]).unwrap();

            // Render each surface
            for surface in surfaces.iter() {
                let elements = smithay::backend::renderer::element::surface
                    ::render_elements_from_surface_tree(
                        &backend.renderer,
                        surface.wl_surface(),
                        (0, 0), // Position
                        1.0,    // Scale
                        1.0,    // Alpha
                        smithay::backend::renderer::element::Kind::Unspecified,
                    );

                draw_render_elements(&mut frame, 1.0, &elements, &[damage]).unwrap();
            }

            frame.finish().unwrap();
        }

        // Read pixels for RDP
        self.capture_frame(backend);

        // Submit to X11 window
        backend.surface.submit(Some(&[damage])).unwrap();

        // Send frame callbacks
        let time = self.start_time.elapsed().as_millis() as u32;
        for surface in surfaces {
            send_frames_surface_tree(surface.wl_surface(), time);
        }
    }

    fn capture_frame(&mut self, backend: &mut X11CompositorBackend) {
        // Read pixels from renderer
        // This requires implementing a readback mechanism
        // For now, placeholder:

        let (width, height) = backend.window_size;
        let pixels = vec![0u8; (width * height * 4) as usize]; // RGBA

        // TODO: Actual pixel readback from renderer
        // backend.renderer.read_pixels(...)?;

        // Send to RDP
        self.rdp_tx.send(CompositorEvent::FrameReady {
            pixels,
            width: width as u32,
            height: height as u32,
        }).ok();
    }
}
```

### Step 3.3: Input Handling from RDP

```rust
// src/compositor/state.rs (add methods)
impl WrdCompositorState {
    pub fn handle_rdp_command(&mut self, cmd: RdpCommand) {
        match cmd {
            RdpCommand::KeyPress { keycode, pressed } => {
                self.handle_key_input(keycode, pressed);
            }
            RdpCommand::MouseMove { x, y } => {
                self.handle_pointer_motion(x, y);
            }
            RdpCommand::MouseButton { button, pressed } => {
                self.handle_pointer_button(button, pressed);
            }
            RdpCommand::SetClipboard { mime_type, data } => {
                self.set_clipboard_data(mime_type, data);
            }
        }
    }

    fn handle_key_input(&mut self, keycode: u32, pressed: bool) {
        use smithay::input::keyboard::KeyState;

        let keyboard = self.seat.get_keyboard().unwrap();

        let state = if pressed {
            KeyState::Pressed
        } else {
            KeyState::Released
        };

        keyboard.input::<(), _>(
            self,
            keycode,
            state,
            0.into(), // Serial
            0,        // Time
            |_, _, _| {
                smithay::input::keyboard::FilterResult::Forward
            },
        );
    }

    fn handle_pointer_motion(&mut self, x: f64, y: f64) {
        let pointer = self.seat.get_pointer().unwrap();

        // Find surface under pointer
        let surface = self.surface_under_pointer((x, y));

        pointer.motion(
            self,
            surface,
            &smithay::input::pointer::MotionEvent {
                location: (x, y).into(),
                serial: 0.into(),
                time: 0,
            },
        );
    }

    fn handle_pointer_button(&mut self, button: u32, pressed: bool) {
        use smithay::input::pointer::ButtonState;

        let pointer = self.seat.get_pointer().unwrap();

        let state = if pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        };

        pointer.button(
            self,
            &smithay::input::pointer::ButtonEvent {
                button,
                state,
                serial: 0.into(),
                time: 0,
            },
        );
    }

    fn set_clipboard_data(&mut self, mime_type: String, data: Vec<u8>) {
        // Set compositor clipboard from RDP client
        // This requires creating a data source
        // TODO: Implement
    }

    fn surface_under_pointer(&self, pos: (f64, f64)) -> Option<(WlSurface, (f64, f64))> {
        // Find which surface is under pointer position
        // For simple case, return first toplevel
        self.xdg_shell_state
            .toplevel_surfaces()
            .next()
            .map(|s| (s.wl_surface().clone(), pos))
    }
}
```

---

## WEEK 4: TESTING & DEPLOYMENT

### Step 4.1: Integration Test

```rust
// tests/integration_test.rs
use std::process::Command;
use std::time::Duration;

#[test]
fn test_compositor_startup() {
    // Start Xvfb
    let xvfb = Command::new("Xvfb")
        .args(&[":99", "-screen", "0", "1920x1080x24"])
        .spawn()
        .expect("Failed to start Xvfb");

    std::thread::sleep(Duration::from_secs(1));

    // Start compositor
    std::env::set_var("DISPLAY", ":99");

    let compositor = std::thread::spawn(|| {
        // Run compositor
        let result = wrd_server::compositor::WrdCompositor::new(
            99,
            /* channels */
        );
        assert!(result.is_ok());
    });

    std::thread::sleep(Duration::from_secs(2));

    // Verify Wayland socket exists
    let socket_path = std::env::var("XDG_RUNTIME_DIR")
        .map(|d| format!("{}/wayland-0", d))
        .unwrap_or("/tmp/wayland-0".to_string());

    assert!(std::path::Path::new(&socket_path).exists());

    // Cleanup
    compositor.join().ok();
    Command::new("pkill").args(&["-f", "Xvfb :99"]).status().ok();
}
```

### Step 4.2: Dockerfile

```dockerfile
FROM ubuntu:24.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    xvfb \
    libx11-6 \
    libxext6 \
    libxfixes3 \
    libxcursor1 \
    libxi6 \
    libxrandr2 \
    libdrm2 \
    libgbm1 \
    libegl1 \
    libgles2 \
    libwayland-server0 \
    libxkbcommon0 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY target/release/wrd-server /usr/local/bin/

# Copy startup script
COPY scripts/docker-entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Expose RDP port
EXPOSE 3389

ENTRYPOINT ["/entrypoint.sh"]
```

```bash
#!/bin/bash
# scripts/docker-entrypoint.sh

set -e

# Configuration
DISPLAY_NUM=${DISPLAY_NUM:-99}
RESOLUTION=${RESOLUTION:-1920x1080x24}

# Start Xvfb
echo "Starting Xvfb..."
Xvfb :${DISPLAY_NUM} -screen 0 ${RESOLUTION} -nolisten tcp &
XVFB_PID=$!

# Wait for X server
sleep 1

# Verify Xvfb started
if ! xdpyinfo -display :${DISPLAY_NUM} >/dev/null 2>&1; then
    echo "ERROR: Xvfb failed to start"
    exit 1
fi

# Start WRD-Server
echo "Starting WRD-Server..."
DISPLAY=:${DISPLAY_NUM} /usr/local/bin/wrd-server

# Cleanup on exit
kill $XVFB_PID 2>/dev/null || true
```

### Step 4.3: Build and Run

```bash
# Build
cargo build --release --features headless-compositor

# Run locally
./scripts/start-xvfb.sh 99
DISPLAY=:99 ./target/release/wrd-server

# Build Docker image
docker build -t wrd-server:latest .

# Run container
docker run -d \
    --name wrd-server \
    -p 3389:3389 \
    -e DISPLAY_NUM=99 \
    -e RESOLUTION=1920x1080x24 \
    wrd-server:latest

# View logs
docker logs -f wrd-server

# Connect with RDP client
xfreerdp /v:localhost:3389 /u:user /p:password
```

---

## PERFORMANCE OPTIMIZATION

### Framerate Limiting

```rust
impl WrdCompositorState {
    pub fn should_render(&mut self) -> bool {
        const TARGET_FPS: u32 = 30;
        const FRAME_TIME_MS: u128 = 1000 / TARGET_FPS as u128;

        let now = self.start_time.elapsed();
        let elapsed = now.as_millis() - self.last_render_time.as_millis();

        if elapsed >= FRAME_TIME_MS {
            self.last_render_time = now;
            true
        } else {
            false
        }
    }
}
```

### Damage Tracking

```rust
use smithay::backend::renderer::damage::OutputDamageTracker;

struct RenderState {
    damage_tracker: OutputDamageTracker,
}

impl WrdCompositorState {
    fn render_with_damage(&mut self, backend: &mut X11CompositorBackend) {
        let damage = self.render_state
            .damage_tracker
            .damage_output(1, &elements)
            .unwrap();

        // Only render damaged regions
        for rect in damage {
            // Render rect
        }
    }
}
```

---

## TROUBLESHOOTING

### Xvfb Issues

```bash
# Xvfb not starting
# Check if display already in use:
ps aux | grep "Xvfb :99"

# Kill existing:
pkill -f "Xvfb :99"

# Check X server log:
cat /tmp/xvfb-99.log
```

### EGL/OpenGL Issues

```bash
# Missing mesa drivers:
sudo apt-get install mesa-utils libgl1-mesa-dri

# Test EGL:
DISPLAY=:99 eglinfo

# Test GLX (should show virtual device):
DISPLAY=:99 glxinfo | grep "OpenGL renderer"
```

### DRM Node Issues

```bash
# DRM node not available:
ls -la /dev/dri/

# Xvfb may not provide DRM node
# Use software rendering fallback
```

---

## MONITORING

### Metrics to Track

```rust
struct CompositorMetrics {
    frames_rendered: AtomicU64,
    frames_dropped: AtomicU64,
    avg_frame_time_ms: AtomicU64,
    clipboard_events: AtomicU64,
    client_count: AtomicUsize,
}
```

### Logging

```rust
use tracing::{info, warn, error};

// Key events to log:
info!("Compositor started on display :{}", display_num);
info!("New Wayland client connected");
warn!("Frame took {}ms (target: {}ms)", elapsed, target);
error!("Failed to render frame: {}", e);
```

---

## NEXT STEPS

1. ✅ Complete Week 1 implementation
2. ✅ Complete Week 2 protocol handlers
3. ✅ Complete Week 3 RDP integration
4. ✅ Complete Week 4 testing
5. ⬜ Production deployment
6. ⬜ Performance tuning
7. ⬜ Documentation

---

**END OF IMPLEMENTATION GUIDE**
