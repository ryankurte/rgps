use std::str::FromStr;

use clap::Parser;
use isocountry::CountryCode;
use strum::{Display, EnumString, VariantNames};
use tracing::debug;

mod client;
pub use client::NtripClient;

mod snip;
pub use snip::{Constellation, MountInfo, Network, Protocol, ServerInfo};

/// NTRIP (Networked Transport of RTCM via Internet Protocol) configuration
#[derive(Clone, PartialEq, Debug, Parser)]
pub struct NtripConfig {
    /// Host name or IP address of the NTRIP server
    #[clap(long = "ntrip-host", env = "NTRIP_HOST", default_value = "rtk2go.com")]
    pub host: String,

    /// Port number of the NTRIP server
    #[clap(long = "ntrip-port", env = "NTRIP_PORT", default_value_t = 2101)]
    pub port: u16,

    /// Use TLS / SSL for the NTRIP connection
    #[clap(long = "ntrip-use-tls", env = "NTRIP_USE_TLS", default_value_t = false)]
    pub use_tls: bool,
}

/// Credentials for an NTRIP (RTCM) service
#[derive(Clone, PartialEq, Debug, Parser)]
pub struct NtripCredentials {
    /// Username for the NTRIP service
    #[clap(long = "ntrip-user", env = "NTRIP_USER")]
    user: String,

    /// Password for the NTRIP service
    #[clap(long = "ntrip-pass", env = "NTRIP_PASS", default_value = "")]
    pass: String,
}

impl Default for NtripConfig {
    fn default() -> Self {
        NtripConfig {
            host: "rtk2go.com".to_string(),
            port: 2101,
            use_tls: false,
        }
    }
}

/// Parse an [NtripConfig] from a URL string
///
/// For example:
/// ```
/// # use gpsrs_daemon::ntrip::NtripConfig;
///
/// let cfg = "ntrip://rtk2go.com:2101".parse::<NtripConfig>().unwrap();
///
/// assert_eq!(cfg.host, "rtk2go.com");
/// assert_eq!(cfg.port, 2101);
/// assert_eq!(cfg.use_tls, false);
/// ```
///
/// This also matches on [RtcmProvider]'s for convenience.
/// ```
/// # use gpsrs_daemon::ntrip::NtripConfig;
///
/// let cfg = "linz".parse::<NtripConfig>().unwrap();
///
/// assert_eq!(cfg.host, "positionz-rt.linz.govt.nz");
/// assert_eq!(cfg.port, 2101);
/// assert_eq!(cfg.use_tls, false);
/// ```
impl FromStr for NtripConfig {
    type Err = anyhow::Error;

    /// Parse an [NtripConfig] from a URL string
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Match on known providers
        if let Ok(provider) = RtcmProvider::from_str(s) {
            return Ok(NtripConfig {
                host: provider.host().to_string(),
                port: provider.port(),
                use_tls: provider.use_tls(),
            });
        }

        // Strip protocol if present
        let proto = if s.starts_with("http://") {
            "http"
        } else if s.starts_with("https://") {
            "https"
        } else if s.starts_with("ntrip://") {
            "ntrip"
        } else {
            "unknown"
        };
        let s = s.trim_start_matches(&format!("{proto}://"));

        // Split host and port
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() < 1 {
            return Err(anyhow::anyhow!("Invalid NTRIP URL".to_string()));
        }
        let host = parts[0].to_string();

        // Parse port or use default
        let port = if parts.len() > 1 {
            parts[1]
                .parse::<u16>()
                .map_err(|_| anyhow::anyhow!("Invalid port number".to_string()))?
        } else if proto == "https" {
            443
        } else {
            2101
        };
        Ok(NtripConfig {
            host,
            port,
            use_tls: port == 443,
        })
    }
}

impl NtripConfig {
    pub fn url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Defines an RTCM mount point, with serialisation to/from mount strings
#[derive(Clone, PartialEq, Debug)]
pub struct RtcmMount {
    site_id: String,
    country: CountryCode,
}

/// Generate a mount string from an [RtcmMount] object
impl ToString for RtcmMount {
    fn to_string(&self) -> String {
        format!("{:4}00{:3}0", self.site_id, self.country.alpha3())
    }
}

/// Parse an [RtcmMount] object from a mount string
impl FromStr for RtcmMount {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 7 {
            return Err(anyhow::anyhow!("Invalid mount string".to_string()));
        }
        let site_id = s[0..4].trim().to_string();
        let country = CountryCode::for_alpha3(&s[5..8])
            .map_err(|_| anyhow::anyhow!("Invalid country code".to_string()))?;
        Ok(RtcmMount { site_id, country })
    }
}

/// Common RTCM data providers
#[derive(Clone, PartialEq, Debug, Parser, EnumString, Display, VariantNames)]
pub enum RtcmProvider {
    /// Land Information New Zealand
    ///
    /// Note: requires credentials
    #[strum(serialize = "linz")]
    Linz,
    /// RTK2GO.com free service
    #[strum(serialize = "rtk2go")]
    Rtk2Go,
    /// Positioning Australia
    ///
    /// Note: requires credentials and TLS
    #[strum(serialize = "posau")]
    PosAu,
}

impl RtcmProvider {
    pub fn host(&self) -> &str {
        match self {
            RtcmProvider::Linz => "positionz-rt.linz.govt.nz",
            RtcmProvider::Rtk2Go => "rtk2go.com",
            RtcmProvider::PosAu => "ntrip.data.gnss.ga.gov.au",
        }
    }

    pub fn port(&self) -> u16 {
        match self {
            RtcmProvider::Linz => 2101,
            RtcmProvider::Rtk2Go => 2101,
            RtcmProvider::PosAu => 443,
        }
    }

    pub fn use_tls(&self) -> bool {
        match self {
            RtcmProvider::Linz => false,
            RtcmProvider::Rtk2Go => false,
            RtcmProvider::PosAu => true,
        }
    }
}
