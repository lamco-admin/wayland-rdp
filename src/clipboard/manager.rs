//! Clipboard Manager
//!
//! Main clipboard synchronization coordinator that manages bidirectional
//! clipboard sharing between RDP client and Wayland compositor.

use crate::clipboard::dbus_bridge::{ClipboardChangedEvent, DbusBridge};
use crate::clipboard::error::{ClipboardError, Result};
use crate::clipboard::formats::{ClipboardFormat, FormatConverter};
use crate::clipboard::sync::{ClipboardState, LoopDetectionConfig, LoopDetector, SyncManager};
use crate::clipboard::transfer::{TransferConfig, TransferEngine};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};

/// Clipboard configuration
#[derive(Debug, Clone)]
pub struct ClipboardConfig {
    /// Maximum data size in bytes
    pub max_data_size: usize,

    /// Enable image format support
    pub enable_images: bool,

    /// Enable file transfer support
    pub enable_files: bool,

    /// Enable HTML format support
    pub enable_html: bool,

    /// Enable RTF format support
    pub enable_rtf: bool,

    /// Chunk size for transfers
    pub chunk_size: usize,

    /// Transfer timeout in milliseconds
    pub timeout_ms: u64,

    /// Loop detection window in milliseconds
    pub loop_detection_window_ms: u64,

    /// Minimum milliseconds between forwarded clipboard events (rate limiting)
    /// Prevents rapid-fire D-Bus signals from overwhelming Portal. Set to 0 to disable.
    pub rate_limit_ms: u64,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            max_data_size: 16 * 1024 * 1024, // 16MB
            enable_images: true,
            enable_files: true,
            enable_html: true,
            enable_rtf: true,
            chunk_size: 64 * 1024, // 64KB chunks
            timeout_ms: 5000,
            loop_detection_window_ms: 500,
            rate_limit_ms: 200, // Max 5 events/second
        }
    }
}

/// Response callback for sending data back to RDP
pub type RdpResponseCallback = Arc<dyn Fn(Vec<u8>) + Send + Sync>;

/// Clipboard events from RDP or Portal
#[derive(Clone)]
pub enum ClipboardEvent {
    /// RDP client announced available formats
    RdpFormatList(Vec<ClipboardFormat>),

    /// RDP client requests data in specific format (with callback to send response)
    RdpDataRequest(u32, Option<RdpResponseCallback>),

    /// RDP client provides requested data
    RdpDataResponse(Vec<u8>),

    /// Portal announced available MIME types
    PortalFormatsAvailable(Vec<String>),

    /// Portal requests data in specific MIME type
    PortalDataRequest(String),

    /// Portal provides requested data
    PortalDataResponse(Vec<u8>),
}

impl std::fmt::Debug for ClipboardEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RdpFormatList(formats) => write!(f, "RdpFormatList({} formats)", formats.len()),
            Self::RdpDataRequest(id, _) => write!(f, "RdpDataRequest({})", id),
            Self::RdpDataResponse(data) => write!(f, "RdpDataResponse({} bytes)", data.len()),
            Self::PortalFormatsAvailable(mimes) => write!(f, "PortalFormatsAvailable({:?})", mimes),
            Self::PortalDataRequest(mime) => write!(f, "PortalDataRequest({})", mime),
            Self::PortalDataResponse(data) => write!(f, "PortalDataResponse({} bytes)", data.len()),
        }
    }
}

/// Clipboard manager coordinates all clipboard operations
#[derive(Debug)]
pub struct ClipboardManager {
    /// Configuration
    config: ClipboardConfig,

    /// Format converter
    converter: Arc<FormatConverter>,

    /// Transfer engine
    transfer_engine: Arc<TransferEngine>,

    /// Synchronization manager
    sync_manager: Arc<RwLock<SyncManager>>,

    /// Event sender
    event_tx: mpsc::Sender<ClipboardEvent>,

    /// Shutdown signal
    shutdown_tx: Option<mpsc::Sender<()>>,

    /// Portal clipboard manager for read/write operations (wrapped for dynamic update)
    portal_clipboard: Arc<RwLock<Option<Arc<crate::portal::clipboard::ClipboardManager>>>>,

    /// Portal session (shared with input handler, wrapped for concurrent access and dynamic update)
    portal_session: Arc<RwLock<Option<Arc<Mutex<ashpd::desktop::Session<'static, ashpd::desktop::remote_desktop::RemoteDesktop<'static>>>>>>>,

    /// Pending Portal SelectionTransfer requests (serial → mime_type)
    /// Used to correlate SelectionTransfer signals with RDP FormatDataResponse
    pending_portal_requests: Arc<RwLock<std::collections::HashMap<u32, String>>>,

    /// Server event sender for sending clipboard requests to IronRDP
    /// Set by WrdCliprdrFactory after ServerEvent sender is available
    server_event_sender: Arc<RwLock<Option<mpsc::UnboundedSender<ironrdp_server::ServerEvent>>>>,

    /// D-Bus bridge for GNOME clipboard extension integration
    /// This enables Linux→Windows clipboard on GNOME where Portal signals don't work
    dbus_bridge: Arc<RwLock<Option<DbusBridge>>>,

    /// Recently written content hashes (for loop suppression)
    /// When we write data to Portal, D-Bus bridge will see it as a clipboard change.
    /// We track hashes of data WE wrote to suppress forwarding it back to RDP.
    /// Maps hash → timestamp of write
    recently_written_hashes: Arc<RwLock<std::collections::HashMap<String, std::time::Instant>>>,
}

impl ClipboardManager {
    /// Create a new clipboard manager
    pub async fn new(config: ClipboardConfig) -> Result<Self> {
        let converter = Arc::new(FormatConverter::new());

        let transfer_config = TransferConfig {
            chunk_size: config.chunk_size,
            max_data_size: config.max_data_size,
            timeout: std::time::Duration::from_millis(config.timeout_ms),
            verify_integrity: true,
        };
        let transfer_engine = Arc::new(TransferEngine::new(transfer_config));

        let loop_config = LoopDetectionConfig {
            window_ms: config.loop_detection_window_ms,
            max_history: 10,
            enable_content_hashing: true,
        };
        let loop_detector = LoopDetector::new(loop_config);
        let sync_manager = Arc::new(RwLock::new(SyncManager::new(loop_detector)));

        let (event_tx, event_rx) = mpsc::channel(100);

        let mut manager = Self {
            config,
            converter,
            transfer_engine,
            sync_manager,
            event_tx,
            shutdown_tx: None,
            portal_clipboard: Arc::new(RwLock::new(None)), // Will be set after Portal initialization
            portal_session: Arc::new(RwLock::new(None)), // Will be set with portal_clipboard
            pending_portal_requests: Arc::new(RwLock::new(std::collections::HashMap::new())),
            server_event_sender: Arc::new(RwLock::new(None)), // Set by WrdCliprdrFactory
            dbus_bridge: Arc::new(RwLock::new(None)), // Will be set by start_dbus_clipboard_listener
            recently_written_hashes: Arc::new(RwLock::new(std::collections::HashMap::new())),
        };

        // Start event processor
        manager.start_event_processor(event_rx);

        info!("Clipboard manager initialized");

        Ok(manager)
    }

    /// Get event sender for external components
    pub fn event_sender(&self) -> mpsc::Sender<ClipboardEvent> {
        self.event_tx.clone()
    }

    /// Set server event sender (called by WrdCliprdrFactory after initialization)
    pub async fn set_server_event_sender(&self, sender: mpsc::UnboundedSender<ironrdp_server::ServerEvent>) {
        *self.server_event_sender.write().await = Some(sender);
        info!("✅ ServerEvent sender registered with clipboard manager");
    }

    /// Set Portal clipboard manager and session (async to acquire write lock)
    pub async fn set_portal_clipboard(
        &mut self,
        portal: Arc<crate::portal::clipboard::ClipboardManager>,
        session: Arc<Mutex<ashpd::desktop::Session<'static, ashpd::desktop::remote_desktop::RemoteDesktop<'static>>>>,
    ) {
        *self.portal_clipboard.write().await = Some(Arc::clone(&portal));
        *self.portal_session.write().await = Some(Arc::clone(&session));
        info!("✅ Portal clipboard and session dynamically set in clipboard manager");

        // Start SelectionTransfer listener for delayed rendering (Windows → Linux paste)
        self.start_selection_transfer_listener(Arc::clone(&portal), Arc::clone(&session)).await;

        // Start SelectionOwnerChanged listener for local clipboard monitoring (Linux → Windows copy)
        self.start_owner_changed_listener(Arc::clone(&portal), Arc::clone(&session)).await;

        // DISABLED: Polling fallback causes session lock contention breaking input injection
        // TODO: Fix by using separate session or different clipboard monitoring approach
        // self.start_clipboard_polling_fallback(portal, session).await;

        // Start D-Bus bridge for GNOME clipboard extension (Linux → Windows fallback)
        // This provides an alternative to SelectionOwnerChanged which doesn't work on GNOME
        self.start_dbus_clipboard_listener().await;
    }

    /// Start SelectionTransfer listener for delayed rendering
    ///
    /// This enables Windows → Linux clipboard flow:
    /// 1. Windows user copies (we announced formats via SetSelection)
    /// 2. Linux user pastes
    /// 3. Portal sends SelectionTransfer signal
    /// 4. We request data from RDP client via ServerEvent
    /// 5. We receive data in on_format_data_response
    /// 6. We write data via SelectionWrite using tracked serial
    async fn start_selection_transfer_listener(
        &self,
        portal: Arc<crate::portal::clipboard::ClipboardManager>,
        _session: Arc<Mutex<ashpd::desktop::Session<'static, ashpd::desktop::remote_desktop::RemoteDesktop<'static>>>>,
    ) {
        // Create channel for SelectionTransfer events
        let (transfer_tx, mut transfer_rx) = mpsc::unbounded_channel();

        // Start the Portal listener (spawns background task)
        match portal.start_selection_transfer_listener(transfer_tx).await {
            Ok(()) => {
                info!("Starting SelectionTransfer handler task");

                // Clone refs for the spawned task
                let pending_requests = Arc::clone(&self.pending_portal_requests);
                let server_event_sender = Arc::clone(&self.server_event_sender);
                let converter = Arc::clone(&self.converter);

                // Spawn task to handle SelectionTransfer events
                tokio::spawn(async move {
                    while let Some(transfer_event) = transfer_rx.recv().await {
                        info!("📥 SelectionTransfer signal: {} (serial {})",
                            transfer_event.mime_type, transfer_event.serial);

                        // Track this transfer request (serial → mime_type)
                        // When RDP FormatDataResponse arrives, we'll use this serial to write to Portal
                        pending_requests.write().await.insert(
                            transfer_event.serial,
                            transfer_event.mime_type.clone()
                        );

                        // Convert MIME type → RDP format ID
                        let format_id = match converter.mime_to_format_id(&transfer_event.mime_type) {
                            Ok(id) => id,
                            Err(e) => {
                                error!("Failed to convert MIME {} to format ID: {}", transfer_event.mime_type, e);
                                pending_requests.write().await.remove(&transfer_event.serial);
                                continue;
                            }
                        };

                        // Send ServerEvent to request data from RDP client (TRUE delayed rendering!)
                        let sender_opt = server_event_sender.read().await.clone();
                        if let Some(sender) = sender_opt {
                            use ironrdp_cliprdr::backend::ClipboardMessage;
                            use ironrdp_cliprdr::pdu::ClipboardFormatId;

                            if let Err(e) = sender.send(ironrdp_server::ServerEvent::Clipboard(
                                ClipboardMessage::SendInitiatePaste(ClipboardFormatId(format_id))
                            )) {
                                error!("Failed to send FormatDataRequest via ServerEvent: {:?}", e);
                                pending_requests.write().await.remove(&transfer_event.serial);
                            } else {
                                info!("✅ Sent FormatDataRequest for format {} (Portal serial {})", format_id, transfer_event.serial);
                            }
                        } else {
                            warn!("ServerEvent sender not available yet - cannot request from RDP");
                            pending_requests.write().await.remove(&transfer_event.serial);
                        }
                    }

                    warn!("SelectionTransfer handler task ended");
                });

                info!("✅ SelectionTransfer listener and handler started - delayed rendering enabled");
            }
            Err(e) => {
                error!("Failed to start SelectionTransfer listener: {:#}", e);
                warn!("Delayed rendering (Windows → Linux paste) will not work");
            }
        }
    }

    /// Start clipboard polling fallback for Linux→Windows
    ///
    /// This is a fallback mechanism in case SelectionOwnerChanged signals don't work
    /// (known limitation in some Portal backends like xdg-desktop-portal-gnome).
    /// Polls the clipboard every 500ms to detect changes.
    async fn start_clipboard_polling_fallback(
        &self,
        portal: Arc<crate::portal::clipboard::ClipboardManager>,
        session: Arc<Mutex<ashpd::desktop::Session<'static, ashpd::desktop::remote_desktop::RemoteDesktop<'static>>>>,
    ) {
        info!("Starting clipboard polling fallback (500ms interval)");

        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            use sha2::{Digest, Sha256};
            let mut last_hash: Option<String> = None;
            let mut poll_count = 0;
            let mut detection_count = 0;

            // Wait 2 seconds before starting polls (let signals try first)
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            info!("Clipboard polling fallback active - checking every 500ms");

            loop {
                poll_count += 1;
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                // Try to read current clipboard content
                let session_guard = session.lock().await;
                let read_result = portal.read_local_clipboard(&session_guard, "text/plain;charset=utf-8").await;

                match read_result {
                    Ok(data) if !data.is_empty() => {
                        // Calculate hash of clipboard content
                        let hash = format!("{:x}", Sha256::digest(&data));

                        if Some(&hash) != last_hash.as_ref() {
                            detection_count += 1;
                            last_hash = Some(hash.clone());

                            info!("📋 Clipboard change detected via POLLING (poll #{}, detection #{})",
                                poll_count, detection_count);
                            info!("   Content hash: {}..., size: {} bytes",
                                &hash[..16], data.len());

                            // Announce to RDP clients
                            let mime_types = vec!["text/plain;charset=utf-8".to_string(), "text/plain".to_string()];
                            if let Err(e) = event_tx.send(ClipboardEvent::PortalFormatsAvailable(mime_types)).await {
                                error!("Failed to send clipboard change event: {}", e);
                                break;
                            }
                        }
                    }
                    Ok(_) => {
                        // Empty clipboard
                        if last_hash.is_some() {
                            debug!("Clipboard now empty (poll #{})", poll_count);
                            last_hash = None;
                        }
                    }
                    Err(e) => {
                        // Can't read clipboard (normal if nothing copied yet)
                        if poll_count % 20 == 0 {
                            debug!("Clipboard read failed on poll #{}: {}", poll_count, e);
                        }
                    }
                }
            }
        });

        info!("✅ Clipboard polling fallback started (workaround for missing SelectionOwnerChanged)");
    }

    /// Start SelectionOwnerChanged listener for local clipboard monitoring
    ///
    /// This enables Linux → Windows clipboard flow:
    /// 1. Linux user copies (another app owns clipboard)
    /// 2. Portal sends SelectionOwnerChanged signal (mime_types, session_is_owner=false)
    /// 3. We convert MIME types → RDP formats
    /// 4. We send FormatList PDU to RDP clients
    /// 5. Windows user pastes → RDP sends FormatDataRequest
    /// 6. We read from Portal clipboard via SelectionRead
    /// 7. We send FormatDataResponse to RDP client
    async fn start_owner_changed_listener(
        &self,
        portal: Arc<crate::portal::clipboard::ClipboardManager>,
        _session: Arc<Mutex<ashpd::desktop::Session<'static, ashpd::desktop::remote_desktop::RemoteDesktop<'static>>>>,
    ) {
        // Create channel for SelectionOwnerChanged events
        let (owner_tx, mut owner_rx) = mpsc::unbounded_channel();

        // Start the Portal listener
        match portal.start_owner_changed_listener(owner_tx).await {
            Ok(()) => {
                info!("Starting SelectionOwnerChanged handler task");

                let event_tx = self.event_tx.clone();

                // Spawn task to handle clipboard ownership changes
                tokio::spawn(async move {
                    info!("SelectionOwnerChanged handler task ready - waiting for clipboard changes");
                    let mut change_count = 0;

                    while let Some(mime_types) = owner_rx.recv().await {
                        change_count += 1;
                        info!("📋 Local clipboard change #{}: {} formats: {:?}", change_count, mime_types.len(), mime_types);

                        // Send event to announce these formats to RDP clients
                        if let Err(e) = event_tx.send(ClipboardEvent::PortalFormatsAvailable(mime_types.clone())).await {
                            error!("Failed to send PortalFormatsAvailable event: {}", e);
                            break;
                        } else {
                            info!("✅ Sent PortalFormatsAvailable event to clipboard manager");
                        }
                    }

                    warn!("SelectionOwnerChanged handler task ended after {} changes", change_count);
                });

                info!("✅ SelectionOwnerChanged listener started - monitoring Linux clipboard");
            }
            Err(e) => {
                error!("Failed to start SelectionOwnerChanged listener: {:#}", e);
                warn!("Linux → Windows clipboard flow will not work via Portal signals");
                warn!("Will attempt D-Bus bridge for GNOME extension fallback");
            }
        }
    }

    /// Start D-Bus clipboard listener for GNOME extension integration
    ///
    /// This enables Linux → Windows clipboard flow on GNOME desktops where
    /// Portal's SelectionOwnerChanged signal doesn't work. The wayland-rdp-clipboard
    /// GNOME Shell extension monitors the clipboard and emits D-Bus signals.
    ///
    /// # Flow
    /// 1. GNOME extension detects clipboard change (via St.Clipboard polling)
    /// 2. Extension emits ClipboardChanged signal on D-Bus
    /// 3. We receive signal and send PortalFormatsAvailable event
    /// 4. ClipboardManager announces formats to RDP client
    /// 5. When RDP client pastes, we read from Portal clipboard
    pub async fn start_dbus_clipboard_listener(&self) {
        info!("Checking for GNOME clipboard extension on D-Bus...");

        let mut bridge = DbusBridge::new();

        // Check if extension is available
        if !bridge.check_extension_available().await {
            info!("GNOME clipboard extension not detected - D-Bus bridge inactive");
            info!("Install wayland-rdp-clipboard extension for Linux → Windows clipboard on GNOME");
            return;
        }

        // Try to get version for logging
        match bridge.get_version().await {
            Ok(version) => info!("GNOME clipboard extension version: {}", version),
            Err(e) => debug!("Could not get extension version: {}", e),
        }

        // Test connectivity
        match bridge.ping().await {
            Ok(reply) => debug!("Extension ping successful: {}", reply),
            Err(e) => {
                warn!("Extension ping failed: {} - continuing anyway", e);
            }
        }

        // Create channel for D-Bus events
        let (dbus_tx, mut dbus_rx) = mpsc::unbounded_channel::<ClipboardChangedEvent>();

        // Start signal listener
        if let Err(e) = bridge.start_signal_listener(dbus_tx).await {
            error!("Failed to start D-Bus signal listener: {}", e);
            return;
        }

        // Store bridge reference
        *self.dbus_bridge.write().await = Some(bridge);

        // Clone event sender and hash tracker for the spawned task
        let event_tx = self.event_tx.clone();
        let recently_written_hashes = Arc::clone(&self.recently_written_hashes);
        let rate_limit_ms = self.config.rate_limit_ms;

        // Spawn task to forward D-Bus events to ClipboardManager
        tokio::spawn(async move {
            info!("D-Bus clipboard event forwarder started (rate limit: {}ms)", rate_limit_ms);
            let mut event_count = 0;
            let mut suppressed_count = 0;
            let mut rate_limited_count = 0;

            // Loop suppression: ignore events within this window after we wrote data
            const LOOP_SUPPRESSION_WINDOW_MS: u128 = 2000;
            // Maximum pending hash entries (prevent unbounded memory)
            const MAX_HASH_CACHE_SIZE: usize = 50;

            let mut last_forward_time: Option<std::time::Instant> = None;

            while let Some(dbus_event) = dbus_rx.recv().await {
                event_count += 1;

                // Skip PRIMARY selection for now (RDP only supports CLIPBOARD)
                if dbus_event.is_primary {
                    debug!("Ignoring PRIMARY selection change (RDP doesn't support it)");
                    continue;
                }

                let hash_short = &dbus_event.content_hash[..8.min(dbus_event.content_hash.len())];

                // RATE LIMITING: Enforce minimum interval between forwarded events
                // This prevents rapid-fire D-Bus signals from overwhelming the Portal
                if rate_limit_ms > 0 {
                    if let Some(last_time) = last_forward_time {
                        let elapsed = last_time.elapsed().as_millis() as u64;
                        if elapsed < rate_limit_ms {
                            rate_limited_count += 1;
                            debug!(
                                "⏱️ Rate limited: {}ms since last event (min: {}ms) - skipping event #{}",
                                elapsed, rate_limit_ms, event_count
                            );
                            continue;
                        }
                    }
                }

                // LOOP SUPPRESSION: Check if this hash matches data we recently wrote to Portal
                // If so, this is feedback from our own write - don't forward back to RDP!
                {
                    let mut hashes = recently_written_hashes.write().await;

                    // Clean up old entries (older than suppression window)
                    let now = std::time::Instant::now();
                    hashes.retain(|_, written_at| {
                        now.duration_since(*written_at).as_millis() < LOOP_SUPPRESSION_WINDOW_MS
                    });

                    // Also enforce max cache size (evict oldest if too large)
                    if hashes.len() > MAX_HASH_CACHE_SIZE {
                        // Find and remove the oldest entry
                        if let Some(oldest_key) = hashes.iter()
                            .min_by_key(|(_, time)| *time)
                            .map(|(k, _)| k.clone())
                        {
                            hashes.remove(&oldest_key);
                            debug!("Evicted oldest hash from cache (size limit: {})", MAX_HASH_CACHE_SIZE);
                        }
                    }

                    // Check if this event's hash matches one we recently wrote
                    if hashes.contains_key(&dbus_event.content_hash) {
                        suppressed_count += 1;
                        info!(
                            "🔄 LOOP SUPPRESSED #{}: D-Bus event hash {} matches our recent write - skipping",
                            suppressed_count, hash_short
                        );
                        continue;
                    }
                }

                // Update rate limit timestamp
                last_forward_time = Some(std::time::Instant::now());

                info!(
                    "📋 D-Bus clipboard change #{}: {} MIME types (hash: {})",
                    event_count,
                    dbus_event.mime_types.len(),
                    hash_short
                );
                debug!("   MIME types: {:?}", dbus_event.mime_types);

                // Forward to ClipboardManager as PortalFormatsAvailable event
                // This triggers the same flow as if Portal had sent SelectionOwnerChanged
                if let Err(e) = event_tx
                    .send(ClipboardEvent::PortalFormatsAvailable(dbus_event.mime_types))
                    .await
                {
                    error!("Failed to forward D-Bus event to ClipboardManager: {}", e);
                    break;
                }

                info!("✅ Forwarded clipboard change to RDP client announcement flow");
            }

            warn!(
                "D-Bus clipboard event forwarder ended after {} events ({} loop-suppressed, {} rate-limited)",
                event_count, suppressed_count, rate_limited_count
            );
        });

        info!("✅ D-Bus clipboard bridge started - GNOME extension integration active");
        info!("   Linux → Windows clipboard now enabled via extension");
    }

    /// Start event processing loop
    fn start_event_processor(&mut self, mut event_rx: mpsc::Receiver<ClipboardEvent>) {
        let converter = self.converter.clone();
        let sync_manager = self.sync_manager.clone();
        let transfer_engine = self.transfer_engine.clone();
        let config = self.config.clone();
        // Clone the Arc<RwLock<>> wrappers - they can be read dynamically
        let portal_clipboard = Arc::clone(&self.portal_clipboard);
        let portal_session = Arc::clone(&self.portal_session);
        let pending_portal_requests = Arc::clone(&self.pending_portal_requests);
        let server_event_sender = Arc::clone(&self.server_event_sender);
        let recently_written_hashes = Arc::clone(&self.recently_written_hashes);

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(event) = event_rx.recv() => {
                        if let Err(e) = Self::handle_event(
                            event,
                            &converter,
                            &sync_manager,
                            &transfer_engine,
                            &config,
                            &portal_clipboard,
                            &portal_session,
                            &pending_portal_requests,
                            &server_event_sender,
                            &recently_written_hashes,
                        ).await {
                            error!("Error handling clipboard event: {:?}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        debug!("Clipboard manager shutting down");
                        break;
                    }
                }
            }
        });
    }

    /// Handle a clipboard event
    async fn handle_event(
        event: ClipboardEvent,
        converter: &FormatConverter,
        sync_manager: &Arc<RwLock<SyncManager>>,
        transfer_engine: &TransferEngine,
        _config: &ClipboardConfig,
        portal_clipboard: &Arc<RwLock<Option<Arc<crate::portal::clipboard::ClipboardManager>>>>,
        portal_session: &Arc<RwLock<Option<Arc<Mutex<ashpd::desktop::Session<'static, ashpd::desktop::remote_desktop::RemoteDesktop<'static>>>>>>>,
        pending_portal_requests: &Arc<RwLock<std::collections::HashMap<u32, String>>>,
        server_event_sender: &Arc<RwLock<Option<mpsc::UnboundedSender<ironrdp_server::ServerEvent>>>>,
        recently_written_hashes: &Arc<RwLock<std::collections::HashMap<String, std::time::Instant>>>,
    ) -> Result<()> {
        match event {
            ClipboardEvent::RdpFormatList(formats) => {
                Self::handle_rdp_format_list(formats, converter, sync_manager, portal_clipboard, portal_session).await
            }

            ClipboardEvent::RdpDataRequest(format_id, response_callback) => {
                Self::handle_rdp_data_request(format_id, response_callback, converter, sync_manager, portal_clipboard, portal_session).await
            }

            ClipboardEvent::RdpDataResponse(data) => {
                Self::handle_rdp_data_response(data, sync_manager, transfer_engine, portal_clipboard, portal_session, pending_portal_requests, recently_written_hashes).await
            }

            ClipboardEvent::PortalFormatsAvailable(mime_types) => {
                Self::handle_portal_formats(mime_types, converter, sync_manager, server_event_sender).await
            }

            ClipboardEvent::PortalDataRequest(mime_type) => {
                Self::handle_portal_data_request(mime_type, converter, sync_manager, portal_clipboard, portal_session).await
            }

            ClipboardEvent::PortalDataResponse(data) => {
                Self::handle_portal_data_response(data, sync_manager, transfer_engine).await
            }
        }
    }

    /// Handle RDP format list announcement
    async fn handle_rdp_format_list(
        formats: Vec<ClipboardFormat>,
        converter: &FormatConverter,
        sync_manager: &Arc<RwLock<SyncManager>>,
        portal_clipboard: &Arc<RwLock<Option<Arc<crate::portal::clipboard::ClipboardManager>>>>,
        portal_session: &Arc<RwLock<Option<Arc<Mutex<ashpd::desktop::Session<'static, ashpd::desktop::remote_desktop::RemoteDesktop<'static>>>>>>>,
    ) -> Result<()> {
        debug!("RDP format list received: {:?}", formats);

        // Check with sync manager (loop detection)
        let should_sync = {
            let mut mgr = sync_manager.write().await;
            mgr.handle_rdp_formats(formats.clone())?
        };

        if !should_sync {
            debug!("Skipping RDP format list due to loop detection");
            return Ok(());
        }

        // Convert RDP formats to MIME types
        let mime_types = converter.rdp_to_mime_types(&formats)?;

        debug!("Converted to MIME types: {:?}", mime_types);

        // Get Portal clipboard and session (dynamically read from Arc<RwLock<>>)
        let portal_opt = portal_clipboard.read().await.clone();
        let session_opt = portal_session.read().await.clone();

        debug!("Checking Portal availability: clipboard={}, session={}",
            portal_opt.is_some(), session_opt.is_some());

        let (portal, session) = match (portal_opt, session_opt) {
            (Some(p), Some(s)) => (p, s),
            (None, Some(_)) => {
                warn!("Portal clipboard not available (but session is)");
                return Ok(());
            }
            (Some(_), None) => {
                warn!("Portal session not available (but clipboard is) - THIS SHOULD NOT HAPPEN");
                return Ok(());
            }
            (None, None) => {
                debug!("Portal clipboard and session not yet initialized (normal during startup)");
                return Ok(());
            }
        };

        // Announce formats to Portal using delayed rendering (SetSelection)
        // This tells Wayland "these formats are available" WITHOUT transferring data
        let session_guard = session.lock().await;
        portal.announce_rdp_formats(&session_guard, mime_types).await
            .map_err(|e| ClipboardError::PortalError(format!("Failed to announce formats: {}", e)))?;

        info!("✅ RDP clipboard formats announced to Portal via SetSelection");

        Ok(())
    }

    /// Handle RDP data request - Client wants data from Portal clipboard (Linux → Windows)
    ///
    /// This is called when Windows user pastes and RDP client requests our (Linux) clipboard data.
    /// We read from Portal clipboard via SelectionRead and send back to RDP client.
    async fn handle_rdp_data_request(
        format_id: u32,
        response_callback: Option<RdpResponseCallback>,
        converter: &FormatConverter,
        _sync_manager: &Arc<RwLock<SyncManager>>,
        portal_clipboard: &Arc<RwLock<Option<Arc<crate::portal::clipboard::ClipboardManager>>>>,
        portal_session: &Arc<RwLock<Option<Arc<Mutex<ashpd::desktop::Session<'static, ashpd::desktop::remote_desktop::RemoteDesktop<'static>>>>>>>,
    ) -> Result<()> {
        debug!("RDP data request for format ID: {}", format_id);

        // Get Portal clipboard and session
        let portal_opt = portal_clipboard.read().await.clone();
        let session_opt = portal_session.read().await.clone();

        let (portal, session) = match (portal_opt, session_opt) {
            (Some(p), Some(s)) => (p, s),
            _ => {
                warn!("Portal not available for RDP data request");
                return Ok(());
            }
        };

        // Convert format ID to MIME type
        let mime_type = converter.format_id_to_mime(format_id)?;
        debug!("Format {} maps to MIME: {}", format_id, mime_type);

        // Read from Portal clipboard via SelectionRead
        let session_guard = session.lock().await;
        let portal_data = match portal.read_local_clipboard(&session_guard, &mime_type).await {
            Ok(data) => {
                info!("📖 Read {} bytes from Portal clipboard ({})", data.len(), mime_type);
                data
            }
            Err(e) => {
                error!("Failed to read from Portal clipboard: {:#}", e);
                return Ok(()); // Return empty/error - RDP client will handle
            }
        };

        // Convert Portal data to RDP format
        let rdp_data = if mime_type.starts_with("text/plain") {
            // Convert UTF-8 to UTF-16LE for RDP
            let text = String::from_utf8_lossy(&portal_data);
            let utf16: Vec<u16> = text.encode_utf16().collect();
            let mut bytes = Vec::with_capacity(utf16.len() * 2 + 2);
            for c in utf16 {
                bytes.extend_from_slice(&c.to_le_bytes());
            }
            bytes.extend_from_slice(&[0, 0]); // Null terminator
            bytes
        } else {
            portal_data
        };

        let data_len = rdp_data.len();
        debug!("Converted to RDP format: {} bytes", data_len);

        // Send response back to RDP client
        if let Some(callback) = response_callback {
            callback(rdp_data);
            info!("✅ Sent {} bytes to RDP client for format {}", data_len, format_id);
        } else {
            warn!("No response callback available for RDP data request");
        }

        Ok(())
    }

    /// Handle RDP data response - Client sent clipboard data, write to Portal
    ///
    /// This is called when RDP client sends FormatDataResponse in response to our
    /// FormatDataRequest (which was triggered by SelectionTransfer signal).
    /// We write the data to Portal using the tracked serial number.
    async fn handle_rdp_data_response(
        data: Vec<u8>,
        sync_manager: &Arc<RwLock<SyncManager>>,
        _transfer_engine: &TransferEngine,
        portal_clipboard: &Arc<RwLock<Option<Arc<crate::portal::clipboard::ClipboardManager>>>>,
        portal_session: &Arc<RwLock<Option<Arc<Mutex<ashpd::desktop::Session<'static, ashpd::desktop::remote_desktop::RemoteDesktop<'static>>>>>>>,
        pending_portal_requests: &Arc<RwLock<std::collections::HashMap<u32, String>>>,
        recently_written_hashes: &Arc<RwLock<std::collections::HashMap<String, std::time::Instant>>>,
    ) -> Result<()> {
        debug!("RDP data response received: {} bytes", data.len());

        // Check for content loop
        let should_transfer = sync_manager.write().await.check_content(&data, true)?;
        if !should_transfer {
            debug!("Skipping RDP data due to content loop detection");
            return Ok(());
        }

        // Get Portal clipboard and session
        let portal_opt = portal_clipboard.read().await.clone();
        let session_opt = portal_session.read().await.clone();

        let (portal, session) = match (portal_opt, session_opt) {
            (Some(p), Some(s)) => (p, s),
            _ => {
                warn!("Portal not available - cannot deliver clipboard data");
                return Ok(());
            }
        };

        // Get the pending Portal request (should have exactly one for the latest SelectionTransfer)
        // In a more sophisticated implementation, we'd track format_id → serial mapping
        // For now, we take the first/only pending request
        let pending = pending_portal_requests.read().await;
        let serial_opt = pending.iter().next().map(|(serial, _mime)| *serial);
        drop(pending);

        let serial = match serial_opt {
            Some(s) => s,
            None => {
                warn!("No pending Portal request - data arrived without SelectionTransfer");
                warn!("This can happen if user hasn't pasted yet - data will be discarded");
                return Ok(());
            }
        };

        // Convert RDP data to Portal format (UTF-16LE → UTF-8 for text)
        let portal_data = if data.len() >= 2 {
            // Detect if this is UTF-16 text
            let utf16_data: Vec<u16> = data
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .take_while(|&c| c != 0) // Stop at null terminator
                .collect();

            if let Ok(text) = String::from_utf16(&utf16_data) {
                // Successfully decoded as UTF-16 text
                let utf8_bytes = text.as_bytes().to_vec();
                debug!("Converted UTF-16 to UTF-8: {} UTF-16 chars ({} bytes) → {} UTF-8 bytes",
                    utf16_data.len(), data.len(), utf8_bytes.len());
                debug!("Text preview: {:?}", &text[..text.len().min(50)]);
                utf8_bytes
            } else {
                // Not valid UTF-16, use raw data
                warn!("Data is not valid UTF-16 text, using raw bytes");
                data
            }
        } else {
            data
        };

        // Write data to Portal via SelectionWrite workflow
        let session_guard = session.lock().await;
        match portal.write_selection_data(&session_guard, serial, portal_data.clone()).await {
            Ok(()) => {
                info!("✅ Clipboard data delivered to Portal via SelectionWrite (serial {})", serial);

                // Record hash of data we just wrote (for loop suppression)
                // D-Bus bridge will see this as a clipboard change and we need to suppress it
                {
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(&portal_data);
                    let hash = format!("{:x}", hasher.finalize());
                    let hash_short = &hash[..8];

                    recently_written_hashes.write().await.insert(hash.clone(), std::time::Instant::now());
                    info!("🔒 Recorded written hash {} for loop suppression", hash_short);
                }

                // Clear the pending request
                pending_portal_requests.write().await.remove(&serial);
            }
            Err(e) => {
                error!("Failed to write clipboard data to Portal: {:#}", e);

                // Clear pending request even on failure
                pending_portal_requests.write().await.remove(&serial);

                return Err(ClipboardError::PortalError(format!("SelectionWrite failed: {}", e)));
            }
        }

        Ok(())
    }

    /// Handle Portal format announcement (Linux → Windows)
    ///
    /// This is called when Linux clipboard changes (SelectionOwnerChanged signal).
    /// We need to send FormatList PDU to RDP clients to announce available formats.
    async fn handle_portal_formats(
        mime_types: Vec<String>,
        converter: &FormatConverter,
        sync_manager: &Arc<RwLock<SyncManager>>,
        server_event_sender: &Arc<RwLock<Option<mpsc::UnboundedSender<ironrdp_server::ServerEvent>>>>,
    ) -> Result<()> {
        info!("📥 handle_portal_formats called with {} MIME types: {:?}", mime_types.len(), mime_types);

        // Check with sync manager (loop detection)
        let should_sync = {
            let mut mgr = sync_manager.write().await;
            mgr.handle_portal_formats(mime_types.clone())?
        };

        if !should_sync {
            debug!("Skipping Portal formats due to loop detection");
            return Ok(());
        }

        // Convert MIME types to RDP formats
        let rdp_formats = converter.mime_to_rdp_formats(&mime_types)?;
        debug!("Converted {} MIME types to {} RDP formats", mime_types.len(), rdp_formats.len());

        // Convert to IronRDP ClipboardFormat type
        let ironrdp_formats: Vec<ironrdp_cliprdr::pdu::ClipboardFormat> = rdp_formats.iter().map(|f| {
            let name = if !f.format_name.is_empty() {
                Some(ironrdp_cliprdr::pdu::ClipboardFormatName::new(f.format_name.clone()))
            } else {
                None
            };
            ironrdp_cliprdr::pdu::ClipboardFormat {
                id: ironrdp_cliprdr::pdu::ClipboardFormatId(f.format_id),
                name,
            }
        }).collect();

        // Send ServerEvent to announce formats to RDP clients
        let sender_opt = server_event_sender.read().await.clone();
        if let Some(sender) = sender_opt {
            use ironrdp_cliprdr::backend::ClipboardMessage;

            if let Err(e) = sender.send(ironrdp_server::ServerEvent::Clipboard(
                ClipboardMessage::SendInitiateCopy(ironrdp_formats)
            )) {
                error!("Failed to send FormatList via ServerEvent: {:?}", e);
            } else {
                info!("✅ Sent FormatList with {} formats to RDP client (Linux clipboard changed)", rdp_formats.len());
            }
        } else {
            warn!("ServerEvent sender not available - cannot announce formats to RDP");
        }

        Ok(())
    }

    /// Handle Portal data request (from SelectionTransfer signal)
    ///
    /// This is called when Linux user pastes and Portal needs the clipboard data.
    /// We send a ServerEvent to request data from the RDP client.
    async fn handle_portal_data_request(
        mime_type: String,
        converter: &FormatConverter,
        _sync_manager: &Arc<RwLock<SyncManager>>,
        _portal_clipboard: &Arc<RwLock<Option<Arc<crate::portal::clipboard::ClipboardManager>>>>,
        _portal_session: &Arc<RwLock<Option<Arc<Mutex<ashpd::desktop::Session<'static, ashpd::desktop::remote_desktop::RemoteDesktop<'static>>>>>>>,
    ) -> Result<()> {
        debug!("Portal data request for MIME type: {}", mime_type);

        // Convert MIME type to RDP format ID
        let format_id = converter.mime_to_format_id(&mime_type)?;

        info!("📤 Portal needs data - will request format {} from RDP client", format_id);

        // NOTE: We can't send ServerEvent from here because we don't have the sender in event handlers.
        // The sender is available in the SelectionTransfer listener task.
        // We need to refactor to send the ServerEvent from there instead.
        //
        // For now, this handler just logs. The actual request will be sent from the
        // SelectionTransfer handler task which has access to both pending_requests and event_tx.

        warn!("⚠️  PortalDataRequest event received but ServerEvent sending happens in SelectionTransfer handler");

        Ok(())
    }

    /// Handle Portal data response
    async fn handle_portal_data_response(
        data: Vec<u8>,
        sync_manager: &Arc<RwLock<SyncManager>>,
        _transfer_engine: &TransferEngine,
    ) -> Result<()> {
        debug!("Portal data response received: {} bytes", data.len());

        // Check for content loop
        let should_transfer = sync_manager.write().await.check_content(&data, false)?;

        if !should_transfer {
            debug!("Skipping Portal data due to content loop detection");
            return Ok(());
        }

        // Note: Data forwarding to RDP happens via ironrdp_backend.rs
        // message proxy. This helper provides conversion logic when needed.

        Ok(())
    }

    /// Announce local clipboard formats to RDP client
    ///
    /// Called when local (Wayland) clipboard changes
    pub async fn announce_local_formats(&self) -> Result<()> {
        debug!("Announcing local clipboard formats");
        // Trigger format announcement - implementation calls helpers
        Ok(())
    }

    /// Handle remote copy from RDP client
    ///
    /// Called when RDP client announces available formats
    pub async fn handle_remote_copy(
        &self,
        formats: Vec<ClipboardFormat>,
    ) -> Result<()> {
        debug!("Handling remote copy with {} formats", formats.len());

        // Use sync manager for loop detection
        let should_sync = {
            let mut mgr = self.sync_manager.write().await;
            mgr.handle_rdp_formats(formats.clone())?
        };

        if !should_sync {
            debug!("Skipping remote copy due to loop detection");
            return Ok(());
        }

        // Convert RDP formats to MIME types
        let mime_types = self.converter.rdp_to_mime_types(&formats)?;
        debug!("Converted {} formats to MIME types: {:?}", mime_types.len(), mime_types);

        Ok(())
    }

    /// Handle format data request from RDP client
    ///
    /// Called when RDP client wants data from Portal clipboard
    pub async fn handle_format_data_request(
        &self,
        format_id: u32,
    ) -> Result<Vec<u8>> {
        debug!("Handling format data request for format ID: {}", format_id);

        // Get MIME type for format
        let mime_type = self.converter.format_id_to_mime(format_id)?;

        // Check current state
        let state = self.sync_manager.read().await.state().clone();

        match state {
            ClipboardState::PortalOwned(_mime_types) => {
                debug!("Fetching data from Portal for MIME type: {}", mime_type);

                // Placeholder: In full implementation, would fetch from Portal here
                // For now, return empty data
                Ok(Vec::new())
            }
            _ => {
                warn!("Format data request in invalid state: {:?}", state);
                Err(ClipboardError::InvalidState(format!(
                    "Cannot handle format data request in state: {:?}",
                    state
                )))
            }
        }
    }

    /// Handle format data response from RDP client
    ///
    /// Called when RDP client provides requested data
    pub async fn handle_format_data_response(
        &self,
        data: Vec<u8>,
    ) -> Result<()> {
        debug!("Handling format data response: {} bytes", data.len());

        // Check for content loop
        let should_transfer = self.sync_manager.write().await.check_content(&data, true)?;

        if !should_transfer {
            debug!("Skipping format data response due to content loop detection");
            return Ok(());
        }

        // Placeholder: In full implementation, would set Portal clipboard here
        debug!("Would set Portal clipboard with {} bytes", data.len());

        Ok(())
    }

    /// Handle file contents request (public wrapper)
    pub async fn handle_file_contents_request(&self, stream_id: u32, list_index: u32) -> Result<()> {
        debug!("File contents request: stream={}, index={}", stream_id, list_index);

        // Create temporary directory if it doesn't exist
        let temp_dir = std::path::Path::new("/tmp/wrd-clipboard");
        if !temp_dir.exists() {
            std::fs::create_dir_all(temp_dir).map_err(|e| {
                ClipboardError::Io(e)
            })?;
        }

        // Placeholder: In full implementation, would read file from Portal
        debug!("File contents request handling - file transfer implementation pending");

        Ok(())
    }

    /// Handle file contents response (public wrapper)
    pub async fn handle_file_contents_response(&self, stream_id: u32, data: Vec<u8>) -> Result<()> {
        debug!("File contents response: stream={}, {} bytes", stream_id, data.len());

        // Create temporary directory if it doesn't exist
        let temp_dir = std::path::Path::new("/tmp/wrd-clipboard");
        if !temp_dir.exists() {
            std::fs::create_dir_all(temp_dir).map_err(|e| {
                ClipboardError::Io(e)
            })?;
        }

        // Write file chunk
        let file_path = temp_dir.join(format!("file_{}", stream_id));
        std::fs::write(&file_path, &data).map_err(|e| {
            ClipboardError::Io(e)
        })?;

        debug!("Wrote {} bytes to {:?}", data.len(), file_path);

        Ok(())
    }

    /// Shutdown the clipboard manager
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down clipboard manager");

        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clipboard_manager_creation() {
        let config = ClipboardConfig::default();
        let mut manager = ClipboardManager::new(config).await.unwrap();

        assert!(manager.event_tx.capacity() > 0);
    }

    #[tokio::test]
    async fn test_rdp_format_list_handling() {
        let config = ClipboardConfig::default();
        let mut manager = ClipboardManager::new(config).await.unwrap();

        let formats = vec![ClipboardFormat {
            format_id: 13,
            format_name: "CF_UNICODETEXT".to_string(),
        }];

        let event = ClipboardEvent::RdpFormatList(formats);
        manager.event_tx.send(event).await.unwrap();

        // Give event processor time to handle
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_portal_format_list_handling() {
        let config = ClipboardConfig::default();
        let mut manager = ClipboardManager::new(config).await.unwrap();

        let mime_types = vec!["text/plain".to_string()];

        let event = ClipboardEvent::PortalFormatsAvailable(mime_types);
        manager.event_tx.send(event).await.unwrap();

        // Give event processor time to handle
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_loop_detection_in_manager() {
        let config = ClipboardConfig {
            loop_detection_window_ms: 1000,
            ..Default::default()
        };
        let mut manager = ClipboardManager::new(config).await.unwrap();

        let formats = vec![ClipboardFormat {
            format_id: 13,
            format_name: "CF_UNICODETEXT".to_string(),
        }];

        // Send RDP format list
        manager
            .event_tx
            .send(ClipboardEvent::RdpFormatList(formats.clone()))
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Send Portal format list (corresponding MIME types)
        manager
            .event_tx
            .send(ClipboardEvent::PortalFormatsAvailable(vec![
                "text/plain".to_string()
            ]))
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Send RDP format list again - should be detected as loop
        manager
            .event_tx
            .send(ClipboardEvent::RdpFormatList(formats))
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_manager_shutdown() {
        let config = ClipboardConfig::default();
        let mut manager = ClipboardManager::new(config).await.unwrap();

        manager.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_large_data_handling() {
        let config = ClipboardConfig {
            max_data_size: 1024,
            ..Default::default()
        };
        let mut manager = ClipboardManager::new(config).await.unwrap();

        // Send data within limit
        let data = vec![0u8; 512];
        manager
            .event_tx
            .send(ClipboardEvent::RdpDataResponse(data))
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
