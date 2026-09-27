//! Debug helper: who did it? Prints events between two ticks with actor names.
use sim_core::prelude::*;
fn main() {
    let mut a = std::env::args().skip(1);
    let (from, to): (u64, u64) = (a.next().unwrap().parse().unwrap(), a.next().unwrap().parse().unwrap());
    let filter = a.next().unwrap_or_default();
    let mut sim = fidenza_world::build(1).unwrap();
    sim.run(to + 1);
    let snap = sim.snapshot(true);
    let name = |id: Option<SimId>| id.and_then(|i| snap.entities.iter().find(|e| e.id == i)).map_or("-".to_string(), |e| format!("{} [{}]", e.name, e.faction.clone().unwrap_or_default()));
    for e in sim.events().all().iter().filter(|e| e.tick >= from && e.tick <= to && e.message.contains(&filter)) {
        println!("[{}] {} | attore: {} | {}", e.tick, e.kind, name(e.actor), e.message);
    }
}
