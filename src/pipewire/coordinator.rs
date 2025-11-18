//! Multi-Stream Coordination
//!
//! Coordinates multiple PipeWire streams for multi-monitor setups.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::pipewire::error::{PipeWireError, Result};
use crate::pipewire::connection::PipeWireConnection;
use crate::pipewire::stream::{PipeWireStream, StreamConfig, PwStreamState};
use crate::pipewire::frame::VideoFrame;
use crate::portal::session::StreamInfo;

/// Monitor information
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    /// Monitor ID
    pub id: u32,

    /// Monitor name
    pub name: String,

    /// Position
    pub position: (i32, i32),

    /// Size
    pub size: (u32, u32),

    /// Refresh rate
    pub refresh_rate: u32,

    /// PipeWire node ID
    pub node_id: u32,
}

impl MonitorInfo {
    /// Create from StreamInfo
    pub fn from_stream_info(stream_info: &StreamInfo, name: String) -> Self {
        Self {
            id: stream_info.node_id,
            name,
            position: stream_info.position,
            size: stream_info.size,
            refresh_rate: 60, // Default
            node_id: stream_info.node_id,
        }
    }
}

/// Monitor event
#[derive(Debug, Clone)]
pub enum MonitorEvent {
    /// Monitor added
    Added(MonitorInfo),

    /// Monitor removed
    Removed(u32),

    /// Monitor changed
    Changed(MonitorInfo),
}

/// Stream handle
#[derive(Clone)]
pub struct StreamHandle {
    /// Stream ID
    pub id: u32,

    /// Stream reference
    pub stream: Arc<Mutex<PipeWireStream>>,

    /// Monitor information
    pub monitor: MonitorInfo,

    /// Stream state
    pub state: StreamState,

    /// Monitoring task handle
    pub task: Option<Arc<JoinHandle<()>>>,
}

/// Stream state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    /// Stream is initializing
    Initializing,
    /// Stream is active
    Active,
    /// Stream is paused
    Paused,
    /// Stream has error
    Error,
    /// Stream is closing
    Closing,
}

/// Pending stream creation
struct PendingStream {
    monitor: MonitorInfo,
    retry_count: u32,
    last_attempt: Instant,
}

/// Multi-stream configuration
#[derive(Debug, Clone)]
pub struct MultiStreamConfig {
    /// Maximum concurrent streams
    pub max_streams: usize,

    /// Dispatcher configuration
    pub dispatcher_config: DispatcherConfig,

    /// Enable stream synchronization
    pub enable_sync: bool,

    /// Retry configuration
    pub retry_attempts: u32,
}

impl Default for MultiStreamConfig {
    fn default() -> Self {
        Self {
            max_streams: 8,
            dispatcher_config: DispatcherConfig::default(),
            enable_sync: true,
            retry_attempts: 5,
        }
    }
}

/// Dispatcher configuration
#[derive(Debug, Clone)]
pub struct DispatcherConfig {
    /// Frame buffer size per stream
    pub frame_buffer_size: usize,

    /// Enable frame ordering
    pub enable_ordering: bool,
}

impl Default for DispatcherConfig {
    fn default() -> Self {
        Self {
            frame_buffer_size: 32,
            enable_ordering: true,
        }
    }
}

/// Multi-stream coordinator
pub struct MultiStreamCoordinator {
    /// Active streams
    streams: Arc<RwLock<HashMap<u32, StreamHandle>>>,

    /// Pending streams
    pending_streams: Arc<Mutex<Vec<PendingStream>>>,

    /// Frame dispatcher
    frame_dispatcher: Arc<FrameDispatcher>,

    /// Configuration
    config: MultiStreamConfig,

    /// Statistics
    stats: Arc<Mutex<CoordinatorStats>>,
}

impl MultiStreamCoordinator {
    /// Create new coordinator
    pub async fn new(config: MultiStreamConfig) -> Result<Self> {
        Ok(Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
            pending_streams: Arc::new(Mutex::new(Vec::new())),
            frame_dispatcher: Arc::new(FrameDispatcher::new(config.dispatcher_config.clone())),
            config,
            stats: Arc::new(Mutex::new(CoordinatorStats::default())),
        })
    }

    /// Add a stream for a monitor
    pub async fn add_stream(
        &self,
        monitor: MonitorInfo,
        connection: &mut PipeWireConnection,
    ) -> Result<u32> {
        // Check stream limit
        if self.streams.read().await.len() >= self.config.max_streams {
            return Err(PipeWireError::TooManyStreams(self.config.max_streams));
        }

        // Create stream configuration
        let stream_config = StreamConfig::new(monitor.name.clone())
            .with_resolution(monitor.size.0, monitor.size.1)
            .with_framerate(monitor.refresh_rate);

        // Create PipeWire stream via connection
        let stream_id = connection.create_stream(stream_config, monitor.node_id).await?;

        // Get the stream
        if let Some(stream_arc) = connection.get_stream(stream_id).await {
            // Set up frame callback
            let dispatcher = self.frame_dispatcher.clone();
            let monitor_id = monitor.id;

            {
                let mut stream = stream_arc.lock().await;
                stream.set_frame_callback(Box::new(move |frame| {
                    dispatcher.dispatch_frame(monitor_id, frame);
                }));
            }

            // Create stream handle
            let handle = StreamHandle {
                id: stream_id,
                stream: stream_arc.clone(),
                monitor: monitor.clone(),
                state: StreamState::Active,
                task: None, // Monitoring task disabled for Send safety
            };

            // Note: Monitoring task disabled in this implementation
            // In production, use a separate monitoring service that checks stream health
            // without needing to clone StreamHandle across thread boundaries

            // Store stream
            self.streams.write().await.insert(monitor.id, handle);

            // Update stats
            self.stats.lock().await.streams_created += 1;

            Ok(stream_id)
        } else {
            Err(PipeWireError::StreamCreationFailed(
                "Stream not found after creation".to_string()
            ))
        }
    }

    /// Remove a stream
    pub async fn remove_stream(&self, monitor_id: u32) -> Result<()> {
        let mut streams = self.streams.write().await;

        if let Some(mut handle) = streams.remove(&monitor_id) {
            // Stop monitoring task
            if let Some(task) = handle.task.take() {
                if let Ok(task) = Arc::try_unwrap(task) {
                    task.abort();
                }
            }

            // Stop the stream
            handle.stream.lock().await.stop().await?;

            // Update stats
            self.stats.lock().await.streams_destroyed += 1;

            Ok(())
        } else {
            Err(PipeWireError::StreamNotFound(monitor_id))
        }
    }

    /// Handle monitor change event
    pub async fn handle_monitor_event(&self, event: MonitorEvent) -> Result<()> {
        match event {
            MonitorEvent::Added(monitor) => {
                // Queue for stream creation
                self.pending_streams.lock().await.push(PendingStream {
                    monitor,
                    retry_count: 0,
                    last_attempt: Instant::now(),
                });
            }

            MonitorEvent::Removed(monitor_id) => {
                // Remove stream
                self.remove_stream(monitor_id).await?;
            }

            MonitorEvent::Changed(monitor) => {
                // For now, just update monitor info
                // Full implementation would reconfigure the stream
                if let Some(handle) = self.streams.write().await.get_mut(&monitor.id) {
                    handle.monitor = monitor;
                }
            }
        }

        Ok(())
    }

    /// Get active stream count
    pub async fn active_streams(&self) -> usize {
        self.streams.read().await.len()
    }

    /// Get stream by monitor ID
    pub async fn get_stream(&self, monitor_id: u32) -> Option<Arc<Mutex<PipeWireStream>>> {
        self.streams.read().await.get(&monitor_id).map(|h| h.stream.clone())
    }

    /// Get frame receiver for a monitor
    pub async fn get_frame_receiver(&self, monitor_id: u32) -> Option<mpsc::Receiver<VideoFrame>> {
        self.frame_dispatcher.register_receiver(monitor_id).await
    }

    /// Get statistics
    pub async fn stats(&self) -> CoordinatorStats {
        self.stats.lock().await.clone()
    }
}

/// Frame dispatcher
pub struct FrameDispatcher {
    /// Frame receivers indexed by monitor ID
    receivers: Arc<RwLock<HashMap<u32, mpsc::Sender<VideoFrame>>>>,

    /// Configuration
    config: DispatcherConfig,
}

impl FrameDispatcher {
    /// Create new dispatcher
    pub fn new(config: DispatcherConfig) -> Self {
        Self {
            receivers: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Dispatch frame to appropriate receiver
    pub fn dispatch_frame(&self, monitor_id: u32, frame: VideoFrame) {
        // Send to monitor-specific receiver
        if let Some(tx) = self.receivers.blocking_read().get(&monitor_id) {
            let _ = tx.try_send(frame);
        }
    }

    /// Register a new receiver for a monitor
    pub async fn register_receiver(&self, monitor_id: u32) -> Option<mpsc::Receiver<VideoFrame>> {
        let (tx, rx) = mpsc::channel(self.config.frame_buffer_size);
        self.receivers.write().await.insert(monitor_id, tx);
        Some(rx)
    }

    /// Unregister receiver
    pub async fn unregister_receiver(&self, monitor_id: u32) {
        self.receivers.write().await.remove(&monitor_id);
    }
}

/// Coordinator statistics
#[derive(Debug, Clone, Default)]
pub struct CoordinatorStats {
    /// Streams created
    pub streams_created: u64,

    /// Streams destroyed
    pub streams_destroyed: u64,

    /// Stream errors
    pub stream_errors: u64,

    /// Reconnections
    pub reconnections: u64,
}

/// Monitor stream health
async fn monitor_stream_health(handle: StreamHandle) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut stall_count = 0;

    loop {
        interval.tick().await;

        // Check stream state
        let state = handle.stream.lock().await.state();

        match state {
            PwStreamState::Error => {
                tracing::warn!("Stream {} in error state", handle.id);
                // Would trigger recovery here
                break;
            }

            PwStreamState::Closing => {
                tracing::info!("Stream {} closing", handle.id);
                break;
            }

            _ => {
                // Reset stall counter
                stall_count = 0;
            }
        }

        // Exit if not active
        if handle.state != StreamState::Active {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coordinator_creation() {
        let config = MultiStreamConfig::default();
        let coordinator = MultiStreamCoordinator::new(config).await.unwrap();

        assert_eq!(coordinator.active_streams().await, 0);
    }

    #[test]
    fn test_monitor_info() {
        let info = MonitorInfo {
            id: 1,
            name: "Monitor-1".to_string(),
            position: (0, 0),
            size: (1920, 1080),
            refresh_rate: 60,
            node_id: 42,
        };

        assert_eq!(info.id, 1);
        assert_eq!(info.size, (1920, 1080));
    }

    #[tokio::test]
    async fn test_frame_dispatcher() {
        let config = DispatcherConfig::default();
        let dispatcher = FrameDispatcher::new(config);

        let mut rx = dispatcher.register_receiver(1).await.unwrap();

        // Create and dispatch a frame
        use crate::pipewire::frame::VideoFrame;
        use crate::pipewire::format::PixelFormat;

        let frame = VideoFrame::new(1, 100, 100, 400, PixelFormat::BGRA, 1);

        // Spawn a task to dispatch the frame (to avoid blocking in sync context)
        tokio::spawn(async move {
            dispatcher.dispatch_frame(1, frame);
        });

        // Should receive the frame
        let received = tokio::time::timeout(
            Duration::from_millis(100),
            rx.recv()
        ).await;

        assert!(received.is_ok());
    }
}
