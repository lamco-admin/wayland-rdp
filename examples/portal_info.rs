//! Example program to test portal integration
//!
//! This demonstrates creating a portal session and displays information
//! about available streams.
//!
//! Run with: cargo run --example portal_info

use anyhow::Result;
use std::sync::Arc;
use wrd_server::config::Config;
use wrd_server::portal::PortalManager;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("=================================");
    println!("WRD-Server Portal Information");
    println!("=================================\n");

    println!("Creating portal manager...");
    let config = Arc::new(Config::default_config()?);
    let portal_manager = PortalManager::new(&config).await?;
    println!("✓ Portal manager created\n");

    println!("Creating portal session...");
    println!("⚠️  A permission dialog will appear on your desktop.");
    println!("    Please APPROVE the request to continue.\n");

    let session = portal_manager.create_session().await?;
    println!("✓ Portal session created!\n");

    println!("Session Information:");
    println!("  Session ID: {}", session.session_id());
    println!("  PipeWire FD: {}", session.pipewire_fd());
    println!("  Stream Count: {}\n", session.streams().len());

    println!("Available Streams:");
    for (i, stream) in session.streams().iter().enumerate() {
        println!("\n  Stream {}:", i);
        println!("    Node ID: {}", stream.node_id);
        println!("    Size: {}x{}", stream.size.0, stream.size.1);
        println!(
            "    Position: ({}, {})",
            stream.position.0, stream.position.1
        );
        println!("    Type: {:?}", stream.source_type);
    }

    println!("\n=================================");
    println!("Testing input injection...");
    println!("=================================\n");

    let rd = portal_manager.remote_desktop();

    println!("Moving mouse cursor (you should see it move)...");
    for _ in 0..10 {
        rd.notify_pointer_motion(5.0, 0.0).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    println!("✓ Mouse moved\n");

    println!("Press Ctrl+C to exit and close session...");
    tokio::signal::ctrl_c().await?;

    println!("\nClosing session...");
    session.close().await?;
    println!("✓ Session closed\n");

    println!("=================================");
    println!("Portal test complete!");
    println!("=================================");

    Ok(())
}
