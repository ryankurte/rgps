use clap::Parser;
use futures::StreamExt;
use tracing::{debug, info, level_filters::LevelFilter, warn};
use tracing_subscriber::{EnvFilter, fmt::Subscriber as FmtSubscriber};
use gpsrs_daemon::{Gpsd, Options as GpsdOptions};

/// GPSD(RS) Control Utility
#[derive(Clone, PartialEq, Debug, Parser)]
struct Args {
    /// Serial port for GPS receiver
    #[clap(flatten)]
    pub gpsd: GpsdOptions,

    #[clap(long, default_value = "debug")]
    /// Set log level
    pub log_level: LevelFilter,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Setup logging
    let filter = EnvFilter::from_default_env().add_directive(args.log_level.into());
    let _ = FmtSubscriber::builder()
        .compact()
        .without_time()
        .with_max_level(args.log_level)
        .with_env_filter(filter)
        .try_init();

    info!("Start GPSD");

    debug!("Args {args:?}");

    // Setup interrupt / exit handler
    // Setup interrupt / exit handler
    let (exit_tx, mut exit_rx) = tokio::sync::broadcast::channel(1);
    let e = exit_tx.clone();
    tokio::task::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        debug!("Received Ctrl-C, shutting down...");
        e.send(()).unwrap();
    });

    // Start GPS daemon
    let _gpsd = Gpsd::new(args.gpsd, exit_tx.clone()).await?;

    // Await exit signal
    exit_rx.recv().await?;


    Ok(())
}
