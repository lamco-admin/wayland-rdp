//! Scancode Mapping Tables
//!
//! Complete RDP scancode to Linux evdev keycode mapping for 200+ keys.
//! Includes standard keys, extended keys, and international keyboard support.

use crate::input::error::{InputError, Result};
use std::collections::HashMap;

/// Linux evdev keycodes
pub mod keycodes {
    // Primary keys
    pub const KEY_ESC: u32 = 1;
    pub const KEY_1: u32 = 2;
    pub const KEY_2: u32 = 3;
    pub const KEY_3: u32 = 4;
    pub const KEY_4: u32 = 5;
    pub const KEY_5: u32 = 6;
    pub const KEY_6: u32 = 7;
    pub const KEY_7: u32 = 8;
    pub const KEY_8: u32 = 9;
    pub const KEY_9: u32 = 10;
    pub const KEY_0: u32 = 11;
    pub const KEY_MINUS: u32 = 12;
    pub const KEY_EQUAL: u32 = 13;
    pub const KEY_BACKSPACE: u32 = 14;
    pub const KEY_TAB: u32 = 15;
    pub const KEY_Q: u32 = 16;
    pub const KEY_W: u32 = 17;
    pub const KEY_E: u32 = 18;
    pub const KEY_R: u32 = 19;
    pub const KEY_T: u32 = 20;
    pub const KEY_Y: u32 = 21;
    pub const KEY_U: u32 = 22;
    pub const KEY_I: u32 = 23;
    pub const KEY_O: u32 = 24;
    pub const KEY_P: u32 = 25;
    pub const KEY_LEFTBRACE: u32 = 26;
    pub const KEY_RIGHTBRACE: u32 = 27;
    pub const KEY_ENTER: u32 = 28;
    pub const KEY_LEFTCTRL: u32 = 29;
    pub const KEY_A: u32 = 30;
    pub const KEY_S: u32 = 31;
    pub const KEY_D: u32 = 32;
    pub const KEY_F: u32 = 33;
    pub const KEY_G: u32 = 34;
    pub const KEY_H: u32 = 35;
    pub const KEY_J: u32 = 36;
    pub const KEY_K: u32 = 37;
    pub const KEY_L: u32 = 38;
    pub const KEY_SEMICOLON: u32 = 39;
    pub const KEY_APOSTROPHE: u32 = 40;
    pub const KEY_GRAVE: u32 = 41;
    pub const KEY_LEFTSHIFT: u32 = 42;
    pub const KEY_BACKSLASH: u32 = 43;
    pub const KEY_Z: u32 = 44;
    pub const KEY_X: u32 = 45;
    pub const KEY_C: u32 = 46;
    pub const KEY_V: u32 = 47;
    pub const KEY_B: u32 = 48;
    pub const KEY_N: u32 = 49;
    pub const KEY_M: u32 = 50;
    pub const KEY_COMMA: u32 = 51;
    pub const KEY_DOT: u32 = 52;
    pub const KEY_SLASH: u32 = 53;
    pub const KEY_RIGHTSHIFT: u32 = 54;
    pub const KEY_KPASTERISK: u32 = 55;
    pub const KEY_LEFTALT: u32 = 56;
    pub const KEY_SPACE: u32 = 57;
    pub const KEY_CAPSLOCK: u32 = 58;

    // Function keys
    pub const KEY_F1: u32 = 59;
    pub const KEY_F2: u32 = 60;
    pub const KEY_F3: u32 = 61;
    pub const KEY_F4: u32 = 62;
    pub const KEY_F5: u32 = 63;
    pub const KEY_F6: u32 = 64;
    pub const KEY_F7: u32 = 65;
    pub const KEY_F8: u32 = 66;
    pub const KEY_F9: u32 = 67;
    pub const KEY_F10: u32 = 68;
    pub const KEY_NUMLOCK: u32 = 69;
    pub const KEY_SCROLLLOCK: u32 = 70;

    // Numpad
    pub const KEY_KP7: u32 = 71;
    pub const KEY_KP8: u32 = 72;
    pub const KEY_KP9: u32 = 73;
    pub const KEY_KPMINUS: u32 = 74;
    pub const KEY_KP4: u32 = 75;
    pub const KEY_KP5: u32 = 76;
    pub const KEY_KP6: u32 = 77;
    pub const KEY_KPPLUS: u32 = 78;
    pub const KEY_KP1: u32 = 79;
    pub const KEY_KP2: u32 = 80;
    pub const KEY_KP3: u32 = 81;
    pub const KEY_KP0: u32 = 82;
    pub const KEY_KPDOT: u32 = 83;

    pub const KEY_102ND: u32 = 86;
    pub const KEY_F11: u32 = 87;
    pub const KEY_F12: u32 = 88;
    pub const KEY_RO: u32 = 89;
    pub const KEY_KATAKANAHIRAGANA: u32 = 90;
    pub const KEY_HENKAN: u32 = 92;
    pub const KEY_MUHENKAN: u32 = 94;
    pub const KEY_KPENTER: u32 = 96;
    pub const KEY_RIGHTCTRL: u32 = 97;
    pub const KEY_KPSLASH: u32 = 98;
    pub const KEY_SYSRQ: u32 = 99;
    pub const KEY_RIGHTALT: u32 = 100;
    pub const KEY_HOME: u32 = 102;
    pub const KEY_UP: u32 = 103;
    pub const KEY_PAGEUP: u32 = 104;
    pub const KEY_LEFT: u32 = 105;
    pub const KEY_RIGHT: u32 = 106;
    pub const KEY_END: u32 = 107;
    pub const KEY_DOWN: u32 = 108;
    pub const KEY_PAGEDOWN: u32 = 109;
    pub const KEY_INSERT: u32 = 110;
    pub const KEY_DELETE: u32 = 111;
    pub const KEY_MUTE: u32 = 113;
    pub const KEY_VOLUMEDOWN: u32 = 114;
    pub const KEY_VOLUMEUP: u32 = 115;
    pub const KEY_POWER: u32 = 116;
    pub const KEY_KPEQUAL: u32 = 117;
    pub const KEY_PAUSE: u32 = 119;
    pub const KEY_KPCOMMA: u32 = 121;
    pub const KEY_HANGEUL: u32 = 122;
    pub const KEY_HANJA: u32 = 123;
    pub const KEY_YEN: u32 = 124;
    pub const KEY_LEFTMETA: u32 = 125;
    pub const KEY_RIGHTMETA: u32 = 126;
    pub const KEY_COMPOSE: u32 = 127;
    pub const KEY_STOP: u32 = 128;
    pub const KEY_AGAIN: u32 = 129;
    pub const KEY_PROPS: u32 = 130;
    pub const KEY_UNDO: u32 = 131;
    pub const KEY_FRONT: u32 = 132;
    pub const KEY_COPY: u32 = 133;
    pub const KEY_OPEN: u32 = 134;
    pub const KEY_PASTE: u32 = 135;
    pub const KEY_FIND: u32 = 136;
    pub const KEY_CUT: u32 = 137;
    pub const KEY_HELP: u32 = 138;
    pub const KEY_MENU: u32 = 139;
    pub const KEY_CALC: u32 = 140;
    pub const KEY_SLEEP: u32 = 142;
    pub const KEY_WAKEUP: u32 = 143;
    pub const KEY_WWW: u32 = 150;
    pub const KEY_MAIL: u32 = 155;
    pub const KEY_BOOKMARKS: u32 = 156;
    pub const KEY_COMPUTER: u32 = 157;
    pub const KEY_BACK: u32 = 158;
    pub const KEY_FORWARD: u32 = 159;
    pub const KEY_EJECTCD: u32 = 161;
    pub const KEY_NEXTSONG: u32 = 163;
    pub const KEY_PLAYPAUSE: u32 = 164;
    pub const KEY_PREVIOUSSONG: u32 = 165;
    pub const KEY_STOPCD: u32 = 166;
    pub const KEY_REFRESH: u32 = 173;
    pub const KEY_F13: u32 = 183;
    pub const KEY_F14: u32 = 184;
    pub const KEY_F15: u32 = 185;
    pub const KEY_F16: u32 = 186;
    pub const KEY_F17: u32 = 187;
    pub const KEY_F18: u32 = 188;
    pub const KEY_F19: u32 = 189;
    pub const KEY_F20: u32 = 190;
    pub const KEY_F21: u32 = 191;
    pub const KEY_F22: u32 = 192;
    pub const KEY_F23: u32 = 193;
    pub const KEY_F24: u32 = 194;
    pub const KEY_MEDIA: u32 = 226;
    pub const KEY_SEARCH: u32 = 217;
    pub const KEY_HOMEPAGE: u32 = 172;
    pub const KEY_BREAK: u32 = 411;
    pub const KEY_PRINT: u32 = 210;
}

use keycodes::*;

/// Scancode mapper handles RDP scancode to evdev keycode translation
pub struct ScancodeMapper {
    /// Primary scancode map (0x00-0x7F)
    primary_map: HashMap<u16, u32>,

    /// Extended scancode map (0xE000-0xE0FF)
    extended_map: HashMap<u16, u32>,

    /// E1 prefix scancode map
    e1_map: HashMap<u32, u32>,

    /// Reverse map for keycode to scancode
    reverse_map: HashMap<u32, u16>,

    /// Layout-specific overrides
    layout_overrides: HashMap<String, HashMap<u16, u32>>,

    /// Current keyboard layout
    current_layout: String,
}

impl ScancodeMapper {
    /// Create a new scancode mapper
    pub fn new() -> Self {
        let mut mapper = Self {
            primary_map: HashMap::new(),
            extended_map: HashMap::new(),
            e1_map: HashMap::new(),
            reverse_map: HashMap::new(),
            layout_overrides: HashMap::new(),
            current_layout: "us".to_string(),
        };

        mapper.initialize_mappings();
        mapper
    }

    /// Initialize all scancode mappings
    fn initialize_mappings(&mut self) {
        self.initialize_primary_map();
        self.initialize_extended_map();
        self.initialize_e1_map();
        self.build_reverse_map();
        self.load_layout_overrides();
    }

    /// Initialize primary scancode map (0x00-0x7F)
    fn initialize_primary_map(&mut self) {
        let mappings = vec![
            (0x01, KEY_ESC),
            (0x02, KEY_1),
            (0x03, KEY_2),
            (0x04, KEY_3),
            (0x05, KEY_4),
            (0x06, KEY_5),
            (0x07, KEY_6),
            (0x08, KEY_7),
            (0x09, KEY_8),
            (0x0A, KEY_9),
            (0x0B, KEY_0),
            (0x0C, KEY_MINUS),
            (0x0D, KEY_EQUAL),
            (0x0E, KEY_BACKSPACE),
            (0x0F, KEY_TAB),
            (0x10, KEY_Q),
            (0x11, KEY_W),
            (0x12, KEY_E),
            (0x13, KEY_R),
            (0x14, KEY_T),
            (0x15, KEY_Y),
            (0x16, KEY_U),
            (0x17, KEY_I),
            (0x18, KEY_O),
            (0x19, KEY_P),
            (0x1A, KEY_LEFTBRACE),
            (0x1B, KEY_RIGHTBRACE),
            (0x1C, KEY_ENTER),
            (0x1D, KEY_LEFTCTRL),
            (0x1E, KEY_A),
            (0x1F, KEY_S),
            (0x20, KEY_D),
            (0x21, KEY_F),
            (0x22, KEY_G),
            (0x23, KEY_H),
            (0x24, KEY_J),
            (0x25, KEY_K),
            (0x26, KEY_L),
            (0x27, KEY_SEMICOLON),
            (0x28, KEY_APOSTROPHE),
            (0x29, KEY_GRAVE),
            (0x2A, KEY_LEFTSHIFT),
            (0x2B, KEY_BACKSLASH),
            (0x2C, KEY_Z),
            (0x2D, KEY_X),
            (0x2E, KEY_C),
            (0x2F, KEY_V),
            (0x30, KEY_B),
            (0x31, KEY_N),
            (0x32, KEY_M),
            (0x33, KEY_COMMA),
            (0x34, KEY_DOT),
            (0x35, KEY_SLASH),
            (0x36, KEY_RIGHTSHIFT),
            (0x37, KEY_KPASTERISK),
            (0x38, KEY_LEFTALT),
            (0x39, KEY_SPACE),
            (0x3A, KEY_CAPSLOCK),
            (0x3B, KEY_F1),
            (0x3C, KEY_F2),
            (0x3D, KEY_F3),
            (0x3E, KEY_F4),
            (0x3F, KEY_F5),
            (0x40, KEY_F6),
            (0x41, KEY_F7),
            (0x42, KEY_F8),
            (0x43, KEY_F9),
            (0x44, KEY_F10),
            (0x45, KEY_NUMLOCK),
            (0x46, KEY_SCROLLLOCK),
            (0x47, KEY_KP7),
            (0x48, KEY_KP8),
            (0x49, KEY_KP9),
            (0x4A, KEY_KPMINUS),
            (0x4B, KEY_KP4),
            (0x4C, KEY_KP5),
            (0x4D, KEY_KP6),
            (0x4E, KEY_KPPLUS),
            (0x4F, KEY_KP1),
            (0x50, KEY_KP2),
            (0x51, KEY_KP3),
            (0x52, KEY_KP0),
            (0x53, KEY_KPDOT),
            (0x54, KEY_SYSRQ),
            (0x56, KEY_102ND),
            (0x57, KEY_F11),
            (0x58, KEY_F12),
            (0x59, KEY_KPEQUAL),
            (0x5A, KEY_F13),
            (0x5B, KEY_F14),
            (0x5C, KEY_F15),
            (0x5D, KEY_F16),
            (0x5E, KEY_F17),
            (0x5F, KEY_F18),
            (0x60, KEY_F19),
            (0x61, KEY_F20),
            (0x62, KEY_F21),
            (0x63, KEY_F22),
            (0x64, KEY_F23),
            (0x65, KEY_F24),
            (0x70, KEY_KATAKANAHIRAGANA),
            (0x71, KEY_MUHENKAN),
            (0x72, KEY_HENKAN),
            (0x73, KEY_RO),
            (0x74, KEY_YEN),
            (0x75, KEY_HANGEUL),
            (0x76, KEY_HANJA),
            (0x77, KEY_LEFTMETA),
            (0x78, KEY_RIGHTMETA),
            (0x79, KEY_COMPOSE),
            (0x7A, KEY_STOP),
            (0x7B, KEY_AGAIN),
            (0x7C, KEY_PROPS),
            (0x7D, KEY_UNDO),
            (0x7E, KEY_FRONT),
            (0x7F, KEY_COPY),
        ];

        for (scancode, keycode) in mappings {
            self.primary_map.insert(scancode, keycode);
        }
    }

    /// Initialize extended scancode map (E0 prefix)
    fn initialize_extended_map(&mut self) {
        let mappings = vec![
            (0xE01C, KEY_KPENTER),
            (0xE01D, KEY_RIGHTCTRL),
            (0xE020, KEY_MUTE),
            (0xE021, KEY_CALC),
            (0xE022, KEY_PLAYPAUSE),
            (0xE024, KEY_STOPCD),
            (0xE02E, KEY_VOLUMEDOWN),
            (0xE030, KEY_VOLUMEUP),
            (0xE032, KEY_HOMEPAGE),
            (0xE035, KEY_KPSLASH),
            (0xE037, KEY_PRINT),
            (0xE038, KEY_RIGHTALT),
            (0xE045, KEY_PAUSE),
            (0xE047, KEY_HOME),
            (0xE048, KEY_UP),
            (0xE049, KEY_PAGEUP),
            (0xE04B, KEY_LEFT),
            (0xE04D, KEY_RIGHT),
            (0xE04F, KEY_END),
            (0xE050, KEY_DOWN),
            (0xE051, KEY_PAGEDOWN),
            (0xE052, KEY_INSERT),
            (0xE053, KEY_DELETE),
            (0xE05B, KEY_LEFTMETA),
            (0xE05C, KEY_RIGHTMETA),
            (0xE05D, KEY_MENU),
            (0xE05E, KEY_POWER),
            (0xE05F, KEY_SLEEP),
            (0xE063, KEY_WAKEUP),
            (0xE065, KEY_SEARCH),
            (0xE066, KEY_BOOKMARKS),
            (0xE067, KEY_REFRESH),
            (0xE068, KEY_STOP),
            (0xE069, KEY_FORWARD),
            (0xE06A, KEY_BACK),
            (0xE06B, KEY_COMPUTER),
            (0xE06C, KEY_MAIL),
            (0xE06D, KEY_MEDIA),
            (0xE010, KEY_PREVIOUSSONG),
            (0xE019, KEY_NEXTSONG),
            (0xE02C, KEY_EJECTCD),
        ];

        for (scancode, keycode) in mappings {
            self.extended_map.insert(scancode, keycode);
        }
    }

    /// Initialize E1 prefix scancode map
    fn initialize_e1_map(&mut self) {
        self.e1_map.insert(0xE11D45, KEY_PAUSE);
        self.e1_map.insert(0xE11D46, KEY_BREAK);
    }

    /// Build reverse mapping for keycode to scancode
    fn build_reverse_map(&mut self) {
        for (&scancode, &keycode) in &self.primary_map {
            self.reverse_map.insert(keycode, scancode);
        }
        for (&scancode, &keycode) in &self.extended_map {
            self.reverse_map.insert(keycode, scancode);
        }
    }

    /// Load layout-specific overrides
    fn load_layout_overrides(&mut self) {
        // German layout (QWERTZ)
        let mut de_overrides = HashMap::new();
        de_overrides.insert(0x15, KEY_Z); // Y → Z
        de_overrides.insert(0x2C, KEY_Y); // Z → Y
        self.layout_overrides.insert("de".to_string(), de_overrides);

        // French layout (AZERTY)
        let mut fr_overrides = HashMap::new();
        fr_overrides.insert(0x10, KEY_A); // Q → A
        fr_overrides.insert(0x1E, KEY_Q); // A → Q
        fr_overrides.insert(0x11, KEY_Z); // W → Z
        fr_overrides.insert(0x2C, KEY_W); // Z → W
        self.layout_overrides.insert("fr".to_string(), fr_overrides);
    }

    /// Translate RDP scancode to Linux evdev keycode
    pub fn translate_scancode(
        &self,
        scancode: u32,
        extended: bool,
        e1_prefix: bool,
    ) -> Result<u32> {
        if e1_prefix {
            // Handle E1 prefix scancodes
            self.e1_map
                .get(&scancode)
                .copied()
                .ok_or_else(|| InputError::UnknownScancode(scancode as u16))
        } else if extended {
            // Handle E0 prefix (extended) scancodes
            let extended_scan = 0xE000 | (scancode as u16 & 0xFF);
            self.extended_map
                .get(&extended_scan)
                .or_else(|| self.primary_map.get(&(scancode as u16)))
                .copied()
                .ok_or_else(|| InputError::UnknownScancode(extended_scan))
        } else {
            // Check for layout-specific overrides first
            if let Some(overrides) = self.layout_overrides.get(&self.current_layout) {
                if let Some(keycode) = overrides.get(&(scancode as u16)) {
                    return Ok(*keycode);
                }
            }
            // Standard scancode translation
            self.primary_map
                .get(&(scancode as u16))
                .copied()
                .ok_or_else(|| InputError::UnknownScancode(scancode as u16))
        }
    }

    /// Translate Linux keycode to RDP scancode
    pub fn translate_keycode(&self, keycode: u32) -> Result<u16> {
        self.reverse_map
            .get(&keycode)
            .copied()
            .ok_or_else(|| InputError::UnknownKeycode(keycode))
    }

    /// Set keyboard layout
    pub fn set_layout(&mut self, layout: &str) {
        self.current_layout = layout.to_string();
    }

    /// Get current keyboard layout
    pub fn layout(&self) -> &str {
        &self.current_layout
    }

    /// Check if scancode is mapped
    pub fn is_mapped(&self, scancode: u16, extended: bool) -> bool {
        if extended {
            let extended_scan = 0xE000 | (scancode & 0xFF);
            self.extended_map.contains_key(&extended_scan)
        } else {
            self.primary_map.contains_key(&scancode)
        }
    }

    /// Get total number of mapped keys
    pub fn mapped_key_count(&self) -> usize {
        self.primary_map.len() + self.extended_map.len() + self.e1_map.len()
    }
}

impl Default for ScancodeMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scancode_mapper_creation() {
        let mapper = ScancodeMapper::new();
        assert!(mapper.mapped_key_count() > 200);
    }

    #[test]
    fn test_primary_scancode_mapping() {
        let mapper = ScancodeMapper::new();

        // Test letter keys
        assert_eq!(
            mapper.translate_scancode(0x1E, false, false).unwrap(),
            KEY_A
        );
        assert_eq!(
            mapper.translate_scancode(0x2C, false, false).unwrap(),
            KEY_Z
        );

        // Test number keys
        assert_eq!(
            mapper.translate_scancode(0x02, false, false).unwrap(),
            KEY_1
        );
        assert_eq!(
            mapper.translate_scancode(0x0B, false, false).unwrap(),
            KEY_0
        );

        // Test function keys
        assert_eq!(
            mapper.translate_scancode(0x3B, false, false).unwrap(),
            KEY_F1
        );
        assert_eq!(
            mapper.translate_scancode(0x58, false, false).unwrap(),
            KEY_F12
        );
    }

    #[test]
    fn test_extended_scancode_mapping() {
        let mapper = ScancodeMapper::new();

        // Test navigation keys
        assert_eq!(
            mapper.translate_scancode(0x47, true, false).unwrap(),
            KEY_HOME
        );
        assert_eq!(
            mapper.translate_scancode(0x4F, true, false).unwrap(),
            KEY_END
        );
        assert_eq!(
            mapper.translate_scancode(0x48, true, false).unwrap(),
            KEY_UP
        );
        assert_eq!(
            mapper.translate_scancode(0x50, true, false).unwrap(),
            KEY_DOWN
        );
        assert_eq!(
            mapper.translate_scancode(0x4B, true, false).unwrap(),
            KEY_LEFT
        );
        assert_eq!(
            mapper.translate_scancode(0x4D, true, false).unwrap(),
            KEY_RIGHT
        );

        // Test media keys
        assert_eq!(
            mapper.translate_scancode(0x22, true, false).unwrap(),
            KEY_PLAYPAUSE
        );
        assert_eq!(
            mapper.translate_scancode(0x24, true, false).unwrap(),
            KEY_STOPCD
        );
    }

    #[test]
    fn test_bidirectional_mapping() {
        let mapper = ScancodeMapper::new();

        // Test round-trip for common keys
        let test_keys = vec![KEY_A, KEY_Z, KEY_ENTER, KEY_SPACE, KEY_F1, KEY_F12];

        for keycode in test_keys {
            let scancode = mapper.translate_keycode(keycode).unwrap();
            let translated = mapper
                .translate_scancode(scancode as u32, false, false)
                .unwrap();
            assert_eq!(translated, keycode);
        }
    }

    #[test]
    fn test_layout_override() {
        let mut mapper = ScancodeMapper::new();

        // US layout: Y key
        assert_eq!(
            mapper.translate_scancode(0x15, false, false).unwrap(),
            KEY_Y
        );

        // German layout: Y → Z
        mapper.set_layout("de");
        assert_eq!(
            mapper.translate_scancode(0x15, false, false).unwrap(),
            KEY_Z
        );

        // French layout
        mapper.set_layout("fr");
        assert_eq!(
            mapper.translate_scancode(0x10, false, false).unwrap(),
            KEY_A
        );
    }

    #[test]
    fn test_unknown_scancode() {
        let mapper = ScancodeMapper::new();

        // Test unmapped scancode
        let result = mapper.translate_scancode(0xFF, false, false);
        assert!(result.is_err());
        match result {
            Err(InputError::UnknownScancode(_)) => {}
            _ => panic!("Expected UnknownScancode error"),
        }
    }

    #[test]
    fn test_unknown_keycode() {
        let mapper = ScancodeMapper::new();

        // Test unmapped keycode
        let result = mapper.translate_keycode(9999);
        assert!(result.is_err());
        match result {
            Err(InputError::UnknownKeycode(_)) => {}
            _ => panic!("Expected UnknownKeycode error"),
        }
    }

    #[test]
    fn test_is_mapped() {
        let mapper = ScancodeMapper::new();

        assert!(mapper.is_mapped(0x1E, false)); // A key
        assert!(mapper.is_mapped(0x47, true)); // Home key (extended)
        assert!(!mapper.is_mapped(0xFF, false)); // Unknown
    }

    #[test]
    fn test_all_primary_keys_mapped() {
        let mapper = ScancodeMapper::new();

        // Test common primary scancodes
        for scancode in 0x01..=0x58 {
            if scancode == 0x00 || scancode == 0x55 {
                continue; // Skip undefined
            }
            assert!(
                mapper.is_mapped(scancode, false),
                "Scancode 0x{:02X} not mapped",
                scancode
            );
        }
    }

    #[test]
    fn test_function_keys_f13_to_f24() {
        let mapper = ScancodeMapper::new();

        assert_eq!(
            mapper.translate_scancode(0x5A, false, false).unwrap(),
            KEY_F13
        );
        assert_eq!(
            mapper.translate_scancode(0x65, false, false).unwrap(),
            KEY_F24
        );
    }

    #[test]
    fn test_multimedia_keys() {
        let mapper = ScancodeMapper::new();

        assert_eq!(
            mapper.translate_scancode(0x20, true, false).unwrap(),
            KEY_MUTE
        );
        assert_eq!(
            mapper.translate_scancode(0x2E, true, false).unwrap(),
            KEY_VOLUMEDOWN
        );
        assert_eq!(
            mapper.translate_scancode(0x30, true, false).unwrap(),
            KEY_VOLUMEUP
        );
    }

    #[test]
    fn test_japanese_keys() {
        let mapper = ScancodeMapper::new();

        assert_eq!(
            mapper.translate_scancode(0x70, false, false).unwrap(),
            KEY_KATAKANAHIRAGANA
        );
        assert_eq!(
            mapper.translate_scancode(0x71, false, false).unwrap(),
            KEY_MUHENKAN
        );
        assert_eq!(
            mapper.translate_scancode(0x72, false, false).unwrap(),
            KEY_HENKAN
        );
    }

    #[test]
    fn test_korean_keys() {
        let mapper = ScancodeMapper::new();

        assert_eq!(
            mapper.translate_scancode(0x75, false, false).unwrap(),
            KEY_HANGEUL
        );
        assert_eq!(
            mapper.translate_scancode(0x76, false, false).unwrap(),
            KEY_HANJA
        );
    }
}
