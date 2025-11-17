# TASK P1-03: RDP PROTOCOL FOUNDATION
**Task ID:** TASK-P1-03
**Phase:** 1
**Milestone:** RDP Protocol
**Duration:** 10-14 days
**Assigned To:** [Agent/Developer Name]
**Dependencies:** TASK-P1-02 (Security)
**Status:** NOT_STARTED

---

## TASK OVERVIEW

### Objective
Implement the RDP protocol foundation using IronRDP, including connection handling, capability negotiation, channel management, and session state machines. This forms the core RDP server functionality.

### Success Criteria
- ✅ Windows mstsc.exe connects successfully
- ✅ TLS handshake completes
- ✅ RDP capability exchange completes
- ✅ MCS channels join successfully
- ✅ Session enters ACTIVE state
- ✅ Connection persists without errors
- ✅ Multiple concurrent connections supported

### Deliverables
1. RDP server wrapper (`src/rdp/server.rs`)
2. Capability negotiation (`src/rdp/capabilities.rs`)
3. Channel management (`src/rdp/channels/mod.rs`)
4. Connection manager (`src/server/connection.rs`)
5. Session manager (`src/server/session.rs`)
6. Server coordinator (`src/server/mod.rs`)
7. Integration tests for RDP connection

---

## TECHNICAL SPECIFICATION

### 1. Study IronRDP

**FIRST STEP:** Before coding, study IronRDP examples:

```bash
git clone https://github.com/Devolutions/IronRDP
cd IronRDP
cargo doc --open
```

Review:
- Server examples (if available)
- PDU encoding/decoding
- Connection state machine
- Capability structures

---

### 2. RDP Server Module

#### File: `src/rdp/mod.rs`

```rust
//! RDP protocol implementation
//!
//! Wraps IronRDP to provide server-side RDP functionality.

pub mod server;
pub mod capabilities;
pub mod channels;
pub mod codec;

pub use server::{RdpServer, RdpSession};
pub use capabilities::CapabilityNegotiator;

/// RDP session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RdpState {
    /// Initial state
    Initial,
    /// TLS handshake in progress
    TlsHandshake,
    /// NLA authentication
    NlaAuth,
    /// X.224 connection
    X224Connection,
    /// MCS attach user
    McsAttach,
    /// Channel join
    ChannelJoin,
    /// Capability exchange
    CapabilityExchange,
    /// Active session
    Active,
    /// Suspended (reconnect possible)
    Suspended,
    /// Terminated
    Terminated,
}

/// Session ID type
pub type SessionId = uuid::Uuid;
```

#### File: `src/rdp/server.rs`

```rust
//! RDP server implementation using IronRDP
//!
//! Handles RDP protocol state machine and session management.

use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio_rustls::server::TlsStream;
use anyhow::{Result, Context};
use tracing::{info, debug, warn, error};

use super::{RdpState, SessionId};
use crate::config::Config;
use crate::video::EncodedFrame;
use crate::input::RdpInputEvent;

/// RDP session
pub struct RdpSession {
    /// Session ID
    session_id: SessionId,

    /// Current state
    state: Arc<RwLock<RdpState>>,

    /// Configuration
    config: Arc<Config>,

    /// Client address
    client_addr: std::net::SocketAddr,

    /// Username (after auth)
    username: Option<String>,

    // Channels for data flow
    /// Encoded video frames to send
    video_rx: mpsc::Receiver<EncodedFrame>,

    /// Input events received
    input_tx: mpsc::Sender<RdpInputEvent>,
}

impl RdpSession {
    /// Create new RDP session
    pub fn new(
        config: Arc<Config>,
        client_addr: std::net::SocketAddr,
    ) -> (Self, mpsc::Sender<EncodedFrame>, mpsc::Receiver<RdpInputEvent>) {
        let session_id = SessionId::new_v4();

        // Create channels
        let (video_tx, video_rx) = mpsc::channel(32);
        let (input_tx, input_rx) = mpsc::channel(64);

        let session = Self {
            session_id,
            state: Arc::new(RwLock::new(RdpState::Initial)),
            config,
            client_addr,
            username: None,
            video_rx,
            input_tx,
        };

        (session, video_tx, input_rx)
    }

    /// Get session ID
    pub fn id(&self) -> SessionId {
        self.session_id
    }

    /// Get current state
    pub async fn state(&self) -> RdpState {
        *self.state.read().await
    }

    /// Handle RDP connection
    pub async fn handle_connection(mut self, stream: TlsStream<TcpStream>) -> Result<()> {
        info!("Handling RDP connection from {}", self.client_addr);

        // Update state
        self.set_state(RdpState::TlsHandshake).await;

        // TODO: Integrate IronRDP here
        // This is a placeholder showing the structure

        // 1. X.224 Connection Request/Confirm
        self.handle_x224_connection().await?;

        // 2. MCS Connect Initial/Response
        self.handle_mcs_connect().await?;

        // 3. MCS Attach User
        self.handle_mcs_attach().await?;

        // 4. Channel Join
        self.handle_channel_join().await?;

        // 5. Capability Exchange
        self.handle_capability_exchange().await?;

        // 6. Connection Finalization
        self.handle_connection_finalization().await?;

        // 7. Enter active state
        self.set_state(RdpState::Active).await;
        info!("RDP session {} entered ACTIVE state", self.session_id);

        // 8. Main loop: handle incoming/outgoing data
        self.main_loop().await?;

        Ok(())
    }

    async fn set_state(&mut self, state: RdpState) {
        *self.state.write().await = state;
        debug!("Session {} state: {:?}", self.session_id, state);
    }

    async fn handle_x224_connection(&mut self) -> Result<()> {
        self.set_state(RdpState::X224Connection).await;
        // TODO: Implement X.224 protocol
        info!("X.224 connection established");
        Ok(())
    }

    async fn handle_mcs_connect(&mut self) -> Result<()> {
        // TODO: Implement MCS connection
        info!("MCS connection established");
        Ok(())
    }

    async fn handle_mcs_attach(&mut self) -> Result<()> {
        self.set_state(RdpState::McsAttach).await;
        // TODO: Implement MCS attach
        info!("MCS user attached");
        Ok(())
    }

    async fn handle_channel_join(&mut self) -> Result<()> {
        self.set_state(RdpState::ChannelJoin).await;
        // TODO: Implement channel join
        info!("Channels joined");
        Ok(())
    }

    async fn handle_capability_exchange(&mut self) -> Result<()> {
        self.set_state(RdpState::CapabilityExchange).await;
        // TODO: Implement capability exchange
        info!("Capabilities negotiated");
        Ok(())
    }

    async fn handle_connection_finalization(&mut self) -> Result<()> {
        // TODO: Implement connection finalization
        info!("Connection finalized");
        Ok(())
    }

    async fn main_loop(&mut self) -> Result<()> {
        info!("Entering main RDP loop");

        loop {
            tokio::select! {
                // Receive video frame to send
                Some(frame) = self.video_rx.recv() => {
                    self.send_video_frame(frame).await?;
                }

                // TODO: Receive RDP PDUs from client
                // Parse input events, forward to input_tx

                // Check for shutdown
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                    // Placeholder for actual event handling
                }
            }
        }
    }

    async fn send_video_frame(&mut self, frame: EncodedFrame) -> Result<()> {
        // TODO: Encode frame into RDP PDU and send
        debug!("Sending video frame {}", frame.sequence);
        Ok(())
    }
}

/// RDP server coordinator
pub struct RdpServer {
    config: Arc<Config>,
}

impl RdpServer {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    pub async fn create_session(
        &self,
        client_addr: std::net::SocketAddr,
    ) -> (RdpSession, mpsc::Sender<EncodedFrame>, mpsc::Receiver<RdpInputEvent>) {
        RdpSession::new(self.config.clone(), client_addr)
    }
}
```

---

### 3. Capability Negotiation

#### File: `src/rdp/capabilities.rs`

```rust
//! RDP capability negotiation
//!
//! Handles advertising and negotiating RDP capabilities with clients.

use anyhow::Result;
use tracing::info;

/// Capability set types
#[derive(Debug, Clone, Copy)]
pub enum CapabilityType {
    General,
    Bitmap,
    Order,
    BitmapCache,
    Pointer,
    Input,
    Brush,
    GlyphCache,
    OffscreenCache,
    VirtualChannel,
    Sound,
    /// Graphics Pipeline (for H.264)
    GraphicsPipeline,
}

/// Capability negotiator
pub struct CapabilityNegotiator {
    /// Supported capabilities
    capabilities: Vec<CapabilityType>,
}

impl CapabilityNegotiator {
    /// Create new capability negotiator
    pub fn new() -> Self {
        let capabilities = vec![
            CapabilityType::General,
            CapabilityType::Bitmap,
            CapabilityType::Input,
            CapabilityType::Pointer,
            CapabilityType::GraphicsPipeline, // For H.264
            CapabilityType::VirtualChannel,
        ];

        Self { capabilities }
    }

    /// Advertise server capabilities
    pub fn advertise_capabilities(&self) -> Result<Vec<u8>> {
        info!("Advertising server capabilities");

        // TODO: Encode capabilities into proper RDP PDU format
        // This requires IronRDP capability structures

        // For now, return empty vec
        Ok(Vec::new())
    }

    /// Negotiate capabilities with client
    pub fn negotiate(&mut self, client_caps: &[u8]) -> Result<()> {
        info!("Negotiating capabilities with client");

        // TODO: Parse client capabilities
        // TODO: Find common capabilities
        // TODO: Configure session based on negotiated caps

        Ok(())
    }

    /// Check if H.264 is supported
    pub fn supports_h264(&self) -> bool {
        self.capabilities.contains(&CapabilityType::GraphicsPipeline)
    }
}

impl Default for CapabilityNegotiator {
    fn default() -> Self {
        Self::new()
    }
}
```

---

### 4. Channel Management

#### File: `src/rdp/channels/mod.rs`

```rust
//! RDP channel management
//!
//! Manages virtual channels for graphics, input, clipboard, etc.

pub mod graphics;
pub mod input;
pub mod clipboard;

use anyhow::Result;

/// Channel ID type
pub type ChannelId = u16;

/// Virtual channel trait
pub trait Channel: Send + Sync {
    /// Get channel ID
    fn id(&self) -> ChannelId;

    /// Get channel name
    fn name(&self) -> &str;

    /// Handle incoming PDU
    fn handle_pdu(&mut self, data: &[u8]) -> Result<()>;
}

/// Channel manager
pub struct ChannelManager {
    channels: std::collections::HashMap<ChannelId, Box<dyn Channel>>,
    next_id: ChannelId,
}

impl ChannelManager {
    pub fn new() -> Self {
        Self {
            channels: std::collections::HashMap::new(),
            next_id: 1000, // Start from 1000 for user channels
        }
    }

    /// Register a channel
    pub fn register(&mut self, channel: Box<dyn Channel>) {
        let id = channel.id();
        self.channels.insert(id, channel);
    }

    /// Get channel by ID
    pub fn get(&self, id: ChannelId) -> Option<&dyn Channel> {
        self.channels.get(&id).map(|b| &**b)
    }

    /// Get mutable channel by ID
    pub fn get_mut(&mut self, id: ChannelId) -> Option<&mut dyn Channel> {
        self.channels.get_mut(&id).map(|b| &mut **b)
    }

    /// Route PDU to appropriate channel
    pub fn route_pdu(&mut self, channel_id: ChannelId, data: &[u8]) -> Result<()> {
        if let Some(channel) = self.get_mut(channel_id) {
            channel.handle_pdu(data)?;
        }
        Ok(())
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}
```

---

### 5. Server Connection Manager

#### File: `src/server/connection.rs`

```rust
//! Connection manager
//!
//! Manages all active RDP connections.

use std::sync::Arc;
use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use anyhow::Result;
use tracing::{info, error};

use crate::config::Config;
use crate::security::SecurityManager;
use crate::portal::PortalManager;
use crate::rdp::{RdpServer, RdpSession, SessionId};

/// Connection manager
pub struct ConnectionManager {
    config: Arc<Config>,
    portal_manager: Arc<PortalManager>,
    security_manager: Arc<SecurityManager>,
    rdp_server: Arc<RdpServer>,
    sessions: Arc<RwLock<HashMap<SessionId, RdpSession>>>,
}

impl ConnectionManager {
    pub fn new(
        config: Arc<Config>,
        portal_manager: Arc<PortalManager>,
        security_manager: Arc<SecurityManager>,
    ) -> Self {
        let rdp_server = Arc::new(RdpServer::new(config.clone()));

        Self {
            config,
            portal_manager,
            security_manager,
            rdp_server,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Handle incoming connection
    pub async fn handle_connection(
        &self,
        stream: TcpStream,
        addr: std::net::SocketAddr,
    ) -> Result<()> {
        info!("Handling connection from {}", addr);

        // Wrap in TLS
        let acceptor = self.security_manager.create_acceptor();
        let tls_stream = tokio_rustls::TlsAcceptor::from(Arc::new(acceptor.accept()))
            .accept(stream)
            .await?;

        // Create RDP session
        let (session, video_tx, input_rx) = self.rdp_server.create_session(addr).await;
        let session_id = session.id();

        // Store session
        self.sessions.write().await.insert(session_id, session);

        // Handle connection (this will block until session ends)
        // TODO: Get session back from HashMap and run it
        // session.handle_connection(tls_stream).await?;

        // Remove session
        self.sessions.write().await.remove(&session_id);

        info!("Session {} ended", session_id);

        Ok(())
    }

    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Close all connections
    pub async fn close_all(&self) -> Result<()> {
        info!("Closing all connections");
        self.sessions.write().await.clear();
        Ok(())
    }
}
```

---

### 6. Session Management

#### File: `src/server/session.rs`

```rust
//! Session state management

use crate::rdp::{RdpState, SessionId};
use std::time::{Duration, Instant};

/// Session state
pub struct Session {
    pub id: SessionId,
    pub state: RdpState,
    pub username: Option<String>,
    pub connected_at: Instant,
    pub last_activity: Instant,
}

impl Session {
    pub fn new(id: SessionId) -> Self {
        let now = Instant::now();
        Self {
            id,
            state: RdpState::Initial,
            username: None,
            connected_at: now,
            last_activity: now,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, RdpState::Active)
    }

    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.last_activity.elapsed() > timeout
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }
}
```

---

### 7. Server Module Coordinator

#### File: `src/server/mod.rs`

```rust
//! Server coordination and lifecycle management

use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::net::TcpListener;
use tracing::{info, warn, error, debug};

pub mod connection;
pub mod session;
pub mod lifecycle;

pub use connection::ConnectionManager;
pub use session::Session;

use crate::config::Config;
use crate::portal::PortalManager;
use crate::security::SecurityManager;

/// Main server structure
pub struct Server {
    config: Arc<Config>,
    listener: TcpListener,
    connection_manager: Arc<ConnectionManager>,
    portal_manager: Arc<PortalManager>,
    security_manager: Arc<SecurityManager>,
    shutdown_tx: broadcast::Sender<()>,
    shutdown_rx: broadcast::Receiver<()>,
}

impl Server {
    /// Create new server instance
    pub async fn new(config: Config) -> Result<Self> {
        let config = Arc::new(config);

        info!("Initializing WRD-Server...");

        // Bind TCP listener
        let listener = TcpListener::bind(&config.server.listen_addr)
            .await
            .context(format!("Failed to bind to {}", config.server.listen_addr))?;

        info!("Listening on {}", config.server.listen_addr);

        // Initialize security manager
        let security_manager = Arc::new(SecurityManager::new(&config).await?);

        // Initialize portal manager (D-Bus connection)
        let portal_manager = Arc::new(PortalManager::new(&config).await?);

        // Initialize connection manager
        let connection_manager = Arc::new(ConnectionManager::new(
            config.clone(),
            portal_manager.clone(),
            security_manager.clone(),
        ));

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

        Ok(Server {
            config,
            listener,
            connection_manager,
            portal_manager,
            security_manager,
            shutdown_tx,
            shutdown_rx,
        })
    }

    /// Run the server
    pub async fn run(mut self) -> Result<()> {
        info!("WRD-Server running");

        loop {
            tokio::select! {
                // Accept new connections
                result = self.listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            info!("New connection from {}", addr);

                            // Check connection limit
                            if self.connection_manager.connection_count().await
                                >= self.config.server.max_connections {
                                warn!("Connection limit reached, rejecting {}", addr);
                                drop(stream);
                                continue;
                            }

                            // Spawn connection handler
                            let manager = self.connection_manager.clone();
                            let mut shutdown = self.shutdown_tx.subscribe();

                            tokio::spawn(async move {
                                tokio::select! {
                                    result = manager.handle_connection(stream, addr) => {
                                        if let Err(e) = result {
                                            error!("Connection error: {}", e);
                                        }
                                    }
                                    _ = shutdown.recv() => {
                                        debug!("Shutting down connection handler");
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                        }
                    }
                }

                // Shutdown signal
                _ = self.shutdown_rx.recv() => {
                    info!("Shutdown initiated");
                    break;
                }
            }
        }

        // Graceful shutdown
        self.shutdown().await?;

        Ok(())
    }

    /// Graceful shutdown
    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down server...");

        // Close all connections
        self.connection_manager.close_all().await?;

        // Cleanup portal resources
        self.portal_manager.cleanup().await?;

        info!("Server shutdown complete");
        Ok(())
    }
}
```

---

## VERIFICATION CHECKLIST

- [ ] `cargo build` succeeds
- [ ] All tests pass
- [ ] RDP server compiles with IronRDP
- [ ] Connection state machine implemented
- [ ] Capability negotiation framework ready
- [ ] Channel management working
- [ ] Session tracking functional
- [ ] Connection limits enforced
- [ ] Graceful shutdown works

---

## INTEGRATION NOTES

**Critical:** This task provides the RDP protocol foundation. The actual IronRDP integration requires:

1. Study IronRDP API documentation
2. Implement PDU encoding/decoding
3. Wire up state machine transitions
4. Complete channel implementations (done in later tasks)

The code provided is a **complete structural framework**. IronRDP-specific code needs to be added in the TODO sections.

---

## COMPLETION CRITERIA

This task is COMPLETE when:
1. Windows mstsc connects successfully
2. TLS handshake works
3. Connection reaches ACTIVE state
4. Session persists without errors
5. Multiple connections supported

**Time Estimate:** 10-14 days

---

**END OF TASK SPECIFICATION**
