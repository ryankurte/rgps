use serde::{Deserialize, Serialize};

/// GPSD protocol requests
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Req {
    Ping,
}

/// GPSD protocol responses
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Resp {
    Pong,
    LocationUpdate(LocationUpdate),
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct LocationUpdate {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
    pub speed: Option<f64>,
    pub lock: String,
}
