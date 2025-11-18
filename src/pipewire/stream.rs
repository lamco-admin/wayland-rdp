//! PipeWire Stream Management
//!
//! Handles individual PipeWire streams for screen capture.

use std::os::fd::RawFd;
use std::ptr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;
use pipewire::stream::{Stream, StreamFlags, StreamState};
use pipewire::spa::utils::{Direction, Fraction, Rectangle};
use libspa::param::video::VideoFormat;

use crate::pipewire::error::{PipeWireError, Result};
use crate::pipewire::buffer::{BufferManager, BufferType, SharedBufferManager};
use crate::pipewire::frame::{VideoFrame, FrameCallback, FrameStats};
use crate::pipewire::format::PixelFormat;
use crate::pipewire::ffi::{self, SpaDataType};

/// Stream configuration
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Stream name
    pub name: String,

    /// Target width
    pub width: u32,

    /// Target height
    pub height: u32,

    /// Target framerate
    pub framerate: u32,

    /// Use DMA-BUF if available
    pub use_dmabuf: bool,

    /// Number of buffers
    pub buffer_count: u32,

    /// Preferred format
    pub preferred_format: Option<PixelFormat>,
}

impl StreamConfig {
    /// Create default configuration
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            width: 1920,
            height: 1080,
            framerate: 30,
            use_dmabuf: true,
            buffer_count: 3,
            preferred_format: Some(PixelFormat::BGRA),
        }
    }

    /// Set resolution
    pub fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set framerate
    pub fn with_framerate(mut self, fps: u32) -> Self {
        self.framerate = fps;
        self
    }

    /// Set DMA-BUF preference
    pub fn with_dmabuf(mut self, use_dmabuf: bool) -> Self {
        self.use_dmabuf = use_dmabuf;
        self
    }

    /// Set buffer count
    pub fn with_buffer_count(mut self, count: u32) -> Self {
        self.buffer_count = count;
        self
    }
}

/// PipeWire stream state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PwStreamState {
    /// Stream is initializing
    Initializing,
    /// Stream is ready to start
    Ready,
    /// Stream is actively streaming
    Streaming,
    /// Stream is paused
    Paused,
    /// Stream encountered an error
    Error,
    /// Stream is closing
    Closing,
}

impl From<StreamState> for PwStreamState {
    fn from(state: StreamState) -> Self {
        match state {
            StreamState::Error(_) => Self::Error,
            StreamState::Unconnected => Self::Initializing,
            StreamState::Connecting => Self::Initializing,
            StreamState::Paused => Self::Paused,
            StreamState::Streaming => Self::Streaming,
        }
    }
}

/// Format negotiation result
#[derive(Debug, Clone)]
pub struct NegotiatedFormat {
    /// Video format
    pub format: VideoFormat,

    /// Width
    pub width: u32,

    /// Height
    pub height: u32,

    /// Row stride
    pub stride: u32,

    /// Framerate
    pub framerate: Fraction,
}

/// PipeWire stream handler
pub struct PipeWireStream {
    /// Stream ID
    id: u32,

    /// Stream configuration
    config: StreamConfig,

    /// PipeWire stream (using pipewire crate)
    stream: Option<Stream>,

    /// Buffer manager
    buffer_manager: SharedBufferManager,

    /// Current state
    state: Arc<Mutex<PwStreamState>>,

    /// Negotiated format
    negotiated_format: Arc<Mutex<Option<NegotiatedFormat>>>,

    /// Frame callback
    frame_callback: Arc<Mutex<Option<FrameCallback>>>,

    /// Frame counter
    frame_counter: Arc<Mutex<u64>>,

    /// Statistics
    stats: Arc<Mutex<FrameStats>>,

    /// Start time
    start_time: Arc<Mutex<Option<SystemTime>>>,

    /// Frame sender channel
    frame_tx: Arc<Mutex<Option<mpsc::Sender<VideoFrame>>>>,
}

impl PipeWireStream {
    /// Create new PipeWire stream
    pub fn new(id: u32, config: StreamConfig) -> Self {
        let buffer_manager = SharedBufferManager::new(config.buffer_count as usize);

        Self {
            id,
            config,
            stream: None,
            buffer_manager,
            state: Arc::new(Mutex::new(PwStreamState::Initializing)),
            negotiated_format: Arc::new(Mutex::new(None)),
            frame_callback: Arc::new(Mutex::new(None)),
            frame_counter: Arc::new(Mutex::new(0)),
            stats: Arc::new(Mutex::new(FrameStats::new())),
            start_time: Arc::new(Mutex::new(None)),
            frame_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Get stream ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get stream state
    pub fn state(&self) -> PwStreamState {
        *self.state.lock().unwrap()
    }

    /// Set frame callback
    pub fn set_frame_callback(&mut self, callback: FrameCallback) {
        *self.frame_callback.lock().unwrap() = Some(callback);
    }

    /// Set frame channel
    pub fn set_frame_channel(&mut self, tx: mpsc::Sender<VideoFrame>) {
        *self.frame_tx.lock().unwrap() = Some(tx);
    }

    /// Get negotiated format
    pub fn negotiated_format(&self) -> Option<NegotiatedFormat> {
        self.negotiated_format.lock().unwrap().clone()
    }

    /// Get statistics
    pub fn stats(&self) -> FrameStats {
        self.stats.lock().unwrap().clone()
    }

    /// Connect to PipeWire node
    pub async fn connect(&mut self, core: &pipewire::core::Core, node_id: u32) -> Result<()> {
        // Build format parameters
        let formats = if let Some(pref) = self.config.preferred_format {
            vec![pref.to_spa()]
        } else {
            vec![
                VideoFormat::BGRx,
                VideoFormat::BGRA,
                VideoFormat::RGBx,
                VideoFormat::RGBA,
            ]
        };

        let framerate = Fraction {
            num: self.config.framerate,
            denom: 1,
        };

        // This is a simplified version - full implementation would use the pipewire crate's
        // stream builder with proper parameter construction
        // For now, we return an error indicating this needs PipeWire runtime
        Err(PipeWireError::StreamCreationFailed(
            "Stream connection requires PipeWire runtime - use integration tests".to_string()
        ))
    }

    /// Start streaming
    pub async fn start(&mut self) -> Result<()> {
        *self.state.lock().unwrap() = PwStreamState::Streaming;
        *self.start_time.lock().unwrap() = Some(SystemTime::now());
        Ok(())
    }

    /// Pause streaming
    pub async fn pause(&mut self) -> Result<()> {
        *self.state.lock().unwrap() = PwStreamState::Paused;
        Ok(())
    }

    /// Resume streaming
    pub async fn resume(&mut self) -> Result<()> {
        *self.state.lock().unwrap() = PwStreamState::Streaming;
        Ok(())
    }

    /// Stop streaming
    pub async fn stop(&mut self) -> Result<()> {
        *self.state.lock().unwrap() = PwStreamState::Closing;
        self.stream = None;
        Ok(())
    }

    /// Restart stream after error
    pub async fn restart(&mut self) -> Result<()> {
        self.stop().await?;
        *self.state.lock().unwrap() = PwStreamState::Initializing;
        Ok(())
    }

    /// Process a frame from PipeWire
    async fn process_frame(&self, buffer_id: u32, pts: u64) -> Result<()> {
        // Get buffer
        let buffer_opt = self.buffer_manager.with_buffer(buffer_id, |buf| {
            // Extract data
            let data = unsafe {
                buf.as_slice().map(|s| s.to_vec())
            };

            (buf.size, buf.buffer_type, data)
        }).await;

        if let Some((size, buffer_type, Some(data))) = buffer_opt {
            // Get negotiated format
            let format_info = self.negotiated_format.lock().unwrap().clone();

            if let Some(format) = format_info {
                // Create video frame
                let frame_id = {
                    let mut counter = self.frame_counter.lock().unwrap();
                    let id = *counter;
                    *counter += 1;
                    id
                };

                let pixel_format = PixelFormat::from_spa(format.format)
                    .unwrap_or(PixelFormat::BGRA);

                let mut frame = VideoFrame::with_data(
                    frame_id,
                    format.width,
                    format.height,
                    format.stride,
                    pixel_format,
                    self.id,
                    data,
                );

                frame.set_timing(pts, pts, 0);

                if buffer_type.is_dmabuf() {
                    frame.flags.set_dmabuf();
                }

                // Update stats
                self.stats.lock().unwrap().update(&frame);

                // Send to callback if set
                if let Some(ref callback) = *self.frame_callback.lock().unwrap() {
                    callback(frame.clone());
                }

                // Send to channel if set
                if let Some(ref tx) = *self.frame_tx.lock().unwrap() {
                    let _ = tx.try_send(frame);
                }
            }
        }

        Ok(())
    }

    /// Get uptime
    pub fn uptime(&self) -> Option<Duration> {
        self.start_time.lock().unwrap().as_ref()
            .and_then(|start| start.elapsed().ok())
    }
}

/// Stream metrics
#[derive(Debug, Clone, Default)]
pub struct StreamMetrics {
    /// Frames processed
    pub frames_processed: u64,

    /// Bytes processed
    pub bytes_processed: u64,

    /// Errors encountered
    pub error_count: u64,

    /// Buffer underruns
    pub underruns: u64,

    /// Average frame latency (milliseconds)
    pub avg_latency_ms: f64,

    /// Current FPS
    pub current_fps: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_config() {
        let config = StreamConfig::new("test-stream")
            .with_resolution(1920, 1080)
            .with_framerate(60)
            .with_dmabuf(true)
            .with_buffer_count(4);

        assert_eq!(config.name, "test-stream");
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert_eq!(config.framerate, 60);
        assert_eq!(config.use_dmabuf, true);
        assert_eq!(config.buffer_count, 4);
    }

    #[test]
    fn test_stream_creation() {
        let config = StreamConfig::new("test");
        let stream = PipeWireStream::new(0, config);

        assert_eq!(stream.id(), 0);
        assert_eq!(stream.state(), PwStreamState::Initializing);
    }

    #[tokio::test]
    async fn test_stream_state_transitions() {
        let config = StreamConfig::new("test");
        let mut stream = PipeWireStream::new(0, config);

        assert_eq!(stream.state(), PwStreamState::Initializing);

        stream.start().await.unwrap();
        assert_eq!(stream.state(), PwStreamState::Streaming);

        stream.pause().await.unwrap();
        assert_eq!(stream.state(), PwStreamState::Paused);

        stream.resume().await.unwrap();
        assert_eq!(stream.state(), PwStreamState::Streaming);

        stream.stop().await.unwrap();
        assert_eq!(stream.state(), PwStreamState::Closing);
    }

    #[test]
    fn test_frame_callback() {
        let config = StreamConfig::new("test");
        let mut stream = PipeWireStream::new(0, config);

        let received = Arc::new(Mutex::new(false));
        let received_clone = Arc::clone(&received);

        stream.set_frame_callback(Box::new(move |_frame| {
            *received_clone.lock().unwrap() = true;
        }));

        // Verify callback is set
        assert!(stream.frame_callback.lock().unwrap().is_some());
    }
}
