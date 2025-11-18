//! Input Event Translator
//!
//! Top-level coordinator for translating RDP input events to Linux evdev events
//! with complete keyboard and mouse support.

use crate::input::coordinates::{CoordinateTransformer, MonitorInfo};
use crate::input::error::{InputError, Result};
use crate::input::keyboard::{KeyModifiers, KeyboardEvent, KeyboardHandler};
use crate::input::mouse::{MouseButton, MouseEvent, MouseHandler};
use std::time::Instant;
use tracing::{debug, warn};

/// RDP input event types
#[derive(Debug, Clone)]
pub enum RdpInputEvent {
    /// Keyboard scancode event
    KeyboardScancode {
        /// Scancode value
        scancode: u16,
        /// Extended scancode (E0 prefix)
        extended: bool,
        /// E1 prefix (for Pause/Break)
        e1_prefix: bool,
        /// Key pressed (true) or released (false)
        pressed: bool,
    },

    /// Mouse movement (absolute)
    MouseMove {
        /// X coordinate
        x: u32,
        /// Y coordinate
        y: u32,
    },

    /// Mouse movement (relative)
    MouseMoveRelative {
        /// X delta
        delta_x: i32,
        /// Y delta
        delta_y: i32,
    },

    /// Mouse button event
    MouseButton {
        /// Button flags (RDP format)
        button: u16,
        /// Button pressed (true) or released (false)
        pressed: bool,
    },

    /// Mouse wheel scroll
    MouseWheel {
        /// Horizontal scroll delta
        delta_x: i32,
        /// Vertical scroll delta
        delta_y: i32,
    },
}

/// Translated Linux input event
#[derive(Debug, Clone)]
pub enum LinuxInputEvent {
    /// Keyboard key event
    Keyboard {
        /// Event type (KeyDown, KeyUp, or KeyRepeat)
        event_type: KeyboardEventType,
        /// Linux evdev keycode
        keycode: u32,
        /// RDP scancode
        scancode: u16,
        /// Active modifiers
        modifiers: KeyModifiers,
        /// Event timestamp
        timestamp: Instant,
    },

    /// Mouse movement event
    MouseMove {
        /// Absolute X coordinate
        x: f64,
        /// Absolute Y coordinate
        y: f64,
        /// Event timestamp
        timestamp: Instant,
    },

    /// Mouse button event
    MouseButton {
        /// Linux button code
        button_code: u32,
        /// Button name
        button: MouseButton,
        /// Button pressed (true) or released (false)
        pressed: bool,
        /// Event timestamp
        timestamp: Instant,
    },

    /// Mouse wheel scroll event
    MouseWheel {
        /// Horizontal scroll delta
        delta_x: i32,
        /// Vertical scroll delta
        delta_y: i32,
        /// Event timestamp
        timestamp: Instant,
    },
}

/// Keyboard event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardEventType {
    /// Key pressed
    KeyDown,
    /// Key released
    KeyUp,
    /// Key repeat
    KeyRepeat,
}

/// Input event translator
pub struct InputTranslator {
    /// Keyboard event handler
    keyboard: KeyboardHandler,

    /// Mouse event handler
    mouse: MouseHandler,

    /// Coordinate transformer
    coord_transformer: CoordinateTransformer,

    /// Total events processed
    events_processed: u64,

    /// Events per second counter
    events_this_second: u64,

    /// Last EPS calculation time
    last_eps_time: Instant,
}

impl InputTranslator {
    /// Create a new input translator
    pub fn new(monitors: Vec<MonitorInfo>) -> Result<Self> {
        Ok(Self {
            keyboard: KeyboardHandler::new(),
            mouse: MouseHandler::new(),
            coord_transformer: CoordinateTransformer::new(monitors)?,
            events_processed: 0,
            events_this_second: 0,
            last_eps_time: Instant::now(),
        })
    }

    /// Translate an RDP input event to Linux format
    pub fn translate_event(&mut self, event: RdpInputEvent) -> Result<LinuxInputEvent> {
        self.events_processed += 1;
        self.events_this_second += 1;

        // Update EPS counter
        if self.last_eps_time.elapsed().as_secs() >= 1 {
            debug!("Input events per second: {}", self.events_this_second);
            self.events_this_second = 0;
            self.last_eps_time = Instant::now();
        }

        match event {
            RdpInputEvent::KeyboardScancode {
                scancode,
                extended,
                e1_prefix,
                pressed,
            } => self.translate_keyboard(scancode, extended, e1_prefix, pressed),

            RdpInputEvent::MouseMove { x, y } => self.translate_mouse_move(x, y),

            RdpInputEvent::MouseMoveRelative { delta_x, delta_y } => {
                self.translate_mouse_move_relative(delta_x, delta_y)
            }

            RdpInputEvent::MouseButton { button, pressed } => {
                self.translate_mouse_button(button, pressed)
            }

            RdpInputEvent::MouseWheel { delta_x, delta_y } => {
                self.translate_mouse_wheel(delta_x, delta_y)
            }
        }
    }

    /// Translate keyboard event
    fn translate_keyboard(
        &mut self,
        scancode: u16,
        extended: bool,
        e1_prefix: bool,
        pressed: bool,
    ) -> Result<LinuxInputEvent> {
        let kbd_event = if pressed {
            self.keyboard
                .handle_key_down(scancode, extended, e1_prefix)?
        } else {
            self.keyboard.handle_key_up(scancode, extended, e1_prefix)?
        };

        match kbd_event {
            KeyboardEvent::KeyDown {
                keycode,
                scancode,
                modifiers,
                timestamp,
            } => Ok(LinuxInputEvent::Keyboard {
                event_type: KeyboardEventType::KeyDown,
                keycode,
                scancode,
                modifiers,
                timestamp,
            }),

            KeyboardEvent::KeyUp {
                keycode,
                scancode,
                modifiers,
                timestamp,
            } => Ok(LinuxInputEvent::Keyboard {
                event_type: KeyboardEventType::KeyUp,
                keycode,
                scancode,
                modifiers,
                timestamp,
            }),

            KeyboardEvent::KeyRepeat {
                keycode,
                scancode,
                modifiers,
                timestamp,
            } => Ok(LinuxInputEvent::Keyboard {
                event_type: KeyboardEventType::KeyRepeat,
                keycode,
                scancode,
                modifiers,
                timestamp,
            }),
        }
    }

    /// Translate mouse move event (absolute)
    fn translate_mouse_move(&mut self, x: u32, y: u32) -> Result<LinuxInputEvent> {
        let mouse_event = self
            .mouse
            .handle_absolute_move(x, y, &mut self.coord_transformer)?;

        match mouse_event {
            MouseEvent::Move { x, y, timestamp } => {
                Ok(LinuxInputEvent::MouseMove { x, y, timestamp })
            }
            _ => Err(InputError::InvalidState(
                "Unexpected mouse event type".to_string(),
            )),
        }
    }

    /// Translate mouse move event (relative)
    fn translate_mouse_move_relative(
        &mut self,
        delta_x: i32,
        delta_y: i32,
    ) -> Result<LinuxInputEvent> {
        let mouse_event =
            self.mouse
                .handle_relative_move(delta_x, delta_y, &mut self.coord_transformer)?;

        match mouse_event {
            MouseEvent::Move { x, y, timestamp } => {
                Ok(LinuxInputEvent::MouseMove { x, y, timestamp })
            }
            _ => Err(InputError::InvalidState(
                "Unexpected mouse event type".to_string(),
            )),
        }
    }

    /// Translate mouse button event
    fn translate_mouse_button(
        &mut self,
        button_flags: u16,
        pressed: bool,
    ) -> Result<LinuxInputEvent> {
        let button = MouseButton::from_rdp_button(button_flags).ok_or_else(|| {
            warn!("Unknown RDP mouse button: 0x{:04X}", button_flags);
            InputError::Unknown(format!("Unknown mouse button: 0x{:04X}", button_flags))
        })?;

        let mouse_event = if pressed {
            self.mouse.handle_button_down(button)?
        } else {
            self.mouse.handle_button_up(button)?
        };

        match mouse_event {
            MouseEvent::ButtonDown { button, timestamp } => Ok(LinuxInputEvent::MouseButton {
                button_code: button.to_linux_button(),
                button,
                pressed: true,
                timestamp,
            }),

            MouseEvent::ButtonUp { button, timestamp } => Ok(LinuxInputEvent::MouseButton {
                button_code: button.to_linux_button(),
                button,
                pressed: false,
                timestamp,
            }),

            _ => Err(InputError::InvalidState(
                "Unexpected mouse event type".to_string(),
            )),
        }
    }

    /// Translate mouse wheel event
    fn translate_mouse_wheel(&mut self, delta_x: i32, delta_y: i32) -> Result<LinuxInputEvent> {
        let mouse_event = self.mouse.handle_scroll(delta_x, delta_y)?;

        match mouse_event {
            MouseEvent::Scroll {
                delta_x,
                delta_y,
                timestamp,
            } => Ok(LinuxInputEvent::MouseWheel {
                delta_x,
                delta_y,
                timestamp,
            }),
            _ => Err(InputError::InvalidState(
                "Unexpected mouse event type".to_string(),
            )),
        }
    }

    /// Update monitor configuration
    pub fn update_monitors(&mut self, monitors: Vec<MonitorInfo>) -> Result<()> {
        self.coord_transformer.update_monitors(monitors)
    }

    /// Set keyboard layout
    pub fn set_keyboard_layout(&mut self, layout: &str) {
        self.keyboard.set_layout(layout);
    }

    /// Get current keyboard layout
    pub fn keyboard_layout(&self) -> &str {
        self.keyboard.layout()
    }

    /// Set mouse acceleration enabled
    pub fn set_mouse_acceleration(&mut self, enabled: bool) {
        self.coord_transformer.set_acceleration_enabled(enabled);
    }

    /// Set mouse acceleration factor
    pub fn set_mouse_acceleration_factor(&mut self, factor: f64) {
        self.coord_transformer.set_acceleration_factor(factor);
    }

    /// Set high-precision mouse scrolling
    pub fn set_high_precision_scroll(&mut self, enabled: bool) {
        self.mouse.set_high_precision_scroll(enabled);
    }

    /// Reset input state (release all keys and buttons)
    pub fn reset(&mut self) {
        self.keyboard.reset();
        self.mouse.reset();
        debug!("Input translator reset");
    }

    /// Get total events processed
    pub fn events_processed(&self) -> u64 {
        self.events_processed
    }

    /// Get current mouse position
    pub fn mouse_position(&self) -> (f64, f64) {
        self.mouse.current_position()
    }

    /// Get current keyboard modifiers
    pub fn keyboard_modifiers(&self) -> KeyModifiers {
        self.keyboard.modifiers()
    }

    /// Get number of monitors
    pub fn monitor_count(&self) -> usize {
        self.coord_transformer.monitor_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_monitor() -> MonitorInfo {
        MonitorInfo {
            id: 1,
            name: "Primary".to_string(),
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            dpi: 96.0,
            scale_factor: 1.0,
            stream_x: 0,
            stream_y: 0,
            stream_width: 1920,
            stream_height: 1080,
            is_primary: true,
        }
    }

    #[test]
    fn test_translator_creation() {
        let translator = InputTranslator::new(vec![create_test_monitor()]).unwrap();
        assert_eq!(translator.events_processed(), 0);
        assert_eq!(translator.monitor_count(), 1);
    }

    #[test]
    fn test_translate_keyboard_event() {
        let mut translator = InputTranslator::new(vec![create_test_monitor()]).unwrap();

        // Key down
        let event = RdpInputEvent::KeyboardScancode {
            scancode: 0x1E, // A key
            extended: false,
            e1_prefix: false,
            pressed: true,
        };

        let result = translator.translate_event(event).unwrap();

        match result {
            LinuxInputEvent::Keyboard {
                event_type,
                keycode,
                ..
            } => {
                assert_eq!(event_type, KeyboardEventType::KeyDown);
                assert!(keycode > 0);
            }
            _ => panic!("Expected Keyboard event"),
        }

        assert_eq!(translator.events_processed(), 1);
    }

    #[test]
    fn test_translate_mouse_move() {
        let mut translator = InputTranslator::new(vec![create_test_monitor()]).unwrap();

        let event = RdpInputEvent::MouseMove { x: 960, y: 540 };

        let result = translator.translate_event(event).unwrap();

        match result {
            LinuxInputEvent::MouseMove { x, y, .. } => {
                assert!(x >= 0.0);
                assert!(y >= 0.0);
            }
            _ => panic!("Expected MouseMove event"),
        }
    }

    #[test]
    fn test_translate_mouse_button() {
        let mut translator = InputTranslator::new(vec![create_test_monitor()]).unwrap();

        let event = RdpInputEvent::MouseButton {
            button: 0x1000, // Left button
            pressed: true,
        };

        let result = translator.translate_event(event).unwrap();

        match result {
            LinuxInputEvent::MouseButton {
                button,
                pressed,
                button_code,
                ..
            } => {
                assert_eq!(button, MouseButton::Left);
                assert!(pressed);
                assert_eq!(button_code, 0x110);
            }
            _ => panic!("Expected MouseButton event"),
        }
    }

    #[test]
    fn test_translate_mouse_wheel() {
        let mut translator = InputTranslator::new(vec![create_test_monitor()]).unwrap();

        let event = RdpInputEvent::MouseWheel {
            delta_x: 0,
            delta_y: 120,
        };

        let result = translator.translate_event(event).unwrap();

        match result {
            LinuxInputEvent::MouseWheel { delta_y, .. } => {
                assert_eq!(delta_y, 1);
            }
            _ => panic!("Expected MouseWheel event"),
        }
    }

    #[test]
    fn test_keyboard_modifiers() {
        let mut translator = InputTranslator::new(vec![create_test_monitor()]).unwrap();

        // Press Shift
        let event = RdpInputEvent::KeyboardScancode {
            scancode: 0x2A, // Left Shift
            extended: false,
            e1_prefix: false,
            pressed: true,
        };

        translator.translate_event(event).unwrap();

        let modifiers = translator.keyboard_modifiers();
        assert!(modifiers.shift);
    }

    #[test]
    fn test_layout_change() {
        let mut translator = InputTranslator::new(vec![create_test_monitor()]).unwrap();

        assert_eq!(translator.keyboard_layout(), "us");

        translator.set_keyboard_layout("de");
        assert_eq!(translator.keyboard_layout(), "de");
    }

    #[test]
    fn test_reset() {
        let mut translator = InputTranslator::new(vec![create_test_monitor()]).unwrap();

        // Press some keys
        translator
            .translate_event(RdpInputEvent::KeyboardScancode {
                scancode: 0x1E,
                extended: false,
                e1_prefix: false,
                pressed: true,
            })
            .unwrap();

        // Reset
        translator.reset();

        let modifiers = translator.keyboard_modifiers();
        assert!(!modifiers.shift);
        assert!(!modifiers.ctrl);
    }

    #[test]
    fn test_events_counter() {
        let mut translator = InputTranslator::new(vec![create_test_monitor()]).unwrap();

        for i in 0..10 {
            translator
                .translate_event(RdpInputEvent::MouseMove {
                    x: i * 100,
                    y: i * 100,
                })
                .unwrap();
        }

        assert_eq!(translator.events_processed(), 10);
    }

    #[test]
    fn test_mouse_position_tracking() {
        let mut translator = InputTranslator::new(vec![create_test_monitor()]).unwrap();

        translator
            .translate_event(RdpInputEvent::MouseMove { x: 100, y: 200 })
            .unwrap();

        let (x, y) = translator.mouse_position();
        assert!(x >= 0.0);
        assert!(y >= 0.0);
    }
}
