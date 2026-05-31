use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{GpsInfo, GpsState};

/// RGPS protocol responses
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resp {
    Ok,
    /// Information about connected GPS devices, keyed by device ID
    Info(HashMap<u32, GpsInfo>),
    /// The current state of connected GPS devices, keyed by device ID
    State(HashMap<u32, GpsState>),
    /// The current satellite information for connected GPS devices, keyed by device ID
    // TODO: wrap this in our own satellite info type?
    Satellites(HashMap<u32, Vec<nmea::Satellite>>),
    /// An NMEA sentence update, including the device ID and raw sentence
    Nmea(u32, String),
    /// An RTCM3 message update, including the device ID and raw message bytes
    Rtcm3(u32, Vec<u8>),
    /// A UBX binary message update, including the device ID and raw message bytes
    Ubx(u32, Vec<u8>),
}

impl TryFrom<Resp> for HashMap<u32, GpsInfo> {
    type Error = String;

    fn try_from(value: Resp) -> Result<Self, Self::Error> {
        match value {
            Resp::Info(info) => Ok(info),
            other => Err(format!("Expected Resp::Info, got {:?}", other)),
        }
    }
}

impl TryFrom<Resp> for HashMap<u32, GpsState> {
    type Error = String;

    fn try_from(value: Resp) -> Result<Self, Self::Error> {
        match value {
            Resp::State(state) => Ok(state),
            other => Err(format!("Expected Resp::State, got {:?}", other)),
        }
    }
}

impl TryFrom<Resp> for HashMap<u32, Vec<nmea::Satellite>> {
    type Error = String;

    fn try_from(value: Resp) -> Result<Self, Self::Error> {
        match value {
            Resp::Satellites(sats) => Ok(sats),
            other => Err(format!("Expected Resp::Satellites, got {:?}", other)),
        }
    }
}

impl TryFrom<Resp> for () {
    type Error = String;

    fn try_from(value: Resp) -> Result<Self, Self::Error> {
        match value {
            Resp::Ok => Ok(()),
            other => Err(format!("Expected Resp::Ok, got {:?}", other)),
        }
    }
}
