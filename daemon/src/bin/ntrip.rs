use clap::Parser;
use futures::StreamExt;
use gpsrs_daemon::ntrip::{NtripConfig, RtcmClient, RtcmProvider};
use tokio::select;
use tracing::{debug, error, info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, fmt::Subscriber as FmtSubscriber};

/// GPSD(RS) Control Utility
#[derive(Clone, PartialEq, Debug, Parser)]
struct Args {
    #[clap(flatten)]
    pub ntrip_cfg: NtripConfig,

    /// NTRIP mount
    #[clap()]
    pub ntrip_mount: String,

    #[clap(long, default_value = "trace")]
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

    debug!("Connecting to NTRIP server");

    // Setup the NTRIP client
    let mut client = RtcmClient::connect(args.ntrip_cfg, args.ntrip_mount, exit_tx.clone())
        .await
        .unwrap();

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

    debug!("Exiting");
}
