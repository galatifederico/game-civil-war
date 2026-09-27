//! Integration tests of the Fidenza world on top of sim_core.

use sim_core::content::Effect;
use sim_core::prelude::*;

fn sim(seed: u64) -> Simulation {
    fidenza_world::build(seed).expect("Fidenza builds")
}

fn id_of(sim: &mut Simulation, template: &str) -> (SimId, Entity) {
    sim.find_template(template).unwrap_or_else(|| panic!("{template} not found"))
}

#[test]
fn demo_chain_crime_arrest_bribe_scoop_price() {
    for seed in [1, 2, 3] {
        let mut sim = sim(seed);
        let (thief_id, thief) = id_of(&mut sim, "scippatore");
        let mut bribed = false;
        for _ in 0..100 {
            sim.tick();
            if !bribed && sim.world.get::<Detained>(thief).is_some() {
                bribed = true;
                sim.enqueue(SimCommand::Bribe { faction: "anarchici_commercio".into(), target: thief_id, amount: None });
            }
        }
        let ev = sim.events().all();
        let by_thief = |k: &str| ev.iter().any(|e| e.kind == k && e.actor == Some(thief_id));
        let on_thief = |k: &str| ev.iter().find(|e| e.kind == k && e.target == Some(thief_id));
        assert!(by_thief(kind::CRIME) && by_thief(kind::CRIME_REPORTED), "seed {seed}: no crime");
        assert!(on_thief(kind::SEARCH).is_some() && on_thief(kind::ARREST).is_some(), "seed {seed}: no search/arrest");
        let bribe = on_thief(kind::BRIBE).unwrap_or_else(|| panic!("seed {seed}: no bribe"));
        let feed = sim.world.resource::<Feed>();
        assert_eq!(feed.name, "Il Piccione Viaggiatore");
        let story: Vec<u64> = ev.iter().filter(|e| e.actor == Some(thief_id) || e.target == Some(thief_id)).map(|e| e.id).collect();
        let scoop = feed.articles.iter().find(|a| a.source_event.is_some_and(|s| story.contains(&s))).expect("scoop published");
        assert!(scoop.author_name != "Anonimo");
        assert!(ev.iter().any(|e| e.kind == kind::PRICE_CHANGE && e.tick >= scoop.tick), "seed {seed}: no price change after the scoop");
        // Charges were cleared by the bribe (later crimes may add new ones).
        let w = sim.world.get::<Wanted>(thief).unwrap();
        assert!(w.charges.iter().all(|c| c.tick > bribe.tick), "seed {seed}: old charges survived the bribe");
    }
}

#[test]
fn deterministic_replay() {
    let mut a = sim(11);
    let mut b = sim(11);
    a.run(50);
    b.run(50);
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn succession_to_the_drunkards_throne() {
    let mut sim = sim(5);
    let (king, _) = id_of(&mut sim, "re_anolino");
    let (leader, _) = id_of(&mut sim, "leader_anarchico");
    // Keep the ubriaconi away from the throne so the player's leader gets there first.
    sim.execute(SimCommand::SetParam { key: "ai.think_interval".into(), value: 1.0 }).unwrap();
    sim.execute(SimCommand::ApplyEffect { subject: Some(king), target: None, effect: Effect::Kill }).unwrap();
    assert!(sim.events().of_kind(kind::SUCCESSION_OPEN).count() == 1);
    sim.execute(SimCommand::Order { entity: leader, job: "muoviti".into(), target: None, zone: Some("trono_ubriaconi".into()) }).unwrap();
    for _ in 0..60 {
        sim.tick();
        if sim.world.resource::<Titles>().holder("trono_degli_ubriaconi").is_some() {
            break;
        }
    }
    let holder = sim.world.resource::<Titles>().holder("trono_degli_ubriaconi");
    assert!(holder.is_some(), "nobody claimed the throne");
    if holder == Some(leader) {
        let f = sim.world.resource::<Factions>();
        assert_eq!(f.states["ubriaconi"].controlled_by.as_deref(), Some("giocatore"));
        assert!(f.states["anarchici_commercio"].victory_points >= 400);
        sim.tick();
        let w = sim.world.resource::<Progress>().winner.clone().expect("political victory");
        assert_eq!(w.victory, "trono");
    }
}

#[test]
fn porcine_mutation_and_vaccine() {
    let mut sim = sim(8);
    let (a, ae) = id_of(&mut sim, "commerciante_ricco");
    let (b, be) = id_of(&mut sim, "fornaio");
    // b is vaccinated first.
    sim.execute(SimCommand::ApplyEffect { subject: Some(b), target: None, effect: Effect::GiveItem { item: "vaccino_porcino".into(), qty: 1 } }).unwrap();
    let def = sim.content().items["vaccino_porcino"].on_use.clone();
    for e in def {
        sim.execute(SimCommand::ApplyEffect { subject: Some(b), target: None, effect: e }).unwrap();
    }
    for id in [a, b] {
        sim.execute(SimCommand::ApplyEffect { subject: Some(id), target: None, effect: Effect::ApplyStatus { status: "mutazione_porcina".into(), severity: 3.5 } }).unwrap();
    }
    sim.run(15);
    assert_eq!(sim.world.get::<sim_core::stats::Race>(ae).unwrap().0, "maiale", "the merchant should be a pig by now");
    assert_ne!(sim.world.get::<sim_core::stats::Race>(be).unwrap().0, "maiale", "the vaccinated baker must stay human");
    assert!(sim.events().of_kind(kind::TRANSMUTATION).count() >= 1);
}

#[test]
fn hacker_takes_over_a_robot() {
    let mut sim = sim(4);
    let (nerd, ne) = id_of(&mut sim, "nerd");
    let pos = *sim.world.get::<Position>(ne).unwrap();
    let msg = sim.execute(SimCommand::Spawn { template: "robot".into(), zone: Some("fumetteria".into()), count: 1, faction: None, name: Some("Robottino".into()) }).unwrap();
    let robot = SimId(msg.trim_start_matches("create [").trim_end_matches(']').parse().unwrap());
    let re = sim.entity(robot).unwrap();
    sim.world.entity_mut(re).insert(pos);
    sim.execute(SimCommand::Order { entity: nerd, job: "hackeraggio".into(), target: Some(robot), zone: None }).unwrap();
    sim.run(10);
    assert_eq!(sim.world.get::<FactionMember>(re).unwrap().faction, "gilda_nerd");
}

#[test]
fn warehouse_damage_delays_gifts_and_raises_prices() {
    let mut sim = sim(6);
    let cap = sim.snapshot(true).entities.iter().find(|e| e.building.as_ref().is_some_and(|b| b.def == "capannone_regali")).unwrap().id;
    let before = sim.world.resource::<Market>().price("regalo_di_natale").unwrap();
    sim.execute(SimCommand::ApplyEffect { subject: Some(cap), target: None, effect: Effect::DamageBuilding(100.0) }).unwrap();
    sim.run(20);
    let gm = sim.world.resource::<GlobalModifiers>();
    assert!(gm.active.iter().any(|m| m.id == "ritardo_regali"));
    let after = sim.world.resource::<Market>().price("regalo_di_natale").unwrap();
    assert!(after > before * 1.1, "gift price {before} -> {after}");
}

#[test]
fn compendium_lists_the_world() {
    let sim = sim(1);
    let c = sim_core::compendium::build(sim.content(), sim.world.resource::<Params>());
    assert!(c["races"].as_array().unwrap().iter().any(|r| r["id"] == "rettiliano"));
    assert!(c["characters"].as_array().unwrap().iter().any(|r| r["id"] == "gerolamo_borgazzi" && r["immortal"] == true));
    assert_eq!(c["feed"], "Il Piccione Viaggiatore");
}

#[test]
fn poisoned_stock_infects_buyers() {
    let mut sim = sim(12);
    let banco = sim.snapshot(true).entities.iter().find(|e| e.building.as_ref().is_some_and(|b| b.def == "banco_mercato")).unwrap().id;
    let (_, buyer) = id_of(&mut sim, "commerciante_ricco");
    sim.execute(SimCommand::ApplyEffect { subject: Some(banco), target: None, effect: Effect::Contaminate { status: "diarrea".into(), load: 3.0 } }).unwrap();
    let shop = sim.entity(banco).unwrap();
    sim_core::economy::buy(&mut sim.world, buyer, shop, "pane").expect("purchase");
    assert!(sim.world.get::<sim_core::status::StatusEffects>(buyer).unwrap().has("diarrea"));
}
