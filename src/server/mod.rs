//! Server Implementation Module
//!
//! This module provides the main WRD-Server implementation, orchestrating all subsystems
//! to provide complete RDP server functionality for Wayland desktops.
//!
//! # Architecture
//!
//! The server integrates multiple subsystems:
//!
//! ```text
//! WrdServer
//!   ├─> Portal Session (screen capture + input injection permissions)
//!   ├─> PipeWire Thread Manager (video frame capture)
//!   ├─> Display Handler (video streaming to RDP clients)
//!   ├─> Input Handler (keyboard/mouse from RDP clients)
//!   ├─> Clipboard Manager (bidirectional clipboard sync)
//!   └─> IronRDP Server (RDP protocol, TLS, RemoteFX encoding)
//! ```
//!
//! # Data Flow
//!
//! **Video Path:** Portal → PipeWire → Display Handler → IronRDP → Client
//!
//! **Input Path:** Client → IronRDP → Input Handler → Portal → Compositor
//!
//! **Clipboard Path:** Client ↔ IronRDP ↔ Clipboard Manager ↔ Portal ↔ Compositor
//!
//! # Threading Model
//!
//! - **Tokio async runtime:** Main server logic, Portal API calls, frame processing
//! - **PipeWire thread:** Dedicated thread for PipeWire MainLoop (handles non-Send types)
//! - **IronRDP threads:** Managed by IronRDP library for protocol handling
//!
//! # Example
//!
//! ```no_run
//! use wrd_server::config::Config;
//! use wrd_server::server::WrdServer;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::load("config.toml")?;
//!     let server = WrdServer::new(config).await?;
//!     server.run().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Security
//!
//! - TLS 1.3 mandatory for all connections
//! - Certificate-based authentication
//! - Portal-based authorization (user approves screen sharing)
//! - No direct Wayland protocol access
//!
//! # Performance
//!
//! - Target: <100ms end-to-end latency
//! - Target: 30-60 FPS video streaming
//! - RemoteFX compression for efficient bandwidth usage

mod display_handler;
mod input_handler;

pub use display_handler::WrdDisplayHandler;
pub use input_handler::WrdInputHandler;

// Compositor mode (requires headless-compositor feature)
#[cfg(feature = "headless-compositor")]
pub mod compositor_mode;

use anyhow::{Context, Result};
use ironrdp_pdu::rdp::capability_sets::server_codecs_capabilities;
use ironrdp_server::{Credentials, RdpServer};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::clipboard::{ClipboardConfig, ClipboardManager, WrdCliprdrFactory};
use crate::config::Config;
use crate::input::coordinates::MonitorInfo as InputMonitorInfo;
use crate::portal::PortalManager;
use crate::security::TlsConfig;

/// WRD Server
///
/// Main server struct that orchestrates all subsystems and integrates
/// with IronRDP for RDP protocol handling.
pub struct WrdServer {
    /// Configuration (kept for future dynamic reconfiguration)
    #[allow(dead_code)]
    config: Arc<Config>,

    /// IronRDP server instance
    rdp_server: RdpServer,

    /// Portal manager for Wayland access (kept for resource cleanup)
    #[allow(dead_code)]
    portal_manager: Arc<PortalManager>,

    /// Display handler (kept for lifecycle management)
    #[allow(dead_code)]
    display_handler: Arc<WrdDisplayHandler>,

    // Portal session handle removed - session is consumed by input_handler
    // TODO: Refactor to allow session sharing between input and clipboard
}

impl WrdServer {
    /// Create a new WRD server instance
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration
    ///
    /// # Returns
    ///
    /// A new `WrdServer` instance ready to run
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing WRD Server");
        let config = Arc::new(config);

        // Initialize Portal manager
        info!("Setting up Portal connection");
        let portal_manager = Arc::new(
            PortalManager::new(&config)
                .await
                .context("Failed to initialize Portal manager")?,
        );

        // Create Portal Clipboard BEFORE creating session (if enabled)
        let portal_clipboard = if config.clipboard.enabled {
            match crate::portal::clipboard::ClipboardManager::new().await {
                Ok(clipboard_mgr) => {
                    info!("Portal Clipboard manager created");
                    Some(Arc::new(clipboard_mgr))
                }
                Err(e) => {
                    warn!("Failed to create Portal Clipboard: {:#}", e);
                    warn!("Clipboard will not be available");
                    None
                }
            }
        } else {
            info!("Clipboard disabled in configuration");
            None
        };

        // Set clipboard in portal manager so create_session() can use it
        if let Some(ref clipboard) = portal_clipboard {
            // Need mutable access to portal_manager
            // Actually PortalManager doesn't have &mut method
            // Clipboard request needs to happen in create_session()
            // Let me check if we can do it there instead
        }

        // Create combined portal session (pass clipboard to be enabled at correct time)
        info!("Creating combined portal session");
        let session_handle = portal_manager
            .create_session(portal_clipboard.as_ref().map(|c| c.as_ref()))
            .await
            .context("Failed to create portal session")?;

        info!("Portal session created successfully (clipboard enabled if available)");

        // Extract session details
        let pipewire_fd = session_handle.pipewire_fd;
        let stream_info = session_handle.streams;

        info!(
            "Portal session started with {} streams, PipeWire FD: {}",
            stream_info.len(),
            pipewire_fd
        );

        // Determine initial desktop size from first stream
        let initial_size = stream_info
            .first()
            .map(|s| (s.size.0 as u16, s.size.1 as u16))
            .unwrap_or((1920, 1080)); // Default fallback

        info!("Initial desktop size: {}x{}", initial_size.0, initial_size.1);

        // Create display handler with PipeWire FD and stream info
        let display_handler = Arc::new(
            WrdDisplayHandler::new(initial_size.0, initial_size.1, pipewire_fd, stream_info.clone())
                .await
                .context("Failed to create display handler")?,
        );

        // Start the display pipeline
        Arc::clone(&display_handler).start_pipeline();

        // Create input handler for mouse and keyboard injection
        info!("Creating input handler for mouse/keyboard control");

        // Convert stream info to monitor info for coordinate transformation
        let monitors: Vec<InputMonitorInfo> = stream_info
            .iter()
            .enumerate()
            .map(|(idx, stream)| InputMonitorInfo {
                id: idx as u32,
                name: format!("Monitor {}", idx),
                x: stream.position.0 as i32,
                y: stream.position.1 as i32,
                width: stream.size.0 as u32,
                height: stream.size.1 as u32,
                dpi: 96.0, // Default DPI
                scale_factor: 1.0, // Default scale, Portal doesn't provide this
                stream_x: stream.position.0 as u32,
                stream_y: stream.position.1 as u32,
                stream_width: stream.size.0 as u32,
                stream_height: stream.size.1 as u32,
                is_primary: idx == 0, // First monitor is primary
            })
            .collect();

        // Get the primary stream node ID for Portal input injection
        let primary_stream_id = stream_info
            .first()
            .map(|s| s.node_id)
            .unwrap_or(0);

        info!("Using PipeWire stream node ID {} for input injection", primary_stream_id);

        // Wrap session in Arc<Mutex> for sharing between input and clipboard
        let shared_session = Arc::new(Mutex::new(session_handle.session));

        let input_handler = WrdInputHandler::new(
            portal_manager.remote_desktop().clone(),
            Arc::clone(&shared_session), // Share session with input handler
            monitors,
            primary_stream_id,
        )
        .context("Failed to create input handler")?;

        info!("Input handler created successfully - mouse/keyboard enabled");

        // Create TLS acceptor from security config
        info!("Setting up TLS");
        let tls_config = TlsConfig::from_files(
            &config.security.cert_path,
            &config.security.key_path,
        )
        .context("Failed to load TLS certificates")?;

        let tls_acceptor = ironrdp_server::tokio_rustls::TlsAcceptor::from(tls_config.server_config());

        // Configure RemoteFX codec (IronRDP's built-in codec)
        // Server uses "remotefx" string to enable RemoteFX codec (default enabled)
        let codecs = server_codecs_capabilities(&["remotefx"])
            .map_err(|e| anyhow::anyhow!("Failed to create codec capabilities: {}", e))?;

        // Create clipboard manager
        info!("Initializing clipboard manager");
        let clipboard_config = ClipboardConfig::default();
        let mut clipboard_mgr = ClipboardManager::new(clipboard_config)
            .await
            .context("Failed to create clipboard manager")?;

        // Set Portal clipboard reference if available (async operation)
        if let Some(portal_clip) = portal_clipboard {
            clipboard_mgr.set_portal_clipboard(
                portal_clip,
                Arc::clone(&shared_session), // Share session with clipboard
            ).await;
            // Note: Success message logged inside set_portal_clipboard
        }

        let clipboard_manager = Arc::new(Mutex::new(clipboard_mgr));

        // Create clipboard factory for IronRDP
        let clipboard_factory = WrdCliprdrFactory::new(Arc::clone(&clipboard_manager));

        // Build IronRDP server using builder pattern
        info!("Building IronRDP server");
        let listen_addr: SocketAddr = config
            .server
            .listen_addr
            .parse()
            .context("Invalid listen address")?;

        let rdp_server = RdpServer::builder()
            .with_addr(listen_addr)
            .with_tls(tls_acceptor)
            .with_input_handler(input_handler)
            .with_display_handler((*display_handler).clone()) // Clone the handler for IronRDP
            .with_bitmap_codecs(codecs)
            .with_cliprdr_factory(Some(Box::new(clipboard_factory)))
            .build();

        info!("WRD Server initialized successfully");

        Ok(Self {
            config,
            rdp_server,
            portal_manager,
            display_handler,
        })
    }

    /// Run the server
    ///
    /// This starts the RDP server and handles incoming connections.
    /// Blocks until the server is shut down.
    pub async fn run(mut self) -> Result<()> {
        info!("╔════════════════════════════════════════════════════════════╗");
        info!("║          WRD-Server is Starting                            ║");
        info!("╚════════════════════════════════════════════════════════════╝");
        info!("  Listen Address: {}", self.config.server.listen_addr);
        info!("  TLS: Enabled (rustls 0.23)");
        info!("  Codec: RemoteFX");
        info!("  Max Connections: {}", self.config.server.max_connections);
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        info!("Server is ready and listening for RDP connections");
        info!("Waiting for clients to connect...");

        // Set credentials for RDP authentication
        // Even with auth_method="none", we need to set empty/test credentials
        // for IronRDP to complete the protocol handshake properly
        let credentials = if self.config.security.auth_method == "none" {
            Some(Credentials {
                username: String::new(),
                password: String::new(),
                domain: None,
            })
        } else {
            // For future authentication support
            None
        };

        self.rdp_server.set_credentials(credentials);
        info!("Authentication configured: {}", self.config.security.auth_method);

        // Run the IronRDP server
        let result = self.rdp_server
            .run()
            .await
            .context("RDP server error");

        if let Err(ref e) = result {
            error!("Server stopped with error: {:#}", e);
        } else {
            info!("Server stopped gracefully");
        }

        info!("WRD Server shutdown complete");
        result
    }

    /// Graceful shutdown
    ///
    /// Sends a quit event to stop the server gracefully.
    pub fn shutdown(&self) {
        info!("Initiating graceful shutdown");
        let _ = self
            .rdp_server
            .event_sender()
            .send(ironrdp_server::ServerEvent::Quit(
                "Shutdown requested".to_string(),
            ));
    }
}

impl Drop for WrdServer {
    fn drop(&mut self) {
        debug!("WrdServer dropped - cleaning up resources");
        // Resources are automatically cleaned up through Arc<Mutex<>> drops
        // and tokio task cancellation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires D-Bus and portal access
    async fn test_server_initialization() {
        // This test would require a full environment
        // For now, just verify compilation
    }
}
