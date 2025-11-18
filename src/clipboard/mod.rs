//! Clipboard Synchronization Module
//!
//! Provides complete bidirectional clipboard synchronization between RDP
//! clients and Wayland compositors with production-grade loop prevention,
//! format conversion, and efficient data transfer.
//!
//! # Overview
//!
//! This module implements the CLIPRDR (Clipboard Virtual Channel Redirection) protocol
//! integration with IronRDP and Portal clipboard APIs, enabling seamless clipboard
//! sharing between Windows RDP clients and Linux Wayland sessions.
//!
//! # Architecture
//!
//! ```text
//! RDP Client                IronRDP               WRD                Portal            Wayland
//! ━━━━━━━━━━                ━━━━━━━               ━━━               ━━━━━━            ━━━━━━━
//!
//! Copy (Ctrl+C)
//!   └─> Format List ────> CliprdrServer ────> Backend ────> Manager
//!                                                               │
//!                                                               ├─> Loop Detection
//!                                                               ├─> Format Conversion
//!                                                               │     (CF_UNICODETEXT → text/plain)
//!                                                               │
//!                                                               └────────────────> Clipboard API
//!                                                                                      └─> Set Data
//!
//! Paste (Ctrl+V) <────── Data Response <──── Format Convert <──── Get Data <──────── Clipboard API
//! ```
//!
//! # Features
//!
//! - **Bidirectional Sync:** RDP ↔ Wayland clipboard sharing
//! - **Format Conversion:**
//!   - Text: UTF-8 ↔ UTF-16, plain ↔ HTML ↔ RTF
//!   - Images: PNG ↔ JPEG ↔ BMP ↔ DIB
//!   - Files: text/uri-list ↔ CF_HDROP
//! - **Loop Prevention:** Sophisticated detection to prevent infinite sync loops
//! - **Chunked Transfer:** Large data (>1MB) transferred efficiently
//! - **Error Recovery:** Comprehensive error handling and retry logic
//!
//! # Loop Prevention
//!
//! Clipboard sync can create loops where changes bounce between RDP and Portal infinitely:
//!
//! ```text
//! RDP sets clipboard → Portal gets it → Portal sets clipboard → RDP gets it → LOOP!
//! ```
//!
//! This module prevents loops using:
//!
//! 1. **Operation History:** Track recent clipboard operations with timestamps
//! 2. **Content Hashing:** Compare content hashes to detect duplicates
//! 3. **State Machine:** Track ownership (Idle, RdpOwned, PortalOwned)
//! 4. **Time Windows:** Ignore operations within 500ms window
//!
//! # IronRDP Integration
//!
//! The [`WrdCliprdrFactory`] implements IronRDP's clipboard backend traits:
//!
//! - `CliprdrBackendFactory` - Creates backend instances per connection
//! - `CliprdrBackend` - Handles clipboard protocol events
//! - `ServerEventSender` - Receives server events
//!
//! # Example
//!
//! ```no_run
//! use wrd_server::clipboard::{ClipboardConfig, ClipboardManager, WrdCliprdrFactory};
//! use std::sync::Arc;
//! use tokio::sync::Mutex;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create clipboard manager
//! let config = ClipboardConfig::default();
//! let manager = Arc::new(Mutex::new(
//!     ClipboardManager::new(config).await?
//! ));
//!
//! // Create IronRDP factory
//! let factory = WrdCliprdrFactory::new(manager);
//!
//! // Factory is passed to IronRDP server builder
//! // It creates backend instances for each RDP connection
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! - **Bidirectional Sync**: RDP ↔ Wayland clipboard sharing
//! - **Format Conversion**: Text (UTF-8/UTF-16/HTML/RTF), Images (PNG/JPEG/BMP/DIB), Files
//! - **Loop Prevention**: Sophisticated detection to prevent sync loops
//! - **Chunked Transfer**: Large data handled efficiently with progress tracking
//! - **Error Recovery**: Comprehensive error handling and recovery strategies
//!
//! # Example
//!
//! ```no_run
//! use wrd_server::clipboard::{ClipboardManager, ClipboardConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ClipboardConfig::default();
//! let manager = ClipboardManager::new(config).await?;
//!
//! // Manager will handle clipboard events automatically
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod formats;
pub mod ironrdp_backend;
pub mod manager;
pub mod sync;
pub mod transfer;

// Re-export main types
pub use error::{ClipboardError, ErrorContext, ErrorType, RecoveryAction, Result};
pub use formats::{ClipboardFormat, FormatConverter};
pub use ironrdp_backend::WrdCliprdrFactory;
pub use manager::{ClipboardConfig, ClipboardEvent, ClipboardManager};
pub use sync::{ClipboardState, LoopDetectionConfig, LoopDetector, SyncDirection, SyncManager};
pub use transfer::{
    TransferConfig, TransferEngine, TransferHandle, TransferProgress, TransferState,
};
