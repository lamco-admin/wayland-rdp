//! Frame Processing Pipeline
//!
//! Handles video frame processing with features including:
//! - Frame rate control and adaptive quality
//! - Frame queue management
//! - Damage region optimization
//! - Performance monitoring
//! - Backpressure handling
//!
//! The processor sits between PipeWire capture and the bitmap converter,
//! managing the flow of frames and adapting to system load.

use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};

use crate::pipewire::frame::VideoFrame;
use crate::video::converter::{BitmapConverter, BitmapUpdate, ConversionError};

/// Default frame queue size
const DEFAULT_QUEUE_SIZE: usize = 30;

/// Default target frame rate (FPS)
const DEFAULT_TARGET_FPS: u32 = 30;

/// Minimum frame interval (nanoseconds)
const MIN_FRAME_INTERVAL_NS: u64 = 16_666_667; // ~60 FPS

/// Maximum frame age before dropping (milliseconds)
const MAX_FRAME_AGE_MS: u64 = 100;

/// Configuration for frame processor
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    /// Target frame rate (FPS)
    pub target_fps: u32,

    /// Maximum frame queue depth
    pub max_queue_depth: usize,

    /// Enable adaptive quality
    pub adaptive_quality: bool,

    /// Damage tracking threshold (0.0-1.0)
    pub damage_threshold: f32,

    /// Drop frames when queue is full
    pub drop_on_full_queue: bool,

    /// Enable performance metrics
    pub enable_metrics: bool,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            target_fps: DEFAULT_TARGET_FPS,
            max_queue_depth: DEFAULT_QUEUE_SIZE,
            adaptive_quality: true,
            damage_threshold: 0.05,
            drop_on_full_queue: true,
            enable_metrics: true,
        }
    }
}

/// Frame processing statistics
#[derive(Debug, Clone, Default)]
pub struct ProcessingStats {
    /// Total frames received
    pub frames_received: u64,

    /// Frames processed successfully
    pub frames_processed: u64,

    /// Frames dropped due to queue full
    pub frames_dropped_queue_full: u64,

    /// Frames dropped due to age
    pub frames_dropped_old: u64,

    /// Frames skipped due to no changes
    pub frames_skipped_no_change: u64,

    /// Total processing time (nanoseconds)
    pub total_processing_time_ns: u64,

    /// Current queue depth
    pub current_queue_depth: usize,

    /// Peak queue depth
    pub peak_queue_depth: usize,
}

impl ProcessingStats {
    /// Get average processing time in milliseconds
    pub fn avg_processing_time_ms(&self) -> f64 {
        if self.frames_processed == 0 {
            0.0
        } else {
            (self.total_processing_time_ns as f64 / self.frames_processed as f64) / 1_000_000.0
        }
    }

    /// Get drop rate
    pub fn drop_rate(&self) -> f64 {
        if self.frames_received == 0 {
            0.0
        } else {
            let total_drops = self.frames_dropped_queue_full + self.frames_dropped_old;
            total_drops as f64 / self.frames_received as f64
        }
    }

    /// Get current FPS
    pub fn current_fps(&self) -> f64 {
        if self.total_processing_time_ns == 0 {
            0.0
        } else {
            (self.frames_processed as f64 * 1_000_000_000.0) / self.total_processing_time_ns as f64
        }
    }
}

/// Frame with metadata
struct QueuedFrame {
    frame: VideoFrame,
    enqueue_time: Instant,
}

impl QueuedFrame {
    fn new(frame: VideoFrame) -> Self {
        Self {
            frame,
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

/// Frame processor
pub struct FrameProcessor {
    config: ProcessorConfig,
    converter: Arc<RwLock<BitmapConverter>>,
    stats: Arc<RwLock<ProcessingStats>>,
    last_frame_time: Arc<RwLock<Option<Instant>>>,
    running: Arc<RwLock<bool>>,
}

impl FrameProcessor {
    /// Create a new frame processor
    ///
    /// # Arguments
    /// * `config` - Processor configuration
    /// * `width` - Frame width in pixels
    /// * `height` - Frame height in pixels
    ///
    /// # Returns
    /// A new `FrameProcessor` instance
    pub fn new(config: ProcessorConfig, width: u16, height: u16) -> Self {
        Self {
            config,
            converter: Arc::new(RwLock::new(BitmapConverter::new(width, height))),
            stats: Arc::new(RwLock::new(ProcessingStats::default())),
            last_frame_time: Arc::new(RwLock::new(None)),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start processing frames
    ///
    /// # Arguments
    /// * `input` - Receiver for incoming video frames
    /// * `output` - Sender for processed bitmap updates
    ///
    /// # Returns
    /// An async task handle
    ///
    /// # Errors
    /// Returns an error if the processor fails to start
    pub async fn start(
        self: Arc<Self>,
        mut input: mpsc::Receiver<VideoFrame>,
        output: mpsc::Sender<BitmapUpdate>,
    ) -> Result<(), ProcessingError> {
        *self.running.write() = true;

        debug!(
            "Frame processor started with target {} FPS",
            self.config.target_fps
        );

        while *self.running.read() {
            // Wait for next frame
            match input.recv().await {
                Some(frame) => {
                    // Update queue depth stats
                    let queue_depth = input.len();
                    let mut stats = self.stats.write();
                    stats.current_queue_depth = queue_depth;
                    if queue_depth > stats.peak_queue_depth {
                        stats.peak_queue_depth = queue_depth;
                    }
                    stats.frames_received += 1;
                    drop(stats);

                    // Check if queue is too full
                    if queue_depth >= self.config.max_queue_depth {
                        if self.config.drop_on_full_queue {
                            warn!(
                                "Frame queue full ({} frames), dropping frame {}",
                                queue_depth, frame.frame_id
                            );
                            self.stats.write().frames_dropped_queue_full += 1;
                            continue;
                        }
                    }

                    // Wrap frame with metadata
                    let queued_frame = QueuedFrame::new(frame);

                    // Check frame age
                    if queued_frame.is_too_old(MAX_FRAME_AGE_MS) {
                        trace!(
                            "Dropping old frame {} (age: {:?})",
                            queued_frame.frame.frame_id,
                            queued_frame.age()
                        );
                        self.stats.write().frames_dropped_old += 1;
                        continue;
                    }

                    // Apply frame rate limiting
                    if !self.should_process_frame(&queued_frame.frame) {
                        continue;
                    }

                    // Process the frame
                    match self.process_frame(queued_frame.frame).await {
                        Ok(bitmap_update) => {
                            // Skip frames with no changes
                            if bitmap_update.rectangles.is_empty() {
                                self.stats.write().frames_skipped_no_change += 1;
                                continue;
                            }

                            // Send processed frame
                            if let Err(e) = output.send(bitmap_update).await {
                                warn!("Failed to send bitmap update: {}", e);
                                break;
                            }

                            self.stats.write().frames_processed += 1;
                        }
                        Err(e) => {
                            warn!("Frame processing error: {}", e);
                            continue;
                        }
                    }
                }
                None => {
                    debug!("Input channel closed, stopping processor");
                    break;
                }
            }
        }

        *self.running.write() = false;
        Ok(())
    }

    /// Stop the processor
    pub fn stop(&self) {
        *self.running.write() = false;
    }

    /// Check if we should process this frame based on frame rate limiting
    fn should_process_frame(&self, frame: &VideoFrame) -> bool {
        let mut last_time = self.last_frame_time.write();

        // Always process first frame
        if last_time.is_none() {
            *last_time = Some(Instant::now());
            return true;
        }

        let last = last_time.unwrap();
        let elapsed = last.elapsed();

        // Calculate minimum interval based on target FPS
        let min_interval = Duration::from_nanos(1_000_000_000 / self.config.target_fps as u64);

        if elapsed >= min_interval {
            *last_time = Some(Instant::now());
            true
        } else {
            false
        }
    }

    /// Process a single frame
    async fn process_frame(&self, frame: VideoFrame) -> Result<BitmapUpdate, ProcessingError> {
        let start_time = Instant::now();

        trace!(
            "Processing frame {} ({}x{}, format: {:?})",
            frame.frame_id,
            frame.width,
            frame.height,
            frame.format
        );

        // Check if frame has significant damage
        if !frame.damage_regions.is_empty() {
            let has_damage = frame.has_significant_damage(self.config.damage_threshold);
            if !has_damage {
                trace!(
                    "Frame {} has insignificant damage, skipping",
                    frame.frame_id
                );
                return Ok(BitmapUpdate { rectangles: vec![] });
            }
        }

        // Convert frame
        let bitmap_update = self
            .converter
            .write()
            .convert_frame(&frame)
            .map_err(|e| ProcessingError::ConversionFailed(e.to_string()))?;

        // Update processing time stats
        let elapsed = start_time.elapsed();
        self.stats.write().total_processing_time_ns += elapsed.as_nanos() as u64;

        trace!("Frame {} processed in {:?}", frame.frame_id, elapsed);

        Ok(bitmap_update)
    }

    /// Get processing statistics
    pub fn get_statistics(&self) -> ProcessingStats {
        self.stats.read().clone()
    }

    /// Reset statistics
    pub fn reset_statistics(&self) {
        let mut stats = self.stats.write();
        *stats = ProcessingStats::default();
    }

    /// Get converter statistics
    pub fn get_converter_statistics(&self) -> crate::video::converter::ConversionStats {
        self.converter.read().get_statistics()
    }

    /// Force full update on next frame
    pub fn force_full_update(&self) {
        self.converter.write().force_full_update();
    }

    /// Check if processor is running
    pub fn is_running(&self) -> bool {
        *self.running.read()
    }
}

/// Processing errors
#[derive(Debug, thiserror::Error)]
pub enum ProcessingError {
    #[error("Conversion failed: {0}")]
    ConversionFailed(String),

    #[error("Queue overflow: max depth {0} exceeded")]
    QueueOverflow(usize),

    #[error("Invalid frame: {0}")]
    InvalidFrame(String),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Processor not running")]
    NotRunning,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipewire::format::PixelFormat;

    #[test]
    fn test_processor_config() {
        let config = ProcessorConfig::default();
        assert_eq!(config.target_fps, DEFAULT_TARGET_FPS);
        assert_eq!(config.max_queue_depth, DEFAULT_QUEUE_SIZE);
        assert!(config.adaptive_quality);
    }

    #[test]
    fn test_processing_stats() {
        let mut stats = ProcessingStats::default();
        stats.frames_received = 100;
        stats.frames_processed = 90;
        stats.frames_dropped_queue_full = 5;
        stats.frames_dropped_old = 5;
        stats.total_processing_time_ns = 500_000_000; // 500ms

        assert_eq!(stats.drop_rate(), 0.1); // 10% drop rate
        assert_eq!(stats.avg_processing_time_ms(), 500.0 / 90.0);
    }

    #[test]
    fn test_queued_frame() {
        let frame = VideoFrame::new(1, 1920, 1080, 7680, PixelFormat::BGRA, 0);
        let queued = QueuedFrame::new(frame);

        assert!(!queued.is_too_old(MAX_FRAME_AGE_MS));
        assert!(queued.age() < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_processor_creation() {
        let config = ProcessorConfig::default();
        let processor = Arc::new(FrameProcessor::new(config, 1920, 1080));

        assert!(!processor.is_running());

        let stats = processor.get_statistics();
        assert_eq!(stats.frames_received, 0);
    }

    #[tokio::test]
    async fn test_processor_lifecycle() {
        let config = ProcessorConfig::default();
        let processor = Arc::new(FrameProcessor::new(config, 1920, 1080));

        let (input_tx, input_rx) = mpsc::channel(10);
        let (output_tx, mut output_rx) = mpsc::channel(10);

        // Start processor
        let processor_clone = processor.clone();
        let handle = tokio::spawn(async move { processor_clone.start(input_rx, output_tx).await });

        // Stop processor
        tokio::time::sleep(Duration::from_millis(10)).await;
        processor.stop();

        // Wait for completion
        let result = handle.await;
        assert!(result.is_ok());
    }
}
