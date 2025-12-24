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
//!                                    (implements GraphicsPipelineHandler)
//!                                              │
//!                                              │ send_avc420_frame()
//!                                              ▼
//!                                    ironrdp_egfx::GraphicsPipelineServer
//!                                              │
//!                                              │ DVC messages
//!                                              ▼
//!                                         RDP Client
//! ```
//!
//! # Protocol Reference
//!
//! - [MS-RDPEGFX](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/)

mod encoder;
mod handler;
mod video_handler;

// Re-export encoder types
pub use encoder::{
    align_to_16, annex_b_to_avc, Avc420Encoder, EncoderConfig, EncoderError, EncoderResult,
    H264Frame,
};

// Re-export our handler implementation
pub use handler::{SharedGraphicsHandler, WrdGraphicsHandler};

// Re-export video handler types
pub use video_handler::{EgfxVideoConfig, EgfxVideoHandler, EncodedFrame, EncodingStats};

// Re-export ironrdp-egfx types needed by consumers
pub use ironrdp_egfx::pdu::Avc420Region;
pub use ironrdp_egfx::server::{
    GraphicsPipelineHandler, GraphicsPipelineServer, QoeMetrics, Surface,
};
