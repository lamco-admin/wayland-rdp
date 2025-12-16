//! RDP integration bridge
//!
//! Bridges the compositor with the RDP server for frame export and input injection.

use super::types::*;
use super::input::{KeyboardEvent, PointerEvent};
use super::CompositorHandle;
use anyhow::Result;
use crossbeam_channel::{Sender, Receiver, unbounded};

/// RDP bridge for compositor integration
pub struct RdpBridge {
    compositor: CompositorHandle,
    frame_tx: Sender<FrameBuffer>,
    input_rx: Receiver<RdpInput>,
}

/// Input from RDP
pub enum RdpInput {
    Keyboard(KeyboardEvent),
    Pointer(PointerEvent),
}

impl RdpBridge {
    pub fn new(compositor: CompositorHandle) -> (Self, RdpBridgeClient) {
        let (frame_tx, frame_rx) = unbounded();
        let (input_tx, input_rx) = unbounded();

        let bridge = Self {
            compositor,
            frame_tx,
            input_rx,
        };

        let client = RdpBridgeClient {
            frame_rx,
            input_tx,
        };

        (bridge, client)
    }

    /// Process input events from RDP
    pub fn process_input(&self) -> Result<()> {
        while let Ok(input) = self.input_rx.try_recv() {
            match input {
                RdpInput::Keyboard(event) => {
                    self.compositor.inject_keyboard(event)?;
                }
                RdpInput::Pointer(event) => {
                    self.compositor.inject_pointer(event)?;
                }
            }
        }
        Ok(())
    }

    /// Send frame to RDP
    pub fn send_frame(&self) -> Result<()> {
        let framebuffer_data = self.compositor.get_framebuffer();
        let damage = self.compositor.get_damage();

        // TODO: Create proper FrameBuffer struct and send
        // For now, this is a placeholder

        Ok(())
    }
}

/// Client side of RDP bridge (for RDP server)
pub struct RdpBridgeClient {
    frame_rx: Receiver<FrameBuffer>,
    input_tx: Sender<RdpInput>,
}

impl RdpBridgeClient {
    /// Get next frame
    pub fn get_frame(&self) -> Option<FrameBuffer> {
        self.frame_rx.try_recv().ok()
    }

    /// Send keyboard input
    pub fn send_keyboard(&self, event: KeyboardEvent) -> Result<()> {
        self.input_tx.send(RdpInput::Keyboard(event))?;
        Ok(())
    }

    /// Send pointer input
    pub fn send_pointer(&self, event: PointerEvent) -> Result<()> {
        self.input_tx.send(RdpInput::Pointer(event))?;
        Ok(())
    }
}
