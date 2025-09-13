
use clap::Parser;
use tracing::{debug, info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::Subscriber as FmtSubscriber, EnvFilter};

/// GPSD(RS) Control Utility
#[derive(Clone, PartialEq, Debug, Parser)]
struct Args {

    #[clap(long, default_value = "debug")]
    /// Set log level
    pub log_level: LevelFilter,
}


#[tokio::main]
async fn main() {
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

    // TODO: start everything

    // Await exit signal
    let _ = exit_rx.recv().await;


}
