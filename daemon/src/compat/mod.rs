//! A gpsd compatibility layer for rgpsd.
//!
//! This provides a TCP server implementing a chunk of the GPSD protocol,
//! allowing existing GPSD clients to connect and receive GPS state updates and
//! satellite information from rgpsd.
//!

#![allow(dead_code, unused)]
use std::fmt::Display;

use futures::StreamExt;
use gpsd_proto::UnifiedResponse;
use serde::{Deserialize, Serialize};
use tokio::{
    select,
    sync::{broadcast::Sender as BroadcastSender, mpsc::UnboundedReceiver},
    task,
};
use tokio_connectors::{codecs::Codec, error::Error as CodecError, tcp::TcpServer};
use tracing::debug;

mod commands;
use commands::{DeviceArgs, GpsdCommand, WatchArgs};

/// GPSD compatibility layer for rgpsd.
pub struct Compat {
    /// The TCP server implementing the GPSD protocol.
    server: TcpServer<GpsdCodec, UnifiedResponse, GpsdCommand>,
}

/// Codec for encoding/decoding GPSD protocol messages.
pub struct GpsdCodec;

pub struct CompatActor {
    _handle: tokio::task::JoinHandle<Result<(), CodecError>>,
    exit_tx: BroadcastSender<()>,
}

impl CompatActor {
    pub async fn spawn(addr: impl tokio::net::ToSocketAddrs + Display) -> Result<Self, CodecError> {
        let (exit_tx, mut exit_rx) = tokio::sync::broadcast::channel(1);

        let mut compat = Compat::bind(addr).await?;

        let handle = tokio::task::spawn(async move {
            loop {
                select! {
                    // Handle incoming GPSD commands from clients
                    Some((req, addr)) = compat.server.next() => {
                        debug!("Received GPSD command from {}: {:?}", addr, req);
                        let res = compat.handle_cmd(req).await;

                        debug!("Responding to {} with: {:?}", addr, res);
                        compat.server.send(res, addr).await?;
                    }
                    _ = exit_rx.recv() => {
                        debug!("Received exit signal, shutting down GPSD compatibility server");
                        return Ok(());
                    }
                }
            }
        });

        Ok(Self {
            _handle: handle,
            exit_tx,
        })
    }
}

impl Compat {
    /// Create a new GPSD compatibility server listening on the specified address.
    pub async fn bind<A: tokio::net::ToSocketAddrs + Display>(addr: A) -> Result<Self, CodecError> {
        debug!("Starting GPSD compatibility server on {}", addr);

        let server = TcpServer::bind(addr).await?;

        Ok(Self { server })
    }

    async fn handle_cmd(&mut self, cmd: GpsdCommand) -> UnifiedResponse {
        match &cmd {
            GpsdCommand::Version => UnifiedResponse::Version(gpsd_proto::Version {
                release: env!("CARGO_PKG_VERSION").to_string(),
                rev: "TODO".to_string(),
                proto_major: 3,
                proto_minor: 0,
                remote: None,
            }),
            GpsdCommand::Devices => {
                debug!("Received DEVICES command");

                UnifiedResponse::Error(gpsd_proto::ErrorResponse {
                    message: format!("Unsupported command: {:?}", cmd),
                })
            }
            GpsdCommand::Watch(args) => {
                debug!("Received WATCH command with args: {:?}", args);

                UnifiedResponse::Error(gpsd_proto::ErrorResponse {
                    message: format!("Unsupported command: {:?}", cmd),
                })
            }
            GpsdCommand::Poll => {
                debug!("Received POLL command");

                UnifiedResponse::Error(gpsd_proto::ErrorResponse {
                    message: format!("Unsupported command: {:?}", cmd),
                })
            }
            GpsdCommand::Device(args) => {
                debug!("Received DEVICE command with args: {:?}", args);

                UnifiedResponse::Error(gpsd_proto::ErrorResponse {
                    message: format!("Unsupported command: {:?}", cmd),
                })
            }
        }
    }
}

/// Implement the Codec trait for GpsdCodec to handle GPSD protocol messages.
impl Codec<UnifiedResponse, GpsdCommand> for GpsdCodec {
    /// Encode a [UnifiedResponse] as JSON bytes for sending to GPSD clients.
    fn encode(item: &UnifiedResponse) -> Result<Vec<u8>, CodecError> {
        serde_json::to_vec(item).map_err(|_e| CodecError::Send)
    }

    /// Decode a [GpsdCommand] from the incoming byte stream from GPSD clients.
    fn try_decode(src: &mut Vec<u8>) -> Result<Option<GpsdCommand>, CodecError> {
        // Look for the command start character ('?') in the buffer
        let start_pos = match src.iter().position(|&b| b == b'?') {
            Some(p) => p,
            None => return Ok(None), // No command start found yet
        };

        // Drain the buffer up to the start position
        if start_pos > 0 {
            src.drain(0..start_pos);
        }

        // Convert the buffer to utf8
        let s = match std::str::from_utf8(src) {
            Ok(s) => s,
            Err(_) => return Ok(None),
        };

        let (cmd, n) = GpsdCommand::parse(&s).map_err(|_e| CodecError::Send)?;

        // Drain the buffer up to the end of the command
        src.drain(0..n);

        Ok(Some(cmd))
    }
}
