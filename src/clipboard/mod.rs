//! Clipboard Synchronization Module
//!
//! Provides complete bidirectional clipboard synchronization between RDP
//! clients and Wayland compositors with production-grade loop prevention,
//! format conversion, and efficient data transfer.
//!
//! # Architecture
//!
//! This module uses the lamco crate ecosystem for clipboard primitives:
//!
//! - [`lamco_clipboard_core`] - Format conversion, loop detection, transfer engine
//! - [`lamco_portal::dbus_clipboard`] - D-Bus GNOME clipboard bridge
//! - [`lamco_rdp_clipboard`] - IronRDP clipboard backend
//!
//! The server adds:
//!
//! - [`SyncManager`] - State machine orchestration (server-specific policy)
//! - [`ClipboardManager`] - Event routing between Portal and RDP
//! - [`WrdCliprdrFactory`] - Server-specific backend factory wrapper
//!
//! # Data Flow
//!
//! ```text
//! RDP Client                IronRDP               WRD                Portal            Wayland
//! ━━━━━━━━━━                ━━━━━━━               ━━━               ━━━━━━            ━━━━━━━
//!
//! Copy (Ctrl+C)
//!   └─> Format List ────> RdpCliprdrBackend ─> ClipboardManager
//!                                                    │
//!                                                    ├─> SyncManager (state)
//!                                                    ├─> LoopDetector (library)
//!                                                    ├─> FormatConverter (library)
//!                                                    │
//!                                                    └─────────────> Clipboard API
//!
//! Paste (Ctrl+V) <── Data Response <── Convert <── Get Data <──── DbusClipboardBridge
//! ```
//!
//! # Features
//!
//! - **Bidirectional Sync**: RDP ↔ Wayland clipboard sharing
//! - **Format Conversion**: Text (UTF-8/UTF-16/HTML/RTF), Images (PNG/JPEG/BMP/DIB), Files
//! - **Loop Prevention**: Content hashing + state machine + echo protection
//! - **Chunked Transfer**: Large data (>1MB) with progress tracking
//! - **Error Recovery**: Policy-based retry and fallback strategies

// Server-specific modules (policy and orchestration)
pub mod error;
pub mod ironrdp_backend;
pub mod manager;
pub mod sync;

// =============================================================================
// Re-export from lamco crates (library primitives)
// =============================================================================

// Format conversion and clipboard types from lamco-clipboard-core
pub use lamco_clipboard_core::{
    ClipboardFormat, FormatConverter,
    // Loop detection
    LoopDetector, LoopDetectionConfig,
    // Transfer engine
    TransferConfig, TransferEngine, TransferProgress, TransferState,
    // Error types (base)
    ClipboardError as CoreClipboardError,
};

// Standalone format mapping functions from library
pub use lamco_clipboard_core::formats::{
    mime_to_rdp_formats as lib_mime_to_rdp_formats,
    rdp_format_to_mime as lib_rdp_format_to_mime,
};

// D-Bus bridge from lamco-portal (GNOME fallback)
pub use lamco_portal::dbus_clipboard::{
    DbusClipboardBridge, DbusClipboardEvent,
};

// RDP backend from lamco-rdp-clipboard
pub use lamco_rdp_clipboard::{
    RdpCliprdrBackend, RdpCliprdrFactory as LibRdpCliprdrFactory,
    ClipboardEvent as RdpClipboardEvent,
    ClipboardEventReceiver, ClipboardEventSender,
    ClipboardGeneralCapabilityFlags,
};

// =============================================================================
// Re-export server-specific types
// =============================================================================

// Server error types (extends library errors with recovery policy)
pub use error::{ClipboardError, ErrorContext, ErrorType, RecoveryAction, Result, RetryConfig};

// Server IronRDP factory (wraps library factory)
pub use ironrdp_backend::WrdCliprdrFactory;

// Server clipboard manager
pub use manager::{ClipboardConfig, ClipboardEvent, ClipboardManager};

// Server sync manager (state machine + echo protection)
pub use sync::{ClipboardState, SyncDirection, SyncManager};

// =============================================================================
// Extension trait for FormatConverter (server-specific convenience methods)
// =============================================================================

/// Extension trait adding convenience methods to FormatConverter
///
/// The library provides standalone functions for format mapping.
/// This trait wraps them as methods for ergonomic use in the server.
pub trait FormatConverterExt {
    /// Convert RDP formats to MIME types
    fn rdp_to_mime_types(&self, formats: &[ClipboardFormat]) -> error::Result<Vec<String>>;

    /// Convert MIME types to RDP formats
    fn mime_to_rdp_formats(&self, mime_types: &[String]) -> error::Result<Vec<RdpFormat>>;

    /// Convert a single format ID to MIME type
    fn format_id_to_mime(&self, format_id: u32) -> error::Result<String>;

    /// Convert a single MIME type to format ID
    fn mime_to_format_id(&self, mime_type: &str) -> error::Result<u32>;
}

/// RDP format with ID and name (for server compatibility)
#[derive(Debug, Clone)]
pub struct RdpFormat {
    /// Format ID
    pub format_id: u32,
    /// Format name (empty string if no name)
    pub format_name: String,
}

impl FormatConverterExt for FormatConverter {
    fn rdp_to_mime_types(&self, formats: &[ClipboardFormat]) -> error::Result<Vec<String>> {
        let mut mime_types = Vec::new();
        for format in formats {
            if let Some(mime) = lib_rdp_format_to_mime(format.id) {
                if !mime_types.contains(&mime.to_string()) {
                    mime_types.push(mime.to_string());
                }
            }
        }
        if mime_types.is_empty() {
            // Fall back to text/plain if no mapping found
            mime_types.push("text/plain".to_string());
        }
        Ok(mime_types)
    }

    fn mime_to_rdp_formats(&self, mime_types: &[String]) -> error::Result<Vec<RdpFormat>> {
        let strs: Vec<&str> = mime_types.iter().map(|s| s.as_str()).collect();
        let formats = lib_mime_to_rdp_formats(&strs);
        Ok(formats
            .into_iter()
            .map(|f| RdpFormat {
                format_id: f.id,
                format_name: f.name.unwrap_or_default(),
            })
            .collect())
    }

    fn format_id_to_mime(&self, format_id: u32) -> error::Result<String> {
        lib_rdp_format_to_mime(format_id)
            .map(|s| s.to_string())
            .ok_or_else(|| {
                error::ClipboardError::Core(CoreClipboardError::UnsupportedFormat(format!(
                    "No MIME type for format ID {}",
                    format_id
                )))
            })
    }

    fn mime_to_format_id(&self, mime_type: &str) -> error::Result<u32> {
        let formats = lib_mime_to_rdp_formats(&[mime_type]);
        formats.first().map(|f| f.id).ok_or_else(|| {
            error::ClipboardError::Core(CoreClipboardError::UnsupportedFormat(
                mime_type.to_string(),
            ))
        })
    }
}
