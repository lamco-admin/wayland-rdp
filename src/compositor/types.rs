//! Core types for the WRD compositor
//!
//! This module defines all the fundamental types used throughout the compositor.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Compositor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositorConfig {
    /// Virtual display width in pixels
    pub width: u32,

    /// Virtual display height in pixels
    pub height: u32,

    /// Refresh rate in Hz
    pub refresh_rate: u32,

    /// Scale factor (1.0 = 100%, 2.0 = 200%)
    pub scale: f64,

    /// Wayland socket name (e.g., "wayland-0")
    pub socket_name: String,

    /// Enable XWayland support
    pub xwayland: bool,

    /// Maximum number of client connections
    pub max_clients: usize,

    /// Frame buffer format
    pub pixel_format: PixelFormat,
}

impl Default for CompositorConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            refresh_rate: 60,
            scale: 1.0,
            socket_name: "wayland-0".to_string(),
            xwayland: false,
            max_clients: 32,
            pixel_format: PixelFormat::BGRA8888,
        }
    }
}

/// Pixel format for framebuffer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PixelFormat {
    /// 32-bit BGRA (Blue, Green, Red, Alpha)
    BGRA8888,
    /// 32-bit RGBA (Red, Green, Blue, Alpha)
    RGBA8888,
    /// 32-bit BGRX (Blue, Green, Red, unused)
    BGRX8888,
    /// 32-bit RGBX (Red, Green, Blue, unused)
    RGBX8888,
}

impl PixelFormat {
    /// Get bytes per pixel
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::BGRA8888 | Self::RGBA8888 | Self::BGRX8888 | Self::RGBX8888 => 4,
        }
    }

    /// Check if format has alpha channel
    pub fn has_alpha(&self) -> bool {
        matches!(self, Self::BGRA8888 | Self::RGBA8888)
    }
}

/// Window identifier (unique per window)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WindowId(pub u64);

impl WindowId {
    /// Create a new window ID
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for WindowId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Window({})", self.0)
    }
}

/// Surface identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceId(pub u64);

impl SurfaceId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for SurfaceId {
    fn default() -> Self {
        Self::new()
    }
}

/// Rectangle in logical coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rectangle {
    /// Create a new rectangle
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Check if rectangle contains a point
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x
            && x < self.x + self.width as i32
            && y >= self.y
            && y < self.y + self.height as i32
    }

    /// Check if rectangle intersects another
    pub fn intersects(&self, other: &Rectangle) -> bool {
        self.x < other.x + other.width as i32
            && self.x + self.width as i32 > other.x
            && self.y < other.y + other.height as i32
            && self.y + self.height as i32 > other.y
    }

    /// Compute intersection with another rectangle
    pub fn intersection(&self, other: &Rectangle) -> Option<Rectangle> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width as i32).min(other.x + other.width as i32);
        let bottom = (self.y + self.height as i32).min(other.y + other.height as i32);

        if right > x && bottom > y {
            Some(Rectangle {
                x,
                y,
                width: (right - x) as u32,
                height: (bottom - y) as u32,
            })
        } else {
            None
        }
    }

    /// Get area of rectangle
    pub fn area(&self) -> u32 {
        self.width * self.height
    }
}

/// Point in logical coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Size in logical coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Window state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowState {
    /// Normal window
    Normal,
    /// Maximized window (fills entire display)
    Maximized,
    /// Fullscreen window (covers entire display, no decorations)
    Fullscreen,
    /// Minimized window (hidden)
    Minimized,
}

/// Compositor events
#[derive(Debug, Clone)]
pub enum CompositorEvent {
    /// Window created
    WindowCreated(WindowId),

    /// Window destroyed
    WindowDestroyed(WindowId),

    /// Window state changed
    WindowStateChanged(WindowId, WindowState),

    /// Window geometry changed
    WindowGeometryChanged(WindowId, Rectangle),

    /// Window title changed
    WindowTitleChanged(WindowId, String),

    /// Window focus changed
    FocusChanged(Option<WindowId>),

    /// Clipboard changed
    ClipboardChanged,

    /// Frame rendered
    FrameRendered,

    /// Compositor shutting down
    Shutdown,
}

/// Input event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

/// Mouse button state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Pressed,
    Released,
}

/// Keyboard modifiers
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub logo: bool, // Windows/Super key
}

impl Modifiers {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        !self.ctrl && !self.alt && !self.shift && !self.logo
    }
}

/// Rendering damage information
#[derive(Debug, Clone)]
pub struct DamageInfo {
    /// List of damaged rectangles
    pub regions: Vec<Rectangle>,

    /// Total damaged area
    pub total_area: u32,

    /// Whether entire framebuffer needs redraw
    pub full_damage: bool,
}

impl DamageInfo {
    /// Create damage info for full framebuffer
    pub fn full(width: u32, height: u32) -> Self {
        Self {
            regions: vec![Rectangle::new(0, 0, width, height)],
            total_area: width * height,
            full_damage: true,
        }
    }

    /// Create damage info from regions
    pub fn from_regions(regions: Vec<Rectangle>) -> Self {
        let total_area = regions.iter().map(|r| r.area()).sum();
        Self {
            regions,
            total_area,
            full_damage: false,
        }
    }

    /// Check if no damage
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty() || self.total_area == 0
    }
}

/// Frame buffer data
#[derive(Debug, Clone)]
pub struct FrameBuffer {
    /// Raw pixel data (format depends on PixelFormat)
    pub data: Vec<u8>,

    /// Width in pixels
    pub width: u32,

    /// Height in pixels
    pub height: u32,

    /// Pixel format
    pub format: PixelFormat,

    /// Damaged regions
    pub damage: DamageInfo,

    /// Frame sequence number
    pub sequence: u64,
}

impl FrameBuffer {
    /// Create new framebuffer
    pub fn new(width: u32, height: u32, format: PixelFormat) -> Self {
        let size = (width * height) as usize * format.bytes_per_pixel();
        Self {
            data: vec![0; size],
            width,
            height,
            format,
            damage: DamageInfo::full(width, height),
            sequence: 0,
        }
    }

    /// Get total size in bytes
    pub fn size_bytes(&self) -> usize {
        self.data.len()
    }

    /// Get stride (bytes per row)
    pub fn stride(&self) -> usize {
        self.width as usize * self.format.bytes_per_pixel()
    }
}

/// Cursor state
#[derive(Debug, Clone)]
pub struct CursorState {
    /// Cursor position (logical coordinates)
    pub position: Point,

    /// Cursor image (RGBA pixels)
    pub image: Option<Vec<u8>>,

    /// Cursor hotspot
    pub hotspot: Point,

    /// Cursor size
    pub size: Size,

    /// Whether cursor is visible
    pub visible: bool,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            position: Point::new(0, 0),
            image: None,
            hotspot: Point::new(0, 0),
            size: Size::new(0, 0),
            visible: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rectangle_contains() {
        let rect = Rectangle::new(10, 10, 100, 100);
        assert!(rect.contains(50, 50));
        assert!(rect.contains(10, 10));
        assert!(!rect.contains(5, 5));
        assert!(!rect.contains(120, 120));
    }

    #[test]
    fn test_rectangle_intersection() {
        let rect1 = Rectangle::new(0, 0, 100, 100);
        let rect2 = Rectangle::new(50, 50, 100, 100);

        let intersection = rect1.intersection(&rect2).unwrap();
        assert_eq!(intersection, Rectangle::new(50, 50, 50, 50));
    }

    #[test]
    fn test_pixel_format() {
        assert_eq!(PixelFormat::BGRA8888.bytes_per_pixel(), 4);
        assert!(PixelFormat::BGRA8888.has_alpha());
        assert!(!PixelFormat::BGRX8888.has_alpha());
    }

    #[test]
    fn test_window_id_unique() {
        let id1 = WindowId::new();
        let id2 = WindowId::new();
        assert_ne!(id1, id2);
    }
}
