//! Clipboard Synchronization Module
//!
//! Provides complete bidirectional clipboard synchronization between RDP
//! clients and Wayland compositors. Supports text, images, and file formats
//! with format conversion, loop prevention, and chunked transfers.
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
pub mod manager;
pub mod sync;
pub mod transfer;

// Re-export main types
pub use error::{ClipboardError, ErrorContext, ErrorType, RecoveryAction, Result};
pub use formats::{ClipboardFormat, FormatConverter};
pub use manager::{ClipboardConfig, ClipboardEvent, ClipboardManager};
pub use sync::{ClipboardState, LoopDetectionConfig, LoopDetector, SyncDirection, SyncManager};
pub use transfer::{
    TransferConfig, TransferEngine, TransferHandle, TransferProgress, TransferState,
};
