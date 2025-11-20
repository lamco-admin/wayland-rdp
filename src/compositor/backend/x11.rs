//! X11 Backend for Lamco Compositor
//!
//! Provides X11 window backend using Smithay's X11Backend.
//! This allows running the compositor in a window for testing and development.
//!
//! For now, we use software rendering and don't require OpenGL.

use anyhow::{Context, Result};
use smithay::backend::x11::{X11Backend, X11Surface, X11Handle, WindowBuilder, Window};
use smithay::reexports::calloop::EventLoop;
use smithay::utils::{Logical, Size};
use tracing::{debug, info};

use crate::compositor::types::{CompositorConfig, FrameBuffer};

/// X11 backend for compositor
pub struct LamcoX11Backend {
    config: CompositorConfig,
    backend: Option<X11Backend>,
    window: Option<Window>,
    surface: Option<X11Surface>,
}

impl LamcoX11Backend {
    /// Create new X11 backend
    pub fn new(config: CompositorConfig) -> Result<Self> {
        info!("Creating X11 backend: {}x{}", config.width, config.height);

        Ok(Self {
            config,
            backend: None,
            window: None,
            surface: None,
        })
    }

    /// Initialize the backend with event loop
    pub fn init<Data: 'static>(
        &mut self,
        _event_loop: &mut EventLoop<Data>,
    ) -> Result<()> {
        info!("Initializing X11 backend");

        // Create X11 backend
        let backend = X11Backend::new()
            .context("Failed to create X11 backend")?;

        // Get handle to interact with X server
        let handle = backend.handle();

        // Create window with proper Size type
        let size = Size::<u16, Logical>::from((self.config.width as u16, self.config.height as u16));
        let window = WindowBuilder::new()
            .title("WRD Compositor")
            .size(size)
            .build(&handle)
            .context("Failed to create X11 window")?;

        info!("X11 window created: {}x{}", self.config.width, self.config.height);

        // Map the window to make it visible
        window.map();

        self.backend = Some(backend);
        self.window = Some(window);

        Ok(())
    }

    /// Get the X11 window
    pub fn window(&self) -> Option<&Window> {
        self.window.as_ref()
    }

    /// Get the X11 surface (if created)
    pub fn surface(&self) -> Option<&X11Surface> {
        self.surface.as_ref()
    }

    /// Get mutable surface
    pub fn surface_mut(&mut self) -> Option<&mut X11Surface> {
        self.surface.as_mut()
    }

    /// Present framebuffer to X11 window
    pub fn present(&mut self, _framebuffer: &FrameBuffer) -> Result<()> {
        // In a full implementation, we would:
        // 1. Get the X11 surface's EGL context
        // 2. Upload the framebuffer to a texture
        // 3. Render the texture to the window
        // 4. Swap buffers

        // For now, this is a stub - actual rendering requires EGL/OpenGL setup
        debug!("Present framebuffer (stub)");
        Ok(())
    }

    /// Get framebuffer for CPU rendering
    pub fn get_framebuffer(&self) -> FrameBuffer {
        FrameBuffer::new(
            self.config.width,
            self.config.height,
            self.config.pixel_format,
        )
    }

    /// Get the X11 backend handle
    pub fn handle(&self) -> Option<X11Handle> {
        self.backend.as_ref().map(|b| b.handle())
    }
}

/// Handle for X11 backend that can be shared
pub struct X11BackendHandle {
    config: CompositorConfig,
}

impl X11BackendHandle {
    pub fn new(config: CompositorConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &CompositorConfig {
        &self.config
    }
}
