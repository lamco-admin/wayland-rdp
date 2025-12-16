//! RDP Server module
//!
//! Provides RDP protocol server implementation with compositor integration.

pub mod channels;
pub mod server;
pub mod encoder;

pub use server::{RdpServer, RdpServerConfig, RdpInputEvent, ServerStats};
pub use encoder::{FrameEncoder, EncodedFrame, EncoderFormat};
