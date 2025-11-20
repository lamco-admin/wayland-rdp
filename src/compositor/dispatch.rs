//! Wayland event dispatcher
//!
//! Handles dispatching of Wayland protocol events using Smithay.

use super::state::CompositorState;
use super::protocols::ClientState;
use anyhow::{Context, Result};
use smithay::reexports::wayland_server::{
    backend::{ClientData, ClientId, DisconnectReason},
    Display, DisplayHandle,
};
use smithay::reexports::calloop::{EventLoop, LoopHandle, LoopSignal};
use std::sync::Arc;
use parking_lot::Mutex;
use tracing::{debug, error, info, trace, warn};

/// Wayland event dispatcher data
pub struct DispatchData {
    /// Display handle
    pub display: Display<CompositorState>,

    /// Compositor state
    pub state: Arc<Mutex<CompositorState>>,
}

/// Wayland event dispatcher
pub struct WaylandDispatcher {
    /// Event loop
    event_loop: EventLoop<'static, DispatchData>,

    /// Loop signal for shutdown
    loop_signal: LoopSignal,

    /// Dispatch data
    data: DispatchData,
}

impl WaylandDispatcher {
    /// Create new Wayland dispatcher
    pub fn new(
        display: Display<CompositorState>,
        event_loop: EventLoop<'static, DispatchData>,
        state: Arc<Mutex<CompositorState>>,
    ) -> Result<Self> {
        info!("Creating Wayland event dispatcher");

        let loop_signal = event_loop.get_signal();

        let data = DispatchData {
            display,
            state,
        };

        Ok(Self {
            event_loop,
            loop_signal,
            data,
        })
    }

    /// Run the event dispatcher
    pub fn run(mut self) -> Result<()> {
        info!("Starting Wayland event dispatcher");

        // Create Wayland socket
        let listening_socket = smithay::wayland::socket::ListeningSocketSource::new_auto()
            .context("Failed to create listening socket")?;

        info!("Wayland socket: {:?}", listening_socket.socket_name());

        // Insert socket source into event loop
        self.event_loop
            .handle()
            .insert_source(listening_socket, |client_stream, _, data| {
                // Accept new client
                if let Err(e) = data
                    .display
                    .handle()
                    .insert_client(client_stream, Arc::new(ClientState::new(
                        smithay::wayland::compositor::CompositorClientState::default()
                    )))
                {
                    error!("Failed to insert client: {}", e);
                }
            })
            .context("Failed to insert socket source")?;

        // Run event loop
        info!("Entering Wayland event loop");

        self.event_loop.run(
            None,
            &mut self.data,
            |data| {
                // Dispatch Wayland events
                let mut state = data.state.lock();
                if let Err(e) = data.display.dispatch_clients(&mut *state) {
                    error!("Failed to dispatch clients: {}", e);
                }

                trace!("Event loop iteration");
            }
        )
        .context("Event loop error")?;

        info!("Wayland event loop exited");

        Ok(())
    }

    /// Get loop signal for shutdown
    pub fn loop_signal(&self) -> LoopSignal {
        self.loop_signal.clone()
    }
}

/// Client data implementation
impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {
        debug!("Client initialized");
    }

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {
        debug!("Client disconnected");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::types::CompositorConfig;

    #[test]
    fn test_client_state() {
        let client_state = ClientState::new(
            smithay::wayland::compositor::CompositorClientState::default()
        );

        // Test ClientData trait
        client_state.initialized(ClientId::from_raw(1).unwrap());
        client_state.disconnected(
            ClientId::from_raw(1).unwrap(),
            DisconnectReason::ClientDisconnected
        );
    }
}
