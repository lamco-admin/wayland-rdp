//! Wayland feature definitions
//!
//! Enumerates detectable Wayland compositor features that can be
//! translated into RDP service advertisements.

use serde::{Deserialize, Serialize};

/// Method used for damage tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageMethod {
    /// Portal provides damage hints
    Portal,

    /// Native compositor damage API (wlroots screencopy)
    NativeScreencopy,

    /// Frame differencing in software
    FrameDiff,

    /// Hybrid: use damage hints when available, fall back to diff
    Hybrid,
}

impl Default for DamageMethod {
    fn default() -> Self {
        Self::FrameDiff
    }
}

/// DRM format for DMA-BUF
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrmFormat {
    /// ARGB8888 (common, compatible)
    Argb8888,

    /// XRGB8888 (no alpha)
    Xrgb8888,

    /// ABGR8888 (reverse byte order)
    Abgr8888,

    /// NV12 (YUV 4:2:0, for hardware encoding)
    Nv12,

    /// Other format (fourcc code)
    Other(u32),
}

impl DrmFormat {
    /// Check if format supports alpha channel
    pub fn has_alpha(&self) -> bool {
        matches!(self, Self::Argb8888 | Self::Abgr8888)
    }

    /// Check if format is YUV (hardware encoder friendly)
    pub fn is_yuv(&self) -> bool {
        matches!(self, Self::Nv12)
    }
}

/// HDR transfer function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HdrTransfer {
    /// Standard dynamic range (sRGB)
    Sdr,

    /// Perceptual Quantizer (HDR10)
    Pq,

    /// Hybrid Log-Gamma (broadcast HDR)
    Hlg,

    /// Extended sRGB (scRGB)
    ScRgb,
}

impl Default for HdrTransfer {
    fn default() -> Self {
        Self::Sdr
    }
}

/// Detectable Wayland feature with associated metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WaylandFeature {
    /// Damage tracking capability
    DamageTracking {
        /// Method used for damage detection
        method: DamageMethod,
        /// Whether compositor provides per-frame damage hints
        compositor_hints: bool,
    },

    /// Zero-copy DMA-BUF buffer access
    DmaBufZeroCopy {
        /// Supported DRM formats
        formats: Vec<DrmFormat>,
        /// Whether modifiers are supported
        supports_modifiers: bool,
    },

    /// Explicit sync protocol support
    ExplicitSync {
        /// Protocol version
        version: u32,
    },

    /// Fractional scaling support
    FractionalScaling {
        /// Maximum supported scale factor
        max_scale: f32,
    },

    /// Metadata cursor (position sent separately from video)
    MetadataCursor {
        /// Whether hotspot is included
        has_hotspot: bool,
        /// Whether cursor image updates are available
        has_shape_updates: bool,
    },

    /// Multi-monitor support
    MultiMonitor {
        /// Maximum number of monitors
        max_monitors: u32,
        /// Whether virtual source type is available
        virtual_source: bool,
    },

    /// Per-window capture
    WindowCapture {
        /// Whether toplevel export is available
        has_toplevel_export: bool,
    },

    /// HDR color space support
    HdrColorSpace {
        /// Transfer function
        transfer: HdrTransfer,
        /// Color gamut
        gamut: String,
    },

    /// Clipboard via portal
    Clipboard {
        /// Portal version supporting clipboard
        portal_version: u32,
    },

    /// Remote input injection
    RemoteInput {
        /// Uses libei for input
        uses_libei: bool,
        /// Keyboard supported
        keyboard: bool,
        /// Pointer supported
        pointer: bool,
        /// Touch supported
        touch: bool,
    },

    /// PipeWire video stream
    PipeWireStream {
        /// Node ID if already connected
        node_id: Option<u32>,
        /// Preferred buffer type
        buffer_type: String,
    },
}

impl WaylandFeature {
    /// Get a short identifier for logging
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::DamageTracking { .. } => "damage",
            Self::DmaBufZeroCopy { .. } => "dmabuf",
            Self::ExplicitSync { .. } => "explicit-sync",
            Self::FractionalScaling { .. } => "fractional-scale",
            Self::MetadataCursor { .. } => "metadata-cursor",
            Self::MultiMonitor { .. } => "multi-monitor",
            Self::WindowCapture { .. } => "window-capture",
            Self::HdrColorSpace { .. } => "hdr",
            Self::Clipboard { .. } => "clipboard",
            Self::RemoteInput { .. } => "remote-input",
            Self::PipeWireStream { .. } => "pipewire",
        }
    }
}

impl std::fmt::Display for WaylandFeature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DamageTracking { method, compositor_hints } => {
                write!(f, "DamageTracking({:?}, hints={})", method, compositor_hints)
            }
            Self::DmaBufZeroCopy { formats, .. } => {
                write!(f, "DmaBuf({} formats)", formats.len())
            }
            Self::ExplicitSync { version } => {
                write!(f, "ExplicitSync(v{})", version)
            }
            Self::FractionalScaling { max_scale } => {
                write!(f, "FractionalScale(max={}x)", max_scale)
            }
            Self::MetadataCursor { .. } => write!(f, "MetadataCursor"),
            Self::MultiMonitor { max_monitors, .. } => {
                write!(f, "MultiMonitor(max={})", max_monitors)
            }
            Self::WindowCapture { .. } => write!(f, "WindowCapture"),
            Self::HdrColorSpace { transfer, .. } => {
                write!(f, "HDR({:?})", transfer)
            }
            Self::Clipboard { portal_version } => {
                write!(f, "Clipboard(portal v{})", portal_version)
            }
            Self::RemoteInput { keyboard, pointer, touch, .. } => {
                write!(f, "RemoteInput(kbd={}, ptr={}, touch={})", keyboard, pointer, touch)
            }
            Self::PipeWireStream { buffer_type, .. } => {
                write!(f, "PipeWire({})", buffer_type)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drm_format_alpha() {
        assert!(DrmFormat::Argb8888.has_alpha());
        assert!(!DrmFormat::Xrgb8888.has_alpha());
        assert!(!DrmFormat::Nv12.has_alpha());
    }

    #[test]
    fn test_drm_format_yuv() {
        assert!(DrmFormat::Nv12.is_yuv());
        assert!(!DrmFormat::Argb8888.is_yuv());
    }

    #[test]
    fn test_feature_display() {
        let feature = WaylandFeature::DamageTracking {
            method: DamageMethod::Portal,
            compositor_hints: true,
        };
        assert!(feature.to_string().contains("DamageTracking"));
    }
}
