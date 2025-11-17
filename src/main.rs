//! WRD-Server - Wayland Remote Desktop Server
//!
//! Entry point for the server binary.

use anyhow::Result;
use clap::Parser;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
// mod server;  // Commented out until implemented
// ... other modules commented for now

use config::Config;
// use server::Server;  // Commented until implemented

/// Command-line arguments for wrd-server
#[derive(Parser, Debug)]
#[command(name = "wrd-server")]
#[command(version, about = "Wayland Remote Desktop Server", long_about = None)]
pub struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "/etc/wrd-server/config.toml")]
    pub config: String,

    /// Listen address
    #[arg(short, long, env = "WRD_LISTEN_ADDR")]
    pub listen: Option<String>,

    /// Listen port
    #[arg(short, long, env = "WRD_PORT", default_value = "3389")]
    pub port: u16,

    /// Verbose logging (can be specified multiple times)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Log format (json|pretty|compact)
    #[arg(long, default_value = "pretty")]
    pub log_format: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    init_logging(&args)?;

    info!("Starting WRD-Server v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = Config::load(&args.config).or_else(|e| {
        tracing::warn!("Failed to load config: {}, using defaults", e);
        Config::default_config()
    })?;

    // Override config with CLI args
    let config = config.with_overrides(args.listen.clone(), args.port);

    info!("Configuration loaded successfully");
    tracing::debug!("Config: {:?}", config);

    // TODO: Create and start server (in future tasks)
    // let server = Server::new(config).await?;
    // server.run().await?;

    info!("Server would start here (not yet implemented)");

    // For now, just wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    info!("Shutdown signal received, exiting");

    Ok(())
}

fn init_logging(args: &Args) -> Result<()> {
    let log_level = match args.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new(format!("wrd_server={},warn", log_level))
    });

    match args.log_format.as_str() {
        "json" => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().json())
                .init();
        }
        "compact" => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().compact())
                .init();
        }
        _ => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().pretty())
                .init();
        }
    }

    Ok(())
}
