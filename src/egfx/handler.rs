//! WrdGraphicsHandler - GraphicsPipelineHandler implementation
//!
//! This module provides the handler that bridges our OpenH264 encoder
//! with ironrdp-egfx's GraphicsPipelineServer.

use ironrdp_egfx::pdu::{CapabilitiesAdvertisePdu, CapabilitiesV81Flags, CapabilitySet};
use ironrdp_egfx::server::{GraphicsPipelineHandler, QoeMetrics, Surface};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::Arc;
use tracing::{debug, info, trace};

/// Handler for EGFX graphics pipeline events
///
/// This implements `GraphicsPipelineHandler` to receive callbacks from
/// ironrdp-egfx's `GraphicsPipelineServer` and manage our OpenH264 encoder.
pub struct WrdGraphicsHandler {
    /// Surface dimensions
    width: u16,
    height: u16,

    /// Whether AVC420 was negotiated
    avc420_enabled: AtomicBool,

    /// Whether the channel is ready for frames
    ready: AtomicBool,

    /// Current primary surface ID (0 = none)
    primary_surface_id: AtomicU16,

    /// Negotiated capability set (stored for reference)
    negotiated_caps: std::sync::RwLock<Option<CapabilitySet>>,
}

impl WrdGraphicsHandler {
    /// Create a new graphics handler
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            avc420_enabled: AtomicBool::new(false),
            ready: AtomicBool::new(false),
            primary_surface_id: AtomicU16::new(0),
            negotiated_caps: std::sync::RwLock::new(None),
        }
    }

    /// Check if the handler is ready and AVC420 is enabled
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    /// Check if H.264 (AVC420) encoding is available
    pub fn is_avc420_enabled(&self) -> bool {
        self.avc420_enabled.load(Ordering::Acquire)
    }

    /// Get the primary surface ID
    pub fn primary_surface_id(&self) -> u16 {
        self.primary_surface_id.load(Ordering::Acquire)
    }

    /// Update dimensions (e.g., on resize)
    pub fn set_dimensions(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    /// Get current dimensions
    pub fn dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }
}

impl GraphicsPipelineHandler for WrdGraphicsHandler {
    fn capabilities_advertise(&mut self, pdu: &CapabilitiesAdvertisePdu) {
        info!("EGFX: Client advertised {} capability sets", pdu.0.len());
        for cap in &pdu.0 {
            debug!("  EGFX capability: {:?}", cap);
        }
    }

    fn on_ready(&mut self, negotiated: &CapabilitySet) {
        info!("EGFX: Channel ready with {:?}", negotiated);

        // Store negotiated caps
        if let Ok(mut guard) = self.negotiated_caps.write() {
            *guard = Some(negotiated.clone());
        }

        // Check for AVC420 support based on capability version
        let avc420 = match negotiated {
            CapabilitySet::V8_1 { flags, .. } => {
                flags.contains(CapabilitiesV81Flags::AVC420_ENABLED)
            }
            // V10+ always support AVC420
            CapabilitySet::V10 { .. }
            | CapabilitySet::V10_1 { .. }
            | CapabilitySet::V10_2 { .. }
            | CapabilitySet::V10_3 { .. }
            | CapabilitySet::V10_4 { .. }
            | CapabilitySet::V10_5 { .. }
            | CapabilitySet::V10_6 { .. }
            | CapabilitySet::V10_7 { .. } => true,
            // V8 and earlier don't support AVC
            _ => false,
        };

        self.avc420_enabled.store(avc420, Ordering::Release);
        self.ready.store(true, Ordering::Release);

        if avc420 {
            info!("EGFX: AVC420 (H.264) encoding enabled");
        } else {
            info!("EGFX: AVC420 not supported by client, will use RemoteFX fallback");
        }
    }

    fn on_frame_ack(&mut self, frame_id: u32, queue_depth: u32) {
        trace!(
            "EGFX: Frame {} acknowledged, client queue depth: {}",
            frame_id,
            queue_depth
        );
    }

    fn on_qoe_metrics(&mut self, metrics: QoeMetrics) {
        debug!(
            "EGFX: QoE metrics - frame {}, decode+render: {}μs",
            metrics.frame_id, metrics.time_diff_dr
        );
        // Future: Use metrics to adjust encoding quality dynamically
    }

    fn on_surface_created(&mut self, surface: &Surface) {
        info!(
            "EGFX: Surface {} created: {}x{}",
            surface.id, surface.width, surface.height
        );

        // Track first surface as primary
        if self.primary_surface_id.load(Ordering::Acquire) == 0 {
            self.primary_surface_id.store(surface.id, Ordering::Release);
        }
    }

    fn on_surface_deleted(&mut self, surface_id: u16) {
        debug!("EGFX: Surface {} deleted", surface_id);

        // Clear primary if it was deleted
        if self.primary_surface_id.load(Ordering::Acquire) == surface_id {
            self.primary_surface_id.store(0, Ordering::Release);
        }
    }

    fn on_close(&mut self) {
        info!("EGFX: Channel closed");
        self.ready.store(false, Ordering::Release);
        self.avc420_enabled.store(false, Ordering::Release);
    }

    fn max_frames_in_flight(&self) -> u32 {
        // Allow 3 frames in flight for smooth streaming
        3
    }

    fn preferred_capabilities(&self) -> Vec<CapabilitySet> {
        // Prefer V10.7 for best features, fall back to V8.1 for AVC420
        vec![
            CapabilitySet::V10_7 {
                flags: ironrdp_egfx::pdu::CapabilitiesV107Flags::SMALL_CACHE,
            },
            CapabilitySet::V8_1 {
                flags: CapabilitiesV81Flags::AVC420_ENABLED
                    | CapabilitiesV81Flags::SMALL_CACHE,
            },
        ]
    }
}

/// Thread-safe wrapper for WrdGraphicsHandler
///
/// Since GraphicsPipelineHandler requires `Send`, but we also need
/// to query state from other tasks, this wrapper provides Arc-based sharing.
pub struct SharedGraphicsHandler {
    inner: Arc<std::sync::RwLock<WrdGraphicsHandler>>,
}

impl SharedGraphicsHandler {
    /// Create a new shared handler
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            inner: Arc::new(std::sync::RwLock::new(WrdGraphicsHandler::new(width, height))),
        }
    }

    /// Get a clone of the inner Arc for querying state
    pub fn clone_inner(&self) -> Arc<std::sync::RwLock<WrdGraphicsHandler>> {
        Arc::clone(&self.inner)
    }

    /// Check if ready (convenience method)
    pub fn is_ready(&self) -> bool {
        self.inner
            .read()
            .map(|h| h.is_ready())
            .unwrap_or(false)
    }

    /// Check if AVC420 is enabled (convenience method)
    pub fn is_avc420_enabled(&self) -> bool {
        self.inner
            .read()
            .map(|h| h.is_avc420_enabled())
            .unwrap_or(false)
    }
}
