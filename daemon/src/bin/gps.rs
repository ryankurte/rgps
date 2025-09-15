use clap::Parser;
use futures::StreamExt;
use gpsrs_daemon::gps::Gps;
use tracing::{debug, info, level_filters::LevelFilter, warn};
use tracing_subscriber::{EnvFilter, fmt::Subscriber as FmtSubscriber};

/// GPSD(RS) Control Utility
#[derive(Clone, PartialEq, Debug, Parser)]
struct Args {
    /// Serial port for GPS receiver
    #[clap(short, long, default_value = "/dev/ttyACM0")]
    pub port: String,

    /// Baud rate for GPS receiver
    #[clap(short, long, default_value = "115200")]
    pub baud_rate: u32,

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

    // Connect to GPS
    let mut gps = Gps::connect(&args.port, args.baud_rate).await?;

    // Listen to GPS data
    loop {
        tokio::select! {
            _ = exit_rx.recv() => {
                debug!("Exiting main loop");
                break;
            }
            Some(msg) = gps.next() => {
                debug!("GPS update: {:?}", msg);
                let state = gps.nmea().await;

                match (state.latitude, state.longitude, state.altitude) {
                    (Some(lat), Some(lon), Some(alt)) => {
                        info!("Current fix {:?} (lat: {:.8}, lon: {:.8}, alt: {})", state.fix_type, lat, lon, alt);
                    }
                    _ => {
                        warn!("Current fix {:?}", state.fix_type);
                    }
                }
            }
        }
    }

    Ok(())
}
