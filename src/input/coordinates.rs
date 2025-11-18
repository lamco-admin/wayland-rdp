//! Coordinate Transformation
//!
//! Handles coordinate transformation between RDP client coordinates and
//! Wayland compositor coordinates with full multi-monitor support, DPI scaling,
//! and sub-pixel accuracy.

use crate::input::error::{InputError, Result};
use tracing::debug;

/// Monitor information
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    /// Monitor ID
    pub id: u32,

    /// Monitor name
    pub name: String,

    /// Physical position in virtual desktop (pixels)
    pub x: i32,
    pub y: i32,

    /// Monitor dimensions (pixels)
    pub width: u32,
    pub height: u32,

    /// DPI setting
    pub dpi: f64,

    /// Scale factor
    pub scale_factor: f64,

    /// Stream position (for video encoding)
    pub stream_x: u32,
    pub stream_y: u32,

    /// Stream dimensions
    pub stream_width: u32,
    pub stream_height: u32,

    /// Is this the primary monitor
    pub is_primary: bool,
}

impl MonitorInfo {
    /// Check if a point is within this monitor
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        x >= self.x as f64
            && x < (self.x + self.width as i32) as f64
            && y >= self.y as f64
            && y < (self.y + self.height as i32) as f64
    }

    /// Check if a stream coordinate is within this monitor's stream region
    pub fn contains_stream_point(&self, x: f64, y: f64) -> bool {
        let end_x = self.stream_x + self.stream_width;
        let end_y = self.stream_y + self.stream_height;

        x >= self.stream_x as f64
            && x < end_x as f64
            && y >= self.stream_y as f64
            && y < end_y as f64
    }
}

/// Coordinate system information
#[derive(Debug, Clone)]
pub struct CoordinateSystem {
    /// RDP coordinate space (client resolution)
    pub rdp_width: u32,
    pub rdp_height: u32,

    /// Virtual desktop space (all monitors combined)
    pub virtual_width: u32,
    pub virtual_height: u32,
    pub virtual_x_offset: i32,
    pub virtual_y_offset: i32,

    /// Stream coordinate space (encoding resolution)
    pub stream_width: u32,
    pub stream_height: u32,

    /// DPI scaling factors
    pub rdp_dpi: f64,
    pub system_dpi: f64,
}

/// Coordinate transformer handles all coordinate transformations
pub struct CoordinateTransformer {
    /// Coordinate system information
    coord_system: CoordinateSystem,

    /// Monitor configurations
    monitors: Vec<MonitorInfo>,

    /// Sub-pixel accumulator for smooth mouse movement
    sub_pixel_x: f64,
    sub_pixel_y: f64,

    /// Previous RDP position for delta calculation
    last_rdp_x: u32,
    last_rdp_y: u32,

    /// Enable mouse acceleration
    enable_acceleration: bool,

    /// Acceleration factor
    acceleration_factor: f64,

    /// Enable sub-pixel precision
    enable_sub_pixel: bool,
}

impl CoordinateTransformer {
    /// Create a new coordinate transformer
    pub fn new(monitors: Vec<MonitorInfo>) -> Result<Self> {
        if monitors.is_empty() {
            return Err(InputError::InvalidMonitorConfig(
                "No monitors configured".to_string(),
            ));
        }

        let coord_system = Self::calculate_coordinate_system(&monitors);

        Ok(Self {
            coord_system,
            monitors,
            sub_pixel_x: 0.0,
            sub_pixel_y: 0.0,
            last_rdp_x: 0,
            last_rdp_y: 0,
            enable_acceleration: true,
            acceleration_factor: 1.0,
            enable_sub_pixel: true,
        })
    }

    /// Calculate coordinate system from monitor configuration
    fn calculate_coordinate_system(monitors: &[MonitorInfo]) -> CoordinateSystem {
        // Calculate virtual desktop bounds
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for monitor in monitors {
            min_x = min_x.min(monitor.x);
            min_y = min_y.min(monitor.y);
            max_x = max_x.max(monitor.x + monitor.width as i32);
            max_y = max_y.max(monitor.y + monitor.height as i32);
        }

        let virtual_width = (max_x - min_x) as u32;
        let virtual_height = (max_y - min_y) as u32;

        // Calculate stream dimensions
        let stream_width = monitors.iter().map(|m| m.stream_width).max().unwrap_or(0);
        let stream_height = monitors.iter().map(|m| m.stream_height).max().unwrap_or(0);

        // Get primary monitor for RDP dimensions and DPI
        let primary = monitors
            .iter()
            .find(|m| m.is_primary)
            .unwrap_or(&monitors[0]);

        CoordinateSystem {
            rdp_width: primary.width,
            rdp_height: primary.height,
            virtual_width,
            virtual_height,
            virtual_x_offset: min_x,
            virtual_y_offset: min_y,
            stream_width,
            stream_height,
            rdp_dpi: primary.dpi,
            system_dpi: 96.0, // Default system DPI
        }
    }

    /// Transform RDP coordinates to stream coordinates
    pub fn rdp_to_stream(&mut self, rdp_x: u32, rdp_y: u32) -> Result<(f64, f64)> {
        // Step 1: Normalize RDP coordinates to [0, 1] range
        let norm_x = rdp_x as f64 / self.coord_system.rdp_width as f64;
        let norm_y = rdp_y as f64 / self.coord_system.rdp_height as f64;

        // Step 2: Apply DPI scaling
        let dpi_scale = self.coord_system.system_dpi / self.coord_system.rdp_dpi;
        let scaled_x = norm_x * dpi_scale;
        let scaled_y = norm_y * dpi_scale;

        // Step 3: Map to virtual desktop space
        let virtual_x = scaled_x * self.coord_system.virtual_width as f64
            + self.coord_system.virtual_x_offset as f64;
        let virtual_y = scaled_y * self.coord_system.virtual_height as f64
            + self.coord_system.virtual_y_offset as f64;

        // Step 4: Find target monitor
        let monitor = self.find_monitor_at_point(virtual_x, virtual_y)?;

        // Step 5: Transform to monitor-local coordinates
        let local_x = virtual_x - monitor.x as f64;
        let local_y = virtual_y - monitor.y as f64;

        // Step 6: Apply monitor scaling
        let monitor_scale_x = monitor.stream_width as f64 / monitor.width as f64;
        let monitor_scale_y = monitor.stream_height as f64 / monitor.height as f64;

        let stream_x = monitor.stream_x as f64 + (local_x * monitor_scale_x * monitor.scale_factor);
        let stream_y = monitor.stream_y as f64 + (local_y * monitor_scale_y * monitor.scale_factor);

        // Step 7: Apply sub-pixel accumulation for smooth movement
        if self.enable_sub_pixel {
            self.sub_pixel_x += stream_x - stream_x.floor();
            self.sub_pixel_y += stream_y - stream_y.floor();

            let final_x = stream_x.floor()
                + if self.sub_pixel_x >= 1.0 {
                    self.sub_pixel_x -= 1.0;
                    1.0
                } else {
                    0.0
                };

            let final_y = stream_y.floor()
                + if self.sub_pixel_y >= 1.0 {
                    self.sub_pixel_y -= 1.0;
                    1.0
                } else {
                    0.0
                };

            Ok((final_x, final_y))
        } else {
            Ok((stream_x, stream_y))
        }
    }

    /// Transform stream coordinates back to RDP coordinates
    pub fn stream_to_rdp(&self, stream_x: f64, stream_y: f64) -> Result<(u32, u32)> {
        // Step 1: Find source monitor from stream coordinates
        let monitor = self.find_monitor_from_stream(stream_x, stream_y)?;

        // Step 2: Convert to monitor-local coordinates
        let local_stream_x = stream_x - monitor.stream_x as f64;
        let local_stream_y = stream_y - monitor.stream_y as f64;

        // Step 3: Reverse monitor scaling
        let monitor_scale_x = monitor.width as f64 / monitor.stream_width as f64;
        let monitor_scale_y = monitor.height as f64 / monitor.stream_height as f64;

        let local_x = local_stream_x * monitor_scale_x / monitor.scale_factor;
        let local_y = local_stream_y * monitor_scale_y / monitor.scale_factor;

        // Step 4: Convert to virtual desktop coordinates
        let virtual_x = monitor.x as f64 + local_x;
        let virtual_y = monitor.y as f64 + local_y;

        // Step 5: Normalize from virtual desktop
        let norm_x = (virtual_x - self.coord_system.virtual_x_offset as f64)
            / self.coord_system.virtual_width as f64;
        let norm_y = (virtual_y - self.coord_system.virtual_y_offset as f64)
            / self.coord_system.virtual_height as f64;

        // Step 6: Reverse DPI scaling
        let dpi_scale = self.coord_system.rdp_dpi / self.coord_system.system_dpi;
        let scaled_x = norm_x * dpi_scale;
        let scaled_y = norm_y * dpi_scale;

        // Step 7: Convert to RDP coordinates
        let rdp_x = (scaled_x * self.coord_system.rdp_width as f64).round() as u32;
        let rdp_y = (scaled_y * self.coord_system.rdp_height as f64).round() as u32;

        // Clamp to valid range
        let rdp_x = rdp_x.min(self.coord_system.rdp_width.saturating_sub(1));
        let rdp_y = rdp_y.min(self.coord_system.rdp_height.saturating_sub(1));

        Ok((rdp_x, rdp_y))
    }

    /// Apply relative mouse movement with optional acceleration
    pub fn apply_relative_movement(&mut self, delta_x: i32, delta_y: i32) -> Result<(f64, f64)> {
        // Apply acceleration if enabled
        let accel_x = if self.enable_acceleration {
            delta_x as f64 * self.calculate_acceleration(delta_x.abs())
        } else {
            delta_x as f64
        };

        let accel_y = if self.enable_acceleration {
            delta_y as f64 * self.calculate_acceleration(delta_y.abs())
        } else {
            delta_y as f64
        };

        // Update RDP position
        let new_rdp_x = (self.last_rdp_x as i32 + accel_x as i32).max(0) as u32;
        let new_rdp_y = (self.last_rdp_y as i32 + accel_y as i32).max(0) as u32;

        // Clamp to bounds
        let new_rdp_x = new_rdp_x.min(self.coord_system.rdp_width.saturating_sub(1));
        let new_rdp_y = new_rdp_y.min(self.coord_system.rdp_height.saturating_sub(1));

        self.last_rdp_x = new_rdp_x;
        self.last_rdp_y = new_rdp_y;

        // Transform to stream coordinates
        self.rdp_to_stream(new_rdp_x, new_rdp_y)
    }

    /// Calculate mouse acceleration based on movement speed
    fn calculate_acceleration(&self, speed: i32) -> f64 {
        // Windows-style mouse acceleration curve
        let base = self.acceleration_factor;
        if speed < 2 {
            base
        } else if speed < 4 {
            base * 1.5
        } else if speed < 6 {
            base * 2.0
        } else if speed < 9 {
            base * 2.5
        } else if speed < 13 {
            base * 3.0
        } else {
            base * 3.5
        }
    }

    /// Find monitor containing the given point
    fn find_monitor_at_point(&self, x: f64, y: f64) -> Result<&MonitorInfo> {
        for monitor in &self.monitors {
            if monitor.contains_point(x, y) {
                return Ok(monitor);
            }
        }

        // Default to primary monitor if point is outside all monitors
        self.monitors
            .iter()
            .find(|m| m.is_primary)
            .or_else(|| self.monitors.first())
            .ok_or_else(|| InputError::InvalidCoordinate(x, y))
    }

    /// Find monitor from stream coordinates
    fn find_monitor_from_stream(&self, stream_x: f64, stream_y: f64) -> Result<&MonitorInfo> {
        for monitor in &self.monitors {
            if monitor.contains_stream_point(stream_x, stream_y) {
                return Ok(monitor);
            }
        }

        // Default to first monitor
        self.monitors
            .first()
            .ok_or_else(|| InputError::InvalidCoordinate(stream_x, stream_y))
    }

    /// Clamp coordinates to monitor bounds
    pub fn clamp_to_bounds(&self, x: f64, y: f64) -> (f64, f64) {
        let clamped_x = x.max(0.0).min(self.coord_system.stream_width as f64 - 1.0);
        let clamped_y = y.max(0.0).min(self.coord_system.stream_height as f64 - 1.0);
        (clamped_x, clamped_y)
    }

    /// Update monitor configuration
    pub fn update_monitors(&mut self, monitors: Vec<MonitorInfo>) -> Result<()> {
        if monitors.is_empty() {
            return Err(InputError::InvalidMonitorConfig(
                "No monitors configured".to_string(),
            ));
        }

        self.coord_system = Self::calculate_coordinate_system(&monitors);
        self.monitors = monitors;
        self.sub_pixel_x = 0.0;
        self.sub_pixel_y = 0.0;

        debug!(
            "Updated monitor configuration: {} monitors",
            self.monitors.len()
        );
        Ok(())
    }

    /// Set mouse acceleration enabled
    pub fn set_acceleration_enabled(&mut self, enabled: bool) {
        self.enable_acceleration = enabled;
    }

    /// Set acceleration factor
    pub fn set_acceleration_factor(&mut self, factor: f64) {
        self.acceleration_factor = factor;
    }

    /// Set sub-pixel precision enabled
    pub fn set_sub_pixel_enabled(&mut self, enabled: bool) {
        self.enable_sub_pixel = enabled;
    }

    /// Get monitor count
    pub fn monitor_count(&self) -> usize {
        self.monitors.len()
    }

    /// Get monitor by ID
    pub fn get_monitor(&self, id: u32) -> Option<&MonitorInfo> {
        self.monitors.iter().find(|m| m.id == id)
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
    fn test_coordinate_transformer_creation() {
        let monitor = create_test_monitor();
        let transformer = CoordinateTransformer::new(vec![monitor]).unwrap();

        assert_eq!(transformer.monitor_count(), 1);
    }

    #[test]
    fn test_rdp_to_stream_single_monitor() {
        let monitor = create_test_monitor();
        let mut transformer = CoordinateTransformer::new(vec![monitor]).unwrap();

        // Test corner cases
        let (x, y) = transformer.rdp_to_stream(0, 0).unwrap();
        assert!(x >= 0.0 && x <= 1.0);
        assert!(y >= 0.0 && y <= 1.0);

        let (x, y) = transformer.rdp_to_stream(1919, 1079).unwrap();
        assert!(x <= 1920.0);
        assert!(y <= 1080.0);

        // Test center
        let (x, y) = transformer.rdp_to_stream(960, 540).unwrap();
        assert!(x > 900.0 && x < 1000.0);
        assert!(y > 500.0 && y < 600.0);
    }

    #[test]
    fn test_stream_to_rdp_single_monitor() {
        let monitor = create_test_monitor();
        let transformer = CoordinateTransformer::new(vec![monitor]).unwrap();

        // Test round-trip
        let (rdp_x, rdp_y) = transformer.stream_to_rdp(100.0, 100.0).unwrap();
        assert!(rdp_x < 1920);
        assert!(rdp_y < 1080);

        let (rdp_x, rdp_y) = transformer.stream_to_rdp(1900.0, 1000.0).unwrap();
        assert!(rdp_x < 1920);
        assert!(rdp_y < 1080);
    }

    #[test]
    fn test_round_trip_transformation() {
        let monitor = create_test_monitor();
        let mut transformer = CoordinateTransformer::new(vec![monitor]).unwrap();
        transformer.set_sub_pixel_enabled(false); // Disable for exact round-trip

        // Test several points
        let test_points = vec![(0, 0), (960, 540), (1919, 1079)];

        for (orig_x, orig_y) in test_points {
            let (stream_x, stream_y) = transformer.rdp_to_stream(orig_x, orig_y).unwrap();
            let (rdp_x, rdp_y) = transformer.stream_to_rdp(stream_x, stream_y).unwrap();

            // Allow for small rounding errors
            assert!((rdp_x as i32 - orig_x as i32).abs() <= 1);
            assert!((rdp_y as i32 - orig_y as i32).abs() <= 1);
        }
    }

    #[test]
    fn test_multi_monitor_configuration() {
        let monitors = vec![
            MonitorInfo {
                id: 1,
                name: "Left".to_string(),
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
            },
            MonitorInfo {
                id: 2,
                name: "Right".to_string(),
                x: 1920,
                y: 0,
                width: 1920,
                height: 1080,
                dpi: 96.0,
                scale_factor: 1.0,
                stream_x: 1920,
                stream_y: 0,
                stream_width: 1920,
                stream_height: 1080,
                is_primary: false,
            },
        ];

        let transformer = CoordinateTransformer::new(monitors).unwrap();
        assert_eq!(transformer.monitor_count(), 2);
    }

    #[test]
    fn test_relative_movement() {
        let monitor = create_test_monitor();
        let mut transformer = CoordinateTransformer::new(vec![monitor]).unwrap();
        transformer.last_rdp_x = 960;
        transformer.last_rdp_y = 540;

        let (x, y) = transformer.apply_relative_movement(10, 10).unwrap();
        assert!(x > 960.0);
        assert!(y > 540.0);
    }

    #[test]
    fn test_mouse_acceleration() {
        let monitor = create_test_monitor();
        let mut transformer = CoordinateTransformer::new(vec![monitor]).unwrap();
        transformer.set_acceleration_enabled(true);
        transformer.set_acceleration_factor(1.0);

        // Small movement should have no acceleration
        let accel_small = transformer.calculate_acceleration(1);
        assert_eq!(accel_small, 1.0);

        // Large movement should have acceleration
        let accel_large = transformer.calculate_acceleration(15);
        assert!(accel_large > 1.0);
    }

    #[test]
    fn test_clamp_to_bounds() {
        let monitor = create_test_monitor();
        let transformer = CoordinateTransformer::new(vec![monitor]).unwrap();

        // Test clamping out-of-bounds coordinates
        let (x, y) = transformer.clamp_to_bounds(-10.0, -10.0);
        assert_eq!(x, 0.0);
        assert_eq!(y, 0.0);

        let (x, y) = transformer.clamp_to_bounds(2000.0, 2000.0);
        assert!(x < 1920.0);
        assert!(y < 1080.0);
    }

    #[test]
    fn test_monitor_contains_point() {
        let monitor = create_test_monitor();

        assert!(monitor.contains_point(100.0, 100.0));
        assert!(monitor.contains_point(1919.0, 1079.0));
        assert!(!monitor.contains_point(-1.0, 0.0));
        assert!(!monitor.contains_point(1920.0, 0.0));
    }

    #[test]
    fn test_update_monitors() {
        let monitor = create_test_monitor();
        let mut transformer = CoordinateTransformer::new(vec![monitor]).unwrap();

        let new_monitors = vec![
            create_test_monitor(),
            MonitorInfo {
                id: 2,
                name: "Secondary".to_string(),
                x: 1920,
                y: 0,
                width: 1920,
                height: 1080,
                dpi: 96.0,
                scale_factor: 1.0,
                stream_x: 1920,
                stream_y: 0,
                stream_width: 1920,
                stream_height: 1080,
                is_primary: false,
            },
        ];

        transformer.update_monitors(new_monitors).unwrap();
        assert_eq!(transformer.monitor_count(), 2);
    }

    #[test]
    fn test_empty_monitor_list_error() {
        let result = CoordinateTransformer::new(vec![]);
        assert!(result.is_err());
    }
}
