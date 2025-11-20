//! Input handling for the compositor
//!
//! This module handles keyboard, pointer, and touch input events from RDP.

use super::types::{ButtonState, KeyState, Modifiers, Point};
use anyhow::Result;
use std::collections::HashMap;

/// Keyboard event from RDP
#[derive(Debug, Clone)]
pub struct KeyboardEvent {
    /// Key code (scancode or keysym)
    pub key: u32,

    /// Key state (pressed or released)
    pub state: KeyState,

    /// Keyboard modifiers
    pub modifiers: Modifiers,

    /// Timestamp in milliseconds
    pub timestamp: u32,
}

impl KeyboardEvent {
    /// Create new keyboard event
    pub fn new(key: u32, state: KeyState, modifiers: Modifiers, timestamp: u32) -> Self {
        Self {
            key,
            state,
            modifiers,
            timestamp,
        }
    }

    /// Create key press event
    pub fn press(key: u32, modifiers: Modifiers) -> Self {
        Self::new(key, KeyState::Pressed, modifiers, 0)
    }

    /// Create key release event
    pub fn release(key: u32, modifiers: Modifiers) -> Self {
        Self::new(key, KeyState::Released, modifiers, 0)
    }
}

/// Pointer event from RDP
#[derive(Debug, Clone)]
pub struct PointerEvent {
    /// X coordinate (logical)
    pub x: f64,

    /// Y coordinate (logical)
    pub y: f64,

    /// Button event (button number, state)
    pub button: Option<(u32, ButtonState)>,

    /// Axis event (scroll)
    pub axis: Option<AxisEvent>,

    /// Timestamp in milliseconds
    pub timestamp: u32,
}

impl PointerEvent {
    /// Create pointer motion event
    pub fn motion(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            button: None,
            axis: None,
            timestamp: 0,
        }
    }

    /// Create pointer button event
    pub fn button(x: f64, y: f64, button: u32, state: ButtonState) -> Self {
        Self {
            x,
            y,
            button: Some((button, state)),
            axis: None,
            timestamp: 0,
        }
    }

    /// Create pointer axis event
    pub fn axis(x: f64, y: f64, axis: AxisEvent) -> Self {
        Self {
            x,
            y,
            button: None,
            axis: Some(axis),
            timestamp: 0,
        }
    }
}

/// Axis event (scroll)
#[derive(Debug, Clone, Copy)]
pub struct AxisEvent {
    /// Horizontal axis value
    pub horizontal: f64,

    /// Vertical axis value
    pub vertical: f64,
}

impl AxisEvent {
    pub fn new(horizontal: f64, vertical: f64) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }

    pub fn vertical(value: f64) -> Self {
        Self::new(0.0, value)
    }

    pub fn horizontal(value: f64) -> Self {
        Self::new(value, 0.0)
    }
}

/// Touch event from RDP
#[derive(Debug, Clone)]
pub struct TouchEvent {
    /// Touch ID
    pub id: u32,

    /// X coordinate
    pub x: f64,

    /// Y coordinate
    pub y: f64,

    /// Touch state
    pub state: TouchState,

    /// Timestamp
    pub timestamp: u32,
}

/// Touch state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchState {
    Down,
    Move,
    Up,
    Cancel,
}

/// Keyboard translator
///
/// Translates RDP scancodes to Linux keycodes/keysyms
pub struct KeyboardTranslator {
    /// Scancode to keycode map
    scancode_map: HashMap<u32, u32>,

    /// Current modifiers
    modifiers: Modifiers,
}

impl KeyboardTranslator {
    /// Create new keyboard translator
    pub fn new() -> Self {
        let mut scancode_map = HashMap::new();

        // Build scancode to Linux keycode mapping
        // This is a simplified mapping; real implementation would be more comprehensive
        Self::init_scancode_map(&mut scancode_map);

        Self {
            scancode_map,
            modifiers: Modifiers::default(),
        }
    }

    /// Initialize scancode mapping
    fn init_scancode_map(map: &mut HashMap<u32, u32>) {
        // Common key mappings (RDP scancode -> Linux keycode)
        // ESC
        map.insert(0x01, 1);
        // Number keys
        map.insert(0x02, 2); // 1
        map.insert(0x03, 3); // 2
        map.insert(0x04, 4); // 3
        map.insert(0x05, 5); // 4
        map.insert(0x06, 6); // 5
        map.insert(0x07, 7); // 6
        map.insert(0x08, 8); // 7
        map.insert(0x09, 9); // 8
        map.insert(0x0A, 10); // 9
        map.insert(0x0B, 11); // 0
        // Letters
        map.insert(0x10, 16); // Q
        map.insert(0x11, 17); // W
        map.insert(0x12, 18); // E
        map.insert(0x13, 19); // R
        map.insert(0x14, 20); // T
        map.insert(0x15, 21); // Y
        map.insert(0x16, 22); // U
        map.insert(0x17, 23); // I
        map.insert(0x18, 24); // O
        map.insert(0x19, 25); // P
        map.insert(0x1E, 30); // A
        map.insert(0x1F, 31); // S
        map.insert(0x20, 32); // D
        map.insert(0x21, 33); // F
        map.insert(0x22, 34); // G
        map.insert(0x23, 35); // H
        map.insert(0x24, 36); // J
        map.insert(0x25, 37); // K
        map.insert(0x26, 38); // L
        map.insert(0x2C, 44); // Z
        map.insert(0x2D, 45); // X
        map.insert(0x2E, 46); // C
        map.insert(0x2F, 47); // V
        map.insert(0x30, 48); // B
        map.insert(0x31, 49); // N
        map.insert(0x32, 50); // M
        // Special keys
        map.insert(0x1C, 28); // Enter
        map.insert(0x39, 57); // Space
        map.insert(0x0E, 14); // Backspace
        map.insert(0x0F, 15); // Tab
        map.insert(0x1D, 29); // Ctrl (left)
        map.insert(0x2A, 42); // Shift (left)
        map.insert(0x36, 54); // Shift (right)
        map.insert(0x38, 56); // Alt (left)
        // Function keys
        map.insert(0x3B, 59); // F1
        map.insert(0x3C, 60); // F2
        map.insert(0x3D, 61); // F3
        map.insert(0x3E, 62); // F4
        map.insert(0x3F, 63); // F5
        map.insert(0x40, 64); // F6
        map.insert(0x41, 65); // F7
        map.insert(0x42, 66); // F8
        map.insert(0x43, 67); // F9
        map.insert(0x44, 68); // F10
        map.insert(0x57, 87); // F11
        map.insert(0x58, 88); // F12
        // Arrow keys
        map.insert(0xE048, 103); // Up
        map.insert(0xE050, 108); // Down
        map.insert(0xE04B, 105); // Left
        map.insert(0xE04D, 106); // Right
    }

    /// Translate RDP scancode to Linux keycode
    pub fn translate_scancode(&self, scancode: u32) -> Option<u32> {
        self.scancode_map.get(&scancode).copied()
    }

    /// Update modifiers from key event
    pub fn update_modifiers(&mut self, keycode: u32, state: KeyState) {
        match keycode {
            29 | 97 => {
                // Left Ctrl or Right Ctrl
                self.modifiers.ctrl = state == KeyState::Pressed;
            }
            42 | 54 => {
                // Left Shift or Right Shift
                self.modifiers.shift = state == KeyState::Pressed;
            }
            56 | 100 => {
                // Left Alt or Right Alt
                self.modifiers.alt = state == KeyState::Pressed;
            }
            125 => {
                // Left Logo/Super
                self.modifiers.logo = state == KeyState::Pressed;
            }
            _ => {}
        }
    }

    /// Get current modifiers
    pub fn get_modifiers(&self) -> Modifiers {
        self.modifiers
    }
}

/// Pointer coordinate translator
///
/// Translates RDP coordinates to compositor logical coordinates
pub struct PointerTranslator {
    /// Display width
    width: u32,

    /// Display height
    height: u32,

    /// Last known position
    last_position: Point,
}

impl PointerTranslator {
    /// Create new pointer translator
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            last_position: Point::new(0, 0),
        }
    }

    /// Translate RDP coordinates to logical coordinates
    pub fn translate(&mut self, x: u16, y: u16) -> Point {
        // Simple direct mapping (1:1)
        // In a real implementation, this might need scaling or transformation
        let point = Point {
            x: x.min(self.width as u16 - 1) as i32,
            y: y.min(self.height as u16 - 1) as i32,
        };

        self.last_position = point;
        point
    }

    /// Get last position
    pub fn last_position(&self) -> Point {
        self.last_position
    }

    /// Update display size
    pub fn update_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        // Clamp last position to new bounds
        if self.last_position.x >= width as i32 {
            self.last_position.x = width as i32 - 1;
        }
        if self.last_position.y >= height as i32 {
            self.last_position.y = height as i32 - 1;
        }
    }
}

/// Input manager
///
/// Manages keyboard and pointer input state and translation
pub struct InputManager {
    /// Keyboard translator
    keyboard_translator: KeyboardTranslator,

    /// Pointer translator
    pointer_translator: PointerTranslator,
}

impl InputManager {
    /// Create new input manager
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            keyboard_translator: KeyboardTranslator::new(),
            pointer_translator: PointerTranslator::new(width, height),
        }
    }

    /// Translate RDP keyboard input
    pub fn translate_keyboard(&mut self, scancode: u32, pressed: bool) -> Option<KeyboardEvent> {
        let keycode = self.keyboard_translator.translate_scancode(scancode)?;

        let state = if pressed {
            KeyState::Pressed
        } else {
            KeyState::Released
        };

        self.keyboard_translator.update_modifiers(keycode, state);
        let modifiers = self.keyboard_translator.get_modifiers();

        Some(KeyboardEvent::new(keycode, state, modifiers, 0))
    }

    /// Translate RDP pointer input
    pub fn translate_pointer(&mut self, x: u16, y: u16) -> PointerEvent {
        let point = self.pointer_translator.translate(x, y);
        PointerEvent::motion(point.x as f64, point.y as f64)
    }

    /// Translate RDP button input
    pub fn translate_button(&mut self, button: u32, pressed: bool) -> Option<PointerEvent> {
        let point = self.pointer_translator.last_position();
        let state = if pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        };

        Some(PointerEvent::button(
            point.x as f64,
            point.y as f64,
            button,
            state,
        ))
    }

    /// Update display size
    pub fn update_size(&mut self, width: u32, height: u32) {
        self.pointer_translator.update_size(width, height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_event_creation() {
        let event = KeyboardEvent::press(30, Modifiers::default());
        assert_eq!(event.key, 30);
        assert_eq!(event.state, KeyState::Pressed);
    }

    #[test]
    fn test_pointer_event_creation() {
        let event = PointerEvent::motion(100.0, 200.0);
        assert_eq!(event.x, 100.0);
        assert_eq!(event.y, 200.0);
        assert!(event.button.is_none());
    }

    #[test]
    fn test_keyboard_translator() {
        let translator = KeyboardTranslator::new();

        // Test a known mapping (Q key)
        assert_eq!(translator.translate_scancode(0x10), Some(16));

        // Test unknown scancode
        assert_eq!(translator.translate_scancode(0xFFFF), None);
    }

    #[test]
    fn test_pointer_translator() {
        let mut translator = PointerTranslator::new(1920, 1080);

        let point = translator.translate(100, 200);
        assert_eq!(point.x, 100);
        assert_eq!(point.y, 200);

        // Test clamping
        let point = translator.translate(2000, 1100);
        assert_eq!(point.x, 1919);
        assert_eq!(point.y, 1079);
    }

    #[test]
    fn test_input_manager() {
        let mut manager = InputManager::new(1920, 1080);

        // Test keyboard translation
        let event = manager.translate_keyboard(0x10, true);
        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.key, 16); // Q key
        assert_eq!(event.state, KeyState::Pressed);

        // Test pointer translation
        let event = manager.translate_pointer(100, 200);
        assert_eq!(event.x, 100.0);
        assert_eq!(event.y, 200.0);
    }

    #[test]
    fn test_modifier_tracking() {
        let mut translator = KeyboardTranslator::new();

        // Press Ctrl
        translator.update_modifiers(29, KeyState::Pressed);
        assert!(translator.get_modifiers().ctrl);

        // Release Ctrl
        translator.update_modifiers(29, KeyState::Released);
        assert!(!translator.get_modifiers().ctrl);

        // Press Shift
        translator.update_modifiers(42, KeyState::Pressed);
        assert!(translator.get_modifiers().shift);
    }
}
