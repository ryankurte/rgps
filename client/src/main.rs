use clap::Parser;
use futures::StreamExt;
use rgpsc::{RgpsClient, RgpsClientConfig};
use tokio::select;
use tracing::{debug, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, fmt::Subscriber as FmtSubscriber};

/// WASM Embedded Runtime CLI
#[derive(Clone, PartialEq, Debug, Parser)]
struct Args {
    #[clap(flatten)]
    config: RgpsClientConfig,

    #[clap(subcommand)]
    command: Command,

    #[clap(long, default_value = "debug")]
    /// Set log level
    pub log_level: LevelFilter,
}

#[derive(Clone, PartialEq, Debug, Parser)]
enum Command {
    /// Fetch information about the configured GPS devices
    GetInfo,
    /// Get the current state of connected GPS devices
    GetState,
    /// Fetch the current satellite information for connected GPS devices
    GetSatellites,
    /// Stream updates for specific message types
    Stream { flags: Vec<rgps::SubscriptionFlags> },
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

    debug!("rgpsc args: {args:?}");

    // Connect to the RGPS daemon
    let mut client = RgpsClient::connect(&args.config).await?;

    // Execute the requested command
    match args.command {
        Command::GetInfo => {
            let info = client.get_info().await?;
            println!("GPS Device Info:");
            for (id, info) in info {
                println!("  Device {}: {info:?}", id);
            }
        }
        Command::GetState => {
            let state = client.get_state().await?;
            println!("GPS Device State:");
            for (id, state) in state {
                println!("  Device {}: {state:?}", id);
            }
        }
        Command::GetSatellites => {
            let sats = client.get_satellites().await?;
            println!("GPS Device Satellites:");
            for (id, sats) in sats {
                println!("  Device {}: {} satellites", id, sats.len());
                for sat in sats {
                    println!("    {sat:?}");
                }
            }
        }
        Command::Stream { flags } => {
            client.subscribe(flags).await?;
            println!("Subscribed to updates. Press Ctrl+C to exit.");
            loop {
                select! {
                    update = client.next() => {
                        println!("Received update: {update:?}");
                    }
                    _ = tokio::signal::ctrl_c() => {
                        println!("Received Ctrl+C, exiting...");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
