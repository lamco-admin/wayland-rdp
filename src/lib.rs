//! WRD-Server Library
//!
//! Wayland Remote Desktop Server using RDP protocol.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod clipboard;
pub mod config;
pub mod input;
pub mod multimon;
pub mod pipewire;
pub mod portal;
pub mod protocol;
pub mod rdp;
pub mod security;
pub mod server;
pub mod utils;
pub mod video;
