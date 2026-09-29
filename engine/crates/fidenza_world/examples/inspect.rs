//! Debug helper: runs N ticks and prints the job board and the AI of one template.
//! `cargo run --release -p fidenza_world --example inspect -- <ticks> <template>`

use sim_core::prelude::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let ticks: u64 = args.next().and_then(|v| v.parse().ok()).unwrap_or(50);
    let template = args.next().unwrap_or_else(|| "elfo_logistica".into());
    let mut sim = fidenza_world::build(1).unwrap();
    sim.run(ticks);
    let board = sim.world.resource::<JobBoard>().clone();
    println!("── bacheca ({} job) ──", board.jobs.len());
    for j in board.jobs.values().take(30) {
        println!("  #{} {} fazione={:?} target={:?} riservato={:?} {:?}", j.id, j.job, j.faction, j.target, j.reserved_by, j.payload);
    }
    let found = match template.parse::<u64>() {
        Ok(id) => sim.entity(SimId(id)).map(|e| (SimId(id), e)),
        Err(_) => sim.find_template(&template),
    };
    if let Some((_, e)) = found {
        if let Some(v) = sim_core::snapshot::entity_view(&sim.world, e, true) {
            println!("id {} pos {:?} inventario: {:?} bisogni: {:?}", v.id, v.pos, v.inventory, v.needs);
        }
        println!("── AI di {template} ──\n{}", serde_json::to_string_pretty(&sim_core::snapshot::ai_inspect(&sim.world, e)).unwrap());
    }
}
