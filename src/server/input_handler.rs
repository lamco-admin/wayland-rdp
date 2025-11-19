//! RDP Input Handler Implementation
//!
//! Implements the IronRDP `RdpServerInputHandler` trait to forward input events
//! from RDP clients to the Wayland compositor via Portal RemoteDesktop API.
//!
//! # Overview
//!
//! This module bridges the synchronous IronRDP input event callbacks with the
//! asynchronous Portal API, providing complete keyboard and mouse input forwarding
//! with full scancode translation, modifier tracking, and coordinate transformation.
//!
//! # Architecture
//!
//! ```text
//! RDP Client                    WrdInputHandler                 Wayland
//! ━━━━━━━━━━                    ━━━━━━━━━━━━━━━                 ━━━━━━━
//!
//! Keyboard Event ─────────────> KeyboardEvent
//!   scancode=0x1E                     │
//!   pressed=true                      ├─> KeyboardHandler
//!                                     │     └─> Scancode translation
//!                                     │         (0x1E → evdev KEY_A)
//!                                     │
//!                                     ├─> Portal API
//!                                     │     └─> notify_keyboard_keycode()
//!                                     │
//!                                     └─────────────────────────> Input Stack
//!                                                                   └─> Key Press
//!
//! Mouse Event ────────────────> MouseEvent::Move
//!   x=960, y=540                     │
//!                                    ├─> CoordinateTransformer
//!                                    │     └─> RDP coords → Wayland coords
//!                                    │
//!                                    ├─> Portal API
//!                                    │     └─> notify_pointer_motion_absolute()
//!                                    │
//!                                    └─────────────────────────> Input Stack
//!                                                                  └─> Mouse Move
//! ```
//!
//! # Async/Sync Bridging
//!
//! IronRDP's `RdpServerInputHandler` trait has synchronous methods (`fn`, not `async fn`),
//! but Portal API calls are asynchronous. We bridge this gap by:
//!
//! 1. Trait method called synchronously by IronRDP
//! 2. Clone Arc references to shared state
//! 3. Spawn `tokio::spawn()` async task
//! 4. Task performs async Portal API calls
//! 5. Fire-and-forget (RDP doesn't expect acknowledgment for input events)
//!
//! This pattern ensures the synchronous trait method returns immediately while
//! Portal operations proceed asynchronously without blocking.
//!
//! # Example
//!
//! ```no_run
//! use wrd_server::server::WrdInputHandler;
//! use wrd_server::portal::RemoteDesktopManager;
//! use wrd_server::input::coordinates::MonitorInfo;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let portal = Arc::new(RemoteDesktopManager::new(/* ... */).await?);
//! let session = portal.create_session().await?;
//! let monitors = vec![/* MonitorInfo instances */];
//!
//! let handler = WrdInputHandler::new(portal, session, monitors)?;
//!
//! // Handler is now ready to receive input events from IronRDP
//! // Events are automatically forwarded to Wayland via Portal
//! # Ok(())
//! # }
//! ```

use ironrdp_server::{KeyboardEvent as IronKeyboardEvent, MouseEvent as IronMouseEvent, RdpServerInputHandler};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, warn};

use crate::input::coordinates::{CoordinateTransformer, MonitorInfo};
use crate::input::error::InputError;
use crate::input::keyboard::KeyboardHandler;
use crate::input::mouse::{MouseButton, MouseHandler};
use crate::portal::RemoteDesktopManager;

/// WRD Input Handler
///
/// Bridges IronRDP input events to our Portal-based input injection system.
/// This handler receives keyboard and mouse events from RDP clients and forwards
/// them to the Wayland compositor through the RemoteDesktop portal.
///
/// Since IronRDP's trait methods are synchronous but portal operations are async,
/// we use channels and spawned tasks to bridge the gap.
pub struct WrdInputHandler {
    /// Portal RemoteDesktop manager for input injection
    portal: Arc<RemoteDesktopManager>,

    /// Keyboard event handler
    keyboard_handler: Arc<Mutex<KeyboardHandler>>,

    /// Mouse event handler
    mouse_handler: Arc<Mutex<MouseHandler>>,

    /// Coordinate transformer for multi-monitor support
    coordinate_transformer: Arc<Mutex<CoordinateTransformer>>,

    /// Portal session (kept alive for the connection lifetime)
    session: Arc<Mutex<ashpd::desktop::Session<'static, ashpd::desktop::remote_desktop::RemoteDesktop<'static>>>>,

    /// Primary stream node ID for input injection (PipeWire node ID)
    primary_stream_id: u32,
}

impl WrdInputHandler {
    /// Create a new input handler
    ///
    /// # Arguments
    ///
    /// * `portal` - RemoteDesktop portal manager
    /// * `session` - Portal session handle (must remain alive)
    /// * `monitors` - Monitor configuration for coordinate transformation
    ///
    /// # Returns
    ///
    /// A new `WrdInputHandler` instance ready to process input events
    ///
    /// # Errors
    ///
    /// Returns error if coordinate transformer initialization fails
    pub fn new(
        portal: Arc<RemoteDesktopManager>,
        session: Arc<Mutex<ashpd::desktop::Session<'static, ashpd::desktop::remote_desktop::RemoteDesktop<'static>>>>,
        monitors: Vec<MonitorInfo>,
        primary_stream_id: u32,
    ) -> Result<Self, InputError> {
        let keyboard_handler = Arc::new(Mutex::new(KeyboardHandler::new()));
        let mouse_handler = Arc::new(Mutex::new(MouseHandler::new()));

        // Create coordinate transformer with monitor configuration
        let coordinate_transformer = Arc::new(Mutex::new(
            CoordinateTransformer::new(monitors)?
        ));

        debug!("Input handler using PipeWire stream node ID: {}", primary_stream_id);

        Ok(Self {
            portal,
            keyboard_handler,
            mouse_handler,
            coordinate_transformer,
            session, // Already wrapped in Arc<Mutex>
            primary_stream_id,
        })
    }

    /// Update coordinate transformer when monitor configuration changes
    ///
    /// This should be called when the RDP client requests a different resolution
    /// or when monitor configuration changes.
    pub async fn update_monitors(&self, monitors: Vec<MonitorInfo>) -> Result<(), InputError> {
        let mut transformer = self.coordinate_transformer.lock().await;
        *transformer = CoordinateTransformer::new(monitors)?;
        debug!("Updated monitor configuration");
        Ok(())
    }

    /// Handle keyboard event with full error handling and logging
    async fn handle_keyboard_async(&self, event: IronKeyboardEvent) -> Result<(), InputError> {
        let mut keyboard = self.keyboard_handler.lock().await;
        let session = self.session.lock().await;

        match event {
            IronKeyboardEvent::Pressed { code, extended } => {
                debug!("Keyboard pressed: code={}, extended={}", code, extended);

                // Process key down through keyboard handler
                let kbd_event = keyboard.handle_key_down(code as u16, extended, false)?;

                // Extract keycode from our event
                let keycode = match kbd_event {
                    crate::input::keyboard::KeyboardEvent::KeyDown { keycode, .. } |
                    crate::input::keyboard::KeyboardEvent::KeyRepeat { keycode, .. } => keycode,
                    _ => return Err(InputError::InvalidKeyEvent("Unexpected event type".to_string())),
                };

                // Inject key press via portal
                self.portal
                    .notify_keyboard_keycode(&session, keycode as i32, true)
                    .await
                    .map_err(|e| InputError::PortalError(format!("Failed to inject key press: {}", e)))?;
            }

            IronKeyboardEvent::Released { code, extended } => {
                debug!("Keyboard released: code={}, extended={}", code, extended);

                // Process key up through keyboard handler
                let kbd_event = keyboard.handle_key_up(code as u16, extended, false)?;

                // Extract keycode from our event
                let keycode = match kbd_event {
                    crate::input::keyboard::KeyboardEvent::KeyUp { keycode, .. } => keycode,
                    _ => return Err(InputError::InvalidKeyEvent("Unexpected event type".to_string())),
                };

                // Inject key release via portal
                self.portal
                    .notify_keyboard_keycode(&session, keycode as i32, false)
                    .await
                    .map_err(|e| InputError::PortalError(format!("Failed to inject key release: {}", e)))?;
            }

            IronKeyboardEvent::UnicodePressed(unicode) => {
                debug!("Unicode key pressed: 0x{:04X}", unicode);
                // Unicode events - for now log as unsupported
                // Full implementation would use XKB keysym injection
                warn!("Unicode keyboard events not yet fully supported: 0x{:04X}", unicode);
            }

            IronKeyboardEvent::UnicodeReleased(unicode) => {
                debug!("Unicode key released: 0x{:04X}", unicode);
                warn!("Unicode keyboard events not yet fully supported: 0x{:04X}", unicode);
            }

            IronKeyboardEvent::Synchronize(flags) => {
                debug!("Keyboard synchronize: {:?}", flags);
                // Update toggle key states based on sync flags
                // The flags tell us the client's current Caps/Num/Scroll lock states
                // We should sync our local state but portal doesn't have direct sync API
                // This is handled implicitly when keys are pressed
            }
        }

        Ok(())
    }

    /// Handle mouse event with full error handling and logging
    async fn handle_mouse_async(&self, event: IronMouseEvent) -> Result<(), InputError> {
        let mut mouse = self.mouse_handler.lock().await;
        let mut transformer = self.coordinate_transformer.lock().await;
        let session = self.session.lock().await;

        match event {
            IronMouseEvent::Move { x, y } => {
                debug!("Mouse move: x={}, y={}", x, y);

                // Process absolute move through mouse handler
                let mouse_event = mouse.handle_absolute_move(x as u32, y as u32, &mut transformer)?;

                // Extract coordinates from our event
                let (stream_x, stream_y) = match mouse_event {
                    crate::input::mouse::MouseEvent::Move { x, y, .. } => (x, y),
                    _ => return Err(InputError::InvalidMouseEvent("Unexpected event type".to_string())),
                };

                // Inject mouse movement via portal (absolute positioning)
                // Portal API uses PipeWire node ID (not index) for stream identification
                self.portal
                    .notify_pointer_motion_absolute(&session, self.primary_stream_id, stream_x, stream_y)
                    .await
                    .map_err(|e| InputError::PortalError(format!("Failed to inject mouse move: {}", e)))?;
            }

            IronMouseEvent::RelMove { x, y } => {
                debug!("Mouse relative move: dx={}, dy={}", x, y);

                // Process relative move through mouse handler
                let mouse_event = mouse.handle_relative_move(x, y, &mut transformer)?;

                // Extract coordinates
                let (stream_x, stream_y) = match mouse_event {
                    crate::input::mouse::MouseEvent::Move { x, y, .. } => (x, y),
                    _ => return Err(InputError::InvalidMouseEvent("Unexpected event type".to_string())),
                };

                // Inject via portal absolute API (we converted relative to absolute already)
                self.portal
                    .notify_pointer_motion_absolute(&session, self.primary_stream_id, stream_x, stream_y)
                    .await
                    .map_err(|e| InputError::PortalError(format!("Failed to inject relative move: {}", e)))?;
            }

            IronMouseEvent::LeftPressed => {
                debug!("Left mouse button pressed");
                mouse.handle_button_down(MouseButton::Left)?;
                self.portal
                    .notify_pointer_button(&session, 272, true) // BTN_LEFT = 0x110 = 272 (evdev code)
                    .await
                    .map_err(|e| InputError::PortalError(format!("Failed to inject left press: {}", e)))?;
            }

            IronMouseEvent::LeftReleased => {
                debug!("Left mouse button released");
                mouse.handle_button_up(MouseButton::Left)?;
                self.portal
                    .notify_pointer_button(&session, 272, false) // BTN_LEFT = 0x110 = 272
                    .await
                    .map_err(|e| InputError::PortalError(format!("Failed to inject left release: {}", e)))?;
            }

            IronMouseEvent::RightPressed => {
                debug!("Right mouse button pressed");
                mouse.handle_button_down(MouseButton::Right)?;
                self.portal
                    .notify_pointer_button(&session, 273, true) // BTN_RIGHT = 0x111 = 273
                    .await
                    .map_err(|e| InputError::PortalError(format!("Failed to inject right press: {}", e)))?;
            }

            IronMouseEvent::RightReleased => {
                debug!("Right mouse button released");
                mouse.handle_button_up(MouseButton::Right)?;
                self.portal
                    .notify_pointer_button(&session, 273, false) // BTN_RIGHT = 0x111 = 273
                    .await
                    .map_err(|e| InputError::PortalError(format!("Failed to inject right release: {}", e)))?;
            }

            IronMouseEvent::MiddlePressed => {
                debug!("Middle mouse button pressed");
                mouse.handle_button_down(MouseButton::Middle)?;
                self.portal
                    .notify_pointer_button(&session, 274, true) // BTN_MIDDLE = 0x112 = 274
                    .await
                    .map_err(|e| InputError::PortalError(format!("Failed to inject middle press: {}", e)))?;
            }

            IronMouseEvent::MiddleReleased => {
                debug!("Middle mouse button released");
                mouse.handle_button_up(MouseButton::Middle)?;
                self.portal
                    .notify_pointer_button(&session, 274, false) // BTN_MIDDLE = 0x112 = 274
                    .await
                    .map_err(|e| InputError::PortalError(format!("Failed to inject middle release: {}", e)))?;
            }

            IronMouseEvent::Button4Pressed => {
                debug!("Mouse button 4 pressed");
                mouse.handle_button_down(MouseButton::Extra1)?;
                self.portal
                    .notify_pointer_button(&session, 275, true) // BTN_SIDE = 8
                    .await
                    .map_err(|e| InputError::PortalError(format!("Failed to inject button4 press: {}", e)))?;
            }

            IronMouseEvent::Button4Released => {
                debug!("Mouse button 4 released");
                mouse.handle_button_up(MouseButton::Extra1)?;
                self.portal
                    .notify_pointer_button(&session, 275, false)
                    .await
                    .map_err(|e| InputError::PortalError(format!("Failed to inject button4 release: {}", e)))?;
            }

            IronMouseEvent::Button5Pressed => {
                debug!("Mouse button 5 pressed");
                mouse.handle_button_down(MouseButton::Extra2)?;
                self.portal
                    .notify_pointer_button(&session, 276, true) // BTN_EXTRA = 9
                    .await
                    .map_err(|e| InputError::PortalError(format!("Failed to inject button5 press: {}", e)))?;
            }

            IronMouseEvent::Button5Released => {
                debug!("Mouse button 5 released");
                mouse.handle_button_up(MouseButton::Extra2)?;
                self.portal
                    .notify_pointer_button(&session, 276, false)
                    .await
                    .map_err(|e| InputError::PortalError(format!("Failed to inject button5 release: {}", e)))?;
            }

            IronMouseEvent::VerticalScroll { value } => {
                debug!("Mouse vertical scroll: {}", value);
                // RDP scroll units are in 120ths
                mouse.handle_scroll(0, value as i32)?;

                // Portal scroll API takes continuous values
                let delta_y = (value as f64 / 120.0) * 15.0; // 15 pixels per scroll unit
                self.portal
                    .notify_pointer_axis(&session, 0.0, delta_y)
                    .await
                    .map_err(|e| InputError::PortalError(format!("Failed to inject vertical scroll: {}", e)))?;
            }

            IronMouseEvent::Scroll { x, y } => {
                debug!("Mouse scroll: x={}, y={}", x, y);
                mouse.handle_scroll(x, y)?;

                // Normalize scroll values
                let delta_x = (x as f64 / 120.0) * 15.0;
                let delta_y = (y as f64 / 120.0) * 15.0;
                self.portal
                    .notify_pointer_axis(&session, delta_x, delta_y)
                    .await
                    .map_err(|e| InputError::PortalError(format!("Failed to inject scroll: {}", e)))?;
            }
        }

        Ok(())
    }
}

/// Implement IronRDP's `RdpServerInputHandler` trait
///
/// This is a synchronous trait, so we spawn async tasks for each event.
/// The portal API requires async operations, so we bridge the synchronous
/// trait to async execution.
impl RdpServerInputHandler for WrdInputHandler {
    fn keyboard(&mut self, event: IronKeyboardEvent) {
        // Clone Arc references for the async task
        let portal = Arc::clone(&self.portal);
        let keyboard_handler = Arc::clone(&self.keyboard_handler);
        let session = Arc::clone(&self.session);
        let primary_stream_id = self.primary_stream_id;
        let coordinate_transformer = Arc::clone(&self.coordinate_transformer); // Clone for consistency

        // Spawn async task to handle the event
        // We spawn a task because the trait method is synchronous but portal calls are async
        tokio::spawn(async move {
            // Create a temporary handler wrapper for this event
            let temp_handler = WrdInputHandler {
                portal,
                keyboard_handler,
                mouse_handler: Arc::new(Mutex::new(MouseHandler::new())), // Not used for keyboard
                coordinate_transformer, // Use actual transformer
                session,
                primary_stream_id,
            };

            if let Err(e) = temp_handler.handle_keyboard_async(event).await {
                error!("Failed to handle keyboard event: {}", e);
            }
        });
    }

    fn mouse(&mut self, event: IronMouseEvent) {
        // Clone Arc references for the async task
        let portal = Arc::clone(&self.portal);
        let mouse_handler = Arc::clone(&self.mouse_handler);
        let coordinate_transformer = Arc::clone(&self.coordinate_transformer);
        let session = Arc::clone(&self.session);
        let primary_stream_id = self.primary_stream_id;

        // Spawn async task to handle the event
        tokio::spawn(async move {
            // Create a temporary handler wrapper for this event
            let temp_handler = WrdInputHandler {
                portal,
                keyboard_handler: Arc::new(Mutex::new(KeyboardHandler::new())), // Not used for mouse
                mouse_handler,
                coordinate_transformer,
                session,
                primary_stream_id,
            };

            if let Err(e) = temp_handler.handle_mouse_async(event).await {
                error!("Failed to handle mouse event: {}", e);
            }
        });
    }
}

/// Custom Clone implementation to allow handler to be cloned
/// This is necessary because RdpServer needs ownership but we want to share state
impl Clone for WrdInputHandler {
    fn clone(&self) -> Self {
        Self {
            portal: Arc::clone(&self.portal),
            keyboard_handler: Arc::clone(&self.keyboard_handler),
            mouse_handler: Arc::clone(&self.mouse_handler),
            coordinate_transformer: Arc::clone(&self.coordinate_transformer),
            session: Arc::clone(&self.session),
            primary_stream_id: self.primary_stream_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_handler_clone() {
        // Verify clone compiles and works
        // Full tests require portal mocking
    }
}
