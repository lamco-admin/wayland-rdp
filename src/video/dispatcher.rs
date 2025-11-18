//! Frame Dispatcher
//!
//! Routes video frames from multiple PipeWire streams to frame processors.
//! Handles:
//! - Multi-stream coordination
//! - Priority-based frame processing
//! - Backpressure management
//! - Load balancing across monitors
//! - Frame drop decisions based on system load

use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, trace, warn};

use crate::pipewire::frame::VideoFrame;

/// Default channel buffer size
const DEFAULT_CHANNEL_SIZE: usize = 30;

/// Maximum frame age before forced drop (milliseconds)
const MAX_FRAME_AGE_MS: u64 = 150;

/// High water mark for backpressure (percentage of queue)
const HIGH_WATER_MARK: f32 = 0.8;

/// Low water mark for backpressure release (percentage of queue)
const LOW_WATER_MARK: f32 = 0.5;

/// Dispatcher configuration
#[derive(Debug, Clone)]
pub struct DispatcherConfig {
    /// Channel buffer size per stream
    pub channel_size: usize,

    /// Enable priority-based dispatch
    pub priority_dispatch: bool,

    /// Maximum frame age before drop (ms)
    pub max_frame_age_ms: u64,

    /// Enable backpressure handling
    pub enable_backpressure: bool,

    /// High water mark (0.0-1.0)
    pub high_water_mark: f32,

    /// Low water mark (0.0-1.0)
    pub low_water_mark: f32,

    /// Enable load balancing
    pub load_balancing: bool,
}

impl Default for DispatcherConfig {
    fn default() -> Self {
        Self {
            channel_size: DEFAULT_CHANNEL_SIZE,
            priority_dispatch: true,
            max_frame_age_ms: MAX_FRAME_AGE_MS,
            enable_backpressure: true,
            high_water_mark: HIGH_WATER_MARK,
            low_water_mark: LOW_WATER_MARK,
            load_balancing: true,
        }
    }
}

/// Dispatcher statistics
#[derive(Debug, Clone, Default)]
pub struct DispatcherStats {
    /// Total frames received
    pub frames_received: u64,

    /// Total frames dispatched
    pub frames_dispatched: u64,

    /// Frames dropped due to age
    pub frames_dropped_age: u64,

    /// Frames dropped due to backpressure
    pub frames_dropped_backpressure: u64,

    /// Current active streams
    pub active_streams: usize,

    /// Total dispatch time (nanoseconds)
    pub total_dispatch_time_ns: u64,

    /// Backpressure active
    pub backpressure_active: bool,
}

impl DispatcherStats {
    /// Get average dispatch time in microseconds
    pub fn avg_dispatch_time_us(&self) -> f64 {
        if self.frames_dispatched == 0 {
            0.0
        } else {
            (self.total_dispatch_time_ns as f64 / self.frames_dispatched as f64) / 1_000.0
        }
    }

    /// Get drop rate
    pub fn drop_rate(&self) -> f64 {
        if self.frames_received == 0 {
            0.0
        } else {
            let total_drops = self.frames_dropped_age + self.frames_dropped_backpressure;
            total_drops as f64 / self.frames_received as f64
        }
    }

    /// Get dispatch rate
    pub fn dispatch_rate(&self) -> f64 {
        if self.frames_received == 0 {
            0.0
        } else {
            self.frames_dispatched as f64 / self.frames_received as f64
        }
    }
}

/// Stream priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StreamPriority {
    Low = 0,
    Normal = 1,
    High = 2,
}

/// Frame with dispatch metadata
struct DispatchFrame {
    frame: VideoFrame,
    priority: StreamPriority,
    enqueue_time: Instant,
}

impl DispatchFrame {
    fn new(frame: VideoFrame, priority: StreamPriority) -> Self {
        Self {
            frame,
            priority,
            enqueue_time: Instant::now(),
        }
    }

    fn age(&self) -> Duration {
        self.enqueue_time.elapsed()
    }

    fn is_too_old(&self, max_age_ms: u64) -> bool {
        self.age().as_millis() as u64 > max_age_ms
    }
}

/// Per-stream state
struct StreamState {
    priority: StreamPriority,
    frame_count: u64,
    last_frame_time: Option<Instant>,
    backpressure_active: bool,
}

impl StreamState {
    fn new(priority: StreamPriority) -> Self {
        Self {
            priority,
            frame_count: 0,
            last_frame_time: None,
            backpressure_active: false,
        }
    }

    fn update_frame_received(&mut self) {
        self.frame_count += 1;
        self.last_frame_time = Some(Instant::now());
    }
}

/// Frame dispatcher
pub struct FrameDispatcher {
    config: DispatcherConfig,
    streams: Arc<RwLock<HashMap<u32, StreamState>>>,
    priority_queue: Arc<RwLock<VecDeque<DispatchFrame>>>,
    stats: Arc<RwLock<DispatcherStats>>,
    running: Arc<RwLock<bool>>,
}

impl FrameDispatcher {
    /// Create a new frame dispatcher
    ///
    /// # Arguments
    /// * `config` - Dispatcher configuration
    ///
    /// # Returns
    /// A new `FrameDispatcher` instance
    pub fn new(config: DispatcherConfig) -> Self {
        Self {
            config,
            streams: Arc::new(RwLock::new(HashMap::new())),
            priority_queue: Arc::new(RwLock::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(DispatcherStats::default())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Register a stream
    ///
    /// # Arguments
    /// * `stream_id` - Unique stream identifier (monitor index)
    /// * `priority` - Stream priority
    pub fn register_stream(&self, stream_id: u32, priority: StreamPriority) {
        self.streams
            .write()
            .insert(stream_id, StreamState::new(priority));
        debug!(
            "Registered stream {} with priority {:?}",
            stream_id, priority
        );
    }

    /// Unregister a stream
    ///
    /// # Arguments
    /// * `stream_id` - Stream identifier to remove
    pub fn unregister_stream(&self, stream_id: u32) {
        self.streams.write().remove(&stream_id);
        debug!("Unregistered stream {}", stream_id);
    }

    /// Start dispatching frames
    ///
    /// # Arguments
    /// * `input` - Receiver for incoming frames from all streams
    /// * `output` - Sender for dispatched frames
    ///
    /// # Returns
    /// An async task handle
    ///
    /// # Errors
    /// Returns an error if dispatcher fails to start
    pub async fn start(
        self: Arc<Self>,
        mut input: mpsc::Receiver<VideoFrame>,
        output: mpsc::Sender<VideoFrame>,
    ) -> Result<(), DispatchError> {
        *self.running.write() = true;

        debug!("Frame dispatcher started");

        while *self.running.read() {
            // Process incoming frames
            match input.recv().await {
                Some(frame) => {
                    self.handle_incoming_frame(frame).await;
                }
                None => {
                    debug!("Input channel closed, stopping dispatcher");
                    break;
                }
            }

            // Dispatch queued frames
            self.dispatch_frames(&output).await?;
        }

        *self.running.write() = false;
        Ok(())
    }

    /// Stop the dispatcher
    pub fn stop(&self) {
        *self.running.write() = false;
    }

    /// Handle an incoming frame
    async fn handle_incoming_frame(&self, frame: VideoFrame) {
        let start_time = Instant::now();

        // Update stats
        self.stats.write().frames_received += 1;

        // Get stream state
        let stream_id = frame.monitor_index;
        let priority = {
            let mut streams = self.streams.write();
            let state = streams
                .entry(stream_id)
                .or_insert_with(|| StreamState::new(StreamPriority::Normal));
            state.update_frame_received();

            // Check backpressure
            if self.config.enable_backpressure {
                let queue = self.priority_queue.read();
                let queue_usage = queue.len() as f32 / self.config.channel_size as f32;

                if !state.backpressure_active && queue_usage >= self.config.high_water_mark {
                    state.backpressure_active = true;
                    self.stats.write().backpressure_active = true;
                    warn!(
                        "Backpressure activated for stream {} (queue usage: {:.1}%)",
                        stream_id,
                        queue_usage * 100.0
                    );
                } else if state.backpressure_active && queue_usage <= self.config.low_water_mark {
                    state.backpressure_active = false;
                    self.stats.write().backpressure_active = false;
                    debug!(
                        "Backpressure released for stream {} (queue usage: {:.1}%)",
                        stream_id,
                        queue_usage * 100.0
                    );
                }

                // Drop frame if backpressure active
                if state.backpressure_active {
                    trace!(
                        "Dropping frame {} from stream {} due to backpressure",
                        frame.frame_id,
                        stream_id
                    );
                    self.stats.write().frames_dropped_backpressure += 1;
                    return;
                }
            }

            state.priority
        };

        // Create dispatch frame
        let dispatch_frame = DispatchFrame::new(frame, priority);

        // Add to priority queue
        self.enqueue_frame(dispatch_frame);

        // Update dispatch time
        let elapsed = start_time.elapsed();
        self.stats.write().total_dispatch_time_ns += elapsed.as_nanos() as u64;
    }

    /// Enqueue a frame in priority order
    fn enqueue_frame(&self, frame: DispatchFrame) {
        let mut queue = self.priority_queue.write();

        if self.config.priority_dispatch {
            // Insert based on priority (higher priority first)
            let mut insert_idx = queue.len();
            for (idx, queued) in queue.iter().enumerate() {
                if frame.priority > queued.priority {
                    insert_idx = idx;
                    break;
                }
            }
            queue.insert(insert_idx, frame);
        } else {
            // FIFO order
            queue.push_back(frame);
        }

        // Update active streams count
        let active_streams = self.streams.read().len();
        self.stats.write().active_streams = active_streams;
    }

    /// Dispatch frames from the queue
    async fn dispatch_frames(
        &self,
        output: &mpsc::Sender<VideoFrame>,
    ) -> Result<(), DispatchError> {
        let mut queue = self.priority_queue.write();

        // Process all available frames
        while let Some(dispatch_frame) = queue.pop_front() {
            // Check frame age
            if dispatch_frame.is_too_old(self.config.max_frame_age_ms) {
                trace!(
                    "Dropping old frame {} (age: {:?})",
                    dispatch_frame.frame.frame_id,
                    dispatch_frame.age()
                );
                self.stats.write().frames_dropped_age += 1;
                continue;
            }

            // Dispatch frame
            match output.try_send(dispatch_frame.frame.clone()) {
                Ok(_) => {
                    trace!(
                        "Dispatched frame {} with priority {:?}",
                        dispatch_frame.frame.frame_id,
                        dispatch_frame.priority
                    );
                    self.stats.write().frames_dispatched += 1;
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    // Put frame back and stop dispatching
                    warn!("Output channel full, requeueing frame");
                    queue.push_front(dispatch_frame);
                    break;
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    error!("Output channel closed");
                    return Err(DispatchError::ChannelClosed);
                }
            }
        }

        Ok(())
    }

    /// Get dispatcher statistics
    pub fn get_statistics(&self) -> DispatcherStats {
        self.stats.read().clone()
    }

    /// Reset statistics
    pub fn reset_statistics(&self) {
        let mut stats = self.stats.write();
        *stats = DispatcherStats::default();
    }

    /// Check if dispatcher is running
    pub fn is_running(&self) -> bool {
        *self.running.read()
    }

    /// Get active stream count
    pub fn active_stream_count(&self) -> usize {
        self.streams.read().len()
    }

    /// Get queue depth
    pub fn queue_depth(&self) -> usize {
        self.priority_queue.read().len()
    }
}

/// Dispatch errors
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("Channel closed")]
    ChannelClosed,

    #[error("Stream {0} not found")]
    StreamNotFound(u32),

    #[error("Queue overflow: {0} frames")]
    QueueOverflow(usize),

    #[error("Dispatcher not running")]
    NotRunning,

    #[error("Invalid priority: {0}")]
    InvalidPriority(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipewire::format::PixelFormat;

    #[test]
    fn test_dispatcher_config() {
        let config = DispatcherConfig::default();
        assert_eq!(config.channel_size, DEFAULT_CHANNEL_SIZE);
        assert!(config.priority_dispatch);
        assert!(config.enable_backpressure);
    }

    #[test]
    fn test_dispatcher_stats() {
        let mut stats = DispatcherStats::default();
        stats.frames_received = 100;
        stats.frames_dispatched = 90;
        stats.frames_dropped_age = 5;
        stats.frames_dropped_backpressure = 5;

        assert_eq!(stats.drop_rate(), 0.1);
        assert_eq!(stats.dispatch_rate(), 0.9);
    }

    #[test]
    fn test_stream_priority() {
        assert!(StreamPriority::High > StreamPriority::Normal);
        assert!(StreamPriority::Normal > StreamPriority::Low);
    }

    #[test]
    fn test_dispatch_frame() {
        let frame = VideoFrame::new(1, 1920, 1080, 7680, PixelFormat::BGRA, 0);
        let dispatch = DispatchFrame::new(frame, StreamPriority::High);

        assert_eq!(dispatch.priority, StreamPriority::High);
        assert!(!dispatch.is_too_old(MAX_FRAME_AGE_MS));
    }

    #[test]
    fn test_dispatcher_creation() {
        let config = DispatcherConfig::default();
        let dispatcher = FrameDispatcher::new(config);

        assert!(!dispatcher.is_running());
        assert_eq!(dispatcher.active_stream_count(), 0);
        assert_eq!(dispatcher.queue_depth(), 0);
    }

    #[test]
    fn test_stream_registration() {
        let config = DispatcherConfig::default();
        let dispatcher = FrameDispatcher::new(config);

        dispatcher.register_stream(0, StreamPriority::High);
        assert_eq!(dispatcher.active_stream_count(), 1);

        dispatcher.register_stream(1, StreamPriority::Normal);
        assert_eq!(dispatcher.active_stream_count(), 2);

        dispatcher.unregister_stream(0);
        assert_eq!(dispatcher.active_stream_count(), 1);
    }

    #[tokio::test]
    async fn test_dispatcher_lifecycle() {
        let config = DispatcherConfig::default();
        let dispatcher = Arc::new(FrameDispatcher::new(config));

        let (input_tx, input_rx) = mpsc::channel(10);
        let (output_tx, mut output_rx) = mpsc::channel(10);

        // Start dispatcher
        let dispatcher_clone = dispatcher.clone();
        let handle = tokio::spawn(async move { dispatcher_clone.start(input_rx, output_tx).await });

        // Stop dispatcher
        tokio::time::sleep(Duration::from_millis(10)).await;
        dispatcher.stop();

        // Wait for completion
        let result = handle.await;
        assert!(result.is_ok());
    }
}
