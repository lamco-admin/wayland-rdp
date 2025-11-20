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

/// Wayland event dispatcher
pub struct WaylandDispatcher {
    /// Display handle
    display: Display<CompositorState>,

    /// Event loop
    event_loop: EventLoop<'static, CompositorState>,

    /// Loop signal for shutdown
    loop_signal: LoopSignal,

    /// Compositor state
    state: Arc<Mutex<CompositorState>>,
}

impl WaylandDispatcher {
    /// Create new Wayland dispatcher
    pub fn new(
        display: Display<CompositorState>,
        event_loop: EventLoop<'static, CompositorState>,
        state: Arc<Mutex<CompositorState>>,
    ) -> Result<Self> {
        info!("Creating Wayland event dispatcher");

        let loop_signal = event_loop.get_signal();

        Ok(Self {
            display,
            event_loop,
            loop_signal,
            state,
        })
    }

    /// Run the event dispatcher
    pub fn run(mut self) -> Result<()> {
        info!("Starting Wayland event dispatcher");

        // Get display handle
        let dh = self.display.handle();

        // Insert Wayland display into event loop
        let display_source = smithay::wayland::socket::ListeningSocketSource::new_auto()
            .context("Failed to create Wayland socket source")?;

        info!("Wayland socket: {:?}", display_source.socket_name());

        // Insert display source into event loop
        self.event_loop
            .handle()
            .insert_source(display_source, |client_stream, _, state| {
                // Accept new client
                if let Err(e) = state
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

        // Insert display into event loop for event dispatching
        self.event_loop
            .handle()
            .insert_source(
                smithay::wayland::socket::WaylandSource::new(self.display),
                |event, _, state| {
                    // Dispatch Wayland events
                    match event {
                        smithay::wayland::socket::WaylandSourceEvent::New(stream) => {
                            debug!("New Wayland client connection");
                        }
                        smithay::wayland::socket::WaylandSourceEvent::Data => {
                            // Client sent data, will be dispatched by Smithay
                            trace!("Wayland client data");
                        }
                        smithay::wayland::socket::WaylandSourceEvent::Error(e) => {
                            error!("Wayland source error: {}", e);
                        }
                    }
                },
            )
            .context("Failed to insert Wayland source")?;

        // Run event loop
        info!("Entering Wayland event loop");

        let mut state = self.state.lock();

        self.event_loop.run(
            None,
            &mut *state,
            |state| {
                // Event loop callback
                // This is called after each event dispatch cycle
                trace!("Event loop iteration");

                // Dispatch pending Wayland events
                // Smithay handles this internally
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
