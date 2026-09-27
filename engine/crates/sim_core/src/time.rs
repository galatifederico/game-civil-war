use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

/// Simulation clock. One tick is one in-game hour by default (`time.ticks_per_day`).
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimClock {
    pub tick: u64,
}

impl SimClock {
    pub fn day(&self, ticks_per_day: u64) -> u64 {
        self.tick / ticks_per_day.max(1)
    }

    pub fn hour_of_day(&self, ticks_per_day: u64) -> u64 {
        self.tick % ticks_per_day.max(1)
    }
}
