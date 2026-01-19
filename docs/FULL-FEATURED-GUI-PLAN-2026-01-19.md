# Full-Featured GUI Implementation Plan - lamco-rdp-server
**Date:** 2026-01-19
**Scope:** Complete configuration GUI with ALL options (automatic + manual override)
**Framework:** iced (preferred) with licensing analysis
**Timeline:** 6-8 weeks for comprehensive implementation

---

## Framework Licensing Analysis

### iced - RECOMMENDED (Per User Preference)

**License:** MIT OR Apache-2.0 (dual-licensed)
**Copyright:** Héctor Ramón and iced contributors

**Commercial Use:** ✅ **FULLY PERMITTED**
- Choose either MIT or Apache-2.0 license terms
- No royalties, no fees
- Can use in proprietary/closed-source products
- BSL 1.1 compatible (iced's permissive license allows inclusion)

**Attribution Requirement:**
- Include license text and copyright notice in distribution
- Do NOT need to open-source your code

**BSL Compatibility:** ✅ **PERFECT**
- iced is permissive open source
- lamco-rdp-server is BSL (source-available, commercial restrictions)
- No license conflicts
- iced can be bundled in BSL products

**Version:** 0.14 (January 2026 release) - Latest stable

**Sources:**
- [iced GitHub Repository](https://github.com/iced-rs/iced)
- [iced License](https://github.com/iced-rs/iced/blob/master/LICENSE)
- [iced Official Site](https://iced.rs/)

---

### GTK4 (gtk4-rs bindings)

**License:** MIT (Rust bindings only)
**Underlying GTK4 Library:** LGPL 2.1+

**Commercial Use:** ✅ **PERMITTED with considerations**
- Rust bindings (gtk4-rs): MIT licensed (permissive)
- GTK4 library itself: LGPL 2.1+ (copyleft for library modifications)

**LGPL Requirements:**
- Dynamic linking allowed without copyleft obligations
- If you modify GTK4 itself, must release modifications
- If you statically link, may need to provide object files for relinking
- Flatpak/shared library: No issues (dynamic linking)

**BSL Compatibility:** ✅ **COMPATIBLE**
- LGPL allows use in proprietary software (via dynamic linking)
- No conflicts with BSL

**Complexity:** LGPL requires legal review for some distributions (static linking)

**Sources:**
- [gtk4-rs GitHub](https://github.com/gtk-rs/gtk4-rs)
- [gtk4-rs License](https://github.com/gtk-rs/gtk4-rs/blob/main/LICENSE)

---

### egui

**License:** MIT OR Apache-2.0 (dual-licensed)
**Copyright:** Emil Ernerfeldt and egui contributors

**Commercial Use:** ✅ **FULLY PERMITTED**
- Same as iced (permissive dual-licensing)
- No restrictions on commercial use
- BSL compatible

**Sources:**
- [egui GitHub](https://github.com/emilk/egui)
- [egui License (Apache)](https://github.com/emilk/egui/blob/main/LICENSE-APACHE)

---

### Licensing Summary

| Framework | License | Commercial Use | BSL Compatible | Attribution |
|-----------|---------|----------------|----------------|-------------|
| **iced** | MIT OR Apache-2.0 | ✅ Yes | ✅ Yes | Include license text |
| **gtk4-rs** | MIT (bindings), LGPL 2.1+ (GTK) | ✅ Yes (dynamic link) | ✅ Yes | Include licenses |
| **egui** | MIT OR Apache-2.0 | ✅ Yes | ✅ Yes | Include license text |

**All options are legally compatible with BSL 1.1 for lamco-rdp-server.**

**User Preference: iced** → ✅ **Proceed with iced**

---

## Full-Featured GUI Scope

### Complete Configuration Coverage

Based on config.toml.example analysis, the GUI will expose **ALL 50+ configuration parameters** organized in a tabbed interface.

---

## Tab 1: Server Settings (Essential)

### Section: Network

**Parameters:**
- `listen_addr` - Text input with validation (IP:port format)
  - Default: "0.0.0.0:3389"
  - Validation: Valid IP + port 1-65535
  - Examples dropdown: 0.0.0.0:3389, 127.0.0.1:3389, [specific IP]:3389

- `max_connections` - Number input (1-100)
  - Default: 5
  - Slider with text override

- `session_timeout` - Number input (seconds, 0 = no timeout)
  - Default: 0
  - Tooltip: "Disconnect idle clients after N seconds (0 = never)"

- `use_portals` - Checkbox
  - Default: true
  - Tooltip: "Required for Wayland (always enabled)"
  - Locked: true (cannot disable)

---

### Section: TLS Certificates

**UI Layout:**
```
Certificate Management:
┌──────────────────────────────────────────────────────────┐
│ Certificate File:                                        │
│ [/etc/lamco-rdp-server/cert.pem           ] [Browse...] │
│                                                          │
│ Private Key File:                                        │
│ [/etc/lamco-rdp-server/key.pem            ] [Browse...] │
│                                                          │
│ Status: ✅ Valid until 2027-01-19                        │
│                                                          │
│ [  Generate Self-Signed Certificate  ]                  │
│ [  Import from Let's Encrypt...       ]                  │
│                                                          │
│ ℹ️  Production: Use Let's Encrypt                        │
│    certbot certonly --standalone -d rdp.yourdomain.com   │
└──────────────────────────────────────────────────────────┘
```

**Parameters:**
- `cert_path` - File browser (must exist, .pem extension)
- `key_path` - File browser (must exist, .pem extension)

**Actions:**
- "Generate Self-Signed" button:
  - Dialog: "Hostname for certificate?" (pre-filled with `hostname`)
  - Dialog: "Certificate location?" (default: ~/.config/lamco-rdp-server/)
  - Runs: `openssl req -x509 -newkey rsa:4096 ...`
  - Shows progress spinner
  - Auto-fills cert_path and key_path on success

- "Import from Let's Encrypt" button:
  - Dialog: "Domain name?"
  - Scans: /etc/letsencrypt/live/{domain}/
  - Auto-fills cert_path = fullchain.pem, key_path = privkey.pem

- Certificate validity check:
  - Parse cert.pem on load
  - Extract expiry date
  - Show: ✅ Valid, ⚠️ Expires soon (<30 days), ❌ Expired

---

### Section: Authentication

**Parameters:**
- `enable_nla` - Checkbox
  - Default: true
  - Tooltip: "Network Level Authentication (client auth before session)"

- `auth_method` - Radio buttons
  - Options: "none" | "pam"
  - Default: "pam"
  - Info message if PAM selected: "⚠️ PAM authentication only available in native packages (not Flatpak)"

- `require_tls_13` - Checkbox
  - Default: true
  - Tooltip: "Require TLS 1.3 or higher (recommended for security)"

---

## Tab 2: Video & Display (Important)

### Section: Video Encoding

**Parameters:**
- `max_fps` - Slider (5-60 FPS)
  - Default: 30
  - Labels: 5, 15, 30, 45, 60
  - Live value display

- `enable_damage_tracking` - Checkbox
  - Default: true
  - Tooltip: "Only send changed regions (90%+ bandwidth savings)"

- `preferred_format` - Dropdown (optional)
  - Options: Auto (default), BGRx, BGRA, RGBx, RGBA
  - Tooltip: "Pixel format (auto-detected if not set)"

---

### Section: EGFX (H.264 Encoding)

**Parameters:**
- `egfx.enabled` - Checkbox
  - Default: true
  - Tooltip: "Enable H.264 graphics pipeline (required for video)"

- `egfx.h264_bitrate` - Slider (1000-20000 kbps)
  - Default: 5000
  - Labels: 1 Mbps, 5 Mbps, 10 Mbps, 20 Mbps
  - Live value display: "5.0 Mbps"

- `egfx.codec` - Radio buttons
  - Options: auto | avc420 | avc444
  - Default: auto
  - Tooltips:
    - avc444: "4:4:4 chroma - perfect text quality, higher bandwidth"
    - avc420: "4:2:0 chroma - standard quality, lower bandwidth"
    - auto: "Use best available (Service Registry detection)"

- `egfx.periodic_idr_interval` - Number input (seconds, 0-60)
  - Default: 5
  - Tooltip: "Force keyframe every N seconds (clears artifacts)"

---

### Section: Multi-Monitor

**Parameters:**
- `multimon.enabled` - Checkbox
  - Default: true
  - Tooltip: "Support multiple displays"

- `multimon.max_monitors` - Number input (1-16)
  - Default: 4
  - Tooltip: "Maximum number of monitors to support"

---

### Section: Cursor

**Parameters:**
- `cursor.mode` - Dropdown
  - Options: metadata | painted | hidden | predictive
  - Default: metadata
  - Tooltips:
    - metadata: "Client-side rendering (lowest latency)"
    - painted: "Composited into video stream"
    - hidden: "No cursor shown"
    - predictive: "Latency compensation (for high-latency connections)"

- `cursor.auto_mode` - Checkbox
  - Default: true
  - Tooltip: "Auto-switch to predictive mode when latency high"

- `cursor.predictive_latency_threshold_ms` - Slider (20-200ms)
  - Default: 100
  - Tooltip: "Switch to predictive mode above this latency"

- `cursor.cursor_update_fps` - Slider (30-120 FPS)
  - Default: 60
  - Tooltip: "Cursor stream update rate"

---

## Tab 3: Input & Clipboard

### Section: Input

**Parameters:**
- `input.keyboard_layout` - Dropdown (optional)
  - Options: Auto (default), us, uk, de, fr, es, etc.
  - Tooltip: "Keyboard layout (auto-detected if not set)"

- `input.input_method` - Radio buttons
  - Options: portal | evdev
  - Default: portal
  - Tooltip:
    - portal: "XDG Desktop Portal (recommended, Wayland-native)"
    - evdev: "Direct input device (requires root permissions)"

---

### Section: Clipboard

**UI Layout:**
```
Clipboard Synchronization:
┌──────────────────────────────────────────────────────────┐
│ ☑ Enable clipboard sync                                  │
│                                                          │
│ Max clipboard size: [16] MB                              │
│ ──────────────────────────────────────────────────────  │
│ 1 MB                    16 MB                    64 MB   │
│                                                          │
│ Rate limiting: [200] ms minimum between events           │
│ ──────────────────────────────────────────────────────  │
│ 0 ms                   200 ms                  1000 ms   │
│ (Info: Prevents rapid changes from overwhelming Portal)  │
│                                                          │
│ Allowed MIME types: (leave empty for all)                │
│ [                                              ] [Add]   │
│ • text/plain                                    [Remove] │
│ • image/png                                     [Remove] │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

**Parameters:**
- `clipboard.enabled` - Checkbox
  - Default: true

- `clipboard.max_size` - Slider (1-64 MB)
  - Default: 16 MB (16777216 bytes)
  - Display: "16 MB" (convert bytes to MB)

- `clipboard.rate_limit_ms` - Slider (0-1000ms)
  - Default: 200
  - Tooltip: "Minimum milliseconds between clipboard events (0 = no limit)"

- `clipboard.allowed_types` - List widget
  - Default: [] (empty = all types)
  - Add/Remove MIME types
  - Common options dropdown: text/plain, text/html, image/png, image/jpeg, application/pdf

---

## Tab 4: Performance Tuning (Advanced)

### Section: Threading

**Parameters:**
- `performance.encoder_threads` - Slider (0-16)
  - Default: 0 (auto-detect)
  - 0 = "Auto (detect from CPU cores)"
  - Tooltip: "H.264 encoder threads (0 = auto from CPU)"

- `performance.network_threads` - Slider (0-16)
  - Default: 0
  - Tooltip: "Network I/O threads (0 = auto)"

- `performance.buffer_pool_size` - Slider (4-64)
  - Default: 16
  - Tooltip: "Frame buffer pool size (higher = more memory, smoother)"

- `performance.zero_copy` - Checkbox
  - Default: true
  - Tooltip: "Enable DMA-BUF zero-copy when available (lower CPU)"

---

### Section: Adaptive Frame Rate

**UI Layout:**
```
Adaptive FPS - Automatic frame rate adjustment:
┌──────────────────────────────────────────────────────────┐
│ ☑ Enable adaptive FPS                                    │
│                                                          │
│ Minimum FPS (static content):  [5 ]                     │
│ Maximum FPS (high activity):   [30]                     │
│                                                          │
│ Activity Thresholds:                                     │
│ • High activity (>30% changed):   [0.30] (30%)          │
│ • Medium activity (10-30%):       [0.10] (10%)          │
│ • Low activity (1-10%):           [0.01] (1%)           │
│                                                          │
│ ℹ️  Automatically adjusts frame rate based on screen     │
│    changes. Static desktop = 5 FPS, video = 60 FPS.     │
└──────────────────────────────────────────────────────────┘
```

**Parameters:**
- `performance.adaptive_fps.enabled` - Checkbox
- `performance.adaptive_fps.min_fps` - Number input (1-60)
- `performance.adaptive_fps.max_fps` - Number input (1-60)
- `performance.adaptive_fps.high_activity_threshold` - Slider (0.0-1.0)
- `performance.adaptive_fps.medium_activity_threshold` - Slider (0.0-1.0)
- `performance.adaptive_fps.low_activity_threshold` - Slider (0.0-1.0)

---

### Section: Latency Governor

**Parameters:**
- `performance.latency.mode` - Radio buttons
  - Options: interactive | balanced | quality
  - Default: balanced
  - Tooltips:
    - interactive: "Lowest latency (<50ms) - best for remote desktop"
    - balanced: "Balanced (<100ms) - good compromise"
    - quality: "Highest quality (<300ms) - best for video playback"

- `performance.latency.interactive_max_delay_ms` - Number input
  - Default: 16
  - Tooltip: "Max delay in interactive mode"

- `performance.latency.balanced_max_delay_ms` - Number input
  - Default: 33

- `performance.latency.quality_max_delay_ms` - Number input
  - Default: 100

---

## Tab 5: Damage Detection (Advanced)

### Section: Damage Tracking Configuration

**UI Layout:**
```
Damage Detection - Only encode changed screen regions:
┌──────────────────────────────────────────────────────────┐
│ ☑ Enable damage tracking                                 │
│                                                          │
│ Detection method: [●] Pixel diff  [ ] Future methods    │
│                                                          │
│ Tile size: [16] pixels                                   │
│ ────────────────────────────────────────────────────    │
│ 8px              16px              32px              64px│
│ (Smaller = more sensitive, more CPU)                     │
│                                                          │
│ Difference threshold: [1%] of tile pixels must change    │
│ Pixel value threshold: [1] (0-255)                      │
│ Merge distance: [16] pixels                              │
│ Minimum region area: [32] pixels                        │
│                                                          │
│ ℹ️  16x16 tiles with 1% threshold = 2-3 pixels minimum   │
│    Catches typing while filtering sub-pixel noise        │
└──────────────────────────────────────────────────────────┘
```

**Parameters:**
- `damage_tracking.enabled` - Checkbox
- `damage_tracking.method` - Radio buttons (currently only "diff")
- `damage_tracking.tile_size` - Slider (8, 16, 32, 64 pixels)
- `damage_tracking.diff_threshold` - Slider (0.0-1.0, display as %)
- `damage_tracking.pixel_threshold` - Slider (0-255)
- `damage_tracking.merge_distance` - Number input (0-256 pixels)
- `damage_tracking.min_region_area` - Number input (1-1024 pixels)

---

## Tab 6: Hardware Encoding (Optional)

### Section: GPU Acceleration

**UI Layout:**
```
Hardware Encoding Configuration:
┌──────────────────────────────────────────────────────────┐
│ ☑ Enable hardware encoding                               │
│                                                          │
│ VA-API device: [/dev/dri/renderD128        ] [Browse]   │
│                                                          │
│ ☑ Enable DMA-BUF zero-copy                               │
│ ☑ Fallback to software if hardware fails                │
│ ☑ Prefer NVENC over VA-API (when both available)        │
│                                                          │
│ Quality preset:                                          │
│ ( ) Speed (low latency)                                  │
│ (●) Balanced                                             │
│ ( ) Quality (best visual)                                │
│                                                          │
│ ℹ️  Requires --features vaapi or --features nvenc build  │
│    Flatpak: Software encoding only (hardware disabled)   │
│                                                          │
│ Detected hardware:                                       │
│ • Intel UHD Graphics 770 (VA-API available) ✅           │
│ • NVIDIA RTX 4090 (NVENC available) ✅                   │
└──────────────────────────────────────────────────────────┘
```

**Parameters:**
- `hardware_encoding.enabled` - Checkbox
- `hardware_encoding.vaapi_device` - File browser (device path)
- `hardware_encoding.enable_dmabuf_zerocopy` - Checkbox
- `hardware_encoding.fallback_to_software` - Checkbox
- `hardware_encoding.quality_preset` - Radio buttons
- `hardware_encoding.prefer_nvenc` - Checkbox

**Smart Behavior:**
- Auto-detect GPU hardware (enumerate /dev/dri/renderD*)
- Show which encoders are available (query VA-API, NVENC)
- Disable controls if hardware encoding not compiled in (check features)
- Show warning in Flatpak: "Hardware encoding unavailable (Flatpak limitation)"

---

## Tab 7: Logging & Monitoring

### Section: Logging Configuration

**Parameters:**
- `logging.level` - Dropdown
  - Options: error | warn | info | debug | trace
  - Default: info
  - Tooltip: "Can also set RUST_LOG environment variable"

- `logging.format` - Radio buttons
  - Options: pretty | compact | json
  - Default: pretty
  - Preview pane showing sample log line in each format

- `logging.log_file` - File path input (optional)
  - Default: empty (stderr only)
  - Browse button
  - Checkbox: "Enable file logging"
  - Example: /var/log/lamco-rdp-server/server.log

---

### Section: Live Log Viewer

**UI Layout:**
```
Server Logs (Last 200 lines):
┌──────────────────────────────────────────────────────────┐
│ Filter: [info ▼]  [⟳ Refresh]  [🗑️ Clear]  ☑ Auto-scroll │
├──────────────────────────────────────────────────────────┤
│ 2026-01-19 12:00:01 [INFO] Server listening on 0.0.0.0...│
│ 2026-01-19 12:00:02 [INFO] Service Registry: ✅6 🔶2 ❌3  │
│ 2026-01-19 12:00:05 [INFO] Client connected: 192.168.1.5 │
│ 2026-01-19 12:00:06 [DEBUG] Video: AVC444 selected       │
│ ...                                                      │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

**Features:**
- Read from server subprocess stdout/stderr
- Filter by log level
- Auto-scroll checkbox
- Color-coded by level (red=error, yellow=warn, etc.)
- Search capability
- Export to file button

---

## Tab 8: Service Registry & Status (Runtime)

### Section: Detected Capabilities

**UI Layout:**
```
Runtime Service Discovery:
┌──────────────────────────────────────────────────────────┐
│ Compositor: GNOME 46.0 (Ubuntu 24.04)                    │
│ Portal Version: 5                                        │
│ Deployment: Flatpak                                      │
│                                                          │
│ [  Refresh Capabilities  ]                               │
│                                                          │
│ Service Advertisement Registry:                          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ Service              Level        RDP Capability    │  │
│ ├────────────────────────────────────────────────────┤  │
│ │ ✅ Video Capture     Guaranteed   ScreenCast        │  │
│ │ ✅ Remote Input      Guaranteed   Input injection   │  │
│ │ ✅ Damage Tracking   Guaranteed   Bandwidth opt     │  │
│ │ ✅ Metadata Cursor   Guaranteed   Client cursor     │  │
│ │ 🔶 Multi-Monitor     BestEffort   Multi-display     │  │
│ │ 🔶 Clipboard         BestEffort   Clipboard sync    │  │
│ │ ⚠️  Session Persist   Degraded     Tokens rejected   │  │
│ │ ❌ DMA-BUF ZeroCopy  Unavailable  (GNOME MemFd)     │  │
│ │ ❌ wlr-screencopy    Unavailable  (Not wlroots)     │  │
│ └────────────────────────────────────────────────────┘  │
│                                                          │
│ Summary: 6 guaranteed, 2 best-effort, 1 degraded, 3 N/A │
│                                                          │
│ Selected Strategy: Portal + Token (GNOME blocks)         │
│ ℹ️  Use native package + Mutter Direct for zero dialogs  │
└──────────────────────────────────────────────────────────┘
```

**Features:**
- Run `--show-capabilities` and parse output
- Display in table with visual indicators
- Color-coded by level
- Expandable details (click service for explanation)
- Refresh button (re-probe at runtime)
- Show selected session persistence strategy
- Platform-specific notes (e.g., "GNOME blocks persistence")

---

### Section: Active Server Status

**UI Layout:**
```
Server Status:
┌──────────────────────────────────────────────────────────┐
│ Status: 🟢 RUNNING                                        │
│ Uptime: 2h 34m                                           │
│ PID: 12345                                               │
│                                                          │
│ Active Connections: 1                                    │
│ ┌────────────────────────────────────────────────────┐  │
│ │ Client IP       Connected    Encoding   Bandwidth  │  │
│ ├────────────────────────────────────────────────────┤  │
│ │ 192.168.1.100   12:05:23     AVC444     4.2 Mbps   │  │
│ └────────────────────────────────────────────────────┘  │
│                                                          │
│ Performance:                                             │
│ • FPS: 30 (adaptive)                                     │
│ • Latency: ~12ms                                         │
│ • Bandwidth: 4.2 Mbps                                    │
│ • Frames encoded: 1,234                                  │
│                                                          │
│ [  Disconnect All  ]  [  Restart Server  ]               │
└──────────────────────────────────────────────────────────┘
```

**Features:**
- Poll server status via D-Bus or monitoring socket
- Show active connections (parse from server state)
- Real-time performance metrics
- Disconnect clients button
- Restart server button (stop + start)

---

## Tab 9: Presets & Quick Setup

### Section: Configuration Presets

**UI Layout:**
```
Quick Configuration Presets:
┌──────────────────────────────────────────────────────────┐
│ Load Preset:                                             │
│                                                          │
│ [  Desktop Sharing (Default)  ]                          │
│ • 30 FPS, AVC444, damage tracking                       │
│ • Portal authentication, TLS 1.3                        │
│ • Optimal for remote desktop work                       │
│                                                          │
│ [  Low Bandwidth (<2 Mbps)  ]                            │
│ • 15 FPS, AVC420, aggressive damage detection           │
│ • Lower quality, minimal bandwidth                      │
│ • Good for slow connections                             │
│                                                          │
│ [  Maximum Quality (LAN)  ]                              │
│ • 60 FPS, AVC444, hardware encoding                     │
│ • Highest quality, assumes fast network                 │
│ • Best for local network use                            │
│                                                          │
│ [  Production Server  ]                                  │
│ • PAM auth, TLS required, logging enabled               │
│ • Multi-monitor, session persistence configured         │
│ • Suitable for unattended operation                     │
│                                                          │
│ Save current config as preset: [My Custom] [Save]       │
│                                                          │
│ [  Restore Defaults  ]  [  Import Config...  ]           │
└──────────────────────────────────────────────────────────┘
```

**Features:**
- Predefined configurations for common use cases
- One-click apply preset
- Save custom presets (stored in XDG_CONFIG_HOME)
- Import existing config.toml
- Export current config
- Restore factory defaults

---

## Tab 10: Advanced Settings (All Remaining Options)

### Collapsible Sections for All Advanced Parameters

**Section: Color Space (Advanced)**
- Color space selection (BT.709, BT.601, sRGB)
- Full/limited range selection
- VUI signaling options

**Section: Session Persistence (Advanced)**
- Credential storage backend selection
- Token encryption settings
- Session restore behavior

**Section: Network Tuning (Advanced)**
- TCP buffer sizes
- Keepalive settings
- Network QoS

**Section: Debug Options (Developer)**
- Enable protocol tracing
- Dump frames to disk
- Performance profiling
- Memory debugging

---

## GUI Architecture (iced)

### Application Structure

```rust
// New crate: lamco-rdp-server-gui

use iced::{
    widget::{button, checkbox, column, container, radio, row, scrollable, slider, text, text_input},
    Application, Command, Element, Settings, Theme,
};

#[derive(Debug, Clone)]
enum Tab {
    ServerSettings,
    VideoDisplay,
    InputClipboard,
    Performance,
    DamageDetection,
    HardwareEncoding,
    LoggingMonitoring,
    ServiceRegistry,
    Presets,
    Advanced,
}

struct ConfigApp {
    // Configuration state
    config: lamco_rdp_server::Config,  // Shared from main crate
    config_path: PathBuf,

    // UI state
    active_tab: Tab,
    server_status: ServerStatus,
    server_process: Option<Child>,

    // Service Registry (runtime data)
    service_registry: Option<ServiceRegistry>,
    capabilities_output: String,

    // Log viewer
    log_lines: Vec<String>,
    log_auto_scroll: bool,

    // Validation errors
    errors: Vec<String>,
}

#[derive(Debug, Clone)]
enum Message {
    // Tab navigation
    TabSelected(Tab),

    // Server section
    ListenAddrChanged(String),
    MaxConnectionsChanged(u32),
    SessionTimeoutChanged(u32),

    // TLS section
    CertPathChanged(String),
    KeyPathChanged(String),
    BrowseCertPath,
    BrowseKeyPath,
    GenerateSelfSignedCert,
    ImportLetsEncrypt,

    // Video section
    MaxFpsChanged(u32),
    DamageTrackingToggled(bool),
    CodecSelected(String),

    // EGFX section
    BitrateChanged(u32),
    PeriodicIdrChanged(u32),

    // Clipboard section
    ClipboardToggled(bool),
    MaxClipboardSizeChanged(u64),
    RateLimitChanged(u32),
    AddAllowedMimeType(String),
    RemoveAllowedMimeType(usize),

    // Performance section
    EncoderThreadsChanged(u32),
    NetworkThreadsChanged(u32),
    BufferPoolSizeChanged(u32),
    ZeroCopyToggled(bool),

    // Adaptive FPS
    AdaptiveFpsToggled(bool),
    MinFpsChanged(u32),
    MaxFpsChanged(u32),
    HighThresholdChanged(f32),
    MediumThresholdChanged(f32),
    LowThresholdChanged(f32),

    // Latency Governor
    LatencyModeSelected(LatencyMode),
    InteractiveDelayChanged(u32),
    BalancedDelayChanged(u32),
    QualityDelayChanged(u32),

    // Damage Detection
    DamageMethodSelected(String),
    TileSizeChanged(u32),
    DiffThresholdChanged(f32),
    PixelThresholdChanged(u8),
    MergeDistanceChanged(u32),
    MinRegionAreaChanged(u32),

    // Hardware Encoding
    HardwareEncodingToggled(bool),
    VaapiDeviceChanged(String),
    DmaBufToggled(bool),
    FallbackToggled(bool),
    QualityPresetSelected(String),
    PreferNvencToggled(bool),

    // Logging
    LogLevelSelected(String),
    LogFormatSelected(String),
    LogFileChanged(String),
    LogFileToggled(bool),

    // Multi-monitor
    MultimonToggled(bool),
    MaxMonitorsChanged(u32),

    // Cursor
    CursorModeSelected(String),
    AutoModeToggled(bool),
    PredictiveThresholdChanged(u32),
    CursorFpsChanged(u32),

    // Input
    KeyboardLayoutSelected(String),
    InputMethodSelected(String),

    // Auth
    NlaToggled(bool),
    AuthMethodSelected(String),
    RequireTls13Toggled(bool),

    // Server control
    StartServer,
    StopServer,
    RestartServer,
    RefreshCapabilities,

    // Configuration management
    SaveConfig,
    LoadConfig,
    ImportConfig,
    ExportConfig,
    RestoreDefaults,
    LoadPreset(String),
    SavePreset(String),

    // Events from server subprocess
    ServerStarted(u32),  // PID
    ServerStopped,
    ServerError(String),
    LogLineReceived(String),
    CapabilitiesUpdated(ServiceRegistry),
}

impl Application for ConfigApp {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        // Load config from file or create default
        let config = load_config().unwrap_or_default();

        (
            ConfigApp {
                config,
                config_path: default_config_path(),
                active_tab: Tab::ServerSettings,
                server_status: ServerStatus::Stopped,
                server_process: None,
                service_registry: None,
                capabilities_output: String::new(),
                log_lines: Vec::new(),
                log_auto_scroll: true,
                errors: Vec::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Lamco RDP Server Configuration".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ListenAddrChanged(addr) => {
                self.config.server.listen_addr = addr;
                Command::none()
            }
            Message::StartServer => {
                // Save config first
                let _ = self.save_config();

                // Launch server subprocess
                match Command::new("lamco-rdp-server")
                    .arg("--config")
                    .arg(&self.config_path)
                    .spawn()
                {
                    Ok(child) => {
                        self.server_process = Some(child);
                        self.server_status = ServerStatus::Running;
                    }
                    Err(e) => {
                        self.errors.push(format!("Failed to start server: {}", e));
                    }
                }
                Command::none()
            }
            Message::GenerateSelfSignedCert => {
                // Run generate-certs.sh script
                Command::perform(
                    generate_certificate(self.config_path.clone()),
                    |result| match result {
                        Ok((cert, key)) => Message::CertPathChanged(cert),
                        Err(e) => Message::ServerError(e),
                    },
                )
            }
            // ... handle all other messages
            _ => Command::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        // Tab bar
        let tabs = row![
            tab_button("Server", Tab::ServerSettings, self.active_tab),
            tab_button("Video", Tab::VideoDisplay, self.active_tab),
            tab_button("Input", Tab::InputClipboard, self.active_tab),
            tab_button("Performance", Tab::Performance, self.active_tab),
            tab_button("Damage", Tab::DamageDetection, self.active_tab),
            tab_button("Hardware", Tab::HardwareEncoding, self.active_tab),
            tab_button("Logs", Tab::LoggingMonitoring, self.active_tab),
            tab_button("Status", Tab::ServiceRegistry, self.active_tab),
            tab_button("Presets", Tab::Presets, self.active_tab),
            tab_button("Advanced", Tab::Advanced, self.active_tab),
        ]
        .spacing(5);

        // Tab content
        let content = match self.active_tab {
            Tab::ServerSettings => self.view_server_settings(),
            Tab::VideoDisplay => self.view_video_display(),
            Tab::InputClipboard => self.view_input_clipboard(),
            Tab::Performance => self.view_performance(),
            Tab::DamageDetection => self.view_damage_detection(),
            Tab::HardwareEncoding => self.view_hardware_encoding(),
            Tab::LoggingMonitoring => self.view_logging_monitoring(),
            Tab::ServiceRegistry => self.view_service_registry(),
            Tab::Presets => self.view_presets(),
            Tab::Advanced => self.view_advanced(),
        };

        // Main layout
        column![
            tabs,
            content,
            self.view_bottom_toolbar(),
        ]
        .into()
    }
}

impl ConfigApp {
    fn view_server_settings(&self) -> Element<Message> {
        column![
            text("Server Settings").size(24),

            // Listen address
            row![
                text("Listen Address:"),
                text_input("0.0.0.0:3389", &self.config.server.listen_addr)
                    .on_input(Message::ListenAddrChanged),
            ],

            // Max connections
            row![
                text("Max Connections:"),
                slider(1..=100, self.config.server.max_connections, Message::MaxConnectionsChanged),
                text(format!("{}", self.config.server.max_connections)),
            ],

            // TLS certificate section
            self.view_tls_section(),

            // Authentication section
            self.view_auth_section(),
        ]
        .spacing(10)
        .padding(20)
        .into()
    }

    fn view_bottom_toolbar(&self) -> Element<Message> {
        row![
            // Server control
            match self.server_status {
                ServerStatus::Stopped => button("▶ Start Server").on_press(Message::StartServer),
                ServerStatus::Running => button("⏹ Stop Server").on_press(Message::StopServer),
                ServerStatus::Error => button("🔄 Restart Server").on_press(Message::RestartServer),
            },

            // Config management
            button("💾 Save Configuration").on_press(Message::SaveConfig),
            button("📂 Load Configuration").on_press(Message::LoadConfig),

            // Status indicator
            container(
                text(match self.server_status {
                    ServerStatus::Stopped => "⚫ Stopped",
                    ServerStatus::Running => "🟢 Running",
                    ServerStatus::Error => "🔴 Error",
                })
            ),
        ]
        .spacing(10)
        .padding(10)
        .into()
    }
}
```

---

## Advanced GUI Features

### 1. Configuration Validation

**Real-time validation:**
- Listen address format check (IP:port validation)
- Port range validation (1-65535)
- Certificate file existence check
- Certificate expiry date parsing and warning
- Configuration consistency checks

**Visual feedback:**
- ✅ Green checkmark for valid fields
- ❌ Red X for invalid fields
- ⚠️ Yellow warning for deprecation or issues
- Tooltip explaining validation error

---

### 2. Smart Defaults & Auto-Detection

**Hardware Detection:**
- Auto-detect available GPUs (enumerate /dev/dri/)
- Check VA-API availability (`vainfo`)
- Check NVENC availability (query NVIDIA driver)
- Pre-fill hardware encoding settings

**Compositor Detection:**
- Parse `--show-capabilities` output
- Pre-fill recommended settings based on compositor
- GNOME: Enable Portal strategy
- wlroots: Suggest wlr-direct (if native package)

**Network Detection:**
- Auto-detect hostname (`gethostname()`)
- Suggest certificate CN
- Detect available network interfaces

---

### 3. Contextual Help System

**Tooltips:**
- Every widget has tooltip explaining what it does
- Advanced options have "Learn More" links to documentation

**Info Panels:**
- Platform-specific notes (e.g., "Flatpak limitations")
- Service Registry explanations
- Performance tuning guidance

**Guided Wizards:**
- "First Time Setup" wizard on first launch
- Step-by-step: TLS → Authentication → Video settings → Done
- Can skip wizard and go to full config

---

### 4. Manual Override Capability

**Auto vs Manual Toggle:**

Every auto-detected setting has manual override option:

```
Codec Selection:
(●) Automatic (Service Registry detection)
    Detected: AVC444 (GNOME 46 supports it)

( ) Manual override:
    [ avc420 ▼]  Force specific codec
    ⚠️  Overrides Service Registry detection
```

**Implementation:**
```rust
pub struct CodecConfig {
    auto: bool,
    manual_override: Option<Codec>,
}

// In config.toml:
[egfx]
codec = "auto"  # or "avc420" / "avc444" for manual

// In GUI:
if auto {
    // Show detected value from Service Registry
    // Allow toggle to manual
} else {
    // Show dropdown for manual selection
    // Allow toggle back to auto
}
```

**Applies to:**
- Video codec (auto vs manual avc420/avc444)
- Frame rate (adaptive vs fixed)
- Keyboard layout (auto-detect vs manual)
- Encoder selection (auto vs force software/hardware)
- Pixel format (auto vs manual BGRx/RGBA)
- Damage detection parameters (auto-tune vs manual thresholds)

---

### 5. Live Configuration Reload

**Hot-reload support:**
- Change video settings while server running
- FPS adjustment without restart
- Bitrate changes applied immediately
- Logging level changes instantly

**Requires:**
- Server modification: Watch config file for changes
- Or: GUI sends updates via IPC (D-Bus signals)
- Or: GUI communicates with server via monitoring socket

**Implementation:**
```rust
// Server watches config file
use notify::Watcher;

let watcher = notify::recommended_watcher(|res| {
    if let Ok(event) = res {
        // Reload config.toml
        // Apply changes to running server
    }
})?;

watcher.watch(&config_path, RecursiveMode::NonRecursive)?;
```

---

## Development Plan (Full-Featured GUI)

### Week 1-2: Foundation & Core Tabs

**Tasks:**
- Set up iced project structure
- Implement tabbed interface (10 tabs)
- Create Tab 1: Server Settings (complete)
- Create Tab 2: Video & Display (complete)
- Basic config loading/saving
- Window chrome, menu bar

**Deliverable:** Basic GUI with first 2 tabs functional

---

### Week 3-4: Input, Performance, Damage Tabs

**Tasks:**
- Tab 3: Input & Clipboard (complete with MIME type list editor)
- Tab 4: Performance Tuning (adaptive FPS, latency governor, threading)
- Tab 5: Damage Detection (all parameters with sliders and tooltips)
- Configuration validation logic

**Deliverable:** 5 tabs complete, config validation working

---

### Week 5: Hardware, Logging, Status Tabs

**Tasks:**
- Tab 6: Hardware Encoding (GPU detection, device enumeration)
- Tab 7: Logging & Monitoring (log viewer, filtering, export)
- Tab 8: Service Registry & Status (parse capabilities, display table)
- Server process management (start/stop/restart)

**Deliverable:** 8 tabs complete, server control working

---

### Week 6: Presets, Advanced, Smart Features

**Tasks:**
- Tab 9: Presets & Quick Setup (preset system, import/export)
- Tab 10: Advanced Settings (all remaining options)
- Smart defaults implementation (hardware detection, auto-fill)
- First-time setup wizard

**Deliverable:** All 10 tabs complete, smart features working

---

### Week 7: Polish & Integration

**Tasks:**
- Manual override toggles for all auto-detected settings
- Contextual help system (tooltips, info panels)
- Error handling and validation messaging
- Visual polish (spacing, alignment, colors)
- Icon integration

**Deliverable:** Production-ready GUI, polished UX

---

### Week 8: Testing & Documentation

**Tasks:**
- Test on GNOME, KDE, Sway
- Test all configuration permutations
- Test server start/stop/restart
- Documentation (how to use GUI)
- Screenshot creation for Flathub
- Update MetaInfo XML with GUI description

**Deliverable:** Tested, documented, Flathub-ready

---

## iced-Specific Implementation Details

### Dependencies

```toml
[package]
name = "lamco-rdp-server-gui"
version = "0.9.0"
edition = "2021"

[dependencies]
# GUI framework
iced = { version = "0.14", features = ["tokio", "advanced"] }
iced_aw = "0.11"  # Additional widgets (tabs, cards, etc.)

# Shared config
lamco-rdp-server = { path = "../lamco-rdp-server", default-features = false }

# Process management
tokio = { version = "1.35", features = ["process", "io-util"] }

# File operations
dirs = "5.0"
notify = "7.0"  # Config file watching

# Serialization
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"

# Certificate parsing
x509-parser = "0.16"
chrono = "0.4"

# GPU detection
libva = { version = "0.7", optional = true }
nvidia-video-codec-sdk = { version = "0.4", optional = true }
```

---

### Custom Widgets

**iced_aw (iced Additional Widgets) provides:**
- Tabs widget (built-in tabbed interface)
- Card widget (grouped sections)
- Number input (spinbox with validation)
- Color picker (if needed for themes)
- Modal dialogs
- Split panes

**Custom widgets to build:**

**1. SliderWithValue**
```rust
pub struct SliderWithValue {
    value: u32,
    range: RangeInclusive<u32>,
    label: String,
}

// Shows: [━━━━●━━━━] 30 FPS
```

**2. FilePathInput**
```rust
pub struct FilePathInput {
    path: String,
    browse_button: button::State,
}

// Shows: [/path/to/file.pem    ] [Browse...]
```

**3. ServiceRegistryTable**
```rust
pub struct ServiceRegistryTable {
    services: Vec<AdvertisedService>,
    sort_column: Column,
}

// Sortable table with colored indicators
```

**4. LogViewer**
```rust
pub struct LogViewer {
    lines: Vec<LogLine>,
    filter: LogLevel,
    auto_scroll: bool,
}

// Scrollable, filterable, color-coded log view
```

---

### Theme & Styling

**iced theming:**
```rust
use iced::{Theme, theme};

// Use Lamco brand colors
let custom_theme = Theme::custom(theme::Custom {
    palette: theme::Palette {
        background: Color::from_rgb(0.95, 0.95, 0.95),
        text: Color::from_rgb(0.1, 0.1, 0.1),
        primary: Color::from_rgb(0.055, 0.647, 0.914),  // #0EA5E9 teal
        success: Color::from_rgb(0.0, 0.8, 0.0),
        danger: Color::from_rgb(0.9, 0.0, 0.0),
    },
});
```

**Visual consistency:**
- Use brand teal for primary actions
- Status indicators match brand (🟢 green, 🔴 red, 🔶 orange)
- Professional, clean layout
- Not overly stylized (function over form)

---

## Binary & Distribution Impact

### Binary Sizes

**iced GUI binary:**
- iced framework: ~5-8 MB
- Custom widgets: ~1 MB
- Total GUI binary: ~10-15 MB

**Combined distribution:**
- `lamco-rdp-server`: 15 MB (server)
- `lamco-rdp-server-gui`: 15 MB (GUI)
- **Total: 30 MB** (both binaries)

**Flatpak impact:**
- Current: 6.7 MB compressed, 24 MB installed
- With GUI: ~15 MB compressed, ~45 MB installed
- Still reasonable for Flatpak

---

### Deployment Structure

```
Flatpak installs both binaries:
/app/bin/
  ├── lamco-rdp-server       # Server daemon (CLI)
  └── lamco-rdp-server-gui   # Configuration GUI

Desktop file launches GUI:
/app/share/applications/io.lamco.rdp-server.desktop
  Exec=lamco-rdp-server-gui

AppImage bundles both:
lamco-rdp-server-x86_64.AppImage
  ├── usr/bin/lamco-rdp-server
  └── usr/bin/lamco-rdp-server-gui
  (AppRun symlinks to lamco-rdp-server-gui)
```

---

## Configuration File Management

### Smart Config Handling

**Config file locations (XDG compliant):**
```
~/.config/lamco-rdp-server/
  ├── config.toml           # Active configuration
  ├── presets/
  │   ├── desktop-sharing.toml
  │   ├── low-bandwidth.toml
  │   ├── maximum-quality.toml
  │   ├── production-server.toml
  │   └── user-custom-1.toml
  └── last-known-good.toml  # Backup before changes
```

**Save workflow:**
1. Validate all fields
2. Backup current config to last-known-good.toml
3. Write new config.toml
4. If server running: offer to reload or restart
5. Confirm save success

**Error recovery:**
- If config.toml invalid on load, offer to restore last-known-good
- If server fails to start, show validation errors
- "Test Configuration" button (validate without saving)

---

## Advanced GUI Features (Full-Featured)

### Feature 1: Configuration Profiles

**Profile Manager:**
```
Profiles:
┌──────────────────────────────────────────────────────────┐
│ Current Profile: [Desktop Sharing ▼]                     │
│                                                          │
│ Available Profiles:                                      │
│ • Desktop Sharing (Default)                              │
│ • Low Bandwidth                                          │
│ • Maximum Quality                                        │
│ • Production Server                                      │
│ • My Custom Setup                                        │
│                                                          │
│ [  New Profile  ]  [  Duplicate  ]  [  Delete  ]         │
│ [  Import from File...  ]  [  Export...  ]               │
└──────────────────────────────────────────────────────────┘
```

**Profile switching:**
- Instant switch between profiles
- Each profile = separate config.toml
- Confirm before switching if unsaved changes

---

### Feature 2: Configuration Comparison

**Compare two profiles side-by-side:**
```
Compare Configurations:
┌─────────────────────────────────────────────────────────────┐
│ Profile 1: Desktop Sharing     Profile 2: Low Bandwidth    │
├────────────────────────────────┬────────────────────────────┤
│ max_fps: 30                    │ max_fps: 15  ⚠️ Different  │
│ codec: auto                    │ codec: avc420  ⚠️ Different│
│ h264_bitrate: 5000            │ h264_bitrate: 2000 ⚠️ Diff │
│ damage_tracking: true          │ damage_tracking: true  ✓   │
└────────────────────────────────┴────────────────────────────┘
```

---

### Feature 3: Performance Recommendations

**AI-like suggestions based on detected hardware:**

```
Performance Recommendations:
┌──────────────────────────────────────────────────────────┐
│ Based on your system (Intel i7-12700, 32GB RAM):        │
│                                                          │
│ ✅ Enable hardware encoding (VA-API detected)            │
│    [  Apply  ]                                           │
│                                                          │
│ ⚠️  Max FPS set to 30, but hardware can handle 60       │
│    [  Increase to 60 FPS  ]                              │
│                                                          │
│ ℹ️  Damage tracking enabled (good for bandwidth)         │
│    Current settings optimal for your hardware.           │
└──────────────────────────────────────────────────────────┘
```

---

### Feature 4: Connection Management

**Active connections panel:**
- List all connected clients
- Show per-client stats (bandwidth, FPS, latency)
- Disconnect individual clients
- View client capabilities
- Session duration
- Client OS detection (Windows, macOS, Linux)

---

### Feature 5: Real-Time Performance Graphs

**Using iced's plotting capabilities:**

```
Performance Monitoring:
┌──────────────────────────────────────────────────────────┐
│ Frame Rate (FPS):                                        │
│  60 ┤                                                     │
│  45 ┤     ╭─╮                                            │
│  30 ┤─────╯ ╰─────────────                               │
│  15 ┤                                                     │
│   0 ┴─────────────────────────────────────────────────  │
│     0s         30s         60s         90s        120s   │
│                                                          │
│ Bandwidth (Mbps):                                        │
│  10 ┤                                                     │
│   5 ┤─────╮  ╭──────╮                                    │
│   0 ┤     ╰──╯      ╰────────                            │
│     0s         30s         60s         90s        120s   │
└──────────────────────────────────────────────────────────┘
```

**Metrics tracked:**
- FPS over time (adaptive changes visible)
- Bandwidth usage
- Latency measurements
- CPU usage
- Memory usage
- Frame drops

---

### Feature 6: Configuration Export/Import

**Export options:**
- Export as TOML (standard format)
- Export as JSON (for programmatic use)
- Export as environment variables (for container deployment)
- Export as systemd service file (ready to install)

**Import options:**
- Import from TOML file
- Import from running server (extract current config via IPC)
- Merge configurations (combine two config files)

---

## iced vs GTK4 - Detailed Comparison for This Project

### iced Advantages

**For lamco-rdp-server GUI:**

✅ **Pure Rust**
- No C FFI complexity
- Type-safe across entire stack
- Easier debugging (all Rust)

✅ **Cross-Platform**
- Works on Linux, Windows, macOS
- Future: If lamco-rdp-server adds Windows support, GUI already portable
- Consistent look across platforms

✅ **Smaller Binary**
- ~15 MB vs GTK's ~35 MB
- Important for AppImage (smaller download)

✅ **Modern Development Experience**
- Elm architecture (clean state management)
- Reactive updates (change state → UI updates automatically)
- No callback spaghetti

✅ **Active Development**
- iced 0.14 released January 2026
- Used by COSMIC desktop (production-grade validation)
- Growing ecosystem

✅ **User Preference**
- You explicitly prefer iced
- Familiarity reduces development time

**Sources:**
- [iced GitHub](https://github.com/iced-rs/iced)
- [iced 0.14 Release](https://www.phoronix.com/news/Iced-0.14-Rust-GUI-LIbrary)

---

### GTK4 Advantages

**For Linux-native system tools:**

✅ **Native Linux Integration**
- Looks native on GNOME/KDE
- File pickers match desktop environment
- Accessibility built-in (screen readers, high contrast)

✅ **Mature Ecosystem**
- More widgets available (complex tables, tree views)
- Better documentation for Linux-specific features
- Flathub reviewers very familiar with GTK apps

✅ **Desktop Integration**
- System dialogs (file chooser, color picker)
- GNOME/KDE theme integration
- Native-looking on Linux

❌ **Drawbacks:**
- Larger binary (LGPL complexity for some)
- C library dependency (GTK4)
- Less modern development experience

---

### Recommendation: iced

**Reasoning:**
1. **User preference** (iced) → Faster development for you
2. **Pure Rust** aligns with project (all-Rust codebase)
3. **Smaller binary** benefits AppImage users
4. **Modern framework** easier to maintain long-term
5. **Cross-platform** future-proofs if Windows support added
6. **COSMIC uses iced** validates production-readiness for Wayland tools

**GTK4 advantages (native look, accessibility) are minor** compared to development velocity with familiar framework.

**Sources:**
- [Rust GUI Framework Comparison](https://an4t.com/rust-gui-libraries-compared/)
- [iced vs GTK Performance](http://lukaskalbertodt.github.io/2023/02/03/tauri-iced-egui-performance-comparison.html)

---

## Timeline & Effort Estimate

### Full-Featured GUI Development

**Total Effort:** 6-8 weeks (240-320 hours)

**Breakdown:**
- **Weeks 1-2:** Foundation, Server & Video tabs (20%)
- **Weeks 3-4:** Input, Performance, Damage tabs (30%)
- **Weeks 5:** Hardware, Logging, Status tabs (20%)
- **Weeks 6:** Presets, Advanced, Smart features (15%)
- **Weeks 7:** Polish, manual overrides, help system (10%)
- **Weeks 8:** Testing, documentation, Flathub prep (5%)

**Critical Path:**
- Server control (start/stop) - Week 5
- Configuration save/load - Week 2
- Service Registry display - Week 5
- All parameters exposed - Week 6

---

### Maintenance Burden

**Ongoing:**
- GUI bugs separate from server bugs
- iced version updates (0.14 → 0.15 → ...)
- Additional configuration parameters (as server evolves)
- Platform-specific GUI issues
- Screenshot updates for Flathub

**Estimated:** +20-25% maintenance burden vs CLI-only

---

## Flathub Implications

### MetaInfo Changes (With GUI)

```xml
<!-- Change from: -->
<component type="console-application">

<!-- To: -->
<component type="desktop-application">
  <id>io.lamco.rdp-server</id>
  <name>Lamco RDP Server</name>
  <summary>Remote access to your Linux desktop via RDP</summary>

  <!-- Add screenshots of GUI -->
  <screenshots>
    <screenshot type="default">
      <caption>Server configuration interface</caption>
      <image>https://raw.githubusercontent.com/lamco-admin/lamco-rdp-server/main/docs/screenshots/gui-config.png</image>
    </screenshot>
    <screenshot>
      <caption>Performance monitoring and status</caption>
      <image>https://raw.githubusercontent.com/lamco-admin/lamco-rdp-server/main/docs/screenshots/gui-status.png</image>
    </screenshot>
    <screenshot>
      <caption>Service Registry capabilities</caption>
      <image>https://raw.githubusercontent.com/lamco-admin/lamco-rdp-server/main/docs/screenshots/gui-services.png</image>
    </screenshot>
  </screenshots>
```

### Desktop File Changes

```desktop
[Desktop Entry]
Type=Application
Name=Lamco RDP Server
GenericName=Remote Desktop Server Configuration
Comment=Configure and manage remote desktop server
Icon=io.lamco.rdp-server
Exec=lamco-rdp-server-gui
Terminal=false  # Changed from true
Categories=Network;RemoteAccess;System;Settings;
Keywords=rdp;remote;desktop;server;wayland;configuration;
StartupNotify=true
```

**Flathub Assessment with GUI:** ✅ **Highly Likely to Accept**
- Desktop application with full GUI
- Screenshots demonstrating functionality
- Professional presentation
- Follows all Flathub guidelines

---

## Strategic Recommendation

### Implement Full-Featured GUI with iced

**Scope:** ALL configuration parameters exposed (50+ options across 10 tabs)

**Framework:** iced 0.14 (MIT OR Apache-2.0 license)

**Timeline:** 8 weeks development

**Benefits:**
1. ✅ Flathub acceptance (desktop application)
2. ✅ Dramatically improved UX (especially TLS setup)
3. ✅ Visual Service Registry display (unique differentiator)
4. ✅ Real-time monitoring and feedback
5. ✅ Manual override for all auto-detected settings
6. ✅ Configuration presets lower barrier to entry
7. ✅ Professional appearance in app stores

**Effort Justification:**
- GUI isn't just for Flathub - it's a genuine UX improvement
- TLS certificate setup is major pain point (one-click solves this)
- Service Registry visualization helps users understand capabilities
- Desktop sharing users WANT simple configuration
- 8 weeks investment for years of better UX

**Licensing:** ✅ No issues (iced MIT/Apache-2.0 compatible with BSL 1.1)

---

## Phased Rollout Strategy

### Phase 1: Minimal Viable GUI (3 weeks)

**Scope:**
- Tabs 1-3 only (Server, Video, Input)
- TLS certificate generation
- Start/stop server
- Basic status display
- Save/load config

**Goal:** Flathub acceptance
**Deliverable:** Resubmit to Flathub with desktop-application type

---

### Phase 2: Full Features (5 weeks additional)

**Scope:**
- All 10 tabs
- Performance monitoring
- Service Registry visualization
- Presets system
- Manual overrides
- Live reload

**Goal:** Production-quality configuration tool
**Deliverable:** v1.0 with comprehensive GUI

---

**Recommendation: Start with Phase 1 (3 weeks) to unblock Flathub, then add Phase 2 features for v1.1.**

**Next steps:**
1. Wait for Flathub PR #7627 feedback (3-7 days)
2. Begin iced GUI development (Phase 1)
3. Resubmit to Flathub when GUI ready

**Should I create detailed iced implementation guide with code structure and widget specifications?**

**Sources:**
- [iced Documentation](https://book.iced.rs/)
- [iced Examples](https://github.com/iced-rs/iced/blob/master/examples/README.md)
- [iced Awesome List](https://github.com/iced-rs/awesome-iced)

---

**END OF FULL-FEATURED GUI ANALYSIS**
