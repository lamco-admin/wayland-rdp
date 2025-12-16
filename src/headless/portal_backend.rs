//! Embedded Portal Backend for Headless Mode
//!
//! Provides a complete xdg-desktop-portal-compatible backend that runs embedded
//! within the headless compositor, automatically granting permissions without
//! requiring GUI dialogs. This is essential for headless server operation.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │  WRD Server (Headless)                                   │
//! │                                                          │
//! │  ┌────────────────────────────────────────────────────┐ │
//! │  │  Embedded Portal Backend                           │ │
//! │  │  • Implements org.freedesktop.impl.portal.*        │ │
//! │  │  • Auto-grants all permissions (headless mode)     │ │
//! │  │  • No GUI dialogs required                         │ │
//! │  │  • Policy-based permission management              │ │
//! │  └────────────┬───────────────────────────────────────┘ │
//! │               │                                          │
//! │               ▼                                          │
//! │  ┌────────────────────────────────────────────────────┐ │
//! │  │  xdg-desktop-portal (System Service)               │ │
//! │  │  • Receives requests from applications             │ │
//! │  │  • Routes to embedded backend                      │ │
//! │  └────────────┬───────────────────────────────────────┘ │
//! │               │                                          │
//! └───────────────┼──────────────────────────────────────────┘
//!                 │
//!                 ▼
//! ┌──────────────────────────────────────────────────────────┐
//! │  Applications (in compositor)                            │
//! │  • Request screen capture via portal API                 │
//! │  • Automatically granted (no dialog)                     │
//! └──────────────────────────────────────────────────────────┘
//! ```

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use zbus::{dbus_interface, ConnectionBuilder};

use crate::headless::config::{PermissionPolicyConfig, PortalConfig};

/// Permission policy for portal requests
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionPolicy {
    /// Always allow
    Allow,

    /// Always deny
    Deny,

    /// Ask user (falls back to Allow in headless mode)
    Ask,
}

impl From<&str> for PermissionPolicy {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "allow" => PermissionPolicy::Allow,
            "deny" => PermissionPolicy::Deny,
            "ask" => PermissionPolicy::Ask,
            _ => PermissionPolicy::Allow,
        }
    }
}

/// Embedded portal backend manager
pub struct EmbeddedPortalBackend {
    config: Arc<PortalConfig>,
    screencast_backend: Arc<ScreenCastBackend>,
    remote_desktop_backend: Arc<RemoteDesktopBackend>,
    _connection: zbus::Connection,
}

impl EmbeddedPortalBackend {
    /// Create and start embedded portal backend
    pub async fn new(config: Arc<PortalConfig>) -> Result<Self> {
        info!("Initializing embedded portal backend");

        // Create D-Bus connection
        let connection = match config.dbus_connection.as_str() {
            "system" => {
                zbus::Connection::system()
                    .await
                    .context("Failed to connect to system bus")?
            }
            _ => {
                zbus::Connection::session()
                    .await
                    .context("Failed to connect to session bus")?
            }
        };

        debug!("Connected to D-Bus {} bus", config.dbus_connection);

        // Create backend implementations
        let screencast_backend = Arc::new(ScreenCastBackend::new(config.clone()));
        let remote_desktop_backend = Arc::new(RemoteDesktopBackend::new(config.clone()));

        // Register D-Bus interfaces
        Self::register_interfaces(
            &connection,
            screencast_backend.clone(),
            remote_desktop_backend.clone(),
        )
        .await?;

        info!("Embedded portal backend registered on D-Bus");

        Ok(Self {
            config,
            screencast_backend,
            remote_desktop_backend,
            _connection: connection,
        })
    }

    /// Register D-Bus interfaces for portal implementation
    async fn register_interfaces(
        connection: &zbus::Connection,
        screencast: Arc<ScreenCastBackend>,
        remote_desktop: Arc<RemoteDesktopBackend>,
    ) -> Result<()> {
        // Register ScreenCast portal implementation
        connection
            .object_server()
            .at(
                "/org/freedesktop/portal/desktop",
                ScreenCastInterface::new(screencast),
            )
            .await
            .context("Failed to register ScreenCast interface")?;

        // Register RemoteDesktop portal implementation
        connection
            .object_server()
            .at(
                "/org/freedesktop/portal/desktop",
                RemoteDesktopInterface::new(remote_desktop),
            )
            .await
            .context("Failed to register RemoteDesktop interface")?;

        debug!("Portal interfaces registered");
        Ok(())
    }

    /// Check if permission should be granted
    fn check_permission(&self, request_type: &str) -> bool {
        if !self.config.auto_grant_permissions {
            return false;
        }

        let policy: PermissionPolicy = match request_type {
            "screencast" => self.config.permission_policy.screencast_policy.as_str().into(),
            "remote-desktop" => self
                .config
                .permission_policy
                .remote_desktop_policy
                .as_str()
                .into(),
            "clipboard" => self
                .config
                .permission_policy
                .clipboard_policy
                .as_str()
                .into(),
            _ => self.config.permission_policy.default_policy.as_str().into(),
        };

        match policy {
            PermissionPolicy::Allow => true,
            PermissionPolicy::Deny => false,
            PermissionPolicy::Ask => {
                // In headless mode, "Ask" becomes "Allow" since no GUI is available
                warn!(
                    "Permission policy set to 'Ask' for {} but in headless mode - defaulting to Allow",
                    request_type
                );
                true
            }
        }
    }
}

/// ScreenCast portal backend implementation
pub struct ScreenCastBackend {
    config: Arc<PortalConfig>,
    sessions: Arc<RwLock<HashMap<String, ScreenCastSession>>>,
}

/// Active screencast session
#[derive(Debug, Clone)]
struct ScreenCastSession {
    session_handle: String,
    app_id: String,
    streams: Vec<StreamInfo>,
    pipewire_node_id: u32,
}

#[derive(Debug, Clone)]
struct StreamInfo {
    node_id: u32,
    width: u32,
    height: u32,
}

impl ScreenCastBackend {
    fn new(config: Arc<PortalConfig>) -> Self {
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn create_session_impl(&self, app_id: String) -> Result<String> {
        let session_handle = format!("/session/{}", uuid::Uuid::new_v4());

        let session = ScreenCastSession {
            session_handle: session_handle.clone(),
            app_id,
            streams: Vec::new(),
            pipewire_node_id: 0, // Will be set when stream starts
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_handle.clone(), session);

        info!("Created ScreenCast session: {}", session_handle);
        Ok(session_handle)
    }

    async fn select_sources_impl(
        &self,
        session_handle: String,
        _source_types: u32,
    ) -> Result<()> {
        debug!("Selecting sources for session: {}", session_handle);

        // In headless mode, we automatically select available streams
        // This would typically show a dialog, but we auto-approve

        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_handle) {
            // Add default stream (virtual display)
            session.streams.push(StreamInfo {
                node_id: 1, // Will be actual PipeWire node ID
                width: 1920,
                height: 1080,
            });

            info!("Auto-selected sources for session: {}", session_handle);
        }

        Ok(())
    }

    async fn start_impl(&self, session_handle: String) -> Result<u32> {
        info!("Starting ScreenCast session: {}", session_handle);

        let sessions = self.sessions.read().await;
        let session = sessions
            .get(&session_handle)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        // Return PipeWire node ID
        // In real implementation, this would create actual PipeWire stream
        let node_id = session.pipewire_node_id;

        info!("ScreenCast session started with PipeWire node: {}", node_id);
        Ok(node_id)
    }
}

/// D-Bus interface for ScreenCast portal
struct ScreenCastInterface {
    backend: Arc<ScreenCastBackend>,
}

impl ScreenCastInterface {
    fn new(backend: Arc<ScreenCastBackend>) -> Self {
        Self { backend }
    }
}

#[dbus_interface(name = "org.freedesktop.impl.portal.ScreenCast")]
impl ScreenCastInterface {
    async fn create_session(
        &self,
        _handle: zbus::zvariant::OwnedObjectPath,
        _session_handle: zbus::zvariant::OwnedObjectPath,
        app_id: String,
        _options: HashMap<String, zbus::zvariant::OwnedValue>,
    ) -> zbus::fdo::Result<u32> {
        self.backend
            .create_session_impl(app_id)
            .await
            .map(|_| 0) // 0 = success
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn select_sources(
        &self,
        _handle: zbus::zvariant::OwnedObjectPath,
        session_handle: zbus::zvariant::OwnedObjectPath,
        _options: HashMap<String, zbus::zvariant::OwnedValue>,
    ) -> zbus::fdo::Result<u32> {
        self.backend
            .select_sources_impl(session_handle.to_string(), 1)
            .await
            .map(|_| 0)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn start(
        &self,
        _handle: zbus::zvariant::OwnedObjectPath,
        session_handle: zbus::zvariant::OwnedObjectPath,
        _parent_window: String,
        _options: HashMap<String, zbus::zvariant::OwnedValue>,
    ) -> zbus::fdo::Result<u32> {
        self.backend
            .start_impl(session_handle.to_string())
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }
}

/// RemoteDesktop portal backend implementation
pub struct RemoteDesktopBackend {
    config: Arc<PortalConfig>,
    sessions: Arc<RwLock<HashMap<String, RemoteDesktopSession>>>,
}

/// Active remote desktop session
#[derive(Debug, Clone)]
struct RemoteDesktopSession {
    session_handle: String,
    app_id: String,
    devices: u32, // Bitmask of allowed devices
}

impl RemoteDesktopBackend {
    fn new(config: Arc<PortalConfig>) -> Self {
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn create_session_impl(&self, app_id: String) -> Result<String> {
        let session_handle = format!("/session/{}", uuid::Uuid::new_v4());

        let session = RemoteDesktopSession {
            session_handle: session_handle.clone(),
            app_id,
            devices: 0,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_handle.clone(), session);

        info!("Created RemoteDesktop session: {}", session_handle);
        Ok(session_handle)
    }

    async fn select_devices_impl(&self, session_handle: String, devices: u32) -> Result<()> {
        debug!(
            "Selecting devices for session: {} (devices: {})",
            session_handle, devices
        );

        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_handle) {
            session.devices = devices;
            info!(
                "Auto-granted devices for session: {} (devices: {})",
                session_handle, devices
            );
        }

        Ok(())
    }

    async fn start_impl(&self, session_handle: String) -> Result<()> {
        info!("Starting RemoteDesktop session: {}", session_handle);
        // Session is now active
        Ok(())
    }

    async fn notify_pointer_motion(&self, _session_handle: String, _dx: f64, _dy: f64) -> Result<()> {
        // Forward to actual input injection
        Ok(())
    }

    async fn notify_pointer_button(&self, _session_handle: String, _button: i32, _state: u32) -> Result<()> {
        // Forward to actual input injection
        Ok(())
    }

    async fn notify_keyboard_keycode(&self, _session_handle: String, _keycode: i32, _state: u32) -> Result<()> {
        // Forward to actual input injection
        Ok(())
    }
}

/// D-Bus interface for RemoteDesktop portal
struct RemoteDesktopInterface {
    backend: Arc<RemoteDesktopBackend>,
}

impl RemoteDesktopInterface {
    fn new(backend: Arc<RemoteDesktopBackend>) -> Self {
        Self { backend }
    }
}

#[dbus_interface(name = "org.freedesktop.impl.portal.RemoteDesktop")]
impl RemoteDesktopInterface {
    async fn create_session(
        &self,
        _handle: zbus::zvariant::OwnedObjectPath,
        _session_handle: zbus::zvariant::OwnedObjectPath,
        app_id: String,
        _options: HashMap<String, zbus::zvariant::OwnedValue>,
    ) -> zbus::fdo::Result<u32> {
        self.backend
            .create_session_impl(app_id)
            .await
            .map(|_| 0)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn select_devices(
        &self,
        _handle: zbus::zvariant::OwnedObjectPath,
        session_handle: zbus::zvariant::OwnedObjectPath,
        options: HashMap<String, zbus::zvariant::OwnedValue>,
    ) -> zbus::fdo::Result<u32> {
        let devices = options
            .get("types")
            .and_then(|v| v.downcast_ref::<u32>())
            .copied()
            .unwrap_or(3); // Default: keyboard + pointer

        self.backend
            .select_devices_impl(session_handle.to_string(), devices)
            .await
            .map(|_| 0)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn start(
        &self,
        _handle: zbus::zvariant::OwnedObjectPath,
        session_handle: zbus::zvariant::OwnedObjectPath,
        _parent_window: String,
        _options: HashMap<String, zbus::zvariant::OwnedValue>,
    ) -> zbus::fdo::Result<u32> {
        self.backend
            .start_impl(session_handle.to_string())
            .await
            .map(|_| 0)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn notify_pointer_motion(
        &self,
        session_handle: zbus::zvariant::OwnedObjectPath,
        _options: HashMap<String, zbus::zvariant::OwnedValue>,
        dx: f64,
        dy: f64,
    ) -> zbus::fdo::Result<()> {
        self.backend
            .notify_pointer_motion(session_handle.to_string(), dx, dy)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn notify_pointer_button(
        &self,
        session_handle: zbus::zvariant::OwnedObjectPath,
        _options: HashMap<String, zbus::zvariant::OwnedValue>,
        button: i32,
        state: u32,
    ) -> zbus::fdo::Result<()> {
        self.backend
            .notify_pointer_button(session_handle.to_string(), button, state)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn notify_keyboard_keycode(
        &self,
        session_handle: zbus::zvariant::OwnedObjectPath,
        _options: HashMap<String, zbus::zvariant::OwnedValue>,
        keycode: i32,
        state: u32,
    ) -> zbus::fdo::Result<()> {
        self.backend
            .notify_keyboard_keycode(session_handle.to_string(), keycode, state)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_policy_from_str() {
        assert_eq!(PermissionPolicy::from("allow"), PermissionPolicy::Allow);
        assert_eq!(PermissionPolicy::from("deny"), PermissionPolicy::Deny);
        assert_eq!(PermissionPolicy::from("ask"), PermissionPolicy::Ask);
        assert_eq!(PermissionPolicy::from("unknown"), PermissionPolicy::Allow);
    }
}
