//! Clipboard portal integration
//!
//! Provides complete clipboard access using wl-clipboard-rs for Wayland
//! and fallback mechanisms for X11/other environments.

use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use wl_clipboard_rs::copy::{MimeType, Options as CopyOptions, Source};
use wl_clipboard_rs::paste::{get_contents, ClipboardType, MimeType as PasteMimeType, Seat};

use crate::config::Config;

/// Clipboard portal manager
#[derive(Debug)]
pub struct ClipboardManager {
    connection: zbus::Connection,
    config: Arc<Config>,
}

impl ClipboardManager {
    /// Create new Clipboard manager
    pub async fn new(connection: zbus::Connection, config: Arc<Config>) -> Result<Self> {
        info!("Initializing Clipboard portal manager");
        Ok(Self { connection, config })
    }

    /// Read clipboard content
    ///
    /// Reads data from the system clipboard in the specified MIME type using
    /// wl-clipboard-rs for direct Wayland clipboard access
    pub async fn read_clipboard(
        &self,
        mime_type: &str,
    ) -> Result<Vec<u8>> {
        debug!("Reading clipboard content for MIME type: {}", mime_type);

        // Clone mime_type for the blocking task
        let mime_str = mime_type.to_string();

        // Use wl-clipboard-rs for direct Wayland clipboard access
        let result = tokio::task::spawn_blocking(move || {
            let mime = PasteMimeType::Specific(&mime_str);
            get_contents(ClipboardType::Regular, Seat::Unspecified, mime)
        })
        .await
        .context("Failed to spawn clipboard read task")?;

        match result {
            Ok((mut pipe_reader, _actual_mime)) => {
                // Read all data from the pipe
                use std::io::Read;
                let mut data = Vec::new();
                pipe_reader.read_to_end(&mut data)
                    .context("Failed to read clipboard pipe")?;

                debug!("Read {} bytes from clipboard ({})", data.len(), mime_type);
                Ok(data)
            }
            Err(e) => {
                // Clipboard might be empty or MIME type not available
                debug!("Failed to read clipboard ({}): {} - returning empty", mime_type, e);
                Ok(Vec::new())
            }
        }
    }

    /// Write clipboard content
    ///
    /// Sets data to the system clipboard with the specified MIME type using
    /// wl-clipboard-rs for direct Wayland clipboard access
    pub async fn write_clipboard(
        &self,
        mime_type: &str,
        data: &[u8],
    ) -> Result<()> {
        debug!(
            "Writing clipboard content: {} ({} bytes)",
            mime_type,
            data.len()
        );

        // Validate size
        if data.len() > self.config.clipboard.max_size {
            anyhow::bail!("Clipboard data exceeds maximum size: {} > {}", data.len(), self.config.clipboard.max_size);
        }

        // Clone data for the blocking task
        let data = data.to_vec();
        let mime_str = mime_type.to_string();
        let mime_log = mime_type.to_string(); // For logging after move

        // Use wl-clipboard-rs to set clipboard
        tokio::task::spawn_blocking(move || {
            let opts = CopyOptions::new();
            let mime = MimeType::Specific(mime_str);
            let source = Source::Bytes(data.into());

            opts.copy(source, mime)
        })
        .await
        .context("Failed to spawn clipboard write task")?
        .context("Failed to write to clipboard")?;

        debug!("Clipboard write successful: {}", mime_log);
        Ok(())
    }

    /// Request clipboard content (legacy method for compatibility)
    ///
    /// Note: This requires a session handle from SelectionWrite/SelectionRead
    pub async fn request_clipboard(
        &self,
        session_handle: &str,
        mime_type: &str,
    ) -> Result<Vec<u8>> {
        debug!("Requesting clipboard content with session {}: {}", session_handle, mime_type);
        self.read_clipboard(mime_type).await
    }

    /// Set clipboard content (legacy method for compatibility)
    pub async fn set_clipboard(
        &self,
        session_handle: &str,
        mime_type: &str,
        data: &[u8],
    ) -> Result<()> {
        debug!("Setting clipboard content with session {}", session_handle);
        self.write_clipboard(mime_type, data).await
    }

    /// Announce available clipboard formats
    ///
    /// For Wayland clipboard, format announcement happens implicitly when data is set.
    /// This method validates the formats and prepares for clipboard operations.
    pub async fn advertise_formats(&self, mime_types: &[String]) -> Result<()> {
        debug!("Available clipboard formats: {:?}", mime_types);

        // In Wayland clipboard protocol, format announcement happens when we set clipboard content
        // The compositor queries available MIME types when an application requests paste
        // So we just validate the formats here and log for monitoring

        if mime_types.is_empty() {
            warn!("No clipboard formats available");
            return Ok(());
        }

        info!("Clipboard ready with {} format(s): {}", mime_types.len(), mime_types.join(", "));

        // Validate each MIME type is well-formed
        for mime in mime_types {
            if !mime.contains('/') {
                warn!("Invalid MIME type format: {}", mime);
            }
        }

        Ok(())
    }
}
