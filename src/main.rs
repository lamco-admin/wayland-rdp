//! WRD-Server - Wayland Remote Desktop Server
//!
//! Entry point for the server binary.

use anyhow::Result;
use clap::Parser;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use lamco_rdp_server::config::Config;
use lamco_rdp_server::server::WrdServer;

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

    /// Write logs to file (in addition to stdout)
    #[arg(long)]
    pub log_file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    init_logging(&args)?;

    info!("════════════════════════════════════════════════════════");
    info!("  lamco-rdp-server v{}", env!("CARGO_PKG_VERSION"));
    info!("  Built: {} {}", env!("BUILD_DATE"), env!("BUILD_TIME"));
    info!("  Commit: {}", env!("GIT_HASH"));
    info!("  Profile: {}", if cfg!(debug_assertions) { "debug" } else { "release" });
    info!("════════════════════════════════════════════════════════");

    // Log startup diagnostics
    lamco_rdp_server::utils::log_startup_diagnostics();

    // Load configuration
    let config = Config::load(&args.config).or_else(|e| {
        tracing::warn!("Failed to load config: {}, using defaults", e);
        Config::default_config()
    })?;

    // Override config with CLI args
    let config = config.with_overrides(args.listen.clone(), args.port);

    info!("Configuration loaded successfully");
    tracing::debug!("Config: {:?}", config);

    // Create and start WRD server
    info!("Initializing WRD Server");
    let server = match WrdServer::new(config).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", lamco_rdp_server::utils::format_user_error(&e));
            return Err(e);
        }
    };

    info!("Starting WRD Server");
    if let Err(e) = server.run().await {
        eprintln!("{}", lamco_rdp_server::utils::format_user_error(&e));
        return Err(e);
    }

    info!("WRD Server shut down");
    Ok(())
}

fn init_logging(args: &Args) -> Result<()> {
    use std::fs::File;

    let log_level = match args.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // Enable lamco crates at requested level, IronRDP protocol at info (debug logs raw packets!)
        // ironrdp_cliprdr/egfx/dvc at debug for channel troubleshooting
        tracing_subscriber::EnvFilter::new(format!(
            "lamco={level},ironrdp_cliprdr={level},ironrdp_egfx={level},ironrdp_dvc={level},ironrdp_server={level},ironrdp=info,ashpd=info,warn",
            level = log_level
        ))
    });

    // If log file is specified, write to both stdout and file
    if let Some(log_file_path) = &args.log_file {
        let file = File::create(log_file_path)?;

        match args.log_format.as_str() {
            "json" => {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(
                        tracing_subscriber::fmt::layer()
                            .json()
                            .with_writer(std::io::stdout),
                    )
                    .with(
                        tracing_subscriber::fmt::layer()
                            .json()
                            .with_writer(file)
                            .with_ansi(false),
                    )
                    .init();
            }
            "compact" => {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(
                        tracing_subscriber::fmt::layer()
                            .compact()
                            .with_writer(std::io::stdout),
                    )
                    .with(
                        tracing_subscriber::fmt::layer()
                            .compact()
                            .with_writer(file)
                            .with_ansi(false),
                    )
                    .init();
            }
            _ => {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(
                        tracing_subscriber::fmt::layer()
                            .pretty()
                            .with_writer(std::io::stdout),
                    )
                    .with(
                        tracing_subscriber::fmt::layer()
                            .with_writer(file)
                            .with_ansi(false),
                    )
                    .init();
            }
        }
        info!("Logging to file: {}", log_file_path);
    } else {
        // Stdout only
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
    }

    Ok(())
}
