//! Headless RDP Server Infrastructure
//!
//! This module provides complete headless RDP server capabilities, enabling:
//! - Operation without physical display or desktop environment
//! - Multi-user session management with systemd-logind integration
//! - Direct RDP login (RDP-as-display-manager)
//! - Embedded portal backend with automatic permission grants
//! - Per-user compositor instances with resource isolation
//! - PAM authentication for secure multi-user access
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │  Headless Server (No GUI)                               │
//! │                                                         │
//! │  ┌────────────────────┐  ┌────────────────────┐        │
//! │  │  Login Service     │  │  Session Manager   │        │
//! │  │  • PAM Auth        │  │  • Per-user        │        │
//! │  │  • RDP Login       │  │  • systemd-logind  │        │
//! │  └─────────┬──────────┘  └─────────┬──────────┘        │
//! │            │                       │                    │
//! │            ▼                       ▼                    │
//! │  ┌─────────────────────────────────────────────┐       │
//! │  │  Headless Compositor (Per-User)             │       │
//! │  │  • Smithay-based minimal compositor         │       │
//! │  │  • Software rendering (llvmpipe/pixman)     │       │
//! │  │  • Virtual display management               │       │
//! │  │  • Embedded portal backend                  │       │
//! │  └─────────┬───────────────────────────────────┘       │
//! │            │                                            │
//! │            ▼                                            │
//! │  ┌─────────────────────────────────────────────┐       │
//! │  │  PipeWire (Headless)                        │       │
//! │  │  • No physical audio/video                  │       │
//! │  │  • Virtual streams for RDP                  │       │
//! │  └─────────┬───────────────────────────────────┘       │
//! │            │                                            │
//! │            ▼                                            │
//! │  ┌─────────────────────────────────────────────┐       │
//! │  │  WRD Server (Per-Session)                   │       │
//! │  │  • RDP protocol                             │       │
//! │  │  • Video encoding                           │       │
//! │  │  • Input injection                          │       │
//! │  └─────────────────────────────────────────────┘       │
//! └─────────────────────────────────────────────────────────┘
//! ```

pub mod auth;
pub mod compositor;
pub mod config;
pub mod login_service;
pub mod portal_backend;
pub mod resources;
pub mod session;

pub use auth::{AuthProvider, AuthResult, PamAuthenticator};
pub use compositor::{HeadlessCompositor, VirtualDisplay};
pub use config::HeadlessConfig;
pub use login_service::{LoginService, LoginServiceHandle};
pub use portal_backend::{EmbeddedPortalBackend, PermissionPolicy};
pub use resources::{ResourceLimits, ResourceManager};
pub use session::{SessionId, SessionInfo, SessionManager, UserSession};

use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info};

/// Headless server instance managing all headless operations
pub struct HeadlessServer {
    config: Arc<HeadlessConfig>,
    login_service: Arc<LoginService>,
    session_manager: Arc<SessionManager>,
    resource_manager: Arc<ResourceManager>,
}

impl HeadlessServer {
    /// Create new headless server instance
    pub async fn new(config: HeadlessConfig) -> Result<Self> {
        info!("Initializing headless RDP server");

        let config = Arc::new(config);

        // Initialize resource manager first (needed for session isolation)
        let resource_manager = Arc::new(ResourceManager::new(config.clone()).await?);
        info!("Resource manager initialized");

        // Initialize session manager (manages per-user compositor instances)
        let session_manager = Arc::new(
            SessionManager::new(config.clone(), resource_manager.clone()).await?,
        );
        info!("Session manager initialized");

        // Initialize login service (handles RDP authentication and session creation)
        let login_service =
            Arc::new(LoginService::new(config.clone(), session_manager.clone()).await?);
        info!("Login service initialized");

        info!("Headless RDP server initialized successfully");

        Ok(Self {
            config,
            login_service,
            session_manager,
            resource_manager,
        })
    }

    /// Start the headless server
    ///
    /// This will:
    /// 1. Start the login service (listening for RDP connections)
    /// 2. Monitor active sessions
    /// 3. Handle session lifecycle
    /// 4. Cleanup resources on shutdown
    pub async fn run(self) -> Result<()> {
        info!("Starting headless RDP server");

        // Start login service (this listens for incoming RDP connections)
        let login_handle = self.login_service.start().await?;
        info!("Login service started on {}", self.config.listen_address);

        // Wait for shutdown signal
        tokio::select! {
            result = login_handle.wait() => {
                match result {
                    Ok(_) => info!("Login service stopped normally"),
                    Err(e) => error!("Login service error: {}", e),
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received shutdown signal");
            }
        }

        // Graceful shutdown
        info!("Shutting down headless server");
        self.shutdown().await?;

        Ok(())
    }

    /// Gracefully shutdown the server
    async fn shutdown(&self) -> Result<()> {
        info!("Performing graceful shutdown");

        // Stop accepting new connections
        self.login_service.stop().await?;

        // Terminate all active sessions
        self.session_manager.terminate_all_sessions().await?;

        // Cleanup resources
        self.resource_manager.cleanup().await?;

        info!("Shutdown complete");
        Ok(())
    }

    /// Get current server status
    pub async fn status(&self) -> ServerStatus {
        ServerStatus {
            active_sessions: self.session_manager.session_count().await,
            total_connections: self.login_service.connection_count().await,
            uptime: self.login_service.uptime().await,
            memory_usage: self.resource_manager.total_memory_usage().await,
            cpu_usage: self.resource_manager.total_cpu_usage().await,
        }
    }
}

/// Server status information
#[derive(Debug, Clone)]
pub struct ServerStatus {
    pub active_sessions: usize,
    pub total_connections: u64,
    pub uptime: std::time::Duration,
    pub memory_usage: u64,
    pub cpu_usage: f32,
}
