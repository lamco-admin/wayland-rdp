//! # lamco-rdp-server
//!
//! Wayland RDP server for Linux - Portal mode for desktop sharing.
//!
//! This is the main server orchestration crate that integrates:
//! - [`lamco_portal`] - XDG Desktop Portal integration
//! - [`lamco_pipewire`] - PipeWire screen capture
//! - [`lamco_video`] - Video frame processing
//! - [`lamco_rdp_input`] - RDP input event translation
//! - [`lamco_rdp_clipboard`] - RDP clipboard integration
//!
//! # Architecture
//!
//! ```text
//! lamco-rdp-server
//!   ├─> Portal Session (screen capture + input injection permissions)
//!   ├─> PipeWire Manager (video frame capture)
//!   ├─> Display Handler (video streaming to RDP clients)
//!   ├─> Input Handler (keyboard/mouse from RDP clients)
//!   ├─> Clipboard Manager (bidirectional clipboard sync)
//!   └─> IronRDP Server (RDP protocol, TLS, RemoteFX encoding)
//! ```
//!
//! # Data Flow
//!
//! **Video Path:** Portal → PipeWire → Display Handler → IronRDP → Client
//!
//! **Input Path:** Client → IronRDP → Input Handler → Portal → Compositor
//!
//! **Clipboard Path:** Client ↔ IronRDP ↔ Clipboard Manager ↔ Portal ↔ Compositor

#![warn(missing_docs)]
#![warn(clippy::all)]

// =============================================================================
// Server-specific modules (kept in this crate)
// =============================================================================

/// Server configuration
pub mod config;

/// Multi-monitor support
pub mod multimon;

/// Protocol utilities
pub mod protocol;

/// RDP channel management
pub mod rdp;

/// Security and TLS
pub mod security;

/// Main server implementation
pub mod server;

/// Utility functions
pub mod utils;

/// Clipboard orchestration (bridges portal ↔ RDP)
///
/// This module provides the glue code that connects:
/// - `lamco_portal::ClipboardManager` (portal clipboard)
/// - `lamco_rdp_clipboard::RdpCliprdrFactory` (RDP clipboard)
///
/// It implements the `ClipboardSink` trait from `lamco_clipboard_core`
/// to bridge the two systems.
pub mod clipboard;

/// EGFX (RDP Graphics Pipeline Extension) for H.264 video streaming
///
/// This module implements the server-side EGFX channel for hardware-accelerated
/// H.264 video encoding over RDP. Requires the `h264` feature.
///
/// EGFX uses Dynamic Virtual Channels (DVC) and provides:
/// - AVC420 (H.264 YUV420) codec support
/// - Surface management for multi-monitor
/// - Flow control via frame acknowledgments
pub mod egfx;

/// Damage region detection for bandwidth optimization
///
/// This module implements tile-based frame differencing to detect changed
/// screen regions, enabling significant bandwidth reduction (90%+ for static content).
///
/// Key features:
/// - SIMD-optimized tile comparison (AVX2/NEON)
/// - Configurable tile size and threshold
/// - Automatic region merging
/// - Statistics tracking for monitoring
pub mod damage;

/// Compositor capability probing
///
/// This module automatically detects the running Wayland compositor
/// (GNOME, KDE, Sway, Hyprland, etc.) and probes its capabilities to
/// enable optimal configuration without manual per-DE settings.
///
/// Key features:
/// - Compositor identification from environment
/// - Portal capability detection
/// - Known quirk profiles for each DE
/// - Automatic adaptation
pub mod compositor;

/// Performance optimization features (Premium)
///
/// This module provides advanced performance optimization features:
///
/// - **Adaptive FPS**: Dynamically adjusts frame rate (5-30 FPS) based on
///   screen activity. Static screens drop to 5 FPS, video maintains 30 FPS.
///
/// - **Latency Governor**: Three professional modes for different use cases:
///   - Interactive (<50ms): Gaming, CAD
///   - Balanced (<100ms): General desktop
///   - Quality (<300ms): Photo/video editing
///
/// These features reduce CPU usage by 30-50% for typical desktop work while
/// maintaining responsive user experience.
pub mod performance;

/// Cursor handling strategies (Premium)
///
/// This module provides advanced cursor handling for different scenarios:
///
/// - **Metadata Mode**: Client-side rendering (lowest latency, default)
/// - **Painted Mode**: Cursor painted into video (maximum compatibility)
/// - **Hidden Mode**: For touch/pen input
/// - **Predictive Mode**: Physics-based prediction for latency compensation
///
/// The predictive cursor is the key innovation - it uses velocity/acceleration
/// tracking to predict where the cursor will be N milliseconds in the future,
/// making cursor movement feel instant even with 100ms+ network latency.
pub mod cursor;

// =============================================================================
// Re-exports from published lamco crates (for convenience)
// =============================================================================

/// Re-export lamco-portal for portal integration
pub use lamco_portal;

/// Re-export lamco-pipewire for PipeWire integration
pub use lamco_pipewire;

/// Re-export lamco-video for video processing
pub use lamco_video;

/// Re-export lamco-rdp-input for input handling
pub use lamco_rdp_input;

/// Re-export lamco-clipboard-core for clipboard primitives
pub use lamco_clipboard_core;

/// Re-export lamco-rdp-clipboard for RDP clipboard
pub use lamco_rdp_clipboard;

// =============================================================================
// Convenience aliases
// =============================================================================

/// Portal types (convenience re-export)
pub mod portal {
    pub use lamco_portal::{
        ClipboardManager as PortalClipboardManager, PortalConfig, PortalConfigBuilder,
        PortalError, PortalManager, PortalSessionHandle, RemoteDesktopManager, Result as PortalResult,
        ScreenCastManager, SourceType, StreamInfo,
    };
}

/// PipeWire types (convenience re-export)
pub mod pipewire {
    pub use lamco_pipewire::{
        // Core manager types
        PipeWireConfig, PipeWireConfigBuilder, PipeWireError, PipeWireManager,
        PipeWireThreadCommand, PipeWireThreadManager, Result as PipeWireResult,
        // Connection types
        PipeWireConnection,
        // Stream types
        PixelFormat, SourceType, StreamConfig, StreamHandle, StreamInfo, VideoFrame,
        // Multi-stream coordinator types
        MonitorInfo, MultiStreamConfig, MultiStreamCoordinator,
    };
}

/// Video processing types (convenience re-export)
pub mod video {
    pub use lamco_video::{
        BitmapConverter, BitmapData, BitmapUpdate, ConversionError, DispatcherConfig,
        FrameDispatcher, FrameProcessor, ProcessorConfig, RdpPixelFormat, Rectangle,
    };
}

/// Input handling types (convenience re-export)
pub mod input {
    pub use lamco_rdp_input::{
        CoordinateTransformer, InputError, InputTranslator, KeyModifiers, KeyboardEvent,
        KeyboardEventType, KeyboardHandler, LinuxInputEvent, MonitorInfo, MouseButton,
        MouseEvent, MouseHandler, RdpInputEvent, Result as InputResult,
    };
}
