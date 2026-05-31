//! A rust-based GPS daemon with dynamic NTRIP client support.

use std::collections::HashMap;

use console_subscriber::ConsoleLayer;
use futures::SinkExt;
use geoutils::Location;
use nmea::{Satellite, SentenceType};
use rtcm_rs::Message;
use tokio::{
    sync::{
        broadcast::Sender as BroadcastSender,
        mpsc::{UnboundedReceiver, unbounded_channel},
    },
    task,
};
use tokio_stream::{StreamExt, StreamMap, wrappers::UnboundedReceiverStream};
use tracing::{debug, error, info, level_filters::LevelFilter, trace, warn};
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

use tokio_connectors::codecs::Json;

// For unix-based platforms we use a Unix domain socket for the control interface.
#[cfg(target_family = "unix")]
use tokio_connectors::unix::{UnixServer, UnixSocketId};

// For non-unix (aka Windows) we use a TCP socket for the control interface,
// since windows unix domain socket support has got lost somewhere.
#[cfg(not(target_family = "unix"))]
use std::net::SocketAddr;
#[cfg(not(target_family = "unix"))]
use tokio_connectors::tcp::TcpServer;

use rgps::{GpsState, SubscriptionFlags, req::Req, resp::Resp};

pub mod config;
pub mod error;
pub mod gps;
pub mod ntrip;

mod subscriptions;
use subscriptions::Subscriptions;
mod compat;

use crate::{
    config::GpsdConfig,
    error::Error,
    gps::{Gps, GpsMessage},
    ntrip::NtripActor,
};

/// RGPSS actor
pub struct Rgpsd {
    _ctl_task_handle: task::JoinHandle<Result<(), Error>>,
}

/// RGPSD daemon context
struct RgpsdCtx {
    /// GPS handles
    gps_handles: Vec<GpsHandle>,
    /// GPS receive channels
    gps_streams: StreamMap<usize, UnboundedReceiverStream<GpsMessage>>,

    exit_tx: BroadcastSender<()>,

    /// Server for control protocol (TCP or UNIX depending on platform)
    #[cfg(target_family = "unix")]
    ctl_server: UnixServer<Json, Resp, Req>,
    #[cfg(not(target_family = "unix"))]
    ctl_server: TcpServer<Json, Resp, Req>,

    /// Subscription management
    subscriptions: Subscriptions,

    /// Handle to the NTRIP actor, if configured
    ntrip_handle: Option<NtripActor>,
    /// Receiver for messages from the NTRIP actor (e.g. RTCM3 messages to forward to GPS devices)
    ntrip_rx: UnboundedReceiver<(Message, Vec<u8>)>,

    // TODO: move this out to ntrip.rs?
    /// The current NTRIP mountpoint we're connected to (if any)
    current_mount: Option<String>,
    /// The last location we sent to the NTRIP actor, used to determine whether to change mounts
    last_mount_location: Option<Location>,
}

#[cfg(target_family = "unix")]
type SubscriptionId = UnixSocketId;

#[cfg(not(target_family = "unix"))]
type SubscriptionId = SocketAddr;

struct GpsHandle {
    gps: Gps<()>,
    info: rgps::GpsInfo,
    state: GpsState,
    satellites: Vec<Satellite>,
}

impl Rgpsd {
    /// Spawn the GPSD main task, connecting to the GPS, NTRIP service, and binding sockets
    /// as necessary.
    pub async fn spawn(opts: GpsdConfig, exit: BroadcastSender<()>) -> Result<Self, Error> {
        // Create GPSD context
        let gpsd_ctx = RgpsdCtx::new(opts, exit.clone()).await?;

        // Spawn a task to run the GPSD main loop
        let task_builder = tokio::task::Builder::new().name("rgpsd");
        let ctl_task_handle = task_builder
            .spawn(async move {
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
            })
            .map_err(|e| {
                Error::Runtime(anyhow::anyhow!("Failed to spawn GPSD main task: {}", e))
            })?;

        Ok(Self {
            _ctl_task_handle: ctl_task_handle,
        })
    }
}

impl RgpsdCtx {
    /// Create a new GPSD context, connecting to the GPS and binding any necessary sockets
    pub async fn new(opts: GpsdConfig, exit_tx: BroadcastSender<()>) -> Result<Self, Error> {
        // Connect to GPS device(s)

        let mut gpss = Vec::with_capacity(opts.gps.len());
        let mut gps_streams = StreamMap::new();

        for (index, gps) in opts.gps.iter().enumerate() {
            info!(
                "Connecting to GPS {index} on port {} at {} baud",
                gps.gps_port.as_path().display(),
                gps.gps_baud
            );

            let (gps_tx, gps_rx) = unbounded_channel::<GpsMessage>();
            let gps_handle = Gps::connect_with_channel(&gps.gps_port, gps.gps_baud, gps_tx).await?;

            gpss.push(GpsHandle {
                gps: gps_handle,
                info: rgps::GpsInfo {
                    port: gps.gps_port.to_string_lossy().into_owned(),
                    baud: gps.gps_baud,
                    kind: gps.gps_kind.clone(),
                },
                state: GpsState::default(),
                satellites: Vec::new(),
            });

            gps_streams.insert(index, UnboundedReceiverStream::new(gps_rx));
        }

        // Bind the control server socket (TCP or UNIX depending on platform)
        #[cfg(target_family = "unix")]
        let ctl_server: UnixServer<Json, Resp, Req> =
            UnixServer::bind(&opts.general.ctl_sock).await?;

        #[cfg(not(target_family = "unix"))]
        let ctl_server: TcpServer<Json, Resp, Req> =
            TcpServer::bind(&opts.general.ctl_sock).await?;

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
            gps_handles: gpss,
            gps_streams,

            exit_tx,

            ctl_server,
            subscriptions: Subscriptions::default(),

            ntrip_handle,
            ntrip_rx,
            last_mount_location: None,
            current_mount: None,
        })
    }

    pub async fn run(mut self) -> Result<(), Error> {
        let mut e = self.exit_tx.subscribe();

        // Interval timer for periodically logging GPS state
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

        loop {
            tokio::select! {
                // Handle incoming control requests on the control socket
                Some((req, req_id)) = self.ctl_server.next() => {
                    debug!("Received request: {:?}", req);

                    // Handle the request
                    let res = self.handle_cmd(req, req_id).await;

                    debug!("Response: {:?}", res);

                    // Forward the response
                    if let Err(e) = self.ctl_server.send(res, req_id).await {
                        error!("Failed to forward response: {}", e);
                    }
                },

                // Handle GPS updates
                Some((index, msg)) = self.gps_streams.next() => {
                    trace!("Received GPS {} message: {:?}", index, msg);
                    self.handle_gps_update(index, msg).await;
                },

                // Handle NTRIP messages
                Some((msg, data)) = self.ntrip_rx.recv() => {
                    trace!("Received NTRIP message: {:?}, {} bytes", msg, data.len());
                    // Forward the RTCM data to all GPS devices
                    for h in &mut self.gps_handles {
                        if let Err(e) = h.gps.send(data.clone()).await {
                            error!("Failed to write RTCM data to GPS: {}", e);
                        }
                    }
                },

                // Perform periodic updates / logging
                _ = interval.tick() => {
                    for (i, h) in self.gps_handles.iter().enumerate() {
                        info!("GPS {}: {}", i, h.state);
                    }

                    // TODO: if we have an NTRIP connection but haven't seen an NTRIP
                    // message in a little bit we should force a re-connection.

                },

                // Handle exit signal or streams closed
                _ = e.recv() => {
                    debug!("Received exit signal");
                    break;
                },
                else => {
                    debug!("Stream(s) closed, exiting");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle an incoming control command and produce a response
    async fn handle_cmd(&mut self, cmd: Req, client_id: SubscriptionId) -> Resp {
        match cmd {
            Req::GetInfo => Resp::Info(
                self.gps_handles
                    .iter()
                    .enumerate()
                    .map(|(i, h)| (i as u32, h.info.clone()))
                    .collect(),
            ),
            Req::GetState => Resp::State(
                self.gps_handles
                    .iter()
                    .enumerate()
                    .map(|(i, h)| (i as u32, h.state.clone()))
                    .collect(),
            ),
            Req::GetSatellites => Resp::Satellites(
                self.gps_handles
                    .iter()
                    .enumerate()
                    .map(|(i, h)| (i as u32, h.satellites.clone()))
                    .collect(),
            ),
            Req::Subscribe(flags) => {
                if flags.is_empty() {
                    debug!("Unsubscribing client {}", client_id);
                    self.subscriptions.remove(&client_id);
                } else {
                    debug!("Subscribing client {} to: {:?}", client_id, flags);
                    self.subscriptions.subscribe(client_id, flags);
                }

                Resp::Ok
            }
        }
    }

    /// Handle GPS updates
    async fn handle_gps_update(&mut self, index: usize, msg: GpsMessage) {
        match msg {
            GpsMessage::Nmea(nmea, sentence_type) => {
                // Handle the NMEA sentence
                self.handle_nmea(index, sentence_type, nmea.as_ref()).await;

                // Forward to any NMEA subscribers
                self.forward_to_subscribers(
                    SubscriptionFlags::NmeaSentences,
                    Resp::Nmea(index as u32, nmea.to_string()),
                )
                .await;
            }
            GpsMessage::Rtcm3(m, raw) => {
                trace!("Received RTCM message: {:?}", m);

                // TODO: handle and route these?

                // Forward to any RTCM3 subscribers
                self.forward_to_subscribers(
                    SubscriptionFlags::Rtcm3Sentences,
                    Resp::Rtcm3(index as u32, raw.clone()),
                )
                .await;
            }
            GpsMessage::Ubx(m) => {
                trace!("Received UBX message: {:?}", m);

                // TODO: handle and route these?

                // Forward to any UBX subscribers
                self.forward_to_subscribers(
                    SubscriptionFlags::UbxMessages,
                    Resp::Ubx(index as u32, m.clone()),
                )
                .await;
            }
        }
    }

    /// Forward a response message to all subscribers of a specific subscription flag.
    async fn forward_to_subscribers(&mut self, flag: SubscriptionFlags, resp: Resp) {
        let mut removals = Vec::new();

        for i in self.subscriptions.get_subscribers(flag) {
            debug!("Forwarding message to subscriber {}", i);
            if let Err(e) = self.ctl_server.send(resp.clone(), *i).await {
                warn!("Failed to forward message to subscriber {}: {}", i, e);
                removals.push(*i);
            }
        }

        for i in removals {
            self.subscriptions.remove(&i);
        }
    }

    async fn handle_nmea(&mut self, index: usize, sentence_type: SentenceType, nmea: &nmea::Nmea) {
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

        let GpsHandle {
            state, satellites, ..
        } = &mut self.gps_handles[index];

        // Update state
        if let Some(fix) = nmea.fix_type {
            state.fix = fix;
        }
        state.altitude = nmea.altitude.map(|s| s as f64);
        state.speed = nmea.speed_over_ground.map(|s| s as f64);
        let loc = match (nmea.latitude, nmea.longitude) {
            (Some(lat), Some(lon)) => {
                let loc = Location::new(lat, lon);
                state.location = Some(loc);
                loc
            }
            _ => {
                trace!("No valid location fix");
                state.location = None;
                return;
            }
        };
        state.num_satellites = nmea.num_of_fix_satellites.unwrap_or(0);
        state.dop.hdop = nmea.hdop;
        state.dop.vdop = nmea.vdop;
        state.dop.pdop = nmea.pdop;

        *satellites = nmea.satellites().to_vec();
        let state = state.clone();

        // Update NTRIP actor with new location (if we've moved enough to warrant an update).
        // TODO: do we want to be specific about which GPS this comes from?
        self.maybe_update_ntrip_location(loc).await;

        // Forward relevant updates to subscribers
        match sentence_type {
            SentenceType::GGA | SentenceType::RMC => {
                let mut states = HashMap::new();
                states.insert(index as u32, state.clone());
                self.forward_to_subscribers(SubscriptionFlags::GpsState, Resp::State(states))
                    .await;
            }
            SentenceType::GSV => {
                let mut satellite_map = HashMap::new();
                satellite_map.insert(index as u32, nmea.satellites().to_vec());
                self.forward_to_subscribers(
                    SubscriptionFlags::GpsSatellites,
                    Resp::Satellites(satellite_map),
                )
                .await;
            }
            _ => {}
        }
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
            .and_then(|last_loc| loc.distance_to(&last_loc).ok())
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
        match ntrip_handle.update_location(loc).await {
            Ok(Some(m)) => {
                info!("Updated NTRIP mountpoint to {}", m.name);
                self.last_mount_location = Some(loc);
                self.current_mount = Some(m.name);
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
pub fn setup_logging(log_level: LevelFilter, enable_console: bool) {
    let fmt_filter = EnvFilter::builder()
        .from_env_lossy()
        .add_directive("hyper_util=WARN".parse().unwrap())
        .add_directive("reqwest=WARN".parse().unwrap())
        .add_directive("rustls=WARN".parse().unwrap())
        .add_directive(log_level.into());

    let fmt_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_filter(fmt_filter);

    let registry = tracing_subscriber::registry();
    let registry = match enable_console {
        true => registry.with(Some(ConsoleLayer::builder().with_default_env().spawn())),
        false => registry.with(None),
    };

    registry.with(fmt_layer).init();
}
