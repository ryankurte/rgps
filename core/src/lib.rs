use std::{net::SocketAddr, path::PathBuf};

use clap::ValueEnum;
use enumset::EnumSetType;
use geoutils::Location;
use nmea::sentences::FixType;
use serde::{Deserialize, Serialize};

pub mod req;
pub mod resp;

/// Information about a GPS device
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct GpsInfo {
    /// The GPS device port (e.g. /dev/ttyACM0)
    pub port: String,
    /// The GPS device baud rate (e.g. 115200)
    pub baud: u32,
    /// The GPS device kind (e.g. generic, ublox, quectel)
    pub kind: GpsKind,
}

/// GPS device kind, used for vendor-specific parsing and configuration
#[derive(Clone, PartialEq, Debug, ValueEnum, Serialize, Deserialize, Default)]
pub enum GpsKind {
    /// Generic GPS device (default)
    #[default]
    Generic,
    /// Ublox GPS device
    Ublox,
    /// Quectel GPS device
    Quectel,
}

/// The current state of a GPS device, including fix type, location, speed, and other relevant information
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct GpsState {
    /// The current GPS fix type
    pub fix: FixType,
    /// The number of satellites currently in view
    pub num_satellites: u32,
    /// The current location of the GPS device, if available
    pub location: Option<Location>,
    /// The current altitude of the GPS device, if available
    pub altitude: Option<f64>,
    /// The current speed of the GPS device, if available
    pub speed: Option<f64>,
    /// The NTRIP mount point currently being used, if applicable
    // TODO: move this
    pub mount: Option<String>,
    /// Degree of Precision (DOP) values, if available
    pub dop: Dop,
}

/// Degree of Precision (DOP) values
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Dop {
    pub hdop: Option<f32>,
    pub vdop: Option<f32>,
    pub pdop: Option<f32>,
}

impl Default for GpsState {
    fn default() -> Self {
        GpsState {
            fix: FixType::Invalid,
            location: None,
            altitude: None,
            speed: None,
            mount: None,
            num_satellites: 0,
            dop: Default::default(),
        }
    }
}

impl std::fmt::Display for GpsState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fix: {:?}, {} satellites", self.fix, self.num_satellites)?;

        if let Some(loc) = self.location {
            write!(
                f,
                ", lat: {:.6}, lng: {:.6}",
                loc.latitude(),
                loc.longitude()
            )?;
        }

        if let Some(alt) = self.altitude {
            write!(f, ", alt: {:.2}m", alt)?;
        }

        if let Some(speed) = self.speed {
            write!(f, ", speed: {:.2}m/s", speed)?;
        }

        if let Some(mount) = &self.mount {
            write!(f, ", mount: {}", mount)?;
        }

        Ok(())
    }
}

#[derive(EnumSetType, Debug, Serialize, Deserialize, ValueEnum)]
pub enum SubscriptionFlags {
    /// Subscribe to GPS device state updates
    GpsState,
    /// Subscribe to GPS device satellite information updates
    GpsSatellites,
    /// Subscribe to NTRIP mount point updates
    NtripMounts,
    /// Subscribe to NMEA sentence updates
    NmeaSentences,
    /// Subscribe to RTCM3 sentence updates
    Rtcm3Sentences,
    /// Subscribe to UBX binary messages
    UbxMessages,
}

/// Load the default socket path depending on whether were running as a user
/// or a system daemon
pub fn default_sock_path() -> PathBuf {
    if let Some(home) = std::env::home_dir() {
        home.join(".rgpsd.sock")
    } else {
        PathBuf::from("/var/run/rgpsd.sock")
    }
}

/// Load the default TCP socket address for non-unix platforms
pub fn default_sock_addr() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 8666))
}

/// Load the default config path depending on whether were running as a user
/// or a system daemon
pub fn default_config_path() -> PathBuf {
    if let Some(home) = std::env::home_dir() {
        home.join(".config/rgpsd.toml")
    } else {
        PathBuf::from("/etc/rgpsd/rgpsd.toml")
    }
}
