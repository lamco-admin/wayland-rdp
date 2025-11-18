//! Video encoding and processing
//!
//! This module manages video encoding using hardware acceleration
//! (VAAPI) or software fallback (OpenH264), as well as frame conversion
//! and processing for RDP transmission.

pub mod converter;
pub mod dispatcher;
pub mod encoder;
pub mod processor;

// Re-export key types
pub use converter::{BitmapConverter, BitmapData, BitmapUpdate, ConversionError, RdpPixelFormat};
pub use dispatcher::{DispatchError, DispatcherConfig, FrameDispatcher};
pub use processor::{FrameProcessor, ProcessingError, ProcessorConfig};
