use clap::Parser;
use futures::Stream;
use hyper::Method;
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
use tracing::{debug, error, warn};

use crate::ntrip::{MountInfo, ServerInfo};

use super::{NtripConfig, RtcmProvider, parser::ParsingNtripClient};

/// RTCM NTRIP Client
pub struct RtcmClient {
    _rx_handle: tokio::task::JoinHandle<()>,
    ntrip_rx: UnboundedReceiver<Message>,
}

impl RtcmClient {
    pub async fn list_mounts(config: NtripConfig) -> Result<ServerInfo, anyhow::Error> {
        let client = reqwest::Client::builder()
            .http1_ignore_invalid_headers_in_responses(true)
            .http09_responses()
            .user_agent(format!(
                "NTRIP {}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .unwrap();

        // TODO: auth etc.

        let req = client
            .request(
                Method::GET,
                format!("http://{}:{}", config.host, config.port),
            )
            .header("Ntrip-Version", "Ntrip/2.0")
            .build()
            .unwrap();

        let res = client.execute(req).await?;

        debug!("Fetched NTRIP response: {:?}", res.status());

        let body = res.text().await?;

        let lines = body.lines().collect::<Vec<&str>>();

        let snip_info = ServerInfo::parse(lines.iter().cloned());

        Ok(snip_info)
    }

    pub async fn mount(
        config: NtripConfig,
        mount: impl ToString,
        exit_tx: BroadcastSender<()>,
    ) -> Result<Self, anyhow::Error> {
        // Generate NTRIP URL
        let url = if config.user != "" {
            format!("http://{}:{}/{}", config.host, config.port, mount.to_string())
        } else {
            format!("http://{}:{}/{}", config.host, config.port, mount.to_string())
        };

        debug!("Connecting to NTRIP URL: {}", url);

        // Setup HTTP client for NTRIP messaging
        let client = reqwest::Client::builder()
            // .tcp_keepalive(std::time::Duration::from_secs(5))
            .http1_ignore_invalid_headers_in_responses(true)
            .http1_allow_obsolete_multiline_headers_in_responses(true)
            .http1_allow_spaces_after_header_name_in_responses(true)
            .http09_responses()
            .http1_only()
            .user_agent(format!(
                "NTRIP {}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .unwrap();

        // Build NTRIP request
        let mut req = client
            .request(Method::GET, &url)
            .header("Ntrip-Version", "Ntrip/2.0")
            .header("Accept", "*/*");

        // Add basic auth if username or password provided
        if config.user != "" || config.pass != "" {
            req = req.basic_auth(config.user.clone(), Some(config.pass.clone()));
        }
        // Finalise request
        let req = req
            .build()
            .unwrap();

        // Issue request to NTRIP server and check response status
        let mut exit_rx = exit_tx.subscribe();
        let mut response = select!{
            r = client.execute(req) => r?,
            _ = exit_rx.recv() => {
                return Err(anyhow::anyhow!("NTRIP connection aborted on exit signal"));
            },
        };
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "NTRIP server returned error status: {}",
                response.status()
            ));
        }

        // Spawn a task to handle incoming NTRIP data

        let (ntrip_tx, ntrip_rx) = unbounded_channel();
        let mut exit_rx = exit_tx.subscribe();
        let _rx_handle = tokio::task::spawn(async move {
            loop {
                select! {
                    d = response.chunk() => match d {
                        Ok(Some(d)) => {
                            debug!("Received {} byte chunk", d.len());

                            debug!("{}", String::from_utf8_lossy(&d));

                            //debug!("Parsed RTCM message: {:?}", m);

                            //ntrip_tx.send(m).unwrap();
                        },
                        Ok(None) => {
                            continue;
                        }
                        Err(e) => {
                            error!("HTTP chunk error: {}", e);
                            break;
                        },
                    },
                    _ = exit_rx.recv() => {
                        error!("Exiting NTRIP read loop on signal");
                        break;
                    }
                }
            }
            warn!("NTRIP read loop exiting");
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

#[cfg(test)]
mod tests {
    use http::{header::USER_AGENT, HeaderMap, HeaderValue};
    use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};
    use tracing::debug;

    fn setup_logging() {
        let _ = tracing_subscriber::FmtSubscriber::builder()
            .compact()
            .without_time()
            .with_max_level(tracing::level_filters::LevelFilter::TRACE)
            .try_init();
    }


    #[tokio::test]
    async fn test_ntrip_wtf() {
        setup_logging();

        debug!("Connecting to NTRIP server");
        let mut sock = TcpStream::connect("3.143.243.81:2101").await.unwrap();

        let mut headers = HeaderMap::new();
        headers.append(USER_AGENT, HeaderValue::from_str(&format!(
                "NTRIP {}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            )).unwrap());

        headers.append("Ntrip-Version", HeaderValue::from_static("Ntrip/2.0"));
        headers.append("Accept", HeaderValue::from_static("*/*"));

        debug!("Write HTTP request");
        sock.write_all(b"GET /AFUMRTCM HTTP/1.1\r\n").await.unwrap();
        sock.write_all(b"Host: 3.143.243.81:2101\r\n").await.unwrap();

        debug!("Writing headers");
        for h in headers.iter() {
            sock.write_all(format!("{}: {}\r\n", h.0.as_str(), h.1.to_str().unwrap()).as_bytes()).await.unwrap();
        }
        sock.flush().await.unwrap();

        debug!("Reading response");

        let mut buf = vec![0; 1024];
        let n = sock.read(&mut buf).await.unwrap();
        debug!("Read {} bytes", n);
        debug!("\r\n{}", String::from_utf8_lossy(&buf[..n]));

    }
}