# Configuration Reference

This document describes all configuration options for `lamco-rdp-server`.

## Configuration Methods

Configuration can be provided via three methods (in order of precedence):

1. **Command-line arguments** (highest priority)
2. **Environment variables** (prefixed with `LAMCO_RDP_`)
3. **TOML configuration file** (lowest priority)

### Configuration File Locations

Default search order:
1. Path specified with `-c/--config` flag
2. `./config.toml` (current directory)
3. `~/.config/lamco-rdp-server/config.toml` (user config)
4. `/etc/lamco-rdp-server/config.toml` (system config)

## Complete Configuration Example

```toml
[server]
listen_addr = "0.0.0.0:3389"
max_connections = 5
session_timeout = 0
use_portals = true

[security]
cert_path = "certs/cert.pem"
key_path = "certs/key.pem"
enable_nla = false
auth_method = "none"
require_tls_13 = false

[video]
encoder = "auto"
vaapi_device = "/dev/dri/renderD128"
target_fps = 30
bitrate = 4000
damage_tracking = true
cursor_mode = "metadata"

[video_pipeline.processor]
target_fps = 30
max_queue_depth = 30
adaptive_quality = true
damage_threshold = 0.05
drop_on_full_queue = true
enable_metrics = true

[video_pipeline.dispatcher]
channel_size = 30
priority_dispatch = true
max_frame_age_ms = 150
enable_backpressure = true
high_water_mark = 0.8
low_water_mark = 0.5
load_balancing = true

[video_pipeline.converter]
buffer_pool_size = 8
enable_simd = true
damage_threshold = 0.75
enable_statistics = true

[input]
use_libei = true
keyboard_layout = "auto"
enable_touch = false

[clipboard]
enabled = true
max_size = 10485760
rate_limit_ms = 200
allowed_types = []

[multimon]
enabled = true
max_monitors = 4

[performance]
encoder_threads = 0
network_threads = 0
buffer_pool_size = 16
zero_copy = true

[logging]
level = "info"
metrics = true
```

## Section: `[server]`

Server-level configuration for network and connections.

### `listen_addr`

- **Type**: String
- **Default**: `"0.0.0.0:3389"`
- **Description**: Address and port to listen on
- **Examples**:
  - `"0.0.0.0:3389"` - Listen on all interfaces, default RDP port
  - `"127.0.0.1:5000"` - Listen on localhost only, custom port
  - `"::0:3389"` - Listen on all IPv6 interfaces

### `max_connections`

- **Type**: Integer
- **Default**: `5`
- **Range**: 1-100
- **Description**: Maximum concurrent client connections
- **Note**: Each connection consumes system resources (CPU, memory, bandwidth)

### `session_timeout`

- **Type**: Integer (seconds)
- **Default**: `0` (disabled)
- **Description**: Idle session timeout in seconds, 0 = unlimited
- **Example**: `3600` for 1 hour timeout

### `use_portals`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Use XDG Desktop Portal for screen capture
- **Note**: Must be `true` for Portal mode (Wayland)

## Section: `[security]`

Security and authentication settings.

### `cert_path`

- **Type**: String
- **Default**: `"certs/cert.pem"`
- **Description**: Path to TLS certificate file
- **Note**: Required for secure connections

### `key_path`

- **Type**: String
- **Default**: `"certs/key.pem"`
- **Description**: Path to TLS private key file
- **Permissions**: Should be mode 600 (read-only by owner)

### `enable_nla`

- **Type**: Boolean
- **Default**: `false`
- **Description**: Enable Network Level Authentication (NLA)
- **Note**: Requires valid system authentication (PAM)
- **Security**: Recommended for production deployments

### `auth_method`

- **Type**: String (enum)
- **Default**: `"none"`
- **Options**:
  - `"none"` - No authentication (testing only)
  - `"pam"` - System authentication via PAM
- **Security**: Use `"pam"` in production

### `require_tls_13`

- **Type**: Boolean
- **Default**: `false`
- **Description**: Require TLS 1.3 (reject TLS 1.2 clients)
- **Security**: Enable for maximum security if all clients support TLS 1.3

## Section: `[video]`

Video capture and encoding configuration.

### `encoder`

- **Type**: String (enum)
- **Default**: `"auto"`
- **Options**:
  - `"auto"` - Automatically select best available encoder
  - `"h264"` - H.264 via OpenH264
  - `"vaapi"` - Hardware encoding via VAAPI (if available)
- **Note**: VAAPI requires compatible GPU and drivers

### `vaapi_device`

- **Type**: String
- **Default**: `"/dev/dri/renderD128"`
- **Description**: VAAPI render device path
- **Note**: Only used when encoder is "vaapi" or "auto" with VAAPI available

### `target_fps`

- **Type**: Integer
- **Default**: `30`
- **Range**: 10-60
- **Description**: Target frames per second for screen capture
- **Performance**: Lower FPS reduces CPU/bandwidth usage

### `bitrate`

- **Type**: Integer (kbps)
- **Default**: `4000`
- **Range**: 500-20000
- **Description**: Video encoding bitrate in kilobits per second
- **Quality**: Higher bitrate = better quality but more bandwidth

### `damage_tracking`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Only encode changed screen regions
- **Performance**: Significantly reduces CPU and bandwidth usage

### `cursor_mode`

- **Type**: String (enum)
- **Default**: `"metadata"`
- **Options**:
  - `"metadata"` - Send cursor as separate metadata (efficient)
  - `"rendered"` - Render cursor in video frames

## Section: `[video_pipeline.processor]`

Frame processing pipeline settings.

### `target_fps`

- **Type**: Integer
- **Default**: `30`
- **Description**: Processing target frame rate
- **Note**: Should match `[video] target_fps`

### `max_queue_depth`

- **Type**: Integer
- **Default**: `30`
- **Description**: Maximum frames in processing queue
- **Performance**: Prevents memory buildup under load

### `adaptive_quality`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Automatically adjust quality based on system load
- **Performance**: Maintains smooth streaming under varying conditions

### `damage_threshold`

- **Type**: Float
- **Default**: `0.05`
- **Range**: 0.0-1.0
- **Description**: Minimum screen change fraction to encode frame (5%)
- **Performance**: Reduces encoding of nearly-static frames

### `drop_on_full_queue`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Drop frames when queue is full (maintain real-time)
- **Note**: Prevents latency buildup

### `enable_metrics`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Collect processing pipeline metrics
- **Performance**: Minimal overhead

## Section: `[video_pipeline.dispatcher]`

Frame distribution and prioritization settings.

### `channel_size`

- **Type**: Integer
- **Default**: `30`
- **Description**: Channel buffer size for frame dispatch

### `priority_dispatch`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Prioritize newer frames over older ones
- **Latency**: Reduces lag by skipping old frames

### `max_frame_age_ms`

- **Type**: Integer (milliseconds)
- **Default**: `150`
- **Description**: Maximum frame age before dropping
- **Latency**: Prevents showing outdated frames

### `enable_backpressure`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Apply backpressure to slow down capture when encoder can't keep up

### `high_water_mark`

- **Type**: Float
- **Default**: `0.8`
- **Range**: 0.0-1.0
- **Description**: Queue fullness to trigger backpressure (80%)

### `low_water_mark`

- **Type**: Float
- **Default**: `0.5`
- **Range**: 0.0-1.0
- **Description**: Queue fullness to release backpressure (50%)

### `load_balancing`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Balance load across processing threads

## Section: `[video_pipeline.converter]`

Frame conversion settings (color space, scaling).

### `buffer_pool_size`

- **Type**: Integer
- **Default**: `8`
- **Description**: Size of reusable buffer pool
- **Memory**: Pre-allocates buffers to reduce allocation overhead

### `enable_simd`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Use SIMD instructions for color conversion
- **Performance**: Significant speedup on supported CPUs

### `damage_threshold`

- **Type**: Float
- **Default**: `0.75`
- **Range**: 0.0-1.0
- **Description**: Minimum change to trigger conversion (75%)

### `enable_statistics`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Collect conversion statistics

## Section: `[input]`

Input handling configuration.

### `use_libei`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Use libei for input injection (Portal mode)
- **Note**: Required for Wayland Portal mode

### `keyboard_layout`

- **Type**: String
- **Default**: `"auto"`
- **Description**: Keyboard layout mapping
- **Options**:
  - `"auto"` - Detect from system
  - Specific layout name (e.g., `"us"`, `"uk"`, `"de"`)

### `enable_touch`

- **Type**: Boolean
- **Default**: `false`
- **Description**: Enable touch input support
- **Note**: Experimental feature

## Section: `[clipboard]`

Clipboard synchronization settings.

### `enabled`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Enable clipboard sync between client and server

### `max_size`

- **Type**: Integer (bytes)
- **Default**: `10485760` (10 MB)
- **Description**: Maximum clipboard content size
- **Security**: Prevents denial-of-service via large clipboard data

### `rate_limit_ms`

- **Type**: Integer (milliseconds)
- **Default**: `200`
- **Description**: Minimum time between clipboard events
- **Performance**: `200ms` = max 5 events/second, `0` = disabled
- **Security**: Prevents clipboard flooding

### `allowed_types`

- **Type**: Array of strings
- **Default**: `[]` (all types allowed)
- **Description**: Whitelist of allowed MIME types
- **Examples**:
  - `["text/plain"]` - Text only
  - `["text/plain", "image/png"]` - Text and PNG images

## Section: `[multimon]`

Multi-monitor support settings.

### `enabled`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Enable multi-monitor support

### `max_monitors`

- **Type**: Integer
- **Default**: `4`
- **Range**: 1-16
- **Description**: Maximum number of monitors to support

## Section: `[performance]`

Performance tuning settings.

### `encoder_threads`

- **Type**: Integer
- **Default**: `0` (auto-detect)
- **Description**: Number of encoder threads
- **Note**: `0` = use CPU count

### `network_threads`

- **Type**: Integer
- **Default**: `0` (auto-detect)
- **Description**: Number of network I/O threads
- **Note**: `0` = use CPU count

### `buffer_pool_size`

- **Type**: Integer
- **Default**: `16`
- **Description**: Global buffer pool size
- **Memory**: Pre-allocates buffers for zero-allocation paths

### `zero_copy`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Use zero-copy DMA-BUF when available
- **Performance**: Eliminates frame copying for significant speedup

## Section: `[logging]`

Logging configuration.

### `level`

- **Type**: String (enum)
- **Default**: `"info"`
- **Options**: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`
- **Description**: Log verbosity level

### `metrics`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Enable performance metrics logging

## Environment Variables

All configuration options can be set via environment variables:

```bash
# Format: LAMCO_RDP_<SECTION>_<KEY>
export LAMCO_RDP_SERVER_LISTEN_ADDR="0.0.0.0:5000"
export LAMCO_RDP_SECURITY_ENABLE_NLA="true"
export LAMCO_RDP_VIDEO_TARGET_FPS="60"
export LAMCO_RDP_LOGGING_LEVEL="debug"
```

## Command-Line Arguments

Key options available as CLI flags:

```bash
lamco-rdp-server \
  --config /path/to/config.toml \  # Config file path
  --listen 0.0.0.0:5000 \           # Listen address
  --port 5000 \                     # Port (alternative to listen)
  -vv \                             # Verbose logging (debug level)
  --log-format json                 # Log format (json|pretty|compact)
```

## Performance Tuning Guide

### Low Latency (Gaming, Video Playback)

```toml
[video]
target_fps = 60
damage_tracking = true

[video_pipeline.processor]
drop_on_full_queue = true

[video_pipeline.dispatcher]
priority_dispatch = true
max_frame_age_ms = 100
```

### Low Bandwidth (Mobile, Remote Networks)

```toml
[video]
target_fps = 15
bitrate = 1000
damage_tracking = true

[video_pipeline.processor]
adaptive_quality = true
damage_threshold = 0.10
```

### High Quality (LAN, Fast Networks)

```toml
[video]
target_fps = 60
bitrate = 8000
encoder = "vaapi"  # If GPU available

[video_pipeline.processor]
damage_threshold = 0.01
```

## Security Best Practices

### Production Configuration

```toml
[server]
listen_addr = "0.0.0.0:3389"

[security]
enable_nla = true
auth_method = "pam"
require_tls_13 = true

[clipboard]
max_size = 1048576  # 1 MB limit
rate_limit_ms = 500  # Max 2/sec
allowed_types = ["text/plain"]  # Text only
```

## Troubleshooting

**High CPU usage**: Reduce `target_fps`, enable `adaptive_quality`

**High bandwidth**: Reduce `bitrate`, increase `damage_threshold`

**Laggy input**: Enable `priority_dispatch`, reduce `max_frame_age_ms`

**Frame drops**: Increase `max_queue_depth`, reduce `target_fps`

**Out of memory**: Reduce `buffer_pool_size`, `max_connections`

## See Also

- [INSTALL.md](INSTALL.md) - Installation instructions
- [README.md](README.md) - General documentation
- [LICENSE](LICENSE) - Licensing terms
