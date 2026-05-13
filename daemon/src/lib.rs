//! A rust-based GPS daemon with dynamic NTRIP client support.

use futures::{SinkExt, StreamExt};
use geoutils::Location;
use nmea::{Satellite};
use rtcm_rs::Message;
use tokio::{
    sync::{broadcast::Sender as BroadcastSender, mpsc::UnboundedReceiver},
    task,
};
use tokio_connectors::{codecs::Json, unix::UnixServer};
use tracing::{debug, error, info, level_filters::LevelFilter, trace};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use rgps::{Req, Resp, State};

pub mod config;
pub mod error;
pub mod gps;
pub mod ntrip;

mod compat;

use crate::{config::GpsdConfig, error::Error, gps::GpsMessage, ntrip::NtripActor};

/// RGPSS actor
pub struct Gpsd {
    _ctl_task_handle: task::JoinHandle<Result<(), Error>>,
}

/// RGPSD daemon context
struct GpsCtx {
    state: State,
    satellites: Vec<Satellite>,

    gps: gps::Gps,

    exit_tx: BroadcastSender<()>,
    unix_server: UnixServer<Json, Resp, Req>,

    ntrip_handle: Option<NtripActor>,
    ntrip_rx: UnboundedReceiver<(Message, Vec<u8>)>,

    last_mount_location: Option<Location>,
}

impl Gpsd {
    /// Spawn the GPSD main task, connecting to the GPS, NTRIP service, and binding sockets
    /// as necessary.
    pub async fn spawn(opts: GpsdConfig, exit: BroadcastSender<()>) -> Result<Self, Error> {
        // Create GPSD context
        let gpsd_ctx = GpsCtx::new(opts, exit.clone()).await?;

        // Spawn a task to run the GPSD main loop
        let ctl_task_handle = task::spawn(async move {
            match gpsd_ctx.run().await {
                Ok(_) => {
                    debug!("GPSD main loop exited successfully");
                    Ok(())
                }
                Err(e) => {
                    error!("GPSD main loop exited with error: {}", e);
                    Err(e)
                }
            }
        });

        Ok(Self {
            _ctl_task_handle: ctl_task_handle,
        })
    }
}

impl GpsCtx {
    /// Create a new GPSD context, connecting to the GPS and binding any necessary sockets
    pub async fn new(opts: GpsdConfig, exit_tx: BroadcastSender<()>) -> Result<Self, Error> {
        // Connect to GPS device
        info!(
            "Connecting to GPS on port {} at {} baud",
            opts.gps.gps_port.as_path().display(),
            opts.gps.gps_baud
        );
        let gps = gps::Gps::connect(&opts.gps.gps_port, opts.gps.gps_baud).await?;

        // Bind the unix socket listener
        let unix_server: UnixServer<Json, Resp, Req> =
            UnixServer::bind(&opts.general.ctl_sock).await?;

        // Connect to NTRIP server
        let (ntrip_tx, ntrip_rx) = tokio::sync::mpsc::unbounded_channel();
        let ntrip_handle = if let Some(ntrip_opts) = opts.ntrip.as_ref() {
            info!("NTRIP configuration found, connecting to NTRIP server...");
            Some(NtripActor::spawn(ntrip_opts.clone(), ntrip_tx).await?)
        } else {
            info!("No NTRIP configuration found, skipping NTRIP connection");
            None
        };

        Ok(Self {
            state: State::default(),
            satellites: vec![],

            gps,

            exit_tx,
            unix_server,

            ntrip_handle,
            ntrip_rx,
            last_mount_location: None,
        })
    }

    pub async fn run(mut self) -> Result<(), Error> {
        let mut e = self.exit_tx.subscribe();

        // Interval timer for periodically logging GPS state
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

        loop {
            tokio::select! {
                // Handle incoming control requests on the unix socket
                req = self.unix_server.next() => match req {
                    Some((req, req_id)) => {
                        debug!("Received request: {:?}", req);

                        // Handle the request
                        let res = self.handle_cmd(req).await;

                        // Forward the response
                        if let Err(e) = self.unix_server.send(res, req_id).await {
                            error!("Failed to forward response: {}", e);
                        }
                    },
                    None => {
                        // Channel closed
                        break;
                    }
                },

                // Handle GPS updates
                gps_update = self.gps.next() => match gps_update {
                    Some(msg) => {
                        trace!("Received GPS message: {:?}", msg);
                        self.handle_gps_update(msg).await;
                    },
                    None => {
                        // GPS stream closed
                        break;
                    }
                },

                // Handle NTRIP messages
                Some((msg, data)) = self.ntrip_rx.recv() => {
                    trace!("Received NTRIP message: {:?}, {} bytes", msg, data.len());
                    // Forward the RTCM data to the GPS device
                    if let Err(e) = self.gps.send(data).await {
                        error!("Failed to write RTCM data to GPS: {}", e);
                    }
                },

                // Perform periodic updates / logging
                _ = interval.tick() => {
                    info!("GPS {}", self.state);

                    // TODO: if we have an NTRIP connection but haven't seen an NTRIP
                    // message in a little bit we should force a re-connection.

                },

                // Handle exit signal
                _ = e.recv() => {
                    debug!("Received exit signal");
                    break;
                },
            }
        }

        Ok(())
    }

    /// Handle an incoming control command and produce a response
    async fn handle_cmd(&mut self, cmd: Req) -> Resp {
        match cmd {
            Req::GetState => Resp::State(self.state.clone()),
            Req::GetSatellites => Resp::Satellites(self.satellites.clone()),
        }
    }

    /// Handle GPS updates
    async fn handle_gps_update(&mut self, msg: GpsMessage) {
        match msg {
            GpsMessage::Nmea(nmea, _sentence_type) => {
                self.handle_nmea(nmea).await;
            },
            GpsMessage::Rtcm3(m, _) => {
                trace!("Received RTCM message: {:?}", m);
                return;
            },
            GpsMessage::Ubx(m) => {
                trace!("Received UBX message: {:?}", m);
                return;
            },
        }
    }

    async fn handle_nmea(&mut self, nmea: nmea::Nmea) {
        trace!(
            "NMEA update: {:?}, lat: {:?}, lng: {:?}, alt: {:?}, vdop: {:?}, hdop: {:?} pdop: {:?}",
            nmea.fix_type,
            nmea.latitude,
            nmea.longitude,
            nmea.altitude,
            nmea.vdop,
            nmea.hdop,
            nmea.pdop
        );

        // Update state
        if let Some(fix) = nmea.fix_type {
            self.state.fix = fix;
        }
        self.state.altitude = nmea.altitude.map(|s| s as f64);
        self.state.speed = nmea.speed_over_ground.map(|s| s as f64);
        let loc = match (nmea.latitude, nmea.longitude) {
            (Some(lat), Some(lon)) => {
                let loc = Location::new(lat, lon);
                self.state.location = Some(loc.clone());
                loc
            }
            _ => {
                trace!("No valid location fix");
                self.state.location = None;
                return;
            }
        };
        self.state.num_satellites = nmea.num_of_fix_satellites.unwrap_or(0) as u32;
        self.state.dop.hdop = nmea.hdop;
        self.state.dop.vdop = nmea.vdop;
        self.state.dop.pdop = nmea.pdop;

        self.satellites = nmea.satellites().to_vec();

        // Update NTRIP actor with new location (if we've moved
        // enough to warrant an update).
        self.maybe_update_ntrip_location(loc).await;
    }

    /// Optionally update the NTRIP actor with a new location.
    ///
    /// Only updates if we have an active NTRIP connection and we've moved some
    /// distance since the last update.
    async fn maybe_update_ntrip_location(&mut self, loc: Location) {
        // Check we have an NTRIP connection before trying to update it
        let ntrip_handle = match self.ntrip_handle.as_ref() {
            Some(handle) => handle,
            None => return, // No NTRIP connection, skip
        };

        // Only update the NTRIP location if we've moved a
        // significant distance since last update.
        match self
            .last_mount_location
            .map(|last_loc| loc.distance_to(&last_loc).ok())
            .flatten()
        {
            Some(d) if d.meters() < 1000.0 => return,
            Some(d) => {
                debug!(
                    "Device moved {} m since last NTRIP update, updating location...",
                    d
                );
            }
            None => {
                debug!("No previous NTRIP location, updating with current location...");
            }
        };

        // Update NTRIP actor with new location
        match ntrip_handle.update_location(loc.clone()).await {
            Ok(Some(m)) => {
                info!("Updated NTRIP mountpoint to {}", m.name);
                self.last_mount_location = Some(loc);
                self.state.mount = Some(m.name);
            }
            Ok(None) => {
                debug!("No closer NTRIP mountpoint found");
            }
            Err(e) => {
                error!("Failed to update NTRIP location: {}", e);
            }
        }
    }
}

/// Helper to set up logging with the provided level filter
pub fn setup_logging(log_level: LevelFilter) {
    let filter = EnvFilter::from_default_env()
        .add_directive("hyper_util=WARN".parse().unwrap())
        .add_directive("reqwest=WARN".parse().unwrap())
        .add_directive("rustls=WARN".parse().unwrap())
        .add_directive(log_level.into());
    let _ = FmtSubscriber::builder()
        .compact()
        .without_time()
        .with_max_level(log_level)
        .with_env_filter(filter)
        .try_init();
}
