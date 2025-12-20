//! IronRDP Clipboard Backend Factory
//!
//! Server-specific factory wrapping lamco-rdp-clipboard's backend.
//! Integrates with the server's ClipboardManager for event routing.

use ironrdp_cliprdr::backend::CliprdrBackendFactory;
use ironrdp_server::ServerEventSender;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info};

// Re-export library backend and types
pub use lamco_rdp_clipboard::{
    RdpCliprdrBackend, RdpCliprdrFactory as LibRdpCliprdrFactory,
    ClipboardEvent, ClipboardEventReceiver, ClipboardEventSender,
    ClipboardGeneralCapabilityFlags,
};

use crate::clipboard::manager::ClipboardManager;

/// Server-specific clipboard backend factory
///
/// Wraps [`LibRdpCliprdrFactory`] from lamco-rdp-clipboard and integrates
/// with the server's [`ClipboardManager`] for event routing.
///
/// # Example
///
/// ```ignore
/// use lamco_rdp_server::clipboard::{ClipboardManager, WrdCliprdrFactory};
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
///
/// let manager = Arc::new(Mutex::new(ClipboardManager::new(config).await?));
/// let factory = WrdCliprdrFactory::new(manager);
///
/// // Pass factory to IronRDP server builder
/// ```
pub struct WrdCliprdrFactory {
    /// Clipboard manager shared across connections
    clipboard_manager: Arc<Mutex<ClipboardManager>>,

    /// Event sender for clipboard events
    event_sender: ClipboardEventSender,

    /// Event receiver (kept to pass to manager)
    event_receiver: Option<ClipboardEventReceiver>,

    /// Server event sender for IronRDP (set via ServerEventSender trait)
    server_event_sender: Option<mpsc::UnboundedSender<ironrdp_server::ServerEvent>>,
}

impl WrdCliprdrFactory {
    /// Create a new clipboard factory
    ///
    /// # Arguments
    ///
    /// * `clipboard_manager` - Shared clipboard manager instance
    pub fn new(clipboard_manager: Arc<Mutex<ClipboardManager>>) -> Self {
        let event_sender = ClipboardEventSender::new();
        let event_receiver = Some(event_sender.subscribe());

        info!("Created WrdCliprdrFactory with event channel");

        Self {
            clipboard_manager,
            event_sender,
            event_receiver,
            server_event_sender: None,
        }
    }

    /// Take the event receiver
    ///
    /// Returns the receiver once. Use this to set up event processing
    /// in the ClipboardManager.
    pub fn take_event_receiver(&mut self) -> Option<ClipboardEventReceiver> {
        self.event_receiver.take()
    }

    /// Get a clone of the event sender
    ///
    /// Use this to create additional backends that share the same event channel.
    pub fn event_sender(&self) -> ClipboardEventSender {
        self.event_sender.clone()
    }
}

impl CliprdrBackendFactory for WrdCliprdrFactory {
    fn build_cliprdr_backend(&self) -> Box<dyn ironrdp_cliprdr::backend::CliprdrBackend> {
        debug!("Building clipboard backend for new connection");

        // Create backend using library implementation
        // Use /tmp/lamco-clipboard for temporary file storage
        let backend = RdpCliprdrBackend::new(
            "/tmp/lamco-clipboard".to_string(),
            self.event_sender.clone(),
        );

        Box::new(backend)
    }
}

impl ServerEventSender for WrdCliprdrFactory {
    fn set_sender(&mut self, sender: mpsc::UnboundedSender<ironrdp_server::ServerEvent>) {
        info!("Clipboard factory received server event sender");
        self.server_event_sender = Some(sender.clone());

        // Register sender with ClipboardManager for delayed rendering requests
        let manager = Arc::clone(&self.clipboard_manager);
        let sender_clone = sender;
        tokio::spawn(async move {
            if let Ok(mgr) = manager.try_lock() {
                mgr.set_server_event_sender(sender_clone).await;
            }
        });
    }
}

impl ironrdp_server::CliprdrServerFactory for WrdCliprdrFactory {}

impl std::fmt::Debug for WrdCliprdrFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WrdCliprdrFactory")
            .field("has_server_sender", &self.server_event_sender.is_some())
            .field("has_event_receiver", &self.event_receiver.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::manager::ClipboardConfig;

    #[tokio::test]
    async fn test_factory_creation() {
        let config = ClipboardConfig::default();
        let manager = Arc::new(Mutex::new(ClipboardManager::new(config).await.unwrap()));

        let factory = WrdCliprdrFactory::new(manager);
        let _backend = factory.build_cliprdr_backend();
        // Backend created successfully
    }

    #[tokio::test]
    async fn test_take_event_receiver() {
        let config = ClipboardConfig::default();
        let manager = Arc::new(Mutex::new(ClipboardManager::new(config).await.unwrap()));

        let mut factory = WrdCliprdrFactory::new(manager);

        // First take should succeed
        assert!(factory.take_event_receiver().is_some());

        // Second take should return None
        assert!(factory.take_event_receiver().is_none());
    }
}
