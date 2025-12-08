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

    /// Minimum milliseconds between clipboard events (rate limiting)
    /// Default: 200 (max 5 events/second). Set to 0 to disable.
    #[serde(default = "default_rate_limit_ms")]
    pub rate_limit_ms: u64,

    /// Allowed MIME types (empty = all types allowed)
    pub allowed_types: Vec<String>,
}

fn default_rate_limit_ms() -> u64 {
    200
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

/// Video pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoPipelineConfig {
    /// Frame processor configuration
    pub processor: ProcessorConfig,

    /// Frame dispatcher configuration
    pub dispatcher: DispatcherConfig,

    /// Bitmap converter configuration
    pub converter: ConverterConfig,
}

/// Frame processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    /// Target frame rate (FPS)
    pub target_fps: u32,

    /// Maximum frame queue depth
    pub max_queue_depth: usize,

    /// Enable adaptive quality
    pub adaptive_quality: bool,

    /// Damage tracking threshold (0.0-1.0)
    pub damage_threshold: f32,

    /// Drop frames when queue is full
    pub drop_on_full_queue: bool,

    /// Enable performance metrics
    pub enable_metrics: bool,
}

/// Frame dispatcher configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatcherConfig {
    /// Channel buffer size per stream
    pub channel_size: usize,

    /// Enable priority-based dispatch
    pub priority_dispatch: bool,

    /// Maximum frame age before drop (ms)
    pub max_frame_age_ms: u64,

    /// Enable backpressure handling
    pub enable_backpressure: bool,

    /// High water mark (0.0-1.0)
    pub high_water_mark: f32,

    /// Low water mark (0.0-1.0)
    pub low_water_mark: f32,

    /// Enable load balancing
    pub load_balancing: bool,
}

/// Bitmap converter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConverterConfig {
    /// Buffer pool size
    pub buffer_pool_size: usize,

    /// Enable SIMD optimizations
    pub enable_simd: bool,

    /// Damage threshold for full update (0.0-1.0)
    pub damage_threshold: f32,

    /// Enable statistics collection
    pub enable_statistics: bool,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            target_fps: 30,
            max_queue_depth: 30,
            adaptive_quality: true,
            damage_threshold: 0.05,
            drop_on_full_queue: true,
            enable_metrics: true,
        }
    }
}

impl Default for DispatcherConfig {
    fn default() -> Self {
        Self {
            channel_size: 30,
            priority_dispatch: true,
            max_frame_age_ms: 150,
            enable_backpressure: true,
            high_water_mark: 0.8,
            low_water_mark: 0.5,
            load_balancing: true,
        }
    }
}

impl Default for ConverterConfig {
    fn default() -> Self {
        Self {
            buffer_pool_size: 8,
            enable_simd: true,
            damage_threshold: 0.75,
            enable_statistics: true,
        }
    }
}

impl Default for VideoPipelineConfig {
    fn default() -> Self {
        Self {
            processor: ProcessorConfig::default(),
            dispatcher: DispatcherConfig::default(),
            converter: ConverterConfig::default(),
        }
    }
}
