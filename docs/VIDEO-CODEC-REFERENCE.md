# Video Codec Reference

**Product:** lamco-rdp-server
**Last Updated:** 2025-12-30

---

## Supported Codecs

### AVC420 (H.264 YUV 4:2:0)

The standard H.264 codec with 4:2:0 chroma subsampling.

| Aspect | Details |
|--------|---------|
| Use Case | General desktop, video playback |
| Chroma | 4:2:0 (half horizontal, half vertical) |
| Bandwidth | Lower (50% less chroma data) |
| Text Quality | Good |
| RDP Version | RDPGFX V8+ |

**Best for:** Video content, general desktop use, bandwidth-constrained networks.

### AVC444 (H.264 YUV 4:4:4)

Premium H.264 codec with full chroma resolution via dual-stream encoding.

| Aspect | Details |
|--------|---------|
| Use Case | Text-heavy work, UI design, programming |
| Chroma | 4:4:4 (full resolution) |
| Bandwidth | Higher (dual streams) |
| Text Quality | Excellent |
| RDP Version | RDPGFX V10+ |

**Best for:** Programming, document editing, CAD, UI design.

**Implementation:**
- Main stream: Luminance (Y) component
- Auxiliary stream: Chroma (UV) components
- Auxiliary omission: Bandwidth optimization when chroma unchanged

---

## Encoder Backends

### OpenH264 (Software)

Software encoder using Cisco's OpenH264 library.

| Feature | Status |
|---------|--------|
| AVC420 | Full support |
| AVC444 | Full support (dual encoder) |
| Color VUI | Full support (via fork) |
| Performance | Good (optimized for low latency) |

**Configuration:**
```toml
[video]
encoder = "openh264"  # or "auto"
```

### NVENC (NVIDIA GPU)

Hardware encoder using NVIDIA Video Codec SDK.

| Feature | Status |
|---------|--------|
| AVC420 | Full support |
| AVC444 | Full support |
| Color VUI | Full support (h264VUIParameters) |
| Performance | Excellent (dedicated hardware) |

**Requirements:**
- NVIDIA GPU (Kepler or newer)
- NVIDIA driver with NVENC support
- libnvidia-encode library

**Configuration:**
```toml
[hardware_encoding]
enabled = true
prefer_nvenc = true

[video]
encoder = "auto"  # Will select NVENC if available
```

### VA-API (Intel/AMD GPU)

Hardware encoder using Video Acceleration API.

| Feature | Status |
|---------|--------|
| AVC420 | Full support |
| AVC444 | Full support |
| Color VUI | Not available (API limitation) |
| Performance | Excellent |

**Requirements:**
- Intel (iHD or i965) or AMD (radeonsi) GPU
- libva and appropriate driver
- Access to /dev/dri/renderD128

**Configuration:**
```toml
[hardware_encoding]
enabled = true
vaapi_device = "/dev/dri/renderD128"

[video]
encoder = "vaapi"
```

---

## Color Management

### Color Spaces

| Standard | Matrix | Range | Use Case |
|----------|--------|-------|----------|
| BT.709 Full | BT.709 | 0-255 | Desktop capture (default) |
| BT.709 Limited | BT.709 | 16-235 | Broadcast content |
| BT.601 Limited | BT.601 | 16-235 | SD content |
| sRGB | BT.709 | 0-255 | Web/graphics |

### Auto-Selection

The server automatically selects color space based on resolution:
- HD (≥1280x720): BT.709 Full
- SD (<1280x720): BT.601 Limited

### VUI Signaling

Video Usability Information (VUI) metadata tells the decoder how to interpret colors.

| Encoder | VUI Support | Notes |
|---------|-------------|-------|
| OpenH264 | Full | Via VuiConfig presets |
| NVENC | Full | Via h264VUIParameters |
| VA-API | None | API limitation (correct colors via decoder assumption) |

---

## H.264 Levels

The server auto-selects H.264 level based on resolution:

| Level | Max Resolution | Max FPS | Typical Use |
|-------|---------------|---------|-------------|
| 3.0 | 720x576 | 25 | SD content |
| 3.1 | 1280x720 | 30 | 720p HD |
| 4.0 | 2048x1024 | 30 | 1080p HD |
| 4.1 | 2048x1024 | 30 | 1080p HD (high bitrate) |
| 5.0 | 3672x1536 | 30 | 1440p QHD |
| 5.1 | 4096x2160 | 30 | 4K UHD |
| 5.2 | 4096x2160 | 60 | 4K UHD @ 60fps |

---

## Quality Presets

### Bitrate

| Preset | Main Stream | Aux Stream (AVC444) |
|--------|-------------|---------------------|
| Low | 2 Mbps | 1 Mbps |
| Balanced | 5 Mbps | 2.5 Mbps |
| High | 10 Mbps | 5 Mbps |

### QP (Quantization Parameter)

Lower QP = higher quality, higher bitrate.

| Mode | QP Range | Notes |
|------|----------|-------|
| Default | 10-40 (default 23) | Balanced quality |
| Quality | 10-30 | Higher quality, more bandwidth |
| Performance | 20-40 | Lower bandwidth, faster |

---

## Performance Features

### Damage Tracking

Tile-based frame differencing detects changed regions.

- **Tile size:** 64x64 pixels (configurable)
- **SIMD acceleration:** AVX2 (x86), NEON (ARM)
- **Bandwidth savings:** 90%+ for static content

### Adaptive FPS

Dynamic frame rate based on screen activity:

| Activity | FPS | Detection |
|----------|-----|-----------|
| Static | 5 | <1% damage |
| Low (typing) | 15 | 1-10% damage |
| Medium (scrolling) | 20-30 | 10-30% damage |
| High (video) | 30-60 | >30% damage |

### AVC444 Auxiliary Omission

Bandwidth optimization for AVC444:

- Skip auxiliary stream when chroma unchanged
- Configurable refresh interval (default: 30 frames)
- Change detection threshold (default: 5%)

---

## Configuration Reference

```toml
[egfx]
enabled = true
codec = "auto"              # auto, avc420, avc444
h264_level = "auto"         # auto, 3.0-5.2
h264_bitrate = 5000         # kbps
qp_min = 10
qp_max = 40
qp_default = 23

# Color management
color_matrix = "auto"       # auto, bt709, bt601, srgb
color_range = "auto"        # auto, limited, full

# AVC444 specific
avc444_enabled = true
avc444_aux_bitrate_ratio = 0.5
avc444_enable_aux_omission = true
avc444_max_aux_interval = 30
avc444_aux_change_threshold = 0.05

# ZGFX compression
zgfx_compression = "never"  # never, auto, always

[hardware_encoding]
enabled = false
vaapi_device = "/dev/dri/renderD128"
prefer_nvenc = true
quality_preset = "balanced" # speed, balanced, quality
fallback_to_software = true
```

---

## Troubleshooting

### Colors Look Wrong

1. Check color_matrix setting (try "bt709")
2. Verify VUI support for your encoder (NVENC/OpenH264 have full support)
3. Check if client supports the codec version

### Poor Text Quality

1. Enable AVC444 if client supports RDPGFX V10+
2. Lower QP values (higher quality)
3. Increase bitrate

### High CPU Usage

1. Enable hardware encoding (NVENC/VA-API)
2. Increase QP values (lower quality)
3. Enable damage tracking
4. Lower max_fps

### Choppy Video

1. Check network bandwidth
2. Enable adaptive FPS
3. Lower bitrate if network constrained
4. Check hardware encoder availability
