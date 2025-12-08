//! Clipboard Synchronization and Loop Prevention
//!
//! Manages bidirectional clipboard synchronization between RDP and Wayland,
//! with sophisticated loop detection and prevention mechanisms.

use crate::clipboard::error::Result;
use crate::clipboard::formats::ClipboardFormat;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::time::{Duration, SystemTime};
use tracing::{debug, warn};

/// Clipboard ownership state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardState {
    /// No active clipboard data
    Idle,
    /// RDP client owns the clipboard
    RdpOwned(Vec<ClipboardFormat>),
    /// Wayland/Portal owns the clipboard
    PortalOwned(Vec<String>),
    /// Currently syncing data
    Syncing(SyncDirection),
}

/// Synchronization direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    /// RDP → Portal
    RdpToPortal,
    /// Portal → RDP
    PortalToRdp,
}

/// Loop detection configuration
#[derive(Debug, Clone)]
pub struct LoopDetectionConfig {
    /// Time window for loop detection (milliseconds)
    pub window_ms: u64,

    /// Maximum history size
    pub max_history: usize,

    /// Enable content hashing for better detection
    pub enable_content_hashing: bool,
}

impl Default for LoopDetectionConfig {
    fn default() -> Self {
        Self {
            window_ms: 500,
            max_history: 10,
            enable_content_hashing: true,
        }
    }
}

/// Clipboard operation record for loop detection
#[derive(Debug, Clone)]
struct ClipboardOperation {
    /// Timestamp of operation
    timestamp: SystemTime,
    /// Source of operation
    source: OperationSource,
    /// Hash of format list
    format_hash: String,
}

/// Operation source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperationSource {
    /// RDP client
    Rdp,
    /// Portal/Wayland
    Portal,
}

/// Content hash entry for loop detection
#[derive(Debug, Clone)]
struct ContentHash {
    /// Timestamp
    timestamp: SystemTime,
    /// Content hash
    hash: String,
    /// Source
    source: OperationSource,
}

/// Loop detector prevents clipboard synchronization loops
#[derive(Debug)]
pub struct LoopDetector {
    /// Recent clipboard operations
    history: VecDeque<ClipboardOperation>,
    /// Content hashes for comparison
    content_cache: VecDeque<ContentHash>,
    /// Detection configuration
    config: LoopDetectionConfig,
}

impl LoopDetector {
    /// Create a new loop detector
    pub fn new(config: LoopDetectionConfig) -> Self {
        Self {
            history: VecDeque::with_capacity(config.max_history),
            content_cache: VecDeque::with_capacity(5),
            config,
        }
    }

    /// Check if format list from RDP would cause a loop
    pub fn would_cause_loop(&mut self, formats: &[ClipboardFormat]) -> bool {
        let now = SystemTime::now();
        let format_hash = self.hash_formats(formats);

        // Clean old entries
        self.clean_old_entries(now);

        // Check if same format list was recently sent from Portal
        for op in &self.history {
            if matches!(op.source, OperationSource::Portal) {
                let age = now
                    .duration_since(op.timestamp)
                    .unwrap_or(Duration::from_secs(0));

                if age.as_millis() < self.config.window_ms as u128 {
                    if op.format_hash == format_hash {
                        debug!(
                            "Loop detected: RDP format list matches recent Portal operation (age: {:?})",
                            age
                        );
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if MIME type list from Portal would cause a loop
    pub fn would_cause_loop_mime(&mut self, mime_types: &[String]) -> bool {
        let now = SystemTime::now();
        let format_hash = self.hash_mime_types(mime_types);

        // Clean old entries
        self.clean_old_entries(now);

        // Check if same MIME types were recently sent from RDP
        for op in &self.history {
            if matches!(op.source, OperationSource::Rdp) {
                let age = now
                    .duration_since(op.timestamp)
                    .unwrap_or(Duration::from_secs(0));

                if age.as_millis() < self.config.window_ms as u128 {
                    if op.format_hash == format_hash {
                        debug!(
                            "Loop detected: Portal MIME types match recent RDP operation (age: {:?})",
                            age
                        );
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Record RDP clipboard operation
    pub fn record_rdp_operation(&mut self, formats: Vec<ClipboardFormat>) {
        let operation = ClipboardOperation {
            timestamp: SystemTime::now(),
            source: OperationSource::Rdp,
            format_hash: self.hash_formats(&formats),
        };

        self.add_to_history(operation);
    }

    /// Record Portal clipboard operation
    pub fn record_portal_operation(&mut self, mime_types: Vec<String>) {
        let operation = ClipboardOperation {
            timestamp: SystemTime::now(),
            source: OperationSource::Portal,
            format_hash: self.hash_mime_types(&mime_types),
        };

        self.add_to_history(operation);
    }

    /// Check if content would cause a loop
    pub fn check_content_loop(&mut self, content: &[u8], source: OperationSource) -> bool {
        if !self.config.enable_content_hashing {
            return false;
        }

        let now = SystemTime::now();
        let content_hash = self.hash_content(content);

        // Clean old content hashes
        self.clean_old_content(now);

        // Check if same content was recently processed from opposite source
        for cached in &self.content_cache {
            let age = now
                .duration_since(cached.timestamp)
                .unwrap_or(Duration::from_secs(0));

            if age.as_millis() < self.config.window_ms as u128 {
                let is_opposite_source = match (&cached.source, &source) {
                    (OperationSource::Rdp, OperationSource::Portal) => true,
                    (OperationSource::Portal, OperationSource::Rdp) => true,
                    _ => false,
                };

                if is_opposite_source && cached.hash == content_hash {
                    debug!(
                        "Content loop detected: identical content from opposite source (age: {:?})",
                        age
                    );
                    return true;
                }
            }
        }

        // Store content hash
        self.content_cache.push_back(ContentHash {
            timestamp: now,
            hash: content_hash,
            source,
        });

        // Limit cache size
        while self.content_cache.len() > 5 {
            self.content_cache.pop_front();
        }

        false
    }

    /// Hash clipboard formats
    fn hash_formats(&self, formats: &[ClipboardFormat]) -> String {
        let mut hasher = Sha256::new();

        // Sort formats for consistent hashing
        let mut sorted_formats = formats.to_vec();
        sorted_formats.sort_by_key(|f| f.format_id);

        for format in sorted_formats {
            hasher.update(format.format_id.to_le_bytes());
            hasher.update(format.format_name.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }

    /// Hash MIME types
    fn hash_mime_types(&self, mime_types: &[String]) -> String {
        let mut hasher = Sha256::new();

        // Sort for consistent hashing
        let mut sorted_types = mime_types.to_vec();
        sorted_types.sort();

        for mime_type in sorted_types {
            hasher.update(mime_type.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }

    /// Hash content
    fn hash_content(&self, content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    /// Add operation to history
    fn add_to_history(&mut self, operation: ClipboardOperation) {
        self.history.push_back(operation);

        // Limit history size
        while self.history.len() > self.config.max_history {
            self.history.pop_front();
        }
    }

    /// Clean old entries from history
    fn clean_old_entries(&mut self, now: SystemTime) {
        let threshold = Duration::from_millis(self.config.window_ms * 2);

        self.history.retain(|op| {
            now.duration_since(op.timestamp)
                .unwrap_or(Duration::from_secs(0))
                < threshold
        });
    }

    /// Clean old content hashes
    fn clean_old_content(&mut self, now: SystemTime) {
        let threshold = Duration::from_millis(self.config.window_ms * 2);

        self.content_cache.retain(|cached| {
            now.duration_since(cached.timestamp)
                .unwrap_or(Duration::from_secs(0))
                < threshold
        });
    }

    /// Reset detector state
    pub fn reset(&mut self) {
        self.history.clear();
        self.content_cache.clear();
    }

    /// Get number of operations in history
    pub fn history_size(&self) -> usize {
        self.history.len()
    }

    /// Get number of content hashes cached
    pub fn cache_size(&self) -> usize {
        self.content_cache.len()
    }
}

impl Default for LoopDetector {
    fn default() -> Self {
        Self::new(LoopDetectionConfig::default())
    }
}

/// Synchronization manager coordinates clipboard sync
#[derive(Debug)]
pub struct SyncManager {
    /// Current clipboard state
    state: ClipboardState,
    /// Loop detector
    loop_detector: LoopDetector,
}

impl SyncManager {
    /// Create a new synchronization manager
    pub fn new(loop_detector: LoopDetector) -> Self {
        Self {
            state: ClipboardState::Idle,
            loop_detector,
        }
    }

    /// Get current state
    pub fn state(&self) -> &ClipboardState {
        &self.state
    }

    /// Handle RDP format list announcement
    pub fn handle_rdp_formats(&mut self, formats: Vec<ClipboardFormat>) -> Result<bool> {
        // Check for loop
        if self.loop_detector.would_cause_loop(&formats) {
            warn!("Ignoring RDP format list due to loop detection");
            return Ok(false); // Don't sync
        }

        // Update state
        self.state = ClipboardState::RdpOwned(formats.clone());

        // Record operation
        self.loop_detector.record_rdp_operation(formats);

        Ok(true) // Proceed with sync
    }

    /// Handle Portal MIME types announcement
    ///
    /// The `force` parameter indicates if this is from an authoritative source (D-Bus extension)
    /// that should override RDP ownership. When force=false, we block if RDP owns the clipboard
    /// to prevent echo loops. When force=true (D-Bus), we always process because the user
    /// genuinely copied something new on the Linux side.
    pub fn handle_portal_formats(&mut self, mime_types: Vec<String>, force: bool) -> Result<bool> {
        // If RDP currently owns the clipboard, only block non-authoritative sources.
        // D-Bus extension (force=true) is authoritative and should override RDP ownership.
        // Portal SelectionOwnerChanged (force=false) may be echo of our SetSelection.
        if !force && matches!(self.state, ClipboardState::RdpOwned(_)) {
            debug!("Ignoring Portal format list - RDP currently owns clipboard (preventing echo loop)");
            return Ok(false); // Don't sync back to RDP
        }

        // Check for loop using hash comparison (belt and suspenders)
        if self.loop_detector.would_cause_loop_mime(&mime_types) {
            warn!("Ignoring Portal format list due to loop detection");
            return Ok(false); // Don't sync
        }

        // Update state - Linux now owns clipboard
        self.state = ClipboardState::PortalOwned(mime_types.clone());
        if force {
            debug!("D-Bus extension signal - taking clipboard ownership from RDP");
        }

        // Record operation
        self.loop_detector.record_portal_operation(mime_types);

        Ok(true) // Proceed with sync
    }

    /// Check if content would cause loop
    pub fn check_content(&mut self, content: &[u8], from_rdp: bool) -> Result<bool> {
        let source = if from_rdp {
            OperationSource::Rdp
        } else {
            OperationSource::Portal
        };

        if self.loop_detector.check_content_loop(content, source) {
            warn!("Content loop detected, skipping transfer");
            return Ok(false); // Don't transfer
        }

        Ok(true) // Proceed with transfer
    }

    /// Set syncing state
    pub fn set_syncing(&mut self, direction: SyncDirection) {
        self.state = ClipboardState::Syncing(direction);
    }

    /// Reset to idle state
    pub fn reset(&mut self) {
        self.state = ClipboardState::Idle;
    }

    /// Reset loop detector
    pub fn reset_loop_detector(&mut self) {
        self.loop_detector.reset();
    }

    /// Check if RDP format list would cause a loop
    ///
    /// # Arguments
    ///
    /// * `formats` - RDP clipboard formats
    ///
    /// # Returns
    ///
    /// True if this would cause a loop, false otherwise
    pub fn would_cause_loop_rdp(&mut self, formats: &[ClipboardFormat]) -> Result<bool> {
        Ok(self.loop_detector.would_cause_loop(formats))
    }

    /// Set RDP formats as current clipboard owner
    ///
    /// # Arguments
    ///
    /// * `formats` - RDP clipboard formats
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    pub fn set_rdp_formats(&mut self, formats: Vec<ClipboardFormat>) -> Result<()> {
        self.state = ClipboardState::RdpOwned(formats.clone());
        self.loop_detector.record_rdp_operation(formats);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_detection_rdp_to_portal() {
        let config = LoopDetectionConfig {
            window_ms: 500,
            ..Default::default()
        };
        let mut detector = LoopDetector::new(config);

        let formats = vec![ClipboardFormat {
            format_id: 13,
            format_name: "CF_UNICODETEXT".to_string(),
        }];

        // First operation should not cause loop
        assert!(!detector.would_cause_loop(&formats));
        detector.record_rdp_operation(formats.clone());

        // Simulate Portal operation with same formats
        let mime_types = vec!["text/plain".to_string()];
        detector.record_portal_operation(mime_types);

        // Now RDP with same format should trigger loop detection
        assert!(detector.would_cause_loop(&formats));
    }

    #[test]
    fn test_loop_detection_portal_to_rdp() {
        let mut detector = LoopDetector::default();

        let mime_types = vec!["text/plain".to_string()];

        // First operation should not cause loop
        assert!(!detector.would_cause_loop_mime(&mime_types));
        detector.record_portal_operation(mime_types.clone());

        // Simulate RDP operation with corresponding formats
        let formats = vec![ClipboardFormat {
            format_id: 13,
            format_name: "CF_UNICODETEXT".to_string(),
        }];
        detector.record_rdp_operation(formats);

        // Now Portal with same MIME types should trigger loop detection
        assert!(detector.would_cause_loop_mime(&mime_types));
    }

    #[tokio::test]
    async fn test_loop_detection_expires() {
        let config = LoopDetectionConfig {
            window_ms: 100, // Short window for testing
            ..Default::default()
        };
        let mut detector = LoopDetector::new(config);

        let formats = vec![ClipboardFormat {
            format_id: 13,
            format_name: "CF_UNICODETEXT".to_string(),
        }];

        detector.record_rdp_operation(formats.clone());

        let mime_types = vec!["text/plain".to_string()];
        detector.record_portal_operation(mime_types.clone());

        // Should detect loop immediately
        assert!(detector.would_cause_loop(&formats));

        // Wait for window to expire
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should not detect loop after expiration
        assert!(!detector.would_cause_loop(&formats));
    }

    #[test]
    fn test_content_loop_detection() {
        let config = LoopDetectionConfig {
            enable_content_hashing: true,
            ..Default::default()
        };
        let mut detector = LoopDetector::new(config);

        let content = b"Test clipboard content";

        // First operation from RDP should not cause loop
        assert!(!detector.check_content_loop(content, OperationSource::Rdp));

        // Same content from Portal should trigger loop
        assert!(detector.check_content_loop(content, OperationSource::Portal));

        // Different content should not trigger loop
        let different_content = b"Different content";
        assert!(!detector.check_content_loop(different_content, OperationSource::Portal));
    }

    #[test]
    fn test_content_loop_same_source() {
        let config = LoopDetectionConfig {
            enable_content_hashing: true,
            ..Default::default()
        };
        let mut detector = LoopDetector::new(config);

        let content = b"Test content";

        // First operation
        assert!(!detector.check_content_loop(content, OperationSource::Rdp));

        // Same content from same source should not trigger loop
        assert!(!detector.check_content_loop(content, OperationSource::Rdp));
    }

    #[test]
    fn test_sync_manager_rdp_formats() {
        let detector = LoopDetector::default();
        let mut manager = SyncManager::new(detector);

        let formats = vec![ClipboardFormat {
            format_id: 13,
            format_name: "CF_UNICODETEXT".to_string(),
        }];

        // Should allow first format announcement
        assert!(manager.handle_rdp_formats(formats.clone()).unwrap());

        // Verify state
        match manager.state() {
            ClipboardState::RdpOwned(f) => assert_eq!(f, &formats),
            _ => panic!("Expected RdpOwned state"),
        }
    }

    #[test]
    fn test_sync_manager_portal_formats() {
        let detector = LoopDetector::default();
        let mut manager = SyncManager::new(detector);

        let mime_types = vec!["text/plain".to_string(), "text/html".to_string()];

        // Should allow first format announcement (force=true simulates D-Bus)
        assert!(manager.handle_portal_formats(mime_types.clone(), true).unwrap());

        // Verify state
        match manager.state() {
            ClipboardState::PortalOwned(m) => assert_eq!(m, &mime_types),
            _ => panic!("Expected PortalOwned state"),
        }
    }

    #[test]
    fn test_sync_manager_loop_prevention() {
        let detector = LoopDetector::new(LoopDetectionConfig {
            window_ms: 1000,
            ..Default::default()
        });
        let mut manager = SyncManager::new(detector);

        let formats = vec![ClipboardFormat {
            format_id: 13,
            format_name: "CF_UNICODETEXT".to_string(),
        }];

        let mime_types = vec!["text/plain".to_string()];

        // First RDP announcement
        assert!(manager.handle_rdp_formats(formats.clone()).unwrap());

        // Portal announcement (force=true to override RDP ownership in test)
        assert!(manager.handle_portal_formats(mime_types, true).unwrap());

        // Second RDP announcement with same formats should be blocked
        assert!(!manager.handle_rdp_formats(formats).unwrap());
    }

    #[test]
    fn test_sync_manager_content_check() {
        let detector = LoopDetector::default();
        let mut manager = SyncManager::new(detector);

        let content = b"Test content";

        // First check from RDP should pass
        assert!(manager.check_content(content, true).unwrap());

        // Same content from Portal should fail (loop)
        assert!(!manager.check_content(content, false).unwrap());
    }

    #[test]
    fn test_sync_manager_reset() {
        let detector = LoopDetector::default();
        let mut manager = SyncManager::new(detector);

        let formats = vec![ClipboardFormat {
            format_id: 13,
            format_name: "CF_UNICODETEXT".to_string(),
        }];

        manager.handle_rdp_formats(formats).unwrap();

        // Reset state
        manager.reset();

        // Verify state is idle
        assert_eq!(manager.state(), &ClipboardState::Idle);
    }

    #[test]
    fn test_hash_formats_deterministic() {
        let detector = LoopDetector::default();

        let formats1 = vec![
            ClipboardFormat {
                format_id: 13,
                format_name: "CF_UNICODETEXT".to_string(),
            },
            ClipboardFormat {
                format_id: 8,
                format_name: "CF_DIB".to_string(),
            },
        ];

        let formats2 = vec![
            ClipboardFormat {
                format_id: 8,
                format_name: "CF_DIB".to_string(),
            },
            ClipboardFormat {
                format_id: 13,
                format_name: "CF_UNICODETEXT".to_string(),
            },
        ];

        // Hashes should be equal regardless of order
        assert_eq!(
            detector.hash_formats(&formats1),
            detector.hash_formats(&formats2)
        );
    }

    #[test]
    fn test_history_size_limit() {
        let config = LoopDetectionConfig {
            max_history: 5,
            ..Default::default()
        };
        let mut detector = LoopDetector::new(config);

        // Add more operations than max_history
        for i in 0..10 {
            let formats = vec![ClipboardFormat {
                format_id: i as u32,
                format_name: format!("Format{}", i),
            }];
            detector.record_rdp_operation(formats);
        }

        // History should be limited to max_history
        assert_eq!(detector.history_size(), 5);
    }
}
