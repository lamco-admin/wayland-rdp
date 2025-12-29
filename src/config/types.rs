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

/// EGFX (Graphics Pipeline Extension) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EgfxConfig {
    /// Enable EGFX graphics pipeline
    pub enabled: bool,

    /// H.264 level: "auto" or explicit "3.0", "3.1", "4.0", "4.1", "5.0", "5.1", "5.2"
    pub h264_level: String,

    /// H.264 bitrate in kbps (main stream for AVC444)
    pub h264_bitrate: u32,

    /// ZGFX compression mode: "never", "auto", "always"
    pub zgfx_compression: String,

    /// Maximum frames in flight before backpressure
    pub max_frames_in_flight: u32,

    /// Frame acknowledgment timeout (ms)
    pub frame_ack_timeout: u64,

    /// Video codec preference: "auto", "avc420", "avc444"
    /// - "auto": Use best available codec (AVC444 if client supports V10+, else AVC420)
    /// - "avc420": Always use AVC420 (4:2:0 chroma), even if AVC444 is available
    /// - "avc444": Prefer AVC444 (4:4:4 chroma) for superior text/UI rendering
    pub codec: String,

    /// Quality parameter range
    pub qp_min: u8,
    pub qp_max: u8,
    pub qp_default: u8,

    // === AVC444-specific configuration ===

    /// AVC444 auxiliary stream bitrate ratio (0.3-1.0)
    /// Ratio of auxiliary stream bitrate relative to main stream.
    /// - 0.5 = aux gets 50% of main's bitrate (good for typical content)
    /// - 1.0 = aux gets same bitrate as main (best quality for text-heavy)
    /// - 0.3 = aux gets 30% of main's bitrate (saves bandwidth)
    #[serde(default = "default_avc444_aux_ratio")]
    pub avc444_aux_bitrate_ratio: f32,

    /// Color matrix for YUV conversion: "auto", "bt709", "bt601"
    /// - "auto": Use BT.709 for HD (≥1080p), BT.601 for SD
    /// - "bt709": Force BT.709 (recommended for HD content)
    /// - "bt601": Force BT.601 (legacy SD content compatibility)
    #[serde(default = "default_color_matrix")]
    pub color_matrix: String,

    /// Enable AVC444 when client supports it
    /// Set to false to disable AVC444 globally regardless of codec preference
    #[serde(default = "default_true")]
    pub avc444_enabled: bool,

    // === PHASE 1: AUX OMISSION (BANDWIDTH OPTIMIZATION) ===

    /// Enable auxiliary stream omission for bandwidth optimization
    /// When true: Implements FreeRDP-style aux omission (LC field)
    /// When false: Always sends both streams (backward compatible)
    /// Default: true (production proven at 0.81 MB/s)
    #[serde(default = "default_true")]
    pub avc444_enable_aux_omission: bool,

    /// Maximum frames between auxiliary updates (1-120)
    /// Forces aux refresh even if unchanged for quality assurance
    /// - 10-20: Responsive to color changes, higher bandwidth
    /// - 30-40: Balanced (recommended)
    /// - 60-120: Aggressive omission, static content optimized
    /// Default: 30 frames (1 second @ 30fps)
    #[serde(default = "default_aux_interval")]
    pub avc444_max_aux_interval: u32,

    /// Auxiliary change detection threshold (0.0-1.0)
    /// Fraction of pixels that must change to trigger aux update
    /// - 0.0: Any change triggers update
    /// - 0.05: 5% changed (balanced, recommended)
    /// - 0.1: 10% changed (aggressive)
    /// Default: 0.05 (5%)
    #[serde(default = "default_aux_threshold")]
    pub avc444_aux_change_threshold: f32,

    /// Force auxiliary IDR when reintroducing after omission
    /// true: Safe mode, but with single encoder forces Main to IDR too!
    /// false: Required for single encoder to allow Main P-frames (PRODUCTION)
    /// Default: false
    #[serde(default = "default_false")]
    pub avc444_force_aux_idr_on_return: bool,
}

fn default_avc444_aux_ratio() -> f32 {
    0.5
}

fn default_aux_interval() -> u32 {
    30  // 1 second @ 30fps
}

fn default_aux_threshold() -> f32 {
    0.05  // 5% pixels changed
}

fn default_false() -> bool {
    false
}

fn default_color_matrix() -> String {
    "auto".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for EgfxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            h264_level: "auto".to_string(),
            h264_bitrate: 5000,
            zgfx_compression: "never".to_string(),
            max_frames_in_flight: 3,
            frame_ack_timeout: 5000,
            codec: "auto".to_string(), // Use best available (AVC444 if supported, else AVC420)
            qp_min: 10,
            qp_max: 40,
            qp_default: 23,
            // AVC444-specific defaults
            avc444_aux_bitrate_ratio: 0.5, // Aux gets 50% of main's bitrate
            color_matrix: "auto".to_string(), // Auto-detect based on resolution
            avc444_enabled: true, // Enable AVC444 when client supports it
            // Phase 1: Aux omission defaults (NOW PRODUCTION DEFAULTS)
            avc444_enable_aux_omission: true,   // Enabled by default (production proven)
            avc444_max_aux_interval: 30,        // 1 second @ 30fps
            avc444_aux_change_threshold: 0.05,  // 5% pixels changed
            avc444_force_aux_idr_on_return: false,  // Must be false for single encoder
        }
    }
}

/// Damage tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageTrackingConfig {
    /// Enable damage region detection
    pub enabled: bool,

    /// Detection method: "pipewire", "diff", "hybrid"
    pub method: String,

    /// Tile size for differencing (pixels)
    pub tile_size: usize,

    /// Difference threshold (0.0-1.0)
    pub diff_threshold: f32,

    /// Merge distance for adjacent tiles (pixels)
    pub merge_distance: u32,
}

impl Default for DamageTrackingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            method: "diff".to_string(),
            tile_size: 64,
            diff_threshold: 0.05,
            merge_distance: 32,
        }
    }
}

/// Hardware encoding configuration
///
/// Supports multiple GPU backends:
/// - VA-API: Intel (iHD/i965) and AMD (radeonsi) GPUs
/// - NVENC: NVIDIA GPUs via Video Codec SDK
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareEncodingConfig {
    /// Enable hardware-accelerated encoding
    pub enabled: bool,

    /// VA-API device path (for Intel/AMD GPUs)
    pub vaapi_device: PathBuf,

    /// Enable zero-copy DMA-BUF path (VA-API only)
    pub enable_dmabuf_zerocopy: bool,

    /// Fallback to software encoding if hardware fails
    pub fallback_to_software: bool,

    /// Encoder quality preset: "speed", "balanced", "quality"
    /// - speed: Low latency, lower quality (3 Mbps)
    /// - balanced: Good balance of quality and latency (5 Mbps)
    /// - quality: Best quality, higher latency (10 Mbps)
    pub quality_preset: String,

    /// Prefer NVENC over VA-API when both are available
    /// NVENC typically has lower latency but requires NVIDIA GPU
    #[serde(default = "default_prefer_nvenc")]
    pub prefer_nvenc: bool,
}

fn default_prefer_nvenc() -> bool {
    true // NVENC preferred when available (lower latency)
}

impl Default for HardwareEncodingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            vaapi_device: PathBuf::from("/dev/dri/renderD128"),
            enable_dmabuf_zerocopy: true,
            fallback_to_software: true,
            quality_preset: "balanced".to_string(),
            prefer_nvenc: true,
        }
    }
}

/// Display control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Allow dynamic resolution changes
    pub allow_resize: bool,

    /// Allowed resolutions (empty = all allowed)
    pub allowed_resolutions: Vec<String>,

    /// DPI scaling support
    pub dpi_aware: bool,

    /// Allow orientation changes
    pub allow_rotation: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            allow_resize: true,
            allowed_resolutions: vec![],
            dpi_aware: false,
            allow_rotation: false,
        }
    }
}

/// Advanced video pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedVideoConfig {
    /// Enable encoder frame skipping
    pub enable_frame_skip: bool,

    /// Scene change detection sensitivity (0.0-1.0)
    pub scene_change_threshold: f32,

    /// Intra refresh interval (frames, 0 = scene changes only)
    pub intra_refresh_interval: u32,

    /// Enable adaptive quality
    pub enable_adaptive_quality: bool,
}

impl Default for AdvancedVideoConfig {
    fn default() -> Self {
        Self {
            enable_frame_skip: true,
            scene_change_threshold: 0.7,
            intra_refresh_interval: 300,
            enable_adaptive_quality: false,
        }
    }
}
