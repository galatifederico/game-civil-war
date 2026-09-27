//! End-to-end tests of the engine with a small, setting-free content pack.

use sim_core::content::{Effect, Scope};
use sim_core::prelude::*;

fn town(seed: u64) -> Simulation {
    let mut b = Simulation::builder(seed);
    b.load_pack_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/town")).expect("pack loads");
    b.build().expect("sim builds")
}

fn count(sim: &Simulation, k: &str) -> usize {
    sim.events().of_kind(k).count()
}

#[test]
fn crime_arrest_bribe_article_price() {
    let mut sim = town(42);
    let (thief_id, thief) = sim.find_template("thief").unwrap();
    let mut arrested_at = None;
    for _ in 0..100 {
        sim.tick();
        if arrested_at.is_none() && sim.world.get::<Detained>(thief).is_some() {
            arrested_at = Some(sim.tick_count());
            // The leader pays exactly what the police asks.
            sim.enqueue(SimCommand::Bribe { faction: "town".into(), target: thief_id, amount: None });
        }
    }
    let log: Vec<String> = sim.events().all().iter().filter(|e| e.kind != kind::PURCHASE).map(|e| format!("[{}] {} {}", e.tick, e.kind, e.message)).collect();
    let dump = log.join("\n");
    assert!(count(&sim, kind::CRIME) >= 1, "no crime\n{dump}");
    assert!(count(&sim, kind::CRIME_REPORTED) >= 1, "crime not reported\n{dump}");
    assert!(count(&sim, kind::ARREST) >= 1, "no arrest\n{dump}");
    assert!(count(&sim, kind::SEIZURE) >= 1, "contraband not seized\n{dump}");
    assert!(count(&sim, kind::BRIBE) >= 1, "no bribe\n{dump}");
    assert!(count(&sim, kind::ARTICLE) >= 1, "no article\n{dump}");
    assert!(sim.events().of_kind(kind::PRICE_CHANGE).any(|e| e.message.contains("Bread")), "bread price did not move\n{dump}");
    let w = sim.world.get::<Wanted>(thief).unwrap();
    let bribe_tick = sim.events().of_kind(kind::BRIBE).next().unwrap().tick;
    let rearrested = sim.events().of_kind(kind::ARREST).any(|e| e.tick > bribe_tick);
    assert!(rearrested || w.charges.iter().all(|c| c.tick > bribe_tick), "charges not cleared by the bribe");
    assert!(sim.world.resource::<Factions>().treasury("town") < 500.0);
}

#[test]
fn same_seed_same_world() {
    let mut a = town(7);
    let mut b = town(7);
    a.run(60);
    b.run(60);
    assert_eq!(a.state_hash(), b.state_hash());
    let mut c = town(8);
    c.run(60);
    // Different seeds normally diverge (not guaranteed, but with this pack they do).
    assert_ne!(a.state_hash(), c.state_hash());
}

#[test]
fn escalation_chain_and_contagion() {
    let mut sim = town(1);
    let (_, victim) = sim.find_template("victim").unwrap();
    let (vid, _) = sim.find_template("victim").unwrap();
    for _ in 0..3 {
        sim.execute(SimCommand::ApplyEffect { subject: Some(vid), target: None, effect: Effect::ApplyStatus { status: "tipsy".into(), severity: 1.0 } })
            .unwrap();
    }
    let st = sim.world.get::<sim_core::status::StatusEffects>(victim).unwrap();
    assert!(st.has("wasted") && !st.has("tipsy"), "tipsy should escalate to wasted: {:?}", st.active.keys());

    // Flu spreads by contact to everyone standing next to the carrier.
    let (cop_id, cop) = sim.find_template("cop").unwrap();
    let pos = *sim.world.get::<Position>(victim).unwrap();
    sim.world.entity_mut(cop).insert(pos);
    sim.execute(SimCommand::ApplyEffect { subject: Some(cop_id), target: None, effect: Effect::ApplyStatus { status: "flu".into(), severity: 1.0 } }).unwrap();
    sim.world.get_mut::<sim_core::ai::Brain>(cop).unwrap().actions.clear();
    sim.world.get_mut::<sim_core::ai::Brain>(victim).unwrap().actions.clear();
    sim.run(2);
    assert!(sim.world.get::<sim_core::status::StatusEffects>(victim).unwrap().has("flu"));
}

#[test]
fn structural_damage_triggers_global_modifier() {
    let mut sim = town(3);
    let wh = sim.snapshot(true).entities.iter().find(|e| e.building.as_ref().is_some_and(|b| b.def == "warehouse")).unwrap().id;
    sim.execute(SimCommand::ApplyEffect { subject: Some(wh), target: None, effect: Effect::DamageBuilding(30.0) }).unwrap();
    sim.run(3);
    assert!(sim.world.resource::<GlobalModifiers>().logistics_disruption() > 0.0);
    assert!(sim.world.resource::<Market>().disruption > 0.0);
}

#[test]
fn exposure_reveals_disguise() {
    let mut sim = town(5);
    let spy = sim.execute(SimCommand::Spawn { template: "spy".into(), zone: Some("square".into()), count: 1, faction: None, name: None }).unwrap();
    assert!(spy.contains('['));
    let snap = sim.snapshot(false);
    let fake = snap.entities.iter().find(|e| e.name == "Officer Smith").expect("disguised name visible");
    assert_eq!(fake.faction.as_deref(), Some("police"));
    assert_eq!(fake.race.as_deref(), Some("human"));
    let id = fake.id;
    sim.execute(SimCommand::ApplyEffect { subject: Some(id), target: None, effect: Effect::ModCover(-50.0) }).unwrap();
    assert_eq!(count(&sim, kind::EXPOSED), 1);
    let snap = sim.snapshot(false);
    let real = snap.entities.iter().find(|e| e.id == id).unwrap();
    assert_eq!(real.race.as_deref(), Some("lizard"));
    assert_eq!(real.faction.as_deref(), Some("town"));
}

#[test]
fn scoped_effects_and_mutilation() {
    let mut sim = town(9);
    let (vid, victim) = sim.find_template("victim").unwrap();
    sim.execute(SimCommand::ApplyEffect {
        subject: Some(vid),
        target: None,
        effect: Effect::On(Scope::Subject, Box::new(Effect::Damage { amount: 50.0, part: Some("arm_l".into()) })),
    })
    .unwrap();
    assert_eq!(count(&sim, kind::MUTILATION), 1);
    let stats = sim.world.get::<Stats>(victim).unwrap();
    assert!(stats.get("heroism") > 0.0);
    let body = sim.world.get::<sim_core::anatomy::Body>(victim).unwrap();
    assert!(body.capacity("manipulation") < 0.6);
}

#[test]
#[ignore]
fn print_town_log() {
    let mut sim = town(42);
    let (thief_id, thief) = sim.find_template("thief").unwrap();
    let mut done = false;
    for _ in 0..100 {
        sim.tick();
        if !done && sim.world.get::<Detained>(thief).is_some() {
            done = true;
            sim.enqueue(SimCommand::Bribe { faction: "town".into(), target: thief_id, amount: None });
        }
    }
    for e in sim.events().all() {
        if e.kind != kind::PURCHASE && e.kind != kind::SPAWN {
            println!("[{:>3}] {:<16} {}", e.tick, e.kind, e.message);
        }
    }
}
