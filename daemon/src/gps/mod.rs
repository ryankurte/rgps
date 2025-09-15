use std::sync::{Arc, Mutex};

use futures::Stream;
use geoutils::Location;
use nmea::{Nmea, SentenceType};
use tokio::{io::AsyncReadExt, select};
use tokio_serial::{SerialPortBuilder, SerialPortBuilderExt as _, SerialStream};
use tracing::debug;

pub struct Gps {
    exit_tx: tokio::sync::broadcast::Sender<()>,
    state: Arc<Mutex<Nmea>>,
    update_rx: tokio::sync::mpsc::UnboundedReceiver<SentenceType>,
    _handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug, thiserror::Error)]
pub enum GpsError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serial port error: {0}")]
    SerialPortError(#[from] tokio_serial::Error),
}

impl Gps {
    pub async fn connect(port: &str, baud_rate: u32) -> Result<Self, GpsError> {
        debug!("Connecting to GPS on port {port} at {baud_rate} baud");

        let mut port = tokio_serial::new(port, baud_rate)
            .open_native_async()
            .map_err(GpsError::SerialPortError)?;

        debug!("Connected to serial port");

        let state = Arc::new(Mutex::new(Nmea::default()));

        let state_handle = state.clone();
        let (exit_tx, mut exit_rx) = tokio::sync::broadcast::channel(1);
        let (update_tx, update_rx) = tokio::sync::mpsc::unbounded_channel::<SentenceType>();

        let _handle = tokio::task::spawn(async move {
            debug!("Spawning GPS read task");

            let mut buff = Vec::with_capacity(1024);
            let mut nmea_parser = Nmea::default();
            let mut ublox_parser = ublox::Parser::default();

            loop {
                select! {
                    result = port.read_buf(&mut buff) => match result {
                        Ok(n) if n == 0 => {
                            debug!("GPS port closed");
                            break;
                        }
                        Ok(n) => {
                            debug!("Read {} bytes from GPS", n);

                            // TODO: NMEA vs. UBX detection / parsing
                            match str::from_utf8(&buff[..n]) {
                                Ok(s) => {
                                    for l in s.lines() {
                                        match nmea_parser.parse(l) {
                                            Ok(sentence) => {
                                                debug!("Parsed NMEA sentence: {:?}", sentence);
                                                debug!("Current state: {:?}", nmea_parser);

                                                state_handle.lock().unwrap().clone_from(&nmea_parser);

                                                update_tx.send(sentence).unwrap();
                                            }
                                            Err(e) => {
                                                debug!("Failed to parse NMEA sentence: {}", e);
                                            }
                                        }
                                    }
                                }
                                Err(_) => {
                                    let mut parsed = ublox_parser.consume_ubx(&buff[..n]);
                                    while let Some(m) = parsed.next() {
                                        debug!("Parsed UBX message: {:?}", m);
                                    }
                                }
                            }

                            buff.drain(..n);
                        }
                        Err(e) => {
                            debug!("Error reading from GPS: {}", e);
                        }
                    },
                    _e = exit_rx.recv() => {
                        debug!("Exiting GPS read task");
                        break;
                    }
                }
            }

            debug!("GPS read task exited");
        });

        Ok(Self {
            exit_tx,
            state,
            update_rx,
            _handle,
        })
    }

    /// Fetch current NMEA state
    pub async fn nmea(&self) -> Nmea {
        self.state.lock().unwrap().clone()
    }
}

impl Drop for Gps {
    fn drop(&mut self) {
        let _ = self.exit_tx.send(());
    }
}

impl Stream for Gps {
    type Item = SentenceType;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.update_rx.poll_recv(cx)
    }
}
