//! Monitor Manager
//!
//! Manages monitor discovery, tracking, and lifecycle with Portal integration.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::multimon::layout::{Layout, LayoutCalculator, LayoutStrategy};
use crate::multimon::Result;
use crate::portal::session::StreamInfo;

/// Monitor information
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    /// Monitor ID (PipeWire node ID)
    pub id: u32,

    /// Monitor name
    pub name: String,

    /// Position in virtual desktop
    pub position: (i32, i32),

    /// Size in pixels
    pub size: (u32, u32),

    /// Refresh rate in Hz
    pub refresh_rate: u32,

    /// Scale factor for HiDPI
    pub scale_factor: f64,

    /// Is this the primary monitor
    pub is_primary: bool,
}

impl MonitorInfo {
    /// Create from Portal StreamInfo
    ///
    /// # Arguments
    ///
    /// * `stream` - Portal stream information
    /// * `is_primary` - Whether this is the primary monitor
    ///
    /// # Returns
    ///
    /// A new MonitorInfo instance
    pub fn from_stream_info(stream: &StreamInfo, is_primary: bool) -> Self {
        Self {
            id: stream.node_id,
            name: format!("Monitor {}", stream.node_id),
            position: stream.position,
            size: stream.size,
            refresh_rate: 60, // Default, would be extracted from stream metadata
            scale_factor: 1.0,
            is_primary,
        }
    }
}

/// Monitor event types
#[derive(Debug, Clone)]
pub enum MonitorEvent {
    /// Monitor added
    Added(MonitorInfo),

    /// Monitor removed
    Removed(u32),

    /// Monitor configuration changed
    Changed(MonitorInfo),

    /// Layout recalculated
    LayoutChanged(Layout),
}

/// Multi-monitor configuration
#[derive(Debug, Clone)]
pub struct MultiMonitorConfig {
    /// Maximum number of monitors to support
    pub max_monitors: usize,

    /// Layout strategy
    pub layout_strategy: LayoutStrategy,

    /// Enable dynamic reconfiguration
    pub enable_dynamic_reconfiguration: bool,

    /// Enable hotplug detection
    pub enable_hotplug: bool,
}

impl Default for MultiMonitorConfig {
    fn default() -> Self {
        Self {
            max_monitors: 16,
            layout_strategy: LayoutStrategy::PreservePositions,
            enable_dynamic_reconfiguration: true,
            enable_hotplug: true,
        }
    }
}

/// Monitor manager coordinates multi-monitor setup
pub struct MonitorManager {
    /// Configuration
    config: MultiMonitorConfig,

    /// Current monitors
    monitors: Arc<RwLock<HashMap<u32, MonitorInfo>>>,

    /// Layout calculator
    layout_calculator: LayoutCalculator,

    /// Current layout
    current_layout: Arc<RwLock<Option<Layout>>>,
}

impl MonitorManager {
    /// Create a new monitor manager
    ///
    /// # Arguments
    ///
    /// * `config` - Multi-monitor configuration
    ///
    /// # Returns
    ///
    /// A new MonitorManager instance
    pub fn new(config: MultiMonitorConfig) -> Self {
        let layout_calculator = LayoutCalculator::new(config.layout_strategy);

        Self {
            config,
            monitors: Arc::new(RwLock::new(HashMap::new())),
            layout_calculator,
            current_layout: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize from Portal streams
    ///
    /// # Arguments
    ///
    /// * `streams` - Stream information from Portal
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Errors
    ///
    /// Returns error if layout calculation fails
    pub async fn initialize_from_streams(&self, streams: &[StreamInfo]) -> Result<()> {
        info!("Initializing monitors from {} streams", streams.len());

        // Create monitor info for each stream
        let mut monitors_map = HashMap::new();

        for (idx, stream) in streams.iter().enumerate() {
            let monitor = MonitorInfo::from_stream_info(stream, idx == 0);
            debug!(
                "Monitor {}: {}x{} at ({}, {})",
                monitor.id, monitor.size.0, monitor.size.1, monitor.position.0, monitor.position.1
            );
            monitors_map.insert(monitor.id, monitor);
        }

        // Store monitors
        *self.monitors.write().await = monitors_map;

        // Calculate layout
        self.recalculate_layout(streams).await?;

        info!("Multi-monitor initialization complete");
        Ok(())
    }

    /// Recalculate layout
    ///
    /// # Arguments
    ///
    /// * `streams` - Current stream information
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Errors
    ///
    /// Returns error if layout calculation fails
    pub async fn recalculate_layout(&self, streams: &[StreamInfo]) -> Result<()> {
        let virtual_desktop = self.layout_calculator.calculate_layout(streams)?;

        let layout = Layout::from_virtual_desktop(virtual_desktop);

        debug!("Layout recalculated: {}x{}", layout.virtual_desktop.width, layout.virtual_desktop.height);

        *self.current_layout.write().await = Some(layout);

        Ok(())
    }

    /// Get current layout
    ///
    /// # Returns
    ///
    /// The current layout if available
    pub async fn get_layout(&self) -> Option<Layout> {
        self.current_layout.read().await.clone()
    }

    /// Get monitor count
    ///
    /// # Returns
    ///
    /// Number of active monitors
    pub async fn monitor_count(&self) -> usize {
        self.monitors.read().await.len()
    }

    /// Get monitor by ID
    ///
    /// # Arguments
    ///
    /// * `id` - Monitor ID
    ///
    /// # Returns
    ///
    /// MonitorInfo if found
    pub async fn get_monitor(&self, id: u32) -> Option<MonitorInfo> {
        self.monitors.read().await.get(&id).cloned()
    }

    /// Get all monitors
    ///
    /// # Returns
    ///
    /// Vector of all monitorsINFO
    pub async fn get_all_monitors(&self) -> Vec<MonitorInfo> {
        self.monitors.read().await.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitor_manager_creation() {
        let config = MultiMonitorConfig::default();
        let _manager = MonitorManager::new(config);
        // Manager created successfully
    }

    #[tokio::test]
    async fn test_monitor_from_stream_info() {
        let stream = StreamInfo {
            node_id: 42,
            position: (0, 0),
            size: (1920, 1080),
            source_type: crate::portal::session::SourceType::Monitor,
        };

        let monitor = MonitorInfo::from_stream_info(&stream, true);
        assert_eq!(monitor.id, 42);
        assert_eq!(monitor.size, (1920, 1080));
        assert!(monitor.is_primary);
    }
}
