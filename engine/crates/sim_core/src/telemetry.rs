//! Telemetry: gauges, counters and histograms through the `metrics` facade (exported by the server
//! feature as Prometheus text on `/metrics`).

use bevy_ecs::prelude::*;

use crate::events::EventLog;
use crate::factions::Factions;
use crate::market::Market;
use crate::stats::{Dead, Pawn};
use crate::time::SimClock;

#[derive(Resource, Default, serde::Serialize, serde::Deserialize)]
pub struct TelemetryCursor(pub u64);

pub fn record_metrics(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    metrics::gauge!("sim_tick").set(tick as f64);
    let (mut alive, mut dead) = (0u64, 0u64);
    let mut q = world.query_filtered::<Option<&Dead>, With<Pawn>>();
    for d in q.iter(world) {
        if d.is_some() { dead += 1 } else { alive += 1 }
    }
    metrics::gauge!("sim_pawns", "state" => "alive").set(alive as f64);
    metrics::gauge!("sim_pawns", "state" => "dead").set(dead as f64);
    for (f, s) in &world.resource::<Factions>().states {
        metrics::gauge!("sim_treasury", "faction" => f.clone()).set(s.treasury);
        metrics::gauge!("sim_victory_points", "faction" => f.clone()).set(s.victory_points as f64);
    }
    for (i, m) in &world.resource::<Market>().items {
        metrics::gauge!("sim_price", "item" => i.clone()).set(m.price);
    }
    let cursor = world.resource::<TelemetryCursor>().0;
    let log = world.resource::<EventLog>();
    let mut last = cursor;
    for e in log.since(cursor) {
        metrics::counter!("sim_events_total", "kind" => e.kind.clone()).increment(1);
        last = e.id;
    }
    world.resource_mut::<TelemetryCursor>().0 = last;
}
