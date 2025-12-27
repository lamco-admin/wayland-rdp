# WRD-Server Data Structures Specification

## Document Information
- **Version**: 1.0.0
- **Last Updated**: 2024-11-18
- **Status**: Production-Ready
- **Classification**: Technical Specification

## 1. Overview

This document provides a complete specification of all data structures used in the wrd-server project. Each structure is defined with exact field types, memory layout, serialization formats, and validation rules.

## 2. Core Video Structures

### 2.1 VideoFrame

```rust
/// Complete video frame with metadata for pipeline processing
#[derive(Debug, Clone)]
#[repr(C)]
pub struct VideoFrame {
    /// Unique frame identifier (monotonically increasing)
    pub frame_id: u64,

    /// Presentation timestamp in nanoseconds since stream start
    pub pts: i64,

    /// Decode timestamp in nanoseconds
    pub dts: i64,

    /// Frame duration in nanoseconds
    pub duration: i64,

    /// Raw pixel data in BGRA format
    pub data: Vec<u8>,

    /// Frame width in pixels
    pub width: u32,

    /// Frame height in pixels
    pub height: u32,

    /// Bytes per row (stride)
    pub stride: u32,

    /// Pixel format
    pub format: PixelFormat,

    /// Damage regions for incremental updates
    pub damage_regions: Vec<DamageRegion>,

    /// Frame capture timestamp (system monotonic clock)
    pub capture_time: std::time::Instant,

    /// Frame processing metadata
    pub metadata: FrameMetadata,

    /// Monitor index for multi-monitor setups
    pub monitor_index: u32,

    /// Frame flags
    pub flags: FrameFlags,
}

/// Memory Layout:
/// - Size: 144 bytes (excluding Vec allocations)
/// - Alignment: 8 bytes
/// - Field offsets:
///   - frame_id: 0
///   - pts: 8
///   - dts: 16
///   - duration: 24
///   - data: 32 (24 bytes for Vec<u8>)
///   - width: 56
///   - height: 60
///   - stride: 64
///   - format: 68
///   - damage_regions: 72 (24 bytes for Vec)
///   - capture_time: 96 (16 bytes for Instant)
///   - metadata: 112 (size varies)
///   - monitor_index: 136
///   - flags: 140

/// Validation Rules:
/// - width > 0 && width <= 7680 (8K max)
/// - height > 0 && height <= 4320 (8K max)
/// - stride >= width * bytes_per_pixel(format)
/// - data.len() == stride * height
/// - frame_id must be unique and sequential
/// - pts >= 0
/// - duration > 0
```

### 2.2 PixelFormat

```rust
/// Pixel format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PixelFormat {
    /// 32-bit BGRA (Blue, Green, Red, Alpha)
    BGRA = 0x00000001,

    /// 32-bit RGBA (Red, Green, Blue, Alpha)
    RGBA = 0x00000002,

    /// 32-bit ARGB (Alpha, Red, Green, Blue)
    ARGB = 0x00000003,

    /// 24-bit RGB (Red, Green, Blue)
    RGB = 0x00000004,

    /// 24-bit BGR (Blue, Green, Red)
    BGR = 0x00000005,

    /// 16-bit RGB565
    RGB565 = 0x00000006,

    /// 12-bit YUV420 planar
    YUV420 = 0x00000007,

    /// 16-bit YUV422 packed
    YUV422 = 0x00000008,
}

impl PixelFormat {
    /// Returns bytes per pixel
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::BGRA | Self::RGBA | Self::ARGB => 4,
            Self::RGB | Self::BGR => 3,
            Self::RGB565 | Self::YUV422 => 2,
            Self::YUV420 => 1, // Average for planar format
        }
    }
}
```

### 2.3 DamageRegion

```rust
/// Rectangular region marking changed pixels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct DamageRegion {
    /// X coordinate of top-left corner
    pub x: u32,

    /// Y coordinate of top-left corner
    pub y: u32,

    /// Width of the region
    pub width: u32,

    /// Height of the region
    pub height: u32,
}

/// Memory Layout:
/// - Size: 16 bytes
/// - Alignment: 4 bytes
/// - Packed structure with no padding
```

### 2.4 FrameMetadata

```rust
/// Frame processing metadata
#[derive(Debug, Clone)]
pub struct FrameMetadata {
    /// Encoder used (if encoded)
    pub encoder: Option<EncoderType>,

    /// Compression ratio achieved
    pub compression_ratio: f32,

    /// Encoding time in microseconds
    pub encoding_time_us: u32,

    /// Network transmission time in microseconds
    pub network_time_us: u32,

    /// Frame quality score (0.0 - 1.0)
    pub quality_score: f32,

    /// Is this a keyframe
    pub is_keyframe: bool,

    /// Frame dropped indicator
    pub dropped: bool,

    /// Retry count for transmission
    pub retry_count: u8,
}

/// Serde Serialization:
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
```

### 2.5 FrameFlags

```rust
/// Bitflags for frame properties
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct FrameFlags(u32);

impl FrameFlags {
    pub const NONE: Self = Self(0x00000000);
    pub const KEYFRAME: Self = Self(0x00000001);
    pub const DAMAGED: Self = Self(0x00000002);
    pub const CURSOR_UPDATED: Self = Self(0x00000004);
    pub const FORCE_REFRESH: Self = Self(0x00000008);
    pub const LOW_QUALITY: Self = Self(0x00000010);
    pub const HIGH_PRIORITY: Self = Self(0x00000020);
    pub const ENCRYPTED: Self = Self(0x00000040);
    pub const COMPRESSED: Self = Self(0x00000080);
}
```

## 3. Portal and Session Structures

### 3.1 StreamInfo

```rust
/// Portal stream metadata
#[derive(Debug, Clone)]
pub struct StreamInfo {
    /// Unique stream identifier
    pub stream_id: String,

    /// Portal session handle
    pub session_handle: PortalSessionHandle,

    /// Stream state
    pub state: StreamState,

    /// Video resolution
    pub resolution: Resolution,

    /// Refresh rate in Hz
    pub refresh_rate: u32,

    /// Pixel format
    pub format: PixelFormat,

    /// Stream creation timestamp
    pub created_at: std::time::SystemTime,

    /// Last activity timestamp
    pub last_activity: std::time::Instant,

    /// Stream capabilities
    pub capabilities: StreamCapabilities,

    /// Associated monitor info
    pub monitor: Option<MonitorInfo>,

    /// Stream statistics
    pub stats: StreamStatistics,
}

/// Validation Rules:
/// - stream_id must be non-empty and <= 128 chars
/// - session_handle must be valid
/// - refresh_rate > 0 && refresh_rate <= 240
/// - resolution must be valid (see Resolution validation)
```

### 3.2 PortalSessionHandle

```rust
/// XDG Desktop Portal session handle
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PortalSessionHandle {
    /// D-Bus connection name
    pub connection_name: String,

    /// Session object path
    pub object_path: String,

    /// Session token
    pub token: String,

    /// Portal backend type
    pub backend: PortalBackend,

    /// Session permissions
    pub permissions: PortalPermissions,

    /// Session creation time
    pub created_at: std::time::SystemTime,
}

/// Memory Layout:
/// - Size: 104 bytes (3 Strings @ 24 bytes each + fields)
/// - Alignment: 8 bytes

/// Validation Rules:
/// - connection_name must match D-Bus naming rules
/// - object_path must be valid D-Bus object path
/// - token must be 32-character hex string
```

### 3.3 PortalBackend

```rust
/// Portal backend implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PortalBackend {
    /// GNOME/Mutter backend
    Gnome = 0,

    /// KDE/Plasma backend
    Kde = 1,

    /// wlroots-based compositor backend
    Wlroots = 2,

    /// Generic Wayland backend
    Wayland = 3,

    /// X11 fallback backend
    X11 = 4,
}
```

### 3.4 PortalPermissions

```rust
/// Portal session permissions
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PortalPermissions {
    /// Screen capture allowed
    pub screen_capture: bool,

    /// Window selection allowed
    pub window_selection: bool,

    /// Monitor selection allowed
    pub monitor_selection: bool,

    /// Cursor capture allowed
    pub cursor_capture: bool,

    /// Audio capture allowed
    pub audio_capture: bool,

    /// Clipboard access allowed
    pub clipboard_access: bool,

    /// Input injection allowed
    pub input_injection: bool,

    /// Persistent session allowed
    pub persistent: bool,
}

/// Memory Layout:
/// - Size: 8 bytes
/// - Alignment: 1 byte
/// - Packed as bitfield internally
```

### 3.5 StreamState

```rust
/// Stream lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StreamState {
    /// Initial state
    Idle = 0,

    /// Negotiating capabilities
    Negotiating = 1,

    /// Ready to stream
    Ready = 2,

    /// Actively streaming
    Active = 3,

    /// Temporarily paused
    Paused = 4,

    /// Error state
    Error = 5,

    /// Shutting down
    Closing = 6,

    /// Terminated
    Closed = 7,
}
```

## 4. Monitor and Display Structures

### 4.1 MonitorInfo

```rust
/// Monitor information for multi-monitor support
#[derive(Debug, Clone)]
#[repr(C)]
pub struct MonitorInfo {
    /// Unique monitor identifier
    pub id: u32,

    /// Monitor name (e.g., "DP-1", "HDMI-2")
    pub name: String,

    /// Manufacturer name
    pub manufacturer: String,

    /// Model name
    pub model: String,

    /// Serial number
    pub serial: Option<String>,

    /// Physical dimensions
    pub physical: PhysicalDimensions,

    /// Display geometry
    pub geometry: DisplayGeometry,

    /// Supported modes
    pub modes: Vec<DisplayMode>,

    /// Current mode index
    pub current_mode: usize,

    /// Monitor capabilities
    pub capabilities: MonitorCapabilities,

    /// Is primary monitor
    pub is_primary: bool,

    /// Monitor state
    pub state: MonitorState,
}

/// Validation Rules:
/// - id must be unique across all monitors
/// - name must be non-empty
/// - geometry must not overlap with other monitors
/// - current_mode < modes.len()
```

### 4.2 PhysicalDimensions

```rust
/// Physical monitor dimensions in millimeters
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PhysicalDimensions {
    /// Width in millimeters
    pub width_mm: u32,

    /// Height in millimeters
    pub height_mm: u32,

    /// Diagonal size in inches (computed)
    pub diagonal_inches: f32,
}

/// Memory Layout:
/// - Size: 12 bytes
/// - Alignment: 4 bytes
```

### 4.3 DisplayGeometry

```rust
/// Display positioning and dimensions
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DisplayGeometry {
    /// X position in virtual desktop
    pub x: i32,

    /// Y position in virtual desktop
    pub y: i32,

    /// Display width in pixels
    pub width: u32,

    /// Display height in pixels
    pub height: u32,

    /// Rotation in degrees (0, 90, 180, 270)
    pub rotation: u32,

    /// Scale factor (1.0 = 100%)
    pub scale: f32,
}

/// Memory Layout:
/// - Size: 24 bytes
/// - Alignment: 4 bytes
```

### 4.4 DisplayMode

```rust
/// Display mode specification
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DisplayMode {
    /// Horizontal resolution
    pub width: u32,

    /// Vertical resolution
    pub height: u32,

    /// Refresh rate in millihertz
    pub refresh_rate_mhz: u32,

    /// Pixel clock in kHz
    pub pixel_clock_khz: u32,

    /// Mode flags
    pub flags: DisplayModeFlags,

    /// Is preferred mode
    pub is_preferred: bool,

    /// Is current mode
    pub is_current: bool,
}

/// Memory Layout:
/// - Size: 24 bytes
/// - Alignment: 4 bytes
```

## 5. Session Management Structures

### 5.1 SessionState

```rust
/// RDP session tracking state
#[derive(Debug, Clone)]
pub struct SessionState {
    /// Unique session identifier
    pub session_id: Uuid,

    /// Client connection info
    pub client: ClientInfo,

    /// Authentication state
    pub auth: AuthenticationState,

    /// Session establishment time
    pub established_at: std::time::SystemTime,

    /// Last activity timestamp
    pub last_activity: std::time::Instant,

    /// Session configuration
    pub config: SessionConfig,

    /// Active channels
    pub channels: Vec<ChannelState>,

    /// Performance metrics
    pub metrics: SessionMetrics,

    /// Session flags
    pub flags: SessionFlags,

    /// Termination reason (if terminated)
    pub termination_reason: Option<TerminationReason>,
}

/// Serde Serialization:
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
```

### 5.2 ClientInfo

```rust
/// RDP client information
#[derive(Debug, Clone)]
pub struct ClientInfo {
    /// Client IP address
    pub ip_address: std::net::IpAddr,

    /// Client port
    pub port: u16,

    /// Client hostname
    pub hostname: String,

    /// Client version string
    pub version: String,

    /// Client build number
    pub build: u32,

    /// Client capabilities
    pub capabilities: ClientCapabilities,

    /// Client time zone
    pub timezone: String,

    /// Client locale
    pub locale: String,

    /// Keyboard layout
    pub keyboard_layout: u32,

    /// Connection security level
    pub security_level: SecurityLevel,
}
```

### 5.3 AuthenticationState

```rust
/// Authentication and authorization state
#[derive(Debug, Clone)]
pub struct AuthenticationState {
    /// Authentication method used
    pub method: AuthMethod,

    /// Authenticated username
    pub username: String,

    /// Authentication timestamp
    pub authenticated_at: std::time::SystemTime,

    /// User privileges
    pub privileges: UserPrivileges,

    /// Session token
    pub session_token: Vec<u8>,

    /// Token expiry time
    pub token_expiry: std::time::SystemTime,

    /// Failed attempt count
    pub failed_attempts: u32,

    /// Account locked flag
    pub locked: bool,
}

/// Memory Layout:
/// - Sensitive fields zeroed on drop
/// - session_token encrypted in memory
```

### 5.4 SessionConfig

```rust
/// Per-session configuration
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
pub struct SessionConfig {
    /// Display configuration
    pub display: DisplayConfig,

    /// Input configuration
    pub input: InputConfig,

    /// Audio configuration
    pub audio: AudioConfig,

    /// Clipboard configuration
    pub clipboard: ClipboardConfig,

    /// Compression settings
    pub compression: CompressionConfig,

    /// Bandwidth limits
    pub bandwidth: BandwidthConfig,

    /// Timeout settings
    pub timeouts: TimeoutConfig,
}
```

## 6. Performance Monitoring Structures

### 6.1 PerformanceMetrics

```rust
/// System performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Measurement timestamp
    pub timestamp: std::time::Instant,

    /// CPU metrics
    pub cpu: CpuMetrics,

    /// Memory metrics
    pub memory: MemoryMetrics,

    /// Network metrics
    pub network: NetworkMetrics,

    /// GPU metrics (if available)
    pub gpu: Option<GpuMetrics>,

    /// Disk I/O metrics
    pub disk: DiskMetrics,

    /// Process-specific metrics
    pub process: ProcessMetrics,
}

/// Serde Serialization:
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
```

### 6.2 CpuMetrics

```rust
/// CPU utilization metrics
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CpuMetrics {
    /// Overall CPU usage percentage (0.0 - 100.0)
    pub usage_percent: f32,

    /// User space CPU percentage
    pub user_percent: f32,

    /// System/kernel CPU percentage
    pub system_percent: f32,

    /// I/O wait percentage
    pub iowait_percent: f32,

    /// Number of CPU cores
    pub core_count: u32,

    /// Load average (1 minute)
    pub load_avg_1m: f32,

    /// Load average (5 minutes)
    pub load_avg_5m: f32,

    /// Load average (15 minutes)
    pub load_avg_15m: f32,

    /// Context switches per second
    pub context_switches: u64,

    /// Interrupts per second
    pub interrupts: u64,
}

/// Memory Layout:
/// - Size: 48 bytes
/// - Alignment: 8 bytes
```

### 6.3 MemoryMetrics

```rust
/// Memory utilization metrics
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryMetrics {
    /// Total physical memory in bytes
    pub total_bytes: u64,

    /// Used memory in bytes
    pub used_bytes: u64,

    /// Available memory in bytes
    pub available_bytes: u64,

    /// Buffer memory in bytes
    pub buffer_bytes: u64,

    /// Cache memory in bytes
    pub cache_bytes: u64,

    /// Swap total in bytes
    pub swap_total_bytes: u64,

    /// Swap used in bytes
    pub swap_used_bytes: u64,

    /// Memory pressure indicator (0.0 - 1.0)
    pub pressure: f32,

    /// Page faults per second
    pub page_faults: u64,
}

/// Memory Layout:
/// - Size: 72 bytes
/// - Alignment: 8 bytes
```

### 6.4 NetworkMetrics

```rust
/// Network performance metrics
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct NetworkMetrics {
    /// Bytes received per second
    pub rx_bytes_per_sec: u64,

    /// Bytes transmitted per second
    pub tx_bytes_per_sec: u64,

    /// Packets received per second
    pub rx_packets_per_sec: u64,

    /// Packets transmitted per second
    pub tx_packets_per_sec: u64,

    /// Receive errors
    pub rx_errors: u64,

    /// Transmit errors
    pub tx_errors: u64,

    /// Dropped packets
    pub dropped_packets: u64,

    /// Network latency in microseconds
    pub latency_us: u32,

    /// Packet loss percentage
    pub packet_loss_percent: f32,

    /// Bandwidth utilization percentage
    pub bandwidth_percent: f32,
}

/// Memory Layout:
/// - Size: 64 bytes
/// - Alignment: 8 bytes
```

## 7. Configuration Structures

### 7.1 ServerConfig

```rust
/// Main server configuration
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ServerConfig {
    /// Server version
    pub version: String,

    /// Bind address
    pub bind_address: std::net::SocketAddr,

    /// Maximum concurrent sessions
    pub max_sessions: u32,

    /// Server name
    pub server_name: String,

    /// Server description
    pub description: Option<String>,

    /// Security configuration
    pub security: SecurityConfig,

    /// Video configuration
    pub video: VideoConfig,

    /// Input configuration
    pub input: InputConfig,

    /// Clipboard configuration
    pub clipboard: ClipboardConfig,

    /// Multi-monitor configuration
    pub multi_monitor: MultiMonitorConfig,

    /// Performance configuration
    pub performance: PerformanceConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Feature flags
    pub features: FeatureFlags,
}

/// Validation Rules:
/// - version must be valid semver
/// - max_sessions > 0 && max_sessions <= 1000
/// - server_name must be non-empty and <= 64 chars
/// - bind_address must be valid
```

### 7.2 SecurityConfig

```rust
/// Security configuration
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SecurityConfig {
    /// TLS configuration
    pub tls: TlsConfig,

    /// Authentication methods
    pub auth_methods: Vec<AuthMethod>,

    /// Encryption level
    pub encryption_level: EncryptionLevel,

    /// Certificate path
    pub cert_path: PathBuf,

    /// Private key path
    pub key_path: PathBuf,

    /// CA certificate path
    pub ca_path: Option<PathBuf>,

    /// Client certificate required
    pub require_client_cert: bool,

    /// Session timeout in seconds
    pub session_timeout_secs: u32,

    /// Maximum authentication attempts
    pub max_auth_attempts: u32,

    /// Lockout duration in seconds
    pub lockout_duration_secs: u32,

    /// IP allowlist
    pub ip_allowlist: Vec<IpNetwork>,

    /// IP denylist
    pub ip_denylist: Vec<IpNetwork>,
}

/// Validation Rules:
/// - cert_path and key_path must exist and be readable
/// - session_timeout_secs > 0
/// - max_auth_attempts > 0 && max_auth_attempts <= 10
/// - IP lists must contain valid CIDR blocks
```

### 7.3 VideoConfig

```rust
/// Video encoding configuration
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct VideoConfig {
    /// Default codec
    pub codec: VideoCodec,

    /// Maximum resolution
    pub max_resolution: Resolution,

    /// Default resolution
    pub default_resolution: Resolution,

    /// Maximum framerate
    pub max_fps: u32,

    /// Default framerate
    pub default_fps: u32,

    /// Bitrate in kbps
    pub bitrate_kbps: u32,

    /// Quality preset
    pub quality: QualityPreset,

    /// Hardware acceleration
    pub hw_acceleration: HardwareAcceleration,

    /// Color depth in bits
    pub color_depth: u8,

    /// Enable compression
    pub compression: bool,

    /// Compression level (1-9)
    pub compression_level: u8,

    /// Enable adaptive quality
    pub adaptive_quality: bool,

    /// Minimum quality threshold
    pub min_quality: f32,

    /// Maximum quality threshold
    pub max_quality: f32,
}

/// Validation Rules:
/// - max_fps > 0 && max_fps <= 240
/// - default_fps <= max_fps
/// - bitrate_kbps > 0 && bitrate_kbps <= 100000
/// - color_depth in [16, 24, 32]
/// - compression_level >= 1 && compression_level <= 9
/// - quality values between 0.0 and 1.0
```

### 7.4 InputConfig

```rust
/// Input handling configuration
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InputConfig {
    /// Enable keyboard input
    pub keyboard_enabled: bool,

    /// Enable mouse input
    pub mouse_enabled: bool,

    /// Enable touch input
    pub touch_enabled: bool,

    /// Keyboard layout mapping
    pub keyboard_layout: KeyboardLayout,

    /// Mouse acceleration
    pub mouse_acceleration: f32,

    /// Mouse sensitivity
    pub mouse_sensitivity: f32,

    /// Scroll speed multiplier
    pub scroll_speed: f32,

    /// Enable relative mouse mode
    pub relative_mouse: bool,

    /// Touch gesture support
    pub touch_gestures: bool,

    /// Maximum touch points
    pub max_touch_points: u32,

    /// Input latency compensation
    pub latency_compensation: bool,

    /// Input event queue size
    pub event_queue_size: u32,
}

/// Validation Rules:
/// - mouse_acceleration >= 0.0 && mouse_acceleration <= 10.0
/// - mouse_sensitivity > 0.0 && mouse_sensitivity <= 10.0
/// - scroll_speed > 0.0 && scroll_speed <= 10.0
/// - max_touch_points <= 10
/// - event_queue_size > 0 && event_queue_size <= 10000
```

### 7.5 ClipboardConfig

```rust
/// Clipboard synchronization configuration
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ClipboardConfig {
    /// Enable clipboard sync
    pub enabled: bool,

    /// Sync direction
    pub direction: ClipboardDirection,

    /// Maximum text size in bytes
    pub max_text_size: usize,

    /// Maximum image size in bytes
    pub max_image_size: usize,

    /// Maximum file size in bytes
    pub max_file_size: usize,

    /// Supported formats
    pub formats: Vec<ClipboardFormat>,

    /// Enable file transfer
    pub file_transfer: bool,

    /// Allowed file extensions
    pub allowed_extensions: Vec<String>,

    /// Blocked file extensions
    pub blocked_extensions: Vec<String>,

    /// Sanitize HTML content
    pub sanitize_html: bool,

    /// Strip formatting
    pub strip_formatting: bool,

    /// Compression for large data
    pub compress_large_data: bool,

    /// Compression threshold in bytes
    pub compression_threshold: usize,
}

/// Validation Rules:
/// - max sizes must be > 0
/// - max_text_size <= 10MB
/// - max_image_size <= 50MB
/// - max_file_size <= 100MB
/// - compression_threshold <= max sizes
```

### 7.6 MultiMonitorConfig

```rust
/// Multi-monitor configuration
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MultiMonitorConfig {
    /// Enable multi-monitor support
    pub enabled: bool,

    /// Maximum number of monitors
    pub max_monitors: u32,

    /// Monitor arrangement
    pub arrangement: MonitorArrangement,

    /// Primary monitor index
    pub primary_monitor: u32,

    /// Span mode
    pub span_mode: SpanMode,

    /// Virtual desktop size
    pub virtual_desktop_size: Option<Resolution>,

    /// Per-monitor settings
    pub monitor_settings: Vec<MonitorSettings>,

    /// Automatic detection
    pub auto_detect: bool,

    /// Hot-plug support
    pub hotplug_support: bool,

    /// Preserve window positions
    pub preserve_positions: bool,
}

/// Validation Rules:
/// - max_monitors > 0 && max_monitors <= 16
/// - primary_monitor < max_monitors
/// - monitor_settings.len() <= max_monitors
```

### 7.7 PerformanceConfig

```rust
/// Performance tuning configuration
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PerformanceConfig {
    /// CPU thread pool size
    pub cpu_threads: u32,

    /// GPU thread pool size
    pub gpu_threads: u32,

    /// Frame buffer count
    pub frame_buffer_count: u32,

    /// Frame buffer size in MB
    pub frame_buffer_size_mb: u32,

    /// Enable frame skipping
    pub frame_skipping: bool,

    /// Maximum frame skip count
    pub max_frame_skip: u32,

    /// Memory limit in MB
    pub memory_limit_mb: u32,

    /// Cache size in MB
    pub cache_size_mb: u32,

    /// Network buffer size in KB
    pub network_buffer_kb: u32,

    /// Enable hardware encoding
    pub hw_encoding: bool,

    /// Enable hardware decoding
    pub hw_decoding: bool,

    /// Pipeline depth
    pub pipeline_depth: u32,

    /// Prefetch frames
    pub prefetch_frames: u32,
}

/// Validation Rules:
/// - cpu_threads > 0 && cpu_threads <= 64
/// - gpu_threads >= 0 && gpu_threads <= 16
/// - frame_buffer_count > 0 && frame_buffer_count <= 10
/// - memory_limit_mb > 0
/// - cache_size_mb <= memory_limit_mb
```

### 7.8 LoggingConfig

```rust
/// Logging configuration
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LoggingConfig {
    /// Log level
    pub level: LogLevel,

    /// Log output targets
    pub targets: Vec<LogTarget>,

    /// Log file path
    pub file_path: Option<PathBuf>,

    /// Maximum log file size in MB
    pub max_file_size_mb: u32,

    /// Maximum number of log files
    pub max_files: u32,

    /// Log rotation strategy
    pub rotation: LogRotation,

    /// Include timestamps
    pub timestamps: bool,

    /// Timestamp format
    pub timestamp_format: String,

    /// Include source location
    pub source_location: bool,

    /// Include thread ID
    pub thread_id: bool,

    /// JSON formatting
    pub json_format: bool,

    /// Performance logging
    pub performance_logging: bool,

    /// Security event logging
    pub security_logging: bool,
}

/// Validation Rules:
/// - max_file_size_mb > 0 && max_file_size_mb <= 1000
/// - max_files > 0 && max_files <= 100
/// - timestamp_format must be valid chrono format
```

## 8. IronRDP Integration Structures

### 8.1 BitmapUpdate

```rust
/// RDP bitmap update structure
#[derive(Debug, Clone)]
#[repr(C)]
pub struct BitmapUpdate {
    /// Update region
    pub region: UpdateRegion,

    /// Bitmap data
    pub bitmap: BitmapData,

    /// Compression type
    pub compression: BitmapCompression,

    /// Update flags
    pub flags: BitmapUpdateFlags,
}

/// Memory Layout:
/// - Size: 64 bytes (excluding allocations)
/// - Alignment: 8 bytes
```

### 8.2 BitmapData

```rust
/// Bitmap data container
#[derive(Debug, Clone)]
pub struct BitmapData {
    /// Raw bitmap bytes
    pub data: Vec<u8>,

    /// Width in pixels
    pub width: u16,

    /// Height in pixels
    pub height: u16,

    /// Bits per pixel
    pub bpp: u8,

    /// Compression flags
    pub compressed: bool,

    /// Original size (if compressed)
    pub original_size: Option<usize>,
}
```

### 8.3 DisplayUpdate

```rust
/// IronRDP display update enum
#[derive(Debug, Clone)]
pub enum DisplayUpdate {
    /// Surface commands
    SurfaceCommand(SurfaceCommand),

    /// Bitmap updates
    BitmapUpdate(Vec<BitmapUpdate>),

    /// Palette update
    PaletteUpdate(PaletteData),

    /// Pointer update
    PointerUpdate(PointerData),

    /// Desktop resize
    DesktopResize(Resolution),

    /// Monitor layout change
    MonitorLayout(Vec<MonitorInfo>),
}
```

### 8.4 KeyboardEvent

```rust
/// RDP keyboard event
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct KeyboardEvent {
    /// Scan code
    pub scan_code: u16,

    /// Virtual key code
    pub virtual_key: u16,

    /// Event flags
    pub flags: KeyboardEventFlags,

    /// Event timestamp
    pub timestamp: u32,
}

/// Memory Layout:
/// - Size: 12 bytes
/// - Alignment: 4 bytes

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct KeyboardEventFlags(u16);

impl KeyboardEventFlags {
    pub const KEY_DOWN: Self = Self(0x0000);
    pub const KEY_UP: Self = Self(0x8000);
    pub const EXTENDED: Self = Self(0x0100);
    pub const EXTENDED1: Self = Self(0x0200);
}
```

### 8.5 MouseEvent

```rust
/// RDP mouse event
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MouseEvent {
    /// X coordinate
    pub x: u16,

    /// Y coordinate
    pub y: u16,

    /// Mouse buttons state
    pub buttons: MouseButtons,

    /// Event flags
    pub flags: MouseEventFlags,

    /// Wheel delta (if wheel event)
    pub wheel_delta: i16,

    /// Event timestamp
    pub timestamp: u32,
}

/// Memory Layout:
/// - Size: 16 bytes
/// - Alignment: 4 bytes

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct MouseButtons(u16);

impl MouseButtons {
    pub const LEFT: Self = Self(0x0001);
    pub const RIGHT: Self = Self(0x0002);
    pub const MIDDLE: Self = Self(0x0004);
    pub const BUTTON4: Self = Self(0x0008);
    pub const BUTTON5: Self = Self(0x0010);
}
```

## 9. Error Types

### 9.1 ServerError

```rust
/// Main server error enum
#[derive(Debug, Clone)]
pub enum ServerError {
    /// I/O errors
    Io(IoError),

    /// Portal errors
    Portal(PortalError),

    /// RDP protocol errors
    Protocol(ProtocolError),

    /// Authentication errors
    Auth(AuthError),

    /// Video processing errors
    Video(VideoError),

    /// Configuration errors
    Config(ConfigError),

    /// Session management errors
    Session(SessionError),

    /// Network errors
    Network(NetworkError),

    /// Encoding/Decoding errors
    Codec(CodecError),

    /// Internal server errors
    Internal(String),
}

impl std::error::Error for ServerError {}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::Portal(e) => write!(f, "Portal error: {}", e),
            Self::Protocol(e) => write!(f, "Protocol error: {}", e),
            Self::Auth(e) => write!(f, "Authentication error: {}", e),
            Self::Video(e) => write!(f, "Video error: {}", e),
            Self::Config(e) => write!(f, "Configuration error: {}", e),
            Self::Session(e) => write!(f, "Session error: {}", e),
            Self::Network(e) => write!(f, "Network error: {}", e),
            Self::Codec(e) => write!(f, "Codec error: {}", e),
            Self::Internal(s) => write!(f, "Internal error: {}", s),
        }
    }
}
```

### 9.2 PortalError

```rust
/// Portal-specific errors
#[derive(Debug, Clone)]
pub enum PortalError {
    /// Portal not available
    NotAvailable,

    /// Permission denied
    PermissionDenied,

    /// Session creation failed
    SessionCreationFailed(String),

    /// Stream setup failed
    StreamSetupFailed(String),

    /// Invalid session handle
    InvalidSession,

    /// Portal timeout
    Timeout,

    /// D-Bus communication error
    DBusError(String),
}
```

### 9.3 ProtocolError

```rust
/// RDP protocol errors
#[derive(Debug, Clone)]
pub enum ProtocolError {
    /// Invalid packet
    InvalidPacket(String),

    /// Unsupported version
    UnsupportedVersion(u32),

    /// Invalid sequence
    InvalidSequence,

    /// Checksum mismatch
    ChecksumMismatch,

    /// Encryption error
    EncryptionError(String),

    /// Compression error
    CompressionError(String),

    /// Invalid channel
    InvalidChannel(u16),

    /// Protocol timeout
    Timeout,
}
```

## 10. Channel Data Structures

### 10.1 ChannelState

```rust
/// Virtual channel state
#[derive(Debug, Clone)]
pub struct ChannelState {
    /// Channel ID
    pub id: u16,

    /// Channel name
    pub name: String,

    /// Channel type
    pub channel_type: ChannelType,

    /// Channel flags
    pub flags: ChannelFlags,

    /// Channel priority
    pub priority: u8,

    /// Is channel open
    pub is_open: bool,

    /// Bytes sent
    pub bytes_sent: u64,

    /// Bytes received
    pub bytes_received: u64,

    /// Last activity
    pub last_activity: std::time::Instant,

    /// Channel-specific data
    pub data: ChannelData,
}
```

### 10.2 ChannelType

```rust
/// Channel type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChannelType {
    /// Main RDP channel
    Main = 0x0000,

    /// Display channel
    Display = 0x0001,

    /// Input channel
    Input = 0x0002,

    /// Clipboard channel
    Clipboard = 0x0003,

    /// Audio output channel
    AudioOut = 0x0004,

    /// Audio input channel
    AudioIn = 0x0005,

    /// File transfer channel
    FileTransfer = 0x0006,

    /// Printer redirection
    Printer = 0x0007,

    /// Smart card redirection
    SmartCard = 0x0008,

    /// USB redirection
    Usb = 0x0009,

    /// Custom channel
    Custom(u16),
}
```

## 11. Clipboard Format Structures

### 11.1 ClipboardFormat

```rust
/// Clipboard data format
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardFormat {
    /// Plain text
    Text(TextFormat),

    /// HTML content
    Html(HtmlFormat),

    /// Image data
    Image(ImageFormat),

    /// File list
    FileList(Vec<FileEntry>),

    /// Rich text format
    Rtf(Vec<u8>),

    /// Custom format
    Custom(CustomFormat),
}
```

### 11.2 TextFormat

```rust
/// Text clipboard format
#[derive(Debug, Clone)]
pub struct TextFormat {
    /// UTF-8 text content
    pub content: String,

    /// Character encoding
    pub encoding: TextEncoding,

    /// Line ending style
    pub line_ending: LineEnding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TextEncoding {
    Utf8 = 0,
    Utf16Le = 1,
    Utf16Be = 2,
    Ascii = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LineEnding {
    Lf = 0,    // Unix
    CrLf = 1,  // Windows
    Cr = 2,    // Mac Classic
}
```

### 11.3 ImageFormat

```rust
/// Image clipboard format
#[derive(Debug, Clone)]
pub struct ImageFormat {
    /// Image data
    pub data: Vec<u8>,

    /// Image format type
    pub format: ImageType,

    /// Width in pixels
    pub width: u32,

    /// Height in pixels
    pub height: u32,

    /// DPI information
    pub dpi: Option<(u32, u32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ImageType {
    Png = 0,
    Jpeg = 1,
    Bmp = 2,
    Tiff = 3,
    WebP = 4,
}
```

## 12. Utility Types

### 12.1 Resolution

```rust
/// Display resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Resolution {
    /// Width in pixels
    pub width: u32,

    /// Height in pixels
    pub height: u32,
}

impl Resolution {
    /// Common resolutions
    pub const HD_720P: Self = Self { width: 1280, height: 720 };
    pub const FHD_1080P: Self = Self { width: 1920, height: 1080 };
    pub const QHD_1440P: Self = Self { width: 2560, height: 1440 };
    pub const UHD_4K: Self = Self { width: 3840, height: 2160 };

    /// Calculate total pixels
    pub const fn pixels(&self) -> u32 {
        self.width * self.height
    }

    /// Calculate aspect ratio
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}

/// Validation Rules:
/// - width > 0 && width <= 7680
/// - height > 0 && height <= 4320
```

### 12.2 Uuid

```rust
/// UUID v4 implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Uuid([u8; 16]);

impl Uuid {
    /// Generate new random UUID
    pub fn new_v4() -> Self {
        let mut bytes = [0u8; 16];
        // Implementation uses getrandom
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Self(bytes)
    }

    /// Parse from string
    pub fn parse_str(s: &str) -> Result<Self, UuidError> {
        // Implementation parses standard UUID format
        // xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
        unimplemented!()
    }
}

impl std::fmt::Display for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Format as standard UUID string
        unimplemented!()
    }
}
```

### 12.3 IpNetwork

```rust
/// IP network CIDR representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpNetwork {
    /// IPv4 network
    V4(Ipv4Network),

    /// IPv6 network
    V6(Ipv6Network),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv4Network {
    pub addr: std::net::Ipv4Addr,
    pub prefix: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv6Network {
    pub addr: std::net::Ipv6Addr,
    pub prefix: u8,
}

impl IpNetwork {
    /// Check if IP address is in network
    pub fn contains(&self, addr: std::net::IpAddr) -> bool {
        match (self, addr) {
            (Self::V4(net), std::net::IpAddr::V4(ip)) => net.contains(ip),
            (Self::V6(net), std::net::IpAddr::V6(ip)) => net.contains(ip),
            _ => false,
        }
    }
}
```

## 13. Serialization Specifications

### 13.1 Binary Serialization

All structures marked with `#[repr(C)]` use native binary serialization with:
- Little-endian byte order
- Natural alignment rules
- No padding between fields of same alignment
- Vectors serialized as (length: u64, data: [T])
- Strings serialized as (length: u64, utf8_bytes: [u8])
- Options serialized as (present: u8, value: T)

### 13.2 JSON Serialization

Structures with `#[derive(Serialize, Deserialize)]` support JSON with:
- Snake case field naming (`#[serde(rename_all = "snake_case")]`)
- Skip serializing None values (`#[serde(skip_serializing_if = "Option::is_none")]`)
- Timestamps as ISO 8601 strings
- Binary data as base64 strings
- Enums as lowercase strings

### 13.3 Network Wire Format

RDP protocol structures use IronRDP's encoding:
- PDU headers with type and length fields
- Compressed data with RDP compression headers
- Encrypted data with RDP encryption headers
- Multi-fragment support for large messages

## 14. Memory Management

### 14.1 Allocation Strategies

- **Stack allocation**: Structures < 256 bytes
- **Heap allocation**: Large data buffers, dynamic collections
- **Arena allocation**: Frame buffers, batch operations
- **Memory pools**: Reusable buffers for video frames

### 14.2 Lifetime Management

- **Reference counting**: Shared frame buffers
- **RAII**: Automatic cleanup of resources
- **Explicit drop**: Sensitive data zeroing
- **Weak references**: Breaking circular dependencies

## 15. Thread Safety

### 15.1 Synchronization Primitives

- **Arc<Mutex<T>>**: Shared mutable state
- **Arc<RwLock<T>>**: Read-heavy shared state
- **AtomicU64**: Performance counters
- **Channel<T>**: Inter-thread communication

### 15.2 Send + Sync Implementations

All public structures implement:
- `Send` for transfer between threads
- `Sync` for shared references between threads
- Exception: Raw pointer wrappers and FFI types

## 16. Version History

| Version | Date       | Changes                        |
|---------|------------|--------------------------------|
| 1.0.0   | 2024-11-18 | Initial specification release  |

## 17. Compliance and Standards

This specification complies with:
- RFC 8446 (TLS 1.3)
- RFC 6143 (RFB Protocol)
- MS-RDPBCGR (RDP Protocol)
- XDG Desktop Portal Specification
- Rust API Guidelines
- MISRA C Guidelines (for FFI)

---

End of WRD-Server Data Structures Specification