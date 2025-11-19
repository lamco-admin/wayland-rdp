//! wl_output protocol implementation
//!
//! Implements display/monitor information for Wayland clients.

use smithay::output::{Output, PhysicalProperties, Scale, Subpixel, Mode};
use smithay::reexports::wayland_server::DisplayHandle;
use smithay::utils::{Size, Physical, Transform};
use crate::compositor::types::CompositorConfig;
use tracing::{debug, info};

/// Initialize output global
pub fn init_output_global(display: &DisplayHandle, config: &CompositorConfig) -> Output {
    info!(
        "Initializing wl_output global: {}x{} @ {}Hz",
        config.width, config.height, config.refresh_rate
    );

    // Create output with headless backend
    let output = Output::new(
        "WRD-0".to_string(), // Output name
        PhysicalProperties {
            size: Size::from((0, 0)),     // Headless = no physical size
            subpixel: Subpixel::Unknown,   // No subpixel for headless
            make: "Wayland RDP".to_string(),
            model: "Headless Compositor".to_string(),
        },
    );

    // Set current mode
    let mode = Mode {
        size: Size::from((config.width as i32, config.height as i32)),
        refresh: (config.refresh_rate * 1000) as i32, // Convert Hz to mHz
    };

    output.change_current_state(
        Some(mode),
        Some(Transform::Normal),
        Some(Scale::Integer(config.scale as i32)),
        Some((0, 0).into()), // Position (0, 0) for single output
    );

    output.set_preferred(mode);

    debug!(
        "Output configured: {} ({}x{} @ {}Hz, scale: {})",
        output.name(),
        config.width,
        config.height,
        config.refresh_rate,
        config.scale
    );

    // Create global
    output.create_global::<crate::compositor::state::CompositorState>(display);

    info!("wl_output global '{}' created", output.name());

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_creation() {
        let config = CompositorConfig::default();

        // Verify mode calculation
        let mode = Mode {
            size: Size::from((config.width as i32, config.height as i32)),
            refresh: (config.refresh_rate * 1000) as i32,
        };

        assert_eq!(mode.size.w, 1920);
        assert_eq!(mode.size.h, 1080);
        assert_eq!(mode.refresh, 60000); // 60 Hz = 60000 mHz
    }
}
