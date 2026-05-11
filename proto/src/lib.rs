use geoutils::Location;
use nmea::sentences::FixType;
use serde::{Deserialize, Serialize};

/// GPSD protocol requests
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Req {
    GetState,
    GetSatellites,
}

/// GPSD protocol responses
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Resp {
    State(State),
    Satellites(Vec<nmea::Satellite>),
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct State {
    pub fix: FixType,
    pub num_satellites: u32,
    pub location: Option<Location>,
    pub altitude: Option<f64>,
    pub speed: Option<f64>,
    pub mount: Option<String>,
    pub dop: Dop,
}

/// Degree of Precision (DOP) values
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Dop {
    pub hdop: Option<f32>,
    pub vdop: Option<f32>,
    pub pdop: Option<f32>,
}

impl Default for State {
    fn default() -> Self {
        State {
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

impl std::fmt::Display for State {
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
