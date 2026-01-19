# GUI Software Design Specification
# lamco-rdp-server Configuration GUI

**Document Version:** 1.0
**Date:** 2026-01-19
**Author:** System Architect
**Status:** Draft - Ready for Implementation

---

## Document Purpose

This specification provides **complete, standalone implementation guidance** for adding a comprehensive configuration GUI to lamco-rdp-server using the iced framework. A developer in a new session should be able to implement the entire GUI from this document alone.

**Target Audience:** Rust developers familiar with async programming and GUI concepts
**Implementation Time:** 6-8 weeks for full implementation
**Framework:** iced 0.12+ (Elm Architecture pattern)

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Configuration Analysis](#configuration-analysis)
3. [GUI Architecture](#gui-architecture)
4. [State Management](#state-management)
5. [Tab Specifications](#tab-specifications)
6. [Service Registry Integration](#service-registry-integration)
7. [IPC Design](#ipc-design)
8. [File Operations](#file-operations)
9. [Hardware Detection](#hardware-detection)
10. [Certificate Generation](#certificate-generation)
11. [Implementation Guide](#implementation-guide)
12. [Code Structure](#code-structure)
13. [Testing Strategy](#testing-strategy)

---

## 1. Architecture Overview

### 1.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    lamco-rdp-server (main)                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │   Config     │  │  RDP Server  │  │  Service Registry    │   │
│  │   Loader     │  │   Runtime    │  │   (18 services)      │   │
│  └──────────────┘  └──────────────┘  └──────────────────────┘   │
│         │                  │                      │               │
│         │ config.toml      │ IPC Messages         │               │
│         └──────────────────┴──────────────────────┘               │
└──────────────────────────┬───────────────────────────────────────┘
                           │ Unix Domain Socket / D-Bus
                           │ (Optional IPC - GUI can run independently)
┌──────────────────────────┴───────────────────────────────────────┐
│            lamco-rdp-server-gui (separate binary)                │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                    iced Application                          │ │
│  │  ┌────────┬────────┬────────┬────────┬────────┬──────────┐  │ │
│  │  │ Server │Security│ Video  │ Input  │Clipbd  │  Logging │  │ │
│  │  │  Tab   │  Tab   │  Tab   │  Tab   │  Tab   │   Tab    │  │ │
│  │  ├────────┼────────┼────────┼────────┼────────┼──────────┤  │ │
│  │  │  Perf  │ EGFX   │ Damage │Hardware│Display │  Cursor  │  │ │
│  │  │  Tab   │  Tab   │  Tab   │  Tab   │  Tab   │   Tab    │  │ │
│  │  └────────┴────────┴────────┴────────┴────────┴──────────┘  │ │
│  │  ┌──────────────────────────────────────────────────────┐   │ │
│  │  │            Service Registry View (Read-only)          │   │ │
│  │  └──────────────────────────────────────────────────────┘   │ │
│  │  ┌──────────────────────────────────────────────────────┐   │ │
│  │  │        Log Viewer (tail -f style, live updates)       │   │ │
│  │  └──────────────────────────────────────────────────────┘   │ │
│  └─────────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │             State Management (AppState)                      │ │
│  │  • Config (80+ parameters)                                   │ │
│  │  • Validation State                                          │ │
│  │  • UI State (active tab, selections, etc.)                   │ │
│  │  • Server Status (running/stopped, connections)              │ │
│  └─────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

### 1.2 GUI-Optional Architecture

**Design Principle:** The GUI is an **optional, separate binary** that complements the CLI server.

**Key Properties:**
1. **Independence:** GUI can run without server, server can run without GUI
2. **File-Based Communication:** Primary interface is config.toml read/write
3. **Optional IPC:** Real-time status updates via Unix socket (if server running)
4. **No Code Coupling:** Server code unchanged, GUI imports library types only

**Binary Structure:**
```
lamco-rdp-server/
├── src/
│   ├── main.rs              # CLI server binary
│   ├── lib.rs               # Library (Config, types, utils)
│   └── gui/                 # GUI module (separate binary)
│       ├── main.rs          # GUI entry point
│       ├── app.rs           # iced Application
│       ├── state.rs         # State management
│       ├── tabs/            # Tab implementations
│       ├── widgets/         # Custom widgets
│       ├── ipc.rs           # Server communication
│       └── theme.rs         # Styling
├── Cargo.toml
│   [[bin]]
│   name = "lamco-rdp-server"      # CLI server
│   path = "src/main.rs"
│
│   [[bin]]
│   name = "lamco-rdp-server-gui"  # GUI binary
│   path = "src/gui/main.rs"
```

---

## 2. Configuration Analysis

### 2.1 Configuration Structure Overview

**Source:** `src/config/types.rs` (941 lines)
**Main Struct:** `Config` with 14 sub-configuration structs
**Total Parameters:** 85+ configuration fields

### 2.2 Complete Configuration Hierarchy

```rust
pub struct Config {
    pub server: ServerConfig,                    // 4 fields
    pub security: SecurityConfig,                // 5 fields
    pub video: VideoConfig,                      // 6 fields
    pub video_pipeline: VideoPipelineConfig,     // 3 sub-structs
    pub input: InputConfig,                      // 3 fields
    pub clipboard: ClipboardConfig,              // 4 fields
    pub multimon: MultiMonitorConfig,            // 2 fields
    pub performance: PerformanceConfig,          // 6 fields + 2 sub-structs
    pub logging: LoggingConfig,                  // 3 fields
    pub egfx: EgfxConfig,                        // 23 fields
    pub damage_tracking: DamageTrackingConfig,   // 7 fields
    pub hardware_encoding: HardwareEncodingConfig, // 6 fields
    pub display: DisplayConfig,                  // 4 fields
    pub advanced_video: AdvancedVideoConfig,     // 4 fields
    pub cursor: CursorConfig,                    // 5 fields + 1 sub-struct
}
```

### 2.3 Detailed Field Inventory

#### ServerConfig (4 fields)
```rust
pub struct ServerConfig {
    pub listen_addr: String,           // Socket address (e.g., "0.0.0.0:3389")
    pub max_connections: usize,        // Maximum concurrent clients
    pub session_timeout: u64,          // Timeout in seconds (0 = no timeout)
    pub use_portals: bool,             // Use XDG Desktop Portals
}
```

**GUI Requirements:**
- Text input for listen_addr with validation (IP:port format)
- Number input for max_connections (range: 1-100)
- Number input for session_timeout (0 = disabled)
- Toggle for use_portals with explanation tooltip

#### SecurityConfig (5 fields)
```rust
pub struct SecurityConfig {
    pub cert_path: PathBuf,            // TLS certificate file
    pub key_path: PathBuf,             // TLS private key file
    pub enable_nla: bool,              // Network Level Authentication
    pub auth_method: String,           // "pam" or "none"
    pub require_tls_13: bool,          // Require TLS 1.3+
}
```

**GUI Requirements:**
- File picker for cert_path
- File picker for key_path
- Button: "Generate Self-Signed Certificate"
- Toggle for enable_nla
- Dropdown for auth_method: ["pam", "none"]
- Toggle for require_tls_13

#### VideoConfig (6 fields)
```rust
pub struct VideoConfig {
    pub encoder: String,               // "vaapi", "openh264", "auto"
    pub vaapi_device: PathBuf,         // VA-API device (e.g., /dev/dri/renderD128)
    pub target_fps: u32,               // Target frames per second
    pub bitrate: u32,                  // Video bitrate in kbps
    pub damage_tracking: bool,         // Enable damage tracking
    pub cursor_mode: String,           // "embedded", "metadata", "hidden"
}
```

**GUI Requirements:**
- Dropdown for encoder with auto-detection
- File picker for vaapi_device (filtered to /dev/dri/render*)
- Slider for target_fps (5-60, default 30)
- Slider for bitrate (1000-20000 kbps)
- Toggle for damage_tracking
- Dropdown for cursor_mode

#### VideoPipelineConfig (3 sub-structs, 16 total fields)
```rust
pub struct VideoPipelineConfig {
    pub processor: ProcessorConfig,    // 6 fields
    pub dispatcher: DispatcherConfig,  // 7 fields
    pub converter: ConverterConfig,    // 3 fields
}

pub struct ProcessorConfig {
    pub target_fps: u32,
    pub max_queue_depth: usize,
    pub adaptive_quality: bool,
    pub damage_threshold: f32,         // 0.0-1.0
    pub drop_on_full_queue: bool,
    pub enable_metrics: bool,
}

pub struct DispatcherConfig {
    pub channel_size: usize,
    pub priority_dispatch: bool,
    pub max_frame_age_ms: u64,
    pub enable_backpressure: bool,
    pub high_water_mark: f32,          // 0.0-1.0
    pub low_water_mark: f32,           // 0.0-1.0
    pub load_balancing: bool,
}

pub struct ConverterConfig {
    pub buffer_pool_size: usize,
    pub enable_simd: bool,
    pub damage_threshold: f32,         // 0.0-1.0
    pub enable_statistics: bool,
}
```

**GUI Requirements:**
- Collapsible section for advanced pipeline settings
- Number inputs with appropriate ranges
- Float sliders for thresholds (0.0-1.0)
- Toggle switches for boolean flags

#### InputConfig (3 fields)
```rust
pub struct InputConfig {
    pub use_libei: bool,               // Use libei for input injection
    pub keyboard_layout: String,       // "auto" or XKB layout name
    pub enable_touch: bool,            // Enable touch input
}
```

**GUI Requirements:**
- Toggle for use_libei
- Dropdown for keyboard_layout (auto + common layouts)
- Toggle for enable_touch

#### ClipboardConfig (4 fields)
```rust
pub struct ClipboardConfig {
    pub enabled: bool,
    pub max_size: usize,               // Maximum clipboard size in bytes
    pub rate_limit_ms: u64,            // Rate limiting (ms between events)
    pub allowed_types: Vec<String>,    // MIME types (empty = all)
}
```

**GUI Requirements:**
- Toggle for enabled
- Number input for max_size (with MB conversion display)
- Number input for rate_limit_ms
- Multi-line text for allowed_types (one per line)

#### MultiMonitorConfig (2 fields)
```rust
pub struct MultiMonitorConfig {
    pub enabled: bool,
    pub max_monitors: usize,
}
```

**GUI Requirements:**
- Toggle for enabled
- Number input for max_monitors (1-8)

#### PerformanceConfig (6 fields + 2 sub-structs)
```rust
pub struct PerformanceConfig {
    pub encoder_threads: usize,        // 0 = auto
    pub network_threads: usize,        // 0 = auto
    pub buffer_pool_size: usize,
    pub zero_copy: bool,
    pub adaptive_fps: AdaptiveFpsConfig,     // 6 fields
    pub latency: LatencyConfig,              // 6 fields
}

pub struct AdaptiveFpsConfig {
    pub enabled: bool,
    pub min_fps: u32,
    pub max_fps: u32,
    pub high_activity_threshold: f32,  // 0.0-1.0
    pub medium_activity_threshold: f32,
    pub low_activity_threshold: f32,
}

pub struct LatencyConfig {
    pub mode: String,                  // "interactive", "balanced", "quality"
    pub interactive_max_delay_ms: u32,
    pub balanced_max_delay_ms: u32,
    pub quality_max_delay_ms: u32,
    pub balanced_damage_threshold: f32,
    pub quality_damage_threshold: f32,
}
```

**GUI Requirements:**
- Number inputs with "0 = auto" help text
- Toggle for zero_copy
- Collapsible sections for adaptive_fps and latency
- Preset buttons: "Interactive", "Balanced", "Quality"

#### LoggingConfig (3 fields)
```rust
pub struct LoggingConfig {
    pub level: String,                 // "trace", "debug", "info", "warn", "error"
    pub log_dir: Option<PathBuf>,      // None = console only
    pub metrics: bool,
}
```

**GUI Requirements:**
- Dropdown for level
- Optional file picker for log_dir
- Toggle for metrics

#### EgfxConfig (23 fields) - Most Complex
```rust
pub struct EgfxConfig {
    pub enabled: bool,
    pub h264_level: String,            // "auto" or "3.0", "3.1", "4.0", etc.
    pub h264_bitrate: u32,             // kbps
    pub zgfx_compression: String,      // "never", "auto", "always"
    pub max_frames_in_flight: u32,
    pub frame_ack_timeout: u64,        // milliseconds
    pub periodic_idr_interval: u32,    // seconds, 0 = disabled
    pub codec: String,                 // "auto", "avc420", "avc444"
    pub qp_min: u8,
    pub qp_max: u8,
    pub qp_default: u8,

    // AVC444-specific
    pub avc444_aux_bitrate_ratio: f32,       // 0.3-1.0
    pub color_matrix: String,                // "auto", "openh264", "bt709", "bt601", "srgb"
    pub color_range: String,                 // "auto", "limited", "full"
    pub avc444_enabled: bool,
    pub avc444_enable_aux_omission: bool,
    pub avc444_max_aux_interval: u32,
    pub avc444_aux_change_threshold: f32,    // 0.0-1.0
    pub avc444_force_aux_idr_on_return: bool,
}
```

**GUI Requirements:**
- Organized into sections: Basic, Quality, AVC444
- Expert mode toggle (hides advanced AVC444 params)
- Preset buttons: "Speed", "Balanced", "Quality"
- Help tooltips for every parameter

#### DamageTrackingConfig (7 fields)
```rust
pub struct DamageTrackingConfig {
    pub enabled: bool,
    pub method: String,                // "pipewire", "diff", "hybrid"
    pub tile_size: usize,              // pixels
    pub diff_threshold: f32,           // 0.0-1.0
    pub pixel_threshold: u8,
    pub merge_distance: u32,           // pixels
    pub min_region_area: u64,          // pixels²
}
```

**GUI Requirements:**
- Toggle for enabled
- Dropdown for method
- Number inputs with sensible ranges
- Preset buttons: "Text Work", "General", "Video"

#### HardwareEncodingConfig (6 fields)
```rust
pub struct HardwareEncodingConfig {
    pub enabled: bool,
    pub vaapi_device: PathBuf,
    pub enable_dmabuf_zerocopy: bool,
    pub fallback_to_software: bool,
    pub quality_preset: String,        // "speed", "balanced", "quality"
    pub prefer_nvenc: bool,
}
```

**GUI Requirements:**
- Toggle for enabled
- Device picker with auto-detection
- Toggle for enable_dmabuf_zerocopy
- Toggle for fallback_to_software
- Dropdown for quality_preset
- Toggle for prefer_nvenc

#### DisplayConfig (4 fields)
```rust
pub struct DisplayConfig {
    pub allow_resize: bool,
    pub allowed_resolutions: Vec<String>,  // e.g., ["1920x1080", "1280x720"]
    pub dpi_aware: bool,
    pub allow_rotation: bool,
}
```

**GUI Requirements:**
- Toggle for allow_resize
- Multi-line text for allowed_resolutions
- Toggle for dpi_aware
- Toggle for allow_rotation

#### AdvancedVideoConfig (4 fields)
```rust
pub struct AdvancedVideoConfig {
    pub enable_frame_skip: bool,
    pub scene_change_threshold: f32,   // 0.0-1.0
    pub intra_refresh_interval: u32,   // frames
    pub enable_adaptive_quality: bool,
}
```

#### CursorConfig (5 fields + sub-struct)
```rust
pub struct CursorConfig {
    pub mode: String,                  // "metadata", "painted", "hidden", "predictive"
    pub auto_mode: bool,
    pub predictive_latency_threshold_ms: u32,
    pub cursor_update_fps: u32,
    pub predictor: CursorPredictorConfig,  // 7 fields
}

pub struct CursorPredictorConfig {
    pub history_size: usize,
    pub lookahead_ms: f32,
    pub velocity_smoothing: f32,       // 0.0-1.0
    pub acceleration_smoothing: f32,   // 0.0-1.0
    pub max_prediction_distance: i32,  // pixels
    pub min_velocity_threshold: f32,   // pixels/second
    pub stop_convergence_rate: f32,    // 0.0-1.0
}
```

---

## 3. GUI Architecture

### 3.1 iced Framework Overview

**Why iced:**
- Pure Rust, cross-platform
- Elm Architecture (predictable state management)
- GPU-accelerated rendering
- Strong typing prevents runtime errors
- Active development, good documentation

**iced Elm Architecture Pattern:**
```
   ┌──────────┐
   │   View   │  (renders current state)
   └────┬─────┘
        │ produces
        ▼
   ┌──────────┐
   │ Messages │  (user interactions)
   └────┬─────┘
        │ processed by
        ▼
   ┌──────────┐
   │  Update  │  (state transitions)
   └────┬─────┘
        │ produces new
        ▼
   ┌──────────┐
   │  State   │  (application data)
   └────┬─────┘
        │ rendered by
        ▼
   (back to View)
```

### 3.2 Application Structure

```rust
use iced::{
    Application, Command, Element, Settings,
    widget::{button, column, container, pick_list, row, scrollable, slider, text, text_input, toggler},
};

pub struct ConfigGuiApp {
    // Application state
    state: AppState,

    // UI state
    current_tab: Tab,

    // Server communication
    ipc_client: Option<IpcClient>,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Tab navigation
    TabSelected(Tab),

    // Config changes (one variant per parameter)
    ServerListenAddrChanged(String),
    ServerMaxConnectionsChanged(String),
    ServerSessionTimeoutChanged(String),
    ServerUsePortalsToggled(bool),

    SecurityCertPathChanged(String),
    SecurityKeyPathChanged(String),
    SecurityGenerateCert,
    SecurityEnableNlaToggled(bool),
    // ... (85+ message variants for all config fields)

    // File operations
    LoadConfig(PathBuf),
    SaveConfig,
    ConfigLoaded(Result<Config, String>),
    ConfigSaved(Result<(), String>),

    // Server control
    StartServer,
    StopServer,
    ServerStatusUpdated(ServerStatus),

    // Validation
    ValidateConfig,
    ValidationComplete(ValidationResult),

    // Hardware detection
    DetectGPUs,
    GPUsDetected(Vec<GpuInfo>),
}

impl Application for ConfigGuiApp {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        // Load config from default location or create new
        let state = AppState::load_or_default();

        (
            Self {
                state,
                current_tab: Tab::Server,
                ipc_client: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "lamco-rdp-server Configuration".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::TabSelected(tab) => {
                self.current_tab = tab;
                Command::none()
            }

            Message::ServerListenAddrChanged(addr) => {
                self.state.config.server.listen_addr = addr;
                self.state.mark_dirty();
                Command::none()
            }

            // ... handle all message variants

            Message::SaveConfig => {
                let config = self.state.config.clone();
                Command::perform(
                    async move {
                        save_config_to_file(&config).await
                    },
                    Message::ConfigSaved,
                )
            }

            // ... other handlers
        }
    }

    fn view(&self) -> Element<Message> {
        let content = match self.current_tab {
            Tab::Server => self.view_server_tab(),
            Tab::Security => self.view_security_tab(),
            Tab::Video => self.view_video_tab(),
            // ... other tabs
        };

        container(
            column![
                self.view_header(),
                self.view_tab_bar(),
                content,
                self.view_footer(),
            ]
        )
        .into()
    }
}
```

---

## 4. State Management

### 4.1 AppState Structure

```rust
use crate::config::Config;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppState {
    // Configuration being edited
    pub config: Config,

    // File state
    pub config_path: PathBuf,
    pub is_dirty: bool,          // Unsaved changes
    pub last_saved: Option<SystemTime>,

    // Validation state
    pub validation: ValidationState,

    // Server state (from IPC)
    pub server_status: ServerStatus,

    // Hardware detection
    pub detected_gpus: Vec<GpuInfo>,
    pub detected_vaapi_devices: Vec<PathBuf>,

    // UI state
    pub active_preset: Option<PresetName>,
    pub expert_mode: bool,       // Show advanced params
    pub show_service_registry: bool,

    // Error/info messages
    pub messages: Vec<UserMessage>,
}

impl AppState {
    pub fn load_or_default() -> Self {
        let config_path = Self::default_config_path();
        let config = Config::load(config_path.to_str().unwrap())
            .unwrap_or_else(|_| Config::default_config().unwrap());

        Self {
            config,
            config_path,
            is_dirty: false,
            last_saved: None,
            validation: ValidationState::default(),
            server_status: ServerStatus::Unknown,
            detected_gpus: Vec::new(),
            detected_vaapi_devices: Vec::new(),
            active_preset: None,
            expert_mode: false,
            show_service_registry: false,
            messages: Vec::new(),
        }
    }

    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    pub fn mark_clean(&mut self) {
        self.is_dirty = false;
        self.last_saved = Some(SystemTime::now());
    }

    fn default_config_path() -> PathBuf {
        // Try in order:
        // 1. $XDG_CONFIG_HOME/lamco-rdp-server/config.toml
        // 2. ~/.config/lamco-rdp-server/config.toml
        // 3. /etc/lamco-rdp-server/config.toml
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/etc"))
            .join("lamco-rdp-server")
            .join("config.toml")
    }
}

#[derive(Debug, Clone)]
pub struct ValidationState {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServerStatus {
    Unknown,
    Stopped,
    Starting,
    Running { connections: usize, uptime: Duration },
    Error(String),
}

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub vendor: String,          // "Intel", "AMD", "NVIDIA"
    pub model: String,           // "Arc A770", "Radeon RX 7900 XTX"
    pub driver: String,          // "iHD", "radeonsi", "nvidia"
    pub vaapi_device: Option<PathBuf>,  // /dev/dri/renderD128
    pub nvenc_available: bool,
    pub supports_h264: bool,
}

#[derive(Debug, Clone)]
pub struct UserMessage {
    pub level: MessageLevel,
    pub text: String,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Copy)]
pub enum MessageLevel {
    Info,
    Warning,
    Error,
    Success,
}
```

---

## 5. Tab Specifications

### 5.1 Tab Organization

**10 Main Tabs:**

1. **Server** - Basic server settings (listen address, connections, portals)
2. **Security** - TLS certificates, authentication, NLA
3. **Video** - Core video settings (encoder, FPS, bitrate)
4. **Input** - Keyboard, mouse, touch configuration
5. **Clipboard** - Clipboard sync settings
6. **Logging** - Log level, file output, metrics
7. **Performance** - Thread counts, adaptive FPS, latency modes
8. **EGFX** - Graphics pipeline (H.264, AVC444, quality)
9. **Advanced** - Damage tracking, hardware encoding, cursor, display
10. **Status** - Service registry, server status, log viewer

### 5.2 Tab 1: Server

**Parameters:** 4 fields from `ServerConfig`

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│ SERVER CONFIGURATION                                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ Listen Address:  [0.0.0.0              ] : [3389]           │
│   ⓘ IP address and port for RDP server                      │
│                                                              │
│ Maximum Connections:  [10          ] ▲▼                      │
│   ⓘ Maximum number of simultaneous clients                   │
│                                                              │
│ Session Timeout:  [0           ] seconds ▲▼                  │
│   ⓘ Auto-disconnect idle sessions (0 = never)               │
│                                                              │
│ [✓] Use XDG Desktop Portals                                 │
│   ⓘ Required for Wayland screen capture                     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Widget Specifications:**

```rust
fn view_server_tab(&self) -> Element<Message> {
    let listen_addr_parts: Vec<&str> = self.state.config.server.listen_addr
        .rsplitn(2, ':')
        .collect();
    let (port, addr) = if listen_addr_parts.len() == 2 {
        (listen_addr_parts[0], listen_addr_parts[1])
    } else {
        ("3389", "0.0.0.0")
    };

    column![
        // Section header
        text("SERVER CONFIGURATION")
            .size(24)
            .style(Theme::Heading),

        // Listen address
        row![
            text("Listen Address:").width(Length::Fixed(150.0)),
            text_input("IP address", addr)
                .on_input(|value| {
                    Message::ServerListenAddrChanged(format!("{}:{}", value, port))
                })
                .width(Length::Fixed(200.0)),
            text(":").size(20),
            text_input("Port", port)
                .on_input(|value| {
                    Message::ServerPortChanged(value)
                })
                .width(Length::Fixed(80.0)),
        ]
        .spacing(10)
        .align_items(Alignment::Center),

        // Help text
        row![
            Space::with_width(Length::Fixed(150.0)),
            text("ⓘ IP address and port for RDP server")
                .size(12)
                .style(Theme::Help),
        ],

        Space::with_height(Length::Fixed(20.0)),

        // Max connections
        row![
            text("Maximum Connections:").width(Length::Fixed(150.0)),
            number_input(
                self.state.config.server.max_connections,
                1..=100,
                Message::ServerMaxConnectionsChanged,
            )
            .width(Length::Fixed(100.0)),
        ]
        .spacing(10)
        .align_items(Alignment::Center),

        row![
            Space::with_width(Length::Fixed(150.0)),
            text("ⓘ Maximum number of simultaneous clients")
                .size(12)
                .style(Theme::Help),
        ],

        Space::with_height(Length::Fixed(20.0)),

        // Session timeout
        row![
            text("Session Timeout:").width(Length::Fixed(150.0)),
            number_input(
                self.state.config.server.session_timeout as i32,
                0..=86400,
                Message::ServerSessionTimeoutChanged,
            )
            .width(Length::Fixed(100.0)),
            text("seconds"),
        ]
        .spacing(10)
        .align_items(Alignment::Center),

        row![
            Space::with_width(Length::Fixed(150.0)),
            text("ⓘ Auto-disconnect idle sessions (0 = never)")
                .size(12)
                .style(Theme::Help),
        ],

        Space::with_height(Length::Fixed(20.0)),

        // Use portals
        row![
            text("Use XDG Desktop Portals:").width(Length::Fixed(150.0)),
            toggler(
                String::new(),
                self.state.config.server.use_portals,
                Message::ServerUsePortalsToggled,
            ),
        ]
        .spacing(10)
        .align_items(Alignment::Center),

        row![
            Space::with_width(Length::Fixed(150.0)),
            text("ⓘ Required for Wayland screen capture")
                .size(12)
                .style(Theme::Help),
        ],
    ]
    .spacing(10)
    .padding(20)
    .into()
}
```

### 5.3 Tab 2: Security

**Parameters:** 5 fields from `SecurityConfig`

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│ SECURITY CONFIGURATION                                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ TLS Certificate:                                             │
│   [/etc/lamco-rdp-server/cert.pem          ] [Browse...]    │
│   [Generate Self-Signed Certificate]                         │
│                                                              │
│ TLS Private Key:                                             │
│   [/etc/lamco-rdp-server/key.pem           ] [Browse...]    │
│                                                              │
│ [✓] Enable Network Level Authentication (NLA)               │
│   ⓘ Requires client to authenticate before connection       │
│                                                              │
│ Authentication Method:  [PAM            ▼]                   │
│   ⓘ PAM = system authentication, None = no password         │
│                                                              │
│ [✓] Require TLS 1.3 or higher                               │
│   ⓘ Recommended for security, may block older clients       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Certificate Generation Dialog:**
```rust
fn show_cert_generation_dialog(&self) -> Element<Message> {
    modal(
        column![
            text("Generate Self-Signed Certificate")
                .size(20),

            Space::with_height(Length::Fixed(20.0)),

            row![
                text("Common Name:").width(Length::Fixed(120.0)),
                text_input("localhost", &self.cert_gen_state.common_name)
                    .on_input(Message::CertGenCommonNameChanged),
            ],

            row![
                text("Organization:").width(Length::Fixed(120.0)),
                text_input("My Company", &self.cert_gen_state.organization)
                    .on_input(Message::CertGenOrganizationChanged),
            ],

            row![
                text("Valid Days:").width(Length::Fixed(120.0)),
                number_input(
                    self.cert_gen_state.valid_days,
                    1..=3650,
                    Message::CertGenValidDaysChanged,
                ),
            ],

            Space::with_height(Length::Fixed(20.0)),

            row![
                button("Cancel")
                    .on_press(Message::CertGenCancel),
                Space::with_width(Length::Fill),
                button("Generate")
                    .on_press(Message::CertGenConfirm)
                    .style(Theme::Primary),
            ]
            .spacing(10),
        ]
        .spacing(10)
        .padding(20)
    )
}
```

**Certificate Generation Implementation:**
```rust
async fn generate_self_signed_certificate(
    cert_path: PathBuf,
    key_path: PathBuf,
    common_name: String,
    organization: String,
    valid_days: u32,
) -> Result<(), String> {
    // Use rcgen crate to generate certificate
    use rcgen::{Certificate, CertificateParams, DistinguishedName};

    let mut params = CertificateParams::new(vec![common_name.clone()]);

    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(rcgen::DnType::CommonName, common_name);
    distinguished_name.push(rcgen::DnType::OrganizationName, organization);
    params.distinguished_name = distinguished_name;

    params.not_before = time::OffsetDateTime::now_utc();
    params.not_after = time::OffsetDateTime::now_utc() + time::Duration::days(valid_days as i64);

    let cert = Certificate::from_params(params)
        .map_err(|e| format!("Failed to generate certificate: {}", e))?;

    // Write certificate
    tokio::fs::write(&cert_path, cert.serialize_pem()?)
        .await
        .map_err(|e| format!("Failed to write certificate: {}", e))?;

    // Write private key
    tokio::fs::write(&key_path, cert.serialize_private_key_pem())
        .await
        .map_err(|e| format!("Failed to write private key: {}", e))?;

    Ok(())
}
```

### 5.4 Tab 3: Video

**Parameters:** 6 fields from `VideoConfig` + 3 sub-structs from `VideoPipelineConfig`

**Layout with Collapsible Sections:**
```
┌─────────────────────────────────────────────────────────────┐
│ VIDEO CONFIGURATION                                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ ▼ Basic Settings                                             │
│                                                              │
│   Encoder:  [Auto              ▼]  [Detect GPUs]            │
│     ⓘ Hardware: vaapi (Intel/AMD), Software: openh264      │
│                                                              │
│   VA-API Device:  [/dev/dri/renderD128     ▼]               │
│     ⓘ GPU device for hardware encoding                      │
│                                                              │
│   Target FPS:     [====●============] 30 fps                 │
│     ⓘ 5 ←────────────────────────────────→ 60               │
│                                                              │
│   Bitrate:        [=======●=========] 5000 kbps              │
│     ⓘ 1000 ←──────────────────────────→ 20000               │
│                                                              │
│   [✓] Enable Damage Tracking                                │
│     ⓘ Only encode changed regions (saves bandwidth)         │
│                                                              │
│   Cursor Mode:  [Metadata       ▼]                          │
│     ⓘ Metadata = client-side (lowest latency)               │
│                                                              │
│ ▶ Advanced Pipeline (click to expand)                       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Expanded Advanced Pipeline Section:**
```
│ ▼ Advanced Pipeline                                          │
│                                                              │
│   ▼ Frame Processor                                          │
│     Max Queue Depth:      [30      ]                         │
│     [✓] Adaptive Quality                                     │
│     Damage Threshold:     [====●=====] 0.05                  │
│     [✓] Drop on Full Queue                                   │
│     [✓] Enable Metrics                                       │
│                                                              │
│   ▼ Frame Dispatcher                                         │
│     Channel Size:         [30      ]                         │
│     [✓] Priority Dispatch                                    │
│     Max Frame Age:        [150     ] ms                      │
│     [✓] Enable Backpressure                                  │
│     High Water Mark:      [==●======] 0.8                    │
│     Low Water Mark:       [===●=====] 0.5                    │
│     [✓] Load Balancing                                       │
│                                                              │
│   ▼ Bitmap Converter                                         │
│     Buffer Pool Size:     [8       ]                         │
│     [✓] Enable SIMD                                          │
│     Damage Threshold:     [=======●=] 0.75                   │
│     [✓] Enable Statistics                                    │
```

**Encoder Auto-Detection:**
```rust
async fn detect_available_encoders() -> Vec<EncoderOption> {
    let mut encoders = vec![EncoderOption {
        name: "Auto".to_string(),
        value: "auto".to_string(),
        description: "Automatically select best available encoder".to_string(),
    }];

    // Check for VA-API
    if let Ok(vaapi_devices) = detect_vaapi_devices().await {
        for device in vaapi_devices {
            encoders.push(EncoderOption {
                name: format!("VA-API ({})", device.driver),
                value: "vaapi".to_string(),
                description: format!("{} on {}", device.model, device.path.display()),
            });
        }
    }

    // Check for NVENC
    if is_nvenc_available().await {
        encoders.push(EncoderOption {
            name: "NVENC (NVIDIA)".to_string(),
            value: "nvenc".to_string(),
            description: "NVIDIA hardware encoder".to_string(),
        });
    }

    // Software fallback
    encoders.push(EncoderOption {
        name: "OpenH264 (Software)".to_string(),
        value: "openh264".to_string(),
        description: "Software encoder (CPU, works everywhere)".to_string(),
    });

    encoders
}
```

---

### 5.5 Tab 4: Input

**Parameters:** 3 fields from `InputConfig`

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│ INPUT CONFIGURATION                                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ [✓] Use libei for Input Injection                           │
│   ⓘ Modern input method via Portal RemoteDesktop            │
│                                                              │
│ Keyboard Layout:  [Auto (Detect)    ▼]                      │
│   ⓘ Auto-detect or specify XKB layout name                  │
│                                                              │
│   Common Layouts:                                            │
│   • us - US English                                          │
│   • gb - UK English                                          │
│   • de - German                                              │
│   • fr - French                                              │
│   [Show All Layouts...]                                      │
│                                                              │
│ [✓] Enable Touch Input                                      │
│   ⓘ Support touchscreen devices (if available)              │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 5.6 Tab 5: Clipboard

**Parameters:** 4 fields from `ClipboardConfig`

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│ CLIPBOARD CONFIGURATION                                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ [✓] Enable Clipboard Synchronization                        │
│   ⓘ Copy/paste between client and server                    │
│                                                              │
│ Maximum Clipboard Size:  [10        ] MB                     │
│   ⓘ Reject clipboard data larger than this                  │
│                                                              │
│ Rate Limiting:  [200    ] ms between events                  │
│   ⓘ Prevents clipboard spam attacks (max 5 events/sec)      │
│                                                              │
│ Allowed MIME Types (one per line, empty = all):              │
│ ┌───────────────────────────────────────────────────────┐   │
│ │ text/plain                                             │   │
│ │ text/html                                              │   │
│ │ image/png                                              │   │
│ │                                                        │   │
│ └───────────────────────────────────────────────────────┘   │
│   ⓘ Leave empty to allow all clipboard formats              │
│                                                              │
│ Common Types: [Text] [Images] [Files] [All]                 │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Preset Buttons Implementation:**
```rust
fn clipboard_preset_buttons(&self) -> Element<Message> {
    row![
        button("Text Only")
            .on_press(Message::ClipboardPreset(ClipboardPreset::TextOnly)),
        button("Text + Images")
            .on_press(Message::ClipboardPreset(ClipboardPreset::TextAndImages)),
        button("All Types")
            .on_press(Message::ClipboardPreset(ClipboardPreset::All)),
    ]
    .spacing(10)
}

#[derive(Debug, Clone)]
enum ClipboardPreset {
    TextOnly,      // text/plain, text/html, text/uri-list
    TextAndImages, // + image/png, image/jpeg
    All,           // empty list = all types
}

impl ClipboardPreset {
    fn to_mime_types(&self) -> Vec<String> {
        match self {
            Self::TextOnly => vec![
                "text/plain".to_string(),
                "text/html".to_string(),
                "text/uri-list".to_string(),
            ],
            Self::TextAndImages => vec![
                "text/plain".to_string(),
                "text/html".to_string(),
                "text/uri-list".to_string(),
                "image/png".to_string(),
                "image/jpeg".to_string(),
            ],
            Self::All => vec![], // empty = all types
        }
    }
}
```

### 5.7 Tab 6: Logging

**Parameters:** 3 fields from `LoggingConfig` + CLI args integration

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│ LOGGING CONFIGURATION                                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ Log Level:  [Info            ▼]                             │
│   ⓘ Trace: Everything | Debug: Verbose | Info: Normal      │
│                                                              │
│ Log Output:                                                  │
│   [✓] Console (stdout)                                      │
│   [✓] File                                                   │
│                                                              │
│ Log Directory:  [~/.local/share/lamco-rdp-server/logs]      │
│                 [Browse...]                                  │
│   ⓘ Leave empty for console-only logging                    │
│                                                              │
│ [✓] Enable Performance Metrics                              │
│   ⓘ Collect FPS, bandwidth, latency statistics              │
│                                                              │
│ ▼ Advanced Logging Options                                  │
│                                                              │
│   Log Format:  [Pretty       ▼]                             │
│     • Pretty - Human-readable with colors                    │
│     • JSON - Machine-readable for log analysis              │
│     • Compact - One-line format                              │
│                                                              │
│   Rotation:  [Daily          ▼]                             │
│     • Daily - New file each day                              │
│     • Size - Rotate at 100MB                                 │
│     • Never - Single file                                    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Log Viewer Integration (in Status tab):**
```rust
fn view_log_viewer(&self) -> Element<Message> {
    let log_lines: Vec<Element<Message>> = self.state.log_buffer
        .iter()
        .map(|line| {
            let style = match line.level {
                LogLevel::Error => Theme::Error,
                LogLevel::Warn => Theme::Warning,
                LogLevel::Info => Theme::Info,
                LogLevel::Debug => Theme::Debug,
                LogLevel::Trace => Theme::Trace,
            };

            row![
                text(format_timestamp(line.timestamp))
                    .size(12)
                    .width(Length::Fixed(100.0)),
                text(line.level.as_str())
                    .size(12)
                    .width(Length::Fixed(60.0))
                    .style(style),
                text(&line.message)
                    .size(12),
            ]
            .spacing(10)
            .into()
        })
        .collect();

    container(
        scrollable(
            column(log_lines)
                .spacing(2)
        )
        .height(Length::Fixed(300.0))
    )
    .style(Theme::LogViewer)
    .padding(10)
    .into()
}

// Tail -f style log following
async fn follow_log_file(log_path: PathBuf, tx: mpsc::Sender<LogLine>) {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::fs::File;

    let file = File::open(&log_path).await.unwrap();
    let mut reader = BufReader::new(file).lines();

    while let Some(line) = reader.next_line().await.unwrap() {
        if let Some(log_line) = parse_log_line(&line) {
            let _ = tx.send(log_line).await;
        }
    }
}
```

---

### 5.8 Tab 7: Performance

**Parameters:** 6 fields from `PerformanceConfig` + 2 sub-structs (12 total fields)

**Layout with Presets:**
```
┌─────────────────────────────────────────────────────────────┐
│ PERFORMANCE CONFIGURATION                                    │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ Preset Profiles:  [Interactive] [Balanced] [Quality]        │
│                                                              │
│ ▼ Threading                                                  │
│                                                              │
│   Encoder Threads:  [0 (Auto)  ] ▲▼                          │
│     ⓘ 0 = Auto-detect CPU cores, or specify 1-16            │
│                                                              │
│   Network Threads:  [0 (Auto)  ] ▲▼                          │
│     ⓘ 0 = Auto-detect, or specify 1-8                       │
│                                                              │
│   Buffer Pool Size: [16        ] ▲▼                          │
│     ⓘ Frame buffers for pipelining                          │
│                                                              │
│   [✓] Enable Zero-Copy Operations                           │
│     ⓘ DMA-BUF zero-copy when supported                      │
│                                                              │
│ ▼ Adaptive FPS                                               │
│                                                              │
│   [✓] Enabled                                                │
│     ⓘ Dynamically adjust FPS based on screen activity       │
│                                                              │
│   Min FPS:  [====●=========] 5 fps                           │
│   Max FPS:  [===========●==] 30 fps                          │
│                                                              │
│   Activity Thresholds:                                       │
│     High Activity:    [========●==] 0.30 (30% changed)       │
│     Medium Activity:  [===●=======] 0.10 (10% changed)       │
│     Low Activity:     [●==========] 0.01 (1% changed)        │
│                                                              │
│ ▼ Latency Governor                                           │
│                                                              │
│   Mode:  [Balanced       ▼]                                  │
│     • Interactive - <50ms (gaming, CAD)                      │
│     • Balanced - <100ms (general desktop) ✓                 │
│     • Quality - <300ms (photo/video editing)                 │
│                                                              │
│   Advanced Tuning:                                           │
│     Interactive Max Delay:  [16   ] ms                       │
│     Balanced Max Delay:     [33   ] ms                       │
│     Quality Max Delay:      [100  ] ms                       │
│     Balanced Threshold:     [==●========] 0.02               │
│     Quality Threshold:      [===●=======] 0.05               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Preset Implementation:**
```rust
#[derive(Debug, Clone, Copy)]
enum PerformancePreset {
    Interactive,  // <50ms latency, 60fps, aggressive
    Balanced,     // <100ms latency, 30fps, moderate
    Quality,      // <300ms latency, quality over speed
}

impl PerformancePreset {
    fn apply_to_config(&self, config: &mut PerformanceConfig) {
        match self {
            Self::Interactive => {
                config.encoder_threads = 0;
                config.network_threads = 0;
                config.buffer_pool_size = 32;
                config.zero_copy = true;

                config.adaptive_fps.enabled = true;
                config.adaptive_fps.min_fps = 15;
                config.adaptive_fps.max_fps = 60;
                config.adaptive_fps.high_activity_threshold = 0.20;
                config.adaptive_fps.medium_activity_threshold = 0.08;
                config.adaptive_fps.low_activity_threshold = 0.01;

                config.latency.mode = "interactive".to_string();
                config.latency.interactive_max_delay_ms = 16;
            }

            Self::Balanced => {
                config.encoder_threads = 0;
                config.network_threads = 0;
                config.buffer_pool_size = 16;
                config.zero_copy = true;

                config.adaptive_fps.enabled = true;
                config.adaptive_fps.min_fps = 5;
                config.adaptive_fps.max_fps = 30;
                config.adaptive_fps.high_activity_threshold = 0.30;
                config.adaptive_fps.medium_activity_threshold = 0.10;
                config.adaptive_fps.low_activity_threshold = 0.01;

                config.latency.mode = "balanced".to_string();
                config.latency.balanced_max_delay_ms = 33;
            }

            Self::Quality => {
                config.encoder_threads = 0;
                config.network_threads = 0;
                config.buffer_pool_size = 8;
                config.zero_copy = false;  // Prefer quality

                config.adaptive_fps.enabled = false;  // Fixed FPS

                config.latency.mode = "quality".to_string();
                config.latency.quality_max_delay_ms = 100;
            }
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Interactive => "<50ms latency for gaming/CAD, 60fps capable",
            Self::Balanced => "<100ms latency for general desktop, 30fps",
            Self::Quality => "<300ms latency, prioritize visual quality",
        }
    }
}
```

---

## 5.9 Tab 8: EGFX

**Parameters:** 23 fields from `EgfxConfig` (most complex tab)

**Layout with Expert Mode:**
```
┌─────────────────────────────────────────────────────────────┐
│ EGFX (GRAPHICS PIPELINE) CONFIGURATION                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ [✓] Enable EGFX Graphics Pipeline                           │
│   ⓘ Required for H.264 video and modern clients             │
│                                                              │
│ Quality Preset:  [Speed] [Balanced] [Quality]               │
│                                                              │
│ ▼ Basic Settings                                             │
│                                                              │
│   H.264 Bitrate:  [5000    ] kbps                            │
│     ⓘ Main stream bitrate (3000-15000 recommended)          │
│                                                              │
│   Codec:  [Auto (AVC444 if supported) ▼]                    │
│     • Auto - Use best available                              │
│     • AVC420 - 4:2:0 chroma (compatible)                     │
│     • AVC444 - 4:4:4 chroma (best quality)                   │
│                                                              │
│   Periodic Keyframe Interval:  [5     ] seconds             │
│     ⓘ Force IDR frame periodically (clears artifacts)       │
│                                                              │
│ ▼ Quality Control                                            │
│                                                              │
│   QP Min:      [10  ] ▲▼                                     │
│   QP Default:  [23  ] ▲▼                                     │
│   QP Max:      [40  ] ▲▼                                     │
│     ⓘ Lower QP = better quality, higher bitrate             │
│                                                              │
│ [Show Expert Settings]                                       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Expert Mode Expanded:**
```
│ [Hide Expert Settings]                                       │
│                                                              │
│ ▼ Advanced EGFX                                              │
│                                                              │
│   H.264 Level:  [Auto            ▼]                          │
│     • Auto - Detect based on resolution                      │
│     • 3.0, 3.1, 4.0, 4.1, 5.0, 5.1, 5.2                     │
│                                                              │
│   ZGFX Compression:  [Never         ▼]                       │
│     ⓘ Never: Disable, Auto: Heuristic, Always: Force        │
│                                                              │
│   Max Frames in Flight:  [3     ]                            │
│     ⓘ Backpressure threshold                                │
│                                                              │
│   Frame Ack Timeout:  [5000   ] ms                           │
│                                                              │
│ ▼ AVC444 Configuration (4:4:4 Chroma)                        │
│                                                              │
│   [✓] Enable AVC444                                          │
│     ⓘ Superior text/UI rendering, requires modern client    │
│                                                              │
│   Auxiliary Bitrate Ratio:  [====●====] 0.50 (50%)           │
│     ⓘ Aux stream gets 50% of main's bitrate                 │
│                                                              │
│   Color Matrix:  [Auto (OpenH264)  ▼]                        │
│     • Auto - OpenH264-compatible                             │
│     • BT.709 - HD content                                    │
│     • BT.601 - SD content                                    │
│     • sRGB - Computer graphics                               │
│                                                              │
│   Color Range:  [Auto (Limited)    ▼]                        │
│     • Auto - Use matrix default                              │
│     • Limited - TV range (16-235)                            │
│     • Full - PC range (0-255)                                │
│                                                              │
│ ▼ AVC444 Aux Omission (Bandwidth Optimization)              │
│                                                              │
│   [✓] Enable Auxiliary Stream Omission                      │
│     ⓘ Skip aux when unchanged (FreeRDP-compatible)          │
│                                                              │
│   Max Aux Interval:  [30    ] frames                         │
│     ⓘ Force aux refresh every N frames                      │
│                                                              │
│   Aux Change Threshold:  [===●======] 0.05 (5%)              │
│     ⓘ Fraction of pixels that must change                   │
│                                                              │
│   [  ] Force Aux IDR on Return                               │
│     ⓘ Must be OFF for single encoder (PRODUCTION)           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Quality Preset Implementation:**
```rust
#[derive(Debug, Clone, Copy)]
enum EgfxPreset {
    Speed,    // 3Mbps, QP 20-40, fast
    Balanced, // 5Mbps, QP 18-36, default
    Quality,  // 10Mbps, QP 15-30, slow
}

impl EgfxPreset {
    fn apply_to_config(&self, config: &mut EgfxConfig) {
        match self {
            Self::Speed => {
                config.h264_bitrate = 3000;
                config.qp_min = 20;
                config.qp_default = 28;
                config.qp_max = 40;
                config.periodic_idr_interval = 10;  // More keyframes
                config.avc444_aux_bitrate_ratio = 0.3;  // Lower aux bitrate
            }

            Self::Balanced => {
                config.h264_bitrate = 5000;
                config.qp_min = 18;
                config.qp_default = 23;
                config.qp_max = 36;
                config.periodic_idr_interval = 5;
                config.avc444_aux_bitrate_ratio = 0.5;
            }

            Self::Quality => {
                config.h264_bitrate = 10000;
                config.qp_min = 15;
                config.qp_default = 20;
                config.qp_max = 30;
                config.periodic_idr_interval = 3;
                config.avc444_aux_bitrate_ratio = 1.0;  // Max aux quality
            }
        }
    }
}
```

---

## 5.10 Tab 9: Advanced

**Combined Tab:** Damage Tracking, Hardware Encoding, Display, Advanced Video, Cursor

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│ ADVANCED CONFIGURATION                                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ ▼ Damage Tracking                                            │
│                                                              │
│   [✓] Enabled                                                │
│     ⓘ Only encode changed regions (bandwidth optimization)  │
│                                                              │
│   Method:  [Diff              ▼]                             │
│     • Diff - CPU pixel comparison (compatible)               │
│     • PipeWire - Use PipeWire damage hints                   │
│     • Hybrid - Combine both methods                          │
│                                                              │
│   Sensitivity Presets:  [Text Work] [General] [Video]       │
│                                                              │
│   Tile Size:  [16     ] pixels                               │
│     ⓘ 16x16 matches FreeRDP (max sensitivity)               │
│                                                              │
│   Diff Threshold:  [==●========] 0.01 (1%)                   │
│     ⓘ % of tile pixels that must change                     │
│                                                              │
│   Pixel Threshold:  [1     ]                                 │
│     ⓘ RGB difference to count as changed                    │
│                                                              │
│ ▼ Hardware Encoding                                          │
│                                                              │
│   [✓] Enable Hardware Acceleration                          │
│     ⓘ Use GPU for H.264 encoding (lower CPU usage)          │
│                                                              │
│   Detected GPUs:                                             │
│   ● Intel Arc A770 (iHD driver)                             │
│   ● NVIDIA GeForce RTX 4090 (nvenc)                         │
│                                                              │
│   VA-API Device:  [/dev/dri/renderD128  ▼]                   │
│                                                              │
│   [✓] Enable DMA-BUF Zero-Copy                               │
│   [✓] Fallback to Software                                   │
│   [✓] Prefer NVENC over VA-API                               │
│                                                              │
│   Quality Preset:  [Balanced       ▼]                        │
│                                                              │
│ ▼ Display Control                                            │
│                                                              │
│   [✓] Allow Dynamic Resolution                               │
│   [  ] DPI Aware                                             │
│   [  ] Allow Rotation                                        │
│                                                              │
│   Allowed Resolutions (empty = all):                         │
│   ┌──────────────────────────────────────────────────────┐  │
│   │ 1920x1080                                             │  │
│   │ 2560x1440                                             │  │
│   │ 3840x2160                                             │  │
│   └──────────────────────────────────────────────────────┘  │
│                                                              │
│ ▼ Advanced Video                                             │
│                                                              │
│   [✓] Enable Frame Skip                                      │
│   Scene Change Threshold:  [======●===] 0.70                 │
│   Intra Refresh Interval:  [300    ] frames                  │
│   [  ] Enable Adaptive Quality                               │
│                                                              │
│ ▼ Cursor Configuration                                       │
│                                                              │
│   Mode:  [Metadata (Client-side)  ▼]                         │
│     • Metadata - Client renders (lowest latency)             │
│     • Painted - Composited into video                        │
│     • Hidden - No cursor (touch/pen)                         │
│     • Predictive - Physics-based compensation                │
│                                                              │
│   [✓] Auto Mode Selection                                    │
│   Predictive Threshold:  [100    ] ms                        │
│   Cursor Update FPS:  [60     ]                              │
│                                                              │
│   ▶ Predictor Configuration (click to expand)               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Damage Tracking Presets:**
```rust
#[derive(Debug, Clone, Copy)]
enum DamageTrackingPreset {
    TextWork,   // Max sensitivity, catch typing
    General,    // Balanced
    Video,      // Less sensitive, prioritize bandwidth
}

impl DamageTrackingPreset {
    fn apply_to_config(&self, config: &mut DamageTrackingConfig) {
        match self {
            Self::TextWork => {
                config.tile_size = 16;   // 16x16 like FreeRDP
                config.diff_threshold = 0.01;  // 1% very sensitive
                config.pixel_threshold = 1;    // Max sensitivity
                config.merge_distance = 16;
                config.min_region_area = 64;   // 8x8 minimum
            }

            Self::General => {
                config.tile_size = 32;
                config.diff_threshold = 0.05;  // 5%
                config.pixel_threshold = 4;
                config.merge_distance = 32;
                config.min_region_area = 256;  // 16x16
            }

            Self::Video => {
                config.tile_size = 128;
                config.diff_threshold = 0.10;  // 10%
                config.pixel_threshold = 8;
                config.merge_distance = 64;
                config.min_region_area = 1024; // 32x32
            }
        }
    }
}
```

---

## 5.11 Tab 10: Status

**This tab displays:**
1. Service Registry (read-only view)
2. Server status
3. Log viewer (live tail)

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│ STATUS & MONITORING                                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ ▼ Server Status                                              │
│                                                              │
│   Status:  ● Running                                         │
│   Uptime:  2h 34m 12s                                        │
│   Connections:  2 active                                     │
│   Address:  192.168.1.100:3389                               │
│                                                              │
│   [Stop Server]  [Restart Server]                            │
│                                                              │
│ ▼ Service Registry (18 Services)                             │
│                                                              │
│   ┌──────────────────────────────────────────────────────┐  │
│   │ Service            │ Level     │ RDP Capability    │  │  │
│   ├──────────────────────────────────────────────────────┤  │
│   │ ✅ Video Capture   │ Guaranteed│ RemoteFX          │  │  │
│   │ ✅ Damage Tracking │ Guaranteed│ -                 │  │  │
│   │ 🔶 DMA-BUF Zero-Copy│ BestEffort│ -               │  │  │
│   │ ✅ Metadata Cursor │ Guaranteed│ Color Pointer     │  │  │
│   │ ✅ Multi-Monitor   │ Guaranteed│ Multi-Monitor     │  │  │
│   │ ✅ Remote Input    │ Guaranteed│ Input             │  │  │
│   │ ✅ Clipboard       │ Guaranteed│ Cliprdr           │  │  │
│   │ ⚠️  Explicit Sync   │ Degraded  │ -                │  │  │
│   │ ❌ HDR Color Space │ Unavailable│ -               │  │  │
│   │ ... (scroll for more)                              │  │  │
│   └──────────────────────────────────────────────────────┘  │
│                                                              │
│   Summary: ✅ 11 Guaranteed │ 🔶 4 BestEffort │             │
│           ⚠️  2 Degraded   │ ❌ 1 Unavailable               │
│                                                              │
│ ▼ Live Logs (last 100 lines)                                │
│                                                              │
│   ┌──────────────────────────────────────────────────────┐  │
│   │ 14:23:45 INFO  Server listening on 0.0.0.0:3389     │  │
│   │ 14:23:46 INFO  Client connected from 192.168.1.50   │  │
│   │ 14:23:47 DEBUG Frame encoded: 1920x1080, 4.2ms      │  │
│   │ 14:23:47 TRACE Damage regions: 12 tiles, 0.8% area  │  │
│   │ ... (auto-scrolling)                                 │  │
│   └──────────────────────────────────────────────────────┘  │
│                                                              │
│   [Clear] [Export to File]                                   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

*This concludes Part 1 of the specification. The document continues in the next segment with:*

- Section 6: Service Registry Integration (detailed implementation)
- Section 7: IPC Design (Unix socket protocol)
- Section 8: File Operations (config save/load/validation)
- Section 9: Hardware Detection (GPU enumeration)
- Section 10: Certificate Generation
- Section 11: Implementation Guide (step-by-step)
- Section 12: Code Structure (complete module layout)
- Section 13: Testing Strategy

**Current Length:** ~8,500 words
**Estimated Total:** 25,000-30,000 words


## 6. Service Registry Integration & Detected Capabilities Display

### 6.1 Requirements

**Critical User Requirement:** The GUI must display ALL detected capabilities from the Service Registry in a clear, comprehensive format, and log this information when logging is enabled.

**Display Requirements:**
1. Show compositor name and version (e.g., "GNOME 46.0")
2. Show Portal version detected (v3, v4, v5)
3. Show Portal interface versions (ScreenCast v1/v2/v3, RemoteDesktop v1/v2)
4. Show all 18 Service Registry services with guarantee levels
5. Show platform quirks detected (e.g., "Avc444Unreliable on RHEL 9 + Mesa 22.x")
6. Show deployment context (Flatpak, Native, systemd-user, etc.)
7. Show selected session persistence strategy
8. Refreshable (user can re-probe capabilities)
9. Auto-log to file when logging enabled

---

###6.2 Detected Capabilities Display Specification

**Location:** Tab 8: Status & Service Registry

**UI Layout:**
```
┌─────────────────────────────────────────────────────────────────┐
│ DETECTED CAPABILITIES & SERVICE REGISTRY                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│ ▼ System Detection                                               │
│   ┌────────────────────────────────────────────────────────────┐│
│   │ Compositor:    GNOME 46.0 (Mutter 46.0)                   ││
│   │ Distribution:  Ubuntu 24.04 LTS (Noble Numbat)            ││
│   │ Kernel:        6.8.0-49-generic                           ││
│   │ Portal Version: 5                                         ││
│   │ Portal Backend: xdg-desktop-portal-gnome 46.2             ││
│   │                                                           ││
│   │ Portal Interfaces:                                        ││
│   │   • ScreenCast: v3 ✅                                     ││
│   │   • RemoteDesktop: v2 ✅                                  ││
│   │   • Secret: v1 ✅                                         ││
│   │                                                           ││
│   │ Deployment Context: Flatpak                               ││
│   │ XDG_RUNTIME_DIR: /run/user/1000                          ││
│   │                                                           ││
│   │ Platform Quirks Detected:                                 ││
│   │   • None detected ✅                                      ││
│   │   (or: "Avc444Unreliable: RHEL 9 + Mesa 22.x blur")     ││
│   │                                                           ││
│   │ Session Persistence Strategy: Portal + Token (Rejected)   ││
│   │   Reason: GNOME Portal rejects RemoteDesktop persistence ││
│   │   Fallback: Requires Mutter Direct API (native pkg)      ││
│   └────────────────────────────────────────────────────────────┘│
│                                                                  │
│   [  Refresh Detection  ]  [  Export Capabilities  ]            │
│                                                                  │
│ ▼ Service Advertisement Registry (18 Services)                  │
│   ┌────────────────────────────────────────────────────────────┐│
│   │ Service              Level       Wayland Source    RDP Cap ││
│   ├────────────────────────────────────────────────────────────┤│
│   │ ✅ VideoCapture      Guaranteed  PipeWire stream   EGFX    ││
│   │ ✅ RemoteInput       Guaranteed  Portal RD v2      Input   ││
│   │ ✅ Clipboard         Guaranteed  Portal RD v2      Cliprdr ││
│   │ ✅ DamageTracking    Guaranteed  Compositor hints  -       ││
│   │ ✅ MetadataCursor    Guaranteed  Portal metadata   Cursor  ││
│   │ 🔶 MultiMonitor      BestEffort  Portal multi      Multi   ││
│   │ 🔶 WindowCapture     BestEffort  Portal window     -       ││
│   │ ⚠️  SessionPersist    Degraded    Portal tokens     -      ││
│   │     Note: GNOME blocks RemoteDesktop persistence           ││
│   │ ❌ DmaBufZeroCopy    Unavailable  (GNOME MemFd)    -       ││
│   │ ❌ ExplicitSync      Unavailable  Not in GNOME     -       ││
│   │ ❌ HdrColorSpace     Unavailable  Future feature   -       ││
│   │ ❌ WlrScreencopy     Unavailable  Not wlroots       -       ││
│   │ ❌ WlrDirectInput    Unavailable  Not wlroots       -       ││
│   │ ✅ DirectCompositorAPI Guaranteed Mutter D-Bus      -       ││
│   │ ✅ CredentialStorage Guaranteed  Secret Portal     -       ││
│   │ ⚠️  UnattendedAccess  Degraded    Mutter Direct     -      ││
│   │ ❌ LibeiInput        Unavailable  Portal lacks EIS -       ││
│   │ 🔶 FractionalScaling BestEffort  Portal scale      -       ││
│   └────────────────────────────────────────────────────────────┘│
│                                                                  │
│   Summary Statistics:                                            │
│   • ✅ Guaranteed: 11 services (fully supported)                 │
│   • 🔶 BestEffort: 4 services (works with limitations)          │
│   • ⚠️  Degraded: 2 services (known issues)                      │
│   • ❌ Unavailable: 1 service (not supported)                    │
│                                                                  │
│   Performance Hints:                                             │
│   • Recommended FPS: 30                                          │
│   • Zero-copy: Not available (using memory copy)                 │
│   • Recommended codec: AVC444 (4:4:4 chroma supported)          │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

### 6.3 Data Sources for Capabilities Display

**Source 1: --show-capabilities Output**

The server provides a built-in capability report:
```bash
lamco-rdp-server --show-capabilities
```

**Output Format:**
```
╔════════════════════════════════════════════════════════════╗
║              Service Advertisement Registry                ║
╚════════════════════════════════════════════════════════════╝
  Compositor: GNOME 46.0
  Services: 6 guaranteed, 2 best-effort, 0 degraded, 3 unavailable
  ─────────────────────────────────────────────────────────
  ✅ Damage Tracking       [Guaranteed]
  ❌ DMA-BUF Zero-Copy     [Unavailable]
      ↳ Compositor prefers MemFd buffers
  ...
```

**GUI Integration:**
```rust
// Run server with --show-capabilities
let output = Command::new("lamco-rdp-server")
    .arg("--show-capabilities")
    .output()
    .await?;

// Parse output into structured data
let capabilities = parse_capabilities_output(&output.stdout)?;

// Display in table widget
self.service_registry = Some(capabilities);
```

---

### 6.4 Service Registry Data Structure

**GUI Internal Representation:**
```rust
#[derive(Debug, Clone)]
pub struct DetectedCapabilities {
    // System information
    pub compositor_name: String,           // "GNOME 46.0"
    pub distribution: String,              // "Ubuntu 24.04 LTS"
    pub kernel_version: String,            // "6.8.0-49-generic"

    // Portal information
    pub portal_version: u32,               // 5
    pub portal_backend: String,            // "xdg-desktop-portal-gnome 46.2"
    pub screencast_version: Option<u32>,   // Some(3)
    pub remote_desktop_version: Option<u32>, // Some(2)
    pub secret_portal_version: Option<u32>,  // Some(1)

    // Deployment
    pub deployment_context: DeploymentContext, // Flatpak, Native, etc.
    pub xdg_runtime_dir: PathBuf,          // /run/user/1000

    // Platform quirks
    pub quirks: Vec<PlatformQuirk>,        // [Avc444Unreliable, ...]

    // Session persistence
    pub persistence_strategy: String,      // "Portal + Token (GNOME rejects)"
    pub persistence_notes: Vec<String>,    // Explanation messages

    // Service Registry (18 services)
    pub services: Vec<ServiceInfo>,

    // Counts
    pub guaranteed_count: usize,
    pub best_effort_count: usize,
    pub degraded_count: usize,
    pub unavailable_count: usize,

    // Performance hints
    pub recommended_fps: Option<u32>,
    pub recommended_codec: Option<String>,
    pub zero_copy_available: bool,

    // Timestamp
    pub detected_at: chrono::DateTime<chrono::Local>,
}

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub id: String,                    // "VideoCapture"
    pub name: String,                  // "Video Capture"
    pub level: ServiceLevel,           // Guaranteed
    pub level_emoji: String,           // "✅"
    pub wayland_source: Option<String>, // "PipeWire stream"
    pub rdp_capability: Option<String>, // "EGFX"
    pub notes: Vec<String>,            // ["Portal v5 required"]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceLevel {
    Guaranteed,
    BestEffort,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone)]
pub struct PlatformQuirk {
    pub quirk_id: String,              // "Avc444Unreliable"
    pub description: String,           // "RHEL 9 + Mesa 22.x blur issue"
    pub impact: String,                // "AVC444 disabled, using AVC420"
}

#[derive(Debug, Clone)]
pub enum DeploymentContext {
    Native,
    Flatpak,
    SystemdUser { linger: bool },
    SystemdSystem,
    InitD,
}
```

---

### 6.5 Capabilities Table Widget

**iced Implementation:**
```rust
use iced::widget::{column, container, row, scrollable, text};
use iced::Length;

fn view_service_registry_table(
    services: &[ServiceInfo]
) -> Element<Message> {
    let header = row![
        text("Service").width(Length::FillPortion(3)),
        text("Level").width(Length::FillPortion(2)),
        text("Wayland Source").width(Length::FillPortion(3)),
        text("RDP Capability").width(Length::FillPortion(2)),
    ]
    .padding(10)
    .spacing(10);

    let mut rows_widget = column![header].spacing(2);

    for service in services {
        let level_color = match service.level {
            ServiceLevel::Guaranteed => Color::from_rgb(0.0, 0.8, 0.0),
            ServiceLevel::BestEffort => Color::from_rgb(1.0, 0.6, 0.0),
            ServiceLevel::Degraded => Color::from_rgb(0.9, 0.7, 0.0),
            ServiceLevel::Unavailable => Color::from_rgb(0.5, 0.5, 0.5),
        };

        let service_row = row![
            text(format!("{} {}", service.level_emoji, service.name))
                .width(Length::FillPortion(3)),

            text(&service.level.to_string())
                .width(Length::FillPortion(2))
                .style(move |_theme| text::Style {
                    color: Some(level_color),
                }),

            text(service.wayland_source.as_deref().unwrap_or("-"))
                .width(Length::FillPortion(3)),

            text(service.rdp_capability.as_deref().unwrap_or("-"))
                .width(Length::FillPortion(2)),
        ]
        .padding(5)
        .spacing(10);

        rows_widget = rows_widget.push(service_row);

        // Add notes if present
        for note in &service.notes {
            let note_row = row![
                text(""),
                text(format!("  ↳ {}", note))
                    .size(12)
                    .style(|_theme| text::Style {
                        color: Some(Color::from_rgb(0.4, 0.4, 0.4)),
                    }),
            ]
            .padding(2);

            rows_widget = rows_widget.push(note_row);
        }
    }

    scrollable(
        container(rows_widget)
            .padding(10)
            .style(container::Style::default())
    )
    .into()
}
```

---

### 6.6 Capabilities Logging Integration

**Requirement:** When logging is enabled, write complete capabilities report to log file at GUI startup.

**Implementation:**
```rust
// In GUI initialization (after loading capabilities)
fn log_detected_capabilities(
    caps: &DetectedCapabilities,
    log_file: Option<&Path>,
) -> Result<()> {
    if let Some(log_path) = log_file {
        let report = format_capabilities_report(caps);

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;

        writeln!(file, "\n╔═══════════════════════════════════════════════════════╗")?;
        writeln!(file, "║     DETECTED CAPABILITIES - GUI Startup               ║")?;
        writeln!(file, "║     {:<52}║", caps.detected_at.format("%Y-%m-%d %H:%M:%S"))?;
        writeln!(file, "╚═══════════════════════════════════════════════════════╝\n")?;

        writeln!(file, "SYSTEM INFORMATION:")?;
        writeln!(file, "  Compositor: {}", caps.compositor_name)?;
        writeln!(file, "  Distribution: {}", caps.distribution)?;
        writeln!(file, "  Kernel: {}", caps.kernel_version)?;
        writeln!(file, "")?;

        writeln!(file, "PORTAL DETECTION:")?;
        writeln!(file, "  Portal Version: {}", caps.portal_version)?;
        writeln!(file, "  Portal Backend: {}", caps.portal_backend)?;
        writeln!(file, "  ScreenCast Interface: v{}", caps.screencast_version.map(|v| v.to_string()).unwrap_or("N/A".to_string()))?;
        writeln!(file, "  RemoteDesktop Interface: v{}", caps.remote_desktop_version.map(|v| v.to_string()).unwrap_or("N/A".to_string()))?;
        writeln!(file, "")?;

        writeln!(file, "DEPLOYMENT:")?;
        writeln!(file, "  Context: {:?}", caps.deployment_context)?;
        writeln!(file, "  XDG_RUNTIME_DIR: {}", caps.xdg_runtime_dir.display())?;
        writeln!(file, "")?;

        if !caps.quirks.is_empty() {
            writeln!(file, "PLATFORM QUIRKS:")?;
            for quirk in &caps.quirks {
                writeln!(file, "  • {} - {}", quirk.quirk_id, quirk.description)?;
                writeln!(file, "    Impact: {}", quirk.impact)?;
            }
            writeln!(file, "")?;
        }

        writeln!(file, "SESSION PERSISTENCE:")?;
        writeln!(file, "  Strategy: {}", caps.persistence_strategy)?;
        for note in &caps.persistence_notes {
            writeln!(file, "  Note: {}", note)?;
        }
        writeln!(file, "")?;

        writeln!(file, "SERVICE REGISTRY ({} total services):", caps.services.len())?;
        writeln!(file, "  Summary: ✅ {} Guaranteed │ 🔶 {} BestEffort │ ⚠️  {} Degraded │ ❌ {} Unavailable",
            caps.guaranteed_count,
            caps.best_effort_count,
            caps.degraded_count,
            caps.unavailable_count)?;
        writeln!(file, "")?;

        writeln!(file, "  ┌────────────────────────────────────────────────────────┐")?;
        writeln!(file, "  │ Service                Level        Wayland → RDP      │")?;
        writeln!(file, "  ├────────────────────────────────────────────────────────┤")?;

        for service in &caps.services {
            let wayland = service.wayland_source.as_deref().unwrap_or("-");
            let rdp = service.rdp_capability.as_deref().unwrap_or("-");

            writeln!(file, "  │ {} {:<18} {:<11} {} → {}",
                service.level_emoji,
                service.name,
                format!("[{:?}]", service.level),
                wayland,
                rdp)?;

            for note in &service.notes {
                writeln!(file, "  │     ↳ {}", note)?;
            }
        }

        writeln!(file, "  └────────────────────────────────────────────────────────┘")?;
        writeln!(file, "")?;

        writeln!(file, "PERFORMANCE RECOMMENDATIONS:")?;
        if let Some(fps) = caps.recommended_fps {
            writeln!(file, "  • Recommended FPS: {}", fps)?;
        }
        if let Some(codec) = &caps.recommended_codec {
            writeln!(file, "  • Recommended Codec: {}", codec)?;
        }
        writeln!(file, "  • Zero-copy available: {}", if caps.zero_copy_available { "Yes" } else { "No" })?;

        writeln!(file, "\n{}\n", "=".repeat(60))?;

        file.sync_all()?;
    }

    Ok(())
}
```

---

### 6.7 Refresh Capabilities Button

**User Action:** Click "Refresh Detection" button

**Implementation:**
```rust
impl ConfigApp {
    fn refresh_capabilities(&mut self) -> Command<Message> {
        // Run lamco-rdp-server --show-capabilities
        Command::perform(
            async {
                let output = tokio::process::Command::new("lamco-rdp-server")
                    .arg("--show-capabilities")
                    .output()
                    .await?;

                if !output.status.success() {
                    return Err(anyhow!("Capabilities detection failed"));
                }

                let stdout = String::from_utf8_lossy(&output.stdout);
                parse_capabilities_output(&stdout)
            },
            |result| match result {
                Ok(caps) => Message::CapabilitiesDetected(caps),
                Err(e) => Message::Error(format!("Detection failed: {}", e)),
            },
        )
    }
}

// Message handler
Message::RefreshCapabilities => {
    self.refresh_capabilities()
}

Message::CapabilitiesDetected(caps) => {
    // Update UI state
    self.detected_capabilities = Some(caps.clone());

    // Log to file if enabled
    if let Some(ref log_file) = self.log_file_path {
        let _ = log_detected_capabilities(&caps, Some(log_file));
    }

    Command::none()
}
```

---

### 6.8 Export Capabilities Button

**User Action:** Click "Export Capabilities" button

**Exports to:**
- JSON format (machine-readable)
- Text format (human-readable, same as log)
- Both saved to user-selected location

**Implementation:**
```rust
Message::ExportCapabilities => {
    if let Some(ref caps) = self.detected_capabilities {
        // Show file save dialog
        Command::perform(
            async {
                let path = rfd::AsyncFileDialog::new()
                    .set_file_name("lamco-capabilities.json")
                    .save_file()
                    .await;

                if let Some(file) = path {
                    let json = serde_json::to_string_pretty(&caps)?;
                    tokio::fs::write(file.path(), json).await?;
                    Ok(file.path().to_path_buf())
                }
            },
            |result| match result {
                Ok(path) => Message::Info(format!("Exported to: {}", path.display())),
                Err(e) => Message::Error(format!("Export failed: {}", e)),
            },
        )
    } else {
        Command::none()
    }
}
```

---

## 7. Logging Integration

### 7.1 Logging Configuration in GUI

**Location:** Tab 7: Logging & Monitoring

**Parameters to Configure:**
1. Log level (error, warn, info, debug, trace)
2. Log format (pretty, compact, json)
3. Log file path (optional)
4. Enable file logging checkbox

**Mapping to CLI Arguments:**
```rust
// GUI config → Server CLI args
fn build_server_args(config: &LoggingConfig) -> Vec<String> {
    let mut args = vec![];

    // Verbosity
    match config.level.as_str() {
        "trace" => args.extend(&["-vv"]),
        "debug" => args.extend(&["-v"]),
        "info" => {}, // Default
        "warn" | "error" => {}, // Filter in RUST_LOG
        _ => {},
    }

    // Format
    args.push("--log-format".to_string());
    args.push(config.format.clone());

    // File logging
    if let Some(ref log_dir) = config.log_dir {
        args.push("--log-file".to_string());
        args.push(log_dir.join("lamco-rdp-server.log").display().to_string());
    }

    args
}
```

---

### 7.2 Live Log Viewer

**Features:**
- Tail server subprocess stdout/stderr
- Color-coded by log level
- Filterable (show only ERROR, or DEBUG+, etc.)
- Auto-scroll (can be toggled)
- Export to file
- Search capability

**Implementation:**
```rust
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStdout;

struct LogViewer {
    lines: Vec<LogLine>,
    filter_level: LogLevel,
    auto_scroll: bool,
    max_lines: usize, // Keep last N lines
}

#[derive(Debug, Clone)]
struct LogLine {
    timestamp: String,
    level: LogLevel,
    message: String,
    raw: String,
}

impl LogViewer {
    async fn tail_subprocess_output(
        stdout: ChildStdout,
        sender: mpsc::Sender<String>,
    ) {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await.ok().flatten() {
            let _ = sender.send(line).await;
        }
    }

    fn parse_log_line(raw: &str) -> LogLine {
        // Parse: "2026-01-19 14:23:45 [INFO] Server listening..."
        let parts: Vec<&str> = raw.splitn(4, ' ').collect();

        let timestamp = if parts.len() >= 2 {
            format!("{} {}", parts[0], parts[1])
        } else {
            "??:??:??".to_string()
        };

        let level = if parts.len() >= 3 {
            match parts[2].trim_matches(|c| c == '[' || c == ']') {
                "TRACE" => LogLevel::Trace,
                "DEBUG" => LogLevel::Debug,
                "INFO" => LogLevel::Info,
                "WARN" => LogLevel::Warn,
                "ERROR" => LogLevel::Error,
                _ => LogLevel::Info,
            }
        } else {
            LogLevel::Info
        };

        let message = parts.get(3).unwrap_or(&"").to_string();

        LogLine {
            timestamp,
            level,
            message,
            raw: raw.to_string(),
        }
    }

    fn view(&self) -> Element<Message> {
        let mut log_column = column![].spacing(1);

        for line in &self.lines {
            if self.should_show_line(&line) {
                let line_text = text(format!(
                    "{} [{:5}] {}",
                    line.timestamp,
                    format!("{:?}", line.level).to_uppercase(),
                    line.message
                ))
                .size(12)
                .style(move |_theme| {
                    let color = match line.level {
                        LogLevel::Error => Color::from_rgb(0.9, 0.0, 0.0),
                        LogLevel::Warn => Color::from_rgb(0.9, 0.7, 0.0),
                        LogLevel::Info => Color::from_rgb(0.0, 0.0, 0.0),
                        LogLevel::Debug => Color::from_rgb(0.3, 0.3, 0.8),
                        LogLevel::Trace => Color::from_rgb(0.5, 0.5, 0.5),
                    };
                    text::Style { color: Some(color) }
                });

                log_column = log_column.push(line_text);
            }
        }

        scrollable(log_column).height(Length::Fill).into()
    }

    fn should_show_line(&self, line: &LogLine) -> bool {
        line.level as u8 >= self.filter_level as u8
    }
}
```

**Message Handling:**
```rust
Message::LogLineReceived(line) => {
    let log_line = LogViewer::parse_log_line(&line);
    self.log_viewer.lines.push(log_line);

    // Trim to max lines
    if self.log_viewer.lines.len() > self.log_viewer.max_lines {
        self.log_viewer.lines.remove(0);
    }

    Command::none()
}
```

---

## 8. Complete Specification Status

**What the agent created (Sections 1-5):**
- ✅ Architecture overview (GUI-optional design)
- ✅ Complete configuration analysis (all 85+ parameters)
- ✅ GUI framework selection (iced)
- ✅ State management structure
- ✅ All 10 tab specifications with widgets

**What I've added:**
- ✅ Section 6: Service Registry Integration
  - Detected capabilities display specification
  - Complete data structures for capabilities
  - Service table widget implementation
  - Capabilities logging to file
  - Refresh mechanism
  - Export functionality

**Still needed (can be added now or you review first):**
- Section 7: IPC Design (optional real-time status)
- Section 8: File Operations (save/load/validate)
- Section 9: Hardware Detection
- Section 10: Certificate Generation
- Section 11-13: Implementation guide, code structure, testing

**Current document: 1,900+ lines, comprehensive coverage of GUI requirements including your detected capabilities display.**

**Should I:**
1. Continue adding remaining sections now?
2. Or is this sufficient for you to review and start implementation?

The specification is at: `~/wayland/wrd-server-specs/docs/GUI-SOFTWARE-DESIGN-SPECIFICATION.md`
