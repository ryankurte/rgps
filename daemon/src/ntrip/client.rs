use std::sync::Arc;

use anyhow::Context;
use base64::{alphabet::STANDARD, engine::general_purpose, prelude::BASE64_STANDARD, Engine as _};
use clap::Parser;
use crc_any::CRC;
use futures::{AsyncReadExt, Stream};
use http::{HeaderMap, HeaderValue, header::USER_AGENT};
use hyper::Method;
use isocountry::CountryCode;
use robust_ntrip_client::RobustNtripClientOptions;
use rtcm_rs::{Message, MessageFrame, next_msg_frame};
use rustls::pki_types::ServerName;
use strum::{Display, EnumString, VariantNames};
use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
    select,
    sync::{
        broadcast::Sender as BroadcastSender,
        mpsc::{unbounded_channel, UnboundedReceiver},
    }, task::JoinHandle,
};
use tokio_rustls::TlsConnector;
use tracing::{debug, error, trace, warn};

use crate::ntrip::{MountInfo, ServerInfo};

use super::{NtripConfig, RtcmProvider};

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
        let proto = if config.use_tls { "https" } else { "http" };

        let req = client
            .request(
                Method::GET,
                format!("{}://{}:{}", proto, config.host, config.port),
            )
            .header("Ntrip-Version", "NTRIP/2.0")
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

        debug!("Connecting to NTRIP server {}/{}", config.url(), mount.to_string());

        let sock = TcpStream::connect(&config.url()).await.with_context(|| "Failed to connect to NTRIP server")?;
        
       let (rx_handle, ntrip_rx) = match config.use_tls {
            true => {
                debug!("Using TLS connection");

                let mut root_cert_store = rustls::RootCertStore::empty();
                root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

                let tls_config = rustls::ClientConfig::builder()
                    .with_root_certificates(root_cert_store)
                    .with_no_client_auth();
                let connector = TlsConnector::from(Arc::new(tls_config));
                let dnsname = ServerName::try_from(config.host
                .clone()).unwrap();

                let tls_sock = connector.connect(dnsname, sock).await.with_context(|| "Failed to establish TLS connection")?;

                Self::handle_connection(&config, &mount.to_string(), exit_tx.clone(), tls_sock).await.with_context(|| "Connection handler failed")?

            },
            false => {
                debug!("Using plain TCP connection");
                
                Self::handle_connection(&config, &mount.to_string(), exit_tx.clone(), sock).await.with_context(|| "Connection handler failed")?
            }
        };

        Ok(RtcmClient {
            _rx_handle: rx_handle,
            ntrip_rx,
        })
    }

    pub async fn handle_connection(config: &NtripConfig, mount: &str, exit_tx: BroadcastSender<()>, mut sock: impl AsyncRead + AsyncWrite + Unpin + Send + 'static) -> Result<(JoinHandle<()>, UnboundedReceiver<Message>), anyhow::Error>{
        // Setup HTTP headers
        let mut headers = HeaderMap::new();
        headers.append(
            USER_AGENT,
            HeaderValue::from_str(&format!(
                "NTRIP {}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .unwrap(),
        );

        headers.append("Ntrip-Version", HeaderValue::from_static("NTRIP/2.0"));
        headers.append("Accept", HeaderValue::from_static("*/*"));
        headers.append("Connection", HeaderValue::from_static("close"));

        // If we have credentials, add the Authorization header
        if !config.user.is_empty() {
            let auth = general_purpose::STANDARD.encode(format!("{}:{}", config.user, config.pass));
            headers.append( "Authorization", HeaderValue::from_str(&format!("Basic {}", auth)).unwrap());
        }

        debug!("Headers: {:#?}", headers);

        // Write HTTP request
        debug!("Write HTTP request");
        sock.write_all(format!("GET /{} HTTP/1.0\r\n", mount.to_string()).as_bytes())
            .await
            .unwrap();
        sock.write_all(format!("Host: {}\r\n", config.url()).as_bytes())
            .await
            .unwrap();

        // Write HTTP headers
        debug!("Writing headers");
        for h in headers.iter() {
            sock.write_all(format!("{}: {}\r\n", h.0.as_str(), h.1.to_str().unwrap()).as_bytes())
                .await
                .unwrap();
        }

        sock.write_all(b"\r\n").await.unwrap();
        sock.flush().await.unwrap();

        debug!("Reading response");
        let mut buff = Vec::with_capacity(1024);

        // Spawn a task to handle incoming NTRIP data

        let (ntrip_tx, ntrip_rx) = unbounded_channel();
        let mut exit_rx = exit_tx.subscribe();
        let rx_handle = tokio::task::spawn(async move {
            // Track parse errors so we can drop data (or abort) if needed
            let mut error_count = 0;

            'listener: loop {
                select! {
                    n = sock.read_buf(&mut buff) => match n {
                        Ok(n) => {
                            debug!("Read {} bytes, current buffer {} bytes", n, buff.len());
                            trace!("Appended {:02x?}", &buff[buff.len()-n..][..n]);

                            if n == 0 {
                                warn!("Zero length response");
                                exit_tx.send(()).unwrap();
                                break 'listener;
                            }

                            // Parse and check / remove response status
                            const ICY_STATUS: &str = "ICY 200 OK\r\n";
                            if buff[..n].starts_with(ICY_STATUS.as_bytes()) {
                                debug!("Got ICY 200 OK");
                                let _ = buff.drain(..ICY_STATUS.len());
                            }

                            // TODO: what about other types of HTTP?
                            const HTTP_STATUS: &str = "HTTP/1.1 200 OK\r\n";
                            if buff[..n].starts_with(HTTP_STATUS.as_bytes()) {
                                debug!("Got HTTP/1.1 200 OK");
                                let _ = buff.drain(..HTTP_STATUS.len());
                            }

                            // Trim any non-message data from the start of the buffer
                            if buff[0] != 0xd3 {
                                if let Some(i) = buff.iter().enumerate().find(|(_i, b)| **b == 0xd3) {
                                    warn!("Trimming buffer to next potential frame start at index {}", i.0);
                                    buff.drain(..i.0);

                                    assert_eq!(buff[0], 0xd3);
                                }
                            }

                            // While we have enough data for a header,
                            // parse out RTCM messages
                            while buff.len() > 6 {
                                // Attempt to parse frames
                                match MessageFrame::new(&buff[..]) {
                                    Ok(f) => {
                                        // Parse out message from frame
                                        let m = f.get_message();

                                        debug!("Parsed RTCM message: {:?} (consumed {} bytes)", m, f.frame_len());

                                        // Emit message
                                        ntrip_tx.send(m).unwrap();

                                        // Remove parsed data from the buffer
                                        let _ = buff.drain(..f.frame_len());

                                        // Reset error counter
                                        error_count = 0;
                                    },
                                    Err(e) => {
                                        warn!("RTCM parse error: {} (count: {})", e, error_count);

                                        // Update error counter
                                        error_count += 1;

                                        // If we keep getting errors, abort the connection
                                        if error_count >= 5 {
                                            error!("Too many parse errors, closing connection");
                                            break 'listener;
                                        }

                                        break;
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            error!("socket read error: {}", e);
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

            if buff.len() > 0 {
                warn!("Dropping {} bytes of unparsed data", buff.len());

                if let Ok(s) = String::from_utf8(buff) {
                    debug!("Unparsed data:\r\n{}", s);
                }
            }
        });

        Ok((rx_handle, ntrip_rx))
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
    use futures::StreamExt;

    use tracing::{debug};

    use crate::ntrip::RtcmClient;

    fn setup_logging() {
        let _ = tracing_subscriber::FmtSubscriber::builder()
            .compact()
            .without_time()
            .with_max_level(tracing::level_filters::LevelFilter::DEBUG)
            .try_init();
    }

    #[tokio::test]
    async fn test_ntrip_client() {
        setup_logging();

        const HOST: &str = "192.168.0.158";
        const MOUNT: &str = "ARGOACU";

        debug!("Connecting to NTRIP server");

        let (exit_tx, _exit_rx) = tokio::sync::broadcast::channel(1);

        let config = crate::ntrip::NtripConfig {
            host: HOST.into(),
            ..Default::default()
        };
        let mut client = RtcmClient::mount(config, MOUNT.to_string(), exit_tx.clone())
            .await
            .unwrap();

        for _i in 0..10 {
            let m = client.next().await.unwrap();
            debug!("Got RTCM message: {:?}", m);
        }

        let _ = exit_tx.send(());
    }
}
