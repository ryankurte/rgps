//! A NTRIP handler task for the GPS daemon.
//!
//! This connects to an NTRIP caster and forwards corrections to the GPS device,
//! updating the mounts based on the current GPS location.
//!

use geoutils::Location;
use rtcm_rs::Message;
use tokio::{
    select,
    sync::{mpsc::UnboundedSender, oneshot::Sender as OneshotSender},
    task::JoinHandle,
};

use ntrip_client::{MountInfo, NtripClient, NtripHandle};
use tracing::{debug, info, trace, warn};

use crate::{config::Ntrip, error::Error};

pub struct NtripContext {
    config: Ntrip,
    /// The underlying NTRIP client
    client: NtripClient,
    /// A handle to the currently mounted NTRIP service
    mount: Option<CurrentMount>,
    /// The current position of the GPS device
    location: Option<Location>,
    /// A sink for forwarding received RTCM messages to the GPS task for writing to the GPS device
    ntrip_sink: UnboundedSender<(Message, Vec<u8>)>,
}

pub struct NtripActor {
    cmd_tx: UnboundedSender<(NtripCmd, OneshotSender<NtripRes>)>,
    _handle: JoinHandle<Result<(), Error>>,
}

struct CurrentMount {
    info: MountInfo,
    _handle: NtripHandle<()>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum NtripState {
    /// NTRIP not configured
    Unconfigured,
    /// NTRIP configured but not mounted
    Available,
    /// NTRIP mounted and running
    Running(MountInfo),
}

/// Commands that can be sent to the NTRIP actor
#[derive(Clone, Debug, PartialEq)]
enum NtripCmd {
    /// Fetch the current NTRIP state
    GetState,

    /// Fetch available mount points
    ListMounts,

    /// A new GPS location update, used to update the NTRIP mountpoint
    UpdateLocation(Location),
}

/// Responses from the NTRIP actor
#[derive(Debug)]
enum NtripRes {
    Ok,

    State(NtripState),

    Mounts(Vec<MountInfo>),

    Error(Error),
}

impl NtripActor {
    /// Spawn a new NTRIP actor with the given configuration
    pub async fn spawn(
        config: Ntrip,
        ntrip_tx: UnboundedSender<(Message, Vec<u8>)>,
    ) -> Result<Self, Error> {
        let (cmd_tx, mut cmd_rx) =
            tokio::sync::mpsc::unbounded_channel::<(NtripCmd, OneshotSender<NtripRes>)>();

        // Setup the NTRIP context and spawn the actor task
        let mut ctx = NtripContext::new(config, ntrip_tx).await?;

        let _handle = tokio::task::spawn(async move {
            loop {
                select! {
                    // Handle incoming commands
                    Some((msg, tx)) = cmd_rx.recv() => {
                        let res = ctx.handle_cmd(msg).await;
                        let _ = tx.send(res);
                    },
                    else => {
                        debug!("NTRIP command channel closed, exiting NTRIP actor");
                        return Ok(());
                    },
                }
            }
        });

        Ok(Self { cmd_tx, _handle })
    }

    /// Send a command to the NTRIP actor and await the response
    async fn cmd<R: TryFrom<NtripRes>>(&self, cmd: NtripCmd) -> Result<R, Error> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        trace!("Sending NTRIP command: {:?}", cmd);

        self.cmd_tx
            .send((cmd, tx))
            .map_err(|_| Error::ChannelClosed)?;

        let res = rx.await.map_err(|_| Error::ChannelClosed)?;

        trace!("Received NTRIP response: {:?}", res);

        let res = R::try_from(res).map_err(|_| Error::UnexpectedResponse)?;

        Ok(res)
    }

    /// Fetch the current NTRIP state
    pub async fn get_state(&self) -> Result<NtripState, Error> {
        self.cmd(NtripCmd::GetState).await
    }

    /// List available NTRIP mounts from the caster
    pub async fn list_mounts(&self) -> Result<Vec<MountInfo>, Error> {
        self.cmd(NtripCmd::ListMounts).await
    }

    /// Update the current GPS location, which may trigger a remount if we move far enough from the current NTRIP mount location
    pub async fn update_location(&self, location: Location) -> Result<Option<MountInfo>, Error> {
        self.cmd(NtripCmd::UpdateLocation(location)).await
    }
}

impl NtripContext {
    pub async fn new(
        config: Ntrip,
        ntrip_sink: UnboundedSender<(Message, Vec<u8>)>,
    ) -> Result<Self, Error> {
        let ntrip_client =
            NtripClient::new(config.host.clone(), config.ntrip_creds.clone()).await?;

        Ok(Self {
            config,
            client: ntrip_client,
            mount: None,
            location: None,
            ntrip_sink,
        })
    }

    async fn update_location(&mut self, location: Location) -> Result<Option<MountInfo>, Error> {
        trace!(
            "Update location to ({}, {})",
            location.latitude(),
            location.longitude()
        );

        // Update the current location
        self.location = Some(location);

        // If we have a mount, check if it's within the distance threshold.
        // Otherwise, we'll try find a new one.
        match &self.mount {
            Some(m) => {
                let distance = m
                    .info
                    .location
                    .distance_to(&location)
                    .map_err(Error::DistanceCalc)?;

                if distance.meters() < self.config.distance_threshold {
                    debug!(
                        "Current mount {} is within distance threshold, no remount needed",
                        m.info.name
                    );
                    return Ok(None);
                }

                info!(
                    "Current mount {} is too far from new location, remounting...",
                    m.info.name
                );
            }
            None => {
                info!("Attempting to find and mount an NTRIP service for new location...");
            }
        }

        // See if we can find a (new|closer) mount
        let server_info = self.client.list_mounts().await.map_err(Error::Ntrip)?;

        let nearest = match server_info.find_nearest(&location) {
            Some((m, d)) if d < self.config.distance_threshold => {
                debug!("Found a closer mount {} at distance {} m", m.name, d);
                m
            }
            _ => {
                warn!("No closer mount found within distance threshold, remounting failed");
                // TODO: if we're out of the distance threshold of the current mount
                // should we drop it or continue using it even though it's far away?
                return Err(Error::NoMountFound);
            }
        };

        // If we found a closer mount, connect to it
        debug!(
            "Connecting to new mount {} at location ({}, {})",
            nearest.name,
            nearest.location.latitude(),
            nearest.location.longitude()
        );
        let handle = self
            .client
            .mount_with_sink(nearest.name.clone(), self.ntrip_sink.clone())
            .await
            .map_err(Error::Ntrip)?;

        info!("Successful (re)mount {}", nearest.name);

        // Update the current mount handle
        self.mount = Some(CurrentMount {
            info: nearest.clone(),
            _handle: handle,
        });

        // Return the new mount info
        Ok(Some(nearest.clone()))
    }

    async fn handle_cmd(&mut self, msg: NtripCmd) -> NtripRes {
        match msg {
            NtripCmd::GetState => {
                let state = match self.mount.as_ref() {
                    Some(m) => NtripState::Running(m.info.clone()),
                    None => NtripState::Available,
                };

                NtripRes::State(state)
            }
            NtripCmd::ListMounts => match self.client.list_mounts().await {
                Ok(server_info) => NtripRes::Mounts(server_info.services),
                Err(e) => NtripRes::Error(e.into()),
            },
            NtripCmd::UpdateLocation(location) => match self.update_location(location).await {
                Ok(Some(mount)) => NtripRes::Mounts(vec![mount]),
                Ok(None) => NtripRes::Ok,
                Err(e) => NtripRes::Error(e),
            },
        }
    }
}

impl TryFrom<NtripRes> for Vec<MountInfo> {
    type Error = Error;

    fn try_from(value: NtripRes) -> Result<Self, Self::Error> {
        match value {
            NtripRes::Mounts(m) => Ok(m),
            NtripRes::Error(e) => Err(e),
            _ => Err(Error::UnexpectedResponse),
        }
    }
}

impl TryFrom<NtripRes> for Option<MountInfo> {
    type Error = Error;

    fn try_from(value: NtripRes) -> Result<Self, Self::Error> {
        match value {
            NtripRes::Ok => Ok(None),
            NtripRes::Mounts(m) => Ok(m.into_iter().next()),
            NtripRes::Error(e) => Err(e),
            _ => Err(Error::UnexpectedResponse),
        }
    }
}

impl TryFrom<NtripRes> for NtripState {
    type Error = Error;

    fn try_from(value: NtripRes) -> Result<Self, Self::Error> {
        match value {
            NtripRes::State(s) => Ok(s),
            NtripRes::Error(e) => Err(e),
            _ => Err(Error::UnexpectedResponse),
        }
    }
}
