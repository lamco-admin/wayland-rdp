//! PAM authentication module
//!
//! Provides system authentication using PAM (Pluggable Authentication Modules).

use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::{debug, info, warn};

#[cfg(feature = "pam-auth")]
use pam::Authenticator;

/// Authenticated user information
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// Username
    pub username: String,

    /// User ID (UID)
    pub uid: u32,

    /// Group ID (GID)
    pub gid: u32,

    /// Home directory
    pub home: PathBuf,

    /// Shell
    pub shell: PathBuf,

    /// Full name (GECOS)
    pub gecos: String,
}

impl AuthenticatedUser {
    /// Get user information from system
    pub fn from_username(username: &str) -> Result<Self> {
        use nix::unistd::{User as NixUser};

        let user = NixUser::from_name(username)
            .context("Failed to query user database")?
            .ok_or_else(|| anyhow::anyhow!("User not found: {}", username))?;

        Ok(Self {
            username: user.name,
            uid: user.uid.as_raw(),
            gid: user.gid.as_raw(),
            home: user.dir,
            shell: user.shell,
            gecos: user.gecos,
        })
    }

    /// Get runtime directory path
    pub fn runtime_dir(&self) -> PathBuf {
        PathBuf::from(format!("/run/user/{}", self.uid))
    }

    /// Get Wayland display name
    pub fn wayland_display(&self) -> String {
        format!("wayland-{}", self.uid)
    }
}

/// PAM authenticator
pub struct PamAuthenticator {
    service_name: String,
}

impl PamAuthenticator {
    /// Create new PAM authenticator
    pub fn new(service_name: String) -> Self {
        Self { service_name }
    }

    /// Authenticate user with username and password
    ///
    /// This method performs blocking PAM operations and should be called
    /// from a tokio blocking task.
    #[cfg(feature = "pam-auth")]
    pub fn authenticate(&self, username: &str, password: &str) -> Result<AuthenticatedUser> {
        info!("Authenticating user: {}", username);

        // Create PAM authenticator
        let mut auth = Authenticator::with_password(&self.service_name)
            .context("Failed to create PAM authenticator")?;

        // Set credentials
        auth.get_handler().set_credentials(username, password);

        // Perform authentication
        auth.authenticate()
            .context("PAM authentication failed")?;

        debug!("PAM authentication successful for user: {}", username);

        // Open PAM session
        auth.open_session()
            .context("Failed to open PAM session")?;

        debug!("PAM session opened for user: {}", username);

        // Get user information from system
        let user = AuthenticatedUser::from_username(username)?;

        info!("User {} (UID: {}) authenticated successfully", username, user.uid);

        Ok(user)
    }

    #[cfg(not(feature = "pam-auth"))]
    pub fn authenticate(&self, username: &str, _password: &str) -> Result<AuthenticatedUser> {
        warn!("PAM authentication not enabled - using fallback");

        // Fallback: just get user info (NO PASSWORD CHECK - INSECURE!)
        AuthenticatedUser::from_username(username)
    }

    /// Validate password complexity
    pub fn validate_password_strength(&self, password: &str) -> Result<()> {
        // Minimum length
        if password.len() < 8 {
            anyhow::bail!("Password must be at least 8 characters");
        }

        // Check for variety
        let has_lower = password.chars().any(|c| c.is_lowercase());
        let has_upper = password.chars().any(|c| c.is_uppercase());
        let has_digit = password.chars().any(|c| c.is_numeric());
        let has_special = password.chars().any(|c| !c.is_alphanumeric());

        let variety_count = [has_lower, has_upper, has_digit, has_special]
            .iter()
            .filter(|&&x| x)
            .count();

        if variety_count < 3 {
            anyhow::bail!("Password must contain at least 3 of: lowercase, uppercase, digits, special characters");
        }

        Ok(())
    }
}

/// Authentication result
#[derive(Debug)]
pub enum AuthResult {
    /// Authentication successful
    Success(AuthenticatedUser),

    /// Authentication failed
    Failed(String),

    /// Account locked
    Locked(String),

    /// Other error
    Error(String),
}

impl AuthResult {
    /// Check if authentication succeeded
    pub fn is_success(&self) -> bool {
        matches!(self, AuthResult::Success(_))
    }

    /// Get user if authentication succeeded
    pub fn user(self) -> Option<AuthenticatedUser> {
        match self {
            AuthResult::Success(user) => Some(user),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_validation() {
        let auth = PamAuthenticator::new("test".to_string());

        // Too short
        assert!(auth.validate_password_strength("pass").is_err());

        // No variety
        assert!(auth.validate_password_strength("password").is_err());

        // Good password
        assert!(auth.validate_password_strength("Password123!").is_ok());
    }

    #[test]
    fn test_user_from_username() {
        // Test with current user
        if let Ok(current_user) = std::env::var("USER") {
            let user = AuthenticatedUser::from_username(&current_user);
            assert!(user.is_ok());

            if let Ok(user) = user {
                assert_eq!(user.username, current_user);
                assert!(user.uid > 0);
            }
        }
    }

    #[test]
    fn test_runtime_dir() {
        let user = AuthenticatedUser {
            username: "testuser".to_string(),
            uid: 1000,
            gid: 1000,
            home: PathBuf::from("/home/testuser"),
            shell: PathBuf::from("/bin/bash"),
            gecos: "Test User".to_string(),
        };

        assert_eq!(user.runtime_dir(), PathBuf::from("/run/user/1000"));
        assert_eq!(user.wayland_display(), "wayland-1000");
    }
}
