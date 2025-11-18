//! PipeWire Thread Communication
//!
//! Message passing system for communicating between the async runtime
//! and the dedicated PipeWire MainLoop thread.

use tokio::sync::oneshot;

use crate::pipewire::error::Result;
use crate::pipewire::stream::StreamConfig;

/// Commands sent to the PipeWire thread
pub enum PipeWireCommand {
    /// Create a new stream
    CreateStream {
        stream_id: u32,
        config: StreamConfig,
        node_id: u32,
        response: oneshot::Sender<Result<()>>,
    },

    /// Destroy a stream
    DestroyStream {
        stream_id: u32,
        response: oneshot::Sender<Result<()>>,
    },

    /// Get stream state
    GetStreamState {
        stream_id: u32,
        response: oneshot::Sender<Option<crate::pipewire::stream::PwStreamState>>,
    },

    /// Shutdown the PipeWire thread
    Shutdown,
}

/// Responses from the PipeWire thread
pub enum PipeWireResponse {
    /// Stream created successfully
    StreamCreated(u32),

    /// Stream destroyed successfully
    StreamDestroyed(u32),

    /// Error occurred
    Error(String),
}
