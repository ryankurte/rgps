use clap::Parser;
use futures::Stream;
use isocountry::CountryCode;
use robust_ntrip_client::RobustNtripClientOptions;
use rtcm_rs::{Message, MessageFrame, next_msg_frame};
use strum::{Display, EnumString, VariantNames};
use tokio::{
    select,
    sync::{
        broadcast::Sender as BroadcastSender,
        mpsc::{UnboundedReceiver, unbounded_channel},
    },
};
use tracing::{debug, error};

use super::{NtripConfig, RtcmProvider, parser::ParsingNtripClient};

/// RTCM NTRIP Client
pub struct RtcmClient {
    _rx_handle: tokio::task::JoinHandle<()>,
    ntrip_rx: UnboundedReceiver<Message>,
}

impl RtcmClient {
    pub async fn connect(
        config: NtripConfig,
        mount: impl ToString,
        exit_tx: BroadcastSender<()>,
    ) -> Result<Self, anyhow::Error> {
        // Generate NTRIP URL
        let url = format!(
            "ntrip://{}:{}@{}/{}",
            config.user,
            config.pass,
            config.host,
            mount.to_string()
        );

        debug!("Connecting to NTRIP URL: {}", url);

        // Connect to NTRIP caster
        let raw_client = robust_ntrip_client::RobustNtripClient::new(
            &url,
            RobustNtripClientOptions {
                timeout: Some(std::time::Duration::from_secs(5)),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create NTRIP client: {}", e))?;
        let mut ntrip = ParsingNtripClient::new(raw_client);

        // Spawn a task to handle incoming NTRIP data

        let (ntrip_tx, ntrip_rx) = unbounded_channel();
        let mut exit_rx = exit_tx.subscribe();
        let _rx_handle = tokio::task::spawn(async move {
            loop {
                select! {
                    n = ntrip.next() => match n {
                        Ok(m) => {
                            debug!("Received NTRIP data message {}, {} bytes", m.message_number, m.frame_data.len());

                            let m = match MessageFrame::new(&m.frame_data) {
                                Ok(frame) => frame.get_message(),
                                Err(e) => {
                                    error!("Failed to parse RTCM message frame: {}", e);
                                    continue;
                                }
                            };

                            debug!("Parsed RTCM message: {:?}", m);

                            ntrip_tx.send(m).unwrap();
                        },
                        Err(e) => {
                            error!("NTRIP error: {}", e);
                            break;
                        },
                    },
                    _ = exit_rx.recv() => {
                        debug!("Exiting NTRIP read loop on signal");
                        break;
                    }
                }
            }
        });

        Ok(RtcmClient {
            _rx_handle,
            ntrip_rx,
        })
    }
}

impl Stream for RtcmClient {
    type Item = Message;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.ntrip_rx.poll_recv(cx)
    }
}
