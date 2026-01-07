# Configuration Reference

This document describes the authoritative configuration format for lamco-rdp-server.

## Configuration File Location

The authoritative configuration template is:
- **lamco-rdp-server**: `config.toml.example`
- **wrd-server-specs**: `config/config.toml.example`

Copy to `config.toml` and adjust paths for your environment.

## Required vs Optional Sections

### Required Sections (must be present)

| Section | Description |
|---------|-------------|
| `[server]` | Server listen address and connection settings |
| `[security]` | TLS certificates and authentication |
| `[video]` | Video encoding settings |
| `[video_pipeline.*]` | Frame processing pipeline |
| `[input]` | Keyboard and mouse settings |
| `[clipboard]` | Clipboard synchronization |
| `[multimon]` | Multi-monitor support |
| `[performance]` | Threading and buffer settings |
| `[logging]` | Log level and metrics |

### Optional Sections (have sensible defaults)

| Section | Description |
|---------|-------------|
| `[egfx]` | Graphics pipeline extension (H.264) |
| `[damage_tracking]` | Screen change detection |
| `[hardware_encoding]` | VA-API/NVENC settings |
| `[display]` | Resolution and DPI settings |
| `[advanced_video]` | Frame skip and quality settings |
| `[cursor]` | Cursor rendering mode |

## Section Reference

### [server]

```toml
[server]
listen_addr = "0.0.0.0:3389"    # IP:port to listen on
max_connections = 10            # Max concurrent connections
session_timeout = 0             # Timeout in seconds (0 = none)
use_portals = true              # Use XDG Desktop Portals
```

### [security]

```toml
[security]
cert_path = "/path/to/cert.pem" # TLS certificate (PEM)
key_path = "/path/to/key.pem"   # TLS private key (PEM)
enable_nla = false              # Network Level Authentication
auth_method = "none"            # "pam" or "none"
require_tls_13 = false          # Require TLS 1.3+
```

**Valid `auth_method` values:** `pam`, `none`

### [video]

```toml
[video]
encoder = "auto"                    # "auto", "openh264", "vaapi"
vaapi_device = "/dev/dri/renderD128"
target_fps = 30
bitrate = 4000                      # kbps
damage_tracking = true
cursor_mode = "metadata"            # "metadata", "embedded", "hidden"
```

**Valid `encoder` values:** `auto`, `openh264`, `vaapi`
**Valid `cursor_mode` values:** `metadata`, `embedded`, `hidden`

### [video_pipeline]

```toml
[video_pipeline.processor]
target_fps = 30
max_queue_depth = 30
adaptive_quality = true
damage_threshold = 0.05         # 0.0-1.0
drop_on_full_queue = true
enable_metrics = true

[video_pipeline.dispatcher]
channel_size = 30
priority_dispatch = true
max_frame_age_ms = 150
enable_backpressure = true
high_water_mark = 0.8           # 0.0-1.0
low_water_mark = 0.5            # 0.0-1.0
load_balancing = true

[video_pipeline.converter]
buffer_pool_size = 8
enable_simd = true
damage_threshold = 0.75         # 0.0-1.0
enable_statistics = true
```

### [input]

```toml
[input]
use_libei = true                # Use libei for Wayland input
keyboard_layout = "auto"        # "auto" or XKB name
enable_touch = false
```

### [clipboard]

```toml
[clipboard]
enabled = true
max_size = 10485760             # 10 MB
rate_limit_ms = 200             # Min ms between events
allowed_types = []              # Empty = all types
```

### [multimon]

```toml
[multimon]
enabled = true
max_monitors = 4
```

### [performance]

```toml
[performance]
encoder_threads = 0             # 0 = auto
network_threads = 0             # 0 = auto
buffer_pool_size = 16
zero_copy = true

[performance.adaptive_fps]      # Optional subsection
enabled = true
min_fps = 5
max_fps = 30
high_activity_threshold = 0.30
medium_activity_threshold = 0.10
low_activity_threshold = 0.01

[performance.latency]           # Optional subsection
mode = "balanced"               # "interactive", "balanced", "quality"
interactive_max_delay_ms = 16
balanced_max_delay_ms = 33
quality_max_delay_ms = 100
balanced_damage_threshold = 0.02
quality_damage_threshold = 0.05
```

### [logging]

```toml
[logging]
level = "info"                  # "trace", "debug", "info", "warn", "error"
metrics = true
# log_dir = "/var/log/..."      # Optional
```

### [egfx] (Optional)

```toml
[egfx]
enabled = true
h264_level = "auto"
h264_bitrate = 5000
zgfx_compression = "never"      # "never", "auto", "always"
max_frames_in_flight = 3
frame_ack_timeout = 5000
codec = "avc420"                # "avc420", "avc444"
qp_min = 10
qp_max = 40
qp_default = 23
avc444_aux_bitrate_ratio = 0.5
color_matrix = "auto"
color_range = "auto"
avc444_enabled = true
avc444_enable_aux_omission = true
avc444_max_aux_interval = 30
avc444_aux_change_threshold = 0.05
avc444_force_aux_idr_on_return = false
```

### [damage_tracking] (Optional)

```toml
[damage_tracking]
enabled = true
method = "diff"                 # "pipewire", "diff", "hybrid"
tile_size = 64
diff_threshold = 0.05
merge_distance = 32
```

### [hardware_encoding] (Optional)

```toml
[hardware_encoding]
enabled = false
vaapi_device = "/dev/dri/renderD128"
enable_dmabuf_zerocopy = true
fallback_to_software = true
quality_preset = "balanced"     # "speed", "balanced", "quality"
prefer_nvenc = true
```

### [display] (Optional)

```toml
[display]
allow_resize = true
allowed_resolutions = []
dpi_aware = false
allow_rotation = false
```

### [advanced_video] (Optional)

```toml
[advanced_video]
enable_frame_skip = true
scene_change_threshold = 0.7
intra_refresh_interval = 300
enable_adaptive_quality = false
```

## Validation Rules

The server validates on startup:

1. `listen_addr` must be valid `ip:port` format
2. `cert_path` and `key_path` must exist
3. `encoder` must be `auto`, `openh264`, or `vaapi`
4. `cursor_mode` must be `metadata`, `embedded`, or `hidden`
5. `qp_min <= qp_default <= qp_max`
6. `codec` must be `avc420` or `avc444`
7. `zgfx_compression` must be `never`, `auto`, or `always`
8. `damage_tracking.method` must be `pipewire`, `diff`, or `hybrid`
9. `quality_preset` must be `speed`, `balanced`, or `quality`

## Environment-Specific Configs

For deployment, create environment-specific configs:

```
config/
  config.toml.example     # Authoritative template
  rhel9-config.toml       # RHEL 9 deployment
  dev-config.toml         # Local development
```

Always derive from `config.toml.example` and only change paths.
