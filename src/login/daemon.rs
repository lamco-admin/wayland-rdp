//! WRD Login Daemon
//!
//! Main daemon process that listens for RDP connections,
//! authenticates users, and creates sessions.

use super::auth::{PamAuthenticator, AuthenticatedUser};
use super::config::LoginConfig;
use super::session::{SessionManager, SessionState};
use super::security::{SecurityManager, ResourceLimits};
use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// WRD Login Daemon
pub struct WrdLoginDaemon {
    /// Configuration
    config: Arc<LoginConfig>,

    /// PAM authenticator
    pam: Arc<PamAuthenticator>,

    /// Session manager
    sessions: Arc<SessionManager>,

    /// Security manager
    security: Arc<Mutex<SecurityManager>>,

    /// Active connections count
    active_connections: Arc<Mutex<u32>>,
}

impl WrdLoginDaemon {
    /// Create new login daemon
    pub async fn new(config: LoginConfig) -> Result<Self> {
        info!("Initializing WRD Login Daemon");

        let config = Arc::new(config);

        // Create PAM authenticator
        let pam = Arc::new(PamAuthenticator::new(
            config.paths.pam_service.clone(),
        ));

        // Create session manager
        let sessions = Arc::new(SessionManager::new(Arc::clone(&config))?);

        // Create security manager
        let limits = ResourceLimits::new(
            config.limits.max_memory_mb,
            config.limits.cpu_shares,
            config.limits.max_processes,
            config.limits.max_open_files,
        );

        let mut security = SecurityManager::new(
            limits,
            config.security.max_failed_attempts,
            config.security.lockout_duration,
            config.security.enable_lockout,
        );

        // Set audit log path
        if config.security.audit_logging {
            let audit_log = config.paths.log_dir.join("audit.log");
            security.set_audit_log(audit_log);
        }

        let security = Arc::new(Mutex::new(security));

        Ok(Self {
            config,
            pam,
            sessions,
            security,
            active_connections: Arc::new(Mutex::new(0)),
        })
    }

    /// Run the login daemon
    pub async fn run(&self) -> Result<()> {
        info!("╔════════════════════════════════════════════════════════════╗");
        info!("║          WRD Login Service Starting                        ║");
        info!("╚════════════════════════════════════════════════════════════╝");
        info!("  Bind Address: {}", self.config.network.bind_address);
        info!("  Port: {}", self.config.network.port);
        info!("  Max Connections: {}", self.config.network.max_connections);
        info!("  PAM Service: {}", self.config.paths.pam_service);
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        // Bind to address
        let addr: SocketAddr = format!("{}:{}",
            self.config.network.bind_address,
            self.config.network.port
        ).parse()?;

        let listener = TcpListener::bind(addr).await
            .context("Failed to bind to address")?;

        info!("Login service listening on {}", addr);

        // Accept connections
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!("Incoming connection from: {}", peer_addr);

                    // Check connection limit
                    let current_connections = *self.active_connections.lock().await;
                    if current_connections >= self.config.network.max_connections {
                        warn!("Rejecting connection from {} - connection limit reached", peer_addr);
                        continue;
                    }

                    // Increment connection count
                    *self.active_connections.lock().await += 1;

                    // Spawn task to handle connection
                    let daemon = self.clone();
                    let active_connections = Arc::clone(&self.active_connections);

                    tokio::spawn(async move {
                        if let Err(e) = daemon.handle_connection(stream, peer_addr).await {
                            error!("Connection error from {}: {}", peer_addr, e);
                        }

                        // Decrement connection count
                        *active_connections.lock().await -= 1;
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    /// Handle a single RDP connection
    async fn handle_connection(&self, stream: TcpStream, peer_addr: SocketAddr) -> Result<()> {
        debug!("Handling connection from: {}", peer_addr);

        // TODO: TLS handshake
        // TODO: RDP protocol negotiation
        // TODO: Present login screen

        // For now, simulate receiving credentials
        // In real implementation, this would come from RDP protocol
        let (username, password) = self.receive_credentials(stream).await?;

        info!("Login attempt for user: {}", username);

        // Check if login is allowed (account not locked)
        self.security.lock().await.check_login_allowed(&username)?;

        // Authenticate user
        let user = self.authenticate_user(&username, &password).await?;

        // Record successful authentication
        self.security.lock().await.record_successful_login(&username)?;

        info!("User {} authenticated successfully", username);

        // Create session
        let session_id = self.sessions.create_session(user.clone()).await?;

        info!("Session created for {}: {}", username, session_id);

        // Apply resource limits
        if let Some(session) = self.sessions.get_session(&session_id) {
            if let Some(pid) = session.compositor_pid {
                let security = self.security.lock().await;
                if let Err(e) = security.apply_resource_limits(pid, user.uid) {
                    warn!("Failed to apply resource limits: {}", e);
                }
            }
        }

        // TODO: Transfer RDP connection to user's compositor

        info!("Connection established for user {}", username);

        Ok(())
    }

    /// Receive credentials from RDP client
    async fn receive_credentials(&self, _stream: TcpStream) -> Result<(String, String)> {
        // TODO: Implement RDP protocol credential reception
        // This is a placeholder that would need to integrate with ironrdp

        // For now, return dummy credentials for testing
        anyhow::bail!("Credential reception not yet implemented");
    }

    /// Authenticate user
    async fn authenticate_user(&self, username: &str, password: &str) -> Result<AuthenticatedUser> {
        // Validate password strength if required
        if self.config.security.require_strong_passwords {
            if let Err(e) = self.pam.validate_password_strength(password) {
                warn!("Weak password rejected for user {}: {}", username, e);

                // Record failed attempt
                self.security.lock().await.record_failed_login(username)?;

                return Err(e);
            }
        }

        // Perform PAM authentication (blocking operation)
        let pam = Arc::clone(&self.pam);
        let username = username.to_string();
        let password = password.to_string();

        let result = tokio::task::spawn_blocking(move || {
            pam.authenticate(&username, &password)
        }).await;

        match result {
            Ok(Ok(user)) => Ok(user),
            Ok(Err(e)) => {
                // Record failed attempt
                self.security.lock().await.record_failed_login(&username)?;
                Err(e)
            }
            Err(e) => {
                Err(anyhow::anyhow!("Authentication task failed: {}", e))
            }
        }
    }

    /// Terminate all sessions
    pub async fn terminate_all_sessions(&self) -> Result<()> {
        info!("Terminating all sessions");

        let session_ids = self.sessions.list_sessions();

        for session_id in session_ids {
            if let Err(e) = self.sessions.terminate_session(&session_id).await {
                error!("Failed to terminate session {}: {}", session_id, e);
            }
        }

        Ok(())
    }
}

impl Clone for WrdLoginDaemon {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            pam: Arc::clone(&self.pam),
            sessions: Arc::clone(&self.sessions),
            security: Arc::clone(&self.security),
            active_connections: Arc::clone(&self.active_connections),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_daemon_creation() {
        let config = LoginConfig::default();

        // This might fail without proper setup (certs, etc.)
        // but should not panic
        let _result = WrdLoginDaemon::new(config).await;
    }
}
