//! Configuration objects and parsing for RGPSD

use std::path::{Path, PathBuf};

#[cfg(not(target_family = "unix"))]
use std::net::SocketAddr;

use clap::Parser;
use serde::{Deserialize, Serialize};

use ntrip_client::{NtripConfig, NtripCredentials};

/// GPS Daemon configuration options
#[derive(Clone, PartialEq, Debug, Parser, Serialize, Deserialize)]
pub struct GpsdConfig {
    /// Daemon control socket
    #[clap(flatten)]
    pub general: General,

    /// GPS device configuration
    #[clap(flatten)]
    pub gps: Gps,

    // NTRIP configuration options
    #[clap(flatten)]
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

#[derive(Clone, PartialEq, Debug, Parser, Serialize, Deserialize)]
pub struct General {
    /// Daemon control socket
    #[cfg(target_family = "unix")]
    #[clap(long, default_value = default_sock_path().into_os_string(), env = "RGPSD_CTL_SOCK")]
    pub ctl_sock: PathBuf,

    #[cfg(not(target_family = "unix"))]
    #[clap(long, default_value = "127.0.0.1:8080", env = "RGPSD_CTL_SOCK")]
    pub ctl_sock: SocketAddr,
}

/// Load the default socket path depending on whether were running as a user
/// or a system daemon
pub fn default_sock_path() -> PathBuf {
    if let Some(home) = std::env::home_dir() {
        home.join(".rgpsd.sock")
    } else {
        PathBuf::from("/tmp/rgpsd.sock")
    }
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
}

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
        assert_eq!(parsed.gps.gps_port, PathBuf::from("/dev/ttyACM0"));
        assert_eq!(parsed.gps.gps_baud, 115200);
    }
}
