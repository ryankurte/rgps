use clap::Parser;
use futures::StreamExt;
use geoutils::Location;
use gpsrs_daemon::ntrip::{NtripConfig, RtcmClient};
use tokio::select;
use tracing::{debug, error, info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, fmt::Subscriber as FmtSubscriber};

/// GPSD(RS) Control Utility
#[derive(Clone, PartialEq, Debug, Parser)]
struct Args {
    #[clap(flatten)]
    pub ntrip_cfg: NtripConfig,

    #[clap(subcommand)]
    pub command: Commands,

    #[clap(long, default_value = "info")]
    /// Set log level
    pub log_level: LevelFilter,
}

#[derive(Clone, PartialEq, Debug, Parser)]
pub enum Commands {
    List,
    FindNearest {
        #[clap()]
        lat: f64,
        #[clap()]
        lon: f64,
    },
    Subscribe {
        #[clap()]
        mount: String,
    },
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

    info!("Start NTRIP/RTMP tool");

    debug!("Args {args:?}");

    // Setup interrupt / exit handler
    let (exit_tx, mut exit_rx) = tokio::sync::broadcast::channel(1);
    let e = exit_tx.clone();
    tokio::task::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        debug!("Received Ctrl-C, shutting down...");
        e.send(()).unwrap();
    });

    match args.command {
        Commands::List => {
            // List available NTRIP mounts using SNIP
            info!("Listing NTRIP mounts");

            let info = RtcmClient::list_mounts(args.ntrip_cfg).await.unwrap();

            for s in info.services {
                info!(
                    "{} - {} ({:.3}, {:.3})",
                    s.name,
                    s.details,
                    s.location.latitude(),
                    s.location.longitude()
                );
            }
        }
        Commands::FindNearest { lat, lon } => {
            // Find the nearest NTRIP mount to the specified location
            info!("Finding nearest NTRIP mount to ({}, {})", lat, lon);

            let info = RtcmClient::list_mounts(args.ntrip_cfg).await.unwrap();

            let target_location = Location::new(lat, lon);

            match info.find_nearest(&target_location) {
                Some((s, d)) => {
                    info!(
                        "Nearest mount: {} - {} ({:.3}, {:.3}), {:.3} km away",
                        s.name,
                        s.details,
                        s.location.latitude(),
                        s.location.longitude(),
                        d / 1000.0
                    );
                }
                None => {
                    info!("No mounts found");
                }
            }
        }
        Commands::Subscribe { mount } => {
            // Subscribe to the specified NTRIP mount
            debug!("Connecting to NTRIP server");

            // Setup the NTRIP client
            let mut client = RtcmClient::mount(args.ntrip_cfg, mount, exit_tx.clone()).await?;

            // Process incoming RTCM messages
            loop {
                select! {
                    m = client.next() => match m {
                        Some(m) => {
                            info!("Received RTCM message: {:?}", m);
                        },
                        None => {
                            error!("NTRIP client stream ended");
                            break;
                        }
                    },
                    _ = exit_rx.recv() => {
                        info!("Exiting on signal");
                        break;
                    }
                }
            }
        }
    }

    debug!("Exiting");

    Ok(())
}
