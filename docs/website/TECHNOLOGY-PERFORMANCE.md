# Technology: Performance Features

**URL:** `https://lamco.ai/technology/performance/`
**Status:** Draft for review

---

## Overview

Remote desktop performance isn't just about encoding speed. It's about the entire pipeline: detecting what changed, encoding efficiently, transmitting only what's needed, and making input feel responsive despite network latency.

lamco-rdp-server includes a suite of premium performance features that work together to deliver a responsive experience across varying network conditions.

---

## Adaptive Frame Rate

### The Problem with Fixed Frame Rate

Most remote desktop solutions use a fixed frame rate—typically 30 FPS regardless of what's on screen.

This creates two problems:
1. **Static content:** Wasting bandwidth and power encoding unchanging frames
2. **Active content:** 30 FPS may not be smooth enough for video or gaming

### Dynamic FPS Based on Activity

lamco-rdp-server continuously monitors screen activity and adjusts frame rate accordingly:

```
Screen Activity Detection
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  Static (<1% changed)          ──────────►  5 FPS           │
│      │                                      (power saving)  │
│      ▼                                                      │
│  Low Activity (1-10%)          ──────────►  15 FPS          │
│  (typing, cursor movement)                  (responsive)    │
│      │                                                      │
│      ▼                                                      │
│  Medium Activity (10-30%)      ──────────►  20-30 FPS       │
│  (scrolling, window movement)               (smooth)        │
│      │                                                      │
│      ▼                                                      │
│  High Activity (>30%)          ──────────►  30-60 FPS       │
│  (video, animation)                         (fluid)         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Benefits

| Scenario | Fixed 30 FPS | Adaptive FPS | Improvement |
|----------|--------------|--------------|-------------|
| Reading document | 30 FPS | 5 FPS | 6x less bandwidth |
| Typing code | 30 FPS | 15 FPS | 2x less bandwidth |
| Watching video | 30 FPS | 60 FPS | 2x smoother |
| Battery (laptop) | Constant drain | Reduced drain | Significant savings |

### Configuration

```toml
[performance.adaptive_fps]
enabled = true
min_fps = 5            # Floor for static content
max_fps = 60           # Ceiling for active content
ramp_up_speed = 0.3    # How quickly to increase FPS
ramp_down_speed = 0.1  # How quickly to decrease FPS

# Activity thresholds (fraction of screen changed)
static_threshold = 0.01         # <1% = static
low_activity_threshold = 0.10   # 1-10% = low
high_activity_threshold = 0.30  # >30% = high
```

---

## Damage Tracking

### What Is Damage Tracking?

Most screen updates are localized—a cursor moves, text appears in an editor, a notification pops up. The rest of the screen stays the same.

Damage tracking identifies exactly which regions changed, so only those regions need to be encoded and transmitted.

### Tile-Based Detection

The screen is divided into a grid of tiles (default 64×64 pixels). Each frame, we compare tiles against the previous frame:

```
Previous Frame              Current Frame              Damage Map
┌───┬───┬───┬───┬───┐      ┌───┬───┬───┬───┬───┐      ┌───┬───┬───┬───┬───┐
│   │   │   │   │   │      │   │   │   │   │   │      │   │   │   │   │   │
├───┼───┼───┼───┼───┤      ├───┼───┼───┼───┼───┤      ├───┼───┼───┼───┼───┤
│   │ A │ B │   │   │      │   │ A'│ B │   │   │  ──► │   │ ▓ │   │   │   │
├───┼───┼───┼───┼───┤      ├───┼───┼───┼───┼───┤      ├───┼───┼───┼───┼───┤
│   │   │   │   │   │      │   │ C │   │   │   │      │   │ ▓ │   │   │   │
├───┼───┼───┼───┼───┤      ├───┼───┼───┼───┼───┤      ├───┼───┼───┼───┼───┤
│   │   │   │   │   │      │   │   │   │   │   │      │   │   │   │   │   │
└───┴───┴───┴───┴───┘      └───┴───┴───┴───┴───┘      └───┴───┴───┴───┴───┘

Only tiles marked ▓ are encoded and sent.
```

### SIMD Acceleration

Comparing millions of pixels per frame requires speed. lamco-rdp-server uses SIMD instructions to compare multiple pixels simultaneously:

| Platform | SIMD | Pixels/cycle |
|----------|------|--------------|
| x86_64 | AVX2 | 32 |
| ARM | NEON | 16 |
| Fallback | Scalar | 1 |

### Real-World Savings

| Scenario | Full Frame | With Damage Tracking | Savings |
|----------|------------|----------------------|---------|
| Static desktop | 8.3 MB/s | 0.08 MB/s | 99% |
| Typing in editor | 8.3 MB/s | 0.4 MB/s | 95% |
| Scrolling web page | 8.3 MB/s | 2.1 MB/s | 75% |
| Full-screen video | 8.3 MB/s | 8.3 MB/s | 0% |

Typical desktop use sees **90%+ bandwidth savings**.

### Configuration

```toml
[damage]
enabled = true
tile_size = 64              # Pixels per tile (32, 64, 128)
diff_threshold = 0.01       # Minimum change to count as "damaged"
merge_adjacent = true       # Combine nearby damaged tiles
```

---

## Latency Governor

### Three Modes for Different Needs

Not all remote desktop use is the same. Coding requires responsive input. Watching a presentation prioritizes visual quality. The latency governor optimizes the pipeline for your specific use case.

| Mode | Priority | Best For |
|------|----------|----------|
| **Interactive** | Input latency | Coding, terminal work, real-time collaboration |
| **Balanced** | Both | General desktop use |
| **Quality** | Visual quality | Presentations, design review, media playback |

### How It Works

The latency governor adjusts multiple parameters:

| Parameter | Interactive | Balanced | Quality |
|-----------|-------------|----------|---------|
| Encode buffer | Minimal | Standard | Larger |
| Frame batching | Disabled | Light | Aggressive |
| QP preference | Higher (faster) | Medium | Lower (better) |
| Key frame interval | Shorter | Medium | Longer |

### Interactive Mode Deep Dive

For coding and terminal work, every millisecond of input lag matters. Interactive mode:

1. **Minimizes encode latency:** Smallest possible buffers
2. **Prioritizes key frames:** Faster recovery from packet loss
3. **Reduces batching:** Send frames immediately
4. **Accepts quality tradeoff:** Slightly higher QP for speed

**Result:** Keystrokes appear faster, cursor movement feels direct.

### Quality Mode Deep Dive

For presentations or reviewing visual content, smoothness and clarity matter more than input speed. Quality mode:

1. **Larger encode buffers:** Better compression efficiency
2. **Aggressive batching:** More efficient network use
3. **Lower QP:** Higher visual quality
4. **Longer key frame intervals:** More bits for detail

**Result:** Sharper image, smoother motion, higher bandwidth.

### Configuration

```toml
[performance.latency]
mode = "balanced"    # interactive, balanced, quality

# Or fine-tune manually:
# encode_buffer_ms = 16
# batch_frames = false
# prefer_low_latency = true
```

---

## Predictive Cursor

### The Latency Problem

Network latency is unavoidable. Even on fast connections, round-trip time of 20-50ms means the cursor lags behind your hand movement. On high-latency connections (100ms+), this becomes disorienting.

### Physics-Based Prediction

lamco-rdp-server's predictive cursor uses physics modeling to extrapolate cursor position:

```
Actual cursor path (network delayed):
    ●───●───●───●───●  ← Client sees this (50ms behind)

Predicted path (shown to user):
    ●───●───●───●───●───○───○  ← Extrapolated position
                        ▲
                        Where cursor probably is NOW
```

### How It Works

1. **Track velocity:** Monitor cursor movement over recent samples
2. **Track acceleration:** Detect speed changes (starting/stopping)
3. **Extrapolate:** Project position forward by latency amount
4. **Snap back:** Smooth correction when actual position arrives

### Prediction Quality

| Cursor Movement | Prediction Accuracy | User Experience |
|-----------------|---------------------|-----------------|
| Steady motion | Excellent | Feels nearly local |
| Acceleration | Good | Slight overshoot, corrects smoothly |
| Sudden stop | Fair | Brief overshoot, snaps to final position |
| Erratic | Poor | Disabled automatically |

The predictor detects erratic movement and disables itself, falling back to standard behavior when prediction would make things worse.

### Cursor Modes

| Mode | Behavior | Use Case |
|------|----------|----------|
| `metadata` | Standard RDP cursor | Default, works everywhere |
| `painted` | Cursor rendered into frame | Compatibility mode |
| `hidden` | Server cursor hidden | Custom cursor handling |
| `predictive` | Physics-based prediction | High-latency connections |
| `auto` | Select based on latency | Recommended |

### Configuration

```toml
[cursor]
mode = "auto"           # auto, metadata, painted, hidden, predictive

[cursor.predictor]
enabled = true
max_prediction_ms = 100     # Don't predict beyond 100ms
velocity_smoothing = 0.8    # How much to smooth velocity estimates
acceleration_factor = 0.5   # Weight for acceleration in prediction
snap_threshold = 5.0        # Pixels of error before snap correction
```

### Auto Mode Logic

```
Measure network latency
         │
         ├─ <20ms  ──────────►  metadata (standard)
         │                      Latency too low to notice
         │
         ├─ 20-80ms ─────────►  predictive
         │                      Sweet spot for prediction
         │
         └─ >80ms  ──────────►  predictive (conservative)
                                Reduced prediction to avoid overshoot
```

---

## Service Advertisement Registry

### Dynamic Capability Detection

Different Wayland compositors and system configurations support different features. The Service Advertisement Registry probes available capabilities and maps them to RDP features.

```
System Capabilities                    RDP Features Advertised
─────────────────────                  ─────────────────────────
PipeWire available?        ──────────► Screen capture support
libei available?           ──────────► Input injection support
Portal clipboard?          ──────────► Clipboard sync support
Multiple outputs?          ──────────► Multi-monitor support
DMA-BUF support?           ──────────► Zero-copy capture
```

### Compositor Probing

lamco-rdp-server automatically detects your compositor and adjusts behavior:

| Compositor | Detection Method | Special Handling |
|------------|------------------|------------------|
| GNOME | `GNOME_DESKTOP_SESSION` | D-Bus clipboard fallback |
| KDE Plasma | `KDE_FULL_SESSION` | KWin-specific optimizations |
| Sway | `SWAYSOCK` | wlroots portal behavior |
| Hyprland | `HYPRLAND_INSTANCE` | Hyprland portal behavior |

### Why This Matters

Different systems have different capabilities. Rather than failing with cryptic errors, lamco-rdp-server:

1. Probes what's available at startup
2. Advertises only supported features to clients
3. Uses optimal code paths for each capability
4. Provides clear diagnostics when features are unavailable

---

## Performance Tuning Guide

### Low Bandwidth (<5 Mbps)

```toml
[egfx]
codec = "avc420"        # Save ~40% vs AVC444
h264_bitrate = 2000
qp_default = 28         # Accept lower quality

[performance.adaptive_fps]
max_fps = 30            # Cap frame rate

[damage]
enabled = true          # Essential for low bandwidth
```

### Low Latency (coding, terminals)

```toml
[performance.latency]
mode = "interactive"

[performance.adaptive_fps]
enabled = true
min_fps = 15            # Keep responsive even when static

[cursor]
mode = "auto"           # Enable prediction if needed
```

### High Quality (presentations, design)

```toml
[egfx]
codec = "avc444"
h264_bitrate = 10000
qp_default = 18

[performance.latency]
mode = "quality"

[performance.adaptive_fps]
max_fps = 60
```

### Maximum Performance (LAN, powerful hardware)

```toml
[hardware_encoding]
enabled = true
prefer_nvenc = true
quality_preset = "quality"

[egfx]
codec = "avc444"
h264_bitrate = 15000
qp_default = 15

[performance.adaptive_fps]
max_fps = 60

[performance.latency]
mode = "balanced"
```

---

## Benchmarks

Measured on Intel i7-12700K, NVIDIA RTX 3070, 1920×1080 desktop:

| Configuration | FPS | Latency | CPU | GPU | Bandwidth |
|---------------|-----|---------|-----|-----|-----------|
| NVENC + AVC444 + 60fps | 60 | 12ms | 3% | 8% | 8.2 Mbps |
| NVENC + AVC420 + 30fps | 30 | 14ms | 2% | 5% | 4.1 Mbps |
| OpenH264 + AVC444 + 30fps | 30 | 18ms | 15% | 0% | 4.8 Mbps |
| OpenH264 + AVC420 + 30fps | 30 | 16ms | 12% | 0% | 3.2 Mbps |

*Latency measured end-to-end from input to display update on LAN connection.*

---

## Further Reading

- [Video Encoding Technology →](/technology/video-encoding/)
- [Color Management →](/technology/color-management/)
- [Wayland Integration →](/technology/wayland/)
