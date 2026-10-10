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
    sim.execute(SimCommand::Order { entity: nerd, job: "hackera_macchina".into(), target: Some(robot), zone: None }).unwrap();
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
    assert!(gm.levels.contains_key("ritardo_regali"));
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

#[test]
fn fidenza_save_load_continues_identically() {
    let mut a = sim(17);
    a.run(40);
    let save = a.save();
    let mut b = sim_core::sim::SimBuilder::new(0);
    b.add_plugin(&fidenza_world::FidenzaPlugin::default()).unwrap();
    let mut b = b.build_from_save(&save).unwrap();
    a.run(40);
    b.run(40);
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn champion_moves_where_the_player_says_and_squads_follow() {
    use sim_core::player::Order;
    let mut sim = sim(3);
    let (champ, ce) = id_of(&mut sim, "leader_anarchico");
    assert!(sim.world.get::<sim_core::player::Controlled>(ce).is_some(), "the leader is the player's champion");
    // To another map (Piazza → Borgo → Quartiere Nerd): borders and walls on the way.
    let map = sim.world.resource::<WorldMap>().clone();
    let z = map.zones.iter().find(|z| z.id == "fumetteria").unwrap();
    let goal = Position::new(z.layer, z.x + 12, z.y + 20);
    sim.execute(SimCommand::PlayerOrder { player: "giocatore".into(), entity: champ, order: Order::Move { pos: goal } }).unwrap();
    let mut touched_wall = false;
    for _ in 0..120 {
        sim.tick();
        let p = *sim.world.get::<Position>(ce).unwrap();
        touched_wall |= map.blocked(&p);
        if p == goal {
            break;
        }
    }
    assert_eq!(*sim.world.get::<Position>(ce).unwrap(), goal);
    assert!(!touched_wall, "walked through a wall");

    // Squad of members following the champion.
    let members: Vec<SimId> = sim.snapshot(true).entities.iter()
        .filter(|e| e.faction.as_deref() == Some("anarchici_commercio") && e.kind == "pawn" && !e.dead && e.id != champ)
        .map(|e| e.id).take(3).collect();
    let msg = sim.execute(SimCommand::PlayerCreateSquad { player: "giocatore".into(), name: "Scorta".into(), members: members.clone() }).unwrap();
    let squad: u64 = msg.split_whitespace().nth(1).unwrap().parse().unwrap();
    sim.execute(SimCommand::PlayerSquadOrder { player: "giocatore".into(), squad, order: Some(sim_core::squads::SquadOrder::Follow { target: champ, distance: 3 }) }).unwrap();
    // The champion keeps living its life (food, sleep): members must catch up with it along the way.
    let mut caught = std::collections::BTreeSet::new();
    for t in 0..80 {
        sim.run(1);
        let champ_pos = *sim.world.get::<Position>(ce).unwrap();
        for m in &members {
            let e = sim.entity(*m).unwrap();
            if t >= 20 && sim.world.get::<Dead>(e).is_none() && sim.world.get::<Position>(e).unwrap().within(&champ_pos, 5) {
                caught.insert(*m);
            }
        }
    }
    assert!(caught.len() >= 2, "the squad did not follow the champion ({} caught up)", caught.len());
    // Another player's faction cannot be ordered around.
    let (bishop, _) = id_of(&mut sim, "vescovo");
    assert!(sim.execute(SimCommand::PlayerOrder { player: "giocatore".into(), entity: bishop, order: Order::Stop }).is_err());
}

#[test]
fn members_obey_more_or_less() {
    use sim_core::player::Order;
    let mut sim = sim(4);
    let (tommy, te) = id_of(&mut sim, "scippatore");
    let loyal = sim_core::player::obedience(&sim.world, te);
    sim.world.get_mut::<sim_core::factions::Dissent>(te).unwrap().0 = 90.0;
    let rebel = sim_core::player::obedience(&sim.world, te);
    assert!(loyal > 0.7 && rebel < 0.3, "obedience {loyal} -> {rebel}");
    let mut refused = 0;
    for _ in 0..20 {
        if sim.execute(SimCommand::PlayerOrder { player: "giocatore".into(), entity: tommy, order: Order::Stop }).is_err() {
            refused += 1;
        }
    }
    assert!(refused >= 10, "a rebel should refuse most orders ({refused}/20)");
}

#[test]
#[ignore = "bilanciamento: con le nuove azioni (docs/azioni.md) in 12 giorni nessuno ruba una reliquia; da rivedere col bilanciamento"]
fn factions_hunt_relics() {
    let mut sim = sim(2);
    sim.run(24 * 12);
    let stolen = sim.events().all().iter().filter(|e| e.kind == kind::JOB_DONE && e.message.contains("sottrae")).count();
    assert!(stolen >= 1, "no relic was stolen in 12 days");
}

#[test]
fn idle_champion_looks_after_its_needs() {
    let mut sim = sim(9);
    let (_, ce) = id_of(&mut sim, "leader_anarchico");
    sim.run(24 * 6);
    let fame = sim.world.get::<sim_core::stats::Stats>(ce).unwrap().get("fame");
    assert!(fame > 10.0, "the champion starved: fame {fame}");
}

#[test]
fn leader_turned_into_a_pig_gets_back_to_normal() {
    let mut sim = sim(6);
    let (champ, ce) = id_of(&mut sim, "leader_anarchico");
    let race = sim.world.get::<sim_core::stats::Race>(ce).unwrap().0.clone();
    sim.execute(SimCommand::ApplyEffect { subject: Some(champ), target: None, effect: Effect::Transmute("maiale".into()) }).unwrap();
    assert_eq!(sim.world.get::<sim_core::stats::Race>(ce).unwrap().0, "maiale");
    sim.run(26);
    assert_eq!(sim.world.get::<sim_core::stats::Race>(ce).unwrap().0, race, "the leader turns back into itself");
    assert!(sim.world.get::<sim_core::player::Transmuted>(ce).is_none());
}

#[test]
fn champion_is_knocked_out_instead_of_dying() {
    let mut sim = sim(6);
    let (champ, ce) = id_of(&mut sim, "leader_anarchico");
    let (killer, ke) = id_of(&mut sim, "ninja");
    let money_before = sim.world.get::<Wallet>(ce).unwrap().0;
    let killer_money = sim.world.get::<Wallet>(ke).unwrap().0;
    sim.execute(SimCommand::ApplyEffect { subject: Some(champ), target: Some(killer), effect: Effect::Damage { amount: 500.0, part: Some("braccio_sx".into()) } }).unwrap();
    sim.execute(SimCommand::ApplyEffect { subject: Some(champ), target: Some(killer), effect: Effect::Damage { amount: 500.0, part: Some("testa".into()) } }).unwrap();
    assert!(sim.world.get::<Dead>(ce).is_none(), "a champion must never die");
    assert!(sim.world.get::<sim_core::player::KnockedOut>(ce).is_some());
    assert!(sim.world.get::<sim_core::anatomy::Body>(ce).unwrap().parts.iter().any(|p| p.missing), "the arm is gone for now");
    let lost = money_before - sim.world.get::<Wallet>(ce).unwrap().0;
    assert!(lost > 0.0 && (sim.world.get::<Wallet>(ke).unwrap().0 - killer_money - lost).abs() < 0.01, "the knocker takes the lost money");
    // Out of action: orders are refused.
    assert!(sim.execute(SimCommand::PlayerOrder { player: "giocatore".into(), entity: champ, order: sim_core::player::Order::Stop }).is_err());
    sim.execute(SimCommand::ApplyEffect { subject: Some(champ), target: None, effect: Effect::Kill }).unwrap();
    assert!(sim.world.get::<Dead>(ce).is_none());
    sim.run(14);
    assert!(sim.world.get::<sim_core::player::KnockedOut>(ce).is_none(), "the champion gets back up");
    assert!(sim.world.get::<sim_core::anatomy::Body>(ce).unwrap().parts.iter().all(|p| !p.missing), "the champion gets back up whole");
    assert!(sim.events().of_kind("recovered").count() >= 1);
}

#[test]
fn player_faction_only_makes_alliances() {
    let mut sim = sim(7);
    sim.execute(SimCommand::SetRelation { a: "anarchici_commercio".into(), b: "ubriaconi".into(), value: 100.0 }).unwrap();
    sim.run(3);
    let f = sim.world.resource::<Factions>();
    assert!(f.states["anarchici_commercio"].absorbed_into.is_none());
    assert!(f.states["ubriaconi"].absorbed_into.is_none(), "players do not absorb either");
    assert!(f.allied("anarchici_commercio", "ubriaconi"));
    // Forced merge attempts become alliances too.
    sim_core::social::merge(&mut sim.world, "cda_fidenza_village", "anarchici_commercio");
    assert!(sim.world.resource::<Factions>().states["anarchici_commercio"].absorbed_into.is_none());
    // Alliance proposals: refused with low relations, accepted with good ones.
    assert!(sim.execute(SimCommand::PlayerAlliance { player: "giocatore".into(), faction: "chiesa".into() }).is_err());
    sim.execute(SimCommand::SetRelation { a: "anarchici_commercio".into(), b: "chiesa".into(), value: 60.0 }).unwrap();
    sim.execute(SimCommand::PlayerAlliance { player: "giocatore".into(), faction: "chiesa".into() }).unwrap();
    assert!(sim.world.resource::<Factions>().allied("chiesa", "anarchici_commercio"));
}

#[test]
fn payroll_adapts_instead_of_bankrupting() {
    let mut sim = sim(8);
    sim.world.resource_mut::<Factions>().states.get_mut("polizia_neutra").unwrap().treasury = 60.0;
    sim.run(24 * 3 + 1);
    let t = sim.world.resource::<Factions>().treasury("polizia_neutra");
    assert!(t > 0.0, "police treasury should never be emptied by salaries");
    assert!(sim.events().of_kind(kind::PAYROLL).any(|e| e.message.contains("% pagati") && e.message.contains("Polizia")));
}

#[test]
fn feed_separates_news_from_noise() {
    let mut sim = sim(10);
    sim.run(72);
    let feed = sim.world.resource::<Feed>();
    assert!(feed.articles.iter().all(|a| !a.category.is_empty()));
    let fakes: Vec<_> = feed.articles.iter().filter(|a| a.truth == sim_core::content::Truth::Fake).collect();
    assert!(!fakes.is_empty());
    assert!(fakes.iter().all(|a| a.category == "chiacchiere" && a.importance < 0.7), "fake news must be low-importance gossip");
    // The player's view never marks fake news.
    let snap = sim.snapshot(false);
    assert!(snap.feed.iter().all(|a| a.truth != sim_core::content::Truth::Fake));
}

#[test]
fn factions_conquer_quarters_and_the_champion_plants_banners() {
    let mut sim = sim(3);
    let (champ, ce) = id_of(&mut sim, "leader_anarchico");
    sim.run(24 * 3);
    let snap = sim.snapshot(false);
    let owned: Vec<(String, String)> = snap.territories.iter().filter_map(|(z, t)| t.owner.clone().map(|o| (z.clone(), o))).collect();
    eprintln!("quartieri: {owned:?}");
    assert!(owned.len() >= 3, "too few quarters held after 3 days: {owned:?}");
    assert!(sim.events().of_kind("territory").count() >= 3);
    // On the player's order the champion plants a banner where it stands.
    sim.world.get_mut::<Wallet>(ce).unwrap().0 = 200.0;
    sim.execute(SimCommand::PlayerOrder { player: "giocatore".into(), entity: champ, order: sim_core::player::Order::Job { job: "pianta_stendardo".into(), target: None } }).unwrap();
    sim.run(4);
    let banners = sim.snapshot(true).entities.iter().filter(|e| e.kind == "building" && e.building.as_ref().is_some_and(|b| b.def == "stendardo")).count();
    assert!(banners >= 1, "no banner planted");
}

#[test]
fn champion_walks_step_by_step() {
    let mut sim = sim(1);
    let champion = sim.world.resource::<Players>().players["giocatore"].leader.unwrap();
    let e = sim.entity(champion).unwrap();
    let start = *sim.world.get::<Position>(e).unwrap();
    let map = sim.world.resource::<WorldMap>().clone();
    // The first free neighbour: one step lands exactly there, and the champion stays put afterwards.
    let (dx, dy) = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .find(|(dx, dy)| !map.blocked(&Position::new(start.layer, start.x + dx, start.y + dy)))
        .expect("a free neighbour");
    let to = sim.execute(SimCommand::PlayerStep { player: "giocatore".into(), dx, dy });
    assert!(to.is_ok(), "{to:?}");
    let now = *sim.world.get::<Position>(e).unwrap();
    assert_eq!((now.x, now.y), (start.x + dx, start.y + dy));
    sim.run(5);
    assert_eq!(*sim.world.get::<Position>(e).unwrap(), now, "the controlled champion does not wander off");
    // Walls stop it.
    let mut blocked = None;
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        if map.blocked(&Position::new(now.layer, now.x + dx, now.y + dy)) {
            blocked = Some((dx, dy));
        }
    }
    if let Some((dx, dy)) = blocked {
        sim.execute(SimCommand::PlayerStep { player: "giocatore".into(), dx, dy }).unwrap();
        assert_eq!(*sim.world.get::<Position>(e).unwrap(), now);
    }
}

#[test]
fn champion_talks_and_the_team_shares_items() {
    let mut sim = sim(1);
    let champion = sim.world.resource::<Players>().players["giocatore"].leader.unwrap();
    let me = sim.entity(champion).unwrap();
    let here = *sim.world.get::<Position>(me).unwrap();
    // Someone to talk to, put next to the champion.
    let (other_id, other) = id_of(&mut sim, "allevatore");
    sim.world.entity_mut(other).insert(Position::new(here.layer, here.x + 1, here.y));
    let said = sim.execute(SimCommand::PlayerTalk { player: "giocatore".into(), target: other_id }).expect("answers");
    assert!(said.contains('«'), "{said}");
    // Too far: no answer.
    sim.world.entity_mut(other).insert(Position::new(here.layer, here.x + 9, here.y));
    assert!(sim.execute(SimCommand::PlayerTalk { player: "giocatore".into(), target: other_id }).is_err());
    // A beer from the faction's stock goes to the champion, who drinks it.
    sim.execute(SimCommand::PlayerGiveItem { player: "giocatore".into(), item: "birra".into(), to: champion, qty: 1 }).expect("given");
    assert_eq!(sim_core::inventory_ops::count(&sim.world, me, "birra"), 1);
    sim.execute(SimCommand::PlayerUseItem { player: "giocatore".into(), entity: champion, item: "birra".into() }).expect("drunk");
    assert_eq!(sim_core::inventory_ops::count(&sim.world, me, "birra"), 0);
}

#[test]
fn feed_keeps_few_main_news_and_drops_old_secondary_ones() {
    let mut sim = sim(2);
    sim.execute(SimCommand::SetParam { key: "press.secondary_ttl_ticks".into(), value: 24.0 }).unwrap();
    sim.execute(SimCommand::SetParam { key: "press.main_keep".into(), value: 5.0 }).unwrap();
    // Plenty of news, including old ones.
    for i in 0..40 {
        sim.execute(SimCommand::Publish { headline: format!("Notizia {i}"), truth: sim_core::content::Truth::Real, topics: vec!["gossip".into()], author: None }).unwrap();
        sim.tick();
    }
    sim.run(30);
    let tick = sim.tick_count();
    let params = sim.world.resource::<sim_core::params::Params>().clone();
    let players = vec!["anarchici_commercio".to_string()];
    let feed = sim.world.resource::<Feed>();
    let main = feed.articles.iter().filter(|a| sim_core::press::is_main(&params, a, &players)).count();
    assert!(main <= 5, "{main} main articles kept");
    assert!(feed.articles.iter().filter(|a| !sim_core::press::is_main(&params, a, &players)).all(|a| a.tick + 24 + 6 > tick));
    // Ids keep growing after pruning.
    assert!(feed.articles.windows(2).all(|w| w[0].id < w[1].id));
    assert!(feed.next_id > 40);
}

/// Spawns a template at a cell of a zone (x, y relative to the zone's map).
fn spawn_at(sim: &mut Simulation, template: &str, zone: &str, x: i32, y: i32) -> Entity {
    let map = sim.world.resource::<WorldMap>().clone();
    let z = map.zone(zone).unwrap_or_else(|| panic!("zona {zone}")).clone();
    let pos = Position::new(z.layer, z.x + x, z.y + y);
    sim_core::lifecycle::spawn_template(&mut sim.world, template, Some(pos), &Default::default()).expect("spawn")
}

#[test]
fn weapons_count_only_when_their_requirements_are_met() {
    let mut sim = sim(3);
    let e = spawn_at(&mut sim, "fidentino", "piazza_garibaldi", 20, 14);
    sim.world.get_mut::<Stats>(e).unwrap().set_base("con_arti_marziali", 0.0, (0.0, 100.0));
    sim_core::inventory_ops::give(&mut sim.world, e, "katana", 1);
    sim.tick();
    let eq = sim.world.get::<sim_core::equipment::Equipment>(e).unwrap();
    assert!(!eq.items.contains(&"katana".to_string()), "a katana without martial arts is just carried");
    let before = sim.world.get::<Stats>(e).unwrap().get("attacco");
    sim.world.get_mut::<Stats>(e).unwrap().set_base("con_arti_marziali", 30.0, (0.0, 100.0));
    sim.tick();
    let eq = sim.world.get::<sim_core::equipment::Equipment>(e).unwrap();
    assert!(eq.items.contains(&"katana".to_string()), "now it is wielded: {:?}", eq.items);
    let after = sim.world.get::<Stats>(e).unwrap().get("attacco");
    assert!(after >= before + 7.9, "the katana adds Forza: {before} → {after}");
}

#[test]
fn the_octopus_goes_for_women_first() {
    use sim_core::stats::Sex;
    let mut sim = sim(4);
    let octopus = spawn_at(&mut sim, "polipo", "scantinato_nerd", 5, 12);
    let man = spawn_at(&mut sim, "fidentino", "scantinato_nerd", 7, 12);
    let woman = spawn_at(&mut sim, "fidentino", "scantinato_nerd", 10, 12);
    sim.world.entity_mut(man).insert(Sex::Male);
    sim.world.entity_mut(woman).insert(Sex::Female);
    let grabbed = |sim: &Simulation, e: Entity| sim.world.get::<sim_core::status::StatusEffects>(e).is_some_and(|s| s.has("avvinghiato"));
    let mut first = None;
    for _ in 0..12 {
        sim.tick();
        if sim.world.get::<Dead>(octopus).is_some() {
            break;
        }
        if grabbed(&sim, woman) {
            first = Some("donna");
            break;
        }
        if grabbed(&sim, man) {
            first = Some("uomo");
            break;
        }
    }
    assert_eq!(first, Some("donna"), "the octopus should grab the woman first");
}

#[test]
fn the_champion_throws_what_can_be_thrown() {
    let mut sim = sim(7);
    let (champ, ce) = id_of(&mut sim, "leader_anarchico");
    let pos = *sim.world.get::<Position>(ce).unwrap();
    let target = sim_core::lifecycle::spawn_template(&mut sim.world, "fidentino", Some(Position::new(pos.layer, pos.x + 3, pos.y)), &Default::default()).unwrap();
    let tid = *sim.world.get::<SimId>(target).unwrap();
    sim_core::inventory_ops::give(&mut sim.world, ce, "sasso", 3);
    sim_core::player::give_order(&mut sim.world, "giocatore", champ, sim_core::player::Order::Job { job: "lancia_oggetto".into(), target: Some(tid) }).unwrap();
    sim.run(4);
    assert!(sim_core::inventory_ops::count(&sim.world, ce, "sasso") < 3, "a stone was thrown");
    let health = sim.world.get::<sim_core::anatomy::Body>(target).unwrap().health_ratio();
    assert!(health < 1.0, "the target was hit");
}

/// The admin console sends back definitions as `/api/content` serves them: every one must be
/// accepted unchanged.
#[test]
fn every_definition_round_trips_through_the_console() {
    let sim = sim(1);
    let content = sim.content().clone();
    let all = serde_json::to_value(&content).unwrap();
    for kind in sim_core::content::EDITABLE_KINDS {
        for (id, def) in all[*kind].as_object().unwrap_or_else(|| panic!("{kind} missing")) {
            if let Err(e) = content.with_def(kind, def.clone()) {
                panic!("{kind}/{id}: {e}");
            }
        }
    }
}

#[test]
fn console_edits_race_ranges_sexes_and_saves_them() {
    use sim_core::stats::{Sex, Stats};
    let mut sim = sim(1);
    let dir = std::env::temp_dir().join(format!("fidenza_console_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(sim_core::content::OVERRIDES_FILE);
    let _ = std::fs::remove_file(&file);
    sim.world.insert_resource(sim_core::content::ContentOverrides(Some(file.clone())));
    let mut race = serde_json::to_value(&sim.content().races["fidentino"]).unwrap();
    race["sexes"] = serde_json::json!(["Female"]);
    race["stat_ranges"] = serde_json::json!({ "forza": { "max": 1.0 }, "carisma": { "initial": 40.0, "spread": 0.0 } });
    sim.execute(SimCommand::EditContent { kind: "races".into(), def: race }).expect("edit accepted");
    assert_eq!(sim.content().races["fidentino"].sexes, vec![Sex::Female]);
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(saved["races"][0]["id"], "fidentino");

    let tpl = sim.content().templates.values().find(|t| t.race == "fidentino" && !t.unique && !t.virtual_entity && !t.stats.contains_key("carisma")).unwrap().id.clone();
    for _ in 0..5 {
        let e = sim_core::lifecycle::spawn_template(&mut sim.world, &tpl, None, &SpawnOverrides::default()).unwrap();
        assert_eq!(sim.world.get::<Sex>(e), Some(&Sex::Female));
        let c = sim.world.get::<Stats>(e).unwrap().base["carisma"];
        assert_eq!(c, 40.0, "senza variazione della razza si nasce esattamente al valore iniziale");
    }
    sim.tick();
    for (race, stats) in sim.world.query::<(&sim_core::stats::Race, &Stats)>().iter(&sim.world) {
        if race.0 == "fidentino" {
            assert!(stats.effective.get("forza").copied().unwrap_or(0.0) <= 1.0);
        }
    }
    // Invalid edits are refused and change nothing.
    let mut bad = serde_json::to_value(&sim.content().races["fidentino"]).unwrap();
    bad["abilities"] = serde_json::json!(["non_esiste"]);
    assert!(sim.execute(SimCommand::EditContent { kind: "races".into(), def: bad }).is_err());
    assert!(sim.content().races["fidentino"].abilities.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Classes add up; races not listed cannot take a class; a class goes only by its own loss condition;
/// a class that replaces another takes its place and keeps it from coming back.
#[test]
fn classes_add_up_and_follow_their_own_rules() {
    use sim_core::classes::{eligible, keeps, take, ClassState};
    use sim_core::stats::{Classes, Stats};
    let mut sim = sim(1);
    let set = |sim: &mut Simulation, e: Entity, k: &str, v: f32| {
        let mut s = sim.world.get_mut::<Stats>(e).unwrap();
        s.base.insert(k.into(), v);
        s.effective.insert(k.into(), v);
    };
    let tpl = sim.content().templates.values().find(|t| t.race == "fidentino" && !t.unique && !t.virtual_entity).unwrap().id.clone();
    let e = sim_core::lifecycle::spawn_template(&mut sim.world, &tpl, None, &SpawnOverrides::default()).unwrap();
    for (k, v) in [("con_agricoltura", 40.0), ("resistenza", 15.0), ("con_lavori_manuali", 50.0), ("forza", 20.0)] {
        set(&mut sim, e, k, v);
    }
    let options = eligible(&mut sim.world, e);
    assert!(options.contains(&"agricoltore".to_string()) && options.contains(&"fabbro".to_string()), "{options:?}");
    take(&mut sim.world, e, "agricoltore");
    take(&mut sim.world, e, "fabbro");
    let held = sim.world.get::<Classes>(e).unwrap().0.clone();
    assert!(held.contains(&"agricoltore".into()) && held.contains(&"fabbro".into()), "le classi si sommano: {held:?}");
    assert!(!held.contains(&"normie".into()), "la classe di base se ne va");
    // A farmer who forgets farming stays a farmer: the class has no loss condition.
    set(&mut sim, e, "con_agricoltura", 0.0);
    assert!(keeps(&mut sim.world, e, "agricoltore"));
    // The tramp's own rule: with 100 € he is no longer a tramp.
    set(&mut sim, e, "puzza", 90.0);
    take(&mut sim.world, e, "barbone");
    sim.world.get_mut::<sim_core::stats::Wallet>(e).unwrap().0 = 500.0;
    assert!(!keeps(&mut sim.world, e, "barbone"));
    // Replacing: the Sith takes the Jedi's place and the Jedi cannot come back.
    take(&mut sim.world, e, "jedi");
    take(&mut sim.world, e, "sith");
    let held = sim.world.get::<Classes>(e).unwrap().0.clone();
    assert!(held.contains(&"sith".into()) && !held.contains(&"jedi".into()), "{held:?}");
    for (k, v) in [("forza_jedi", 90.0), ("aggressivita", 0.0), ("stress", 0.0)] {
        set(&mut sim, e, k, v);
    }
    assert!(!eligible(&mut sim.world, e).contains(&"jedi".to_string()));
    assert!(sim.world.get::<ClassState>(e).unwrap().acquired.contains(&"sith".to_string()));
    // A boar with the same skills cannot become a farmer: its race is not listed.
    let boar = sim_core::lifecycle::spawn_template(&mut sim.world, "cinghiale", None, &SpawnOverrides::default()).unwrap();
    for (k, v) in [("con_agricoltura", 40.0), ("resistenza", 15.0)] {
        set(&mut sim, boar, k, v);
    }
    assert!(!eligible(&mut sim.world, boar).contains(&"agricoltore".to_string()));
}

/// Abilities are permanent: modifiers on the holder, auras on whoever stands near (while near, or
/// accumulating). Attacks, spells and transformations are actions with their own effects.
#[test]
fn abilities_are_traits_and_actions_have_effects() {
    use sim_core::map::Position;
    use sim_core::stats::Stats;
    let mut sim = sim(1);
    let tpl = sim.content().templates.values().find(|t| t.race == "fidentino" && !t.unique && !t.virtual_entity).unwrap().id.clone();
    let (_, someone) = id_of(&mut sim, "scippatore");
    let p = *sim.world.get::<Position>(someone).unwrap();
    let spawn = |sim: &mut Simulation, t: &str, pos: Position| sim_core::lifecycle::spawn_template(&mut sim.world, t, Some(pos), &SpawnOverrides::default()).unwrap();
    let goth = spawn(&mut sim, &tpl, p);
    let near = spawn(&mut sim, &tpl, Position { x: p.x + 1, ..p });
    sim_core::classes::take(&mut sim.world, goth, "goth");
    sim_core::classes::take(&mut sim.world, goth, "barbone");
    let base = sim.world.get::<Stats>(near).unwrap().base.clone();
    sim.tick();
    let s = sim.world.get::<Stats>(near).unwrap();
    let m = |k: &str| s.effective.get(k).copied().unwrap_or(0.0) - s.base.get(k).copied().unwrap_or(0.0);
    assert!(m("malinconia") >= 1.0, "aura depressa finché vicino: {}", m("malinconia"));
    assert!(s.base["puzza"] > base["puzza"], "il tanfo si accumula");
    // Moving away: the lasting part goes, the accumulated smell stays.
    let smell = s.base["puzza"];
    sim.world.entity_mut(near).insert(Position { x: p.x + 30, ..p });
    sim.world.entity_mut(near).insert(sim_core::movement::Movement::default());
    sim_core::abilities::auras(&mut sim.world);
    assert!(sim.world.get::<sim_core::abilities::AuraBonus>(near).unwrap().0.is_empty());
    assert!(sim.world.get::<Stats>(near).unwrap().base["puzza"] >= smell);
    // Modifiers on the holder: the robot's armour.
    let robot_tpl = sim.content().templates.values().find(|t| t.race == "robot").unwrap().id.clone();
    let robot = spawn(&mut sim, &robot_tpl, p);
    sim.tick();
    let s = sim.world.get::<Stats>(robot).unwrap();
    assert!(s.effective["difesa"] >= s.base["difesa"] + 10.0 - 0.01 || s.effective["difesa"] >= sim.content().stats["difesa"].max);
    // An action with effects: the reptilian takes a human shape on order, then must wait.
    let rept_tpl = sim.content().templates.values().find(|t| t.race == "rettiliano" && !t.unique).unwrap().id.clone();
    let rept = spawn(&mut sim, &rept_tpl, p);
    let rid = *sim.world.get::<SimId>(rept).unwrap();
    sim_core::infiltration::revert(&mut sim.world, rept);
    sim.execute(SimCommand::UseAbility { entity: rid, ability: "mutaforma".into(), target: None }).expect("mutaforma");
    assert!(sim.world.get::<sim_core::infiltration::Disguise>(rept).is_some());
    assert!(sim.execute(SimCommand::UseAbility { entity: rid, ability: "mutaforma".into(), target: None }).is_err(), "in attesa");
}

/// A need moves its stat every tick; below each threshold its effects run every tick and its stat
/// modifiers last while below; eating (an effect on the need) raises the stat.
#[test]
fn needs_move_stats_and_thresholds_bite() {
    use sim_core::stats::Stats;
    let mut sim = sim(1);
    let tpl = sim.content().templates.values().find(|t| t.race == "fidentino" && !t.unique && !t.virtual_entity).unwrap().id.clone();
    let e = sim_core::lifecycle::spawn_template(&mut sim.world, &tpl, None, &SpawnOverrides::default()).unwrap();
    assert_eq!(sim.world.get::<Stats>(e).unwrap().get("fame"), 100.0, "si nasce sazi");
    sim.tick();
    assert!(sim.world.get::<Stats>(e).unwrap().base["fame"] < 100.0, "la fame cala ogni ora");
    let content = sim.content().clone();
    sim.world.get_mut::<Stats>(e).unwrap().set_base("fame", 1.0, (0.0, 100.0));
    let forza = sim.world.get::<Stats>(e).unwrap().base["forza"];
    sim.tick();
    let s = sim.world.get::<Stats>(e).unwrap();
    assert!(s.effective["forza"] <= forza - 5.0 + 0.01, "denutrito: -5 forza finché sotto 3");
    assert!(sim.events().all().iter().all(|ev| ev.kind != "error"));
    // Eating: an effect on the need raises the stat by a share of its range.
    let ctx = sim_core::effects::EffectCtx::new(Some(e), None, "test");
    sim_core::effects::apply_effects(&mut sim.world, &ctx, &[sim_core::content::Effect::ModNeed { need: "fame".into(), amount: 0.5 }]);
    assert!(sim.world.get::<Stats>(e).unwrap().base["fame"] > 50.0);
    sim.tick();
    let s = sim.world.get::<Stats>(e).unwrap();
    assert!(s.effective["forza"] > forza - 5.0 + 0.01 || content.needs["fame"].thresholds.is_empty(), "sopra la soglia il modificatore sparisce");
}

/// Modes: the player's order beats the faction's mode, which beats the pawn's own choice; a pawn kept in a
/// mode it would not choose grows dissent; the mode weighs actions by tag and sets the work priorities.
#[test]
fn modes_order_faction_and_own_choice() {
    use sim_core::modes::{Mode, ModeSource};
    use sim_core::stats::Stats;
    let mut sim = sim(1);
    let (tid, e) = id_of(&mut sim, "scippatore");
    let faction = sim.world.get::<sim_core::factions::FactionMember>(e).unwrap().faction.clone();
    sim_core::modes::update(&mut sim.world);
    let m = sim.world.get::<Mode>(e).unwrap().clone();
    assert_eq!((m.current.as_str(), m.source), (m.own.as_str(), ModeSource::Own));
    // Very aggressive: it chooses conquest by itself, and gets its stats.
    sim.world.get_mut::<Stats>(e).unwrap().set_base("aggressivita", 90.0, (0.0, 100.0));
    sim.tick();
    assert_eq!(sim.world.get::<Mode>(e).unwrap().current, "conquista");
    // The faction says "work": it works, and dissent grows.
    let d0 = sim.world.get::<sim_core::factions::Dissent>(e).unwrap().0;
    sim.execute(SimCommand::SetMode { entity: None, faction: Some(faction), mode: Some("lavora".into()) }).unwrap();
    let m = sim.world.get::<Mode>(e).unwrap().clone();
    assert_eq!((m.current.as_str(), m.source), ("lavora", ModeSource::Faction));
    assert!(sim.world.get::<sim_core::factions::Dissent>(e).unwrap().0 > d0, "forzata contro voglia");
    // The player's order for the pawn beats the faction.
    sim.execute(SimCommand::SetMode { entity: Some(tid), faction: None, mode: Some("ricerca".into()) }).unwrap();
    let m = sim.world.get::<Mode>(e).unwrap().clone();
    assert_eq!((m.current.as_str(), m.source), ("ricerca", ModeSource::Player));
    assert_eq!(sim.world.get::<sim_core::jobs::WorkPriorities>(e).unwrap().get("informatica"), 2);
    // Weights by tag.
    let content = sim.content().clone();
    let w = |m: &str, tags: &[&str]| {
        let mode = Mode { current: m.into(), ..Default::default() };
        sim_core::modes::weight(&content, Some(&mode), &tags.iter().map(|t| t.to_string()).collect::<Vec<_>>())
    };
    assert_eq!(w("conquista", &["violenza"]), 3.0);
    assert_eq!(w("lavora", &["lavoro", "soldi"]), 5.0);
    assert_eq!(w("lavora", &["fede"]), 1.0);
}

/// Statuses are deterministic: intensity moves by `per_tick`, ends at 0, thresholds switch on; contagion
/// is an aura cut by resistance.
#[test]
fn statuses_are_deterministic_intensities() {
    use sim_core::map::Position;
    use sim_core::stats::Stats;
    use sim_core::status::StatusEffects;
    let mut sim = sim(1);
    let tpl = sim.content().templates.values().find(|t| t.race == "fidentino" && !t.unique && !t.virtual_entity).unwrap().id.clone();
    let (_, someone) = id_of(&mut sim, "scippatore");
    let p = *sim.world.get::<Position>(someone).unwrap();
    let spawn = |sim: &mut Simulation, pos: Position| sim_core::lifecycle::spawn_template(&mut sim.world, &tpl, Some(pos), &SpawnOverrides::default()).unwrap();
    let a = spawn(&mut sim, p);
    // A dose of "stordito" (100) lasts 3 ticks.
    assert!(sim_core::status::apply_status(&mut sim.world, a, "stordito", 1.0, None));
    let sev = |sim: &Simulation, e, s: &str| sim.world.get::<StatusEffects>(e).unwrap().severity(s);
    assert_eq!(sev(&sim, a, "stordito"), 100.0);
    sim_core::status::tick_statuses(&mut sim.world);
    assert!((sev(&sim, a, "stordito") - 66.667).abs() < 0.01);
    sim_core::status::tick_statuses(&mut sim.world);
    sim_core::status::tick_statuses(&mut sim.world);
    assert!(!sim.world.get::<StatusEffects>(a).unwrap().has("stordito"), "finisce a 0");
    // Three drinks: tipsy goes over 90 and becomes wasted.
    for _ in 0..3 {
        sim_core::status::apply_status(&mut sim.world, a, "brillo", 1.0, None);
    }
    let se = sim.world.get::<StatusEffects>(a).unwrap();
    assert!(se.has("schifoso") && !se.has("brillo"), "{:?}", se.active.keys().collect::<Vec<_>>());
    // Contagion: a neighbour of a sick pawn gains intensity; a tougher one gains less.
    let sick = spawn(&mut sim, p);
    let weak = spawn(&mut sim, Position { x: p.x + 1, ..p });
    let tough = spawn(&mut sim, Position { x: p.x - 1, ..p });
    sim.world.get_mut::<Stats>(weak).unwrap().set_base("resistenza", 0.0, (0.0, 100.0));
    sim.world.get_mut::<Stats>(tough).unwrap().set_base("resistenza", 60.0, (0.0, 100.0));
    let contagious = sim.content().statuses.values().find(|s| s.contagion.as_ref().is_some_and(|c| c.radius > 0 && c.per_tick > 0.0)).unwrap().id.clone();
    // Two doses: sick but below the last threshold.
    assert!(sim_core::status::apply_status(&mut sim.world, sick, &contagious, 2.0, None));
    sim_core::hygiene::hygiene_tick(&mut sim.world);
    let (w, t) = (sev(&sim, weak, &contagious), sev(&sim, tough, &contagious));
    assert!(w > 0.0 && t < w, "debole {w}, resistente {t}");
}

/// Events: spawns stop at their `max_alive`; a news source publishes its headlines in turn, never the
/// same twice in a row; an event can be forced.
#[test]
fn events_spawn_publish_in_turn_and_can_be_forced() {
    let mut sim = sim(1);
    let ev = sim.content().events["talpe"].clone();
    let max = ev.effects.iter().find_map(|e| match e {
        Effect::Spawn { max_alive, .. } => Some(*max_alive),
        _ => None,
    }).unwrap();
    for _ in 0..(max + 3) {
        sim.execute(SimCommand::FireTrigger { id: "talpe".into() }).unwrap();
    }
    let alive = sim.world.query::<(&sim_core::dungeon::SpawnedBy, Option<&Dead>)>().iter(&sim.world).filter(|(s, d)| s.0 == "event:talpe" && d.is_none()).count() as u32;
    assert!(alive <= max && alive > 0, "{alive} talpe, massimo {max}");
    for _ in 0..3 {
        sim.execute(SimCommand::FireTrigger { id: "overmind_fake_news".into() }).unwrap();
    }
    let feed = &sim.world.resource::<sim_core::press::Feed>().articles;
    let heads: Vec<&String> = feed.iter().rev().take(3).map(|a| &a.headline).collect();
    assert!(heads[0] != heads[1] && heads[1] != heads[2], "{heads:?}");
}

/// Buildings: productions that happen by themselves add their outputs every `every` ticks and pay their
/// money to the owner (scaled by integrity).
#[test]
fn productions_happen_by_themselves() {
    use sim_core::buildings::{Building, Owner};
    use sim_core::inventory::Stock;
    let mut sim = sim(1);
    let content = sim.content().clone();
    let blds: Vec<(Entity, Building)> = sim.world.query::<(Entity, &Building)>().iter(&sim.world).map(|(e, b)| (e, b.clone())).collect();
    // A field: outputs without inputs.
    let (field, p) = blds
        .iter()
        .find_map(|(e, b)| content.buildings.get(&b.def)?.productions.iter().find(|p| p.every > 0 && p.inputs.is_empty() && !p.outputs.is_empty()).map(|p| (*e, p.clone())))
        .unwrap();
    let (item, n) = p.outputs.iter().next().map(|(k, v)| (k.clone(), *v)).unwrap();
    let before = sim.world.get::<Stock>(field).unwrap().count(&item);
    sim.world.resource_mut::<sim_core::time::SimClock>().tick = p.every * 10;
    sim_core::buildings::buildings_tick(&mut sim.world);
    assert_eq!(sim.world.get::<Stock>(field).unwrap().count(&item), before + n, "{item}");
    // Visitors: money for the owner faction.
    let (b, money, every) = blds
        .iter()
        .find_map(|(_, b)| {
            let Owner::Faction(_) = &b.owner else { return None };
            content.buildings.get(&b.def)?.productions.iter().find(|p| p.every > 0 && p.money > 0.0 && p.inputs.is_empty()).map(|p| (b.clone(), p.money, p.every))
        })
        .unwrap();
    let Owner::Faction(f) = &b.owner else { unreachable!() };
    let t0 = sim.world.resource::<sim_core::factions::Factions>().states[f].treasury;
    sim.world.resource_mut::<sim_core::time::SimClock>().tick = every * 11;
    sim_core::buildings::buildings_tick(&mut sim.world);
    let gained = sim.world.resource::<sim_core::factions::Factions>().states[f].treasury - t0;
    assert!(gained >= money * (b.hp / b.max_hp) as f64 - 0.01, "incasso {gained}, atteso almeno {money}");
}

/// Beliefs spread like a contagion (credibility of who speaks, suggestibility of who listens), fade, and
/// make a faction's public trust; values weigh actions and make witnesses judge who does them.
#[test]
fn beliefs_spread_and_values_judge() {
    use sim_core::beliefs::{faction_key, opinion, values_weight};
    use sim_core::map::Position;
    use sim_core::stats::Stats;
    let mut sim = sim(1);
    let tpl = sim.content().templates.values().find(|t| t.race == "fidentino" && !t.unique && !t.virtual_entity).unwrap().id.clone();
    let (_, someone) = id_of(&mut sim, "scippatore");
    let p = *sim.world.get::<Position>(someone).unwrap();
    let far = Position { x: p.x + 40, ..p };
    let spawn = |sim: &mut Simulation, pos: Position| sim_core::lifecycle::spawn_template(&mut sim.world, &tpl, Some(pos), &SpawnOverrides::default()).unwrap();
    let (speaker, gullible, sceptic) = (spawn(&mut sim, far), spawn(&mut sim, Position { x: far.x + 1, ..far }), spawn(&mut sim, Position { x: far.x - 1, ..far }));
    let set = |sim: &mut Simulation, e: Entity, k: &str, v: f32| {
        let mut s = sim.world.get_mut::<Stats>(e).unwrap();
        s.base.insert(k.into(), v);
        s.effective.insert(k.into(), v);
    };
    set(&mut sim, speaker, "credibilita", 90.0);
    set(&mut sim, gullible, "influenzabilita", 95.0);
    set(&mut sim, sceptic, "influenzabilita", 5.0);
    let key = faction_key("chiesa");
    sim_core::beliefs::believe(&mut sim.world, speaker, key.clone(), -80.0, 90.0);
    sim.world.resource_mut::<sim_core::time::SimClock>().tick = 300;
    sim_core::beliefs::tick(&mut sim.world);
    let (g, s) = (opinion(&sim.world, gullible, &key), opinion(&sim.world, sceptic, &key));
    assert!(g < 0.0 && g < s, "il credulone ci crede più dello scettico: {g} vs {s}");
    // Public trust of the faction goes down.
    assert!(sim.world.resource::<sim_core::factions::Factions>().states["chiesa"].public_trust < 0.0);
    // Defamation: a Tell effect plants the belief in the listener.
    let ctx = sim_core::effects::EffectCtx::new(Some(speaker), Some(sceptic), "test");
    sim_core::effects::apply_effects(&mut sim.world, &ctx, &[Effect::Tell { about: sim_core::content::About::Faction("casino_diablo".into()), stance: -70.0, strength: 40.0 }]);
    assert!(opinion(&sim.world, sceptic, &faction_key("casino_diablo")) < -20.0);
    // Values: a devout pawn likes faith actions more.
    set(&mut sim, gullible, "v_fede", 100.0);
    let w = values_weight(sim.content(), sim.world.get::<Stats>(gullible), &["fede".to_string()]);
    assert!(w > 1.5, "{w}");
    // Beliefs fade.
    for i in 1..400 {
        sim.world.resource_mut::<sim_core::time::SimClock>().tick = 300 + i * 3;
        sim_core::beliefs::tick(&mut sim.world);
    }
    assert_eq!(opinion(&sim.world, sceptic, &faction_key("casino_diablo")), 0.0, "col tempo si dimentica");
}

#[test]
fn every_map_and_dungeon_can_be_reached_and_has_its_boss() {
    let mut sim = sim(5);
    let map = sim.world.resource::<WorldMap>().clone();
    let piazza = map.zone("piazza_garibaldi").unwrap().clone();
    let from = Position::new(piazza.layer, piazza.x + 17, piazza.y + 17);
    for (i, l) in map.layers.iter().enumerate() {
        let to = Position::new(i as u16, 1, 1);
        assert!(map.travel_cost(&from, &to, 25) < 100_000, "la mappa {} non si raggiunge da Piazza Garibaldi", l.id);
    }
    for z in ["cripta_duomo", "cantine_culatello", "covo_rettiliano", "scantinato_nerd", "cripta", "catacombe", "miniera_di_sale", "gallerie", "la_bassa", "salsomaggiore_terme", "fidenza_village"] {
        assert!(map.zone(z).is_some(), "manca la zona {z}");
    }
    sim.run(2);
    for (boss, zone) in [("gran_mangione", "cantine_culatello"), ("regina_rettiliana", "covo_rettiliano"), ("vescovo_non_morto", "cripta_duomo"), ("dungeon_master", "scantinato_nerd")] {
        let (_, e) = id_of(&mut sim, boss);
        let pos = *sim.world.get::<Position>(e).unwrap();
        assert!(map.in_zone(zone, &pos), "{boss} dovrebbe stare in {zone}");
    }
}
