use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use futures::{Sink, Stream};
use nmea::{Nmea, SentenceType};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    select,
    sync::{
        broadcast::Sender as BroadcastSender,
        mpsc::{UnboundedReceiver, UnboundedSender},
    },
};
use tokio_serial::SerialPortBuilderExt as _;
use tracing::{debug, error, trace};

mod parser;
use parser::GpsAccumulator;

/// A handle to a GPS device, providing a stream of updates* and a sink for sending
/// data / control messages to the GPS device.
///
/// Updates can be received as a [Stream] on [Gps] using [Gps::connect] or
/// sent to a previously created channel using [Gps::connect_with_channel].
pub struct Gps<RX = UnboundedReceiver<GpsMessage>> {
    exit_tx: tokio::sync::broadcast::Sender<()>,
    state: Arc<Mutex<Box<Nmea>>>,
    update_rx: RX,
    write_tx: UnboundedSender<Vec<u8>>,
    _handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug, thiserror::Error)]
pub enum GpsError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serial port error: {0}")]
    SerialPortError(#[from] tokio_serial::Error),
    #[error("Runtime error: {0}")]
    Runtime(#[from] anyhow::Error),
}

/// Messages received from the GPS
#[derive(Debug)]
pub enum GpsMessage {
    /// NMEA object with the type of sentence that was parsed (e.g. GGA, RMC, etc.)
    // TODO: rework / write a new NMEA parser that uses an enum?
    Nmea(Box<Nmea>, SentenceType),
    /// Parsed RTCM3 messages along with the raw byte representation for RTK forwarding etc.
    Rtcm3(Box<rtcm_rs::Message>, Vec<u8>),
    /// Raw UBX messages (not yet parsed, just forwarded as bytes)
    Ubx(Vec<u8>),
}

impl Gps<UnboundedReceiver<GpsMessage>> {
    /// Connect to a GPS device on the specified serial port and baud rate,
    pub async fn connect(port: &Path, baud_rate: u32) -> Result<Gps, GpsError> {
        let (update_tx, update_rx) = tokio::sync::mpsc::unbounded_channel::<GpsMessage>();
        let (exit_tx, _exit_rx) = tokio::sync::broadcast::channel(1);

        let (state, write_tx, _handle) =
            Self::connect_internal(port, baud_rate, exit_tx.clone(), update_tx).await?;

        Ok(Gps {
            exit_tx,
            state,
            update_rx,
            write_tx,
            _handle,
        })
    }
}

impl Gps<()> {
    /// Connect to a GPS device on the specified serial port and baud rate.
    ///
    /// Using the provided channel for updates instead of creating an internal one.
    pub async fn connect_with_channel(
        port: &Path,
        baud_rate: u32,
        update_tx: UnboundedSender<GpsMessage>,
    ) -> Result<Self, GpsError> {
        let (exit_tx, _exit_rx) = tokio::sync::broadcast::channel(1);

        let (state, write_tx, _handle) =
            Self::connect_internal(port, baud_rate, exit_tx.clone(), update_tx).await?;

        Ok(Gps {
            exit_tx,
            state,
            update_rx: (),
            write_tx,
            _handle,
        })
    }
}

impl<RX> Gps<RX> {
    /// Connect to a GPS device on the specified serial port and baud rate
    pub async fn connect_internal(
        serial_port: &Path,
        baud_rate: u32,
        exit_tx: BroadcastSender<()>,
        update_tx: UnboundedSender<GpsMessage>,
    ) -> Result<
        (
            Arc<Mutex<Box<Nmea>>>,
            UnboundedSender<Vec<u8>>,
            tokio::task::JoinHandle<()>,
        ),
        GpsError,
    > {
        debug!(
            "Connecting to GPS on port {} at {} baud",
            serial_port.display(),
            baud_rate
        );

        let mut port = tokio_serial::new(serial_port.as_os_str().to_str().unwrap(), baud_rate)
            .open_native_async()
            .map_err(GpsError::SerialPortError)?;

        debug!("Connected to serial port");

        let state = Arc::new(Mutex::new(Box::new(Nmea::default())));

        let state_handle = state.clone();
        let mut exit_rx = exit_tx.subscribe();
        let (write_tx, mut write_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

        let _handle = tokio::task::spawn(async move {
            debug!("Spawning GPS read task");

            let mut accumulator = GpsAccumulator::new();

            let mut buff: Vec<u8> = Vec::with_capacity(1024);

            'gps: loop {
                select! {
                    // Read from the GPS serial port
                    result = port.read_buf(&mut buff) => match result {
                        Ok(0) => {
                            debug!("GPS port closed");
                            break;
                        }
                        Ok(n) => {
                            trace!("Read {} bytes from GPS: {:02x?}", n, &buff[..n]);

                            // Add to accumulator
                            accumulator.push(&buff[..n]);

                            // Look for complete NMEA, RTCM, or UBX messages
                            while let Some(message) = accumulator.next_message() {
                                // Update internal NMEA state if it's an NMEA message
                                if let GpsMessage::Nmea(ref nmea, _) = message {
                                    let mut state = state_handle.lock().unwrap();
                                    *state = nmea.clone();
                                }

                                // Send the parsed message to the main task
                                if let Err(e) = update_tx.send(message) {
                                    error!("Failed to send GPS update: {}", e);
                                    break 'gps;
                                }
                            }

                            buff.clear();
                        }
                        Err(e) => {
                            debug!("Error reading from GPS: {}", e);
                        }
                    },
                    // Write to the GPS serial port
                    Some(data) = write_rx.recv() => {
                        match port.write(&data).await {
                            Ok(n) => {
                                // TODO: check n == data.len()
                                trace!("Wrote {} bytes to GPS", n);
                            }
                            Err(e) => {
                                debug!("Error writing to GPS: {}", e);
                            }
                        }
                    },
                    // Handle the exit signal
                    _e = exit_rx.recv() => {
                        debug!("Exiting GPS read task");
                        break;
                    }
                }
            }

            debug!("GPS read task exited");
        });

        Ok((state, write_tx, _handle))
    }

    /// Fetch current NMEA state
    pub async fn nmea(&self) -> Nmea {
        self.state.lock().unwrap().as_ref().clone()
    }
}

impl<RX> Drop for Gps<RX> {
    fn drop(&mut self) {
        let _ = self.exit_tx.send(());
    }
}

/// [Stream] of [GpsMessage] updates from the GPS device
///
/// Note: current GPS state can be fetched using the [nmea](Gps::nmea) method
impl Stream for Gps<UnboundedReceiver<GpsMessage>> {
    type Item = GpsMessage;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.update_rx.poll_recv(cx)
    }
}

/// [Sink] for sending raw data / control messages to the GPS device
impl<RX> Sink<Vec<u8>> for Gps<RX> {
    type Error = tokio::sync::mpsc::error::SendError<Vec<u8>>;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: Vec<u8>) -> Result<(), Self::Error> {
        self.write_tx.send(item)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
}
