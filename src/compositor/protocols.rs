//! Wayland protocol handlers
//!
//! Complete implementation of Wayland protocols using Smithay.

pub mod compositor;
pub mod shm;
pub mod xdg_shell;
pub mod seat;
pub mod output;
pub mod data_device;

// Re-export initialization functions for convenience
pub use compositor::ClientState;
pub use shm::init_shm_global;
pub use seat::init_seat_global;
pub use output::init_output_global;
pub use data_device::init_data_device_global;
