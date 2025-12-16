//! systemd-logind D-Bus client implementation
//!
//! Provides complete integration with systemd-logind for session management.

use super::auth::AuthenticatedUser;
use anyhow::{Context, Result};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use zbus::{Connection, proxy};

/// systemd-logind Manager interface
#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait LoginManager {
    /// Create a new session
    #[zbus(name = "CreateSession")]
    fn create_session(
        &self,
        uid: u32,
        pid: u32,
        service: &str,
        type_: &str,
        class: &str,
        desktop: &str,
        seat_id: &str,
        vtnr: u32,
        tty: &str,
        display: &str,
        remote: bool,
        remote_user: &str,
        remote_host: &str,
    ) -> zbus::Result<(String, zbus::zvariant::OwnedObjectPath, String, zbus::zvariant::OwnedFd, u32, String, u32, bool)>;

    /// Terminate a session
    #[zbus(name = "TerminateSession")]
    fn terminate_session(&self, session_id: &str) -> zbus::Result<()>;

    /// Get session by PID
    #[zbus(name = "GetSessionByPID")]
    fn get_session_by_pid(&self, pid: u32) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// List sessions
    #[zbus(name = "ListSessions")]
    fn list_sessions(&self) -> zbus::Result<Vec<(String, u32, String, String, zbus::zvariant::OwnedObjectPath)>>;

    /// Kill session
    #[zbus(name = "KillSession")]
    fn kill_session(&self, session_id: &str, who: &str, signal: i32) -> zbus::Result<()>;
}

/// systemd-logind Session interface
#[proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1"
)]
trait LoginSession {
    /// Terminate the session
    #[zbus(name = "Terminate")]
    fn terminate(&self) -> zbus::Result<()>;

    /// Activate the session
    #[zbus(name = "Activate")]
    fn activate(&self) -> zbus::Result<()>;

    /// Lock the session
    #[zbus(name = "Lock")]
    fn lock(&self) -> zbus::Result<()>;

    /// Unlock the session
    #[zbus(name = "Unlock")]
    fn unlock(&self) -> zbus::Result<()>;

    /// Kill the session
    #[zbus(name = "Kill")]
    fn kill(&self, who: &str, signal: i32) -> zbus::Result<()>;

    /// Get session properties
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn user(&self) -> zbus::Result<(u32, zbus::zvariant::OwnedObjectPath)>;

    #[zbus(property)]
    fn name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn timestamp(&self) -> zbus::Result<u64>;

    #[zbus(property)]
    fn remote_user(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn remote_host(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn service(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn type_(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn class(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn desktop(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn state(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn idle_since_hint(&self) -> zbus::Result<u64>;

    #[zbus(property)]
    fn idle_since_hint_monotonic(&self) -> zbus::Result<u64>;

    #[zbus(property)]
    fn locked_hint(&self) -> zbus::Result<bool>;
}

/// systemd-logind client
pub struct LogindClient {
    /// D-Bus connection
    connection: Connection,

    /// Manager proxy
    manager_proxy: LoginManagerProxy<'static>,

    /// Active sessions (session_id -> session_path)
    active_sessions: HashMap<String, String>,
}

impl LogindClient {
    /// Create new logind client
    pub async fn new() -> Result<Self> {
        info!("Connecting to systemd-logind via D-Bus");

        // Connect to system bus
        let connection = Connection::system()
            .await
            .context("Failed to connect to system D-Bus")?;

        // Create manager proxy
        let manager_proxy = LoginManagerProxy::new(&connection)
            .await
            .context("Failed to create logind manager proxy")?;

        debug!("Successfully connected to systemd-logind");

        Ok(Self {
            connection,
            manager_proxy,
            active_sessions: HashMap::new(),
        })
    }

    /// Create a new systemd-logind session
    pub async fn create_session(&mut self, user: &AuthenticatedUser) -> Result<String> {
        info!("Creating systemd-logind session for user: {} (UID: {})",
            user.username, user.uid);

        // Call CreateSession D-Bus method
        let result = self.manager_proxy.create_session(
            user.uid,                    // uid
            0,                           // pid (0 = calling process)
            &user.username,              // service
            "wayland",                   // type
            "user",                      // class
            "wrd-compositor",            // desktop
            "",                          // seat_id (empty for headless)
            0,                           // vtnr (0 for headless)
            "",                          // tty (empty for Wayland)
            "",                          // display (empty for Wayland)
            true,                        // remote (true for RDP)
            "",                          // remote_user (empty for now)
            "",                          // remote_host (empty for now)
        ).await
            .context("Failed to call CreateSession D-Bus method")?;

        let session_id = result.0;
        let session_path = result.1.to_string();

        info!("Created logind session: {} (path: {})", session_id, session_path);

        // Store session
        self.active_sessions.insert(session_id.clone(), session_path);

        Ok(session_id)
    }

    /// Terminate a session
    pub async fn terminate_session(&mut self, session_id: &str) -> Result<()> {
        info!("Terminating systemd-logind session: {}", session_id);

        // Call TerminateSession D-Bus method
        self.manager_proxy.terminate_session(session_id)
            .await
            .context("Failed to terminate session")?;

        // Remove from tracking
        self.active_sessions.remove(session_id);

        debug!("Session terminated: {}", session_id);

        Ok(())
    }

    /// Get session by PID
    pub async fn get_session_by_pid(&self, pid: u32) -> Result<String> {
        debug!("Looking up session for PID: {}", pid);

        let session_path = self.manager_proxy.get_session_by_pid(pid)
            .await
            .context("Failed to get session by PID")?;

        // Extract session ID from path (e.g., /org/freedesktop/login1/session/c1 -> c1)
        let session_id = session_path.as_str()
            .rsplit('/')
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid session path"))?
            .to_string();

        Ok(session_id)
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        debug!("Listing all sessions");

        let sessions = self.manager_proxy.list_sessions()
            .await
            .context("Failed to list sessions")?;

        let session_infos: Vec<SessionInfo> = sessions.into_iter()
            .map(|(id, uid, user, seat, path)| SessionInfo {
                id,
                uid,
                user,
                seat,
                path: path.to_string(),
            })
            .collect();

        debug!("Found {} sessions", session_infos.len());

        Ok(session_infos)
    }

    /// Get session properties
    pub async fn get_session_properties(&self, session_id: &str) -> Result<SessionProperties> {
        debug!("Getting properties for session: {}", session_id);

        let session_path = self.active_sessions.get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        // Create session proxy
        let session_proxy = LoginSessionProxyBuilder::new(&self.connection)
            .path(session_path)?
            .build()
            .await
            .context("Failed to create session proxy")?;

        // Get properties
        let id = session_proxy.id().await?;
        let user = session_proxy.user().await?;
        let name = session_proxy.name().await?;
        let service = session_proxy.service().await?;
        let type_ = session_proxy.type_().await?;
        let class = session_proxy.class().await?;
        let state = session_proxy.state().await?;

        let props = SessionProperties {
            id,
            uid: user.0,
            name,
            service,
            session_type: type_,
            session_class: class,
            state,
        };

        debug!("Session properties: {:?}", props);

        Ok(props)
    }

    /// Kill a session (forcefully)
    pub async fn kill_session(&self, session_id: &str, signal: i32) -> Result<()> {
        warn!("Killing session {} with signal {}", session_id, signal);

        self.manager_proxy.kill_session(session_id, "all", signal)
            .await
            .context("Failed to kill session")?;

        Ok(())
    }

    /// Activate a session
    pub async fn activate_session(&self, session_id: &str) -> Result<()> {
        info!("Activating session: {}", session_id);

        let session_path = self.active_sessions.get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        let session_proxy = LoginSessionProxyBuilder::new(&self.connection)
            .path(session_path)?
            .build()
            .await?;

        session_proxy.activate().await
            .context("Failed to activate session")?;

        Ok(())
    }

    /// Lock a session
    pub async fn lock_session(&self, session_id: &str) -> Result<()> {
        info!("Locking session: {}", session_id);

        let session_path = self.active_sessions.get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        let session_proxy = LoginSessionProxyBuilder::new(&self.connection)
            .path(session_path)?
            .build()
            .await?;

        session_proxy.lock().await
            .context("Failed to lock session")?;

        Ok(())
    }

    /// Unlock a session
    pub async fn unlock_session(&self, session_id: &str) -> Result<()> {
        info!("Unlocking session: {}", session_id);

        let session_path = self.active_sessions.get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        let session_proxy = LoginSessionProxyBuilder::new(&self.connection)
            .path(session_path)?
            .build()
            .await?;

        session_proxy.unlock().await
            .context("Failed to unlock session")?;

        Ok(())
    }

    /// Check if session exists
    pub fn has_session(&self, session_id: &str) -> bool {
        self.active_sessions.contains_key(session_id)
    }

    /// Get number of active sessions
    pub fn active_session_count(&self) -> usize {
        self.active_sessions.len()
    }
}

/// Session information from ListSessions
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub uid: u32,
    pub user: String,
    pub seat: String,
    pub path: String,
}

/// Session properties
#[derive(Debug, Clone)]
pub struct SessionProperties {
    pub id: String,
    pub uid: u32,
    pub name: String,
    pub service: String,
    pub session_type: String,
    pub session_class: String,
    pub state: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires systemd-logind running
    async fn test_logind_connection() {
        let client = LogindClient::new().await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires systemd-logind running
    async fn test_list_sessions() {
        let client = LogindClient::new().await.unwrap();
        let sessions = client.list_sessions().await;

        if let Ok(sessions) = sessions {
            println!("Found {} sessions", sessions.len());
            for session in sessions {
                println!("  Session: {} (UID: {}, User: {})",
                    session.id, session.uid, session.user);
            }
        }
    }
}
