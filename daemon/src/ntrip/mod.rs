use clap::Parser;
use isocountry::CountryCode;
use strum::{Display, EnumString, VariantNames};

mod client;
pub use client::RtcmClient;

mod linz;
pub(crate) mod parser;

/// Credentials for an NTRIP (RTCM) service
#[derive(Clone, PartialEq, Debug, Parser)]
pub struct NtripConfig {
    #[clap(long = "ntrip-user", env = "NTRIP_USER")]
    user: String,

    #[clap(long = "ntrip-pass", env = "NTRIP_PASS", default_value = "")]
    pass: String,

    #[clap(
        long = "ntrip-host",
        env = "NTRIP_HOST",
        default_value = "positionz-rt.linz.govt.nz"
    )]
    host: String,

    #[clap(long = "ntrip-port", env = "NTRIP_PORT", default_value_t = 2101)]
    port: u16,
}

impl Default for NtripConfig {
    fn default() -> Self {
        NtripConfig {
            user: "".to_string(),
            pass: "".to_string(),
            host: "positionz-rt.linz.govt.nz".to_string(),
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
    use hyper_util::rt::TokioExecutor;
    use tracing::{debug, info, level_filters::LevelFilter};
    use tracing_subscriber::FmtSubscriber;

    use super::*;

    #[tokio::test]
    async fn test_ntrip_rtk2go() {
        let _ = FmtSubscriber::builder()
            .compact()
            .without_time()
            .with_max_level(LevelFilter::TRACE)
            .try_init();

        let client = reqwest::Client::builder()
            .http1_ignore_invalid_headers_in_responses(true)
            .http09_responses()
            .user_agent(format!("NTRIP {}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap();

        let req = client.request(Method::GET, "http://rtk2go.com:2101")
            .header("Ntrip-Version", "Ntrip/2.0")
            .build().unwrap();

        let res = client.execute(req).await.expect("Fetch failed");

        info!("Fetched NTRIP response: {:?}", res.status());

        assert!(res.status().is_success());

        debug!("Response: {:?}", res.text().await.unwrap());

    }
}
