//! Session management with systemd-logind integration
//!
//! Manages user sessions, compositor instances, and lifecycle.

use super::auth::AuthenticatedUser;
use super::config::LoginConfig;
use super::logind::LogindClient;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Arc;
use parking_lot::Mutex;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Session is being created
    Creating,

    /// Session is active
    Active,

    /// Session is suspended
    Suspended,

    /// Session is terminating
    Terminating,

    /// Session has terminated
    Terminated,
}

/// User session
#[derive(Debug)]
pub struct UserSession {
    /// Session ID
    pub id: String,

    /// User information
    pub user: AuthenticatedUser,

    /// systemd-logind session ID
    pub logind_session_id: Option<String>,

    /// Compositor process
    pub compositor_process: Option<Child>,

    /// Compositor PID
    pub compositor_pid: Option<u32>,

    /// Wayland socket path
    pub wayland_socket: PathBuf,

    /// Runtime directory
    pub runtime_dir: PathBuf,

    /// Session state
    pub state: SessionState,

    /// Creation timestamp
    pub created_at: std::time::SystemTime,

    /// Last activity timestamp
    pub last_activity: std::time::SystemTime,
}

impl UserSession {
    /// Create new user session
    pub fn new(user: AuthenticatedUser) -> Self {
        let id = Uuid::new_v4().to_string();
        let runtime_dir = user.runtime_dir();
        let wayland_socket = runtime_dir.join(&user.wayland_display());

        Self {
            id,
            user,
            logind_session_id: None,
            compositor_process: None,
            compositor_pid: None,
            wayland_socket,
            runtime_dir,
            state: SessionState::Creating,
            created_at: std::time::SystemTime::now(),
            last_activity: std::time::SystemTime::now(),
        }
    }

    /// Update last activity time
    pub fn touch(&mut self) {
        self.last_activity = std::time::SystemTime::now();
    }

    /// Get session age in seconds
    pub fn age_seconds(&self) -> u64 {
        self.created_at
            .elapsed()
            .unwrap_or_default()
            .as_secs()
    }

    /// Get idle time in seconds
    pub fn idle_seconds(&self) -> u64 {
        self.last_activity
            .elapsed()
            .unwrap_or_default()
            .as_secs()
    }

    /// Check if session is active
    pub fn is_active(&self) -> bool {
        self.state == SessionState::Active
    }

    /// Terminate compositor process
    pub fn terminate_compositor(&mut self) -> Result<()> {
        if let Some(mut child) = self.compositor_process.take() {
            info!("Terminating compositor for user {} (PID: {:?})",
                self.user.username, child.id());

            // Send SIGTERM
            child.kill()
                .context("Failed to kill compositor process")?;

            // Wait for process to exit
            match child.wait_timeout(std::time::Duration::from_secs(5)) {
                Ok(Some(status)) => {
                    debug!("Compositor exited with status: {}", status);
                }
                Ok(None) => {
                    warn!("Compositor did not exit within 5 seconds, may still be running");
                }
                Err(e) => {
                    error!("Error waiting for compositor to exit: {}", e);
                }
            }

            self.compositor_pid = None;
        }

        Ok(())
    }
}

/// Session manager
pub struct SessionManager {
    /// Active sessions by session ID
    sessions: Arc<Mutex<HashMap<String, UserSession>>>,

    /// Sessions by UID
    sessions_by_uid: Arc<Mutex<HashMap<u32, String>>>,

    /// Configuration
    config: Arc<LoginConfig>,

    /// systemd-logind client
    logind_client: Option<Arc<tokio::sync::Mutex<LogindClient>>>,
}

impl SessionManager {
    /// Create new session manager
    pub async fn new(config: Arc<LoginConfig>) -> Result<Self> {
        info!("Initializing session manager");

        // Create systemd-logind client
        let logind_client = match LogindClient::new().await {
            Ok(client) => Some(Arc::new(tokio::sync::Mutex::new(client))),
            Err(e) => {
                warn!("Failed to connect to systemd-logind: {} - session tracking limited", e);
                None
            }
        };

        Ok(Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            sessions_by_uid: Arc::new(Mutex::new(HashMap::new())),
            config,
            logind_client,
        })
    }

    /// Create a new user session
    pub async fn create_session(&self, user: AuthenticatedUser) -> Result<String> {
        info!("Creating session for user: {} (UID: {})", user.username, user.uid);

        // Check if user already has a session
        {
            let sessions_by_uid = self.sessions_by_uid.lock();
            if let Some(existing_session_id) = sessions_by_uid.get(&user.uid) {
                info!("User {} already has an active session: {}",
                    user.username, existing_session_id);
                return Ok(existing_session_id.clone());
            }
        }

        let mut session = UserSession::new(user.clone());

        // Create systemd-logind session
        if let Some(logind) = &self.logind_client {
            match logind.lock().await.create_session(&user).await {
                Ok(session_id) => {
                    info!("Created systemd-logind session: {}", session_id);
                    session.logind_session_id = Some(session_id);
                }
                Err(e) => {
                    warn!("Failed to create systemd-logind session: {}", e);
                }
            }
        }

        // Create runtime directory
        self.create_runtime_directory(&session)?;

        // Spawn compositor
        self.spawn_compositor(&mut session)?;

        // Wait for compositor to be ready
        self.wait_for_compositor(&session).await?;

        // Update session state
        session.state = SessionState::Active;

        let session_id = session.id.clone();
        let uid = session.user.uid;

        // Store session
        {
            let mut sessions = self.sessions.lock();
            sessions.insert(session_id.clone(), session);
        }

        {
            let mut sessions_by_uid = self.sessions_by_uid.lock();
            sessions_by_uid.insert(uid, session_id.clone());
        }

        info!("Session created successfully: {}", session_id);

        Ok(session_id)
    }

    /// Terminate a session
    pub async fn terminate_session(&self, session_id: &str) -> Result<()> {
        info!("Terminating session: {}", session_id);

        let mut session = {
            let mut sessions = self.sessions.lock();
            sessions.remove(session_id)
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?
        };

        // Remove from UID map
        {
            let mut sessions_by_uid = self.sessions_by_uid.lock();
            sessions_by_uid.remove(&session.user.uid);
        }

        // Update state
        session.state = SessionState::Terminating;

        // Terminate compositor
        session.terminate_compositor()?;

        // Terminate systemd-logind session
        if let Some(logind) = &self.logind_client {
            if let Some(logind_session_id) = &session.logind_session_id {
                logind.lock().await.terminate_session(logind_session_id).await?;
            }
        }

        // Cleanup runtime directory
        self.cleanup_runtime_directory(&session)?;

        session.state = SessionState::Terminated;

        info!("Session terminated: {}", session_id);

        Ok(())
    }

    /// Get session by ID
    pub fn get_session(&self, session_id: &str) -> Option<UserSession> {
        let sessions = self.sessions.lock();
        // Clone the session for safe access
        sessions.get(session_id).map(|s| {
            // Create a lightweight clone without the Child process
            UserSession {
                id: s.id.clone(),
                user: s.user.clone(),
                logind_session_id: s.logind_session_id.clone(),
                compositor_process: None,
                compositor_pid: s.compositor_pid,
                wayland_socket: s.wayland_socket.clone(),
                runtime_dir: s.runtime_dir.clone(),
                state: s.state,
                created_at: s.created_at,
                last_activity: s.last_activity,
            }
        })
    }

    /// List all active sessions
    pub fn list_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.lock();
        sessions.keys().cloned().collect()
    }

    /// Create runtime directory for user
    fn create_runtime_directory(&self, session: &UserSession) -> Result<()> {
        info!("Creating runtime directory: {:?}", session.runtime_dir);

        // Create directory
        std::fs::create_dir_all(&session.runtime_dir)
            .context("Failed to create runtime directory")?;

        // Set ownership
        Self::chown_recursive(&session.runtime_dir, session.user.uid, session.user.gid)?;

        // Set permissions (0700)
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(&session.runtime_dir, permissions)
            .context("Failed to set permissions on runtime directory")?;

        Ok(())
    }

    /// Cleanup runtime directory
    fn cleanup_runtime_directory(&self, session: &UserSession) -> Result<()> {
        info!("Cleaning up runtime directory: {:?}", session.runtime_dir);

        if session.runtime_dir.exists() {
            std::fs::remove_dir_all(&session.runtime_dir)
                .context("Failed to remove runtime directory")?;
        }

        Ok(())
    }

    /// Change ownership recursively
    fn chown_recursive(path: &PathBuf, uid: u32, gid: u32) -> Result<()> {
        use nix::unistd::{Uid, Gid, chown};

        let uid = Uid::from_raw(uid);
        let gid = Gid::from_raw(gid);

        chown(path, Some(uid), Some(gid))
            .context("Failed to chown path")?;

        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                Self::chown_recursive(&entry.path(), uid.as_raw(), gid.as_raw())?;
            }
        }

        Ok(())
    }

    /// Spawn compositor for session
    fn spawn_compositor(&self, session: &mut UserSession) -> Result<()> {
        info!("Spawning compositor for user: {}", session.user.username);

        // Prepare environment
        let mut cmd = Command::new(&self.config.paths.compositor_path);

        cmd.env("XDG_RUNTIME_DIR", &session.runtime_dir)
            .env("WAYLAND_DISPLAY", session.user.wayland_display())
            .env("HOME", &session.user.home)
            .env("USER", &session.user.username)
            .env("LOGNAME", &session.user.username)
            .env("SHELL", &session.user.shell);

        // Set UID/GID
        cmd.uid(session.user.uid)
            .gid(session.user.gid);

        // Spawn process
        let child = cmd.spawn()
            .context("Failed to spawn compositor process")?;

        let pid = child.id();
        info!("Compositor spawned with PID: {}", pid);

        session.compositor_process = Some(child);
        session.compositor_pid = Some(pid);

        Ok(())
    }

    /// Wait for compositor to be ready
    async fn wait_for_compositor(&self, session: &UserSession) -> Result<()> {
        info!("Waiting for compositor to be ready...");

        let socket_path = &session.wayland_socket;
        let timeout = std::time::Duration::from_secs(10);
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            if socket_path.exists() {
                info!("Compositor ready: socket exists at {:?}", socket_path);
                return Ok(());
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        anyhow::bail!("Timeout waiting for compositor to create socket");
    }
}

// systemd-logind client is now in its own module (logind.rs)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let user = AuthenticatedUser {
            username: "testuser".to_string(),
            uid: 1000,
            gid: 1000,
            home: PathBuf::from("/home/testuser"),
            shell: PathBuf::from("/bin/bash"),
            gecos: "Test User".to_string(),
        };

        let session = UserSession::new(user);

        assert_eq!(session.state, SessionState::Creating);
        assert!(session.compositor_pid.is_none());
        assert_eq!(session.runtime_dir, PathBuf::from("/run/user/1000"));
    }

    #[test]
    fn test_session_state_transitions() {
        let user = AuthenticatedUser {
            username: "testuser".to_string(),
            uid: 1000,
            gid: 1000,
            home: PathBuf::from("/home/testuser"),
            shell: PathBuf::from("/bin/bash"),
            gecos: "Test User".to_string(),
        };

        let mut session = UserSession::new(user);

        assert_eq!(session.state, SessionState::Creating);

        session.state = SessionState::Active;
        assert!(session.is_active());

        session.state = SessionState::Suspended;
        assert!(!session.is_active());
    }
}
