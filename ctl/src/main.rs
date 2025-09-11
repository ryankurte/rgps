use clap::Parser;
use tracing::{debug, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, fmt::Subscriber as FmtSubscriber};

/// WASM Embedded Runtime CLI
#[derive(Clone, PartialEq, Debug, Parser)]
struct Args {
    #[clap(long, default_value = "debug")]
    /// Set log level
    pub log_level: LevelFilter,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
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

    debug!("CTL args: {args:?}");

    Ok(())
}
