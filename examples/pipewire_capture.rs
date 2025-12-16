//! PipeWire Screen Capture Example
//!
//! This example demonstrates how to use the PipeWire integration module
//! to capture screens via the XDG Desktop Portal.
//!
//! Usage:
//!   cargo run --example pipewire_capture
//!
//! Requirements:
//! - Active Wayland session
//! - PipeWire installed and running
//! - XDG Desktop Portal with ScreenCast support

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use lamco_rdp_server::config::Config;
use lamco_rdp_server::pipewire::{
    MonitorInfo, MultiStreamConfig, MultiStreamCoordinator, PipeWireConnection, PixelFormat,
    StreamConfig, VideoFrame,
};
use lamco_rdp_server::portal::PortalManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("wrd_server=debug,pipewire_capture=debug")
        .init();

    tracing::info!("PipeWire Screen Capture Example");
    tracing::info!("================================");

    // Create configuration
    let config = Arc::new(Config::default_config()?);

    // Create portal manager
    tracing::info!("Creating portal manager...");
    let portal = PortalManager::new(&config).await?;

    // Create portal session (this will trigger permission dialog)
    tracing::info!("Creating portal session...");
    tracing::info!("A permission dialog should appear - please grant screen capture permission");
    let session = portal.create_session().await?;

    tracing::info!("Portal session created successfully");
    tracing::info!("  Session ID: {}", session.session_id());
    tracing::info!("  PipeWire FD: {}", session.pipewire_fd());
    tracing::info!("  Available streams: {}", session.streams().len());

    // List available streams
    for (i, stream_info) in session.streams().iter().enumerate() {
        tracing::info!(
            "  Stream {}: node_id={}, size={}x{}, pos=({},{})",
            i,
            stream_info.node_id,
            stream_info.size.0,
            stream_info.size.1,
            stream_info.position.0,
            stream_info.position.1,
        );
    }

    // Create PipeWire connection
    tracing::info!("Connecting to PipeWire...");
    let fd = session.pipewire_fd();
    let mut connection = PipeWireConnection::new(fd)?;
    connection.connect().await?;

    tracing::info!("Connected to PipeWire successfully");

    // Create multi-stream coordinator
    tracing::info!("Creating multi-stream coordinator...");
    let coordinator_config = MultiStreamConfig::default();
    let coordinator = MultiStreamCoordinator::new(coordinator_config).await?;

    // Add streams for all available monitors
    let mut stream_receivers = Vec::new();

    for stream_info in session.streams() {
        let monitor = MonitorInfo {
            id: stream_info.node_id,
            name: format!("Monitor-{}", stream_info.node_id),
            position: stream_info.position,
            size: stream_info.size,
            refresh_rate: 60,
            node_id: stream_info.node_id,
        };

        tracing::info!("Creating stream for monitor: {}", monitor.name);

        match coordinator
            .add_stream(monitor.clone(), &mut connection)
            .await
        {
            Ok(stream_id) => {
                tracing::info!("  Stream {} created successfully", stream_id);

                // Get frame receiver for this monitor
                if let Some(rx) = coordinator.get_frame_receiver(stream_info.node_id).await {
                    stream_receivers.push((monitor.name.clone(), rx));
                }
            }
            Err(e) => {
                tracing::error!("  Failed to create stream: {}", e);
            }
        }
    }

    tracing::info!("All streams created successfully");
    tracing::info!("Active streams: {}", coordinator.active_streams().await);

    // Spawn frame processing tasks
    let mut tasks = Vec::new();

    for (monitor_name, mut rx) in stream_receivers {
        let task = tokio::spawn(async move {
            let mut frame_count = 0u64;
            let mut total_bytes = 0u64;
            let start = std::time::Instant::now();

            tracing::info!("[{}] Starting frame capture...", monitor_name);

            while let Some(frame) = rx.recv().await {
                frame_count += 1;
                total_bytes += frame.data_size() as u64;

                if frame_count % 30 == 0 {
                    let elapsed = start.elapsed().as_secs_f64();
                    let fps = frame_count as f64 / elapsed;
                    let mbps = (total_bytes as f64 / elapsed) / (1024.0 * 1024.0);

                    tracing::info!(
                        "[{}] Captured {} frames: {:.1} FPS, {:.1} MB/s, {}x{} {}",
                        monitor_name,
                        frame_count,
                        fps,
                        mbps,
                        frame.width,
                        frame.height,
                        format_pixel_format(frame.format),
                    );

                    if let Some(damage_count) = non_zero_damage_count(&frame) {
                        tracing::debug!("  Damage regions: {}", damage_count);
                    }

                    if frame.flags.has_dmabuf() {
                        tracing::debug!("  Using DMA-BUF (zero-copy)");
                    }
                }

                // Stop after 300 frames (about 10 seconds at 30fps)
                if frame_count >= 300 {
                    tracing::info!("[{}] Reached frame limit, stopping", monitor_name);
                    break;
                }
            }

            tracing::info!(
                "[{}] Final stats: {} frames, {:.2} MB total",
                monitor_name,
                frame_count,
                total_bytes as f64 / (1024.0 * 1024.0)
            );
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete or timeout
    tracing::info!("Capturing frames for 15 seconds...");

    let capture_duration = Duration::from_secs(15);
    let timeout_result =
        tokio::time::timeout(capture_duration, futures::future::join_all(tasks)).await;

    match timeout_result {
        Ok(_) => tracing::info!("All capture tasks completed"),
        Err(_) => tracing::info!("Capture timeout reached"),
    }

    // Get final statistics
    let stats = coordinator.stats().await;
    tracing::info!("Coordinator statistics:");
    tracing::info!("  Streams created: {}", stats.streams_created);
    tracing::info!("  Streams destroyed: {}", stats.streams_destroyed);

    // Cleanup
    tracing::info!("Cleaning up...");
    connection.disconnect().await?;
    session.close().await?;

    tracing::info!("Example completed successfully");

    Ok(())
}

fn format_pixel_format(format: PixelFormat) -> &'static str {
    match format {
        PixelFormat::BGRA => "BGRA",
        PixelFormat::BGRx => "BGRx",
        PixelFormat::RGBA => "RGBA",
        PixelFormat::RGBx => "RGBx",
        PixelFormat::RGB => "RGB",
        PixelFormat::BGR => "BGR",
        PixelFormat::GRAY8 => "GRAY8",
        PixelFormat::NV12 => "NV12",
        PixelFormat::YUY2 => "YUY2",
        PixelFormat::I420 => "I420",
    }
}

fn non_zero_damage_count(frame: &VideoFrame) -> Option<usize> {
    if frame.damage_regions.is_empty() {
        None
    } else {
        Some(frame.damage_regions.len())
    }
}
