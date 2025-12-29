//! Compositor to Service translation
//!
//! This module contains the logic to translate detected compositor
//! capabilities into advertised services with appropriate levels.

use crate::compositor::{
    BufferType, CompositorCapabilities, CompositorType, CursorMode, Quirk, SourceType,
};

use super::{
    rdp_capabilities::RdpCapability,
    service::{AdvertisedService, PerformanceHints, ServiceId, ServiceLevel},
    wayland_features::{DamageMethod, DrmFormat, WaylandFeature},
};

/// Translate compositor capabilities into a list of advertised services
pub fn translate_capabilities(caps: &CompositorCapabilities) -> Vec<AdvertisedService> {
    let mut services = Vec::new();

    // Damage Tracking
    services.push(translate_damage_tracking(caps));

    // DMA-BUF Zero-Copy
    services.push(translate_dmabuf(caps));

    // Explicit Sync
    services.push(translate_explicit_sync(caps));

    // Fractional Scaling
    services.push(translate_fractional_scaling(caps));

    // Metadata Cursor
    services.push(translate_metadata_cursor(caps));

    // Multi-Monitor
    services.push(translate_multi_monitor(caps));

    // Window Capture
    services.push(translate_window_capture(caps));

    // HDR Color Space (future - currently unavailable)
    services.push(AdvertisedService::unavailable(ServiceId::HdrColorSpace));

    // Clipboard
    services.push(translate_clipboard(caps));

    // Remote Input
    services.push(translate_remote_input(caps));

    // Video Capture (PipeWire)
    services.push(translate_video_capture(caps));

    services
}

fn translate_damage_tracking(caps: &CompositorCapabilities) -> AdvertisedService {
    let profile = &caps.profile;

    let (level, method, hints) = if profile.supports_damage_hints {
        // Compositor provides native damage hints
        let method = if caps.compositor.is_wlroots_based() {
            DamageMethod::NativeScreencopy
        } else {
            DamageMethod::Portal
        };

        (ServiceLevel::Guaranteed, method, true)
    } else {
        // Fall back to frame differencing
        (ServiceLevel::BestEffort, DamageMethod::FrameDiff, false)
    };

    let feature = WaylandFeature::DamageTracking {
        method,
        compositor_hints: hints,
    };

    let mut service = if level == ServiceLevel::Guaranteed {
        AdvertisedService::guaranteed(ServiceId::DamageTracking, feature)
    } else {
        AdvertisedService::best_effort(ServiceId::DamageTracking, feature)
    };

    // Set performance hints based on method
    let mut perf = PerformanceHints::default();
    match method {
        DamageMethod::NativeScreencopy => {
            perf.latency_overhead_ms = Some(1);
            perf.simd_available = true;
        }
        DamageMethod::Portal => {
            perf.latency_overhead_ms = Some(2);
            perf.simd_available = true;
        }
        DamageMethod::FrameDiff | DamageMethod::Hybrid => {
            perf.latency_overhead_ms = Some(5);
            perf.simd_available = true; // Our SIMD tile comparison
        }
    }
    service.performance = perf;

    service
}

fn translate_dmabuf(caps: &CompositorCapabilities) -> AdvertisedService {
    let profile = &caps.profile;

    // Check for DMA-BUF quirk
    if profile.has_quirk(&Quirk::PoorDmaBufSupport) {
        return AdvertisedService::unavailable(ServiceId::DmaBufZeroCopy)
            .with_note("Compositor has unreliable DMA-BUF support");
    }

    match profile.recommended_buffer_type {
        BufferType::DmaBuf => {
            let feature = WaylandFeature::DmaBufZeroCopy {
                formats: vec![DrmFormat::Argb8888, DrmFormat::Xrgb8888],
                supports_modifiers: true,
            };

            AdvertisedService::guaranteed(ServiceId::DmaBufZeroCopy, feature)
                .with_rdp_capability(RdpCapability::egfx_full())
                .with_performance(PerformanceHints::zero_copy())
        }
        BufferType::Any => {
            // DMA-BUF may work but not preferred
            let feature = WaylandFeature::DmaBufZeroCopy {
                formats: vec![DrmFormat::Argb8888],
                supports_modifiers: false,
            };

            AdvertisedService::best_effort(ServiceId::DmaBufZeroCopy, feature)
                .with_rdp_capability(RdpCapability::egfx_avc420())
                .with_performance(PerformanceHints::memcpy())
        }
        BufferType::MemFd => {
            AdvertisedService::unavailable(ServiceId::DmaBufZeroCopy)
                .with_note("Compositor prefers MemFd buffers")
        }
    }
}

fn translate_explicit_sync(caps: &CompositorCapabilities) -> AdvertisedService {
    if caps.profile.supports_explicit_sync {
        let feature = WaylandFeature::ExplicitSync { version: 1 };
        AdvertisedService::guaranteed(ServiceId::ExplicitSync, feature)
    } else {
        AdvertisedService::unavailable(ServiceId::ExplicitSync)
    }
}

fn translate_fractional_scaling(caps: &CompositorCapabilities) -> AdvertisedService {
    if caps.has_fractional_scale() {
        let feature = WaylandFeature::FractionalScaling { max_scale: 3.0 };
        AdvertisedService::guaranteed(ServiceId::FractionalScaling, feature)
            .with_rdp_capability(RdpCapability::DesktopComposition {
                multi_mon: false,
                max_monitors: 1,
                scaling: true,
            })
    } else {
        AdvertisedService::unavailable(ServiceId::FractionalScaling)
    }
}

fn translate_metadata_cursor(caps: &CompositorCapabilities) -> AdvertisedService {
    let portal = &caps.portal;
    let profile = &caps.profile;

    // Check if metadata cursor mode is available
    let has_metadata = portal.available_cursor_modes.contains(&CursorMode::Metadata);

    // Check for cursor quirks
    let needs_composite = profile.has_quirk(&Quirk::NeedsExplicitCursorComposite);

    if has_metadata && !needs_composite {
        let feature = WaylandFeature::MetadataCursor {
            has_hotspot: true,
            has_shape_updates: true,
        };

        AdvertisedService::guaranteed(ServiceId::MetadataCursor, feature)
            .with_rdp_capability(RdpCapability::cursor_metadata())
    } else if has_metadata {
        // Metadata available but needs workaround
        let feature = WaylandFeature::MetadataCursor {
            has_hotspot: true,
            has_shape_updates: false,
        };

        AdvertisedService::degraded(
            ServiceId::MetadataCursor,
            feature,
            "Requires explicit cursor compositing",
        )
        .with_rdp_capability(RdpCapability::cursor_painted())
    } else {
        AdvertisedService::unavailable(ServiceId::MetadataCursor)
            .with_rdp_capability(RdpCapability::cursor_painted())
    }
}

fn translate_multi_monitor(caps: &CompositorCapabilities) -> AdvertisedService {
    let portal = &caps.portal;
    let profile = &caps.profile;

    let has_virtual = portal.available_source_types.contains(&SourceType::Virtual);
    let has_monitor = portal.available_source_types.contains(&SourceType::Monitor);

    if !has_monitor && !has_virtual {
        return AdvertisedService::unavailable(ServiceId::MultiMonitor);
    }

    // Check for multi-monitor quirks
    let has_position_quirk = profile.has_quirk(&Quirk::MultiMonitorPositionQuirk);
    let restart_on_resize = profile.has_quirk(&Quirk::RestartCaptureOnResize);

    let max_monitors = if has_virtual { 16 } else { 4 };

    let feature = WaylandFeature::MultiMonitor {
        max_monitors,
        virtual_source: has_virtual,
    };

    let rdp_cap = RdpCapability::DesktopComposition {
        multi_mon: true,
        max_monitors,
        scaling: caps.has_fractional_scale(),
    };

    if has_position_quirk {
        AdvertisedService::degraded(
            ServiceId::MultiMonitor,
            feature,
            "Monitor positions may be incorrect",
        )
        .with_rdp_capability(rdp_cap)
    } else if restart_on_resize {
        AdvertisedService::best_effort(ServiceId::MultiMonitor, feature)
            .with_rdp_capability(rdp_cap)
            .with_note("Capture restarts on resolution change")
    } else {
        AdvertisedService::guaranteed(ServiceId::MultiMonitor, feature)
            .with_rdp_capability(rdp_cap)
    }
}

fn translate_window_capture(caps: &CompositorCapabilities) -> AdvertisedService {
    let portal = &caps.portal;

    let has_window = portal.available_source_types.contains(&SourceType::Window);

    if has_window {
        let feature = WaylandFeature::WindowCapture {
            has_toplevel_export: caps.compositor.is_wlroots_based(),
        };

        // wlroots has better window capture via toplevel export
        if caps.compositor.is_wlroots_based() {
            AdvertisedService::guaranteed(ServiceId::WindowCapture, feature)
        } else {
            AdvertisedService::best_effort(ServiceId::WindowCapture, feature)
        }
    } else {
        AdvertisedService::unavailable(ServiceId::WindowCapture)
    }
}

fn translate_clipboard(caps: &CompositorCapabilities) -> AdvertisedService {
    let portal = &caps.portal;

    if portal.supports_clipboard {
        let feature = WaylandFeature::Clipboard {
            portal_version: portal.version,
        };

        let has_extra_handshake = caps.profile.has_quirk(&Quirk::ClipboardExtraHandshake);

        if has_extra_handshake {
            AdvertisedService::degraded(
                ServiceId::Clipboard,
                feature,
                "Requires extra handshake for paste",
            )
            .with_rdp_capability(RdpCapability::clipboard_standard(10 * 1024 * 1024))
        } else {
            AdvertisedService::guaranteed(ServiceId::Clipboard, feature)
                .with_rdp_capability(RdpCapability::clipboard_standard(10 * 1024 * 1024))
        }
    } else {
        AdvertisedService::unavailable(ServiceId::Clipboard)
    }
}

fn translate_remote_input(caps: &CompositorCapabilities) -> AdvertisedService {
    if caps.portal.supports_remote_desktop {
        let feature = WaylandFeature::RemoteInput {
            uses_libei: true,
            keyboard: true,
            pointer: true,
            touch: false, // Touch typically requires additional setup
        };

        AdvertisedService::guaranteed(ServiceId::RemoteInput, feature)
            .with_rdp_capability(RdpCapability::input_full())
    } else {
        AdvertisedService::unavailable(ServiceId::RemoteInput)
    }
}

fn translate_video_capture(caps: &CompositorCapabilities) -> AdvertisedService {
    if caps.portal.supports_screencast {
        let buffer_type = match caps.profile.recommended_buffer_type {
            BufferType::DmaBuf => "dmabuf",
            BufferType::MemFd => "memfd",
            BufferType::Any => "any",
        };

        let feature = WaylandFeature::PipeWireStream {
            node_id: None,
            buffer_type: buffer_type.to_string(),
        };

        let mut perf = PerformanceHints::default();
        perf.recommended_fps = Some(caps.profile.recommended_fps_cap);
        perf.zero_copy_available = caps.profile.recommended_buffer_type == BufferType::DmaBuf;

        AdvertisedService::guaranteed(ServiceId::VideoCapture, feature)
            .with_performance(perf)
            .with_rdp_capability(RdpCapability::egfx_avc420())
    } else {
        AdvertisedService::unavailable(ServiceId::VideoCapture)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::PortalCapabilities;

    fn make_gnome_caps() -> CompositorCapabilities {
        let compositor = CompositorType::Gnome {
            version: Some("46.0".to_string()),
        };

        let mut portal = PortalCapabilities::default();
        portal.supports_screencast = true;
        portal.supports_remote_desktop = true;
        portal.supports_clipboard = true;
        portal.version = 5;
        portal.available_cursor_modes = vec![CursorMode::Metadata, CursorMode::Embedded];
        portal.available_source_types = vec![SourceType::Monitor, SourceType::Window];

        CompositorCapabilities::new(compositor, portal, vec![])
    }

    #[test]
    fn test_gnome_translation() {
        let caps = make_gnome_caps();
        let services = translate_capabilities(&caps);

        // Check damage tracking
        let damage = services.iter().find(|s| s.id == ServiceId::DamageTracking);
        assert!(damage.is_some());
        assert!(damage.unwrap().level.is_reliable());

        // Check metadata cursor
        let cursor = services.iter().find(|s| s.id == ServiceId::MetadataCursor);
        assert!(cursor.is_some());
        assert_eq!(cursor.unwrap().level, ServiceLevel::Guaranteed);

        // Check clipboard
        let clipboard = services.iter().find(|s| s.id == ServiceId::Clipboard);
        assert!(clipboard.is_some());
        assert!(clipboard.unwrap().level.is_usable());
    }

    #[test]
    fn test_service_count() {
        let caps = make_gnome_caps();
        let services = translate_capabilities(&caps);

        // Should have all service types
        assert_eq!(services.len(), ServiceId::all().len());
    }
}
