//! PipeWire Connection Management
//!
//! Handles connection to PipeWire daemon via portal file descriptor.

use std::os::fd::RawFd;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{mpsc, Mutex};
use pipewire::{context::Context, core::Core, main_loop::MainLoop};

use crate::pipewire::error::{PipeWireError, Result};
use crate::pipewire::stream::{PipeWireStream, StreamConfig};

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected and ready
    Connected,
    /// Connection error
    Error,
}

/// PipeWire connection statistics
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    /// Number of streams created
    pub streams_created: u64,

    /// Number of streams destroyed
    pub streams_destroyed: u64,

    /// Total frames processed
    pub total_frames: u64,

    /// Total bytes processed
    pub total_bytes: u64,

    /// Connection uptime (seconds)
    pub uptime_secs: u64,

    /// Number of reconnections
    pub reconnections: u64,
}

/// PipeWire connection events
#[derive(Debug, Clone)]
pub enum PipeWireEvent {
    /// Connection established
    Connected,

    /// Connection lost
    Disconnected,

    /// Stream added
    StreamAdded(u32),

    /// Stream removed
    StreamRemoved(u32),

    /// Stream error
    StreamError(u32, String),

    /// Core error
    CoreError(String),
}

/// PipeWire connection manager
pub struct PipeWireConnection {
    /// File descriptor from portal
    fd: RawFd,

    /// Main loop (not running in this simplified version)
    main_loop: Option<MainLoop>,

    /// PipeWire context
    context: Option<Context>,

    /// PipeWire core
    core: Option<Core>,

    /// Active streams
    streams: Arc<Mutex<HashMap<u32, Arc<Mutex<PipeWireStream>>>>>,

    /// Connection state
    state: Arc<Mutex<ConnectionState>>,

    /// Event sender
    event_tx: Option<mpsc::Sender<PipeWireEvent>>,

    /// Statistics
    stats: Arc<Mutex<ConnectionStats>>,

    /// Next stream ID
    next_stream_id: Arc<Mutex<u32>>,
}

impl PipeWireConnection {
    /// Create new PipeWire connection
    pub fn new(fd: RawFd) -> Result<Self> {
        Ok(Self {
            fd,
            main_loop: None,
            context: None,
            core: None,
            streams: Arc::new(Mutex::new(HashMap::new())),
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            event_tx: None,
            stats: Arc::new(Mutex::new(ConnectionStats::default())),
            next_stream_id: Arc::new(Mutex::new(0)),
        })
    }

    /// Initialize PipeWire connection
    ///
    /// Note: This is a simplified version that doesn't actually connect to PipeWire.
    /// The full implementation would use pipewire::MainLoop and connect via FD.
    pub async fn connect(&mut self) -> Result<()> {
        *self.state.lock().await = ConnectionState::Connecting;

        // In a full implementation, we would:
        // 1. Initialize PipeWire library
        // 2. Create main loop
        // 3. Create context
        // 4. Connect core using the portal FD
        // 5. Set up event listeners

        // For this implementation, we'll mark as connected
        // Real connection would be established in integration tests with actual PipeWire
        *self.state.lock().await = ConnectionState::Connected;

        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(PipeWireEvent::Connected).await;
        }

        Ok(())
    }

    /// Disconnect from PipeWire
    pub async fn disconnect(&mut self) -> Result<()> {
        *self.state.lock().await = ConnectionState::Disconnected;

        // Stop all streams
        let stream_ids: Vec<u32> = self.streams.lock().await.keys().copied().collect();
        for id in stream_ids {
            self.remove_stream(id).await?;
        }

        // Clean up PipeWire resources
        self.core = None;
        self.context = None;
        self.main_loop = None;

        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(PipeWireEvent::Disconnected).await;
        }

        Ok(())
    }

    /// Get connection state
    pub async fn state(&self) -> ConnectionState {
        *self.state.lock().await
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.state.lock().await == ConnectionState::Connected
    }

    /// Create a new stream
    pub async fn create_stream(
        &mut self,
        config: StreamConfig,
        node_id: u32,
    ) -> Result<u32> {
        if !self.is_connected().await {
            return Err(PipeWireError::ConnectionFailed(
                "Not connected to PipeWire".to_string()
            ));
        }

        // Generate stream ID
        let stream_id = {
            let mut id = self.next_stream_id.lock().await;
            let sid = *id;
            *id += 1;
            sid
        };

        // Create stream
        let mut stream = PipeWireStream::new(stream_id, config);

        // In a full implementation, we would:
        // 1. Get the core
        // 2. Call stream.connect(core, node_id)
        // 3. Wait for stream to be ready

        // For now, just start it
        stream.start().await?;

        // Store stream
        self.streams.lock().await.insert(stream_id, Arc::new(Mutex::new(stream)));

        // Update stats
        self.stats.lock().await.streams_created += 1;

        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(PipeWireEvent::StreamAdded(stream_id)).await;
        }

        Ok(stream_id)
    }

    /// Get a stream
    pub async fn get_stream(&self, stream_id: u32) -> Option<Arc<Mutex<PipeWireStream>>> {
        self.streams.lock().await.get(&stream_id).cloned()
    }

    /// Remove a stream
    pub async fn remove_stream(&mut self, stream_id: u32) -> Result<()> {
        if let Some(stream_arc) = self.streams.lock().await.remove(&stream_id) {
            // Stop the stream
            let mut stream = stream_arc.lock().await;
            stream.stop().await?;

            // Update stats
            self.stats.lock().await.streams_destroyed += 1;

            if let Some(ref tx) = self.event_tx {
                let _ = tx.send(PipeWireEvent::StreamRemoved(stream_id)).await;
            }

            Ok(())
        } else {
            Err(PipeWireError::StreamNotFound(stream_id))
        }
    }

    /// Get all active stream IDs
    pub async fn active_streams(&self) -> Vec<u32> {
        self.streams.lock().await.keys().copied().collect()
    }

    /// Get stream count
    pub async fn stream_count(&self) -> usize {
        self.streams.lock().await.len()
    }

    /// Set event channel
    pub fn set_event_channel(&mut self, tx: mpsc::Sender<PipeWireEvent>) {
        self.event_tx = Some(tx);
    }

    /// Get statistics
    pub async fn stats(&self) -> ConnectionStats {
        self.stats.lock().await.clone()
    }

    /// Get file descriptor
    pub fn fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for PipeWireConnection {
    fn drop(&mut self) {
        // Clean up resources
        let _ = futures::executor::block_on(self.disconnect());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_creation() {
        let conn = PipeWireConnection::new(3).unwrap();
        assert_eq!(conn.state().await, ConnectionState::Disconnected);
        assert_eq!(conn.fd(), 3);
    }

    #[tokio::test]
    async fn test_connection_lifecycle() {
        let mut conn = PipeWireConnection::new(3).unwrap();

        assert_eq!(conn.state().await, ConnectionState::Disconnected);
        assert!(!conn.is_connected().await);

        conn.connect().await.unwrap();
        assert_eq!(conn.state().await, ConnectionState::Connected);
        assert!(conn.is_connected().await);

        conn.disconnect().await.unwrap();
        assert_eq!(conn.state().await, ConnectionState::Disconnected);
        assert!(!conn.is_connected().await);
    }

    #[tokio::test]
    async fn test_stream_creation() {
        let mut conn = PipeWireConnection::new(3).unwrap();
        conn.connect().await.unwrap();

        let config = StreamConfig::new("test-stream");
        let stream_id = conn.create_stream(config, 42).await.unwrap();

        assert_eq!(conn.stream_count().await, 1);
        assert!(conn.get_stream(stream_id).await.is_some());

        let stats = conn.stats().await;
        assert_eq!(stats.streams_created, 1);
    }

    #[tokio::test]
    async fn test_stream_removal() {
        let mut conn = PipeWireConnection::new(3).unwrap();
        conn.connect().await.unwrap();

        let config = StreamConfig::new("test-stream");
        let stream_id = conn.create_stream(config, 42).await.unwrap();

        assert_eq!(conn.stream_count().await, 1);

        conn.remove_stream(stream_id).await.unwrap();
        assert_eq!(conn.stream_count().await, 0);

        let stats = conn.stats().await;
        assert_eq!(stats.streams_created, 1);
        assert_eq!(stats.streams_destroyed, 1);
    }

    #[tokio::test]
    async fn test_multiple_streams() {
        let mut conn = PipeWireConnection::new(3).unwrap();
        conn.connect().await.unwrap();

        let config1 = StreamConfig::new("stream-1");
        let config2 = StreamConfig::new("stream-2");
        let config3 = StreamConfig::new("stream-3");

        let id1 = conn.create_stream(config1, 42).await.unwrap();
        let id2 = conn.create_stream(config2, 43).await.unwrap();
        let id3 = conn.create_stream(config3, 44).await.unwrap();

        assert_eq!(conn.stream_count().await, 3);

        let active = conn.active_streams().await;
        assert!(active.contains(&id1));
        assert!(active.contains(&id2));
        assert!(active.contains(&id3));
    }

    #[tokio::test]
    async fn test_event_channel() {
        let mut conn = PipeWireConnection::new(3).unwrap();

        let (tx, mut rx) = mpsc::channel(10);
        conn.set_event_channel(tx);

        conn.connect().await.unwrap();

        // Should receive Connected event
        if let Some(event) = rx.recv().await {
            assert!(matches!(event, PipeWireEvent::Connected));
        }
    }
}
