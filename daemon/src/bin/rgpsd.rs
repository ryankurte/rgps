use std::path::PathBuf;

use clap::Parser;
use rustls::crypto::CryptoProvider;
use tracing::{debug, info, level_filters::LevelFilter};

use rgpsd::{
    Gpsd,
    config::{GpsdConfig, default_config_path},
    setup_logging,
};

/// RGPSD (GPS + RTK + NTRIP) Daemon
#[derive(Clone, PartialEq, Debug, Parser)]
struct Args {
    #[clap(long, default_value = default_config_path().into_os_string(), env = "RGPSD_CONFIG")]
    /// Path to the RGPSD configuration file
    pub config: PathBuf,

    #[clap(long, default_value = "debug", env = "RGPSD_LOG_LEVEL")]
    /// Set log level
    pub log_level: LevelFilter,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Install the default crypto provider
    CryptoProvider::install_default(rustls::crypto::ring::default_provider()).ok();

    // Setup logging
    setup_logging(args.log_level);

    info!("Start GPSD");

    // Read configuration file
    let config = GpsdConfig::load(&args.config)?;

    debug!("Config {config:?}");

    // Setup interrupt / exit handler
    let (exit_tx, mut exit_rx) = tokio::sync::broadcast::channel(1);
    let e = exit_tx.clone();
    tokio::task::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        debug!("Received Ctrl-C, shutting down...");
        e.send(()).unwrap();
    });

    // Start GPS daemon
    let _gpsd = Gpsd::spawn(config, exit_tx.clone()).await?;

    // Await exit signal
    exit_rx.recv().await?;

    Ok(())
}
