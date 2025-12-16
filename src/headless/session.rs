//! Session management for headless multi-user RDP server
//!
//! Manages per-user compositor instances, session lifecycle with systemd-logind,
//! resource allocation, and session persistence/reconnection.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Child;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::headless::auth::UserInfo;
use crate::headless::compositor::HeadlessCompositor;
use crate::headless::config::HeadlessConfig;
use crate::headless::resources::ResourceManager;

/// Unique session identifier
pub type SessionId = String;

/// Session manager coordinates all user sessions
pub struct SessionManager {
    config: Arc<HeadlessConfig>,
    resource_manager: Arc<ResourceManager>,
    sessions: Arc<RwLock<HashMap<SessionId, Arc<UserSession>>>>,
    user_sessions: Arc<RwLock<HashMap<u32, Vec<SessionId>>>>, // UID -> SessionIDs
    port_allocator: Arc<RwLock<PortAllocator>>,
}

/// User session information
pub struct UserSession {
    pub session_id: SessionId,
    pub user_info: UserInfo,
    pub compositor: Arc<HeadlessCompositor>,
    pub state: Arc<RwLock<SessionState>>,
    pub created_at: SystemTime,
    pub last_activity: Arc<RwLock<SystemTime>>,
    pub rdp_port: u16,
    pub wayland_display: String,
    pub pipewire_fd: Option<i32>,
    pub environment: HashMap<String, String>,
    pub logind_session_id: Option<String>,
}

/// Session state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    /// Session is being created
    Creating,

    /// Session is active with connected client
    Active,

    /// Session is idle (no client connected)
    Idle,

    /// Session is suspended (client disconnected, waiting for reconnection)
    Suspended,

    /// Session is being terminated
    Terminating,

    /// Session has been terminated
    Terminated,
}

/// Port allocator for RDP ports
struct PortAllocator {
    base_port: u16,
    port_range: u16,
    allocated_ports: HashMap<u16, SessionId>,
}

impl PortAllocator {
    fn new(base_port: u16, port_range: u16) -> Self {
        Self {
            base_port,
            port_range,
            allocated_ports: HashMap::new(),
        }
    }

    fn allocate(&mut self, session_id: SessionId) -> Result<u16> {
        for offset in 0..self.port_range {
            let port = self.base_port + offset;

            if !self.allocated_ports.contains_key(&port) {
                self.allocated_ports.insert(port, session_id);
                return Ok(port);
            }
        }

        anyhow::bail!("No available ports in range {}-{}", self.base_port, self.base_port + self.port_range)
    }

    fn release(&mut self, port: u16) {
        self.allocated_ports.remove(&port);
    }
}

impl SessionManager {
    /// Create new session manager
    pub async fn new(
        config: Arc<HeadlessConfig>,
        resource_manager: Arc<ResourceManager>,
    ) -> Result<Self> {
        info!("Initializing session manager");

        let port_allocator = PortAllocator::new(
            config.multiuser.base_port,
            config.multiuser.port_range,
        );

        // Create session persistence directory if needed
        if config.multiuser.enable_persistence {
            tokio::fs::create_dir_all(&config.multiuser.persistence_dir)
                .await
                .context("Failed to create session persistence directory")?;
        }

        Ok(Self {
            config,
            resource_manager,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            user_sessions: Arc::new(RwLock::new(HashMap::new())),
            port_allocator: Arc::new(RwLock::new(port_allocator)),
        })
    }

    /// Create a new user session
    pub async fn create_session(&self, user_info: UserInfo) -> Result<Arc<UserSession>> {
        info!("Creating session for user: {}", user_info.username);

        // Check session limits
        self.check_session_limits(&user_info).await?;

        // Generate session ID
        let session_id = format!("session-{}-{}", user_info.username, uuid::Uuid::new_v4());

        // Allocate RDP port
        let rdp_port = self
            .port_allocator
            .write()
            .await
            .allocate(session_id.clone())?;

        info!("Allocated port {} for session {}", rdp_port, session_id);

        // Create systemd-logind session if enabled
        let logind_session_id = if self.config.session.use_systemd_logind {
            Some(self.create_logind_session(&user_info).await?)
        } else {
            None
        };

        // Set up session environment
        let environment = self.setup_environment(&user_info, &session_id).await?;

        // Determine Wayland display name
        let wayland_display = format!("wayland-{}", rdp_port - self.config.multiuser.base_port);

        // Create headless compositor for this user
        let compositor = Arc::new(
            HeadlessCompositor::new(
                self.config.clone(),
                &user_info,
                &wayland_display,
                environment.clone(),
            )
            .await?,
        );

        info!("Created headless compositor for session {}", session_id);

        // Start compositor
        compositor.start().await?;

        // Apply resource limits via cgroups
        self.resource_manager
            .apply_session_limits(&session_id, user_info.uid)
            .await?;

        // Create session object
        let session = Arc::new(UserSession {
            session_id: session_id.clone(),
            user_info: user_info.clone(),
            compositor,
            state: Arc::new(RwLock::new(SessionState::Creating)),
            created_at: SystemTime::now(),
            last_activity: Arc::new(RwLock::new(SystemTime::now())),
            rdp_port,
            wayland_display,
            pipewire_fd: None, // Will be set when portal session is created
            environment,
            logind_session_id,
        });

        // Update session state
        *session.state.write().await = SessionState::Idle;

        // Store session
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session.clone());
        }

        // Track per-user sessions
        {
            let mut user_sessions = self.user_sessions.write().await;
            user_sessions
                .entry(user_info.uid)
                .or_insert_with(Vec::new)
                .push(session_id.clone());
        }

        // Persist session if enabled
        if self.config.multiuser.enable_persistence {
            self.persist_session(&session).await?;
        }

        info!("Session created successfully: {}", session_id);
        info!("  User: {}", user_info.username);
        info!("  Port: {}", rdp_port);
        info!("  Wayland display: {}", wayland_display);

        Ok(session)
    }

    /// Get existing session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<Arc<UserSession>> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Get all sessions for a user
    pub async fn get_user_sessions(&self, uid: u32) -> Vec<Arc<UserSession>> {
        let user_sessions = self.user_sessions.read().await;
        let sessions = self.sessions.read().await;

        user_sessions
            .get(&uid)
            .map(|session_ids| {
                session_ids
                    .iter()
                    .filter_map(|id| sessions.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Terminate a session
    pub async fn terminate_session(&self, session_id: &str) -> Result<()> {
        info!("Terminating session: {}", session_id);

        let session = {
            let sessions = self.sessions.read().await;
            sessions
                .get(session_id)
                .ok_or_else(|| anyhow::anyhow!("Session not found"))?
                .clone()
        };

        // Update state
        *session.state.write().await = SessionState::Terminating;

        // Stop compositor
        session.compositor.stop().await?;

        // Release resources
        self.resource_manager
            .release_session_resources(session_id)
            .await?;

        // Close logind session if exists
        if let Some(ref logind_id) = session.logind_session_id {
            self.close_logind_session(logind_id).await?;
        }

        // Release port
        self.port_allocator.write().await.release(session.rdp_port);

        // Remove from tracking
        {
            let mut sessions = self.sessions.write().await;
            sessions.remove(session_id);
        }

        {
            let mut user_sessions = self.user_sessions.write().await;
            if let Some(session_ids) = user_sessions.get_mut(&session.user_info.uid) {
                session_ids.retain(|id| id != session_id);

                if session_ids.is_empty() {
                    user_sessions.remove(&session.user_info.uid);
                }
            }
        }

        // Remove persistence
        if self.config.multiuser.enable_persistence {
            self.remove_persisted_session(session_id).await?;
        }

        // Update final state
        *session.state.write().await = SessionState::Terminated;

        info!("Session terminated: {}", session_id);
        Ok(())
    }

    /// Terminate all sessions
    pub async fn terminate_all_sessions(&self) -> Result<()> {
        info!("Terminating all sessions");

        let session_ids: Vec<String> = {
            let sessions = self.sessions.read().await;
            sessions.keys().cloned().collect()
        };

        for session_id in session_ids {
            if let Err(e) = self.terminate_session(&session_id).await {
                error!("Failed to terminate session {}: {}", session_id, e);
            }
        }

        info!("All sessions terminated");
        Ok(())
    }

    /// Get session count
    pub async fn session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Check if new session creation is allowed
    async fn check_session_limits(&self, user_info: &UserInfo) -> Result<()> {
        let current_count = self.session_count().await;

        // Check system-wide limit
        if self.config.multiuser.max_sessions > 0
            && current_count >= self.config.multiuser.max_sessions
        {
            anyhow::bail!(
                "Maximum session limit reached ({}/{})",
                current_count,
                self.config.multiuser.max_sessions
            );
        }

        // Check per-user limit
        let user_session_count = {
            let user_sessions = self.user_sessions.read().await;
            user_sessions
                .get(&user_info.uid)
                .map(|sessions| sessions.len())
                .unwrap_or(0)
        };

        if self.config.multiuser.max_sessions_per_user > 0
            && user_session_count >= self.config.multiuser.max_sessions_per_user
        {
            anyhow::bail!(
                "Maximum sessions per user limit reached ({}/{})",
                user_session_count,
                self.config.multiuser.max_sessions_per_user
            );
        }

        Ok(())
    }

    /// Create systemd-logind session
    #[cfg(feature = "systemd-integration")]
    async fn create_logind_session(&self, user_info: &UserInfo) -> Result<String> {
        use std::process::Command;

        info!("Creating systemd-logind session for user: {}", user_info.username);

        // Use loginctl to create a session
        let output = Command::new("loginctl")
            .args(&[
                "create-session",
                &user_info.username,
                &user_info.uid.to_string(),
                "wayland",
            ])
            .output()
            .context("Failed to create logind session")?;

        if !output.status.success() {
            anyhow::bail!("loginctl failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        let session_id = String::from_utf8(output.stdout)?
            .trim()
            .to_string();

        info!("Created logind session: {}", session_id);
        Ok(session_id)
    }

    #[cfg(not(feature = "systemd-integration"))]
    async fn create_logind_session(&self, user_info: &UserInfo) -> Result<String> {
        // Fallback when systemd integration is not available
        debug!("systemd integration not available, using mock session ID");
        Ok(format!("mock-session-{}", user_info.uid))
    }

    /// Close systemd-logind session
    #[cfg(feature = "systemd-integration")]
    async fn close_logind_session(&self, session_id: &str) -> Result<()> {
        use std::process::Command;

        info!("Closing systemd-logind session: {}", session_id);

        let output = Command::new("loginctl")
            .args(&["terminate-session", session_id])
            .output()
            .context("Failed to terminate logind session")?;

        if !output.status.success() {
            warn!("loginctl terminate failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }

    #[cfg(not(feature = "systemd-integration"))]
    async fn close_logind_session(&self, session_id: &str) -> Result<()> {
        debug!("Mock logind session closed: {}", session_id);
        Ok(())
    }

    /// Set up environment variables for session
    async fn setup_environment(
        &self,
        user_info: &UserInfo,
        session_id: &str,
    ) -> Result<HashMap<String, String>> {
        let mut env = HashMap::new();

        // Standard environment
        env.insert("HOME".to_string(), user_info.home_dir.clone());
        env.insert("USER".to_string(), user_info.username.clone());
        env.insert("LOGNAME".to_string(), user_info.username.clone());
        env.insert("SHELL".to_string(), user_info.shell.clone());

        // XDG directories
        let runtime_dir = format!("/run/user/{}", user_info.uid);
        env.insert("XDG_RUNTIME_DIR".to_string(), runtime_dir.clone());
        env.insert("XDG_SESSION_ID".to_string(), session_id.to_string());
        env.insert("XDG_SESSION_TYPE".to_string(), "wayland".to_string());
        env.insert("XDG_SESSION_CLASS".to_string(), self.config.session.session_class.clone());

        // Wayland
        env.insert("WAYLAND_DISPLAY".to_string(), format!("wayland-{}", session_id));

        // Session info
        env.insert("WRD_SESSION_ID".to_string(), session_id.to_string());

        // Add custom environment from config
        for (key, value) in &self.config.autostart.environment {
            env.insert(key.clone(), value.clone());
        }

        Ok(env)
    }

    /// Persist session information to disk
    async fn persist_session(&self, session: &UserSession) -> Result<()> {
        let session_file = self
            .config
            .multiuser
            .persistence_dir
            .join(format!("{}.json", session.session_id));

        let session_info = SessionInfo::from_session(session);
        let json = serde_json::to_string_pretty(&session_info)?;

        tokio::fs::write(session_file, json)
            .await
            .context("Failed to persist session")?;

        Ok(())
    }

    /// Remove persisted session
    async fn remove_persisted_session(&self, session_id: &str) -> Result<()> {
        let session_file = self
            .config
            .multiuser
            .persistence_dir
            .join(format!("{}.json", session_id));

        if session_file.exists() {
            tokio::fs::remove_file(session_file)
                .await
                .context("Failed to remove persisted session")?;
        }

        Ok(())
    }

    /// Monitor sessions for idle timeout and cleanup
    pub async fn start_session_monitor(&self) -> Result<()> {
        let sessions = self.sessions.clone();
        let config = self.config.clone();
        let manager = Arc::new(self);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            loop {
                interval.tick().await;

                let session_ids: Vec<String> = {
                    let sessions = sessions.read().await;
                    sessions.keys().cloned().collect()
                };

                for session_id in session_ids {
                    let session = {
                        let sessions = sessions.read().await;
                        sessions.get(&session_id).cloned()
                    };

                    if let Some(session) = session {
                        // Check idle timeout
                        if config.multiuser.idle_timeout > 0 {
                            let last_activity = *session.last_activity.read().await;
                            let idle_duration = SystemTime::now()
                                .duration_since(last_activity)
                                .unwrap_or_default();

                            if idle_duration > Duration::from_secs(config.multiuser.idle_timeout) {
                                warn!("Session {} idle timeout, terminating", session_id);
                                if let Err(e) = manager.terminate_session(&session_id).await {
                                    error!("Failed to terminate idle session: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }
}

/// Serializable session information for persistence
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub username: String,
    pub uid: u32,
    pub created_at: u64,
    pub rdp_port: u16,
    pub wayland_display: String,
}

impl SessionInfo {
    fn from_session(session: &UserSession) -> Self {
        Self {
            session_id: session.session_id.clone(),
            username: session.user_info.username.clone(),
            uid: session.user_info.uid,
            created_at: session
                .created_at
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            rdp_port: session.rdp_port,
            wayland_display: session.wayland_display.clone(),
        }
    }
}
