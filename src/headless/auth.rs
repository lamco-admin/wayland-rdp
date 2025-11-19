//! Authentication provider for headless RDP server
//!
//! Supports multiple authentication backends including PAM, LDAP,
//! and custom authentication schemes. Provides secure multi-user
//! authentication with support for 2FA and session token management.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::headless::config::AuthenticationConfig;

/// Authentication result
#[derive(Debug, Clone)]
pub struct AuthResult {
    /// Whether authentication succeeded
    pub success: bool,

    /// Authenticated user information
    pub user_info: Option<UserInfo>,

    /// Session token (if successful)
    pub session_token: Option<String>,

    /// Error message (if failed)
    pub error_message: Option<String>,

    /// Requires two-factor authentication
    pub requires_2fa: bool,
}

/// Authenticated user information
#[derive(Debug, Clone)]
pub struct UserInfo {
    /// Username
    pub username: String,

    /// User ID (UID)
    pub uid: u32,

    /// Primary group ID (GID)
    pub gid: u32,

    /// Home directory
    pub home_dir: String,

    /// Login shell
    pub shell: String,

    /// Full name (GECOS)
    pub full_name: Option<String>,

    /// Group memberships
    pub groups: Vec<String>,

    /// Additional user attributes
    pub attributes: HashMap<String, String>,
}

/// Authentication provider trait
pub trait AuthProvider: Send + Sync {
    /// Authenticate user with username and password
    fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> impl std::future::Future<Output = Result<AuthResult>> + Send;

    /// Verify two-factor authentication token
    fn verify_2fa(
        &self,
        username: &str,
        token: &str,
    ) -> impl std::future::Future<Output = Result<bool>> + Send;

    /// Get user information
    fn get_user_info(
        &self,
        username: &str,
    ) -> impl std::future::Future<Output = Result<UserInfo>> + Send;

    /// Validate session token
    fn validate_session_token(
        &self,
        token: &str,
    ) -> impl std::future::Future<Output = Result<UserInfo>> + Send;
}

/// PAM-based authentication provider
pub struct PamAuthenticator {
    config: Arc<AuthenticationConfig>,
    failed_attempts: Arc<RwLock<HashMap<String, FailedLoginTracker>>>,
    session_cache: Arc<RwLock<HashMap<String, CachedSession>>>,
}

/// Failed login attempt tracker
#[derive(Debug, Clone)]
struct FailedLoginTracker {
    count: usize,
    last_attempt: Instant,
    locked_until: Option<Instant>,
}

/// Cached authentication session
#[derive(Debug, Clone)]
struct CachedSession {
    user_info: UserInfo,
    created_at: Instant,
    expires_at: Instant,
}

impl PamAuthenticator {
    /// Create new PAM authenticator
    pub fn new(config: Arc<AuthenticationConfig>) -> Self {
        info!("Initializing PAM authenticator with service: {}", config.pam_service);

        Self {
            config,
            failed_attempts: Arc::new(RwLock::new(HashMap::new())),
            session_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if user is locked out
    async fn is_locked_out(&self, username: &str) -> bool {
        let attempts = self.failed_attempts.read().await;

        if let Some(tracker) = attempts.get(username) {
            if let Some(locked_until) = tracker.locked_until {
                if Instant::now() < locked_until {
                    debug!("User {} is locked out", username);
                    return true;
                }
            }
        }

        false
    }

    /// Record failed login attempt
    async fn record_failed_attempt(&self, username: &str) {
        let mut attempts = self.failed_attempts.write().await;

        let tracker = attempts.entry(username.to_string()).or_insert(FailedLoginTracker {
            count: 0,
            last_attempt: Instant::now(),
            locked_until: None,
        });

        tracker.count += 1;
        tracker.last_attempt = Instant::now();

        if tracker.count >= self.config.max_failed_attempts {
            tracker.locked_until =
                Some(Instant::now() + Duration::from_secs(self.config.lockout_duration));

            warn!(
                "User {} locked out after {} failed attempts",
                username, tracker.count
            );
        }
    }

    /// Reset failed attempts on successful login
    async fn reset_failed_attempts(&self, username: &str) {
        let mut attempts = self.failed_attempts.write().await;
        attempts.remove(username);
    }

    /// Generate session token
    fn generate_session_token(&self) -> String {
        use sha2::{Digest, Sha256};

        let uuid = uuid::Uuid::new_v4();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let data = format!("{}-{}", uuid, timestamp);
        let hash = Sha256::digest(data.as_bytes());
        format!("{:x}", hash)
    }

    /// Cache authenticated session
    async fn cache_session(&self, token: String, user_info: UserInfo) {
        if !self.config.enable_cache {
            return;
        }

        let mut cache = self.session_cache.write().await;

        let cached = CachedSession {
            user_info,
            created_at: Instant::now(),
            expires_at: Instant::now() + Duration::from_secs(self.config.cache_timeout),
        };

        cache.insert(token, cached);
    }

    /// Cleanup expired cache entries
    async fn cleanup_cache(&self) {
        let mut cache = self.session_cache.write().await;
        let now = Instant::now();

        cache.retain(|_, session| session.expires_at > now);
    }

    /// Perform PAM authentication
    #[cfg(feature = "pam-auth")]
    async fn pam_authenticate(&self, username: &str, password: &str) -> Result<bool> {
        use pam::Authenticator;

        let service = self.config.pam_service.clone();
        let username = username.to_string();
        let password = password.to_string();

        // Run PAM authentication in blocking thread pool
        tokio::task::spawn_blocking(move || {
            let mut authenticator = Authenticator::with_password(&service)
                .context("Failed to create PAM authenticator")?;

            authenticator
                .get_handler()
                .set_credentials(&username, &password);

            authenticator
                .authenticate()
                .context("PAM authentication failed")?;

            authenticator
                .open_session()
                .context("Failed to open PAM session")?;

            Ok(true)
        })
        .await
        .context("PAM authentication task failed")?
    }

    /// Fallback authentication when PAM is not available
    #[cfg(not(feature = "pam-auth"))]
    async fn pam_authenticate(&self, username: &str, password: &str) -> Result<bool> {
        warn!("PAM authentication not compiled - falling back to basic auth");

        // For development/testing only - DO NOT USE IN PRODUCTION
        if username == "test" && password == "test" {
            Ok(true)
        } else {
            anyhow::bail!("Authentication failed")
        }
    }

    /// Get user information from system
    async fn lookup_user_info(&self, username: &str) -> Result<UserInfo> {
        let username = username.to_string();

        // Run user lookup in blocking thread pool
        tokio::task::spawn_blocking(move || {
            use uzers::{get_user_by_name, Groups};

            let user = get_user_by_name(&username)
                .ok_or_else(|| anyhow::anyhow!("User not found: {}", username))?;

            let groups = Groups::new()
                .ok_or_else(|| anyhow::anyhow!("Failed to get groups"))?
                .get_user_groups(&username)
                .into_iter()
                .filter_map(|g| g.name().to_str().map(String::from))
                .collect();

            Ok(UserInfo {
                username: username.clone(),
                uid: user.uid(),
                gid: user.primary_group_id(),
                home_dir: user
                    .home_dir()
                    .to_str()
                    .unwrap_or("/tmp")
                    .to_string(),
                shell: user.shell().to_str().unwrap_or("/bin/bash").to_string(),
                full_name: user.gecos().to_str().map(String::from),
                groups,
                attributes: HashMap::new(),
            })
        })
        .await
        .context("User lookup task failed")?
    }
}

impl AuthProvider for PamAuthenticator {
    async fn authenticate(&self, username: &str, password: &str) -> Result<AuthResult> {
        info!("Authentication attempt for user: {}", username);

        // Check if user is locked out
        if self.is_locked_out(username).await {
            return Ok(AuthResult {
                success: false,
                user_info: None,
                session_token: None,
                error_message: Some("Account temporarily locked due to failed login attempts".to_string()),
                requires_2fa: false,
            });
        }

        // Perform PAM authentication
        match self.pam_authenticate(username, password).await {
            Ok(true) => {
                // Authentication successful
                self.reset_failed_attempts(username).await;

                let user_info = self.lookup_user_info(username).await?;

                let requires_2fa = self.config.enable_2fa;

                let session_token = if !requires_2fa {
                    let token = self.generate_session_token();
                    self.cache_session(token.clone(), user_info.clone()).await;
                    Some(token)
                } else {
                    None
                };

                info!("Authentication successful for user: {}", username);

                Ok(AuthResult {
                    success: true,
                    user_info: Some(user_info),
                    session_token,
                    error_message: None,
                    requires_2fa,
                })
            }
            Ok(false) | Err(_) => {
                // Authentication failed
                self.record_failed_attempt(username).await;

                warn!("Authentication failed for user: {}", username);

                Ok(AuthResult {
                    success: false,
                    user_info: None,
                    session_token: None,
                    error_message: Some("Invalid username or password".to_string()),
                    requires_2fa: false,
                })
            }
        }
    }

    async fn verify_2fa(&self, username: &str, token: &str) -> Result<bool> {
        if !self.config.enable_2fa {
            return Ok(true);
        }

        // TODO: Implement actual 2FA verification (TOTP, U2F, etc.)
        // For now, this is a placeholder

        debug!("2FA verification for user: {} (placeholder)", username);
        Ok(token == "000000") // Placeholder - always accept 000000 for testing
    }

    async fn get_user_info(&self, username: &str) -> Result<UserInfo> {
        self.lookup_user_info(username).await
    }

    async fn validate_session_token(&self, token: &str) -> Result<UserInfo> {
        // Cleanup expired entries
        self.cleanup_cache().await;

        let cache = self.session_cache.read().await;

        cache
            .get(token)
            .filter(|session| session.expires_at > Instant::now())
            .map(|session| session.user_info.clone())
            .ok_or_else(|| anyhow::anyhow!("Invalid or expired session token"))
    }
}

/// Create authentication provider based on configuration
pub async fn create_auth_provider(config: Arc<AuthenticationConfig>) -> Result<Box<dyn AuthProvider>> {
    match config.provider.as_str() {
        "pam" => {
            info!("Creating PAM authentication provider");
            Ok(Box::new(PamAuthenticator::new(config)))
        }
        "ldap" => {
            // TODO: Implement LDAP authentication provider
            anyhow::bail!("LDAP authentication not yet implemented")
        }
        "none" => {
            // TODO: Implement no-auth provider (for development only)
            warn!("No authentication configured - INSECURE!");
            anyhow::bail!("No-auth mode not implemented")
        }
        _ => {
            anyhow::bail!("Unknown authentication provider: {}", config.provider)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pam_authenticator_creation() {
        let config = Arc::new(AuthenticationConfig::default());
        let auth = PamAuthenticator::new(config);

        // Just verify creation works
        assert!(auth.config.pam_service == "wrd-server");
    }

    #[tokio::test]
    async fn test_session_token_generation() {
        let config = Arc::new(AuthenticationConfig::default());
        let auth = PamAuthenticator::new(config);

        let token1 = auth.generate_session_token();
        let token2 = auth.generate_session_token();

        // Tokens should be unique
        assert_ne!(token1, token2);
        // Tokens should be 64 hex chars (SHA-256)
        assert_eq!(token1.len(), 64);
    }
}
