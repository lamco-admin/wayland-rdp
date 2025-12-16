//! Clipboard Data Transfer Engine
//!
//! Handles chunked transfer of clipboard data with progress tracking,
//! cancellation support, and integrity verification.

use crate::clipboard::error::{ClipboardError, Result};
use bytes::{Bytes, BytesMut};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, warn};

/// Transfer configuration
#[derive(Debug, Clone)]
pub struct TransferConfig {
    /// Chunk size in bytes
    pub chunk_size: usize,

    /// Maximum data size
    pub max_data_size: usize,

    /// Transfer timeout
    pub timeout: Duration,

    /// Enable integrity verification
    pub verify_integrity: bool,
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            chunk_size: 64 * 1024,           // 64KB chunks
            max_data_size: 16 * 1024 * 1024, // 16MB max
            timeout: Duration::from_secs(30),
            verify_integrity: true,
        }
    }
}

/// Transfer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferState {
    /// Transfer is pending
    Pending,
    /// Transfer is in progress
    InProgress,
    /// Transfer completed successfully
    Completed,
    /// Transfer was cancelled
    Cancelled,
    /// Transfer failed
    Failed,
}

/// Transfer progress information
#[derive(Debug, Clone)]
pub struct TransferProgress {
    /// Transfer ID
    pub transfer_id: u64,

    /// Total bytes to transfer
    pub total_bytes: usize,

    /// Bytes transferred so far
    pub transferred_bytes: usize,

    /// Current state
    pub state: TransferState,

    /// Start time
    pub start_time: Instant,

    /// Last update time
    pub last_update: Instant,

    /// Data hash (if verification enabled)
    pub data_hash: Option<String>,
}

impl TransferProgress {
    /// Get transfer progress percentage (0-100)
    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            return 100.0;
        }
        (self.transferred_bytes as f64 / self.total_bytes as f64) * 100.0
    }

    /// Get transfer speed in bytes/second
    pub fn speed_bps(&self) -> f64 {
        let elapsed = self
            .last_update
            .duration_since(self.start_time)
            .as_secs_f64();
        if elapsed == 0.0 {
            return 0.0;
        }
        self.transferred_bytes as f64 / elapsed
    }

    /// Get estimated time remaining
    pub fn eta(&self) -> Option<Duration> {
        let remaining_bytes = self.total_bytes.saturating_sub(self.transferred_bytes);
        if remaining_bytes == 0 {
            return Some(Duration::ZERO);
        }

        let speed = self.speed_bps();
        if speed == 0.0 {
            return None;
        }

        Some(Duration::from_secs_f64(remaining_bytes as f64 / speed))
    }
}

/// Transfer handle for controlling ongoing transfer
pub struct TransferHandle {
    transfer_id: u64,
    cancel_tx: mpsc::Sender<()>,
    progress_rx: mpsc::Receiver<TransferProgress>,
}

impl TransferHandle {
    /// Get transfer ID
    pub fn id(&self) -> u64 {
        self.transfer_id
    }

    /// Cancel the transfer
    pub async fn cancel(&mut self) -> Result<()> {
        self.cancel_tx
            .send(())
            .await
            .map_err(|_| ClipboardError::ChannelSend)?;
        Ok(())
    }

    /// Get current transfer progress
    pub async fn progress(&mut self) -> Option<TransferProgress> {
        self.progress_rx.recv().await
    }

    /// Wait for transfer completion
    pub async fn wait(mut self) -> Result<TransferProgress> {
        loop {
            match self.progress_rx.recv().await {
                Some(progress) => match progress.state {
                    TransferState::Completed => return Ok(progress),
                    TransferState::Cancelled => return Err(ClipboardError::TransferCancelled),
                    TransferState::Failed => {
                        return Err(ClipboardError::Unknown("Transfer failed".to_string()))
                    }
                    _ => continue,
                },
                None => return Err(ClipboardError::ChannelReceive),
            }
        }
    }
}

/// Transfer engine manages clipboard data transfers
#[derive(Debug)]
pub struct TransferEngine {
    config: TransferConfig,
    next_transfer_id: Arc<RwLock<u64>>,
}

impl TransferEngine {
    /// Create a new transfer engine
    pub fn new(config: TransferConfig) -> Self {
        Self {
            config,
            next_transfer_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Start a chunked send operation
    pub async fn send_chunked(
        &self,
        data: Bytes,
    ) -> Result<(TransferHandle, mpsc::Receiver<Bytes>)> {
        // Validate data size
        if data.len() > self.config.max_data_size {
            return Err(ClipboardError::DataSizeExceeded(
                data.len(),
                self.config.max_data_size,
            ));
        }

        // Generate transfer ID
        let transfer_id = {
            let mut id = self.next_transfer_id.write().await;
            let current = *id;
            *id += 1;
            current
        };

        // Create channels
        let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);
        let (progress_tx, progress_rx) = mpsc::channel::<TransferProgress>(16);
        let (chunk_tx, chunk_rx) = mpsc::channel::<Bytes>(16);

        // Calculate hash if verification enabled
        let data_hash = if self.config.verify_integrity {
            let mut hasher = Sha256::new();
            hasher.update(&data);
            Some(format!("{:x}", hasher.finalize()))
        } else {
            None
        };

        // Initial progress
        let mut progress = TransferProgress {
            transfer_id,
            total_bytes: data.len(),
            transferred_bytes: 0,
            state: TransferState::Pending,
            start_time: Instant::now(),
            last_update: Instant::now(),
            data_hash,
        };

        // Spawn transfer task
        let config = self.config.clone();
        tokio::spawn(async move {
            progress.state = TransferState::InProgress;
            progress.last_update = Instant::now();

            let _ = progress_tx.send(progress.clone()).await;

            let start_time = Instant::now();
            let mut offset = 0;

            while offset < data.len() {
                // Check for cancellation
                if cancel_rx.try_recv().is_ok() {
                    progress.state = TransferState::Cancelled;
                    let _ = progress_tx.send(progress).await;
                    debug!("Transfer {} cancelled", transfer_id);
                    return;
                }

                // Check timeout
                if start_time.elapsed() > config.timeout {
                    progress.state = TransferState::Failed;
                    let _ = progress_tx.send(progress).await;
                    warn!("Transfer {} timed out", transfer_id);
                    return;
                }

                // Send next chunk
                let end = (offset + config.chunk_size).min(data.len());
                let chunk = data.slice(offset..end);

                if chunk_tx.send(chunk).await.is_err() {
                    progress.state = TransferState::Failed;
                    let _ = progress_tx.send(progress).await;
                    warn!("Transfer {} failed: channel closed", transfer_id);
                    return;
                }

                offset = end;
                progress.transferred_bytes = offset;
                progress.last_update = Instant::now();

                let _ = progress_tx.send(progress.clone()).await;

                // Small delay to avoid overwhelming the receiver
                if offset < data.len() {
                    tokio::time::sleep(Duration::from_micros(100)).await;
                }
            }

            // Mark as completed
            progress.state = TransferState::Completed;
            progress.last_update = Instant::now();
            let _ = progress_tx.send(progress).await;

            debug!(
                "Transfer {} completed: {} bytes in {:?}",
                transfer_id,
                data.len(),
                start_time.elapsed()
            );
        });

        let handle = TransferHandle {
            transfer_id,
            cancel_tx,
            progress_rx,
        };

        Ok((handle, chunk_rx))
    }

    /// Receive chunked data
    pub async fn receive_chunked(
        &self,
        mut chunk_rx: mpsc::Receiver<Bytes>,
        expected_size: Option<usize>,
    ) -> Result<Bytes> {
        let transfer_id = {
            let mut id = self.next_transfer_id.write().await;
            let current = *id;
            *id += 1;
            current
        };

        let start_time = Instant::now();
        let mut buffer = BytesMut::new();
        let mut hasher = if self.config.verify_integrity {
            Some(Sha256::new())
        } else {
            None
        };

        debug!("Starting receive transfer {}", transfer_id);

        while let Some(chunk) = tokio::time::timeout(self.config.timeout, chunk_rx.recv())
            .await
            .map_err(|_| ClipboardError::TransferTimeout(self.config.timeout.as_millis() as u64))?
        {
            // Check size limit
            if buffer.len() + chunk.len() > self.config.max_data_size {
                return Err(ClipboardError::DataSizeExceeded(
                    buffer.len() + chunk.len(),
                    self.config.max_data_size,
                ));
            }

            // Verify expected size if provided
            if let Some(expected) = expected_size {
                if buffer.len() + chunk.len() > expected {
                    return Err(ClipboardError::InvalidData(format!(
                        "Received more data than expected: {} > {}",
                        buffer.len() + chunk.len(),
                        expected
                    )));
                }
            }

            if let Some(ref mut h) = hasher {
                h.update(&chunk);
            }

            buffer.extend_from_slice(&chunk);

            debug!(
                "Transfer {} received chunk: {} bytes (total: {})",
                transfer_id,
                chunk.len(),
                buffer.len()
            );
        }

        debug!(
            "Transfer {} completed: {} bytes in {:?}",
            transfer_id,
            buffer.len(),
            start_time.elapsed()
        );

        Ok(buffer.freeze())
    }

    /// Verify data integrity using hash
    pub fn verify_integrity(data: &[u8], expected_hash: &str) -> Result<()> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let actual_hash = format!("{:x}", hasher.finalize());

        if actual_hash != expected_hash {
            return Err(ClipboardError::InvalidData(format!(
                "Hash mismatch: expected {}, got {}",
                expected_hash, actual_hash
            )));
        }

        Ok(())
    }

    /// Calculate hash of data
    pub fn calculate_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

impl Default for TransferEngine {
    fn default() -> Self {
        Self::new(TransferConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chunked_transfer_small_data() {
        let engine = TransferEngine::new(TransferConfig {
            chunk_size: 10,
            ..Default::default()
        });

        let data = Bytes::from("Hello, World!");

        let (mut handle, mut chunk_rx) = engine.send_chunked(data.clone()).await.unwrap();

        // Collect chunks
        let mut received = BytesMut::new();
        while let Some(chunk) = chunk_rx.recv().await {
            received.extend_from_slice(&chunk);
        }

        // Verify data
        assert_eq!(received.freeze(), data);

        // Verify completion
        let progress = handle.wait().await.unwrap();
        assert_eq!(progress.state, TransferState::Completed);
        assert_eq!(progress.transferred_bytes, data.len());
        assert_eq!(progress.percentage(), 100.0);
    }

    #[tokio::test]
    async fn test_chunked_transfer_large_data() {
        let engine = TransferEngine::new(TransferConfig {
            chunk_size: 1024,
            ..Default::default()
        });

        // 10KB of data
        let data = Bytes::from(vec![0x42u8; 10 * 1024]);

        let (mut handle, chunk_rx) = engine.send_chunked(data.clone()).await.unwrap();

        // Receive data
        let received = engine
            .receive_chunked(chunk_rx, Some(data.len()))
            .await
            .unwrap();

        assert_eq!(received, data);

        let progress = handle.wait().await.unwrap();
        assert_eq!(progress.state, TransferState::Completed);
    }

    #[tokio::test]
    async fn test_transfer_cancellation() {
        let engine = TransferEngine::new(TransferConfig {
            chunk_size: 1024,
            timeout: Duration::from_secs(10),
            ..Default::default()
        });

        // Large data to ensure transfer takes enough time for cancellation to be processed
        // With 1KB chunks and 100us delay per chunk, 1MB = ~1024 chunks = ~100ms minimum
        let data = Bytes::from(vec![0u8; 1024 * 1024]);

        let (mut handle, _chunk_rx) = engine.send_chunked(data).await.unwrap();

        // Give the transfer task a moment to start and send initial InProgress
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Cancel the transfer
        handle.cancel().await.unwrap();

        // Use wait() to get the terminal state - it will return Cancelled error or Completed
        let result = handle.wait().await;

        // The cancel should be processed since the transfer takes much longer than our timing
        // wait() returns Err(TransferCancelled) for cancelled transfers
        assert!(
            matches!(result, Err(ClipboardError::TransferCancelled)),
            "Expected transfer to be cancelled, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_transfer_size_limit() {
        let engine = TransferEngine::new(TransferConfig {
            max_data_size: 1024,
            ..Default::default()
        });

        // Data larger than limit
        let data = Bytes::from(vec![0u8; 2048]);

        let result = engine.send_chunked(data).await;
        assert!(result.is_err());

        match result {
            Err(ClipboardError::DataSizeExceeded(_, _)) => {}
            _ => panic!("Expected DataSizeExceeded error"),
        }
    }

    #[tokio::test]
    async fn test_transfer_progress() {
        let engine = TransferEngine::new(TransferConfig {
            chunk_size: 100,
            ..Default::default()
        });

        let data = Bytes::from(vec![0u8; 1000]);

        let (mut handle, _chunk_rx) = engine.send_chunked(data.clone()).await.unwrap();

        let mut last_progress = 0.0;
        while let Some(progress) = handle.progress().await {
            if progress.state == TransferState::Completed {
                assert_eq!(progress.percentage(), 100.0);
                break;
            }

            let current_progress = progress.percentage();
            assert!(current_progress >= last_progress);
            last_progress = current_progress;

            // Verify progress metrics
            assert!(progress.speed_bps() >= 0.0);
            assert!(progress.transferred_bytes <= progress.total_bytes);
        }
    }

    #[test]
    fn test_integrity_verification() {
        let data = b"Test data for integrity verification";
        let hash = TransferEngine::calculate_hash(data);

        // Verify with correct hash
        assert!(TransferEngine::verify_integrity(data, &hash).is_ok());

        // Verify with incorrect hash
        assert!(TransferEngine::verify_integrity(data, "invalid_hash").is_err());
    }

    #[tokio::test]
    async fn test_transfer_with_integrity_check() {
        let engine = TransferEngine::new(TransferConfig {
            verify_integrity: true,
            ..Default::default()
        });

        let data = Bytes::from("Data with integrity check");

        let (handle, chunk_rx) = engine.send_chunked(data.clone()).await.unwrap();

        // Receive and verify
        let received = engine
            .receive_chunked(chunk_rx, Some(data.len()))
            .await
            .unwrap();

        assert_eq!(received, data);

        // Get final progress with hash
        let progress = handle.wait().await.unwrap();
        assert!(progress.data_hash.is_some());

        // Verify hash
        let hash = progress.data_hash.unwrap();
        assert!(TransferEngine::verify_integrity(&received, &hash).is_ok());
    }

    #[tokio::test]
    async fn test_empty_transfer() {
        let engine = TransferEngine::default();

        let data = Bytes::new();

        let (handle, chunk_rx) = engine.send_chunked(data.clone()).await.unwrap();

        let received = engine.receive_chunked(chunk_rx, Some(0)).await.unwrap();

        assert_eq!(received.len(), 0);

        let progress = handle.wait().await.unwrap();
        assert_eq!(progress.state, TransferState::Completed);
        assert_eq!(progress.percentage(), 100.0);
    }

    #[tokio::test]
    async fn test_transfer_eta() {
        let engine = TransferEngine::new(TransferConfig {
            chunk_size: 100,
            ..Default::default()
        });

        let data = Bytes::from(vec![0u8; 1000]);

        let (mut handle, _chunk_rx) = engine.send_chunked(data).await.unwrap();

        // Check ETA during transfer
        if let Some(progress) = handle.progress().await {
            if progress.state == TransferState::InProgress && progress.transferred_bytes > 0 {
                let eta = progress.eta();
                assert!(eta.is_some());
            }
        }
    }
}
