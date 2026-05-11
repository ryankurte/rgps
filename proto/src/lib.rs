use geoutils::Location;
use nmea::sentences::FixType;
use serde::{Deserialize, Serialize};

/// GPSD protocol requests
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Req {
    GetState,
}

/// GPSD protocol responses
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Resp {
    Pong,
    State(State),
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct State {
    pub fix: FixType,
    pub location: Option<Location>,
    pub altitude: Option<f64>,
    pub speed: Option<f64>,
    pub mount: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        State {
            fix: FixType::Invalid,
            location: None,
            altitude: None,
            speed: None,
            mount: None,
        }
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fix: {:?}", self.fix)?;

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
