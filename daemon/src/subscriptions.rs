use std::collections::HashMap;

use super::SubscriptionId;

use rgps_core::SubscriptionFlags;

/// Subscription management
#[derive(Debug, Default)]
pub struct Subscriptions {
    subs: HashMap<SubscriptionId, Vec<SubscriptionFlags>>,
}

impl Subscriptions {
    /// Subscribe a client to a set of subscription flags (this will replace any existing subscriptions)
    pub fn subscribe(&mut self, id: SubscriptionId, flags: Vec<SubscriptionFlags>) {
        self.subs.insert(id, flags);
    }

    /// Unsubscribe a client from all subscription flags
    pub fn remove(&mut self, id: &SubscriptionId) {
        self.subs.remove(id);
    }

    /// Fetch an iterator over subscribers to a specific subscription flag
    pub fn get_subscribers(
        &self,
        flag: SubscriptionFlags,
    ) -> impl Iterator<Item = &SubscriptionId> {
        self.subs.iter().filter_map(move |(id, flags)| {
            if flags.contains(&flag) {
                Some(id)
            } else {
                None
            }
        })
    }
}
