//! Portal + Token Strategy Implementation
//!
//! Uses XDG Portal with restore tokens for session persistence.
//! This is the universal strategy that works across all desktop environments.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::portal::PortalManager;
use crate::services::ServiceRegistry;
use crate::session::strategy::{
    PipeWireAccess, SessionHandle, SessionStrategy, SessionType, StreamInfo,
};
use crate::session::TokenManager;

/// Portal session handle implementation
pub struct PortalSessionHandleImpl {
    /// PipeWire file descriptor
    pipewire_fd: i32,
    /// Stream information
    streams: Vec<StreamInfo>,
    /// Remote desktop manager (for input injection)
    remote_desktop: Arc<lamco_portal::RemoteDesktopManager>,
    /// Session for input injection and clipboard
    session: Arc<tokio::sync::Mutex<ashpd::desktop::Session<'static, ashpd::desktop::remote_desktop::RemoteDesktop<'static>>>>,
    /// Clipboard manager (for clipboard operations)
    clipboard_manager: Arc<lamco_portal::ClipboardManager>,
    /// Session type
    session_type: SessionType,
}

impl PortalSessionHandleImpl {
    /// Create from existing Portal handle and session components (for hybrid Mutter strategy)
    pub fn from_portal_session(
        session: Arc<tokio::sync::Mutex<ashpd::desktop::Session<'static, ashpd::desktop::remote_desktop::RemoteDesktop<'static>>>>,
        remote_desktop: Arc<lamco_portal::RemoteDesktopManager>,
        clipboard_manager: Arc<lamco_portal::ClipboardManager>,
    ) -> Self {
        // Input-only handle - doesn't provide video/clipboard
        Self {
            pipewire_fd: 0,  // Not used for input-only
            streams: vec![],  // Not used for input-only
            remote_desktop,
            session,
            clipboard_manager,
            session_type: SessionType::Portal,
        }
    }
}

#[async_trait]
impl SessionHandle for PortalSessionHandleImpl {
    fn pipewire_access(&self) -> PipeWireAccess {
        PipeWireAccess::FileDescriptor(self.pipewire_fd)
    }

    fn streams(&self) -> Vec<StreamInfo> {
        self.streams.clone()
    }

    fn session_type(&self) -> SessionType {
        self.session_type
    }

    async fn notify_keyboard_keycode(&self, keycode: i32, pressed: bool) -> Result<()> {
        let session = self.session.lock().await;
        self.remote_desktop
            .notify_keyboard_keycode(&session, keycode, pressed)
            .await
            .context("Failed to inject keyboard keycode via Portal")
    }

    async fn notify_pointer_motion_absolute(&self, stream_id: u32, x: f64, y: f64) -> Result<()> {
        let session = self.session.lock().await;
        self.remote_desktop
            .notify_pointer_motion_absolute(&session, stream_id, x, y)
            .await
            .context("Failed to inject pointer motion via Portal")
    }

    async fn notify_pointer_button(&self, button: i32, pressed: bool) -> Result<()> {
        let session = self.session.lock().await;
        self.remote_desktop
            .notify_pointer_button(&session, button, pressed)
            .await
            .context("Failed to inject pointer button via Portal")
    }

    async fn notify_pointer_axis(&self, dx: f64, dy: f64) -> Result<()> {
        let session = self.session.lock().await;
        self.remote_desktop
            .notify_pointer_axis(&session, dx, dy)
            .await
            .context("Failed to inject pointer axis via Portal")
    }

    fn portal_clipboard(&self) -> Option<crate::session::strategy::ClipboardComponents> {
        // Portal strategy shares its session with clipboard
        Some(crate::session::strategy::ClipboardComponents {
            manager: Arc::clone(&self.clipboard_manager),
            session: Arc::clone(&self.session),
        })
    }
}

/// Portal + Token strategy
///
/// This strategy uses the XDG Portal with restore tokens for session persistence.
/// Works across all desktop environments with portal v4+.
pub struct PortalTokenStrategy {
    service_registry: Arc<ServiceRegistry>,
    token_manager: Arc<TokenManager>,
}

impl PortalTokenStrategy {
    /// Create a new Portal + Token strategy
    ///
    /// # Arguments
    ///
    /// * `service_registry` - For checking capabilities
    /// * `token_manager` - For loading/saving tokens
    pub fn new(service_registry: Arc<ServiceRegistry>, token_manager: Arc<TokenManager>) -> Self {
        Self {
            service_registry,
            token_manager,
        }
    }
}

#[async_trait]
impl SessionStrategy for PortalTokenStrategy {
    fn name(&self) -> &'static str {
        "Portal + Restore Token"
    }

    fn requires_initial_setup(&self) -> bool {
        // First time requires dialog, but subsequent runs use token
        true
    }

    fn supports_unattended_restore(&self) -> bool {
        // If portal v4+ and we have storage, yes
        self.service_registry.supports_session_persistence()
    }

    async fn create_session(&self) -> Result<Arc<dyn SessionHandle>> {
        info!("Creating session using Portal + Token strategy");

        // Load existing token (may be None on first run)
        let restore_token = self
            .token_manager
            .load_token("default")
            .await
            .context("Failed to load restore token")?;

        if let Some(ref token) = restore_token {
            info!(
                "Loaded restore token ({} chars), will attempt restoration",
                token.len()
            );
        } else {
            info!("No restore token found, permission dialog will appear");
        }

        // Configure portal with token
        let mut portal_config = lamco_portal::PortalConfig::default();
        portal_config.restore_token = restore_token.clone();

        // Some portals reject persistence for RemoteDesktop sessions
        // Start with ExplicitlyRevoked (default), but if that fails, we'll retry with DoNot
        // persist_mode is already ExplicitlyRevoked in default

        debug!("Portal config: persist_mode={:?}, has_token={}",
               portal_config.persist_mode,
               portal_config.restore_token.is_some());

        // Create portal manager
        let portal_manager = Arc::new(
            PortalManager::new(portal_config)
                .await
                .context("Failed to create Portal manager")?,
        );

        // Create session (may or may not show dialog depending on token validity)
        let session_id = format!("lamco-rdp-{}", uuid::Uuid::new_v4());

        // Try to create session - if persistence is rejected, retry without it
        // We need to track if we used a pre-created clipboard manager (for retry case)
        let (portal_handle, new_token, pre_created_clipboard_mgr) = match portal_manager
            .create_session(session_id.clone(), None)
            .await
        {
            Ok(result) => (result.0, result.1, None),
            Err(e) => {
                let error_msg = format!("{:#}", e);

                // Check if error is about persistence rejection
                if error_msg.contains("cannot persist") || error_msg.contains("InvalidArgument") {
                    warn!("Portal rejected persistence request, retrying without persistence");
                    warn!("Note: Session will not persist across restarts");

                    // Create new portal manager without persistence
                    let mut no_persist_config = lamco_portal::PortalConfig::default();
                    no_persist_config.persist_mode = ashpd::desktop::PersistMode::DoNot;
                    no_persist_config.restore_token = None;

                    let no_persist_manager = Arc::new(
                        PortalManager::new(no_persist_config)
                            .await
                            .context("Failed to create Portal manager without persistence")?,
                    );

                    // CRITICAL: Create clipboard manager and KEEP it
                    // This manager gets enabled in the session and must be the one we use
                    let clipboard_mgr = Arc::new(
                        lamco_portal::ClipboardManager::new()
                            .await
                            .context("Failed to create clipboard manager for non-persistent session")?,
                    );

                    let result = no_persist_manager
                        .create_session(session_id.clone(), Some(clipboard_mgr.as_ref()))
                        .await
                        .context("Failed to create portal session (non-persistent)")?;

                    (result.0, result.1, Some(clipboard_mgr))
                } else {
                    // Different error, propagate it
                    return Err(e).context("Failed to create portal session");
                }
            }
        };

        // Save new token if received
        if let Some(ref token) = new_token {
            info!("Received new restore token from portal, saving...");
            self.token_manager
                .save_token("default", token)
                .await
                .context("Failed to save new restore token")?;
            info!("✅ New restore token saved successfully");
        } else if restore_token.is_some() {
            info!("No new token returned (existing token may have been used successfully)");
        } else {
            warn!("⚠️  Portal did not return restore token (portal v3 or below?)");
        }

        // Extract fields from portal_handle
        let pipewire_fd = portal_handle.pipewire_fd();
        let portal_streams = portal_handle.streams();
        let streams: Vec<StreamInfo> = portal_streams
            .iter()
            .map(|s| StreamInfo {
                node_id: s.node_id,
                width: s.size.0,
                height: s.size.1,
                position_x: s.position.0,
                position_y: s.position.1,
            })
            .collect();

        // Move session out of portal_handle
        let session = portal_handle.session;

        // Use the clipboard manager from non-persistent retry if it exists,
        // otherwise create a new one for normal sessions
        let clipboard_manager = if let Some(clipboard_mgr) = pre_created_clipboard_mgr {
            info!("Using clipboard manager from non-persistent session retry");
            clipboard_mgr
        } else {
            let mgr = Arc::new(
                lamco_portal::ClipboardManager::new()
                    .await
                    .context("Failed to create Portal clipboard manager")?,
            );
            info!("Portal clipboard manager created for session");
            mgr
        };

        // Wrap in our handle type with input injection and clipboard support
        let handle = PortalSessionHandleImpl {
            pipewire_fd,
            streams,
            remote_desktop: portal_manager.remote_desktop().clone(),
            session: Arc::new(tokio::sync::Mutex::new(session)),
            clipboard_manager,
            session_type: SessionType::Portal,
        };

        info!("Portal session created successfully with input injection and clipboard support");

        Ok(Arc::new(handle))
    }

    async fn cleanup(&self, _session: &dyn SessionHandle) -> Result<()> {
        // Portal sessions clean up automatically when dropped
        debug!("Portal session cleanup (automatic via Drop)");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Wayland session with portal
    async fn test_portal_token_strategy() {
        // Would require full environment
        // Tested via integration tests
    }
}
