//! Configuration objects and parsing for RGPSD

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

use clap::Parser;
use serde::{Deserialize, Serialize};

use ntrip_client::{NtripConfig, NtripCredentials};

use rgps_core::{GpsKind};

#[cfg(target_family = "unix")]
use rgps_core::default_sock_path;

#[cfg(not(target_family = "unix"))]
use rgps_core::default_sock_addr;

/// GPS Daemon configuration options
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct GpsdConfig {
    /// General configuration
    pub general: General,

    /// GPS devices
    pub gps: Vec<Gps>,

    // NTRIP configuration options
    #[serde(default)]
    pub ntrip: Option<Ntrip>,
}

impl GpsdConfig {
    /// Load the configuration from a file path
    pub fn load(path: &Path) -> Result<Self, anyhow::Error> {
        // Read configuration file
        let config_str = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read config file {}: {}", path.display(), e))?;

        // Deserialize the config file
        let config: GpsdConfig = toml::from_str(&config_str).map_err(|e| {
            anyhow::anyhow!("Failed to parse config file {}: {}", path.display(), e)
        })?;

        Ok(config)
    }
}

/// General daemon configuration
#[derive(Clone, PartialEq, Debug, Parser, Serialize, Deserialize)]
pub struct General {
    /// Daemon control socket
    #[cfg(target_family = "unix")]
    #[clap(long, default_value = default_sock_path().into_os_string(), env = "RGPSD_CTL_SOCK")]
    #[serde(default = "default_sock_path")]
    pub ctl_sock: PathBuf,

    /// Daemon control socket (TCP for non-unix platforms)
    #[cfg(not(target_family = "unix"))]
    #[clap(long, default_value = "127.0.0.1:8666", env = "RGPSD_CTL_SOCK")]
    #[serde(default = "default_sock_addr")]
    pub ctl_sock: SocketAddr,

    /// Bind address for the optional (read only) HTTP server
    #[clap(long, default_value = "127.0.0.1:8667", env = "RGPSD_HTTP_ADDR")]
    pub http_addr: Option<SocketAddr>,
}

/// GPS device configuration
#[derive(Clone, PartialEq, Debug, Parser, Serialize, Deserialize)]
pub struct Gps {
    /// GPS device serial port
    #[clap(
        short = 'p',
        long,
        default_value = "/dev/ttyACM0",
        env = "GPSD_GPS_PORT"
    )]
    pub gps_port: PathBuf,

    /// GPS device baud rate
    #[clap(short = 'b', long, default_value = "115200", env = "GPSD_GPS_BAUD")]
    pub gps_baud: u32,

    /// GPS device kind
    #[clap(long, default_value = "generic", env = "GPSD_GPS_KIND")]
    #[serde(default)]
    pub gps_kind: GpsKind,
}

/// NTRIP server configuration
#[derive(Clone, PartialEq, Debug, Parser, Serialize, Deserialize)]
pub struct Ntrip {
    /// NTRIP host
    #[clap(long, default_value = "posau", env = "NTRIP_HOST")]
    #[serde(flatten)]
    pub host: NtripConfig,

    /// NTRIP credentials
    #[clap(flatten)]
    #[serde(flatten)]
    pub ntrip_creds: NtripCredentials,

    /// Distance threshold for remounting
    #[clap(long, default_value = "100000", env = "NTRIP_DISTANCE_THRESHOLD")]
    #[serde(default = "default_distance_threshold")]
    pub distance_threshold: f64,
}

fn default_distance_threshold() -> f64 {
    100_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_general() {
        let toml = r#"
            ctl_sock = "/tmp/rgpsd.sock"
        "#;
        let parsed: General = toml::from_str(toml).expect("Failed to parse General");
        assert_eq!(parsed.ctl_sock, PathBuf::from("/tmp/rgpsd.sock"));
    }

    #[test]
    fn test_parse_gps() {
        let toml = r#"
            gps_port = "/dev/ttyUSB0"
            gps_baud = 115200
        "#;
        let parsed: Gps = toml::from_str(toml).expect("Failed to parse Gps");
        assert_eq!(parsed.gps_port, PathBuf::from("/dev/ttyUSB0"));
        assert_eq!(parsed.gps_baud, 115200);
    }

    #[test]
    fn test_parse_ntrip() {
        let toml = r#"
            host = "rtk2go.com"
            port = 2101
            use_tls = false
            user = "test@example.com"
            pass = "secret"
        "#;
        let parsed: Ntrip = toml::from_str(toml).expect("Failed to parse Ntrip");
        assert_eq!(parsed.host.host, "rtk2go.com");
        assert_eq!(parsed.host.port, 2101);
        assert!(!parsed.host.use_tls);
        assert_eq!(parsed.ntrip_creds.user, "test@example.com");
        assert_eq!(parsed.ntrip_creds.pass, "secret");
    }

    #[test]
    fn test_parse_config_toml() {
        let config = include_str!("../rgpsd.toml");

        let parsed: GpsdConfig = toml::from_str(config).expect("Failed to parse config");

        assert_eq!(parsed.general.ctl_sock, PathBuf::from("/tmp/rgpsd.sock"));
        assert_eq!(parsed.gps[0].gps_port, PathBuf::from("/dev/ttyACM0"));
        assert_eq!(parsed.gps[0].gps_baud, 115200);
    }
}
