/// Error types for RGPSD
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error occurred while communicating with the NTRIP client
    #[error("NTRIP client error: {0}")]
    Ntrip(#[from] ntrip_client::NtripClientError),

    #[error("Failed to calculate distance: {0}")]
    DistanceCalc(String),

    #[error("NTRIP mount not found")]
    NoMountFound,

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Unexpected response")]
    UnexpectedResponse,

    #[error("Connector error: {0}")]
    Connector(#[from] tokio_connectors::error::Error),

    #[error("GPS error: {0}")]
    Gps(#[from] crate::gps::GpsError),
}
