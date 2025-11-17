//! Configuration type definitions

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Address to listen on (e.g., "0.0.0.0:3389")
    pub listen_addr: String,

    /// Maximum number of concurrent connections
    pub max_connections: usize,

    /// Session timeout in seconds (0 = no timeout)
    pub session_timeout: u64,

    /// Use XDG Desktop Portals for screen capture
    pub use_portals: bool,
}

/// Security and authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Path to TLS certificate file
    pub cert_path: PathBuf,

    /// Path to TLS private key file
    pub key_path: PathBuf,

    /// Enable Network Level Authentication
    pub enable_nla: bool,

    /// Authentication method ("pam", "none")
    pub auth_method: String,

    /// Require TLS 1.3 or higher
    pub require_tls_13: bool,
}

/// Video encoding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    /// Video encoder to use ("vaapi", "openh264", "auto")
    pub encoder: String,

    /// VAAPI device path
    pub vaapi_device: PathBuf,

    /// Target frames per second
    pub target_fps: u32,

    /// Video bitrate in kbps
    pub bitrate: u32,

    /// Enable damage tracking for efficient updates
    pub damage_tracking: bool,

    /// Cursor rendering mode ("embedded", "metadata", "hidden")
    pub cursor_mode: String,
}

/// Input handling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    /// Use libei for input injection
    pub use_libei: bool,

    /// Keyboard layout ("auto" or XKB layout name)
    pub keyboard_layout: String,

    /// Enable touch input support
    pub enable_touch: bool,
}

/// Clipboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardConfig {
    /// Enable clipboard synchronization
    pub enabled: bool,

    /// Maximum clipboard data size in bytes
    pub max_size: usize,

    /// Allowed MIME types (empty = all types allowed)
    pub allowed_types: Vec<String>,
}

/// Multi-monitor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiMonitorConfig {
    /// Enable multi-monitor support
    pub enabled: bool,

    /// Maximum number of monitors to support
    pub max_monitors: usize,
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Number of encoder threads (0 = auto)
    pub encoder_threads: usize,

    /// Number of network threads (0 = auto)
    pub network_threads: usize,

    /// Size of the frame buffer pool
    pub buffer_pool_size: usize,

    /// Enable zero-copy operations where possible
    pub zero_copy: bool,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level ("trace", "debug", "info", "warn", "error")
    pub level: String,

    /// Directory for log files (None = console only)
    pub log_dir: Option<PathBuf>,

    /// Enable metrics collection
    pub metrics: bool,
}
