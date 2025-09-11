use isocountry::CountryCode;

mod linz;

pub struct RtcmActor {

}


impl RtcmActor {
    pub async fn connect(provider: RtcmProvider, creds: RtcmCredentials, mount: impl ToString) -> Result<Self, anyhow::Error> {

        let url = format!("ntrip://{}:{}@{}/{}", creds.username, creds.password, provider.url(), mount.to_string());

        let raw_client = robust_ntrip_client::RobustNtripClient::new(
            &url,
            Default::default()
        ).await
        .map_err(|e| anyhow::anyhow!("Failed to create NTRIP client: {}", e))?;
        let mut ntrip = robust_ntrip_client::ParsingNtripClient::new(raw_client);

        Ok(RtcmActor {

        })
    }
}

pub enum RtcmProvider {
    Linz,
    Rtk2Go,
    Other{
        host: String,
        port: u16,
    }
}

pub struct RtcmCredentials {
    username: String,
    password: String,
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

impl RtcmCredentials {
    pub fn new(username: &str, password: &str) -> Self {
        RtcmCredentials {
            username: username.to_string(),
            password: password.to_string(),
        }
    }
}

impl RtcmProvider {
    pub fn url(&self) -> &str {
        match self {
            RtcmProvider::Linz => "caster.centipede.fr",
            RtcmProvider::Rtk2Go => "rtk2go.com",
            RtcmProvider::Other{ host, .. } => host,
        }
    }

    pub fn port(&self) -> u16 {
        match self {
            RtcmProvider::Linz => 2101,
            RtcmProvider::Rtk2Go => 2101,
            RtcmProvider::Other{ port, .. } => *port,
        }
    }
}
