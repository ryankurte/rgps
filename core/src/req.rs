use serde::{Deserialize, Serialize};

use crate::SubscriptionFlags;

/// RGPS protocol requests
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Req {
    /// Fetch information about the configured GPS devices
    GetInfo,
    /// Get the current state of connected GPS devices
    GetState,
    /// Fetch the current satellite information for connected GPS devices
    GetSatellites,
    /// Subscribe to updates for specific message types
    Subscribe(Vec<SubscriptionFlags>),
}

impl From<Vec<SubscriptionFlags>> for Req {
    fn from(flags: Vec<SubscriptionFlags>) -> Self {
        Req::Subscribe(flags)
    }
}
