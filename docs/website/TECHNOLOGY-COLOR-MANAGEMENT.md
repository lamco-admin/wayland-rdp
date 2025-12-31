# Technology: Color Management

**URL:** `https://lamco.ai/technology/color-management/`
**Status:** Draft for review

---

## Why Color Matters in Remote Desktop

When you view a document locally, colors appear as the application intended. When that same document travels through a remote desktop connection, color accuracy depends on a chain of conversions and metadata that most remote desktop solutions get wrong—or ignore entirely.

The result: colors shift, whites look yellow or blue, and what you see remotely doesn't match what's actually on screen. For design work, photo editing, or any color-critical application, this makes remote desktop unusable.

lamco-rdp-server implements comprehensive color management to ensure what you see on your client matches what's on your Linux desktop.

---

## The Color Pipeline

Every frame travels through multiple color space conversions:

```
Desktop                Server              Network           Client
───────────────────────────────────────────────────────────────────────

Compositor            lamco-rdp-server                     RDP Client
    │                       │                                  │
    ▼                       ▼                                  ▼
┌────────┐            ┌────────────┐                     ┌────────────┐
│  sRGB  │───────────►│ BGRA→YUV   │─────────────────────►│  YUV→RGB  │
│ (BGRA) │            │ Conversion │    H.264 Stream     │ Conversion │
└────────┘            └────────────┘    + VUI Metadata   └────────────┘
                            │                                  │
                            ▼                                  ▼
                      Color Matrix                       Color Matrix
                      (BT.709/601)                      (from VUI)
                            │                                  │
                            ▼                                  ▼
                      Range Mapping                      Range Mapping
                      (Full/Limited)                     (from VUI)
```

If any step uses different assumptions than the others, colors shift.

---

## Color Space Standards

### BT.709 (Rec. 709)

The HD video standard, used for content ≥720p.

| Property | Value |
|----------|-------|
| **Use Case** | HD/4K video, modern displays |
| **Primaries** | sRGB-equivalent |
| **White Point** | D65 (6504K) |
| **Transfer** | BT.709 gamma (~2.4) |
| **Matrix** | BT.709 coefficients |

**When lamco-rdp-server uses it:** Default for HD resolutions (≥1280×720)

### BT.601 (Rec. 601)

The SD video standard, from the analog TV era.

| Property | Value |
|----------|-------|
| **Use Case** | SD video, legacy content |
| **Primaries** | NTSC/PAL (differs from sRGB) |
| **White Point** | D65 |
| **Transfer** | BT.601 gamma |
| **Matrix** | BT.601 coefficients |

**When lamco-rdp-server uses it:** SD resolutions (<1280×720) or explicit configuration

### sRGB

The web and desktop standard, closely related to BT.709.

| Property | Value |
|----------|-------|
| **Use Case** | Web graphics, desktop applications |
| **Primaries** | Same as BT.709 |
| **White Point** | D65 |
| **Transfer** | sRGB gamma (~2.2 with linear segment) |

**Relationship:** sRGB primaries match BT.709. The main difference is the transfer function (gamma curve). For desktop capture, BT.709 with full range closely approximates sRGB.

---

## Full Range vs Limited Range

This is where most remote desktop solutions fail.

### The Problem

Video standards define two value ranges:

| Range | Values | Origin |
|-------|--------|--------|
| **Full** | 0-255 | Computer graphics, desktop |
| **Limited** | 16-235 | Broadcast TV, video |

Your desktop uses **full range**. Video codecs traditionally assume **limited range**.

If the encoder treats full-range desktop content as limited-range video:
- Black (0) gets clipped
- White (255) gets clipped
- Contrast is crushed
- Colors look washed out

### The Solution: VUI Signaling

H.264 includes **Video Usability Information (VUI)** metadata that tells the decoder exactly how to interpret the encoded data.

lamco-rdp-server sets VUI parameters correctly:

```
video_full_range_flag = 1      # Full range (0-255)
colour_primaries = 1           # BT.709
transfer_characteristics = 1   # BT.709
matrix_coefficients = 1        # BT.709
```

The client decoder reads this metadata and applies the correct conversion.

---

## Encoder Color Support

### OpenH264

Full VUI support via our enhanced integration:

| Parameter | Support |
|-----------|---------|
| video_full_range_flag | ✓ |
| colour_primaries | ✓ |
| transfer_characteristics | ✓ |
| matrix_coefficients | ✓ |

OpenH264 allows complete control over color metadata, ensuring accurate reproduction.

### NVENC

Full VUI support via NVIDIA's h264VUIParameters:

| Parameter | Support |
|-----------|---------|
| video_full_range_flag | ✓ |
| colour_primaries | ✓ |
| transfer_characteristics | ✓ |
| matrix_coefficients | ✓ |

NVENC provides native color management controls matching OpenH264 capabilities.

### VA-API

Limited VUI support due to API constraints:

| Parameter | Support |
|-----------|---------|
| video_full_range_flag | ✗ (driver dependent) |
| colour_primaries | ✗ |
| transfer_characteristics | ✗ |
| matrix_coefficients | ✗ |

**Workaround:** VA-API typically produces correct colors because most clients assume BT.709 for HD content, which matches our encoding. However, explicit VUI signaling isn't available.

**Recommendation:** For color-critical work, prefer NVENC or OpenH264 over VA-API.

---

## SIMD Color Conversion

Converting BGRA desktop frames to YUV for H.264 encoding is computationally intensive. lamco-rdp-server uses SIMD (Single Instruction, Multiple Data) acceleration to process multiple pixels simultaneously.

### Implementation

| Architecture | SIMD | Speedup |
|--------------|------|---------|
| x86_64 | AVX2 | ~8x |
| ARM | NEON | ~4x |
| Fallback | Scalar | 1x (baseline) |

### The Math

Each pixel conversion requires:
```
Y  = 0.2126 R + 0.7152 G + 0.0722 B  (BT.709 luminance)
Cb = (B - Y) / 1.8556
Cr = (R - Y) / 1.5748
```

AVX2 processes 8 pixels per instruction, NEON processes 4. This optimization is essential for real-time encoding at high resolutions.

---

## Auto-Selection Logic

When `color_matrix = "auto"` (default):

```
Resolution ≥ 1280×720?
    │
    ├─ Yes → BT.709, Full Range
    │
    └─ No  → BT.601, Limited Range (legacy compatibility)
```

Most desktop use triggers BT.709 with full range—the correct choice for modern systems.

---

## Configuration Reference

```toml
[egfx]
# Color space selection
color_matrix = "auto"     # auto, bt709, bt601, srgb
color_range = "auto"      # auto, limited, full

# For explicit control:
# color_matrix = "bt709"
# color_range = "full"
```

### Configuration Options

| color_matrix | Description |
|--------------|-------------|
| `auto` | BT.709 for HD, BT.601 for SD |
| `bt709` | Force BT.709 (recommended for desktop) |
| `bt601` | Force BT.601 (legacy only) |
| `srgb` | sRGB (same primaries as BT.709) |

| color_range | Description |
|-------------|-------------|
| `auto` | Full for desktop capture |
| `full` | 0-255 (correct for desktop) |
| `limited` | 16-235 (broadcast video) |

---

## Troubleshooting

### Colors Look Washed Out

**Symptom:** Blacks aren't black, whites aren't white, everything looks low-contrast.

**Cause:** Limited range encoding being displayed as full range (or vice versa).

**Fix:**
```toml
[egfx]
color_range = "full"
```

If using VA-API, try switching to OpenH264 or NVENC for proper VUI signaling.

### Colors Have a Tint

**Symptom:** Everything looks slightly yellow, blue, or green.

**Cause:** Wrong color matrix (BT.601 vs BT.709).

**Fix:**
```toml
[egfx]
color_matrix = "bt709"
```

### Different Colors on Different Clients

**Symptom:** Same server, different clients show different colors.

**Cause:** Some RDP clients ignore VUI metadata or have bugs.

**Fix:**
- Try different RDP clients (Windows mstsc.exe, FreeRDP)
- FreeRDP with recent versions handles VUI correctly
- Ensure client isn't applying its own color correction

### VA-API Colors Slightly Off

**Symptom:** Colors are close but not perfect with VA-API encoding.

**Cause:** VA-API lacks VUI parameter control.

**Fix:**
- Accept minor variance (usually imperceptible)
- Switch to NVENC or OpenH264 for critical work:
```toml
[hardware_encoding]
enabled = false  # Falls back to OpenH264

# Or for NVIDIA:
[hardware_encoding]
enabled = true
prefer_nvenc = true
```

---

## Color Accuracy Comparison

How lamco-rdp-server compares to alternatives:

| Solution | Color Matrix | Range Signaling | VUI Support |
|----------|--------------|-----------------|-------------|
| **lamco-rdp-server** | Configurable | Full + Limited | Full |
| xrdp | Fixed | Limited only | Partial |
| gnome-remote-desktop | Auto | Unknown | Unknown |
| VNC | N/A | RGB direct | N/A |

lamco-rdp-server provides the most comprehensive color management of any Linux RDP server.

---

## Further Reading

- [Video Encoding Technology →](/technology/video-encoding/)
- [Performance Features →](/technology/performance/)
- [ITU-T BT.709 Specification](https://www.itu.int/rec/R-REC-BT.709/)
- [ITU-T BT.601 Specification](https://www.itu.int/rec/R-REC-BT.601/)
