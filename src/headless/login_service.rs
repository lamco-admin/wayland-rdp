//! Direct Login Service for Headless RDP
//!
//! Implements RDP-as-display-manager functionality where RDP connections
//! directly create user sessions without requiring a local desktop login.
//! This is essential for headless VDI deployment.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │  RDP Client (Windows/Linux/macOS)                   │
//! └──────────────────────┬──────────────────────────────┘
//!                        │
//!                        │ TCP:3389 (TLS encrypted)
//!                        ▼
//! ┌─────────────────────────────────────────────────────┐
//! │  Login Service (Listens on port 3389)               │
//! │  • Accept RDP connection                            │
//! │  • Extract username/password from RDP handshake     │
//! │  • Authenticate via PAM                             │
//! │  • Create user session (compositor + wrd-server)    │
//! │  • Hand off to session's RDP server                 │
//! └─────────────────────────────────────────────────────┘
//! ```

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::headless::auth::{AuthProvider, create_auth_provider};
use crate::headless::config::HeadlessConfig;
use crate::headless::session::{SessionManager, UserSession};

/// Login service handle for lifecycle management
pub struct LoginServiceHandle {
    shutdown_tx: mpsc::Sender<()>,
    stats: Arc<LoginServiceStats>,
}

impl LoginServiceHandle {
    /// Wait for service to stop
    pub async fn wait(self) -> Result<()> {
        // Service will run until shutdown
        Ok(())
    }

    /// Get service statistics
    pub fn stats(&self) -> Arc<LoginServiceStats> {
        self.stats.clone()
    }
}

/// Login service statistics
#[derive(Debug)]
pub struct LoginServiceStats {
    pub total_connections: AtomicU64,
    pub successful_logins: AtomicU64,
    pub failed_logins: AtomicU64,
    pub active_connections: AtomicU64,
    pub start_time: SystemTime,
}

impl LoginServiceStats {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            total_connections: AtomicU64::new(0),
            successful_logins: AtomicU64::new(0),
            failed_logins: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            start_time: SystemTime::now(),
        })
    }
}

/// Direct RDP login service
pub struct LoginService {
    config: Arc<HeadlessConfig>,
    session_manager: Arc<SessionManager>,
    auth_provider: Arc<dyn AuthProvider>,
    stats: Arc<LoginServiceStats>,
    active_sessions: Arc<RwLock<HashMap<String, Arc<UserSession>>>>,
}

impl LoginService {
    /// Create new login service
    pub async fn new(
        config: Arc<HeadlessConfig>,
        session_manager: Arc<SessionManager>,
    ) -> Result<Self> {
        info!("Initializing login service");

        // Create authentication provider
        let auth_provider = create_auth_provider(Arc::new(config.authentication.clone()))
            .await
            .context("Failed to create authentication provider")?;

        let stats = LoginServiceStats::new();

        Ok(Self {
            config,
            session_manager,
            auth_provider,
            stats,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Start the login service
    pub async fn start(&self) -> Result<LoginServiceHandle> {
        info!("Starting login service on {}", self.config.listen_address);

        // Create TCP listener
        let listener = TcpListener::bind(&self.config.listen_address)
            .await
            .context("Failed to bind to listen address")?;

        let local_addr = listener.local_addr()?;
        info!("Login service listening on {}", local_addr);

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Clone for task
        let config = self.config.clone();
        let session_manager = self.session_manager.clone();
        let auth_provider = self.auth_provider.clone();
        let stats = self.stats.clone();
        let active_sessions = self.active_sessions.clone();

        // Spawn accept loop
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, addr)) => {
                                info!("New RDP connection from: {}", addr);
                                stats.total_connections.fetch_add(1, Ordering::Relaxed);
                                stats.active_connections.fetch_add(1, Ordering::Relaxed);

                                // Spawn handler for this connection
                                let config = config.clone();
                                let session_manager = session_manager.clone();
                                let auth_provider = auth_provider.clone();
                                let stats = stats.clone();
                                let active_sessions = active_sessions.clone();

                                tokio::spawn(async move {
                                    if let Err(e) = Self::handle_connection(
                                        stream,
                                        addr,
                                        config,
                                        session_manager,
                                        auth_provider,
                                        active_sessions,
                                        stats.clone(),
                                    )
                                    .await
                                    {
                                        error!("Connection handler error: {}", e);
                                        stats.failed_logins.fetch_add(1, Ordering::Relaxed);
                                    }

                                    stats.active_connections.fetch_sub(1, Ordering::Relaxed);
                                });
                            }
                            Err(e) => {
                                error!("Failed to accept connection: {}", e);
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Login service shutdown requested");
                        break;
                    }
                }
            }

            info!("Login service stopped");
        });

        Ok(LoginServiceHandle {
            shutdown_tx,
            stats: self.stats.clone(),
        })
    }

    /// Stop the login service
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping login service");
        // Shutdown signal sent via handle
        Ok(())
    }

    /// Get connection count
    pub async fn connection_count(&self) -> u64 {
        self.stats.total_connections.load(Ordering::Relaxed)
    }

    /// Get uptime
    pub async fn uptime(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.stats.start_time)
            .unwrap_or_default()
    }

    /// Handle incoming RDP connection
    async fn handle_connection(
        stream: tokio::net::TcpStream,
        addr: SocketAddr,
        config: Arc<HeadlessConfig>,
        session_manager: Arc<SessionManager>,
        auth_provider: Arc<dyn AuthProvider>,
        active_sessions: Arc<RwLock<HashMap<String, Arc<UserSession>>>>,
        stats: Arc<LoginServiceStats>,
    ) -> Result<()> {
        info!("Handling RDP connection from {}", addr);

        // TODO: Implement RDP handshake and authentication
        // For now, this is a placeholder showing the structure

        // 1. Perform RDP protocol handshake
        let credentials = Self::rdp_handshake(&stream).await?;

        // 2. Authenticate user
        let auth_result = auth_provider
            .authenticate(&credentials.username, &credentials.password)
            .await?;

        if !auth_result.success {
            warn!("Authentication failed for user: {}", credentials.username);
            return Err(anyhow::anyhow!("Authentication failed"));
        }

        let user_info = auth_result
            .user_info
            .ok_or_else(|| anyhow::anyhow!("No user info"))?;

        info!("User {} authenticated successfully", user_info.username);
        stats.successful_logins.fetch_add(1, Ordering::Relaxed);

        // 3. Check for existing session (reconnection)
        let existing_sessions = session_manager.get_user_sessions(user_info.uid).await;

        let session = if config.multiuser.enable_reconnection && !existing_sessions.is_empty() {
            // Reuse existing session
            info!(
                "Reconnecting to existing session for user {}",
                user_info.username
            );
            existing_sessions[0].clone()
        } else {
            // Create new session
            info!("Creating new session for user {}", user_info.username);
            session_manager.create_session(user_info.clone()).await?
        };

        // 4. Store active session
        {
            let mut sessions = active_sessions.write().await;
            sessions.insert(session.session_id.clone(), session.clone());
        }

        // 5. Hand off connection to session's RDP server
        info!(
            "Handing off connection to session {} on port {}",
            session.session_id, session.rdp_port
        );

        // TODO: Proxy/redirect connection to session's RDP port
        // This would either:
        // - Proxy the TCP stream to the session's port
        // - Send redirect to client to connect to different port
        // - Keep connection and integrate with session's IronRDP server

        // For now, simulate session activity
        tokio::time::sleep(Duration::from_secs(1)).await;

        Ok(())
    }

    /// Perform RDP protocol handshake and extract credentials
    async fn rdp_handshake(_stream: &tokio::net::TcpStream) -> Result<RdpCredentials> {
        // TODO: Implement actual RDP handshake using IronRDP
        // This would:
        // 1. Parse RDP connection request
        // 2. Negotiate capabilities
        // 3. Perform TLS handshake
        // 4. Extract NLA credentials (username/password)
        // 5. Return credentials for authentication

        // Placeholder
        Ok(RdpCredentials {
            username: "test".to_string(),
            password: "test".to_string(),
            domain: None,
        })
    }
}

/// RDP credentials extracted from handshake
#[derive(Debug, Clone)]
struct RdpCredentials {
    username: String,
    password: String,
    domain: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_stats_creation() {
        let stats = LoginServiceStats::new();

        assert_eq!(stats.total_connections.load(Ordering::Relaxed), 0);
        assert_eq!(stats.successful_logins.load(Ordering::Relaxed), 0);
        assert_eq!(stats.failed_logins.load(Ordering::Relaxed), 0);
    }
}
