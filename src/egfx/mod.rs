//! EGFX (RDP Graphics Pipeline Extension) Server Implementation
//!
//! This module implements the server-side EGFX channel for H.264 video streaming
//! over RDP. EGFX uses Dynamic Virtual Channels (DVC) and provides hardware-accelerated
//! video encoding via H.264/AVC420.
//!
//! # Architecture
//!
//! The EGFX channel operates as a state machine:
//!
//! 1. **Capability Negotiation**: Client sends `CapabilitiesAdvertise`, server responds
//!    with `CapabilitiesConfirm` selecting AVC420 codec
//! 2. **Surface Setup**: Server creates surface and maps to output
//! 3. **Frame Streaming**: Server sends `StartFrame` + `WireToSurface1` + `EndFrame`
//! 4. **Flow Control**: Client sends `FrameAcknowledge` for received frames
//!
//! # Protocol Reference
//!
//! - [MS-RDPEGFX](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/)

mod encoder;
mod surface;
mod video_handler;

pub use encoder::{
    align_to_16, annex_b_to_avc, create_avc420_bitmap_stream, Avc420Encoder, Avc420Region,
    EncoderConfig, EncoderError, EncoderResult, H264Frame,
};
pub use surface::{EgfxSurface, SurfaceManager};
pub use video_handler::{
    DefaultEgfxVideoHandlerFactory, EgfxVideoConfig, EgfxVideoHandler, EgfxVideoHandlerFactory,
    EncodedFrame, EncodingStats,
};

use ironrdp_core::{decode, impl_as_any, Encode, EncodeResult, WriteCursor};
use ironrdp_dvc::{DvcEncode, DvcMessage, DvcProcessor, DvcServerProcessor};
use ironrdp_pdu::rdp::vc::dvc::gfx::{
    CapabilitiesAdvertisePdu, CapabilitiesConfirmPdu, CapabilitiesV81Flags, CapabilitySet,
    ClientPdu, Codec1Type, CreateSurfacePdu, EndFramePdu, FrameAcknowledgePdu,
    MapSurfaceToOutputPdu, PixelFormat, ServerPdu, StartFramePdu, Timestamp, WireToSurface1Pdu,
};
use ironrdp_pdu::{decode_err, PduResult};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, trace, warn};

/// EGFX channel name as registered in RDP DVC
pub const CHANNEL_NAME: &str = "Microsoft::Windows::RDS::Graphics";

/// Maximum frames in flight before applying backpressure
const MAX_FRAMES_IN_FLIGHT: u32 = 3;

/// Handler trait for EGFX events
///
/// Implement this trait to receive notifications about EGFX channel state changes
/// and to provide frame data for encoding.
pub trait EgfxHandler: Send + Sync {
    /// Called when the client has acknowledged capabilities and the channel is ready
    fn on_ready(&self, surface_id: u16, width: u16, height: u16);

    /// Called when the client acknowledges a frame (for flow control)
    fn on_frame_ack(&self, frame_id: u32);

    /// Called when the channel is closed
    fn on_close(&self);
}

/// Server state for capability negotiation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EgfxState {
    /// Waiting for client CapabilitiesAdvertise
    WaitingCapabilities,
    /// Capabilities confirmed, surface created
    Ready,
    /// Channel closed
    Closed,
}

/// Pending frame waiting for acknowledgment
#[derive(Debug)]
struct PendingFrame {
    frame_id: u32,
    #[allow(dead_code)]
    timestamp_ms: u64,
}

/// EGFX Server implementation
///
/// This implements the `DvcProcessor` trait from IronRDP to handle
/// the EGFX dynamic virtual channel for H.264 video streaming.
pub struct EgfxServer {
    state: EgfxState,
    handler: Arc<dyn EgfxHandler>,

    /// Surface manager for creating and tracking surfaces
    surface_manager: SurfaceManager,

    /// Frame ID counter (monotonically increasing)
    next_frame_id: AtomicU32,

    /// Frames sent but not yet acknowledged (for flow control)
    pending_frames: VecDeque<PendingFrame>,

    /// Last acknowledged frame ID
    last_ack_frame_id: u32,

    /// Selected capability set version
    selected_caps: Option<CapabilitySet>,

    /// Channel ID assigned by DVC infrastructure
    channel_id: Option<u32>,

    /// Output queue for PDUs generated outside of process() calls
    output_queue: Arc<RwLock<VecDeque<ServerPdu>>>,

    /// Desktop dimensions
    width: u16,
    height: u16,
}

impl EgfxServer {
    /// Create a new EGFX server
    ///
    /// # Arguments
    ///
    /// * `handler` - Handler for EGFX events
    /// * `width` - Desktop width in pixels
    /// * `height` - Desktop height in pixels
    pub fn new(handler: Arc<dyn EgfxHandler>, width: u16, height: u16) -> Self {
        Self {
            state: EgfxState::WaitingCapabilities,
            handler,
            surface_manager: SurfaceManager::new(),
            next_frame_id: AtomicU32::new(1),
            pending_frames: VecDeque::new(),
            last_ack_frame_id: 0,
            selected_caps: None,
            channel_id: None,
            output_queue: Arc::new(RwLock::new(VecDeque::new())),
            width,
            height,
        }
    }

    /// Check if H.264 (AVC420) is available
    pub fn is_avc420_available(&self) -> bool {
        self.selected_caps.as_ref().map_or(false, |caps| {
            matches!(caps, CapabilitySet::V8_1 { flags, .. } if flags.contains(CapabilitiesV81Flags::AVC420_ENABLED))
        })
    }

    /// Check if the channel is ready to send frames
    pub fn is_ready(&self) -> bool {
        self.state == EgfxState::Ready
    }

    /// Get number of frames in flight (sent but not acked)
    pub fn frames_in_flight(&self) -> usize {
        self.pending_frames.len()
    }

    /// Check if we should apply backpressure (too many frames in flight)
    pub fn should_backpressure(&self) -> bool {
        self.pending_frames.len() >= MAX_FRAMES_IN_FLIGHT as usize
    }

    /// Queue a frame for sending
    ///
    /// Returns the PDUs to send, or None if backpressure should be applied.
    /// The caller should send these PDUs via the DVC channel.
    pub fn queue_frame(
        &mut self,
        h264_data: Vec<u8>,
        timestamp_ms: u64,
    ) -> Option<Vec<ServerPdu>> {
        if self.state != EgfxState::Ready {
            warn!("Cannot queue frame: EGFX not ready");
            return None;
        }

        if self.should_backpressure() {
            trace!("Backpressure: {} frames in flight", self.frames_in_flight());
            return None;
        }

        let surface = self.surface_manager.primary_surface()?;
        let frame_id = self.next_frame_id.fetch_add(1, Ordering::SeqCst);

        // Track pending frame
        self.pending_frames.push_back(PendingFrame {
            frame_id,
            timestamp_ms,
        });

        // Create frame PDUs
        let pdus = self.create_frame_pdus(surface.id, frame_id, timestamp_ms, h264_data);
        Some(pdus)
    }

    /// Get the output queue for sending PDUs asynchronously
    pub fn output_queue(&self) -> Arc<RwLock<VecDeque<ServerPdu>>> {
        Arc::clone(&self.output_queue)
    }

    // === Private methods ===

    fn handle_capabilities_advertise(
        &mut self,
        caps: CapabilitiesAdvertisePdu,
    ) -> PduResult<Vec<ServerPdu>> {
        // CapabilitiesAdvertisePdu is a tuple struct: CapabilitiesAdvertisePdu(Vec<CapabilitySet>)
        info!("Received EGFX CapabilitiesAdvertise with {} capability sets", caps.0.len());

        // Find the best capability set that supports AVC420
        let selected = self.select_best_capabilities(&caps.0);

        match selected {
            Some(selected_caps) => {
                info!("Selected EGFX capability set: {:?}", selected_caps);
                self.selected_caps = Some(selected_caps.clone());

                // Respond with CapabilitiesConfirm (tuple struct constructor)
                let confirm = CapabilitiesConfirmPdu(selected_caps);
                let mut pdus = vec![ServerPdu::CapabilitiesConfirm(confirm)];

                // Create surface and map to output
                let surface_id = self.surface_manager.create_surface(self.width, self.height);
                pdus.push(ServerPdu::CreateSurface(CreateSurfacePdu {
                    surface_id,
                    width: self.width,
                    height: self.height,
                    pixel_format: PixelFormat::XRgb,
                }));

                pdus.push(ServerPdu::MapSurfaceToOutput(MapSurfaceToOutputPdu {
                    surface_id,
                    output_origin_x: 0,
                    output_origin_y: 0,
                }));

                self.state = EgfxState::Ready;

                // Notify handler
                self.handler.on_ready(surface_id, self.width, self.height);

                Ok(pdus)
            }
            None => {
                warn!("No suitable EGFX capability set found (AVC420 not supported)");
                // Still confirm with first available (fallback)
                if let Some(first) = caps.0.first() {
                    self.selected_caps = Some(first.clone());
                    let confirm = CapabilitiesConfirmPdu(first.clone());
                    Ok(vec![ServerPdu::CapabilitiesConfirm(confirm)])
                } else {
                    Ok(vec![])
                }
            }
        }
    }

    fn handle_frame_ack(&mut self, ack: FrameAcknowledgePdu) -> PduResult<Vec<ServerPdu>> {
        trace!("Received FrameAcknowledge: frame_id={}", ack.frame_id);

        // Remove all frames up to and including this frame_id
        // Use a loop that checks condition then pops to avoid borrow issues
        loop {
            let should_pop = self
                .pending_frames
                .front()
                .is_some_and(|pending| pending.frame_id <= ack.frame_id);

            if should_pop {
                if let Some(pending) = self.pending_frames.pop_front() {
                    self.last_ack_frame_id = pending.frame_id;
                }
            } else {
                break;
            }
        }

        // Notify handler
        self.handler.on_frame_ack(ack.frame_id);

        Ok(vec![])
    }

    fn select_best_capabilities(&self, sets: &[CapabilitySet]) -> Option<CapabilitySet> {
        // Prefer V8.1 with AVC420 enabled
        for set in sets {
            if let CapabilitySet::V8_1 { flags, .. } = set {
                if flags.contains(CapabilitiesV81Flags::AVC420_ENABLED) {
                    return Some(set.clone());
                }
            }
        }

        // Fallback: any V10+ with AVC support
        for set in sets {
            match set {
                CapabilitySet::V10_4 { .. }
                | CapabilitySet::V10_5 { .. }
                | CapabilitySet::V10_6 { .. }
                | CapabilitySet::V10_7 { .. } => {
                    return Some(set.clone());
                }
                _ => {}
            }
        }

        None
    }

    fn create_frame_pdus(
        &self,
        surface_id: u16,
        frame_id: u32,
        timestamp_ms: u64,
        h264_data: Vec<u8>,
    ) -> Vec<ServerPdu> {
        // Convert timestamp to EGFX format (ms, sec, min, hour breakdown)
        // Note: milliseconds and hours are u16, seconds and minutes are u8
        let timestamp = Timestamp {
            milliseconds: (timestamp_ms % 1000) as u16,
            seconds: ((timestamp_ms / 1000) % 60) as u8,
            minutes: ((timestamp_ms / 60000) % 60) as u8,
            hours: ((timestamp_ms / 3600000) % 24) as u16,
        };

        let destination_rectangle = ironrdp_pdu::geometry::InclusiveRectangle {
            left: 0,
            top: 0,
            right: self.width.saturating_sub(1),
            bottom: self.height.saturating_sub(1),
        };

        vec![
            ServerPdu::StartFrame(StartFramePdu { timestamp, frame_id }),
            ServerPdu::WireToSurface1(WireToSurface1Pdu {
                surface_id,
                codec_id: Codec1Type::Avc420,
                pixel_format: PixelFormat::XRgb,
                destination_rectangle,
                bitmap_data: h264_data,
            }),
            ServerPdu::EndFrame(EndFramePdu { frame_id }),
        ]
    }
}

impl_as_any!(EgfxServer);

impl DvcProcessor for EgfxServer {
    fn channel_name(&self) -> &str {
        CHANNEL_NAME
    }

    fn start(&mut self, channel_id: u32) -> PduResult<Vec<DvcMessage>> {
        info!("EGFX channel opened with channel_id={}", channel_id);
        self.channel_id = Some(channel_id);
        // We wait for client CapabilitiesAdvertise, don't send anything yet
        Ok(vec![])
    }

    fn process(&mut self, _channel_id: u32, payload: &[u8]) -> PduResult<Vec<DvcMessage>> {
        let client_pdu: ClientPdu = decode(payload).map_err(|e| decode_err!(e))?;

        let server_pdus = match client_pdu {
            ClientPdu::CapabilitiesAdvertise(caps) => self.handle_capabilities_advertise(caps)?,
            ClientPdu::FrameAcknowledge(ack) => self.handle_frame_ack(ack)?,
        };

        // Convert ServerPdu to DvcMessage (Box<dyn PduEncode>)
        let messages: Vec<DvcMessage> = server_pdus
            .into_iter()
            .map(|pdu| -> DvcMessage { Box::new(PduWrapper(pdu)) })
            .collect();

        Ok(messages)
    }
}

impl DvcServerProcessor for EgfxServer {}

/// Wrapper to implement DvcEncode for ServerPdu
struct PduWrapper(ServerPdu);

impl Encode for PduWrapper {
    fn encode(&self, dst: &mut WriteCursor<'_>) -> EncodeResult<()> {
        self.0.encode(dst)
    }

    fn name(&self) -> &'static str {
        "GfxServerPdu"
    }

    fn size(&self) -> usize {
        self.0.size()
    }
}

// Required: DvcEncode extends Encode + Send
impl DvcEncode for PduWrapper {}

impl std::fmt::Debug for EgfxServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EgfxServer")
            .field("state", &self.state)
            .field("frames_in_flight", &self.pending_frames.len())
            .field("last_ack_frame_id", &self.last_ack_frame_id)
            .field("channel_id", &self.channel_id)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler;

    impl EgfxHandler for TestHandler {
        fn on_ready(&self, surface_id: u16, width: u16, height: u16) {
            println!("Ready: surface={}, {}x{}", surface_id, width, height);
        }
        fn on_frame_ack(&self, frame_id: u32) {
            println!("Ack: frame_id={}", frame_id);
        }
        fn on_close(&self) {
            println!("Closed");
        }
    }

    #[test]
    fn test_egfx_server_creation() {
        let handler = Arc::new(TestHandler);
        let server = EgfxServer::new(handler, 1920, 1080);
        assert_eq!(server.state, EgfxState::WaitingCapabilities);
        assert!(!server.is_ready());
        assert!(!server.is_avc420_available());
    }
}
