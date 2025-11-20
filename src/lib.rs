//! WRD-Server Library
//!
//! Wayland Remote Desktop Server using RDP protocol.
//!
//! This library provides two primary modes of operation:
//!
//! 1. **Portal Mode** (default): Uses xdg-desktop-portal to connect to an existing compositor
//! 2. **Headless Mode** (with `headless-compositor` feature): Custom Smithay-based compositor
//!
//! # Features
//!
//! - `headless-compositor`: Enable custom Smithay compositor and direct login service
//! - `pam-auth`: Enable PAM authentication (required for login service)

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

// Headless compositor (optional)
#[cfg(feature = "headless-compositor")]
pub mod compositor;

// Direct login service (optional, requires headless-compositor)
#[cfg(all(feature = "headless-compositor", feature = "pam-auth"))]
pub mod login;
