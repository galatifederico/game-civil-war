//! Where are the pawns? `cargo run --release -p fidenza_world --example maps -- <days>`
use std::collections::BTreeMap;

use sim_core::prelude::*;

fn main() {
    let days: u64 = std::env::args().nth(1).and_then(|v| v.parse().ok()).unwrap_or(3);
    let mut sim = fidenza_world::build(1).unwrap();
    let map = sim.world.resource::<WorldMap>().clone();
    for d in 1..=days {
        sim.run(24);
        let snap = sim.snapshot(true);
        let mut per: BTreeMap<String, (usize, f32)> = BTreeMap::new();
        for e in snap.entities.iter().filter(|e| e.kind == "pawn" && !e.dead) {
            if let Some(p) = e.pos {
                let x = per.entry(map.layers[p.layer as usize].id.clone()).or_default();
                x.0 += 1;
                x.1 += *e.needs.get("fame").unwrap_or(&1.0);
            }
        }
        let per: BTreeMap<String, String> = per.into_iter().map(|(k, (n, f))| (k, format!("{n} (fame {:.2})", f / n as f32))).collect();
        let deliveries = sim.events().all().iter().filter(|e| e.message.contains("consegna")).count();
        let dug = sim.world.resource::<sim_core::map::TerrainChanges>().0.len();
        let stone: u32 = snap.entities.iter().filter_map(|e| e.building.as_ref()).filter(|b| b.def == "deposito").map(|b| b.stock.values().sum::<u32>()).sum();
        println!("giorno {d}: {dug} celle scavate, {stone} materiali nei depositi, {} consegne totali · {:?}", deliveries, per);
    }
}
