# Technology: Video Encoding

**URL:** `https://lamco.ai/technology/video-encoding/`
**Status:** Draft for review

---

## Overview

Remote desktop is fundamentally a video streaming problem. Your desktop generates frames, those frames need to reach the client, and the client needs to display them with minimal delay. The quality of this video pipeline determines whether remote work feels responsive or frustrating.

lamco-rdp-server implements a sophisticated H.264 encoding pipeline with multiple encoder backends, intelligent codec selection, and bandwidth optimization techniques that adapt to your content and network conditions.

---

## H.264/AVC Fundamentals

H.264 (also called AVC or MPEG-4 Part 10) is the dominant video codec for remote desktop protocols. It offers excellent compression efficiency, hardware acceleration support on virtually all GPUs, and universal client compatibility.

### Why H.264 for Remote Desktop?

| Factor | Benefit |
|--------|---------|
| **Compression** | 10-50x smaller than raw frames |
| **Hardware Support** | Dedicated encode/decode on all modern GPUs |
| **Latency** | Designed for real-time streaming |
| **Compatibility** | Every RDP client supports it |
| **Quality** | Excellent at typical desktop bitrates |

### H.264 Profiles and Levels

lamco-rdp-server uses the **High Profile** for maximum quality and automatically selects the appropriate **Level** based on your resolution:

| Level | Max Resolution | Typical Use |
|-------|---------------|-------------|
| 3.1 | 1280×720 | 720p HD |
| 4.0 | 1920×1080 | 1080p Full HD |
| 4.1 | 1920×1080 | 1080p (higher bitrate) |
| 5.0 | 2560×1440 | 1440p QHD |
| 5.1 | 3840×2160 | 4K UHD |
| 5.2 | 3840×2160 @ 60fps | 4K UHD high frame rate |

---

## AVC420 vs AVC444

This is where lamco-rdp-server distinguishes itself from most remote desktop solutions.

### The Chroma Subsampling Problem

Standard H.264 video uses **4:2:0 chroma subsampling**—the color information is stored at half the resolution of the brightness information. This is imperceptible for natural video content (movies, camera footage) but creates visible artifacts on computer-generated content like text and UI elements.

```
Original pixel grid:        4:2:0 subsampling:
┌───┬───┬───┬───┐          ┌───┬───┬───┬───┐
│ R │ G │ B │ Y │          │ R │   │ B │   │  ← Color samples
├───┼───┼───┼───┤          ├───┼───┼───┼───┤     at half density
│ C │ M │ W │ K │          │   │   │   │   │
├───┼───┼───┼───┤          ├───┼───┼───┼───┤
│ R │ G │ B │ Y │          │ R │   │ B │   │
├───┼───┼───┼───┤          ├───┼───┼───┼───┤
│ C │ M │ W │ K │          │   │   │   │   │
└───┴───┴───┴───┘          └───┴───┴───┴───┘
```

The result: colored text looks fuzzy, thin lines have color fringing, and sharp UI edges become muddy.

### AVC444: Full Chroma Resolution

**AVC444** (4:4:4 chroma) preserves full color resolution. Every pixel retains its exact color, resulting in perfect reproduction of text and UI elements.

lamco-rdp-server implements AVC444 using the RDP Graphics Pipeline Extension (RDPGFX) dual-stream approach:

```
Desktop Frame (BGRA)
        │
        ▼
┌───────────────────┐
│  Color Convert    │
│   BGRA → YUV444   │
└───────────────────┘
        │
        ├─────────────────────────┐
        ▼                         ▼
┌───────────────┐         ┌───────────────┐
│  Main Stream  │         │  Aux Stream   │
│  (Luminance)  │         │   (Chroma)    │
│    Y only     │         │   U + V       │
└───────────────┘         └───────────────┘
        │                         │
        ▼                         ▼
   H.264 Encode              H.264 Encode
        │                         │
        └──────────┬──────────────┘
                   ▼
            RDP Client
         (recombines streams)
```

### When to Use Each Codec

| Content Type | Recommended | Why |
|--------------|-------------|-----|
| **Programming/Code** | AVC444 | Text clarity critical |
| **Document Editing** | AVC444 | Sharp text rendering |
| **UI/Design Work** | AVC444 | Color accuracy matters |
| **Video Playback** | AVC420 | Native video is already 4:2:0 |
| **General Desktop** | AVC444 | Best overall quality |
| **Bandwidth Limited** | AVC420 | ~40% less data |

### AVC444 Auxiliary Omission

When screen content is static or only the brightness changes (not colors), lamco-rdp-server can skip sending the auxiliary chroma stream. This optimization reduces bandwidth by up to 40% during typical desktop use while maintaining full quality when colors do change.

**Configuration:**
```toml
[egfx]
avc444_enabled = true
avc444_enable_aux_omission = true
avc444_max_aux_interval = 30      # Force chroma refresh every 30 frames
avc444_aux_change_threshold = 0.05 # 5% change triggers chroma send
```

---

## Encoder Backends

### OpenH264 (Software)

Cisco's open-source H.264 encoder, optimized for real-time encoding.

| Aspect | Details |
|--------|---------|
| **Availability** | Always available (bundled) |
| **CPU Usage** | Moderate to high |
| **Quality** | Excellent |
| **Latency** | Low (optimized for real-time) |
| **AVC444** | Full support |
| **Color VUI** | Full support |

**Best for:** Systems without capable GPUs, maximum compatibility.

**Configuration:**
```toml
[video]
encoder = "openh264"
```

### NVIDIA NVENC

Hardware encoder using NVIDIA's dedicated encoding silicon.

| Aspect | Details |
|--------|---------|
| **Availability** | NVIDIA GPUs (Kepler+) |
| **CPU Usage** | Near zero |
| **Quality** | Excellent |
| **Latency** | Very low |
| **AVC444** | Full support |
| **Color VUI** | Full support (h264VUIParameters) |

**Requirements:**
- NVIDIA GPU with NVENC support
- Proprietary NVIDIA driver
- libnvidia-encode library

**Best for:** NVIDIA GPU systems, lowest CPU usage, best quality.

**Configuration:**
```toml
[hardware_encoding]
enabled = true
prefer_nvenc = true
```

### VA-API (Intel/AMD)

Video Acceleration API for Intel and AMD GPUs.

| Aspect | Details |
|--------|---------|
| **Availability** | Intel iGPU, AMD GPU |
| **CPU Usage** | Near zero |
| **Quality** | Good to excellent |
| **Latency** | Very low |
| **AVC444** | Full support |
| **Color VUI** | Limited (API constraint) |

**Intel Drivers:**
- **intel-media-va-driver (iHD):** Modern Intel (Broadwell+), recommended
- **i965-va-driver:** Legacy Intel, older systems

**AMD Drivers:**
- **mesa-va-drivers (radeonsi):** All modern AMD GPUs

**Best for:** Intel laptops, AMD systems, open-source driver preference.

**Configuration:**
```toml
[hardware_encoding]
enabled = true
vaapi_device = "/dev/dri/renderD128"
```

### Encoder Selection Logic

When `encoder = "auto"` (default):

```
1. Check for NVENC availability
   └─ If available and prefer_nvenc = true → Use NVENC

2. Check for VA-API availability
   └─ If available → Use VA-API

3. Fall back to OpenH264
   └─ Always available
```

---

## Quality Parameters

### Bitrate

Controls the data rate of the encoded stream.

| Preset | Main Stream | Aux Stream (AVC444) |
|--------|-------------|---------------------|
| Low | 2 Mbps | 1 Mbps |
| Balanced | 5 Mbps | 2.5 Mbps |
| High | 10 Mbps | 5 Mbps |

**Configuration:**
```toml
[egfx]
h264_bitrate = 5000  # kbps
avc444_aux_bitrate_ratio = 0.5  # Aux = 50% of main
```

### Quantization Parameter (QP)

Controls quality vs file size tradeoff. Lower = higher quality, larger size.

| Value | Quality | Use Case |
|-------|---------|----------|
| 10-15 | Excellent | LAN, high bandwidth |
| 18-23 | Very good | Typical use (default: 23) |
| 25-30 | Good | Bandwidth constrained |
| 35-40 | Acceptable | Very limited bandwidth |

**Configuration:**
```toml
[egfx]
qp_min = 10
qp_max = 40
qp_default = 23
```

---

## Bandwidth Optimization

### Damage Tracking

Not every frame contains changes across the entire screen. lamco-rdp-server uses tile-based damage tracking to identify which regions actually changed.

```
Frame divided into 64×64 pixel tiles:
┌───┬───┬───┬───┬───┬───┐
│   │   │   │   │   │   │
├───┼───┼───┼───┼───┼───┤
│   │ ▓ │ ▓ │   │   │   │  ▓ = Changed tiles
├───┼───┼───┼───┼───┼───┤
│   │ ▓ │ ▓ │ ▓ │   │   │      (cursor moved,
├───┼───┼───┼───┼───┼───┤       text typed)
│   │   │   │   │   │   │
├───┼───┼───┼───┼───┼───┤
│   │   │   │   │   │   │
└───┴───┴───┴───┴───┴───┘

Only changed tiles are encoded and transmitted.
```

**Result:** 90%+ bandwidth savings for typical desktop use (static content with localized changes).

**Implementation:** SIMD-accelerated comparison using AVX2 (x86) or NEON (ARM).

### Adaptive Frame Rate

Frame rate adjusts based on content activity:

| Screen Activity | FPS | Detection |
|-----------------|-----|-----------|
| Static | 5 | <1% of tiles changed |
| Low (typing) | 15 | 1-10% changed |
| Medium (scrolling) | 20-30 | 10-30% changed |
| High (video) | 30-60 | >30% changed |

**Benefits:**
- Battery savings on static content
- Full fluidity when needed
- Automatic, no user intervention

**Configuration:**
```toml
[performance.adaptive_fps]
enabled = true
min_fps = 5
max_fps = 60
static_threshold = 0.01
low_activity_threshold = 0.10
high_activity_threshold = 0.30
```

---

## Configuration Reference

Complete video encoding configuration:

```toml
[egfx]
enabled = true
codec = "auto"              # auto, avc420, avc444
h264_level = "auto"         # auto, 3.0, 3.1, 4.0, 4.1, 5.0, 5.1, 5.2
h264_bitrate = 5000         # kbps

# Quality
qp_min = 10
qp_max = 40
qp_default = 23

# AVC444
avc444_enabled = true
avc444_aux_bitrate_ratio = 0.5
avc444_enable_aux_omission = true
avc444_max_aux_interval = 30
avc444_aux_change_threshold = 0.05

[hardware_encoding]
enabled = true
prefer_nvenc = true
vaapi_device = "/dev/dri/renderD128"
quality_preset = "balanced"  # speed, balanced, quality
fallback_to_software = true

[damage]
enabled = true
tile_size = 64
diff_threshold = 0.01
merge_adjacent = true
```

---

## Troubleshooting

### Poor Text Quality

1. Enable AVC444: `codec = "avc444"` or `codec = "auto"` (prefers AVC444)
2. Lower QP values: `qp_default = 18`
3. Increase bitrate: `h264_bitrate = 8000`
4. Verify client supports RDPGFX v10+ (required for AVC444)

### High CPU Usage

1. Enable hardware encoding: `[hardware_encoding] enabled = true`
2. Verify hardware encoder is being used (check logs)
3. For NVENC: ensure libnvidia-encode is installed
4. For VA-API: verify with `vainfo` command

### Choppy Video

1. Check network bandwidth (need ~5 Mbps for comfortable use)
2. Enable adaptive FPS: `[performance.adaptive_fps] enabled = true`
3. If bandwidth limited, reduce bitrate and accept lower quality
4. Try AVC420 instead of AVC444 for ~40% bandwidth reduction

### Colors Look Wrong

See [Color Management Technology →](/technology/color-management/)
