use clap::Parser;
use futures::{SinkExt, StreamExt};
use geoutils::Location;
use nmea::SentenceType;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt}, net::UnixListener, sync::{broadcast::Sender as BroadcastSender, mpsc::{Sender, UnboundedSender}}, task,
};
use tracing::{debug, error, info, span, trace, warn, Level};

use gpsrs_proto::{Req, Resp};

use crate::ntrip::{NtripClient, NtripConfig, NtripCredentials};

pub mod gps;
pub mod ntrip;
pub mod unix;

/// GPS Daemon Command Line Options
#[derive(Clone, PartialEq, Debug, Parser)]
pub struct Options {
    /// GPS device serial port
    #[clap(short='p', long, default_value = "/dev/ttyACM0", env = "GPSD_GPS_PORT")]
    pub gps_port: String,

    /// GPS device baud rate
    #[clap(short='b', long, default_value = "115200", env = "GPSD_GPS_BAUD")]
    pub gps_baud: u32,

    /// Daemon control socket
    #[clap(long, default_value = ".gpsd.sock", env = "GPSD_CTL_SOCK")]
    pub ctl_sock: String,

    // NTRIP options
    #[clap(long, default_value = "posau", env = "NTRIP_HOST")]
    pub ntrip_host: NtripConfig,

    // NTRIP credentials
    #[clap(flatten)]
    pub ntrip_creds: NtripCredentials,
}

/// GPS Daemon Context
pub struct Gpsd {
    ctl_task_handle: task::JoinHandle<Result<(), anyhow::Error>>,
}

impl Gpsd {
    pub async fn new(opts: Options, exit: BroadcastSender<()>) -> Result<Self, anyhow::Error> {
        // Setup unix listening socket

        // Clear the open file if it exists
        let _ = std::fs::remove_file(&opts.ctl_sock);

        // Bind the unix socket listener
        let ctl_listener = UnixListener::bind(&opts.ctl_sock)?;
        let (rx_sink, mut rx_stream) = tokio::sync::mpsc::unbounded_channel();

        // Connect to GPS device
        info!("Connecting to GPS on port {} at {} baud", opts.gps_port, opts.gps_baud);
        let mut gps = gps::Gps::connect(&opts.gps_port, opts.gps_baud).await?;

        // Connect to NTRIP server
        let mut ntrip_client = NtripClient::new(opts.ntrip_host, opts.ntrip_creds).await?;

        // Find the nearest mount point
        // TODO: dynamically add / remove / update based on GPS location
        let mounts = ntrip_client.list_mounts().await?;
        let nearest = mounts.find_nearest(&Location::new(-36.792229246066135, 174.77309828570796));
        info!("Nearest mount point: {:?}", nearest);

        let mut mount = match nearest {
            Some((m, d)) if d < 50_000.0 => {
                info!("Using mount point {} at distance {:.2} meters", m.name, d);
                
                let h = ntrip_client.mount(&m.name, exit.clone()).await?;

                h
            },
            _ => {   
                error!("No mount points found");
                return Err(anyhow::anyhow!("No mount points found"));
            }
        };

        // Create listening task
        let exit = exit.clone();
        let ctl_task_handle = task::spawn(async move {
            let mut index = 0u32;
            let mut e = exit.subscribe();

            loop {
                tokio::select! {
                    // Handle new unix socket connections
                    c = ctl_listener.accept() => match c {
                        Ok((stream, addr)) => {
                            println!("new client {addr:?}!");

                            Self::handle_new_client(index, stream, exit.clone(), rx_sink.clone());
                            index += 1;
                            
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {e:?}");
                        }
                    },

                    // Handle incoming requests from clients
                    req = rx_stream.recv() => match req {
                        Some((req, resp_tx)) => {
                            println!("Received request: {:?}", req);
                            // Handle the request

                            if let Err(e) = resp_tx.send(Resp::Pong){
                                warn!("Failed to send response: {e}");
                            }
                        },
                        None => {
                            // Channel closed
                            break;
                        }
                    },

                    // Handle GPS updates
                    gps_update = gps.next() => match gps_update {
                        Some(msg) => {
                            trace!("GPS update: {:?}", msg);

                            if msg == SentenceType::GLL {
                                let state = gps.nmea().await;

                                debug!("Current state: {:?}, lat: {:?}, lng: {:?}, alt: {:?}, vdop: {:?}, hdop: {:?} pdop: {:?}", state.fix_type, state.latitude, state.longitude, state.altitude, state.vdop, state.hdop, state.pdop);
                            }
                        },
                        None => {
                            // GPS stream closed
                            break;
                        }
                    },

                    // Poll for NTRIP messages
                    ntrip_msg = mount.next() => match ntrip_msg {
                        Some((msg, raw)) => {
                            trace!("NTRIP message {:?}", msg.number());

                            // Forward to GPS
                            gps.send(raw).await?;
                        },
                        None => {
                            warn!("NTRIP stream closed");
                            break;
                        }
                    },

                    // Handle exit signal
                    _ = e.recv() => {
                        println!("Received exit signal");
                        break;
                    },
                }
            }

            Ok(())
        });

        Ok(Self { ctl_task_handle })
    }

    fn handle_new_client(index: u32, mut stream: tokio::net::UnixStream, exit: BroadcastSender<()>, rx_sink: UnboundedSender<(Req, UnboundedSender<Resp>)>) {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Resp>();
        let mut e = exit.subscribe();

        // Spawn a handler task for this unix stream
        task::spawn(async move {
            let (mut unix_rx, mut unix_tx) = stream.split();
            let mut rx_buff = [0u8; 1024];

            loop {
                tokio::select! {
                    // Handle incoming requests from the client
                    req = unix_rx.read(&mut rx_buff) => match req {
                        Ok(n) => {
                            // Process the request
                            let s = std::str::from_utf8(&rx_buff[..n]).unwrap();
                            println!("Received request: {s}");
                            let r = serde_json::from_str::<Req>(s).unwrap();

                            // Forward to request handling
                            rx_sink.send((r, tx.clone())).unwrap();
                        }
                        Err(e) => {
                            // Channel closed
                            trace!("Client {index} disconnected: {e}");
                            break;
                        }
                    },
                    // Handle outgoing responses for this client
                    resp = rx.recv() => match resp {
                        Some(resp) => {
                            // Encode to JSON
                            let s = match serde_json::to_string(&resp) {
                                Ok(s) => s,
                                Err(e) => {
                                    error!("Failed to serialize response: {e}");
                                    continue;
                                }
                            };
                            // Write to the unix socket
                            if let Err(e) = unix_tx.write_all(s.as_bytes()).await {
                                error!("Failed to send response to client {index}: {e}");
                            }
                        }
                        None => {
                            // Channel closed
                            break;
                        }
                    },

                    // Handle exit signal
                    _ = e.recv() => {
                        println!("Received exit signal");
                        break;
                    }
                }
            }
        });

    }
}
