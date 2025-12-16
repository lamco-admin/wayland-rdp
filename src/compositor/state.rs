//! Compositor state management
//!
//! This module implements the core compositor state and main event loop.

use super::types::*;
use super::input::{KeyboardEvent, PointerEvent};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;
use tracing::{debug, error, info, trace};

#[cfg(feature = "headless-compositor")]
use calloop::{EventLoop, LoopHandle};

#[cfg(feature = "headless-compositor")]
use smithay::{
    desktop::Space,
    input::Seat,
    output::Output,
    utils::Serial,
    wayland::compositor::CompositorState as SmithayCompositorState,
    wayland::shm::ShmState,
    wayland::shell::xdg::XdgShellState,
    wayland::seat::SeatState,
    wayland::selection::data_device::DataDeviceState,
};

/// Main compositor state
///
/// Holds all compositor state including surfaces, windows, and input state.
pub struct CompositorState {
    /// Configuration
    pub config: CompositorConfig,

    /// Windows indexed by ID
    pub windows: HashMap<WindowId, Window>,

    /// Surfaces indexed by ID
    pub surfaces: HashMap<SurfaceId, Surface>,

    /// Current framebuffer
    pub framebuffer: FrameBuffer,

    /// Damage tracker
    pub damage: DamageTracker,

    /// Keyboard state
    pub keyboard: KeyboardState,

    /// Pointer state
    pub pointer: PointerState,

    /// Clipboard state
    pub clipboard: ClipboardState,

    /// Focused window
    pub focused_window: Option<WindowId>,

    /// Z-order (bottom to top)
    pub z_order: Vec<WindowId>,

    /// Frame sequence number
    pub frame_sequence: u64,

    /// Event listeners
    pub event_listeners: Vec<Box<dyn Fn(CompositorEvent) + Send + Sync>>,

    // Smithay protocol states (initialized after Display creation)
    #[cfg(feature = "headless-compositor")]
    pub smithay_compositor_state: Option<SmithayCompositorState>,

    #[cfg(feature = "headless-compositor")]
    pub shm_state: Option<ShmState>,

    #[cfg(feature = "headless-compositor")]
    pub xdg_shell_state: Option<XdgShellState>,

    #[cfg(feature = "headless-compositor")]
    pub seat_state: Option<SeatState<Self>>,

    #[cfg(feature = "headless-compositor")]
    pub seat: Option<Seat<Self>>,

    #[cfg(feature = "headless-compositor")]
    pub output: Option<Output>,

    #[cfg(feature = "headless-compositor")]
    pub data_device_state: Option<DataDeviceState>,

    #[cfg(feature = "headless-compositor")]
    pub space: Option<Space<Window>>,

    /// Serial counter for Wayland protocol
    #[cfg(feature = "headless-compositor")]
    serial_counter: u32,
}

impl CompositorState {
    /// Create new compositor state
    pub fn new(config: CompositorConfig) -> Result<Self> {
        info!("Initializing compositor state: {}x{}", config.width, config.height);

        let framebuffer = FrameBuffer::new(config.width, config.height, config.pixel_format);

        Ok(Self {
            config: config.clone(),
            windows: HashMap::new(),
            surfaces: HashMap::new(),
            framebuffer,
            damage: DamageTracker::new(config.width, config.height),
            keyboard: KeyboardState::new()?,
            pointer: PointerState::new(),
            clipboard: ClipboardState::new(),
            focused_window: None,
            z_order: Vec::new(),
            frame_sequence: 0,
            event_listeners: Vec::new(),
            #[cfg(feature = "headless-compositor")]
            smithay_compositor_state: None,
            #[cfg(feature = "headless-compositor")]
            shm_state: None,
            #[cfg(feature = "headless-compositor")]
            xdg_shell_state: None,
            #[cfg(feature = "headless-compositor")]
            seat_state: None,
            #[cfg(feature = "headless-compositor")]
            seat: None,
            #[cfg(feature = "headless-compositor")]
            output: None,
            #[cfg(feature = "headless-compositor")]
            data_device_state: None,
            #[cfg(feature = "headless-compositor")]
            space: None,
            #[cfg(feature = "headless-compositor")]
            serial_counter: 0,
        })
    }

    /// Get current framebuffer
    pub fn get_framebuffer(&self) -> Vec<u8> {
        self.framebuffer.data.clone()
    }

    /// Get damaged regions
    pub fn get_damage(&self) -> Vec<Rectangle> {
        self.damage.get_regions()
    }

    /// Inject keyboard event
    pub fn inject_keyboard(&mut self, event: KeyboardEvent) -> Result<()> {
        trace!("Injecting keyboard event: {:?}", event);

        // Update keyboard state
        self.keyboard.handle_event(&event)?;

        // Send to focused window if any
        if let Some(window_id) = self.focused_window {
            if let Some(window) = self.windows.get_mut(&window_id) {
                window.handle_keyboard(&event)?;
            }
        }

        Ok(())
    }

    /// Inject pointer event
    pub fn inject_pointer(&mut self, event: PointerEvent) -> Result<()> {
        trace!("Injecting pointer event: {:?}", event);

        // Update pointer state
        self.pointer.handle_event(&event)?;

        // Determine window under pointer
        let window_id = self.window_at(event.x as i32, event.y as i32);

        // Send to window
        if let Some(window_id) = window_id {
            if let Some(window) = self.windows.get_mut(&window_id) {
                window.handle_pointer(&event)?;
            }

            // Update focus if clicked
            if let Some(_) = event.button {
                self.set_focus(Some(window_id));
            }
        }

        Ok(())
    }

    /// Get clipboard data
    pub fn get_clipboard(&self) -> Result<Vec<u8>> {
        Ok(self.clipboard.data.clone())
    }

    /// Set clipboard data
    pub fn set_clipboard(&mut self, data: Vec<u8>) -> Result<()> {
        self.clipboard.data = data;
        self.emit_event(CompositorEvent::ClipboardChanged);
        Ok(())
    }

    /// Find window at given coordinates
    pub fn window_at(&self, x: i32, y: i32) -> Option<WindowId> {
        // Iterate from top to bottom (reverse Z-order)
        for window_id in self.z_order.iter().rev() {
            if let Some(window) = self.windows.get(window_id) {
                if window.geometry.contains(x, y) && window.state != WindowState::Minimized {
                    return Some(*window_id);
                }
            }
        }
        None
    }

    /// Set focused window
    pub fn set_focus(&mut self, window_id: Option<WindowId>) {
        if self.focused_window != window_id {
            debug!("Focus changed: {:?} -> {:?}", self.focused_window, window_id);
            self.focused_window = window_id;
            self.emit_event(CompositorEvent::FocusChanged(window_id));
        }
    }

    /// Add window
    pub fn add_window(&mut self, window: Window) -> WindowId {
        let window_id = window.id;
        info!("Adding window: {} ({}x{})", window_id, window.geometry.width, window.geometry.height);

        self.windows.insert(window_id, window);
        self.z_order.push(window_id);

        self.damage.damage_all();
        self.emit_event(CompositorEvent::WindowCreated(window_id));

        window_id
    }

    /// Remove window
    pub fn remove_window(&mut self, window_id: WindowId) -> Option<Window> {
        info!("Removing window: {}", window_id);

        self.z_order.retain(|id| *id != window_id);

        if self.focused_window == Some(window_id) {
            self.focused_window = None;
        }

        self.damage.damage_all();
        self.emit_event(CompositorEvent::WindowDestroyed(window_id));

        self.windows.remove(&window_id)
    }

    /// Render frame
    pub fn render_frame(&mut self) -> Result<()> {
        trace!("Rendering frame {}", self.frame_sequence);

        // Clear damaged regions
        self.clear_damage();

        // Render windows in Z-order (bottom to top)
        for window_id in &self.z_order {
            if let Some(window) = self.windows.get(window_id) {
                if window.state != WindowState::Minimized {
                    self.render_window(window)?;
                }
            }
        }

        // Composite cursor
        self.render_cursor()?;

        self.frame_sequence += 1;
        self.framebuffer.sequence = self.frame_sequence;

        self.emit_event(CompositorEvent::FrameRendered);

        Ok(())
    }

    /// Clear damaged regions
    fn clear_damage(&mut self) {
        for region in &self.damage.regions {
            let start_y = region.y.max(0) as usize;
            let end_y = (region.y + region.height as i32).min(self.config.height as i32) as usize;
            let start_x = region.x.max(0) as usize;
            let end_x = (region.x + region.width as i32).min(self.config.width as i32) as usize;

            let stride = self.framebuffer.stride();
            let bpp = self.config.pixel_format.bytes_per_pixel();

            for y in start_y..end_y {
                let row_offset = y * stride;
                for x in start_x..end_x {
                    let offset = row_offset + x * bpp;
                    // Clear to black
                    self.framebuffer.data[offset..offset + bpp].fill(0);
                }
            }
        }
    }

    /// Render a window
    fn render_window(&mut self, window: &Window) -> Result<()> {
        // In a real implementation, this would copy the window's surface buffer
        // to the framebuffer. For now, we'll render a simple filled rectangle.

        let rect = &window.geometry;
        let stride = self.framebuffer.stride();
        let bpp = self.config.pixel_format.bytes_per_pixel();

        // Simple solid color based on window ID (for testing)
        let color = self.get_window_color(window.id);

        let start_y = rect.y.max(0) as usize;
        let end_y = (rect.y + rect.height as i32).min(self.config.height as i32) as usize;
        let start_x = rect.x.max(0) as usize;
        let end_x = (rect.x + rect.width as i32).min(self.config.width as i32) as usize;

        for y in start_y..end_y {
            let row_offset = y * stride;
            for x in start_x..end_x {
                let offset = row_offset + x * bpp;
                self.framebuffer.data[offset..offset + bpp].copy_from_slice(&color);
            }
        }

        Ok(())
    }

    /// Get a color for a window (for testing)
    fn get_window_color(&self, window_id: WindowId) -> [u8; 4] {
        // Generate deterministic color from window ID
        let id = window_id.0;
        let r = ((id * 73) % 200 + 55) as u8;
        let g = ((id * 151) % 200 + 55) as u8;
        let b = ((id * 233) % 200 + 55) as u8;

        // BGRA format
        [b, g, r, 255]
    }

    /// Render cursor
    fn render_cursor(&mut self) -> Result<()> {
        if !self.pointer.cursor.visible {
            return Ok(());
        }

        // TODO: Composite cursor image onto framebuffer
        // For now, cursor is just tracked in pointer state

        Ok(())
    }

    /// Emit event to listeners
    fn emit_event(&self, event: CompositorEvent) {
        for listener in &self.event_listeners {
            listener(event.clone());
        }
    }

    /// Add event listener
    pub fn add_event_listener<F>(&mut self, listener: F)
    where
        F: Fn(CompositorEvent) + Send + Sync + 'static,
    {
        self.event_listeners.push(Box::new(listener));
    }

    /// Get next Wayland protocol serial
    #[cfg(feature = "headless-compositor")]
    pub fn next_serial(&mut self) -> Serial {
        self.serial_counter = self.serial_counter.wrapping_add(1);
        Serial::from(self.serial_counter)
    }

    /// Mark entire framebuffer as damaged
    pub fn damage_all(&mut self) {
        self.damage.damage_all();
    }

    /// Add XDG shell window to tracking
    #[cfg(feature = "headless-compositor")]
    pub fn add_xdg_window(&mut self, window: smithay::desktop::Window) {
        // Map the window in the space
        debug!("Adding XDG window to space");
        // Window is already mapped in the protocol handler
    }

    /// Initialize Smithay protocol states with a DisplayHandle
    #[cfg(feature = "headless-compositor")]
    pub fn init_smithay_states(&mut self, display: &smithay::reexports::wayland_server::DisplayHandle) -> Result<()> {
        use super::protocols::*;

        info!("Initializing Smithay protocol states");

        // Initialize compositor state
        self.smithay_compositor_state = Some(SmithayCompositorState::new::<Self>(display));
        debug!("wl_compositor state initialized");

        // Initialize SHM state
        self.shm_state = Some(init_shm_global(display));

        // Initialize XDG shell state
        self.xdg_shell_state = Some(XdgShellState::new::<Self>(display));
        debug!("xdg_shell state initialized");

        // Initialize seat and data device states
        let (seat_state, seat) = init_seat_global(display, "wrd-seat");
        self.seat_state = Some(seat_state);
        self.seat = Some(seat);

        // Initialize data device state
        self.data_device_state = Some(init_data_device_global(display));

        // Initialize output
        self.output = Some(init_output_global(display, &self.config));

        // Initialize space for window management
        self.space = Some(Space::default());
        debug!("Desktop space initialized");

        info!("All Smithay protocol states initialized successfully");

        Ok(())
    }
}

/// Window representation
#[derive(Debug, Clone)]
pub struct Window {
    pub id: WindowId,
    pub geometry: Rectangle,
    pub state: WindowState,
    pub title: String,
    pub app_id: Option<String>,
    pub surface_id: Option<SurfaceId>,
}

impl Window {
    pub fn new(geometry: Rectangle) -> Self {
        Self {
            id: WindowId::new(),
            geometry,
            state: WindowState::Normal,
            title: String::new(),
            app_id: None,
            surface_id: None,
        }
    }

    pub fn handle_keyboard(&mut self, _event: &KeyboardEvent) -> Result<()> {
        // Forward to Wayland surface
        Ok(())
    }

    pub fn handle_pointer(&mut self, _event: &PointerEvent) -> Result<()> {
        // Forward to Wayland surface
        Ok(())
    }
}

/// Surface representation (Wayland surface)
#[derive(Debug)]
pub struct Surface {
    pub id: SurfaceId,
    pub buffer: Option<SurfaceBuffer>,
    pub damage: Vec<Rectangle>,
}

/// Surface buffer
#[derive(Debug)]
pub struct SurfaceBuffer {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub format: PixelFormat,
}

/// Damage tracker
#[derive(Debug)]
pub struct DamageTracker {
    pub regions: Vec<Rectangle>,
    pub width: u32,
    pub height: u32,
}

impl DamageTracker {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            regions: vec![Rectangle::new(0, 0, width, height)],
            width,
            height,
        }
    }

    pub fn damage_all(&mut self) {
        self.regions = vec![Rectangle::new(0, 0, self.width, self.height)];
    }

    pub fn damage_region(&mut self, region: Rectangle) {
        self.regions.push(region);
    }

    pub fn get_regions(&self) -> Vec<Rectangle> {
        self.regions.clone()
    }

    pub fn clear(&mut self) {
        self.regions.clear();
    }
}

/// Keyboard state
#[derive(Debug)]
pub struct KeyboardState {
    pub modifiers: Modifiers,
    pub pressed_keys: Vec<u32>,
}

impl KeyboardState {
    pub fn new() -> Result<Self> {
        Ok(Self {
            modifiers: Modifiers::default(),
            pressed_keys: Vec::new(),
        })
    }

    pub fn handle_event(&mut self, event: &KeyboardEvent) -> Result<()> {
        match event.state {
            KeyState::Pressed => {
                if !self.pressed_keys.contains(&event.key) {
                    self.pressed_keys.push(event.key);
                }
            }
            KeyState::Released => {
                self.pressed_keys.retain(|k| *k != event.key);
            }
        }

        self.modifiers = event.modifiers;
        Ok(())
    }
}

/// Pointer state
#[derive(Debug)]
pub struct PointerState {
    pub position: Point,
    pub pressed_buttons: Vec<u32>,
    pub cursor: CursorState,
}

impl PointerState {
    pub fn new() -> Self {
        Self {
            position: Point::new(0, 0),
            pressed_buttons: Vec::new(),
            cursor: CursorState::default(),
        }
    }

    pub fn handle_event(&mut self, event: &PointerEvent) -> Result<()> {
        self.position.x = event.x as i32;
        self.position.y = event.y as i32;

        if let Some((button, state)) = event.button {
            match state {
                ButtonState::Pressed => {
                    if !self.pressed_buttons.contains(&button) {
                        self.pressed_buttons.push(button);
                    }
                }
                ButtonState::Released => {
                    self.pressed_buttons.retain(|b| *b != button);
                }
            }
        }

        self.cursor.position = self.position;
        Ok(())
    }
}

/// Clipboard state
#[derive(Debug)]
pub struct ClipboardState {
    pub data: Vec<u8>,
    pub mime_type: String,
}

impl ClipboardState {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            mime_type: "text/plain".to_string(),
        }
    }
}

/// Main compositor struct
pub struct WrdCompositor {
    pub state: Arc<Mutex<CompositorState>>,
    #[cfg(feature = "headless-compositor")]
    pub event_loop: Option<EventLoop<'static, CompositorState>>,
}

impl WrdCompositor {
    /// Create new compositor
    pub fn new(config: CompositorConfig) -> Result<Self> {
        info!("Creating WRD compositor");

        let state = Arc::new(Mutex::new(CompositorState::new(config)?));

        #[cfg(feature = "headless-compositor")]
        {
            let event_loop = EventLoop::try_new()
                .context("Failed to create event loop")?;

            Ok(Self {
                state,
                event_loop: Some(event_loop),
            })
        }

        #[cfg(not(feature = "headless-compositor"))]
        {
            Ok(Self {
                state,
            })
        }
    }

    /// Get compositor handle
    pub fn handle(&self) -> super::CompositorHandle {
        super::CompositorHandle {
            state: Arc::clone(&self.state),
        }
    }

    /// Run compositor (blocking)
    #[cfg(feature = "headless-compositor")]
    pub fn run(&mut self) -> Result<()> {
        info!("Starting compositor event loop");

        let event_loop = self.event_loop.take()
            .ok_or_else(|| anyhow::anyhow!("Event loop already consumed"))?;

        // TODO: Register Wayland socket, timers, etc.

        event_loop.run(None, &mut *self.state.lock(), |_| {
            // Event loop callback
        }).context("Event loop error")?;

        Ok(())
    }

    #[cfg(not(feature = "headless-compositor"))]
    pub fn run(&mut self) -> Result<()> {
        anyhow::bail!("Compositor feature not enabled");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compositor_state_creation() {
        let config = CompositorConfig::default();
        let state = CompositorState::new(config);
        assert!(state.is_ok());
    }

    #[test]
    fn test_window_management() {
        let config = CompositorConfig::default();
        let mut state = CompositorState::new(config).unwrap();

        let window = Window::new(Rectangle::new(0, 0, 640, 480));
        let window_id = state.add_window(window);

        assert!(state.windows.contains_key(&window_id));
        assert_eq!(state.z_order.len(), 1);

        state.remove_window(window_id);
        assert!(!state.windows.contains_key(&window_id));
        assert_eq!(state.z_order.len(), 0);
    }

    #[test]
    fn test_window_at() {
        let config = CompositorConfig::default();
        let mut state = CompositorState::new(config).unwrap();

        let window = Window::new(Rectangle::new(100, 100, 200, 200));
        let window_id = state.add_window(window);

        assert_eq!(state.window_at(150, 150), Some(window_id));
        assert_eq!(state.window_at(50, 50), None);
    }
}
