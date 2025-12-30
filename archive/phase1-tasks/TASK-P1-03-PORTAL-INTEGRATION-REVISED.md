# TASK P1-03: PORTAL INTEGRATION (REVISED)
**Task ID:** TASK-P1-03-REVISED
**Phase:** 1
**Milestone:** Portal Integration
**Duration:** 5-7 days
**Assigned To:** [CCW Session or Developer]
**Dependencies:** TASK-P1-01 (Foundation) ✅, TASK-P1-02 (Security) ✅
**Status:** READY TO START

---

## TASK OVERVIEW

### Objective
Implement complete xdg-desktop-portal integration using the `ashpd` crate. This provides access to the compositor's screen content (via ScreenCast), input injection (via RemoteDesktop), and clipboard. This is a **standalone, testable module** that other components will use.

### Why This Task First
- Portal integration is the **foundation** for video and input
- It's **independent** of RDP protocol complexity
- It's **testable** standalone (you can verify you get a PipeWire FD)
- `ashpd` is a **mature library** with good documentation
- No complex protocol handling needed yet

### Success Criteria
- ✅ D-Bus session connection established
- ✅ Portal session creates successfully
- ✅ User permission dialog appears on desktop
- ✅ PipeWire file descriptor obtained (valid FD > 0)
- ✅ Stream metadata extracted (resolution, position, etc.)
- ✅ Multiple monitors detected correctly
- ✅ Input injection methods work (keyboard and pointer via portal)
- ✅ Clipboard portal accessible
- ✅ All unit tests pass
- ✅ Integration test demonstrates full flow

### Deliverables
1. Portal manager module (`src/portal/mod.rs`)
2. ScreenCast manager (`src/portal/screencast.rs`)
3. RemoteDesktop manager (`src/portal/remote_desktop.rs`)
4. Clipboard manager (`src/portal/clipboard.rs`)
5. Session management (`src/portal/session.rs`)
6. Integration test that creates portal session
7. Example program that dumps portal information

---

## TECHNICAL SPECIFICATION

### 1. Module Structure

```
src/portal/
├── mod.rs              # Portal manager coordinator
├── screencast.rs       # ScreenCast portal wrapper
├── remote_desktop.rs   # RemoteDesktop portal wrapper
├── clipboard.rs        # Clipboard portal wrapper
└── session.rs          # Portal session lifecycle
```

### 2. Data Structures

First, define the data structures we'll use:

#### File: `src/portal/session.rs`

```rust
//! Portal session management
//!
//! Manages the lifecycle of portal sessions and associated resources.

use std::os::fd::RawFd;
use anyhow::Result;
use tracing::{info, debug};

/// Information about a PipeWire stream from the portal
#[derive(Debug, Clone)]
pub struct StreamInfo {
    /// PipeWire node ID
    pub node_id: u32,

    /// Stream position (for multi-monitor)
    pub position: (i32, i32),

    /// Stream size
    pub size: (u32, u32),

    /// Source type (monitor, window, etc.)
    pub source_type: SourceType,
}

/// Source type for streams
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Monitor,
    Window,
    Virtual,
}

/// Handle to an active portal session
pub struct PortalSessionHandle {
    /// Session identifier from portal
    session_id: String,

    /// PipeWire file descriptor
    pipewire_fd: RawFd,

    /// Available streams (one per monitor typically)
    streams: Vec<StreamInfo>,

    /// RemoteDesktop session for input injection
    remote_desktop_session: Option<String>,
}

impl PortalSessionHandle {
    /// Create new session handle
    pub fn new(
        session_id: String,
        pipewire_fd: RawFd,
        streams: Vec<StreamInfo>,
        remote_desktop_session: Option<String>,
    ) -> Self {
        info!("Created portal session handle: {}, {} streams, fd: {}",
              session_id, streams.len(), pipewire_fd);

        Self {
            session_id,
            pipewire_fd,
            streams,
            remote_desktop_session,
        }
    }

    /// Get PipeWire file descriptor
    pub fn pipewire_fd(&self) -> RawFd {
        self.pipewire_fd
    }

    /// Get stream information
    pub fn streams(&self) -> &[StreamInfo] {
        &self.streams
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get remote desktop session (for input injection)
    pub fn remote_desktop_session(&self) -> Option<&str> {
        self.remote_desktop_session.as_deref()
    }

    /// Close the portal session
    pub async fn close(self) -> Result<()> {
        info!("Closing portal session: {}", self.session_id);

        // Close PipeWire FD
        unsafe {
            libc::close(self.pipewire_fd);
        }

        // Portal sessions are automatically cleaned up when dropped

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_handle_creation() {
        let streams = vec![
            StreamInfo {
                node_id: 42,
                position: (0, 0),
                size: (1920, 1080),
                source_type: SourceType::Monitor,
            }
        ];

        let handle = PortalSessionHandle::new(
            "test-session".to_string(),
            3,
            streams.clone(),
            Some("rd-session".to_string()),
        );

        assert_eq!(handle.session_id(), "test-session");
        assert_eq!(handle.pipewire_fd(), 3);
        assert_eq!(handle.streams().len(), 1);
        assert_eq!(handle.remote_desktop_session(), Some("rd-session"));
    }
}
```

---

### 3. ScreenCast Portal Manager

#### File: `src/portal/screencast.rs`

```rust
//! ScreenCast portal integration
//!
//! Provides access to screen content via xdg-desktop-portal ScreenCast interface.

use ashpd::desktop::screencast::{ScreenCast, SourceType, CursorMode};
use ashpd::WindowIdentifier;
use std::sync::Arc;
use std::os::fd::{AsRawFd, RawFd};
use anyhow::{Result, Context};
use tracing::{info, debug, error};

use crate::config::Config;
use super::session::StreamInfo;

/// ScreenCast portal manager
pub struct ScreenCastManager {
    connection: zbus::Connection,
    config: Arc<Config>,
}

impl ScreenCastManager {
    /// Create new ScreenCast manager
    pub async fn new(connection: zbus::Connection, config: Arc<Config>) -> Result<Self> {
        info!("Initializing ScreenCast portal manager");
        Ok(Self { connection, config })
    }

    /// Create a screencast session
    pub async fn create_session(&self) -> Result<ashpd::desktop::Request<ashpd::desktop::screencast::CreateSessionResponse>> {
        info!("Creating ScreenCast session");

        let proxy = ScreenCast::new(&self.connection).await?;
        let session = proxy.create_session().await?;

        debug!("ScreenCast session created");
        Ok(session)
    }

    /// Select sources (monitors, windows, etc.)
    pub async fn select_sources(
        &self,
        session_handle: &str,
    ) -> Result<()> {
        info!("Selecting screencast sources");

        let proxy = ScreenCast::new(&self.connection).await?;

        // Parse cursor mode from config
        let cursor_mode = match self.config.video.cursor_mode.as_str() {
            "hidden" => CursorMode::Hidden,
            "embedded" => CursorMode::Embedded,
            "metadata" => CursorMode::Metadata,
            _ => CursorMode::Metadata,
        };

        // Select sources: monitors + windows if available
        let source_types = SourceType::Monitor | SourceType::Window;

        proxy.select_sources(
            session_handle,
            cursor_mode,
            source_types,
            true,  // multiple sources
            None,  // no restore token yet
        ).await.context("Failed to select sources")?;

        info!("Sources selected successfully");
        Ok(())
    }

    /// Start the screencast and get PipeWire details
    pub async fn start(
        &self,
        session_handle: &str,
    ) -> Result<(RawFd, Vec<StreamInfo>)> {
        info!("Starting screencast session");

        let proxy = ScreenCast::new(&self.connection).await?;

        let streams = proxy.start(session_handle, &WindowIdentifier::default())
            .await
            .context("Failed to start screencast")?;

        info!("Screencast started with {} streams", streams.streams.len());

        // Get PipeWire FD
        let fd = proxy.open_pipe_wire_remote(session_handle)
            .await
            .context("Failed to open PipeWire remote")?;

        let raw_fd = fd.as_raw_fd();
        info!("PipeWire FD obtained: {}", raw_fd);

        // Convert stream info
        let stream_info: Vec<StreamInfo> = streams.streams.iter().map(|(node_id, properties)| {
            StreamInfo {
                node_id: *node_id,
                position: properties.position.unwrap_or((0, 0)),
                size: properties.size,
                source_type: super::session::SourceType::Monitor, // Simplified for now
            }
        }).collect();

        // Don't close fd - we need to keep it
        std::mem::forget(fd);

        Ok((raw_fd, stream_info))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Portal tests require a running Wayland session with portal
    // These are integration tests that may not work in CI

    #[tokio::test]
    #[ignore] // Ignore in CI, run manually
    async fn test_screencast_manager_creation() {
        let connection = zbus::Connection::session().await.unwrap();
        let config = Arc::new(Config::default_config().unwrap());

        let manager = ScreenCastManager::new(connection, config).await;
        assert!(manager.is_ok());
    }
}
```

---

### 4. RemoteDesktop Portal Manager

#### File: `src/portal/remote_desktop.rs`

```rust
//! RemoteDesktop portal integration
//!
//! Provides input injection and screen capture via RemoteDesktop portal.

use ashpd::desktop::remote_desktop::{RemoteDesktop, DeviceType, Axis};
use ashpd::WindowIdentifier;
use std::sync::Arc;
use std::os::fd::{AsRawFd, RawFd};
use anyhow::{Result, Context};
use tracing::{info, debug, warn};

use crate::config::Config;
use super::session::StreamInfo;

/// RemoteDesktop portal manager
pub struct RemoteDesktopManager {
    connection: zbus::Connection,
    config: Arc<Config>,
    session_handle: Arc<tokio::sync::RwLock<Option<String>>>,
}

impl RemoteDesktopManager {
    /// Create new RemoteDesktop manager
    pub async fn new(connection: zbus::Connection, config: Arc<Config>) -> Result<Self> {
        info!("Initializing RemoteDesktop portal manager");
        Ok(Self {
            connection,
            config,
            session_handle: Arc::new(tokio::sync::RwLock::new(None)),
        })
    }

    /// Create a remote desktop session
    pub async fn create_session(&self) -> Result<String> {
        info!("Creating RemoteDesktop session");

        let proxy = RemoteDesktop::new(&self.connection).await?;
        let session = proxy.create_session().await?;

        let session_handle = session.to_string();
        debug!("RemoteDesktop session created: {}", session_handle);

        // Store session handle
        *self.session_handle.write().await = Some(session_handle.clone());

        Ok(session_handle)
    }

    /// Select devices for remote control
    pub async fn select_devices(
        &self,
        session_handle: &str,
        devices: DeviceType,
    ) -> Result<()> {
        info!("Selecting devices: {:?}", devices);

        let proxy = RemoteDesktop::new(&self.connection).await?;

        proxy.select_devices(
            session_handle,
            devices,
            None, // No restore token yet
        ).await.context("Failed to select devices")?;

        info!("Devices selected successfully");
        Ok(())
    }

    /// Start the remote desktop session
    pub async fn start_session(
        &self,
        session_handle: &str,
    ) -> Result<(RawFd, Vec<StreamInfo>)> {
        info!("Starting RemoteDesktop session");

        let proxy = RemoteDesktop::new(&self.connection).await?;

        let response = proxy.start(session_handle, &WindowIdentifier::default())
            .await
            .context("Failed to start remote desktop session")?;

        info!("RemoteDesktop started with {} streams", response.streams.len());

        // Get PipeWire FD
        let fd = proxy.open_pipe_wire_remote(session_handle)
            .await
            .context("Failed to open PipeWire remote")?;

        let raw_fd = fd.as_raw_fd();
        info!("PipeWire FD obtained: {}", raw_fd);

        // Convert stream info
        let stream_info: Vec<StreamInfo> = response.streams.iter().map(|(node_id, properties)| {
            StreamInfo {
                node_id: *node_id,
                position: properties.position.unwrap_or((0, 0)),
                size: properties.size,
                source_type: super::session::SourceType::Monitor,
            }
        }).collect();

        // Don't close fd - we need to keep it
        std::mem::forget(fd);

        Ok((raw_fd, stream_info))
    }

    /// Inject pointer motion (relative)
    pub async fn notify_pointer_motion(&self, dx: f64, dy: f64) -> Result<()> {
        let session = self.session_handle.read().await;
        let session_handle = session.as_ref()
            .context("No active session")?;

        let proxy = RemoteDesktop::new(&self.connection).await?;
        proxy.notify_pointer_motion(session_handle, dx, dy).await?;

        Ok(())
    }

    /// Inject pointer motion (absolute in stream coordinates)
    pub async fn notify_pointer_motion_absolute(&self, stream: u32, x: f64, y: f64) -> Result<()> {
        let session = self.session_handle.read().await;
        let session_handle = session.as_ref()
            .context("No active session")?;

        let proxy = RemoteDesktop::new(&self.connection).await?;
        proxy.notify_pointer_motion_absolute(session_handle, stream, x, y).await?;

        Ok(())
    }

    /// Inject pointer button
    pub async fn notify_pointer_button(&self, button: i32, pressed: bool) -> Result<()> {
        let session = self.session_handle.read().await;
        let session_handle = session.as_ref()
            .context("No active session")?;

        let proxy = RemoteDesktop::new(&self.connection).await?;

        let state = if pressed { 1u32 } else { 0u32 };
        proxy.notify_pointer_button(session_handle, button, state).await?;

        Ok(())
    }

    /// Inject pointer axis (scroll)
    pub async fn notify_pointer_axis(&self, dx: f64, dy: f64) -> Result<()> {
        let session = self.session_handle.read().await;
        let session_handle = session.as_ref()
            .context("No active session")?;

        let proxy = RemoteDesktop::new(&self.connection).await?;

        if dy != 0.0 {
            proxy.notify_pointer_axis(session_handle, dy, Axis::Vertical).await?;
        }
        if dx != 0.0 {
            proxy.notify_pointer_axis(session_handle, dx, Axis::Horizontal).await?;
        }

        Ok(())
    }

    /// Inject keyboard key
    pub async fn notify_keyboard_keycode(&self, keycode: i32, pressed: bool) -> Result<()> {
        let session = self.session_handle.read().await;
        let session_handle = session.as_ref()
            .context("No active session")?;

        let proxy = RemoteDesktop::new(&self.connection).await?;

        let state = if pressed { 1u32 } else { 0u32 };
        proxy.notify_keyboard_keycode(session_handle, keycode, state).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_remote_desktop_session_creation() {
        let connection = zbus::Connection::session().await.unwrap();
        let config = Arc::new(Config::default_config().unwrap());

        let manager = RemoteDesktopManager::new(connection, config).await.unwrap();

        // This will trigger permission dialog
        // let session = manager.create_session().await;
        // assert!(session.is_ok());
    }
}
```

---

### 5. Clipboard Portal Manager

#### File: `src/portal/clipboard.rs`

```rust
//! Clipboard portal integration

use ashpd::desktop::clipboard::Clipboard;
use std::sync::Arc;
use anyhow::Result;
use tracing::{info, debug};

use crate::config::Config;

/// Clipboard portal manager
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

    /// Request clipboard content
    /// Note: This requires a session handle from SelectionWrite/SelectionRead
    pub async fn request_clipboard(&self, session_handle: &str, mime_type: &str) -> Result<Vec<u8>> {
        debug!("Requesting clipboard content: {}", mime_type);

        // Note: ashpd Clipboard API may have different structure
        // This is a placeholder for the actual implementation

        Ok(Vec::new())
    }

    /// Set clipboard content
    pub async fn set_clipboard(&self, session_handle: &str, mime_type: &str, data: &[u8]) -> Result<()> {
        debug!("Setting clipboard content: {} ({} bytes)", mime_type, data.len());

        // Validate size
        if data.len() > self.config.clipboard.max_size {
            anyhow::bail!("Clipboard data exceeds maximum size");
        }

        // Note: Actual clipboard portal integration
        // This is a placeholder

        Ok(())
    }
}
```

---

### 6. Portal Manager Coordinator

#### File: `src/portal/mod.rs`

```rust
//! XDG Desktop Portal integration
//!
//! Provides unified access to ScreenCast, RemoteDesktop, and Clipboard portals.

use std::sync::Arc;
use anyhow::{Result, Context};
use tracing::{info, debug, error};
use ashpd::desktop::remote_desktop::DeviceType;

pub mod screencast;
pub mod remote_desktop;
pub mod clipboard;
pub mod session;

pub use screencast::ScreenCastManager;
pub use remote_desktop::RemoteDesktopManager;
pub use clipboard::ClipboardManager;
pub use session::{PortalSessionHandle, StreamInfo, SourceType};

use crate::config::Config;

/// Portal manager coordinates all portal interactions
pub struct PortalManager {
    config: Arc<Config>,
    connection: zbus::Connection,
    screencast: Arc<ScreenCastManager>,
    remote_desktop: Arc<RemoteDesktopManager>,
    clipboard: Arc<ClipboardManager>,
}

impl PortalManager {
    /// Create new portal manager
    pub async fn new(config: &Arc<Config>) -> Result<Self> {
        info!("Initializing Portal Manager");

        // Connect to session D-Bus
        let connection = zbus::Connection::session()
            .await
            .context("Failed to connect to D-Bus session bus")?;

        debug!("Connected to D-Bus session bus");

        // Initialize portal managers
        let screencast = Arc::new(ScreenCastManager::new(
            connection.clone(),
            config.clone(),
        ).await?);

        let remote_desktop = Arc::new(RemoteDesktopManager::new(
            connection.clone(),
            config.clone(),
        ).await?);

        let clipboard = Arc::new(ClipboardManager::new(
            connection.clone(),
            config.clone(),
        ).await?);

        info!("Portal Manager initialized successfully");

        Ok(Self {
            config: config.clone(),
            connection,
            screencast,
            remote_desktop,
            clipboard,
        })
    }

    /// Create a complete portal session (RemoteDesktop + ScreenCast)
    ///
    /// This triggers the user permission dialog and returns a session handle
    /// with PipeWire access for video and input injection capabilities.
    pub async fn create_session(&self) -> Result<PortalSessionHandle> {
        info!("Creating portal session (RemoteDesktop + ScreenCast)");

        // Create RemoteDesktop session (this includes ScreenCast capabilities)
        let session_handle = self.remote_desktop.create_session().await?;

        // Select devices for input injection
        let devices = DeviceType::Keyboard | DeviceType::Pointer;
        self.remote_desktop.select_devices(&session_handle, devices).await?;

        // Note: We can also use ScreenCast directly for screen-only capture
        // But RemoteDesktop gives us both screen + input

        // Start the session (triggers permission dialog)
        let (pipewire_fd, streams) = self.remote_desktop.start_session(&session_handle).await?;

        info!("Portal session created successfully");
        debug!("  Session: {}", session_handle);
        debug!("  PipeWire FD: {}", pipewire_fd);
        debug!("  Streams: {}", streams.len());

        // Create session handle
        let handle = PortalSessionHandle::new(
            session_handle.clone(),
            pipewire_fd,
            streams,
            Some(session_handle),
        );

        Ok(handle)
    }

    /// Access screencast manager
    pub fn screencast(&self) -> &Arc<ScreenCastManager> {
        &self.screencast
    }

    /// Access remote desktop manager
    pub fn remote_desktop(&self) -> &Arc<RemoteDesktopManager> {
        &self.remote_desktop
    }

    /// Access clipboard manager
    pub fn clipboard(&self) -> &Arc<ClipboardManager> {
        &self.clipboard
    }

    /// Cleanup all portal resources
    pub async fn cleanup(&self) -> Result<()> {
        info!("Cleaning up portal resources");
        // Portal sessions are automatically cleaned up
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Wayland session
    async fn test_portal_manager_creation() {
        let config = Arc::new(Config::default_config().unwrap());
        let manager = PortalManager::new(&config).await;

        // May fail if not in Wayland session or portal not available
        if manager.is_err() {
            eprintln!("Portal manager creation failed (expected if not in Wayland session)");
        }
    }
}
```

---

### 7. Update lib.rs

Add portal module to `src/lib.rs`:
```rust
pub mod portal;
```

---

### 8. Create Integration Test

#### File: `tests/integration/portal_test.rs`

```rust
//! Integration test for portal functionality
//!
//! NOTE: This test requires a running Wayland session with xdg-desktop-portal
//! Run manually with: cargo test --test portal_test -- --ignored --nocapture

use wrd_server::portal::PortalManager;
use wrd_server::config::Config;
use std::sync::Arc;

#[tokio::test]
#[ignore] // Only run manually in Wayland session
async fn test_create_portal_session() {
    // Initialize logging for test
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init();

    println!("Creating portal session...");
    println!("NOTE: This will show a permission dialog on your desktop.");
    println!("Please APPROVE the request to continue.");

    // Create config
    let config = Arc::new(Config::default_config().unwrap());

    // Create portal manager
    let portal_manager = PortalManager::new(&config)
        .await
        .expect("Failed to create portal manager");

    println!("Portal manager created successfully");

    // Create session (this will show permission dialog)
    let session = portal_manager.create_session()
        .await
        .expect("Failed to create portal session");

    println!("Portal session created!");
    println!("  Session ID: {}", session.session_id());
    println!("  PipeWire FD: {}", session.pipewire_fd());
    println!("  Streams: {}", session.streams().len());

    // Verify FD is valid
    assert!(session.pipewire_fd() > 0, "PipeWire FD should be positive");

    // Verify we have at least one stream
    assert!(!session.streams().is_empty(), "Should have at least one stream");

    // Print stream details
    for (i, stream) in session.streams().iter().enumerate() {
        println!("  Stream {}: node_id={}, size={}x{}, pos=({},{})",
            i, stream.node_id, stream.size.0, stream.size.1,
            stream.position.0, stream.position.1);
    }

    // Test input injection
    println!("\nTesting input injection...");
    let rd_manager = portal_manager.remote_desktop();

    // Move mouse (relative)
    rd_manager.notify_pointer_motion(10.0, 10.0)
        .await
        .expect("Failed to inject pointer motion");
    println!("  ✓ Pointer motion injected");

    // Press and release a key (keycode 28 = Enter on most systems)
    rd_manager.notify_keyboard_keycode(28, true)
        .await
        .expect("Failed to inject key press");
    rd_manager.notify_keyboard_keycode(28, false)
        .await
        .expect("Failed to inject key release");
    println!("  ✓ Keyboard input injected");

    println!("\n✅ All portal tests passed!");

    // Cleanup
    session.close().await.expect("Failed to close session");
    println!("Session closed");
}

#[tokio::test]
#[ignore]
async fn test_portal_input_injection() {
    let _ = tracing_subscriber::fmt().with_env_filter("debug").try_init();

    println!("Testing portal input injection...");

    let config = Arc::new(Config::default_config().unwrap());
    let portal_manager = PortalManager::new(&config).await.unwrap();
    let session = portal_manager.create_session().await.unwrap();

    // Test various input methods
    let rd = portal_manager.remote_desktop();

    // Mouse movements
    rd.notify_pointer_motion(5.0, 5.0).await.unwrap();
    rd.notify_pointer_motion(-5.0, -5.0).await.unwrap();

    // Mouse button
    rd.notify_pointer_button(272, true).await.unwrap(); // Left click press
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    rd.notify_pointer_button(272, false).await.unwrap(); // Left click release

    // Keyboard
    rd.notify_keyboard_keycode(57, true).await.unwrap(); // Space press
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    rd.notify_keyboard_keycode(57, false).await.unwrap(); // Space release

    println!("✅ Input injection test passed");

    session.close().await.unwrap();
}
```

---

### 9. Create Example Program

#### File: `examples/portal_info.rs`

```rust
//! Example program to test portal integration
//!
//! This demonstrates creating a portal session and displays information
//! about available streams.
//!
//! Run with: cargo run --example portal_info

use wrd_server::portal::PortalManager;
use wrd_server::config::Config;
use std::sync::Arc;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("=================================");
    println!("WRD-Server Portal Information");
    println!("=================================\n");

    println!("Creating portal manager...");
    let config = Arc::new(Config::default_config()?);
    let portal_manager = PortalManager::new(&config).await?;
    println!("✓ Portal manager created\n");

    println!("Creating portal session...");
    println!("⚠️  A permission dialog will appear on your desktop.");
    println!("    Please APPROVE the request to continue.\n");

    let session = portal_manager.create_session().await?;
    println!("✓ Portal session created!\n");

    println!("Session Information:");
    println!("  Session ID: {}", session.session_id());
    println!("  PipeWire FD: {}", session.pipewire_fd());
    println!("  Stream Count: {}\n", session.streams().len());

    println!("Available Streams:");
    for (i, stream) in session.streams().iter().enumerate() {
        println!("\n  Stream {}:", i);
        println!("    Node ID: {}", stream.node_id);
        println!("    Size: {}x{}", stream.size.0, stream.size.1);
        println!("    Position: ({}, {})", stream.position.0, stream.position.1);
        println!("    Type: {:?}", stream.source_type);
    }

    println!("\n=================================");
    println!("Testing input injection...");
    println!("=================================\n");

    let rd = portal_manager.remote_desktop();

    println!("Moving mouse cursor (you should see it move)...");
    for _ in 0..10 {
        rd.notify_pointer_motion(5.0, 0.0).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    println!("✓ Mouse moved\n");

    println!("Press Ctrl+C to exit and close session...");
    tokio::signal::ctrl_c().await?;

    println!("\nClosing session...");
    session.close().await?;
    println!("✓ Session closed\n");

    println!("=================================");
    println!("Portal test complete!");
    println!("=================================");

    Ok(())
}
```

---

## VERIFICATION CHECKLIST

### Build Verification
- [ ] `cargo build` succeeds without errors
- [ ] `cargo build --example portal_info` succeeds
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] No compiler warnings

### Test Verification
- [ ] `cargo test portal::` passes
- [ ] Unit tests for session handle pass
- [ ] Can run `cargo test --test portal_test -- --ignored` manually
- [ ] Integration test creates session successfully

### Functionality Verification (Manual - Requires Wayland)
- [ ] Run `cargo run --example portal_info`
- [ ] Permission dialog appears
- [ ] After approval, PipeWire FD obtained (positive number)
- [ ] At least one stream detected
- [ ] Stream metadata correct (resolution, position)
- [ ] Mouse injection works (cursor moves)
- [ ] Keyboard injection works
- [ ] Session closes cleanly

### Integration Verification
- [ ] Portal module compiles with rest of project
- [ ] Config integration works
- [ ] No circular dependencies
- [ ] Module is usable by future tasks

---

## COMMON ISSUES AND SOLUTIONS

### Issue: "Failed to connect to D-Bus session bus"
**Solution:**
```bash
# Verify D-Bus session bus is available
echo $DBUS_SESSION_BUS_ADDRESS

# If empty, you're not in a graphical session
# Portal integration requires a Wayland session
```

### Issue: "Permission dialog doesn't appear"
**Solution:**
```bash
# Check portal is running
systemctl --user status xdg-desktop-portal

# Check compositor-specific backend
systemctl --user status xdg-desktop-portal-gnome  # GNOME
systemctl --user status xdg-desktop-portal-kde    # KDE
systemctl --user status xdg-desktop-portal-wlr    # Sway

# Start if needed
systemctl --user start xdg-desktop-portal
```

### Issue: "Failed to open PipeWire remote"
**Solution:**
```bash
# Check PipeWire is running
systemctl --user status pipewire

# Check version
pipewire --version  # Need 0.3.77+

# Start PipeWire
systemctl --user start pipewire
```

### Issue: "ashpd API doesn't match examples"
**Solution:**
- Check ashpd version (should be 0.12.0)
- Review ashpd docs: https://docs.rs/ashpd/latest/ashpd/
- ashpd API may have changed - adapt code to match actual API

---

## DELIVERABLE CHECKLIST

- [ ] `src/portal/mod.rs` implemented
- [ ] `src/portal/screencast.rs` implemented
- [ ] `src/portal/remote_desktop.rs` implemented
- [ ] `src/portal/clipboard.rs` implemented
- [ ] `src/portal/session.rs` implemented
- [ ] Integration test created
- [ ] Example program created
- [ ] All unit tests passing
- [ ] Manual verification complete (in Wayland session)
- [ ] Documentation complete (rustdoc)
- [ ] Code reviewed

---

## COMPLETION CRITERIA

This task is COMPLETE when:

1. ✅ All portal managers compile and work
2. ✅ Can create portal session programmatically
3. ✅ User permission dialog appears
4. ✅ PipeWire FD obtained successfully
5. ✅ Stream information extracted correctly
6. ✅ Input injection methods work (mouse and keyboard)
7. ✅ All tests pass
8. ✅ Example program runs successfully
9. ✅ Code reviewed and approved

**Estimated Time:** 5-7 days for experienced Rust developer
**AI Agent Time:** 5-7 days (this task is well-scoped)

---

## HANDOFF TO NEXT TASK

### What This Task Provides

**Exports for future tasks:**
```rust
// Other modules can now use:
use wrd_server::portal::{
    PortalManager,
    PortalSessionHandle,
    StreamInfo,
};

// Example usage in PipeWire task:
let portal_manager = PortalManager::new(&config).await?;
let session = portal_manager.create_session().await?;
let pipewire_fd = session.pipewire_fd();
let streams = session.streams();

// Now connect PipeWire using this FD and stream info
```

### Next Tasks Can Use
- `PortalManager::create_session()` → Get PipeWire access
- `PortalSessionHandle::pipewire_fd()` → For PipeWire connection
- `PortalSessionHandle::streams()` → For stream metadata
- `RemoteDesktopManager::notify_*()` → For input injection

---

**END OF TASK SPECIFICATION**

This is a **focused, achievable task** that can be completed in one AI session.
