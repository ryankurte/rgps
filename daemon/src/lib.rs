use clap::Parser;
use tokio::{io::AsyncReadExt, net::UnixListener, sync::broadcast::Sender as BroadcastSender, task};
use tracing::{error, span, trace, Instrument, Level};

use gpsrs_proto::{Req, Resp};

#[derive(Clone, PartialEq, Debug, Parser)]
pub struct Options {
    /// Daemon control socket
    #[clap(long, default_value = "debug")]
    pub ctl_sock: String,
}

/// GPS Daemon Context
pub struct Gpsd {
    ctl_task_handle: task::JoinHandle<Result<(), anyhow::Error>>,
}

impl Gpsd {
    pub async fn new(opts: Options, exit: BroadcastSender<()>) -> Result<Self, anyhow::Error> {
        // Setup unix listening socket

        // Clear the open file if it exists
        let _ = std::fs::remove_file(&opts.ctl_sock);

        // Bind the unix socket listener
        let ctl_listener = UnixListener::bind(&opts.ctl_sock)?;
        let (rx_sink, rx_stream) = tokio::sync::mpsc::channel::<Req>(0);

    


        // Create listening task
        let exit = exit.clone();
        let ctl_task_handle = task::spawn(
            async move {
                let mut index = 0u32;
                let mut e = exit.subscribe();

                loop {
                    tokio::select! {
                        c = ctl_listener.accept() => {
                            match c {
                                Ok((mut stream, addr)) => {
                                    println!("new client {addr:?}!");

                                    // Spawn a handler task for this stream
                                    let exit = exit.clone();
                                    let rx_sink = rx_sink.clone();
                                    task::spawn(async move {
                                        let (mut unix_rx, unix_tx) = stream.split();
                                        let (mut tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Resp>();
                                        let mut e = exit.subscribe();
                                        let mut rx_buff = [0u8; 1024];

                                        loop {
                                            tokio::select! {
                                                // Handle incoming requests from the client
                                                req = unix_rx.read(&mut rx_buff) => match req {
                                                    Ok(n) => {
                                                        // Process the request
                                                        let s = std::str::from_utf8(&rx_buff[..n]).unwrap();
                                                        println!("Received request: {s}");
                                                        let r = serde_json::from_str::<Req>(s).unwrap();

                                                        // Forward to request handling
                                                        rx_sink.send(r).await.unwrap();

                                                    }
                                                    Err(e) => {
                                                        // Channel closed
                                                        break;
                                                    }
                                                },
                                                // Handle outgoing responses for this client
                                                resp = rx.recv() => match resp {
                                                    Some(resp) => {
                                                        // Send the response
                                                    }
                                                    None => {
                                                        // Channel closed
                                                        break;
                                                    }
                                                },

                                                // Handle exit signal
                                                _ = e.recv() => {
                                                    println!("Received exit signal");
                                                    break;
                                                }
                                            }
                                        }
                                    });


                                    index += 1;
                                }
                                Err(e) => { 
                                    error!("Failed to accept connection: {e:?}");
                                }
                            }
                        },

                        _ = e.recv() => {
                            println!("Received exit signal");
                            break;
                        }
                    }
                }

                Ok(())
            }
        );

        Ok(Self {
            ctl_task_handle,
        })
    }
}