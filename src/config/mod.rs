//! Configuration management
//!
//! Handles loading, validation, and merging of configuration from:
//! - TOML files
//! - Environment variables
//! - CLI arguments

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;

pub mod types;

// Use types from types.rs
use types::*;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Video configuration
    pub video: VideoConfig,
    /// Video pipeline configuration
    pub video_pipeline: VideoPipelineConfig,
    /// Input configuration
    pub input: InputConfig,
    /// Clipboard configuration
    pub clipboard: ClipboardConfig,
    /// Multi-monitor configuration
    pub multimon: MultiMonitorConfig,
    /// Performance configuration
    pub performance: PerformanceConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
}

impl Config {
    /// Load configuration from file
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context(format!("Failed to read config file: {}", path))?;

        let config: Config = toml::from_str(&content).context("Failed to parse config file")?;

        config.validate()?;
        Ok(config)
    }

    /// Create default configuration
    pub fn default_config() -> Result<Self> {
        Ok(Config {
            server: ServerConfig {
                listen_addr: "0.0.0.0:3389".to_string(),
                max_connections: 10,
                session_timeout: 0,
                use_portals: true,
            },
            security: SecurityConfig {
                cert_path: PathBuf::from("/etc/wrd-server/cert.pem"),
                key_path: PathBuf::from("/etc/wrd-server/key.pem"),
                enable_nla: true,
                auth_method: "pam".to_string(),
                require_tls_13: true,
            },
            video: VideoConfig {
                encoder: "auto".to_string(),
                vaapi_device: PathBuf::from("/dev/dri/renderD128"),
                target_fps: 30,
                bitrate: 4000,
                damage_tracking: true,
                cursor_mode: "metadata".to_string(),
            },
            video_pipeline: VideoPipelineConfig::default(),
            input: InputConfig {
                use_libei: true,
                keyboard_layout: "auto".to_string(),
                enable_touch: false,
            },
            clipboard: ClipboardConfig {
                enabled: true,
                max_size: 10485760, // 10 MB
                rate_limit_ms: 200, // Max 5 events/second
                allowed_types: vec![],
            },
            multimon: MultiMonitorConfig {
                enabled: true,
                max_monitors: 4,
            },
            performance: PerformanceConfig {
                encoder_threads: 0,
                network_threads: 0,
                buffer_pool_size: 16,
                zero_copy: true,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                log_dir: None,
                metrics: true,
            },
        })
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate listen address
        self.server
            .listen_addr
            .parse::<SocketAddr>()
            .context("Invalid listen address")?;

        // Validate cert paths exist
        if !self.security.cert_path.exists() {
            anyhow::bail!("Certificate not found: {:?}", self.security.cert_path);
        }
        if !self.security.key_path.exists() {
            anyhow::bail!("Private key not found: {:?}", self.security.key_path);
        }

        // Validate encoder choice
        match self.video.encoder.as_str() {
            "vaapi" | "openh264" | "auto" => {}
            _ => anyhow::bail!("Invalid encoder: {}", self.video.encoder),
        }

        // Validate cursor mode
        match self.video.cursor_mode.as_str() {
            "embedded" | "metadata" | "hidden" => {}
            _ => anyhow::bail!("Invalid cursor mode: {}", self.video.cursor_mode),
        }

        Ok(())
    }

    /// Override config with CLI arguments
    pub fn with_overrides(mut self, listen: Option<String>, port: u16) -> Self {
        if let Some(listen_addr) = listen {
            self.server.listen_addr = format!("{}:{}", listen_addr, port);
        } else {
            // Just update port
            if let Ok(mut addr) = self.server.listen_addr.parse::<SocketAddr>() {
                addr.set_port(port);
                self.server.listen_addr = addr.to_string();
            }
        }

        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::default_config().expect("Failed to create default config")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default_config().unwrap();
        assert_eq!(config.server.listen_addr, "0.0.0.0:3389");
        assert!(config.server.use_portals);
        assert_eq!(config.video.target_fps, 30);
    }

    #[test]
    fn test_config_validation_invalid_address() {
        let mut config = Config::default_config().unwrap();
        config.server.listen_addr = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_encoder() {
        let mut config = Config::default_config().unwrap();
        config.video.encoder = "invalid_encoder".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_cursor_mode() {
        let mut config = Config::default_config().unwrap();
        config.video.cursor_mode = "invalid_mode".to_string();
        assert!(config.validate().is_err());
    }
}
