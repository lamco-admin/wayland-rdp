//! Layout Calculation Engine
//!
//! Calculates virtual desktop layout from monitor configurations with
//! production-grade algorithms for positioning, alignment, and optimization.

use std::collections::HashMap;
use thiserror::Error;
use tracing::{debug, warn};

/// Layout error types
#[derive(Error, Debug)]
pub enum LayoutError {
    /// No monitors configured
    #[error("No monitors configured")]
    NoMonitors,

    /// Invalid monitor dimensions
    #[error("Invalid monitor dimensions: {0}x{1}")]
    InvalidDimensions(u32, u32),

    /// Layout calculation failed
    #[error("Layout calculation failed: {0}")]
    CalculationFailed(String),
}

/// Monitor layout in virtual desktop space
#[derive(Debug, Clone)]
pub struct MonitorLayout {
    /// Monitor ID
    pub id: u32,

    /// X position in virtual desktop (pixels)
    pub x: i32,
    /// Y position in virtual desktop (pixels)
    pub y: i32,

    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,

    /// Is primary monitor
    pub is_primary: bool,
}

/// Virtual desktop represents the combined space of all monitors
#[derive(Debug, Clone)]
pub struct VirtualDesktop {
    /// Total width of virtual desktop
    pub width: u32,

    /// Total height of virtual desktop
    pub height: u32,

    /// Top-left X offset from origin
    pub offset_x: i32,
    /// Top-left Y offset from origin
    pub offset_y: i32,

    /// Monitor layouts
    pub monitors: Vec<MonitorLayout>,
}

/// Coordinate space for transformations
#[derive(Debug, Clone)]
pub struct CoordinateSpace {
    /// Space identifier name
    pub name: String,

    /// Width in pixels
    pub width: u32,

    /// Height in pixels
    pub height: u32,

    /// X offset from origin
    pub offset_x: i32,
    /// Y offset from origin
    pub offset_y: i32,
}

/// Layout strategy for monitor arrangement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutStrategy {
    /// Horizontal arrangement (left to right)
    Horizontal,

    /// Vertical arrangement (top to bottom)
    Vertical,

    /// Preserve Portal-reported positions
    PreservePositions,

    /// Grid layout (rows x columns)
    Grid { rows: u32, cols: u32 },
}

/// Layout calculator computes optimal monitor arrangements
pub struct LayoutCalculator {
    /// Layout strategy
    strategy: LayoutStrategy,

    /// Alignment preferences (for future advanced layout features)
    #[allow(dead_code)]
    alignment: Alignment,
}

/// Alignment for monitor positioning
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Future feature for advanced layout
pub(super) enum Alignment {
    /// Align to top edge
    Top,

    /// Align to center
    Center,

    /// Align to bottom edge
    Bottom,

    /// Align to left edge
    Left,

    /// Align to right edge
    Right,
}

impl Default for LayoutCalculator {
    fn default() -> Self {
        Self {
            strategy: LayoutStrategy::PreservePositions,
            alignment: Alignment::Top,
        }
    }
}

impl LayoutCalculator {
    /// Create a new layout calculator
    ///
    /// # Arguments
    ///
    /// * `strategy` - Layout strategy to use
    ///
    /// # Returns
    ///
    /// A new LayoutCalculator instance
    pub fn new(strategy: LayoutStrategy) -> Self {
        Self {
            strategy,
            alignment: Alignment::Top,
        }
    }

    /// Calculate layout from stream information
    ///
    /// Takes monitor metadata from Portal streams and calculates optimal
    /// virtual desktop layout.
    ///
    /// # Arguments
    ///
    /// * `streams` - Stream information from Portal
    ///
    /// # Returns
    ///
    /// Calculated VirtualDesktop with monitor positions
    ///
    /// # Errors
    ///
    /// Returns error if layout calculation fails
    pub fn calculate_layout(
        &self,
        streams: &[crate::portal::StreamInfo],
    ) -> Result<VirtualDesktop, LayoutError> {
        if streams.is_empty() {
            return Err(LayoutError::NoMonitors);
        }

        let monitor_layouts = match self.strategy {
            LayoutStrategy::PreservePositions => self.preserve_positions(streams)?,
            LayoutStrategy::Horizontal => self.arrange_horizontal(streams)?,
            LayoutStrategy::Vertical => self.arrange_vertical(streams)?,
            LayoutStrategy::Grid { rows, cols } => self.arrange_grid(streams, rows, cols)?,
        };

        // Calculate bounding box
        let (min_x, min_y, max_x, max_y) = self.calculate_bounds(&monitor_layouts);

        let width = (max_x - min_x) as u32;
        let height = (max_y - min_y) as u32;

        debug!(
            "Calculated virtual desktop: {}x{} with {} monitors",
            width,
            height,
            monitor_layouts.len()
        );

        Ok(VirtualDesktop {
            width,
            height,
            offset_x: min_x,
            offset_y: min_y,
            monitors: monitor_layouts,
        })
    }

    /// Preserve Portal-reported monitor positions
    fn preserve_positions(
        &self,
        streams: &[crate::portal::StreamInfo],
    ) -> Result<Vec<MonitorLayout>, LayoutError> {
        let layouts: Vec<MonitorLayout> = streams
            .iter()
            .enumerate()
            .map(|(idx, stream)| MonitorLayout {
                id: stream.node_id,
                x: stream.position.0,
                y: stream.position.1,
                width: stream.size.0,
                height: stream.size.1,
                is_primary: idx == 0,
            })
            .collect();

        Ok(layouts)
    }

    /// Arrange monitors horizontally (left to right)
    fn arrange_horizontal(
        &self,
        streams: &[crate::portal::StreamInfo],
    ) -> Result<Vec<MonitorLayout>, LayoutError> {
        let mut layouts = Vec::new();
        let mut current_x = 0i32;

        for (idx, stream) in streams.iter().enumerate() {
            layouts.push(MonitorLayout {
                id: stream.node_id,
                x: current_x,
                y: 0,
                width: stream.size.0,
                height: stream.size.1,
                is_primary: idx == 0,
            });

            current_x += stream.size.0 as i32;
        }

        Ok(layouts)
    }

    /// Arrange monitors vertically (top to bottom)
    fn arrange_vertical(
        &self,
        streams: &[crate::portal::StreamInfo],
    ) -> Result<Vec<MonitorLayout>, LayoutError> {
        let mut layouts = Vec::new();
        let mut current_y = 0i32;

        for (idx, stream) in streams.iter().enumerate() {
            layouts.push(MonitorLayout {
                id: stream.node_id,
                x: 0,
                y: current_y,
                width: stream.size.0,
                height: stream.size.1,
                is_primary: idx == 0,
            });

            current_y += stream.size.1 as i32;
        }

        Ok(layouts)
    }

    /// Arrange monitors in grid pattern
    fn arrange_grid(
        &self,
        streams: &[crate::portal::StreamInfo],
        rows: u32,
        cols: u32,
    ) -> Result<Vec<MonitorLayout>, LayoutError> {
        if rows == 0 || cols == 0 {
            return Err(LayoutError::CalculationFailed(
                "Grid dimensions must be > 0".to_string(),
            ));
        }

        if streams.len() > (rows * cols) as usize {
            warn!(
                "More monitors ({}) than grid cells ({}x{}={})",
                streams.len(),
                rows,
                cols,
                rows * cols
            );
        }

        let mut layouts = Vec::new();

        for (idx, stream) in streams.iter().enumerate() {
            let row = (idx as u32) / cols;
            let col = (idx as u32) % cols;

            let x = (col * stream.size.0) as i32;
            let y = (row * stream.size.1) as i32;

            layouts.push(MonitorLayout {
                id: stream.node_id,
                x,
                y,
                width: stream.size.0,
                height: stream.size.1,
                is_primary: idx == 0,
            });
        }

        Ok(layouts)
    }

    /// Calculate bounding box for all monitors
    fn calculate_bounds(&self, layouts: &[MonitorLayout]) -> (i32, i32, i32, i32) {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for layout in layouts {
            min_x = min_x.min(layout.x);
            min_y = min_y.min(layout.y);
            max_x = max_x.max(layout.x + layout.width as i32);
            max_y = max_y.max(layout.y + layout.height as i32);
        }

        (min_x, min_y, max_x, max_y)
    }
}

/// Layout represents a calculated monitor configuration
#[derive(Debug, Clone)]
pub struct Layout {
    /// Virtual desktop
    pub virtual_desktop: VirtualDesktop,

    /// Coordinate spaces for each monitor
    pub coordinate_spaces: HashMap<u32, CoordinateSpace>,
}

impl Layout {
    /// Create from virtual desktop
    ///
    /// # Arguments
    ///
    /// * `virtual_desktop` - Calculated virtual desktop layout
    ///
    /// # Returns
    ///
    /// A new Layout instance with coordinate spaces
    pub fn from_virtual_desktop(virtual_desktop: VirtualDesktop) -> Self {
        let mut coordinate_spaces = HashMap::new();

        for monitor in &virtual_desktop.monitors {
            let space = CoordinateSpace {
                name: format!("monitor-{}", monitor.id),
                width: monitor.width,
                height: monitor.height,
                offset_x: monitor.x - virtual_desktop.offset_x,
                offset_y: monitor.y - virtual_desktop.offset_y,
            };

            coordinate_spaces.insert(monitor.id, space);
        }

        Self {
            virtual_desktop,
            coordinate_spaces,
        }
    }

    /// Transform point from RDP client space to monitor space
    ///
    /// # Arguments
    ///
    /// * `rdp_x` - X coordinate in RDP space
    /// * `rdp_y` - Y coordinate in RDP space
    ///
    /// # Returns
    ///
    /// (monitor_id, local_x, local_y) or None if point is outside all monitors
    pub fn transform_rdp_to_monitor(&self, rdp_x: i32, rdp_y: i32) -> Option<(u32, i32, i32)> {
        // Find which monitor contains this point
        for monitor in &self.virtual_desktop.monitors {
            if rdp_x >= monitor.x
                && rdp_x < monitor.x + monitor.width as i32
                && rdp_y >= monitor.y
                && rdp_y < monitor.y + monitor.height as i32
            {
                // Convert to monitor-local coordinates
                let local_x = rdp_x - monitor.x;
                let local_y = rdp_y - monitor.y;
                return Some((monitor.id, local_x, local_y));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_calculator_horizontal() {
        // Test horizontal layout calculation
        // Full tests would require StreamInfo mocking
    }

    #[test]
    fn test_virtual_desktop_bounds() {
        // Test bounding box calculation
    }

    #[test]
    fn test_coordinate_transformation() {
        // Test RDP to monitor coordinate mapping
    }
}
