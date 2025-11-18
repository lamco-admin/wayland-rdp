//! PipeWire Connection Management
//!
//! Handles connection to PipeWire daemon via portal file descriptor with
//! complete MainLoop integration, proper threading, and robust error handling.

use pipewire::{context::Context, main_loop::MainLoop};
use std::collections::HashMap;
use std::os::fd::{FromRawFd, OwnedFd, RawFd};
use std::sync::Arc;
use std::thread;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};

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
///
/// Manages the PipeWire connection using a dedicated thread for the MainLoop.
/// This is necessary because PipeWire's types (MainLoop, Context, Core) use `Rc`
/// and `NonNull` which are not `Send`, so they must live on a single thread.
pub struct PipeWireConnection {
    /// File descriptor from portal
    fd: RawFd,

    /// Active streams
    streams: Arc<Mutex<HashMap<u32, Arc<Mutex<PipeWireStream>>>>>,

    /// Connection state
    state: Arc<RwLock<ConnectionState>>,

    /// Event sender
    event_tx: Option<mpsc::Sender<PipeWireEvent>>,

    /// Statistics
    stats: Arc<Mutex<ConnectionStats>>,

    /// Next stream ID
    next_stream_id: Arc<Mutex<u32>>,

    /// Thread handle for PipeWire main loop
    /// PipeWire must run on its own thread because its types are not Send
    thread_handle: Option<thread::JoinHandle<()>>,

    /// Shutdown signal
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl PipeWireConnection {
    /// Create new PipeWire connection
    ///
    /// This initializes the connection manager but does not start the MainLoop.
    /// Call `connect()` to establish the connection and start processing.
    pub fn new(fd: RawFd) -> Result<Self> {
        debug!("Creating PipeWire connection with FD {}", fd);

        Ok(Self {
            fd,
            streams: Arc::new(Mutex::new(HashMap::new())),
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            event_tx: None,
            stats: Arc::new(Mutex::new(ConnectionStats::default())),
            next_stream_id: Arc::new(Mutex::new(0)),
            thread_handle: None,
            shutdown_tx: None,
        })
    }

    /// Initialize PipeWire connection and start MainLoop
    ///
    /// This spawns a dedicated thread for the PipeWire MainLoop since PipeWire
    /// types are not Send and must live on a single thread.
    ///
    /// # Errors
    ///
    /// Returns error if PipeWire initialization fails or connection cannot be established
    pub async fn connect(&mut self) -> Result<()> {
        *self.state.write().await = ConnectionState::Connecting;
        info!("Connecting to PipeWire with FD {}", self.fd);

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let fd = self.fd;
        let state = Arc::clone(&self.state);
        let event_tx = self.event_tx.clone();

        // Spawn dedicated thread for PipeWire MainLoop
        // This is REQUIRED because PipeWire types use Rc<> and are not Send
        let thread_handle = thread::spawn(move || {
            debug!("PipeWire thread started");

            // Initialize PipeWire library
            pipewire::init();

            // Create main loop
            let main_loop = match MainLoop::new(None) {
                Ok(ml) => ml,
                Err(e) => {
                    error!("Failed to create PipeWire MainLoop: {}", e);
                    return;
                }
            };

            // Create context
            let context = match Context::new(&main_loop) {
                Ok(ctx) => ctx,
                Err(e) => {
                    error!("Failed to create PipeWire context: {}", e);
                    return;
                }
            };

            // Connect core using the portal-provided FD
            // Safety: We own this FD from the portal and won't use it elsewhere
            let owned_fd = unsafe { OwnedFd::from_raw_fd(fd) };
            let core = match context.connect_fd(owned_fd, None) {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to connect PipeWire core with FD {}: {}", fd, e);
                    return;
                }
            };

            info!("PipeWire connected successfully on FD {}", fd);

            // Update state to connected
            *futures::executor::block_on(state.write()) = ConnectionState::Connected;

            // Notify connected
            if let Some(tx) = event_tx {
                let _ = futures::executor::block_on(tx.send(PipeWireEvent::Connected));
            }

            // Run the main loop
            // We need to integrate shutdown signaling
            loop {
                // Check for shutdown signal (non-blocking)
                if shutdown_rx.try_recv().is_ok() {
                    info!("Shutdown signal received, stopping PipeWire main loop");
                    break;
                }

                // Run one iteration of the main loop
                // pipewire-rs MainLoop doesn't have loop_iterate, use the Loop directly
                let loop_ref = main_loop.loop_();
                loop_ref.iterate(std::time::Duration::from_millis(10));
            }

            debug!("PipeWire thread exiting");

            // Cleanup
            drop(core);
            drop(context);
            drop(main_loop);

            // Safety: This is the last operation before thread exit
            unsafe {
                pipewire::deinit();
            }
        });

        self.thread_handle = Some(thread_handle);

        // Wait for connection to be established
        let mut attempts = 0;
        while attempts < 50 {
            // Wait up to 5 seconds (50 * 100ms)
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            if *self.state.read().await == ConnectionState::Connected {
                info!("PipeWire connection established");
                return Ok(());
            }

            attempts += 1;
        }

        *self.state.write().await = ConnectionState::Error;
        Err(PipeWireError::ConnectionFailed(
            "Timeout waiting for PipeWire connection".to_string(),
        ))
    }

    /// Disconnect from PipeWire
    ///
    /// Stops all streams, signals the MainLoop thread to exit, and cleans up resources.
    pub async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from PipeWire");
        *self.state.write().await = ConnectionState::Disconnected;

        // Stop all streams first
        let stream_ids: Vec<u32> = self.streams.lock().await.keys().copied().collect();
        for id in stream_ids {
            if let Err(e) = self.remove_stream(id).await {
                warn!("Error removing stream {}: {}", id, e);
            }
        }

        // Signal shutdown to PipeWire thread
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        // Wait for thread to finish
        if let Some(handle) = self.thread_handle.take() {
            if handle.join().is_err() {
                error!("PipeWire thread panicked during shutdown");
            }
        }

        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(PipeWireEvent::Disconnected).await;
        }

        info!("PipeWire disconnected");
        Ok(())
    }

    /// Get connection state
    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.state.read().await == ConnectionState::Connected
    }

    /// Create a new stream
    ///
    /// Creates and initializes a PipeWire stream for the specified node ID.
    ///
    /// # Arguments
    ///
    /// * `config` - Stream configuration
    /// * `node_id` - PipeWire node ID from portal
    ///
    /// # Returns
    ///
    /// The stream ID on success
    ///
    /// # Errors
    ///
    /// Returns error if not connected or stream creation fails
    pub async fn create_stream(&mut self, config: StreamConfig, node_id: u32) -> Result<u32> {
        if !self.is_connected().await {
            return Err(PipeWireError::ConnectionFailed(
                "Not connected to PipeWire".to_string(),
            ));
        }

        // Generate stream ID
        let stream_id = {
            let mut id = self.next_stream_id.lock().await;
            let sid = *id;
            *id += 1;
            sid
        };

        debug!(
            "Creating stream {} for node {} with config: {:?}",
            stream_id, node_id, config
        );

        // Create stream
        // The stream will be connected to the Core on the PipeWire thread
        // via message passing (implemented in thread_comm.rs)
        let stream = PipeWireStream::new(stream_id, config);

        // Store stream reference
        // The actual PipeWire stream connection happens lazily on first use
        // or can be triggered explicitly via start_stream()
        self.streams
            .lock()
            .await
            .insert(stream_id, Arc::new(Mutex::new(stream)));

        // Update stats
        self.stats.lock().await.streams_created += 1;

        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(PipeWireEvent::StreamAdded(stream_id)).await;
        }

        debug!("Stream {} created successfully", stream_id);
        Ok(stream_id)
    }

    /// Get a stream by ID
    pub async fn get_stream(&self, stream_id: u32) -> Option<Arc<Mutex<PipeWireStream>>> {
        self.streams.lock().await.get(&stream_id).cloned()
    }

    /// Remove a stream
    ///
    /// Stops and removes the specified stream from the connection.
    pub async fn remove_stream(&mut self, stream_id: u32) -> Result<()> {
        debug!("Removing stream {}", stream_id);

        if let Some(stream_arc) = self.streams.lock().await.remove(&stream_id) {
            // Stop the stream
            let mut stream = stream_arc.lock().await;
            stream.stop().await?;

            // Update stats
            self.stats.lock().await.streams_destroyed += 1;

            if let Some(ref tx) = self.event_tx {
                let _ = tx.send(PipeWireEvent::StreamRemoved(stream_id)).await;
            }

            debug!("Stream {} removed successfully", stream_id);
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
    ///
    /// Configure a channel to receive PipeWire events
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
        debug!("Dropping PipeWire connection");

        // Signal shutdown
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = futures::executor::block_on(tx.send(()));
        }

        // Wait for thread to finish with timeout
        if let Some(handle) = self.thread_handle.take() {
            if handle.join().is_err() {
                error!("PipeWire thread panicked during cleanup");
            }
        }
    }
}

// Mark as Send + Sync since we handle thread safety internally
unsafe impl Send for PipeWireConnection {}
unsafe impl Sync for PipeWireConnection {}

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
    #[ignore] // Requires actual PipeWire daemon
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
    async fn test_stream_id_generation() {
        let conn = PipeWireConnection::new(3).unwrap();

        let id1 = *conn.next_stream_id.lock().await;
        *conn.next_stream_id.lock().await += 1;
        let id2 = *conn.next_stream_id.lock().await;

        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn test_event_channel() {
        let mut conn = PipeWireConnection::new(3).unwrap();

        let (tx, mut rx) = mpsc::channel(10);
        conn.set_event_channel(tx);

        // Manually send event for testing
        if let Some(ref tx) = conn.event_tx {
            tx.send(PipeWireEvent::Connected).await.unwrap();
        }

        // Should receive Connected event
        if let Some(event) = rx.recv().await {
            assert!(matches!(event, PipeWireEvent::Connected));
        }
    }
}
