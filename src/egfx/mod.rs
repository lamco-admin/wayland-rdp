//! EGFX (RDP Graphics Pipeline Extension) Integration
//!
//! This module integrates H.264 video streaming via the EGFX channel, using:
//! - `ironrdp-egfx` for protocol handling and frame transmission
//! - OpenH264 for actual video encoding
//! - PipeWire for screen capture
//!
//! # Architecture
//!
//! ```text
//! PipeWire → VideoFrame → EgfxVideoHandler → Avc420Encoder → H.264 NAL data
//!                              │                                    │
//!                              └────────────────────────────────────┘
//!                                              │
//!                                              ▼
//!                                    WrdGraphicsHandler
//!                                    (capability negotiation + callbacks)
//!                                              │
//!                                              │ send_avc420_frame()
//!                                              ▼
//!                                    EGFX Protocol Layer (internal)
//!                                              │
//!                                              │ DVC messages
//!                                              ▼
//!                                         RDP Client
//! ```
//!
//! # API Boundaries
//!
//! This module exports only our own types. IronRDP types are used internally
//! by the server infrastructure but are not part of the public API.
//!
//! # Protocol Reference
//!
//! - [MS-RDPEGFX](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/)

mod encoder;
// TODO: Fix compilation errors in encoder_ext before enabling
// mod encoder_ext;
mod h264_level;
mod handler;
mod video_handler;

// Re-export our encoder types (clean API - no IronRDP types)
pub use encoder::{
    align_to_16, annex_b_to_avc, Avc420Encoder, EncoderConfig, EncoderError, EncoderResult,
    EncoderStats, H264Frame,
};

// TODO: Re-enable after fixing compilation
// Re-export level-aware encoder (with C API access)
// pub use encoder_ext::LevelAwareEncoder;

// Re-export H.264 level management
pub use h264_level::{ConstraintViolation, H264Level, LevelConstraints};

// Re-export our handler implementation
// Note: WrdGraphicsHandler implements ironrdp_egfx::GraphicsPipelineHandler internally
// but that trait is not part of our public API
pub use handler::{SharedGraphicsHandler, WrdGraphicsHandler};

// Re-export video handler types (clean API - no IronRDP types)
pub use video_handler::{EgfxVideoConfig, EgfxVideoHandler, EncodedFrame, EncodingStats};

// Note: IronRDP EGFX types (Avc420Region, GraphicsPipelineServer, etc.) are NOT
// re-exported here. They are used internally by src/server/gfx_factory.rs which
// bridges our implementation with IronRDP's infrastructure.
