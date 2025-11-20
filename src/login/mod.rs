//! WRD Direct Login Service
//!
//! Provides RDP-as-display-manager functionality, allowing users to connect
//! via RDP before any user session exists and authenticate directly.
//!
//! # Architecture
//!
//! ```text
//! WRD Login Daemon
//!   ├─> RDP Listener (port 3389)
//!   ├─> PAM Authenticator
//!   ├─> Session Manager (systemd-logind)
//!   ├─> Compositor Spawner
//!   └─> Security Manager
//! ```
//!
//! # Features
//!
//! - Pre-authentication RDP connection handling
//! - PAM integration for system authentication
//! - systemd-logind session creation
//! - Per-user compositor spawning
//! - Resource limits and cgroups
//! - Multi-user concurrent sessions
//!
//! # Usage
//!
//! The login service runs as a system daemon:
//!
//! ```bash
//! systemctl start wrd-login.service
//! ```

pub mod daemon;
pub mod auth;
pub mod session;
pub mod security;
pub mod config;

pub use daemon::WrdLoginDaemon;
pub use auth::{PamAuthenticator, AuthenticatedUser};
pub use session::{SessionManager, UserSession, SessionState};
pub use security::{SecurityManager, ResourceLimits};
pub use config::LoginConfig;
pub use logind::{LogindClient, SessionInfo, SessionProperties};

use anyhow::Result;

/// Initialize login service subsystem
pub fn init() -> Result<()> {
    tracing::info!("Initializing WRD login service subsystem");

    // Verify we're running as root
    if !nix::unistd::Uid::effective().is_root() {
        anyhow::bail!("Login service must run as root");
    }

    tracing::info!("Login service subsystem initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        // Can only test init if running as root
        if nix::unistd::Uid::effective().is_root() {
            assert!(init().is_ok());
        }
    }
}
