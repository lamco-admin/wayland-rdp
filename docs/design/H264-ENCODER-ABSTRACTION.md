# H.264 Encoder Abstraction Layer Design

## Overview

This document describes a trait-based abstraction for H.264 encoding that allows plugging in multiple encoder backends while maintaining MS-RDPEGFX compliance.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    H264EncoderBackend Trait                      │
├─────────────────────────────────────────────────────────────────┤
│  + encode_frame(yuv: &YuvFrame) -> Result<EncodedFrame>         │
│  + force_keyframe()                                              │
│  + configure(config: EncoderConfig)                              │
│  + capabilities() -> EncoderCapabilities                         │
└─────────────────────────────────────────────────────────────────┘
                              ▲
          ┌───────────────────┼───────────────────┐
          │                   │                   │
┌─────────┴─────────┐ ┌──────┴──────┐ ┌─────────┴─────────┐
│  OpenH264Backend  │ │VaapiBackend │ │   NvencBackend    │
│  (Software)       │ │(Intel/AMD)  │ │   (NVIDIA)        │
│  Feature: h264    │ │Feature:vaapi│ │  Feature: nvenc   │
└───────────────────┘ └─────────────┘ └───────────────────┘
```

## Trait Definition

```rust
//! src/egfx/encoder/backend.rs

use async_trait::async_trait;

/// Capabilities reported by an encoder backend
#[derive(Debug, Clone)]
pub struct EncoderCapabilities {
    /// Human-readable backend name
    pub name: &'static str,

    /// Whether hardware acceleration is available
    pub hardware_accelerated: bool,

    /// Supported H.264 profiles
    pub profiles: Vec<H264Profile>,

    /// Maximum supported resolution
    pub max_resolution: (u32, u32),

    /// Whether B-frames are supported
    pub b_frames_supported: bool,

    /// Estimated latency category
    pub latency: LatencyCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum H264Profile {
    Baseline,
    Main,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatencyCategory {
    /// < 5ms encode time (hardware)
    VeryLow,
    /// 5-15ms encode time
    Low,
    /// 15-50ms encode time (software)
    Medium,
    /// > 50ms encode time
    High,
}

/// Input frame in YUV420 format
pub struct YuvFrame<'a> {
    pub y_plane: &'a [u8],
    pub u_plane: &'a [u8],
    pub v_plane: &'a [u8],
    pub width: u32,
    pub height: u32,
    pub y_stride: u32,
    pub uv_stride: u32,
    pub timestamp_ms: u64,
}

/// Encoded H.264 output (AVC format, length-prefixed NALs)
#[derive(Debug)]
pub struct EncodedFrame {
    /// AVC-formatted NAL units (length-prefixed for MS-RDPEGFX)
    pub data: Vec<u8>,

    /// Whether this is a keyframe (IDR)
    pub is_keyframe: bool,

    /// Frame timestamp
    pub timestamp_ms: u64,

    /// Encoding time in microseconds
    pub encode_time_us: u64,
}

/// Encoder configuration
#[derive(Debug, Clone)]
pub struct EncoderConfig {
    pub width: u32,
    pub height: u32,
    pub bitrate_kbps: u32,
    pub max_fps: u32,
    pub profile: H264Profile,
    pub keyframe_interval: u32,
}

/// Backend-specific error
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Initialization failed: {0}")]
    InitFailed(String),

    #[error("Encoding failed: {0}")]
    EncodeFailed(String),

    #[error("Backend not available: {0}")]
    NotAvailable(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Core encoder backend trait
#[async_trait]
pub trait H264EncoderBackend: Send + Sync {
    /// Get backend capabilities
    fn capabilities(&self) -> &EncoderCapabilities;

    /// Encode a YUV420 frame to H.264
    ///
    /// Returns AVC-formatted (length-prefixed) NAL units
    async fn encode_frame(&mut self, frame: YuvFrame<'_>) -> Result<Option<EncodedFrame>, BackendError>;

    /// Force next frame to be a keyframe (IDR)
    fn force_keyframe(&mut self);

    /// Reconfigure encoder (may require restart)
    async fn reconfigure(&mut self, config: EncoderConfig) -> Result<(), BackendError>;
}
```

## Backend Implementations

### 1. OpenH264 Backend (Current)

```rust
//! src/egfx/encoder/openh264.rs
//! Feature: "h264" (default: false)

#[cfg(feature = "h264")]
pub struct OpenH264Backend {
    encoder: openh264::Encoder,
    config: EncoderConfig,
    capabilities: EncoderCapabilities,
    force_idr: bool,
}

#[cfg(feature = "h264")]
impl OpenH264Backend {
    pub fn new(config: EncoderConfig) -> Result<Self, BackendError> {
        // Current implementation moved here
    }
}

#[cfg(feature = "h264")]
#[async_trait]
impl H264EncoderBackend for OpenH264Backend {
    fn capabilities(&self) -> &EncoderCapabilities {
        &EncoderCapabilities {
            name: "OpenH264 (Cisco)",
            hardware_accelerated: false,
            profiles: vec![H264Profile::Baseline, H264Profile::Main],
            max_resolution: (4096, 2160),
            b_frames_supported: false,
            latency: LatencyCategory::Medium,
        }
    }

    async fn encode_frame(&mut self, frame: YuvFrame<'_>) -> Result<Option<EncodedFrame>, BackendError> {
        // Implementation with annex_b_to_avc conversion
    }

    // ...
}
```

### 2. VA-API Backend (Linux Hardware Acceleration)

```rust
//! src/egfx/encoder/vaapi.rs
//! Feature: "vaapi"
//! Dependencies: va (libva bindings)

#[cfg(feature = "vaapi")]
use va::{VADisplay, VAContext, VAConfigAttrib};

#[cfg(feature = "vaapi")]
pub struct VaapiBackend {
    display: VADisplay,
    context: VAContext,
    config: EncoderConfig,
    capabilities: EncoderCapabilities,
}

#[cfg(feature = "vaapi")]
impl VaapiBackend {
    pub fn new(config: EncoderConfig) -> Result<Self, BackendError> {
        // Open DRM device and create VA display
        let drm_fd = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/dri/renderD128")?;

        let display = va::VADisplay::new(&drm_fd)?;

        // Check for H.264 encode support
        let profiles = display.query_config_profiles()?;
        if !profiles.contains(&va::VAProfile::H264Main) {
            return Err(BackendError::NotAvailable("VA-API H.264 encoding not supported".into()));
        }

        // Create encoder context
        // ...
    }
}
```

### 3. NVENC Backend (NVIDIA)

```rust
//! src/egfx/encoder/nvenc.rs
//! Feature: "nvenc"
//! Dependencies: nvenc-rs or nvidia-video-codec-sdk bindings

#[cfg(feature = "nvenc")]
pub struct NvencBackend {
    encoder: nvenc::Encoder,
    config: EncoderConfig,
    capabilities: EncoderCapabilities,
}
```

## Color Space Conversion with wgpu

For GPU-accelerated BGRA→YUV420 conversion, wgpu compute shaders can be used:

```rust
//! src/egfx/encoder/gpu_convert.rs
//! Feature: "gpu-convert"

use wgpu::{Device, Queue, Buffer, ComputePipeline};

pub struct GpuColorConverter {
    device: Device,
    queue: Queue,
    pipeline: ComputePipeline,
    input_buffer: Buffer,
    output_y: Buffer,
    output_u: Buffer,
    output_v: Buffer,
}

impl GpuColorConverter {
    pub fn new(device: &Device, max_width: u32, max_height: u32) -> Self {
        // WGSL compute shader for BGRA → YUV420 conversion
        let shader = device.create_shader_module(wgpu::include_wgsl!("bgra_to_yuv420.wgsl"));
        // ...
    }

    /// Convert BGRA frame to YUV420 on GPU
    pub async fn convert(&self, bgra: &[u8], width: u32, height: u32) -> YuvFrame {
        // Upload BGRA to GPU
        // Run compute shader
        // Download YUV planes
    }
}
```

**bgra_to_yuv420.wgsl:**
```wgsl
@group(0) @binding(0) var<storage, read> bgra: array<u32>;
@group(0) @binding(1) var<storage, read_write> y_plane: array<u8>;
@group(0) @binding(2) var<storage, read_write> u_plane: array<u8>;
@group(0) @binding(3) var<storage, read_write> v_plane: array<u8>;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;

    // BT.601 conversion coefficients (MS-RDPEGFX requirement)
    let pixel = bgra[y * width + x];
    let b = f32((pixel >> 0) & 0xFF);
    let g = f32((pixel >> 8) & 0xFF);
    let r = f32((pixel >> 16) & 0xFF);

    // Y = 0.299*R + 0.587*G + 0.114*B
    let y_val = u8(clamp(16.0 + 65.481*r + 128.553*g + 24.966*b, 0.0, 255.0));
    y_plane[y * y_stride + x] = y_val;

    // Chroma subsampling (4:2:0) - process 2x2 blocks
    if (x % 2 == 0) && (y % 2 == 0) {
        // U = -0.169*R - 0.331*G + 0.500*B + 128
        // V =  0.500*R - 0.419*G - 0.081*B + 128
        let u_val = u8(clamp(128.0 - 37.797*r - 74.203*g + 112.0*b, 0.0, 255.0));
        let v_val = u8(clamp(128.0 + 112.0*r - 93.786*g - 18.214*b, 0.0, 255.0));

        let chroma_idx = (y / 2) * uv_stride + (x / 2);
        u_plane[chroma_idx] = u_val;
        v_plane[chroma_idx] = v_val;
    }
}
```

## Feature Flags

```toml
# Cargo.toml
[features]
default = ["pam-auth"]

# H.264 encoding backends (choose one or more)
h264-openh264 = ["openh264"]      # Software, BSD license
h264-vaapi = ["va"]               # Linux hardware (Intel/AMD)
h264-nvenc = ["nvenc-rs"]         # NVIDIA hardware
h264-qsv = ["intel-media-sdk"]    # Intel QuickSync

# GPU color conversion (optional optimization)
gpu-convert = ["wgpu"]

# Convenience: enable best available backend
h264 = ["h264-openh264"]          # Default to software
h264-hw = ["h264-vaapi"]          # Prefer hardware on Linux
```

## Backend Selection Logic

```rust
//! src/egfx/encoder/mod.rs

/// Create the best available encoder backend
pub async fn create_encoder(config: EncoderConfig) -> Result<Box<dyn H264EncoderBackend>, BackendError> {
    // Try hardware encoders first (lower latency)
    #[cfg(feature = "h264-nvenc")]
    if let Ok(encoder) = NvencBackend::new(config.clone()) {
        tracing::info!("Using NVENC hardware encoder");
        return Ok(Box::new(encoder));
    }

    #[cfg(feature = "h264-vaapi")]
    if let Ok(encoder) = VaapiBackend::new(config.clone()) {
        tracing::info!("Using VA-API hardware encoder");
        return Ok(Box::new(encoder));
    }

    #[cfg(feature = "h264-qsv")]
    if let Ok(encoder) = QsvBackend::new(config.clone()) {
        tracing::info!("Using Intel QuickSync encoder");
        return Ok(Box::new(encoder));
    }

    // Fall back to software encoder
    #[cfg(feature = "h264-openh264")]
    {
        tracing::info!("Using OpenH264 software encoder");
        return Ok(Box::new(OpenH264Backend::new(config)?));
    }

    Err(BackendError::NotAvailable("No H.264 encoder backend available".into()))
}
```

## Performance Expectations

| Backend | Latency (1080p) | Quality | GPU Usage | CPU Usage |
|---------|-----------------|---------|-----------|-----------|
| OpenH264 | ~30-50ms | Good | 0% | 100% (1 core) |
| VA-API (Intel) | ~3-5ms | Good | ~20% | ~10% |
| VA-API (AMD) | ~3-5ms | Good | ~15% | ~10% |
| NVENC | ~2-4ms | Excellent | ~5% | ~5% |
| QSV | ~3-5ms | Good | ~10% | ~10% |

## Implementation Roadmap

1. **Phase 1: Abstraction** (This PR)
   - Define `H264EncoderBackend` trait
   - Refactor OpenH264 to implement trait
   - Add feature flags

2. **Phase 2: VA-API**
   - Add `va` crate dependency
   - Implement VaapiBackend
   - Test on Intel and AMD GPUs

3. **Phase 3: GPU Convert**
   - Add wgpu BGRA→YUV shader
   - Integrate with VA-API pipeline
   - Benchmark DMA-BUF zero-copy path

4. **Phase 4: NVENC** (optional)
   - Add NVIDIA SDK bindings
   - Implement NvencBackend
   - Test CUDA memory sharing

## References

- [MS-RDPEGFX](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/)
- [VA-API Documentation](https://intel.github.io/libva/)
- [wgpu Documentation](https://wgpu.rs/)
- [OpenH264](https://github.com/cisco/openh264)
