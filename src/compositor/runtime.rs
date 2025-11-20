//! Compositor runtime
//!
//! Complete integrated compositor with all components working together.

use super::state::CompositorState;
use super::types::CompositorConfig;
use super::software_renderer::SoftwareRenderer;
use super::buffer_management::BufferManager;
use super::input_delivery::InputDelivery;
use super::dispatch::WaylandDispatcher;
use super::integration::CompositorRdpIntegration;
use anyhow::{Context, Result};
use std::sync::Arc;
use parking_lot::Mutex;
use smithay::reexports::wayland_server::Display;
use smithay::reexports::calloop::EventLoop;
use tracing::{debug, error, info};

/// Complete compositor runtime
pub struct CompositorRuntime {
    /// Compositor state
    state: Arc<Mutex<CompositorState>>,

    /// Software renderer
    renderer: Arc<Mutex<SoftwareRenderer>>,

    /// Buffer manager
    buffer_manager: Arc<Mutex<BufferManager>>,

    /// Input delivery
    input_delivery: Arc<Mutex<InputDelivery>>,

    /// RDP integration
    rdp_integration: Arc<CompositorRdpIntegration>,

    /// Wayland dispatcher
    dispatcher: Option<WaylandDispatcher>,

    /// Configuration
    config: CompositorConfig,
}

impl CompositorRuntime {
    /// Create new compositor runtime
    pub fn new(config: CompositorConfig) -> Result<Self> {
        info!("Creating compositor runtime");

        // Create compositor state
        let state = Arc::new(Mutex::new(
            CompositorState::new(config.clone())
                .context("Failed to create compositor state")?
        ));

        // Create software renderer
        let renderer = Arc::new(Mutex::new(
            SoftwareRenderer::new(config.width, config.height, config.pixel_format)
        ));

        // Create buffer manager
        let buffer_manager = Arc::new(Mutex::new(BufferManager::new()));

        // Create input delivery
        let input_delivery = Arc::new(Mutex::new(InputDelivery::new()));

        // Create RDP integration
        let rdp_integration = Arc::new(
            CompositorRdpIntegration::new(config.clone())
                .context("Failed to create RDP integration")?
        );

        info!("Compositor runtime created successfully");

        Ok(Self {
            state,
            renderer,
            buffer_manager,
            input_delivery,
            rdp_integration,
            dispatcher: None,
            config,
        })
    }

    /// Initialize Wayland server
    pub fn init_wayland(&mut self) -> Result<()> {
        info!("Initializing Wayland server");

        // Create Wayland display
        let mut display = Display::new()
            .context("Failed to create Wayland display")?;

        // Initialize Smithay protocol states
        {
            let mut state = self.state.lock();
            state.init_smithay_states(&display.handle())
                .context("Failed to initialize Smithay states")?;
        }

        // Create event loop
        let event_loop = EventLoop::try_new()
            .context("Failed to create event loop")?;

        // Create Wayland dispatcher
        let dispatcher = WaylandDispatcher::new(
            display,
            event_loop,
            Arc::clone(&self.state),
        )?;

        self.dispatcher = Some(dispatcher);

        info!("Wayland server initialized successfully");

        Ok(())
    }

    /// Run the compositor
    pub fn run(self) -> Result<()> {
        info!("Starting compositor runtime");

        // Run Wayland dispatcher
        if let Some(dispatcher) = self.dispatcher {
            dispatcher.run()
                .context("Wayland dispatcher error")?;
        } else {
            anyhow::bail!("Wayland server not initialized - call init_wayland() first");
        }

        info!("Compositor runtime stopped");

        Ok(())
    }

    /// Get RDP integration handle
    pub fn rdp_integration(&self) -> Arc<CompositorRdpIntegration> {
        Arc::clone(&self.rdp_integration)
    }

    /// Get compositor state handle
    pub fn state(&self) -> Arc<Mutex<CompositorState>> {
        Arc::clone(&self.state)
    }

    /// Render a frame (for RDP streaming)
    pub fn render_frame_for_rdp(&self) -> Result<super::integration::RenderedFrame> {
        debug!("Rendering frame for RDP");

        let mut state = self.state.lock();
        let mut renderer = self.renderer.lock();
        let buffer_manager = self.buffer_manager.lock();

        // Clear renderer
        renderer.clear();

        // Render all windows
        if let Some(space) = &state.space {
            for window in space.elements() {
                // Get window geometry
                if let Some(geo) = space.element_geometry(window) {
                    // Get window surface
                    let surface = window.toplevel().wl_surface();

                    // Render surface tree
                    buffer_manager.render_surface_tree(
                        surface,
                        &mut renderer,
                        (geo.loc.x, geo.loc.y),
                        &state,
                    )?;
                }
            }
        }

        // Render cursor
        renderer.render_cursor(&state.pointer.cursor)?;

        // Update damage
        renderer.update_framebuffer_damage();

        // Get framebuffer
        let framebuffer = renderer.framebuffer().clone();

        // Increment frame counter
        state.frame_sequence += 1;
        let sequence = state.frame_sequence;

        Ok(super::integration::RenderedFrame {
            framebuffer,
            sequence,
        })
    }

    /// Inject input from RDP
    pub fn inject_rdp_input(
        &self,
        event: super::input::KeyboardEvent,
    ) -> Result<()> {
        let mut state = self.state.lock();
        let mut input_delivery = self.input_delivery.lock();

        input_delivery.deliver_keyboard(&event, &mut state)
    }

    /// Inject pointer input from RDP
    pub fn inject_rdp_pointer(
        &self,
        event: super::input::PointerEvent,
    ) -> Result<()> {
        let mut state = self.state.lock();
        let mut input_delivery = self.input_delivery.lock();

        // Deliver motion
        input_delivery.deliver_pointer_motion(&event, &mut state)?;

        // Deliver button if present
        if event.button.is_some() {
            input_delivery.deliver_pointer_button(&event, &mut state)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let config = CompositorConfig::default();
        let runtime = CompositorRuntime::new(config);
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_rdp_integration_handle() {
        let config = CompositorConfig::default();
        let runtime = CompositorRuntime::new(config).unwrap();

        let rdp = runtime.rdp_integration();
        let stats = rdp.get_stats();
        assert_eq!(stats.window_count, 0);
    }
}
