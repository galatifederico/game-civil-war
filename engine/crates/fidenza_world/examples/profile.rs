//! Times every system of a tick on the Fidenza world: `cargo run --release -p fidenza_world --example profile`.

use std::time::{Duration, Instant};

use sim_core::bevy_ecs::system::RunSystemOnce;
use sim_core::prelude::*;

fn main() {
    let mut sim = fidenza_world::build(1).unwrap();
    sim.run(20);
    let w = &mut sim.world;
    type Step = (&'static str, fn(&mut World));
    let steps: Vec<Step> = vec![
        ("commands", sim_core::commands::apply_commands),
        ("recompute_stats", |w| { w.run_system_once(sim_core::stats::recompute_stats).unwrap(); }),
        ("decay_needs", |w| { w.run_system_once(sim_core::stats::decay_needs).unwrap(); }),
        ("statuses", sim_core::status::tick_statuses),
        ("healing", |w| { w.run_system_once(sim_core::anatomy::natural_healing).unwrap(); }),
        ("hygiene", sim_core::hygiene::hygiene_tick),
        ("tethers", sim_core::dungeon::tethers),
        ("events", sim_core::dungeon::events),
        ("ai_think", sim_core::ai::think),
        ("run_jobs", sim_core::jobs::run_jobs),
        ("crime", sim_core::crime::crime_upkeep),
        ("buildings", sim_core::buildings::buildings_tick),
        ("market", sim_core::market::update_market),
        ("payroll", sim_core::economy::payroll),
        ("defections", sim_core::social::defections),
        ("merges", sim_core::social::merges),
        ("succession", sim_core::social::succession),
        ("collections", sim_core::victory::collections),
        ("victory", sim_core::victory::check_victory),
        ("scoops", sim_core::press::gather_scoops),
        ("activity", sim_core::snapshot::update_activity),
        ("telemetry", sim_core::telemetry::record_metrics),
    ];
    let mut totals = vec![Duration::ZERO; steps.len()];
    let n = 50;
    for _ in 0..n {
        for (i, (_, f)) in steps.iter().enumerate() {
            let t = Instant::now();
            f(w);
            totals[i] += t.elapsed();
        }
        w.resource_mut::<SimClock>().tick += 1;
    }
    let all: Duration = totals.iter().sum();
    println!("{} tick, {:.2} ms/tick", n, all.as_secs_f64() * 1000.0 / n as f64);
    let mut rows: Vec<_> = steps.iter().zip(&totals).map(|((name, _), d)| (*name, *d)).collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1));
    for (name, d) in rows.iter().take(10) {
        println!("  {name:<16} {:>8.3} ms/tick  ({:.0}%)", d.as_secs_f64() * 1000.0 / n as f64, d.as_secs_f64() / all.as_secs_f64() * 100.0);
    }
}
