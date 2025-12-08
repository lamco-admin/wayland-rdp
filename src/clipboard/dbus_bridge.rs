//! D-Bus Bridge for GNOME Clipboard Extension
//!
//! Connects to the wayland-rdp-clipboard GNOME Shell extension via D-Bus
//! to receive clipboard change notifications. This solves the Linux→Windows
//! clipboard direction which doesn't work via Portal because GNOME doesn't
//! emit SelectionOwnerChanged signals.
//!
//! # Architecture
//!
//! ```text
//! GNOME Shell Extension              D-Bus Session Bus              wrd-server
//! ┌──────────────────────┐           ┌─────────────────┐           ┌──────────────┐
//! │ wayland-rdp-clipboard│  signal   │                 │  subscribe│              │
//! │ St.Clipboard poll    │──────────▶│ ClipboardChanged│◀──────────│ DbusBridge   │
//! │                      │           │                 │           │              │
//! └──────────────────────┘           └─────────────────┘           └──────────────┘
//! ```
//!
//! # D-Bus Interface
//!
//! - Service: `org.wayland_rdp.Clipboard`
//! - Path: `/org/wayland_rdp/Clipboard`
//! - Interface: `org.wayland_rdp.Clipboard`
//! - Signal: `ClipboardChanged(as mime_types, s content_hash)`

use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use zbus::names::BusName;
use zbus::{Connection, MatchRule, MessageStream};
use zbus::fdo::DBusProxy;

/// D-Bus service details for the GNOME clipboard extension
const DBUS_SERVICE: &str = "org.wayland_rdp.Clipboard";
const DBUS_PATH: &str = "/org/wayland_rdp/Clipboard";
const DBUS_INTERFACE: &str = "org.wayland_rdp.Clipboard";

/// Event from the D-Bus bridge
#[derive(Debug, Clone)]
pub struct ClipboardChangedEvent {
    /// Available MIME types
    pub mime_types: Vec<String>,
    /// Content hash for deduplication
    pub content_hash: String,
    /// Whether this is from PRIMARY selection (vs CLIPBOARD)
    pub is_primary: bool,
}

/// D-Bus bridge for clipboard extension communication
pub struct DbusBridge {
    /// D-Bus connection
    connection: Option<Connection>,
    /// Whether the extension is available
    extension_available: bool,
}

impl DbusBridge {
    /// Create a new D-Bus bridge (doesn't connect yet)
    pub fn new() -> Self {
        Self {
            connection: None,
            extension_available: false,
        }
    }

    /// Check if the clipboard extension is available on D-Bus
    pub async fn check_extension_available(&mut self) -> bool {
        match Connection::session().await {
            Ok(conn) => {
                // Check if the service name is registered
                match DBusProxy::new(&conn).await {
                    Ok(dbus_proxy) => {
                        match dbus_proxy.name_has_owner(DBUS_SERVICE.try_into().unwrap()).await {
                            Ok(has_owner) => {
                                self.extension_available = has_owner;
                                if has_owner {
                                    info!("✅ GNOME clipboard extension detected on D-Bus");
                                    self.connection = Some(conn);
                                } else {
                                    warn!("GNOME clipboard extension not running (service {} not found)", DBUS_SERVICE);
                                    warn!("Linux → Windows clipboard will NOT work");
                                    warn!("Install and enable the wayland-rdp-clipboard extension");
                                }
                                has_owner
                            }
                            Err(e) => {
                                error!("Failed to check D-Bus name owner: {}", e);
                                false
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to create D-Bus proxy: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                error!("Failed to connect to D-Bus session bus: {}", e);
                false
            }
        }
    }

    /// Test connection to the extension by calling Ping
    pub async fn ping(&self) -> Result<String, zbus::Error> {
        let conn = self.connection.as_ref()
            .ok_or_else(|| zbus::Error::Failure("Not connected".into()))?;

        let bus_name: BusName = DBUS_SERVICE.try_into().unwrap();
        let interface: zbus::names::InterfaceName = DBUS_INTERFACE.try_into().unwrap();

        let reply: String = conn
            .call_method(
                Some(bus_name),
                DBUS_PATH,
                Some(interface),
                "Ping",
                &("test",),
            )
            .await?
            .body()
            .deserialize()?;

        Ok(reply)
    }

    /// Get extension version
    pub async fn get_version(&self) -> Result<String, zbus::Error> {
        let conn = self.connection.as_ref()
            .ok_or_else(|| zbus::Error::Failure("Not connected".into()))?;

        let bus_name: BusName = DBUS_SERVICE.try_into().unwrap();
        let interface: zbus::names::InterfaceName = DBUS_INTERFACE.try_into().unwrap();

        let reply: String = conn
            .call_method(
                Some(bus_name),
                DBUS_PATH,
                Some(interface),
                "GetVersion",
                &(),
            )
            .await?
            .body()
            .deserialize()?;

        Ok(reply)
    }

    /// Get current clipboard text via D-Bus (for debugging/testing)
    pub async fn get_text(&self) -> Result<String, zbus::Error> {
        let conn = self.connection.as_ref()
            .ok_or_else(|| zbus::Error::Failure("Not connected".into()))?;

        let bus_name: BusName = DBUS_SERVICE.try_into().unwrap();
        let interface: zbus::names::InterfaceName = DBUS_INTERFACE.try_into().unwrap();

        let reply: String = conn
            .call_method(
                Some(bus_name),
                DBUS_PATH,
                Some(interface),
                "GetText",
                &(),
            )
            .await?
            .body()
            .deserialize()?;

        Ok(reply)
    }

    /// Start listening for clipboard change signals
    ///
    /// Spawns a background task that subscribes to ClipboardChanged and
    /// PrimaryChanged signals and forwards them to the provided channel.
    pub async fn start_signal_listener(
        &self,
        event_tx: mpsc::UnboundedSender<ClipboardChangedEvent>,
    ) -> Result<(), zbus::Error> {
        let conn = self.connection.as_ref()
            .ok_or_else(|| zbus::Error::Failure("Not connected".into()))?
            .clone();

        // Subscribe to ClipboardChanged signal
        let clipboard_rule = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .sender(DBUS_SERVICE)?
            .path(DBUS_PATH)?
            .interface(DBUS_INTERFACE)?
            .member("ClipboardChanged")?
            .build();

        // Subscribe to PrimaryChanged signal
        let primary_rule = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .sender(DBUS_SERVICE)?
            .path(DBUS_PATH)?
            .interface(DBUS_INTERFACE)?
            .member("PrimaryChanged")?
            .build();

        // Add match rules
        let dbus_proxy = DBusProxy::new(&conn).await?;
        dbus_proxy.add_match_rule(clipboard_rule.clone()).await?;
        dbus_proxy.add_match_rule(primary_rule.clone()).await?;

        info!("Subscribed to D-Bus signals: ClipboardChanged, PrimaryChanged");

        // Clone for the spawned task
        let event_tx_clone = event_tx.clone();

        // Spawn signal listener task
        tokio::spawn(async move {
            let mut stream = MessageStream::from(&conn);

            info!("D-Bus signal listener started - waiting for clipboard changes");
            let mut signal_count = 0;

            use futures_util::StreamExt;
            while let Some(msg_result) = stream.next().await {
                match msg_result {
                    Ok(msg) => {
                        // Check if this is a signal we care about
                        let header = msg.header();

                        if header.message_type() != zbus::message::Type::Signal {
                            continue;
                        }

                        let member = match header.member() {
                            Some(m) => m.as_str(),
                            None => continue,
                        };

                        let interface = match header.interface() {
                            Some(i) => i.as_str(),
                            None => continue,
                        };

                        if interface != DBUS_INTERFACE {
                            continue;
                        }

                        let is_primary = member == "PrimaryChanged";
                        let is_clipboard = member == "ClipboardChanged";

                        if !is_primary && !is_clipboard {
                            continue;
                        }

                        signal_count += 1;

                        // Parse signal body: (as mime_types, s content_hash)
                        match msg.body().deserialize::<(Vec<String>, String)>() {
                            Ok((mime_types, content_hash)) => {
                                let selection_type = if is_primary { "PRIMARY" } else { "CLIPBOARD" };
                                info!(
                                    "📋 D-Bus {} signal #{}: {} types, hash={}",
                                    selection_type, signal_count, mime_types.len(), &content_hash[..8.min(content_hash.len())]
                                );
                                debug!("   MIME types: {:?}", mime_types);

                                let event = ClipboardChangedEvent {
                                    mime_types,
                                    content_hash,
                                    is_primary,
                                };

                                if event_tx_clone.send(event).is_err() {
                                    warn!("D-Bus signal listener stopping (receiver dropped)");
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse {} signal body: {}", member, e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("D-Bus message stream error: {}", e);
                    }
                }
            }

            warn!("D-Bus signal listener ended after {} signals", signal_count);
        });

        info!("✅ D-Bus clipboard signal listener started");
        Ok(())
    }

    /// Whether the extension is available
    pub fn is_available(&self) -> bool {
        self.extension_available
    }
}

impl Default for DbusBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DbusBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbusBridge")
            .field("extension_available", &self.extension_available)
            .field("connected", &self.connection.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires running GNOME session with extension
    async fn test_extension_detection() {
        let mut bridge = DbusBridge::new();
        let available = bridge.check_extension_available().await;
        println!("Extension available: {}", available);
    }

    #[tokio::test]
    #[ignore] // Requires running GNOME session with extension
    async fn test_ping() {
        let mut bridge = DbusBridge::new();
        if bridge.check_extension_available().await {
            match bridge.ping().await {
                Ok(reply) => println!("Ping reply: {}", reply),
                Err(e) => println!("Ping failed: {}", e),
            }
        }
    }
}
