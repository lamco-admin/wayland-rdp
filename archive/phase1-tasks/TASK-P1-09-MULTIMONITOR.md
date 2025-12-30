# TASK P1-09: MULTI-MONITOR SUPPORT - COMPLETE SPECIFICATION
**Task ID:** TASK-P1-09
**Duration:** 7-8 days
**Dependencies:** TASK-P1-05 (PipeWire), P1-06 (Video), P1-08 (IronRDP)
**Status:** NOT_STARTED

## OBJECTIVE
Implement complete multi-monitor support with production-grade layout calculation, coordinate mapping, hotplug handling, and full IronRDP DisplayControl integration for up to 16 displays.

## SUCCESS CRITERIA
- Multiple monitors detected and enumerated via Portal API
- Layout calculation with exact positioning algorithm
- Virtual desktop coordinate space management
- Per-monitor PipeWire stream capture
- Coordinate transformation between spaces
- Monitor hotplug detection and handling
- IronRDP DisplayControl integration
- Resolution change support
- Edge case handling (gaps, overlaps)
- Production-grade error recovery

## ARCHITECTURE

### Core Components
1. **Monitor Manager** - Central coordination and lifecycle
2. **Layout Engine** - Virtual desktop layout calculation
3. **Coordinate Mapper** - Space transformation logic
4. **Stream Manager** - Per-monitor capture streams
5. **Display Controller** - IronRDP integration
6. **Hotplug Handler** - Dynamic monitor changes

### Data Flow
```
Portal Monitor Enumeration → Monitor Detection
            ↓
    Layout Calculation → Virtual Desktop
            ↓
    Stream Creation → Per-Monitor Capture
            ↓
    IronRDP Notification → Client Update
```

## DETAILED IMPLEMENTATION

### 1. Monitor Detection and Metadata

```rust
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use ashpd::desktop::screencast::{SourceType, CursorMode, PersistMode};
use ironrdp::server::display_control::{DisplayControlMonitorLayout, MonitorLayoutEntry};

/// Complete monitor information with all metadata
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    /// Unique monitor identifier (stable across sessions)
    pub id: MonitorId,
    /// Human-readable name (e.g., "DELL P2419H")
    pub name: String,
    /// Connector name (e.g., "DP-1", "HDMI-A-2")
    pub connector: String,
    /// Manufacturer (extracted from EDID)
    pub manufacturer: String,
    /// Model number
    pub model: String,
    /// Serial number (if available)
    pub serial: Option<String>,
    /// Physical dimensions in millimeters
    pub physical_size: PhysicalSize,
    /// Current resolution
    pub resolution: Resolution,
    /// Supported resolutions
    pub supported_resolutions: Vec<Resolution>,
    /// Current refresh rate in Hz
    pub refresh_rate: f64,
    /// Supported refresh rates
    pub supported_refresh_rates: Vec<f64>,
    /// Current rotation in degrees (0, 90, 180, 270)
    pub rotation: Rotation,
    /// Scale factor for HiDPI
    pub scale_factor: f64,
    /// Is this the primary monitor
    pub is_primary: bool,
    /// Monitor capabilities
    pub capabilities: MonitorCapabilities,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MonitorId(pub u32);

#[derive(Debug, Clone, Copy)]
pub struct PhysicalSize {
    pub width_mm: u32,
    pub height_mm: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    Normal = 0,
    Left = 90,
    Inverted = 180,
    Right = 270,
}

#[derive(Debug, Clone)]
pub struct MonitorCapabilities {
    pub hdr_support: bool,
    pub vrr_support: bool,  // Variable Refresh Rate
    pub color_depth: u8,     // Bits per channel
    pub color_gamut: ColorGamut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorGamut {
    SRGB,
    AdobeRGB,
    DCI_P3,
    Rec2020,
}

/// Monitor detection implementation
pub struct MonitorDetector {
    portal: Arc<ScreenCastPortal>,
    cache: RwLock<HashMap<MonitorId, MonitorInfo>>,
}

impl MonitorDetector {
    pub async fn new(portal: Arc<ScreenCastPortal>) -> Result<Self> {
        Ok(Self {
            portal,
            cache: RwLock::new(HashMap::new()),
        })
    }

    /// Enumerate all available monitors
    pub async fn enumerate_monitors(&self) -> Result<Vec<MonitorInfo>> {
        // Request monitor enumeration from Portal
        let sources = self.portal.available_source_types().await?;

        if !sources.contains(SourceType::Monitor) {
            return Err(Error::MonitorNotSupported);
        }

        // Get monitor list from compositor via Portal
        let session = self.portal.create_session().await?;
        let monitors = session.select_sources(
            SourceType::Monitor | SourceType::Window,
            true,  // multiple
            CursorMode::Hidden,
            PersistMode::Application,
        ).await?;

        // Parse monitor information
        let mut monitor_list = Vec::new();
        for (idx, source) in monitors.sources.iter().enumerate() {
            let info = self.parse_monitor_info(source, idx as u32).await?;
            monitor_list.push(info);
        }

        // Update cache
        let mut cache = self.cache.write().await;
        cache.clear();
        for monitor in &monitor_list {
            cache.insert(monitor.id, monitor.clone());
        }

        Ok(monitor_list)
    }

    /// Parse monitor information from Portal source
    async fn parse_monitor_info(&self, source: &PortalSource, id: u32) -> Result<MonitorInfo> {
        // Extract metadata from source properties
        let properties = source.properties();

        Ok(MonitorInfo {
            id: MonitorId(id),
            name: properties.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string(),
            connector: properties.get("connector")
                .and_then(|v| v.as_str())
                .unwrap_or(&format!("DISP-{}", id))
                .to_string(),
            manufacturer: properties.get("manufacturer")
                .and_then(|v| v.as_str())
                .unwrap_or("Generic")
                .to_string(),
            model: properties.get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("Monitor")
                .to_string(),
            serial: properties.get("serial")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            physical_size: PhysicalSize {
                width_mm: properties.get("physical_width")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(510) as u32,
                height_mm: properties.get("physical_height")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(287) as u32,
            },
            resolution: Resolution {
                width: source.position.width,
                height: source.position.height,
            },
            supported_resolutions: self.get_supported_resolutions(&properties),
            refresh_rate: properties.get("refresh_rate")
                .and_then(|v| v.as_f64())
                .unwrap_or(60.0),
            supported_refresh_rates: self.get_supported_refresh_rates(&properties),
            rotation: self.parse_rotation(&properties),
            scale_factor: properties.get("scale")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0),
            is_primary: properties.get("primary")
                .and_then(|v| v.as_bool())
                .unwrap_or(id == 0),
            capabilities: self.parse_capabilities(&properties),
        })
    }

    fn get_supported_resolutions(&self, props: &Properties) -> Vec<Resolution> {
        // Standard resolutions to check
        vec![
            Resolution { width: 3840, height: 2160 },  // 4K
            Resolution { width: 2560, height: 1440 },  // 1440p
            Resolution { width: 1920, height: 1080 },  // 1080p
            Resolution { width: 1680, height: 1050 },
            Resolution { width: 1600, height: 900 },
            Resolution { width: 1366, height: 768 },
            Resolution { width: 1280, height: 1024 },
            Resolution { width: 1280, height: 720 },   // 720p
        ]
    }

    fn get_supported_refresh_rates(&self, props: &Properties) -> Vec<f64> {
        vec![60.0, 75.0, 120.0, 144.0, 165.0, 240.0]
    }

    fn parse_rotation(&self, props: &Properties) -> Rotation {
        props.get("rotation")
            .and_then(|v| v.as_u64())
            .map(|r| match r {
                90 => Rotation::Left,
                180 => Rotation::Inverted,
                270 => Rotation::Right,
                _ => Rotation::Normal,
            })
            .unwrap_or(Rotation::Normal)
    }

    fn parse_capabilities(&self, props: &Properties) -> MonitorCapabilities {
        MonitorCapabilities {
            hdr_support: props.get("hdr")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            vrr_support: props.get("vrr")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            color_depth: props.get("depth")
                .and_then(|v| v.as_u64())
                .unwrap_or(8) as u8,
            color_gamut: ColorGamut::SRGB,
        }
    }
}
```

### 2. Layout Calculation Algorithm - Complete Mathematical Specification

```rust
/// Virtual desktop layout engine with complete positioning algorithm
pub struct LayoutEngine {
    monitors: Vec<MonitorInfo>,
    layout: VirtualDesktopLayout,
    alignment: LayoutAlignment,
    gap: i32,
}

/// Virtual desktop coordinate space
#[derive(Debug, Clone)]
pub struct VirtualDesktopLayout {
    /// Bounding box of all monitors
    pub bounds: Rectangle,
    /// Individual monitor positions
    pub monitor_positions: HashMap<MonitorId, Rectangle>,
    /// Adjacency graph for navigation
    pub adjacency: HashMap<MonitorId, AdjacentMonitors>,
    /// Total virtual desktop area (may include gaps)
    pub total_area: u64,
    /// Actual display area (excludes gaps)
    pub display_area: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct AdjacentMonitors {
    pub left: Option<MonitorId>,
    pub right: Option<MonitorId>,
    pub top: Option<MonitorId>,
    pub bottom: Option<MonitorId>,
}

#[derive(Debug, Clone, Copy)]
pub enum LayoutAlignment {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    Center,
    MiddleRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl LayoutEngine {
    pub fn new(alignment: LayoutAlignment, gap: i32) -> Self {
        Self {
            monitors: Vec::new(),
            layout: VirtualDesktopLayout {
                bounds: Rectangle { x: 0, y: 0, width: 0, height: 0 },
                monitor_positions: HashMap::new(),
                adjacency: HashMap::new(),
                total_area: 0,
                display_area: 0,
            },
            alignment,
            gap,
        }
    }

    /// Calculate complete layout for all monitors
    /// Algorithm: Grid-based optimal packing with alignment constraints
    pub fn calculate_layout(&mut self, monitors: Vec<MonitorInfo>) -> Result<VirtualDesktopLayout> {
        if monitors.is_empty() {
            return Err(Error::NoMonitors);
        }

        self.monitors = monitors;

        // Step 1: Sort monitors by priority (primary first, then by size)
        self.monitors.sort_by(|a, b| {
            if a.is_primary != b.is_primary {
                b.is_primary.cmp(&a.is_primary)
            } else {
                let area_a = a.resolution.width * a.resolution.height;
                let area_b = b.resolution.width * b.resolution.height;
                area_b.cmp(&area_a)
            }
        });

        // Step 2: Calculate optimal grid arrangement
        let grid = self.calculate_optimal_grid();

        // Step 3: Position monitors in grid
        self.position_monitors_in_grid(grid)?;

        // Step 4: Apply rotation transformations
        self.apply_rotations();

        // Step 5: Calculate bounding box
        self.calculate_bounds();

        // Step 6: Apply alignment offset
        self.apply_alignment();

        // Step 7: Build adjacency graph
        self.build_adjacency_graph();

        // Step 8: Calculate areas
        self.calculate_areas();

        Ok(self.layout.clone())
    }

    /// Calculate optimal grid dimensions for monitor count
    fn calculate_optimal_grid(&self) -> (usize, usize) {
        let count = self.monitors.len();
        match count {
            1 => (1, 1),
            2 => (2, 1),  // Side by side
            3 => (3, 1),  // Three in a row
            4 => (2, 2),  // 2x2 grid
            5..=6 => (3, 2),  // 3x2 grid
            7..=9 => (3, 3),  // 3x3 grid
            10..=12 => (4, 3),  // 4x3 grid
            13..=16 => (4, 4),  // 4x4 grid
            _ => {
                // For more than 16, calculate square-ish grid
                let sqrt = (count as f64).sqrt().ceil() as usize;
                (sqrt, (count + sqrt - 1) / sqrt)
            }
        }
    }

    /// Position monitors in calculated grid
    fn position_monitors_in_grid(&mut self, grid: (usize, usize)) -> Result<()> {
        let (cols, rows) = grid;
        let mut positions = HashMap::new();

        // Find primary monitor or use first
        let primary_idx = self.monitors.iter()
            .position(|m| m.is_primary)
            .unwrap_or(0);

        // Place primary monitor at preferred grid position
        let primary_pos = self.get_primary_grid_position(cols, rows);

        // Spiral out from primary position for remaining monitors
        let grid_positions = self.generate_spiral_positions(primary_pos, cols, rows);

        for (idx, monitor) in self.monitors.iter().enumerate() {
            let grid_pos = if idx == primary_idx {
                primary_pos
            } else {
                let adjusted_idx = if idx > primary_idx { idx - 1 } else { idx };
                grid_positions[adjusted_idx]
            };

            // Calculate pixel position from grid position
            let rect = self.calculate_monitor_rectangle(monitor, grid_pos, &positions);
            positions.insert(monitor.id, rect);
        }

        self.layout.monitor_positions = positions;
        Ok(())
    }

    /// Calculate exact pixel position for monitor in grid
    fn calculate_monitor_rectangle(
        &self,
        monitor: &MonitorInfo,
        grid_pos: (usize, usize),
        existing: &HashMap<MonitorId, Rectangle>,
    ) -> Rectangle {
        let (grid_x, grid_y) = grid_pos;

        // Calculate base position
        let mut x = 0i32;
        let mut y = 0i32;

        // Add widths of all monitors to the left
        for pos in existing.values() {
            let pos_grid_x = self.pixel_to_grid_x(pos.x);
            if pos_grid_x < grid_x {
                x = x.max(pos.x + pos.width as i32 + self.gap);
            }
        }

        // Add heights of all monitors above
        for pos in existing.values() {
            let pos_grid_y = self.pixel_to_grid_y(pos.y);
            if pos_grid_y < grid_y {
                y = y.max(pos.y + pos.height as i32 + self.gap);
            }
        }

        // Apply rotation to dimensions
        let (width, height) = match monitor.rotation {
            Rotation::Normal | Rotation::Inverted => {
                (monitor.resolution.width, monitor.resolution.height)
            }
            Rotation::Left | Rotation::Right => {
                (monitor.resolution.height, monitor.resolution.width)
            }
        };

        Rectangle { x, y, width, height }
    }

    /// Generate spiral positions from center
    fn generate_spiral_positions(
        &self,
        center: (usize, usize),
        cols: usize,
        rows: usize,
    ) -> Vec<(usize, usize)> {
        let mut positions = Vec::new();
        let mut visited = vec![vec![false; cols]; rows];

        // Directions: right, down, left, up
        let directions = [(1i32, 0i32), (0, 1), (-1, 0), (0, -1)];
        let mut dir_idx = 0;
        let mut x = center.0 as i32;
        let mut y = center.1 as i32;

        visited[y as usize][x as usize] = true;

        let mut steps = 1;
        let mut step_count = 0;
        let mut turns = 0;

        while positions.len() < cols * rows - 1 {
            for _ in 0..steps {
                let (dx, dy) = directions[dir_idx];
                x += dx;
                y += dy;

                if x >= 0 && x < cols as i32 && y >= 0 && y < rows as i32 {
                    if !visited[y as usize][x as usize] {
                        visited[y as usize][x as usize] = true;
                        positions.push((x as usize, y as usize));
                    }
                }

                step_count += 1;
                if step_count >= steps {
                    break;
                }
            }

            step_count = 0;
            dir_idx = (dir_idx + 1) % 4;
            turns += 1;

            if turns % 2 == 0 {
                steps += 1;
            }
        }

        positions
    }

    /// Get preferred grid position for primary monitor based on alignment
    fn get_primary_grid_position(&self, cols: usize, rows: usize) -> (usize, usize) {
        match self.alignment {
            LayoutAlignment::TopLeft => (0, 0),
            LayoutAlignment::TopCenter => (cols / 2, 0),
            LayoutAlignment::TopRight => (cols - 1, 0),
            LayoutAlignment::MiddleLeft => (0, rows / 2),
            LayoutAlignment::Center => (cols / 2, rows / 2),
            LayoutAlignment::MiddleRight => (cols - 1, rows / 2),
            LayoutAlignment::BottomLeft => (0, rows - 1),
            LayoutAlignment::BottomCenter => (cols / 2, rows - 1),
            LayoutAlignment::BottomRight => (cols - 1, rows - 1),
        }
    }

    /// Apply rotation transformations to positions
    fn apply_rotations(&mut self) {
        for monitor in &self.monitors {
            if let Some(rect) = self.layout.monitor_positions.get_mut(&monitor.id) {
                match monitor.rotation {
                    Rotation::Inverted => {
                        // 180° rotation: flip both axes
                        // No position change needed, just dimension swap handled earlier
                    }
                    Rotation::Left | Rotation::Right => {
                        // 90° or 270° rotation: dimensions already swapped
                        // Adjust position if needed for alignment
                    }
                    Rotation::Normal => {}
                }
            }
        }
    }

    /// Calculate virtual desktop bounding box
    fn calculate_bounds(&mut self) {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for rect in self.layout.monitor_positions.values() {
            min_x = min_x.min(rect.x);
            min_y = min_y.min(rect.y);
            max_x = max_x.max(rect.x + rect.width as i32);
            max_y = max_y.max(rect.y + rect.height as i32);
        }

        self.layout.bounds = Rectangle {
            x: min_x,
            y: min_y,
            width: (max_x - min_x) as u32,
            height: (max_y - min_y) as u32,
        };
    }

    /// Apply alignment offset to normalize coordinates
    fn apply_alignment(&mut self) {
        // Normalize to (0, 0) origin
        let offset_x = -self.layout.bounds.x;
        let offset_y = -self.layout.bounds.y;

        for rect in self.layout.monitor_positions.values_mut() {
            rect.x += offset_x;
            rect.y += offset_y;
        }

        self.layout.bounds.x = 0;
        self.layout.bounds.y = 0;
    }

    /// Build adjacency graph for monitor navigation
    fn build_adjacency_graph(&mut self) {
        const ADJACENCY_THRESHOLD: i32 = 50; // pixels

        for monitor in &self.monitors {
            let rect = self.layout.monitor_positions[&monitor.id];
            let mut adjacent = AdjacentMonitors {
                left: None,
                right: None,
                top: None,
                bottom: None,
            };

            for other in &self.monitors {
                if monitor.id == other.id {
                    continue;
                }

                let other_rect = self.layout.monitor_positions[&other.id];

                // Check left adjacency
                if (other_rect.x + other_rect.width as i32 - rect.x).abs() <= ADJACENCY_THRESHOLD {
                    if Self::rects_overlap_vertically(&rect, &other_rect) {
                        adjacent.left = Some(other.id);
                    }
                }

                // Check right adjacency
                if (rect.x + rect.width as i32 - other_rect.x).abs() <= ADJACENCY_THRESHOLD {
                    if Self::rects_overlap_vertically(&rect, &other_rect) {
                        adjacent.right = Some(other.id);
                    }
                }

                // Check top adjacency
                if (other_rect.y + other_rect.height as i32 - rect.y).abs() <= ADJACENCY_THRESHOLD {
                    if Self::rects_overlap_horizontally(&rect, &other_rect) {
                        adjacent.top = Some(other.id);
                    }
                }

                // Check bottom adjacency
                if (rect.y + rect.height as i32 - other_rect.y).abs() <= ADJACENCY_THRESHOLD {
                    if Self::rects_overlap_horizontally(&rect, &other_rect) {
                        adjacent.bottom = Some(other.id);
                    }
                }
            }

            self.layout.adjacency.insert(monitor.id, adjacent);
        }
    }

    fn rects_overlap_vertically(r1: &Rectangle, r2: &Rectangle) -> bool {
        let r1_top = r1.y;
        let r1_bottom = r1.y + r1.height as i32;
        let r2_top = r2.y;
        let r2_bottom = r2.y + r2.height as i32;

        r1_top < r2_bottom && r2_top < r1_bottom
    }

    fn rects_overlap_horizontally(r1: &Rectangle, r2: &Rectangle) -> bool {
        let r1_left = r1.x;
        let r1_right = r1.x + r1.width as i32;
        let r2_left = r2.x;
        let r2_right = r2.x + r2.width as i32;

        r1_left < r2_right && r2_left < r1_right
    }

    /// Calculate total and display areas
    fn calculate_areas(&mut self) {
        self.layout.total_area = (self.layout.bounds.width as u64) *
                                 (self.layout.bounds.height as u64);

        self.layout.display_area = self.layout.monitor_positions.values()
            .map(|rect| (rect.width as u64) * (rect.height as u64))
            .sum();
    }

    // Helper methods for grid conversion
    fn pixel_to_grid_x(&self, x: i32) -> usize {
        // Simplified grid mapping
        (x / 1920).max(0) as usize
    }

    fn pixel_to_grid_y(&self, y: i32) -> usize {
        // Simplified grid mapping
        (y / 1080).max(0) as usize
    }
}
```

### 3. Coordinate Mapping System

```rust
/// Coordinate transformation between virtual desktop and monitor spaces
pub struct CoordinateMapper {
    layout: Arc<RwLock<VirtualDesktopLayout>>,
    monitor_transforms: HashMap<MonitorId, Transform>,
}

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    /// Monitor-to-virtual offset
    pub offset_x: i32,
    pub offset_y: i32,
    /// Scale factors
    pub scale_x: f64,
    pub scale_y: f64,
    /// Rotation matrix
    pub rotation_matrix: [[f64; 2]; 2],
}

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct MonitorPoint {
    pub monitor_id: MonitorId,
    pub local_x: f64,
    pub local_y: f64,
}

impl CoordinateMapper {
    pub fn new(layout: Arc<RwLock<VirtualDesktopLayout>>) -> Self {
        Self {
            layout,
            monitor_transforms: HashMap::new(),
        }
    }

    /// Initialize transforms for all monitors
    pub async fn update_transforms(&mut self, monitors: &[MonitorInfo]) -> Result<()> {
        let layout = self.layout.read().await;

        for monitor in monitors {
            let rect = layout.monitor_positions.get(&monitor.id)
                .ok_or(Error::MonitorNotFound)?;

            let transform = self.calculate_transform(monitor, rect);
            self.monitor_transforms.insert(monitor.id, transform);
        }

        Ok(())
    }

    /// Calculate transformation matrix for monitor
    fn calculate_transform(&self, monitor: &MonitorInfo, rect: &Rectangle) -> Transform {
        let rotation_angle = match monitor.rotation {
            Rotation::Normal => 0.0,
            Rotation::Left => std::f64::consts::PI / 2.0,
            Rotation::Inverted => std::f64::consts::PI,
            Rotation::Right => 3.0 * std::f64::consts::PI / 2.0,
        };

        let cos_r = rotation_angle.cos();
        let sin_r = rotation_angle.sin();

        Transform {
            offset_x: rect.x,
            offset_y: rect.y,
            scale_x: monitor.scale_factor,
            scale_y: monitor.scale_factor,
            rotation_matrix: [
                [cos_r, -sin_r],
                [sin_r, cos_r],
            ],
        }
    }

    /// Transform virtual desktop coordinates to monitor-local coordinates
    pub async fn virtual_to_monitor(&self, point: Point) -> Result<MonitorPoint> {
        let layout = self.layout.read().await;

        // Find which monitor contains this point
        for (monitor_id, rect) in &layout.monitor_positions {
            if self.point_in_rect(point, rect) {
                let transform = self.monitor_transforms.get(monitor_id)
                    .ok_or(Error::TransformNotFound)?;

                // Apply inverse transformation
                let translated = Point {
                    x: point.x - transform.offset_x as f64,
                    y: point.y - transform.offset_y as f64,
                };

                // Apply inverse rotation
                let inv_matrix = self.invert_rotation_matrix(transform.rotation_matrix);
                let rotated = self.apply_matrix(translated, inv_matrix);

                // Apply inverse scale
                let local = Point {
                    x: rotated.x / transform.scale_x,
                    y: rotated.y / transform.scale_y,
                };

                return Ok(MonitorPoint {
                    monitor_id: *monitor_id,
                    local_x: local.x,
                    local_y: local.y,
                });
            }
        }

        // Point is outside all monitors - find nearest
        self.find_nearest_monitor(point).await
    }

    /// Transform monitor-local coordinates to virtual desktop coordinates
    pub fn monitor_to_virtual(&self, point: MonitorPoint) -> Result<Point> {
        let transform = self.monitor_transforms.get(&point.monitor_id)
            .ok_or(Error::TransformNotFound)?;

        // Apply scale
        let scaled = Point {
            x: point.local_x * transform.scale_x,
            y: point.local_y * transform.scale_y,
        };

        // Apply rotation
        let rotated = self.apply_matrix(scaled, transform.rotation_matrix);

        // Apply translation
        Ok(Point {
            x: rotated.x + transform.offset_x as f64,
            y: rotated.y + transform.offset_y as f64,
        })
    }

    fn point_in_rect(&self, point: Point, rect: &Rectangle) -> bool {
        point.x >= rect.x as f64 &&
        point.x < (rect.x + rect.width as i32) as f64 &&
        point.y >= rect.y as f64 &&
        point.y < (rect.y + rect.height as i32) as f64
    }

    fn apply_matrix(&self, point: Point, matrix: [[f64; 2]; 2]) -> Point {
        Point {
            x: matrix[0][0] * point.x + matrix[0][1] * point.y,
            y: matrix[1][0] * point.x + matrix[1][1] * point.y,
        }
    }

    fn invert_rotation_matrix(&self, matrix: [[f64; 2]; 2]) -> [[f64; 2]; 2] {
        // For rotation matrices, inverse = transpose
        [
            [matrix[0][0], matrix[1][0]],
            [matrix[0][1], matrix[1][1]],
        ]
    }

    /// Find nearest monitor for out-of-bounds point
    async fn find_nearest_monitor(&self, point: Point) -> Result<MonitorPoint> {
        let layout = self.layout.read().await;
        let mut min_distance = f64::MAX;
        let mut nearest_monitor = None;
        let mut nearest_point = Point { x: 0.0, y: 0.0 };

        for (monitor_id, rect) in &layout.monitor_positions {
            // Clamp point to rectangle bounds
            let clamped = Point {
                x: point.x.max(rect.x as f64)
                    .min((rect.x + rect.width as i32 - 1) as f64),
                y: point.y.max(rect.y as f64)
                    .min((rect.y + rect.height as i32 - 1) as f64),
            };

            let distance = ((point.x - clamped.x).powi(2) +
                           (point.y - clamped.y).powi(2)).sqrt();

            if distance < min_distance {
                min_distance = distance;
                nearest_monitor = Some(*monitor_id);
                nearest_point = clamped;
            }
        }

        let monitor_id = nearest_monitor.ok_or(Error::NoMonitorFound)?;
        let transform = self.monitor_transforms.get(&monitor_id)
            .ok_or(Error::TransformNotFound)?;

        // Convert to monitor-local coordinates
        let translated = Point {
            x: nearest_point.x - transform.offset_x as f64,
            y: nearest_point.y - transform.offset_y as f64,
        };

        Ok(MonitorPoint {
            monitor_id,
            local_x: translated.x / transform.scale_x,
            local_y: translated.y / transform.scale_y,
        })
    }

    /// Handle edge transitions between monitors
    pub async fn handle_edge_transition(
        &self,
        from: MonitorPoint,
        direction: Direction,
    ) -> Result<MonitorPoint> {
        let layout = self.layout.read().await;
        let adjacency = layout.adjacency.get(&from.monitor_id)
            .ok_or(Error::AdjacencyNotFound)?;

        let target_monitor = match direction {
            Direction::Left => adjacency.left,
            Direction::Right => adjacency.right,
            Direction::Up => adjacency.top,
            Direction::Down => adjacency.bottom,
        };

        if let Some(target_id) = target_monitor {
            // Calculate entry point on target monitor
            let from_rect = layout.monitor_positions.get(&from.monitor_id)
                .ok_or(Error::MonitorNotFound)?;
            let to_rect = layout.monitor_positions.get(&target_id)
                .ok_or(Error::MonitorNotFound)?;

            // Map relative position
            let (new_x, new_y) = match direction {
                Direction::Left => (to_rect.width as f64 - 1.0, from.local_y),
                Direction::Right => (0.0, from.local_y),
                Direction::Up => (from.local_x, to_rect.height as f64 - 1.0),
                Direction::Down => (from.local_x, 0.0),
            };

            Ok(MonitorPoint {
                monitor_id: target_id,
                local_x: new_x,
                local_y: new_y,
            })
        } else {
            // No adjacent monitor - clamp to edge
            Ok(from)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}
```

### 4. Monitor Hotplug Handler

```rust
/// Dynamic monitor hotplug detection and handling
pub struct HotplugHandler {
    detector: Arc<MonitorDetector>,
    layout_engine: Arc<RwLock<LayoutEngine>>,
    event_tx: mpsc::Sender<MonitorEvent>,
    monitoring: AtomicBool,
}

#[derive(Debug, Clone)]
pub enum MonitorEvent {
    Connected(MonitorInfo),
    Disconnected(MonitorId),
    ResolutionChanged(MonitorId, Resolution),
    Moved(MonitorId, Rectangle),
    Primary(MonitorId),
}

impl HotplugHandler {
    pub fn new(
        detector: Arc<MonitorDetector>,
        layout_engine: Arc<RwLock<LayoutEngine>>,
    ) -> (Self, mpsc::Receiver<MonitorEvent>) {
        let (event_tx, event_rx) = mpsc::channel(32);

        (Self {
            detector,
            layout_engine,
            event_tx,
            monitoring: AtomicBool::new(false),
        }, event_rx)
    }

    /// Start monitoring for hotplug events
    pub async fn start_monitoring(&self) -> Result<()> {
        if self.monitoring.swap(true, Ordering::SeqCst) {
            return Ok(()); // Already monitoring
        }

        let detector = Arc::clone(&self.detector);
        let layout_engine = Arc::clone(&self.layout_engine);
        let event_tx = self.event_tx.clone();
        let monitoring = Arc::new(self.monitoring.clone());

        tokio::spawn(async move {
            let mut last_monitors = HashMap::new();
            let mut interval = tokio::time::interval(Duration::from_secs(2));

            while monitoring.load(Ordering::SeqCst) {
                interval.tick().await;

                match detector.enumerate_monitors().await {
                    Ok(current_monitors) => {
                        let current_map: HashMap<_, _> = current_monitors
                            .iter()
                            .map(|m| (m.id, m.clone()))
                            .collect();

                        // Check for new monitors
                        for monitor in &current_monitors {
                            if !last_monitors.contains_key(&monitor.id) {
                                let _ = event_tx.send(MonitorEvent::Connected(monitor.clone())).await;
                            }
                        }

                        // Check for removed monitors
                        for (id, _) in &last_monitors {
                            if !current_map.contains_key(id) {
                                let _ = event_tx.send(MonitorEvent::Disconnected(*id)).await;
                            }
                        }

                        // Check for changes in existing monitors
                        for monitor in &current_monitors {
                            if let Some(last) = last_monitors.get(&monitor.id) {
                                if last.resolution != monitor.resolution {
                                    let _ = event_tx.send(MonitorEvent::ResolutionChanged(
                                        monitor.id,
                                        monitor.resolution,
                                    )).await;
                                }

                                if last.is_primary != monitor.is_primary && monitor.is_primary {
                                    let _ = event_tx.send(MonitorEvent::Primary(monitor.id)).await;
                                }
                            }
                        }

                        // Recalculate layout if monitors changed
                        if current_map != last_monitors {
                            let mut engine = layout_engine.write().await;
                            if let Ok(layout) = engine.calculate_layout(current_monitors.clone()) {
                                // Layout updated successfully
                                log::info!("Layout recalculated: {} monitors", current_monitors.len());
                            }
                        }

                        last_monitors = current_map;
                    }
                    Err(e) => {
                        log::error!("Failed to enumerate monitors: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop monitoring
    pub fn stop_monitoring(&self) {
        self.monitoring.store(false, Ordering::SeqCst);
    }

    /// Handle graceful degradation when monitors are removed
    pub async fn handle_monitor_loss(&self, lost_id: MonitorId) -> Result<()> {
        let monitors = self.detector.enumerate_monitors().await?;

        if monitors.is_empty() {
            return Err(Error::AllMonitorsLost);
        }

        // If primary was lost, designate new primary
        let needs_primary = !monitors.iter().any(|m| m.is_primary);
        if needs_primary {
            // Choose largest remaining monitor as primary
            let mut monitors = monitors;
            monitors.sort_by_key(|m| m.resolution.width * m.resolution.height);
            if let Some(new_primary) = monitors.last_mut() {
                new_primary.is_primary = true;
                let _ = self.event_tx.send(MonitorEvent::Primary(new_primary.id)).await;
            }
        }

        // Recalculate layout
        let mut engine = self.layout_engine.write().await;
        engine.calculate_layout(monitors)?;

        Ok(())
    }
}
```

### 5. IronRDP DisplayControl Integration

```rust
use ironrdp::server::{
    DisplayControlServer,
    DisplayControlMonitorLayout,
    MonitorLayoutEntry,
    DisplayControlCaps,
};

/// IronRDP display control server implementation
pub struct RdpDisplayController {
    layout: Arc<RwLock<VirtualDesktopLayout>>,
    monitors: Arc<RwLock<Vec<MonitorInfo>>>,
    capabilities: DisplayControlCaps,
}

impl RdpDisplayController {
    pub fn new(
        layout: Arc<RwLock<VirtualDesktopLayout>>,
        monitors: Arc<RwLock<Vec<MonitorInfo>>>,
    ) -> Self {
        Self {
            layout,
            monitors,
            capabilities: DisplayControlCaps {
                max_monitors: 16,
                max_monitor_area_bytes: 8294400, // 4K resolution
            },
        }
    }

    /// Convert our layout to IronRDP format
    pub async fn get_monitor_layout(&self) -> Result<DisplayControlMonitorLayout> {
        let layout = self.layout.read().await;
        let monitors = self.monitors.read().await;

        let mut entries = Vec::new();

        for monitor in monitors.iter() {
            let rect = layout.monitor_positions.get(&monitor.id)
                .ok_or(Error::MonitorNotInLayout)?;

            entries.push(MonitorLayoutEntry {
                flags: if monitor.is_primary { 0x01 } else { 0x00 },
                left: rect.x,
                top: rect.y,
                width: rect.width,
                height: rect.height,
                physical_width: monitor.physical_size.width_mm,
                physical_height: monitor.physical_size.height_mm,
                orientation: monitor.rotation as u32,
                desktop_scale_factor: (monitor.scale_factor * 100.0) as u32,
                device_scale_factor: 100,
            });
        }

        Ok(DisplayControlMonitorLayout { entries })
    }

    /// Handle monitor layout request from RDP client
    pub async fn handle_layout_request(
        &self,
        requested: DisplayControlMonitorLayout,
    ) -> Result<()> {
        // Validate request
        if requested.entries.len() > self.capabilities.max_monitors as usize {
            return Err(Error::TooManyMonitors);
        }

        // Check if we can accommodate the requested layout
        for entry in &requested.entries {
            let area = entry.width * entry.height * 4; // 4 bytes per pixel
            if area > self.capabilities.max_monitor_area_bytes {
                return Err(Error::MonitorAreaTooLarge);
            }
        }

        // Apply the requested layout if possible
        // This would involve reconfiguring the compositor outputs
        log::info!("Received layout request for {} monitors", requested.entries.len());

        // For now, we acknowledge but don't change our layout
        // In a full implementation, this would reconfigure Wayland outputs

        Ok(())
    }
}

#[async_trait]
impl DisplayControlServer for RdpDisplayController {
    async fn get_capabilities(&self) -> DisplayControlCaps {
        self.capabilities.clone()
    }

    async fn get_monitor_layout(&self) -> Result<DisplayControlMonitorLayout> {
        self.get_monitor_layout().await
    }

    async fn set_monitor_layout(
        &mut self,
        layout: DisplayControlMonitorLayout,
    ) -> Result<()> {
        self.handle_layout_request(layout).await
    }
}
```

### 6. Complete Multi-Monitor Manager

```rust
/// Main multi-monitor management coordinator
pub struct MultiMonitorManager {
    detector: Arc<MonitorDetector>,
    layout_engine: Arc<RwLock<LayoutEngine>>,
    coordinate_mapper: Arc<RwLock<CoordinateMapper>>,
    hotplug_handler: Arc<HotplugHandler>,
    display_controller: Arc<RwLock<RdpDisplayController>>,
    streams: Arc<RwLock<HashMap<MonitorId, PipeWireStream>>>,
    event_rx: mpsc::Receiver<MonitorEvent>,
    shutdown: Arc<AtomicBool>,
}

impl MultiMonitorManager {
    pub async fn new(
        portal: Arc<ScreenCastPortal>,
        alignment: LayoutAlignment,
        gap: i32,
    ) -> Result<Self> {
        let detector = Arc::new(MonitorDetector::new(portal.clone()).await?);
        let layout_engine = Arc::new(RwLock::new(LayoutEngine::new(alignment, gap)));

        // Initial enumeration
        let monitors = detector.enumerate_monitors().await?;
        let layout = {
            let mut engine = layout_engine.write().await;
            engine.calculate_layout(monitors.clone())?
        };

        let layout_arc = Arc::new(RwLock::new(layout));
        let monitors_arc = Arc::new(RwLock::new(monitors));

        let coordinate_mapper = Arc::new(RwLock::new(
            CoordinateMapper::new(Arc::clone(&layout_arc))
        ));

        let (hotplug_handler, event_rx) = HotplugHandler::new(
            Arc::clone(&detector),
            Arc::clone(&layout_engine),
        );

        let display_controller = Arc::new(RwLock::new(
            RdpDisplayController::new(Arc::clone(&layout_arc), Arc::clone(&monitors_arc))
        ));

        Ok(Self {
            detector,
            layout_engine,
            coordinate_mapper,
            hotplug_handler: Arc::new(hotplug_handler),
            display_controller,
            streams: Arc::new(RwLock::new(HashMap::new())),
            event_rx,
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Start multi-monitor management
    pub async fn start(&mut self) -> Result<()> {
        // Start hotplug monitoring
        self.hotplug_handler.start_monitoring().await?;

        // Create initial streams
        self.create_monitor_streams().await?;

        // Start event processing
        self.process_events().await;

        Ok(())
    }

    /// Create PipeWire streams for all monitors
    async fn create_monitor_streams(&self) -> Result<()> {
        let monitors = self.detector.enumerate_monitors().await?;
        let mut streams = self.streams.write().await;

        for monitor in monitors {
            if !streams.contains_key(&monitor.id) {
                let stream = self.create_stream_for_monitor(&monitor).await?;
                streams.insert(monitor.id, stream);
            }
        }

        Ok(())
    }

    /// Create PipeWire stream for specific monitor
    async fn create_stream_for_monitor(&self, monitor: &MonitorInfo) -> Result<PipeWireStream> {
        // Implementation would create actual PipeWire stream
        // This is a placeholder
        Ok(PipeWireStream::new(monitor.id))
    }

    /// Process monitor events
    async fn process_events(&mut self) {
        while !self.shutdown.load(Ordering::SeqCst) {
            tokio::select! {
                Some(event) = self.event_rx.recv() => {
                    if let Err(e) = self.handle_event(event).await {
                        log::error!("Failed to handle event: {}", e);
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    // Periodic tasks if needed
                }
            }
        }
    }

    /// Handle monitor event
    async fn handle_event(&self, event: MonitorEvent) -> Result<()> {
        match event {
            MonitorEvent::Connected(monitor) => {
                log::info!("Monitor connected: {} ({})", monitor.name, monitor.id.0);

                // Create stream for new monitor
                let stream = self.create_stream_for_monitor(&monitor).await?;
                self.streams.write().await.insert(monitor.id, stream);

                // Update coordinate mapper
                let monitors = self.detector.enumerate_monitors().await?;
                self.coordinate_mapper.write().await.update_transforms(&monitors).await?;
            }

            MonitorEvent::Disconnected(id) => {
                log::info!("Monitor disconnected: {}", id.0);

                // Remove stream
                self.streams.write().await.remove(&id);

                // Handle graceful degradation
                self.hotplug_handler.handle_monitor_loss(id).await?;
            }

            MonitorEvent::ResolutionChanged(id, resolution) => {
                log::info!("Monitor {} resolution changed to {}x{}",
                    id.0, resolution.width, resolution.height);

                // Recreate stream with new resolution
                if let Some(monitor) = self.detector.enumerate_monitors().await?
                    .into_iter()
                    .find(|m| m.id == id) {
                    let stream = self.create_stream_for_monitor(&monitor).await?;
                    self.streams.write().await.insert(id, stream);
                }
            }

            MonitorEvent::Moved(id, rect) => {
                log::info!("Monitor {} moved to ({}, {})", id.0, rect.x, rect.y);
            }

            MonitorEvent::Primary(id) => {
                log::info!("Monitor {} is now primary", id.0);
            }
        }

        Ok(())
    }

    /// Get current monitor layout for RDP
    pub async fn get_rdp_layout(&self) -> Result<DisplayControlMonitorLayout> {
        self.display_controller.read().await.get_monitor_layout().await
    }

    /// Transform coordinates from virtual to monitor space
    pub async fn map_coordinates(&self, x: f64, y: f64) -> Result<MonitorPoint> {
        self.coordinate_mapper.read().await
            .virtual_to_monitor(Point { x, y }).await
    }

    /// Shutdown manager
    pub async fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.hotplug_handler.stop_monitoring();
    }
}

// Placeholder for actual PipeWire stream
struct PipeWireStream {
    monitor_id: MonitorId,
}

impl PipeWireStream {
    fn new(monitor_id: MonitorId) -> Self {
        Self { monitor_id }
    }
}
```

## TESTING REQUIREMENTS

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_layout_single_monitor() {
        let mut engine = LayoutEngine::new(LayoutAlignment::Center, 0);
        let monitors = vec![
            create_test_monitor(0, 1920, 1080, true),
        ];

        let layout = engine.calculate_layout(monitors).unwrap();
        assert_eq!(layout.monitor_positions.len(), 1);
        assert_eq!(layout.bounds.width, 1920);
        assert_eq!(layout.bounds.height, 1080);
    }

    #[tokio::test]
    async fn test_layout_dual_monitors() {
        let mut engine = LayoutEngine::new(LayoutAlignment::TopLeft, 10);
        let monitors = vec![
            create_test_monitor(0, 1920, 1080, true),
            create_test_monitor(1, 1920, 1080, false),
        ];

        let layout = engine.calculate_layout(monitors).unwrap();
        assert_eq!(layout.monitor_positions.len(), 2);
        assert_eq!(layout.bounds.width, 3850); // 1920 + 10 + 1920
        assert_eq!(layout.bounds.height, 1080);
    }

    #[tokio::test]
    async fn test_layout_quad_monitors() {
        let mut engine = LayoutEngine::new(LayoutAlignment::Center, 0);
        let monitors = vec![
            create_test_monitor(0, 1920, 1080, true),
            create_test_monitor(1, 1920, 1080, false),
            create_test_monitor(2, 1920, 1080, false),
            create_test_monitor(3, 1920, 1080, false),
        ];

        let layout = engine.calculate_layout(monitors).unwrap();
        assert_eq!(layout.monitor_positions.len(), 4);
        // 2x2 grid
        assert_eq!(layout.bounds.width, 3840);
        assert_eq!(layout.bounds.height, 2160);
    }

    #[tokio::test]
    async fn test_coordinate_mapping() {
        let layout = create_test_layout();
        let mapper = CoordinateMapper::new(Arc::new(RwLock::new(layout)));

        // Test point in first monitor
        let point = Point { x: 100.0, y: 100.0 };
        let monitor_point = mapper.virtual_to_monitor(point).await.unwrap();
        assert_eq!(monitor_point.monitor_id, MonitorId(0));
        assert_eq!(monitor_point.local_x, 100.0);
        assert_eq!(monitor_point.local_y, 100.0);

        // Test point in second monitor
        let point = Point { x: 2000.0, y: 100.0 };
        let monitor_point = mapper.virtual_to_monitor(point).await.unwrap();
        assert_eq!(monitor_point.monitor_id, MonitorId(1));
        assert_eq!(monitor_point.local_x, 80.0); // 2000 - 1920
        assert_eq!(monitor_point.local_y, 100.0);
    }

    #[tokio::test]
    async fn test_edge_transition() {
        let layout = create_test_layout();
        let mapper = CoordinateMapper::new(Arc::new(RwLock::new(layout)));

        // Test transition from left monitor to right
        let from = MonitorPoint {
            monitor_id: MonitorId(0),
            local_x: 1919.0,
            local_y: 540.0,
        };

        let to = mapper.handle_edge_transition(from, Direction::Right).await.unwrap();
        assert_eq!(to.monitor_id, MonitorId(1));
        assert_eq!(to.local_x, 0.0);
        assert_eq!(to.local_y, 540.0);
    }

    #[tokio::test]
    async fn test_rotation_transform() {
        let mut engine = LayoutEngine::new(LayoutAlignment::TopLeft, 0);
        let mut monitor = create_test_monitor(0, 1920, 1080, true);
        monitor.rotation = Rotation::Left; // 90 degrees

        let layout = engine.calculate_layout(vec![monitor]).unwrap();
        let rect = layout.monitor_positions.get(&MonitorId(0)).unwrap();

        // Rotated dimensions
        assert_eq!(rect.width, 1080);
        assert_eq!(rect.height, 1920);
    }

    fn create_test_monitor(id: u32, width: u32, height: u32, primary: bool) -> MonitorInfo {
        MonitorInfo {
            id: MonitorId(id),
            name: format!("Monitor {}", id),
            connector: format!("DP-{}", id),
            manufacturer: "TEST".to_string(),
            model: "TestMonitor".to_string(),
            serial: None,
            physical_size: PhysicalSize { width_mm: 510, height_mm: 287 },
            resolution: Resolution { width, height },
            supported_resolutions: vec![Resolution { width, height }],
            refresh_rate: 60.0,
            supported_refresh_rates: vec![60.0],
            rotation: Rotation::Normal,
            scale_factor: 1.0,
            is_primary: primary,
            capabilities: MonitorCapabilities {
                hdr_support: false,
                vrr_support: false,
                color_depth: 8,
                color_gamut: ColorGamut::SRGB,
            },
        }
    }

    fn create_test_layout() -> VirtualDesktopLayout {
        let mut positions = HashMap::new();
        positions.insert(MonitorId(0), Rectangle { x: 0, y: 0, width: 1920, height: 1080 });
        positions.insert(MonitorId(1), Rectangle { x: 1920, y: 0, width: 1920, height: 1080 });

        let mut adjacency = HashMap::new();
        adjacency.insert(MonitorId(0), AdjacentMonitors {
            left: None,
            right: Some(MonitorId(1)),
            top: None,
            bottom: None,
        });
        adjacency.insert(MonitorId(1), AdjacentMonitors {
            left: Some(MonitorId(0)),
            right: None,
            top: None,
            bottom: None,
        });

        VirtualDesktopLayout {
            bounds: Rectangle { x: 0, y: 0, width: 3840, height: 1080 },
            monitor_positions: positions,
            adjacency,
            total_area: 3840 * 1080,
            display_area: 2 * 1920 * 1080,
        }
    }
}
```

## ERROR HANDLING

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("No monitors detected")]
    NoMonitors,

    #[error("Monitor not found: {0:?}")]
    MonitorNotFound(MonitorId),

    #[error("Transform not found for monitor: {0:?}")]
    TransformNotFound(MonitorId),

    #[error("Monitor not in layout: {0:?}")]
    MonitorNotInLayout(MonitorId),

    #[error("No monitor found at coordinates")]
    NoMonitorFound,

    #[error("Adjacency information not found")]
    AdjacencyNotFound,

    #[error("All monitors lost")]
    AllMonitorsLost,

    #[error("Too many monitors requested: {0}")]
    TooManyMonitors(usize),

    #[error("Monitor area too large")]
    MonitorAreaTooLarge,

    #[error("Monitor enumeration not supported")]
    MonitorNotSupported,

    #[error("Portal error: {0}")]
    Portal(#[from] ashpd::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

type Result<T> = std::result::Result<T, Error>;
```

## PERFORMANCE CONSIDERATIONS

1. **Layout Calculation**: O(n²) for n monitors, cached and recalculated only on changes
2. **Coordinate Mapping**: O(n) worst case, O(1) average with spatial indexing
3. **Hotplug Polling**: 2-second interval to balance responsiveness and CPU usage
4. **Memory Usage**: ~10KB per monitor for metadata and transforms
5. **Stream Management**: Lazy creation, immediate cleanup on disconnect

## INTEGRATION POINTS

1. **Portal API**: Monitor enumeration and metadata
2. **PipeWire**: Per-monitor stream creation
3. **IronRDP**: DisplayControl protocol implementation
4. **Input System**: Coordinate transformation for mouse/touch
5. **Video Pipeline**: Per-monitor encoding configuration

## DELIVERABLES

1. Complete monitor detection and enumeration
2. Production-grade layout calculation algorithm
3. Precise coordinate transformation system
4. Robust hotplug handling with graceful degradation
5. Full IronRDP DisplayControl integration
6. Comprehensive test coverage
7. Performance benchmarks
8. Documentation and examples

## ACCEPTANCE CRITERIA

- Supports 1-16 monitors in any configuration
- Layout calculation completes in <100ms for 16 monitors
- Coordinate mapping accuracy within 1 pixel
- Zero-downtime hotplug handling
- Graceful single-monitor fallback
- Memory usage <200KB for 16 monitors
- 100% test coverage for layout algorithm
- No race conditions or deadlocks
- Production-ready error handling

**Time Estimate:** 7-8 days
**Lines of Code:** ~1000
**Complexity:** HIGH
**Priority:** P1