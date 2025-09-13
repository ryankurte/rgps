use std::str::FromStr;

use clap::Parser;
use isocountry::CountryCode;
use strum::{Display, EnumString, VariantNames};
use tracing::debug;

mod client;
pub use client::RtcmClient;

mod snip;
pub use snip::{Constellation, MountInfo, Network, Protocol, ServerInfo};

/// Credentials for an NTRIP (RTCM) service
#[derive(Clone, PartialEq, Debug, Parser)]
pub struct NtripConfig {
    #[clap(long = "ntrip-user", env = "NTRIP_USER")]
    user: String,

    #[clap(long = "ntrip-pass", env = "NTRIP_PASS", default_value = "")]
    pass: String,

    #[clap(long = "ntrip-host", env = "NTRIP_HOST", default_value = "rtk2go.com")]
    host: String,

    #[clap(long = "ntrip-port", env = "NTRIP_PORT", default_value_t = 2101)]
    port: u16,
}

impl Default for NtripConfig {
    fn default() -> Self {
        NtripConfig {
            user: "".to_string(),
            pass: "".to_string(),
            host: "rtk2go.com".to_string(),
            port: 2101,
        }
    }
}

pub struct RtcmMount {
    site_id: String,
    country: CountryCode,
}

impl ToString for RtcmMount {
    fn to_string(&self) -> String {
        format!("{:4}00{:3}0", self.site_id, self.country.alpha3())
    }
}

/// RTCM NTRIP Provider
#[derive(Clone, PartialEq, Debug, Parser, EnumString, Display, VariantNames)]
pub enum RtcmProvider {
    Linz,
    Rtk2Go,
}

impl RtcmProvider {
    pub fn url(&self) -> &str {
        match self {
            RtcmProvider::Linz => "positionz-rt.linz.govt.nz",
            RtcmProvider::Rtk2Go => "rtk2go.com",
        }
    }

    pub fn port(&self) -> u16 {
        match self {
            RtcmProvider::Linz => 2101,
            RtcmProvider::Rtk2Go => 2101,
        }
    }
}

#[cfg(test)]
mod tests {
    use hyper::{Method, Request, Uri};
    use rtcm_rs::MessageFrame;
    use tracing::{debug, info, level_filters::LevelFilter, trace};
    use tracing_subscriber::FmtSubscriber;

    use super::*;

    fn setup_logging() {
        let _ = FmtSubscriber::builder()
            .compact()
            .without_time()
            .with_max_level(LevelFilter::DEBUG)
            .try_init();
    }
}
