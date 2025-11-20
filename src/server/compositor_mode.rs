//! Compositor Mode Server
//!
//! Runs WRD-Server with integrated Lamco compositor for headless deployment.

use anyhow::{Context, Result};
use bytes::Bytes;
use ironrdp_pdu::rdp::capability_sets::server_codecs_capabilities;
use ironrdp_server::{
    BitmapUpdate as IronBitmapUpdate, Credentials, DesktopSize, DisplayUpdate,
    KeyboardEvent as IronKeyboardEvent, MouseEvent as IronMouseEvent,
    PixelFormat as IronPixelFormat, RdpServer, RdpServerDisplay, RdpServerDisplayUpdates,
    RdpServerInputHandler,
};
use std::net::SocketAddr;
use std::num::{NonZeroU16, NonZeroUsize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, trace, warn};

use crate::compositor::{CompositorConfig, CompositorRdpIntegration, CompositorRuntime};
use crate::config::Config;
use crate::security::TlsConfig;

/// Run WRD-Server in compositor mode
///
/// This mode uses the integrated Lamco compositor instead of Portal,
/// enabling headless deployment without requiring an existing desktop environment.
pub async fn run_compositor_mode(config: Config) -> Result<()> {
    info!("╔════════════════════════════════════════════════════════════╗");
    info!("║    WRD-Server - Compositor Mode (Headless Capable)        ║");
    info!("╚════════════════════════════════════════════════════════════╝");

    let config = Arc::new(config);

    // Create compositor configuration
    let compositor_config = CompositorConfig {
        width: 1920,  // TODO: Get from config
        height: 1080,
        refresh_rate: 60,
        scale: 1.0,
        socket_name: "wayland-rdp".to_string(),
        xwayland: false,
        max_clients: 4,
        pixel_format: crate::compositor::types::PixelFormat::BGRX8888,
    };

    info!("Initializing Lamco compositor: {}x{}", compositor_config.width, compositor_config.height);

    // Create compositor integration first
    let integration = Arc::new(CompositorRdpIntegration::new(compositor_config.clone())?);

    info!("Compositor integration created");

    // TODO: Create proper compositor runtime that uses X11 backend
    // For now, we use integration directly without full compositor event loop
    // This provides rendering capability for RDP without actual Wayland client support

    // Future work:
    // 1. Create CompositorRuntime with X11 backend
    // 2. Initialize Wayland server
    // 3. Wire compositor event loop to X11 events
    // 4. Run compositor in dedicated thread

    // Build RDP server with compositor integration
    info!("Building RDP server with compositor backends");
    let mut rdp_server = build_compositor_rdp_server(Arc::clone(&integration), &config).await?;

    info!("Compositor mode initialized successfully");
    info!("Ready for RDP connections");

    // Set credentials for RDP authentication
    let credentials = if config.security.auth_method == "none" {
        Some(Credentials {
            username: String::new(),
            password: String::new(),
            domain: None,
        })
    } else {
        None
    };
    rdp_server.set_credentials(credentials);

    info!("Starting RDP server on {}", config.server.listen_addr);

    // Run RDP server (this is the main loop)
    let result = rdp_server.run().await.context("RDP server error");

    if let Err(ref e) = result {
        error!("Server stopped with error: {:#}", e);
    } else {
        info!("Server stopped gracefully");
    }

    info!("Compositor mode shutdown complete");
    result
}

/// Build IronRDP server for compositor mode
///
/// Creates RDP server that uses compositor for video/input instead of Portal.
async fn build_compositor_rdp_server(
    integration: Arc<CompositorRdpIntegration>,
    config: &Config,
) -> Result<RdpServer> {
    info!("Building RDP server for compositor mode");

    // Create display handler that gets frames from compositor
    let display_handler = CompositorDisplayHandler::new(Arc::clone(&integration))?;

    // Start the rendering pipeline
    Arc::new(display_handler.clone()).start_pipeline();

    // Create input handler that injects to compositor
    let input_handler = CompositorInputHandler::new(Arc::clone(&integration))?;

    // Setup TLS
    let tls_config = TlsConfig::from_files(
        &config.security.cert_path,
        &config.security.key_path,
    )?;

    let tls_acceptor = ironrdp_server::tokio_rustls::TlsAcceptor::from(tls_config.server_config());

    // Configure RemoteFX codec
    let codecs = server_codecs_capabilities(&["remotefx"])
        .map_err(|e| anyhow::anyhow!("Failed to create codec capabilities: {}", e))?;

    // Parse listen address
    let listen_addr: SocketAddr = config
        .server
        .listen_addr
        .parse()
        .context("Invalid listen address")?;

    // Build RDP server
    let server = RdpServer::builder()
        .with_addr(listen_addr)
        .with_tls(tls_acceptor)
        .with_input_handler(input_handler)
        .with_display_handler(display_handler)
        .with_bitmap_codecs(codecs)
        .build();

    info!("RDP server built successfully");

    Ok(server)
}

/// Display handler for compositor mode
///
/// Implements RdpServerDisplay to provide video frames from the compositor.
pub struct CompositorDisplayHandler {
    /// Compositor integration
    integration: Arc<CompositorRdpIntegration>,

    /// Current desktop size
    size: Arc<RwLock<DesktopSize>>,

    /// Display update sender
    update_sender: mpsc::Sender<DisplayUpdate>,

    /// Display update receiver (wrapped for cloning)
    update_receiver: Arc<Mutex<Option<mpsc::Receiver<DisplayUpdate>>>>,
}

impl CompositorDisplayHandler {
    /// Create new compositor display handler
    pub fn new(integration: Arc<CompositorRdpIntegration>) -> Result<Self> {
        // Note: integration provides rendering without actual compositor event loop

        // Default size 1920x1080
        let size = Arc::new(RwLock::new(DesktopSize {
            width: 1920,
            height: 1080,
        }));

        // Create channel for display updates
        let (update_sender, update_receiver) = mpsc::channel(64);
        let update_receiver = Arc::new(Mutex::new(Some(update_receiver)));

        info!("Compositor display handler created");

        Ok(Self {
            integration,
            size,
            update_sender,
            update_receiver,
        })
    }

    /// Start the rendering pipeline
    pub fn start_pipeline(self: Arc<Self>) {
        let handler = Arc::clone(&self);

        tokio::spawn(async move {
            info!("Starting compositor rendering pipeline");

            // Target 30 FPS (33ms per frame)
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(33));

            loop {
                interval.tick().await;

                // Render frame from compositor
                let frame = match handler.integration.render_frame() {
                    Ok(f) => f,
                    Err(e) => {
                        error!("Failed to render frame: {}", e);
                        continue;
                    }
                };

                trace!("Rendered frame {}", frame.sequence);

                // Convert to IronRDP bitmap format
                let iron_update = match handler.convert_to_iron_format(&frame).await {
                    Ok(u) => u,
                    Err(e) => {
                        error!("Failed to convert frame: {}", e);
                        continue;
                    }
                };

                // Send update
                let update = DisplayUpdate::Bitmap(iron_update);
                if let Err(e) = handler.update_sender.send(update).await {
                    error!("Failed to send display update: {}", e);
                    // Channel closed, stop pipeline
                    return;
                }
            }
        });
    }

    /// Convert rendered frame to IronRDP bitmap format
    async fn convert_to_iron_format(&self, frame: &crate::compositor::integration::RenderedFrame) -> Result<IronBitmapUpdate> {
        let (width, height) = frame.dimensions();

        // Map pixel format
        let iron_format = match frame.pixel_format() {
            crate::compositor::types::PixelFormat::BGRX8888 => IronPixelFormat::BgrX32,
            crate::compositor::types::PixelFormat::BGRA8888 => IronPixelFormat::BgrX32,
            crate::compositor::types::PixelFormat::RGBA8888 => IronPixelFormat::XRgb32,
            crate::compositor::types::PixelFormat::RGBX8888 => IronPixelFormat::XRgb32,
        };

        let bytes_per_pixel = iron_format.bytes_per_pixel() as usize;
        let stride = NonZeroUsize::new(width as usize * bytes_per_pixel)
            .ok_or_else(|| anyhow::anyhow!("Invalid stride"))?;

        // Create bitmap update for full frame
        // TODO: Use damage regions for partial updates
        let iron_bitmap = IronBitmapUpdate {
            x: 0,
            y: 0,
            width: NonZeroU16::new(width as u16)
                .ok_or_else(|| anyhow::anyhow!("Invalid width"))?,
            height: NonZeroU16::new(height as u16)
                .ok_or_else(|| anyhow::anyhow!("Invalid height"))?,
            format: iron_format,
            data: Bytes::from(frame.pixel_data().to_vec()),
            stride,
        };

        Ok(iron_bitmap)
    }
}

impl Clone for CompositorDisplayHandler {
    fn clone(&self) -> Self {
        Self {
            integration: Arc::clone(&self.integration),
            size: Arc::clone(&self.size),
            update_sender: self.update_sender.clone(),
            update_receiver: Arc::clone(&self.update_receiver),
        }
    }
}

#[async_trait::async_trait]
impl RdpServerDisplay for CompositorDisplayHandler {
    async fn size(&mut self) -> DesktopSize {
        let size = self.size.read().await;
        *size
    }

    async fn updates(&mut self) -> Result<Box<dyn RdpServerDisplayUpdates>> {
        let mut receiver_option = self.update_receiver.lock().await;
        let receiver = receiver_option
            .take()
            .ok_or_else(|| anyhow::anyhow!("Display updates already claimed"))?;

        Ok(Box::new(CompositorDisplayUpdatesStream::new(receiver)))
    }

    fn request_layout(&mut self, layout: ironrdp_displaycontrol::pdu::DisplayControlMonitorLayout) {
        debug!("Client requested layout change: {:?}", layout);
        warn!("Dynamic layout changes not yet implemented in compositor mode");
    }
}

/// Display updates stream for compositor
struct CompositorDisplayUpdatesStream {
    receiver: mpsc::Receiver<DisplayUpdate>,
}

impl CompositorDisplayUpdatesStream {
    fn new(receiver: mpsc::Receiver<DisplayUpdate>) -> Self {
        Self { receiver }
    }
}

#[async_trait::async_trait]
impl RdpServerDisplayUpdates for CompositorDisplayUpdatesStream {
    async fn next_update(&mut self) -> Result<Option<DisplayUpdate>> {
        match self.receiver.recv().await {
            Some(update) => {
                trace!("Providing display update");
                Ok(Some(update))
            }
            None => {
                debug!("Display update stream closed");
                Ok(None)
            }
        }
    }
}

/// Input handler for compositor mode
///
/// Implements RdpServerInputHandler to forward input events to the compositor.
pub struct CompositorInputHandler {
    /// Compositor integration
    integration: Arc<CompositorRdpIntegration>,
}

impl CompositorInputHandler {
    /// Create new compositor input handler
    pub fn new(integration: Arc<CompositorRdpIntegration>) -> Result<Self> {
        info!("Compositor input handler created");
        Ok(Self { integration })
    }

    /// Handle keyboard event asynchronously
    async fn handle_keyboard_async(&self, event: IronKeyboardEvent) -> Result<()> {
        match event {
            IronKeyboardEvent::Pressed { code, extended } => {
                debug!("Keyboard pressed: code={}, extended={}", code, extended);
                // RDP scancode → evdev keycode (approximately scancode + 8 for most keys)
                let scancode = if extended {
                    0xE000 | (code as u32)
                } else {
                    code as u32
                };
                self.integration.handle_rdp_keyboard(scancode, true)?;
            }
            IronKeyboardEvent::Released { code, extended } => {
                debug!("Keyboard released: code={}, extended={}", code, extended);
                let scancode = if extended {
                    0xE000 | (code as u32)
                } else {
                    code as u32
                };
                self.integration.handle_rdp_keyboard(scancode, false)?;
            }
            IronKeyboardEvent::UnicodePressed(unicode) => {
                debug!("Unicode pressed: 0x{:04X}", unicode);
                warn!("Unicode keyboard events not yet supported in compositor mode");
            }
            IronKeyboardEvent::UnicodeReleased(unicode) => {
                debug!("Unicode released: 0x{:04X}", unicode);
                warn!("Unicode keyboard events not yet supported in compositor mode");
            }
            IronKeyboardEvent::Synchronize(flags) => {
                debug!("Keyboard synchronize: {:?}", flags);
                // Toggle key state sync - compositor handles this internally
            }
        }
        Ok(())
    }

    /// Handle mouse event asynchronously
    async fn handle_mouse_async(&self, event: IronMouseEvent) -> Result<()> {
        match event {
            IronMouseEvent::Move { x, y } => {
                trace!("Mouse move: x={}, y={}", x, y);
                self.integration.handle_rdp_pointer_motion(x, y)?;
            }
            IronMouseEvent::RelMove { x, y } => {
                debug!("Mouse relative move: dx={}, dy={}", x, y);
                // Compositor expects absolute coordinates
                // For now, log as unsupported - would need to track current position
                warn!("Relative mouse movement not yet fully supported in compositor mode");
            }
            IronMouseEvent::LeftPressed => {
                debug!("Left button pressed");
                self.integration.handle_rdp_pointer_button(1, true)?; // BTN_LEFT
            }
            IronMouseEvent::LeftReleased => {
                debug!("Left button released");
                self.integration.handle_rdp_pointer_button(1, false)?;
            }
            IronMouseEvent::RightPressed => {
                debug!("Right button pressed");
                self.integration.handle_rdp_pointer_button(2, true)?; // BTN_RIGHT
            }
            IronMouseEvent::RightReleased => {
                debug!("Right button released");
                self.integration.handle_rdp_pointer_button(2, false)?;
            }
            IronMouseEvent::MiddlePressed => {
                debug!("Middle button pressed");
                self.integration.handle_rdp_pointer_button(3, true)?; // BTN_MIDDLE
            }
            IronMouseEvent::MiddleReleased => {
                debug!("Middle button released");
                self.integration.handle_rdp_pointer_button(3, false)?;
            }
            IronMouseEvent::Button4Pressed => {
                debug!("Button 4 pressed");
                self.integration.handle_rdp_pointer_button(4, true)?;
            }
            IronMouseEvent::Button4Released => {
                debug!("Button 4 released");
                self.integration.handle_rdp_pointer_button(4, false)?;
            }
            IronMouseEvent::Button5Pressed => {
                debug!("Button 5 pressed");
                self.integration.handle_rdp_pointer_button(5, true)?;
            }
            IronMouseEvent::Button5Released => {
                debug!("Button 5 released");
                self.integration.handle_rdp_pointer_button(5, false)?;
            }
            IronMouseEvent::VerticalScroll { value } => {
                debug!("Vertical scroll: {}", value);
                // TODO: Implement scroll support in compositor
                warn!("Mouse scroll not yet supported in compositor mode");
            }
            IronMouseEvent::Scroll { x, y } => {
                debug!("Scroll: x={}, y={}", x, y);
                warn!("Mouse scroll not yet supported in compositor mode");
            }
        }
        Ok(())
    }
}

impl Clone for CompositorInputHandler {
    fn clone(&self) -> Self {
        Self {
            integration: Arc::clone(&self.integration),
        }
    }
}

impl RdpServerInputHandler for CompositorInputHandler {
    fn keyboard(&mut self, event: IronKeyboardEvent) {
        let integration = Arc::clone(&self.integration);

        tokio::spawn(async move {
            let handler = CompositorInputHandler { integration };
            if let Err(e) = handler.handle_keyboard_async(event).await {
                error!("Failed to handle keyboard event: {}", e);
            }
        });
    }

    fn mouse(&mut self, event: IronMouseEvent) {
        let integration = Arc::clone(&self.integration);

        tokio::spawn(async move {
            let handler = CompositorInputHandler { integration };
            if let Err(e) = handler.handle_mouse_async(event).await {
                error!("Failed to handle mouse event: {}", e);
            }
        });
    }
}
