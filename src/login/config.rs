//! Login service configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::{Context, Result};

/// Login service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginConfig {
    /// Network configuration
    pub network: NetworkConfig,

    /// Security configuration
    pub security: SecurityConfig,

    /// Session configuration
    pub session: SessionConfig,

    /// Paths configuration
    pub paths: PathsConfig,

    /// Resource limits
    pub limits: ResourceLimitsConfig,
}

impl Default for LoginConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            security: SecurityConfig::default(),
            session: SessionConfig::default(),
            paths: PathsConfig::default(),
            limits: ResourceLimitsConfig::default(),
        }
    }
}

impl LoginConfig {
    /// Load configuration from file
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;

        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path))?;

        config.validate()?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self, path: &str) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path))?;

        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate port
        if self.network.port == 0 {
            anyhow::bail!("Invalid port number: 0");
        }

        // Validate paths
        if !self.paths.compositor_path.exists() {
            tracing::warn!("Compositor binary not found: {:?}", self.paths.compositor_path);
        }

        if !self.paths.cert_path.exists() {
            anyhow::bail!("Certificate file not found: {:?}", self.paths.cert_path);
        }

        if !self.paths.key_path.exists() {
            anyhow::bail!("Key file not found: {:?}", self.paths.key_path);
        }

        Ok(())
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Bind address
    #[serde(default = "default_bind_address")]
    pub bind_address: String,

    /// Listen port
    #[serde(default = "default_port")]
    pub port: u16,

    /// Maximum concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Connection timeout (seconds)
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout: u32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            port: default_port(),
            max_connections: default_max_connections(),
            connection_timeout: default_connection_timeout(),
        }
    }
}

fn default_bind_address() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3389
}

fn default_max_connections() -> u32 {
    100
}

fn default_connection_timeout() -> u32 {
    300
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable account lockout after failed attempts
    #[serde(default = "default_true")]
    pub enable_lockout: bool,

    /// Failed login attempts before lockout
    #[serde(default = "default_max_failed_attempts")]
    pub max_failed_attempts: u32,

    /// Lockout duration (seconds)
    #[serde(default = "default_lockout_duration")]
    pub lockout_duration: u32,

    /// Require strong passwords
    #[serde(default = "default_true")]
    pub require_strong_passwords: bool,

    /// Enable audit logging
    #[serde(default = "default_true")]
    pub audit_logging: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_lockout: true,
            max_failed_attempts: default_max_failed_attempts(),
            lockout_duration: default_lockout_duration(),
            require_strong_passwords: true,
            audit_logging: true,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_max_failed_attempts() -> u32 {
    5
}

fn default_lockout_duration() -> u32 {
    300 // 5 minutes
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Session timeout (seconds, 0 = no timeout)
    #[serde(default)]
    pub timeout: u32,

    /// Enable XWayland for X11 applications
    #[serde(default)]
    pub enable_xwayland: bool,

    /// Applications to auto-start
    #[serde(default)]
    pub auto_start_apps: Vec<String>,

    /// Environment variables to set
    #[serde(default)]
    pub environment: Vec<EnvVar>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout: 0,
            enable_xwayland: false,
            auto_start_apps: Vec::new(),
            environment: Vec::new(),
        }
    }
}

/// Environment variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

/// Paths configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    /// Path to compositor binary
    #[serde(default = "default_compositor_path")]
    pub compositor_path: PathBuf,

    /// Path to TLS certificate
    #[serde(default = "default_cert_path")]
    pub cert_path: PathBuf,

    /// Path to TLS private key
    #[serde(default = "default_key_path")]
    pub key_path: PathBuf,

    /// PAM service name
    #[serde(default = "default_pam_service")]
    pub pam_service: String,

    /// Log directory
    #[serde(default = "default_log_dir")]
    pub log_dir: PathBuf,
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            compositor_path: default_compositor_path(),
            cert_path: default_cert_path(),
            key_path: default_key_path(),
            pam_service: default_pam_service(),
            log_dir: default_log_dir(),
        }
    }
}

fn default_compositor_path() -> PathBuf {
    PathBuf::from("/usr/bin/wrd-compositor")
}

fn default_cert_path() -> PathBuf {
    PathBuf::from("/etc/wrd-login/certs/server.crt")
}

fn default_key_path() -> PathBuf {
    PathBuf::from("/etc/wrd-login/certs/server.key")
}

fn default_pam_service() -> String {
    "wrd-login".to_string()
}

fn default_log_dir() -> PathBuf {
    PathBuf::from("/var/log/wrd-login")
}

/// Resource limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimitsConfig {
    /// Maximum memory per user (MB)
    #[serde(default = "default_max_memory")]
    pub max_memory_mb: u64,

    /// CPU shares per user
    #[serde(default = "default_cpu_shares")]
    pub cpu_shares: u64,

    /// Maximum processes per user
    #[serde(default = "default_max_processes")]
    pub max_processes: u32,

    /// Maximum open files per user
    #[serde(default = "default_max_open_files")]
    pub max_open_files: u32,
}

impl Default for ResourceLimitsConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: default_max_memory(),
            cpu_shares: default_cpu_shares(),
            max_processes: default_max_processes(),
            max_open_files: default_max_open_files(),
        }
    }
}

fn default_max_memory() -> u64 {
    2048 // 2GB
}

fn default_cpu_shares() -> u64 {
    1024
}

fn default_max_processes() -> u32 {
    256
}

fn default_max_open_files() -> u32 {
    1024
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LoginConfig::default();
        assert_eq!(config.network.port, 3389);
        assert_eq!(config.security.max_failed_attempts, 5);
    }

    #[test]
    fn test_config_serialization() {
        let config = LoginConfig::default();
        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("port"));
        assert!(toml.contains("max_failed_attempts"));
    }
}
