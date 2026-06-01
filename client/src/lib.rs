use std::{collections::HashMap, time::Duration};

use clap::Parser;
use futures::{Stream, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

// Re-export core types to simplify client use
pub use rgps_core::{GpsInfo, GpsState, SubscriptionFlags, req::Req, resp::Resp};

use tokio_connectors::codecs::Json;

// For unix-based platforms we use a Unix domain socket for the control interface.
#[cfg(target_family = "unix")]
use rgps_core::default_sock_path;
#[cfg(target_family = "unix")]
use tokio_connectors::unix::UnixClient;

// For non-unix (aka Windows) we use a TCP socket for the control interface,
// since windows unix domain socket support has got lost somewhere.
#[cfg(not(target_family = "unix"))]
use rgps_core::default_sock_addr;
#[cfg(not(target_family = "unix"))]
use std::net::SocketAddr;
#[cfg(not(target_family = "unix"))]
use tokio_connectors::tcp::TcpClient;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, Parser)]
pub struct RgpsClientConfig {
    /// Daemon control socket
    #[cfg(target_family = "unix")]
    #[clap(long, default_value = default_sock_path().into_os_string(), env = "RGPSD_CTL_SOCK")]
    #[serde(default = "default_sock_path")]
    pub ctl_sock: std::path::PathBuf,

    /// Daemon control socket (TCP for non-unix platforms)
    #[cfg(not(target_family = "unix"))]
    #[clap(long, default_value = "127.0.0.1:8666", env = "RGPSD_CTL_SOCK")]
    #[serde(default = "default_sock_addr")]
    pub ctl_sock: SocketAddr,

    /// Timeout for client requests (default: 5 seconds)
    #[clap(long, value_parser = humantime::parse_duration, default_value = "5s")]
    pub timeout: Duration,
}

/// A client connected to the RGPS daemon
pub struct RgpsClient {
    #[cfg(target_family = "unix")]
    client: UnixClient<Json, Req, Resp>,

    #[cfg(not(target_family = "unix"))]
    client: TcpClient<Json, Req, Resp>,

    /// Timeout for client requests
    timeout: Duration,
}

// TODO: allow UnixClient/TcpClient to forward errors on disconnect (fix in tokio_connectors crate)?

impl RgpsClient {
    /// Connect to the RGPS daemon using the provided configuration
    pub async fn connect(config: &RgpsClientConfig) -> anyhow::Result<Self> {
        #[cfg(target_family = "unix")]
        let client = UnixClient::<Json, Req, Resp>::connect(&config.ctl_sock).await?;

        #[cfg(not(target_family = "unix"))]
        let client = TcpClient::<Json, Req, Resp>::connect(config.ctl_sock).await?;

        Ok(Self {
            client,
            timeout: config.timeout,
        })
    }

    async fn send<RESP: TryFrom<Resp>>(&mut self, req: impl Into<Req>) -> anyhow::Result<RESP>
    where
        <RESP as TryFrom<Resp>>::Error: std::fmt::Debug,
    {
        let req = req.into();

        debug!("Sending request: {:?}", req);
        self.client.send(req).await?;

        trace!("Awaiting response...");
        let resp = match tokio::time::timeout(self.timeout, self.client.next()).await {
            Ok(Some(resp)) => resp,
            Ok(None) => return Err(anyhow::anyhow!("Connection closed by server")),
            Err(_) => return Err(anyhow::anyhow!("Request timed out")),
        };

        trace!("Received response: {:?}", resp);

        // Convert the response to the expected type
        let res = resp
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to convert response: {e:?}"))?;

        Ok(res)
    }

    /// Fetch information about connected GPS devices
    pub async fn get_info(&mut self) -> anyhow::Result<HashMap<u32, GpsInfo>> {
        self.send(Req::GetInfo).await
    }

    /// Fetch the current state of connected GPS devices
    pub async fn get_state(&mut self) -> anyhow::Result<HashMap<u32, GpsState>> {
        self.send(Req::GetState).await
    }

    /// Fetch the current satellite information of connected GPS devices
    pub async fn get_satellites(&mut self) -> anyhow::Result<HashMap<u32, Vec<nmea::Satellite>>> {
        self.send(Req::GetSatellites).await
    }

    /// Subscribe to updates for specific message types
    pub async fn subscribe(&mut self, flags: Vec<SubscriptionFlags>) -> anyhow::Result<()> {
        self.send::<()>(Req::Subscribe(flags)).await
    }

    /// Unsubscribe from all message types
    pub async fn unsubscribe(&mut self) -> anyhow::Result<()> {
        self.send::<()>(Req::Subscribe(vec![])).await
    }
}

/// Implement Stream for RgpsClient to allow streaming responses (e.g. for subscriptions)
impl Stream for RgpsClient {
    type Item = Resp;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.client.poll_next_unpin(cx)
    }
}
