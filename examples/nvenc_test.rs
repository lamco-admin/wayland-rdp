//! Minimal NVENC test to diagnose segfault
//!
//! Run with: CUDARC_CUDA_VERSION=12090 cargo run --example nvenc_test --features nvenc

use std::path::PathBuf;

fn main() {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    println!("=== NVENC Diagnostic Test ===\n");

    // Step 1: Check device nodes
    println!("Step 1: Checking NVIDIA device nodes...");
    let devices = ["/dev/nvidia0", "/dev/nvidiactl", "/dev/nvidia-uvm"];
    for device in &devices {
        let exists = std::path::Path::new(device).exists();
        println!("  {}: {}", device, if exists { "✓ exists" } else { "✗ missing" });
    }

    // Step 2: Check driver version
    println!("\nStep 2: Checking driver version...");
    if let Ok(version) = std::fs::read_to_string("/proc/driver/nvidia/version") {
        println!("  {}", version.lines().next().unwrap_or("unknown"));
    } else {
        println!("  ✗ Could not read /proc/driver/nvidia/version");
    }

    // Step 3: Test NVENC encoder (it creates its own CUDA context)
    println!("\nStep 3: Testing NVENC encoder creation and encoding...");

    // Flush stdout before potential crash
    use std::io::Write;
    std::io::stdout().flush().unwrap();

    test_nvenc_encoder();

    println!("\n=== Test Complete ===");
}

fn test_nvenc_encoder() {
    use lamco_rdp_server::config::HardwareEncodingConfig;
    use lamco_rdp_server::egfx::hardware::{
        QualityPreset,
        nvenc::NvencEncoder,
    };

    let config = HardwareEncodingConfig {
        enabled: true,
        vaapi_device: PathBuf::from("/dev/dri/renderD128"),
        enable_dmabuf_zerocopy: false,
        fallback_to_software: true,
        quality_preset: "balanced".to_string(),
        prefer_nvenc: true,
    };

    println!("  About to create NvencEncoder...");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();

    match NvencEncoder::new(&config, 1920, 1080, QualityPreset::Balanced) {
        Ok(mut encoder) => {
            println!("  ✓ NvencEncoder created successfully!");

            // Step 5: Encode a test frame
            println!("\nStep 5: Encoding test frame...");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            // Create a simple test frame (1920x1080 BGRA)
            let frame_size = 1920 * 1080 * 4;
            println!("  Creating {} byte BGRA frame...", frame_size);
            let mut frame = vec![0u8; frame_size];

            // Fill with a gradient pattern
            for y in 0..1080 {
                for x in 0..1920 {
                    let idx = (y * 1920 + x) * 4;
                    frame[idx] = (x % 256) as u8;     // B
                    frame[idx + 1] = (y % 256) as u8; // G
                    frame[idx + 2] = 128;             // R
                    frame[idx + 3] = 255;             // A
                }
            }
            println!("  ✓ Frame created");

            use lamco_rdp_server::egfx::hardware::HardwareEncoder;

            println!("  Calling encode_bgra...");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            match encoder.encode_bgra(&frame, 1920, 1080, 0) {
                Ok(Some(h264_frame)) => {
                    println!("  ✓ Frame encoded successfully!");
                    println!("    Size: {} bytes", h264_frame.size);
                    println!("    Is keyframe: {}", h264_frame.is_keyframe);
                    println!("    First bytes: {:02x} {:02x} {:02x} {:02x}",
                        h264_frame.data.get(0).unwrap_or(&0),
                        h264_frame.data.get(1).unwrap_or(&0),
                        h264_frame.data.get(2).unwrap_or(&0),
                        h264_frame.data.get(3).unwrap_or(&0));
                }
                Ok(None) => {
                    println!("  ✓ Frame submitted (no output yet - buffered)");
                }
                Err(e) => {
                    println!("  ✗ Frame encoding failed: {:?}", e);
                }
            }

            // Let encoder drop cleanly
            println!("\n  Dropping encoder...");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            drop(encoder);
            println!("  ✓ Encoder dropped cleanly!");
        }
        Err(e) => {
            println!("  ✗ NvencEncoder creation failed: {:?}", e);
        }
    }
}
