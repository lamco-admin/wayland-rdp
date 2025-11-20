//! wl-clipboard Monitoring Backend
//!
//! Uses wl-clipboard-rs for clipboard monitoring on Wayland compositors.

use anyhow::{Context, Result};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use super::manager::ClipboardEvent;

/// Start clipboard monitoring using wl-clipboard
pub async fn start_clipboard_monitoring(
    event_tx: mpsc::Sender<ClipboardEvent>,
) -> Result<()> {
    info!("Starting wl-clipboard monitoring");

    tokio::task::spawn_blocking(move || {
        use wl_clipboard_rs::{
            paste::{get_contents, ClipboardType, Seat},
        };
        use sha2::{Digest, Sha256};

        let mut last_hash: Option<String> = None;

        loop {
            // Wait before checking
            std::thread::sleep(std::time::Duration::from_millis(500));

            // Try to get clipboard contents
            let result = get_contents(
                ClipboardType::Regular,
                Seat::Unspecified,
                wl_clipboard_rs::paste::MimeType::Text,
            );

            match result {
                Ok((mut pipe, mime)) => {
                    // Read data from pipe
                    use std::io::Read;
                    let mut data = Vec::new();
                    if pipe.read_to_end(&mut data).is_ok() && !data.is_empty() {
                        let hash = format!("{:x}", Sha256::digest(&data));

                        if Some(&hash) != last_hash.as_ref() {
                            last_hash = Some(hash);
                            info!("🎯 Clipboard changed via wl-clipboard monitoring!");

                            // Announce to RDP
                            let mime_types = vec![mime];
                            let _ = event_tx.send(ClipboardEvent::PortalFormatsAvailable(mime_types));
                        }
                    }
                }
                Ok(_) => {
                    if last_hash.is_some() {
                        last_hash = None;
                    }
                }
                Err(e) => {
                    debug!("Clipboard read error (normal if empty): {}", e);
                }
            }
        }
    });

    info!("✅ wl-clipboard monitoring started");
    Ok(())
}
