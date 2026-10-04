//! What pawns spend their time on: `cargo run --release -p fidenza_world --example azioni -- [days] [seed]`.
//! Counts, every tick, the job (or label) of each living pawn; then the classes and the roles at the end.

use std::collections::BTreeMap;

use sim_core::prelude::*;

fn main() {
    let mut a = std::env::args().skip(1);
    let days: u64 = a.next().and_then(|v| v.parse().ok()).unwrap_or(10);
    let seed: u64 = a.next().and_then(|v| v.parse().ok()).unwrap_or(1);
    let mut sim = fidenza_world::build(seed).unwrap();
    let mut doing: BTreeMap<String, u64> = BTreeMap::new();
    for _ in 0..days * 24 {
        sim.tick();
        let snap = sim.snapshot(true);
        for e in snap.entities.iter().filter(|e| e.kind == "pawn" && !e.dead) {
            let what = e.activity.as_ref().map_or("-".to_string(), |a| a.job.clone().unwrap_or_else(|| a.label.clone()));
            *doing.entry(what).or_default() += 1;
        }
    }
    let total: u64 = doing.values().sum();
    let mut v: Vec<_> = doing.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    println!("── Cosa fanno ({days} giorni, seed {seed}) ──");
    for (k, n) in v.iter().take(60) {
        println!("{:>6.2}%  {k}", *n as f64 * 100.0 / total as f64);
    }
    let snap = sim.snapshot(true);
    let mut classes: BTreeMap<String, u32> = BTreeMap::new();
    for e in snap.entities.iter().filter(|e| e.kind == "pawn" && !e.dead) {
        for c in &e.classes {
            *classes.entry(c.clone()).or_default() += 1;
        }
    }
    println!("── Classi ──");
    for (k, n) in &classes {
        println!("{n:>4}  {k}");
    }
    println!("── Ruoli ──");
    for (t, h) in &snap.titles {
        let who = h.and_then(|id| snap.entities.iter().find(|e| e.id == id)).map_or("vacante".to_string(), |e| e.name.clone());
        println!("{t}: {who}");
    }
}
