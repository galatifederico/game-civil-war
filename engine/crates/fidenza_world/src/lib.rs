//! # fidenza_world
//!
//! The satirical world of Fidenza and Salsomaggiore as a `sim_core` plugin. Almost everything is data
//! (`data/*.ron`); this crate only adds what data alone cannot express:
//!
//! - effect `oracle_assign`: the Oracle joins a random faction at tick 0;
//! - effect `oracle_reveal`: the Oracle reveals a map secret (a disguised infiltrator or where a relic is);
//! - effect `borgazzi_masterpiece`: Gerolamo Borgazzi paints the next of his unique artworks;
//! - condition `intruders`: pawns of other factions inside a zone (dungeon exit conditions);
//! - job handler `hack`: a hacker takes control of a robot or drone;
//! - effect `indaga`: the Investigator finds out something specific (where a relic is, the way to a place,
//!   who is in disguise nearby, who is wanted, who holds the roles) and writes it in its journal;
//! - condition `fertile_coppia` and effect `concepisci`: who can have children with whom (Rettiliani among
//!   themselves, Salsesi, Fidentini and Nani with each other, everybody else within their race) and the birth.

use std::path::{Path, PathBuf};

use sim_core::bevy_ecs::prelude::*;
use sim_core::content::{ContentError, Truth};
use sim_core::effects::EffectCtx;
use sim_core::factions::FactionMember;
use sim_core::infiltration::Disguise;
use sim_core::jobs::{JobCtx, JobResult};
use sim_core::prelude::*;

/// Folder with the RON content of the world.
pub fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data")
}

pub struct FidenzaPlugin {
    pub data: PathBuf,
}

impl Default for FidenzaPlugin {
    fn default() -> Self {
        Self { data: data_dir() }
    }
}

impl SimPlugin for FidenzaPlugin {
    fn name(&self) -> &str {
        "fidenza_world"
    }

    fn build(&self, b: &mut SimBuilder) -> Result<(), ContentError> {
        b.load_pack_dir(&self.data)?;
        b.register_effect("oracle_assign", oracle_assign);
        b.register_effect("oracle_reveal", oracle_reveal);
        b.register_effect("borgazzi_masterpiece", borgazzi_masterpiece);
        b.register_condition("intruders", intruders);
        b.register_effect("mount", mount);
        b.register_effect("dismount", dismount);
        b.register_effect("dig_frontier", dig_frontier);
        b.register_condition("near", near);
        b.register_job_handler("hack", hack);
        b.register_effect("concepisci", conceive);
        b.register_effect("indaga", investigate);
        b.register_condition("fertile_coppia", fertile_couple);
        Ok(())
    }
}

/// Builds the Fidenza simulation with a seed.
pub fn build(seed: u64) -> Result<Simulation, ContentError> {
    let mut b = Simulation::builder(seed);
    b.add_plugin(&FidenzaPlugin::default())?;
    b.build()
}

/// A builder with the Fidenza content and plugin, e.g. to load a save into.
pub fn builder(seed: u64) -> Result<SimBuilder, String> {
    let mut b = Simulation::builder(seed);
    b.add_plugin(&FidenzaPlugin::default()).map_err(|e| e.to_string())?;
    Ok(b)
}

fn find_template(world: &mut World, t: &str) -> Option<Entity> {
    sim_core::sorted_entities::<TemplateId>(world)
        .into_iter()
        .find(|e| world.get::<TemplateId>(*e).is_some_and(|x| x.0 == t) && world.get::<Dead>(*e).is_none())
}

fn oracle_assign(world: &mut World, _ctx: &EffectCtx, _p: &serde_json::Value) {
    let Some(oracle) = find_template(world, "oracolo") else { return };
    let content = world.resource::<Content>();
    let candidates: Vec<String> = content
        .factions
        .values()
        .filter(|f| f.playable && f.role == sim_core::content::FactionRole::Regular)
        .map(|f| f.id.clone())
        .collect();
    let Some(f) = world.resource_mut::<SimRng>().pick(&candidates).cloned() else { return };
    sim_core::social::change_faction(world, oracle, &f, "assegnazione dell'Oracolo");
}

fn oracle_reveal(world: &mut World, _ctx: &EffectCtx, _p: &serde_json::Value) {
    let Some(oracle) = find_template(world, "oracolo") else { return };
    // Half the time: unmask an infiltrator; otherwise: tell where a major relic is.
    let disguised: Vec<Entity> = sim_core::sorted_entities::<Disguise>(world)
        .into_iter()
        .filter(|e| world.get::<Dead>(*e).is_none())
        .collect();
    let roll = world.resource_mut::<SimRng>().next_f32();
    if roll < 0.5
        && let Some(i) = world.resource_mut::<SimRng>().index(disguised.len()) {
            sim_core::infiltration::expose(world, disguised[i], "profezia dell'Oracolo", Some(oracle));
            return;
        }
    let content = world.resource::<Content>().clone();
    let relics: Vec<String> = content.items.values().filter(|i| i.tags.iter().any(|t| t == "reliquia_maggiore")).map(|i| i.id.clone()).collect();
    let Some(relic) = world.resource_mut::<SimRng>().pick(&relics).cloned() else { return };
    let factions: Vec<String> = world.resource::<Factions>().states.keys().cloned().collect();
    let holder = factions.into_iter().find(|f| sim_core::inventory_ops::faction_holdings(world, f).contains_key(&relic));
    let relic_name = content.items[&relic].name.clone();
    let where_ = holder.and_then(|f| content.factions.get(&f).map(|d| d.name.clone())).unwrap_or_else(|| "nessuno".into());
    sim_core::press::publish(
        world,
        Some(oracle),
        format!("L'Oracolo rivela: la {relic_name} è in mano a {where_}"),
        Truth::Real,
        vec!["oracolo".into(), "reliquia".into()],
        None,
        None,
    );
}

fn borgazzi_masterpiece(world: &mut World, _ctx: &EffectCtx, _p: &serde_json::Value) {
    let Some(b) = find_template(world, "gerolamo_borgazzi") else { return };
    // The works already in the world (carried or in some building), then the next one of the collection.
    let existing: std::collections::BTreeSet<String> = {
        let mut q = world.query::<&Inventory>();
        let mut seen: std::collections::BTreeSet<String> = q.iter(world).flat_map(|i| i.items().map(|(k, _)| k.clone()).collect::<Vec<_>>()).collect();
        let mut qs = world.query::<&sim_core::inventory::Stock>();
        seen.extend(qs.iter(world).flat_map(|s| s.0.iter().filter(|(_, n)| **n > 0).map(|(k, _)| k.clone()).collect::<Vec<_>>()));
        seen
    };
    let works = world.resource::<Content>().collections.get("opere_borgazzi").map(|c| c.items.clone()).unwrap_or_default();
    let Some(opera) = works.into_iter().find(|o| !existing.contains(o)) else { return };
    let opera = opera.as_str();
    // Finished works go on show (and on sale) in his gallery; without it he keeps them.
    let gallery = sim_core::sorted_entities::<sim_core::buildings::Building>(world)
        .into_iter()
        .find(|e| world.get::<sim_core::buildings::Building>(*e).is_some_and(|x| x.def == "galleria_borgazzi" && x.hp > 0.0));
    if sim_core::inventory_ops::give(world, gallery.unwrap_or(b), opera, 1) == 0 {
        return;
    }
    let name = world.resource::<Content>().items[opera].name.clone();
    let tick = world.resource::<SimClock>().tick;
    let (id, pos) = (world.get::<SimId>(b).copied(), world.get::<Position>(b).copied());
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new("masterpiece", format!("Gerolamo Borgazzi completa {name}")).actor(id).pos(pos).news(0.8).tags(["arte"]),
    );
}

/// `{"zone": "...", "faction": "...", "min": n}`: at least `min` living pawns not in `faction` inside the zone.
fn intruders(world: &mut World, _ctx: &EffectCtx, p: &serde_json::Value) -> bool {
    let zone = p.get("zone").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let faction = p.get("faction").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let min = p.get("min").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
    let map = world.resource::<WorldMap>().clone();
    let mut q = world.query_filtered::<(&Position, Option<&FactionMember>), (With<Pawn>, Without<Dead>)>();
    q.iter(world)
        .filter(|(pos, m)| map.in_zone(&zone, pos) && m.is_none_or(|m| m.faction != faction))
        .count()
        >= min
}

/// Dwarf-Fortress style expansion for AI factions: `{"zone", "faction", "count", "max_open"}` designates
/// up to `count` diggable cells next to already open floor inside the zone (`from` is relative to the zone).
fn dig_frontier(world: &mut World, _ctx: &EffectCtx, p: &serde_json::Value) {
    let zone = p.get("zone").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let faction = p.get("faction").and_then(|v| v.as_str()).map(str::to_string);
    let count = p.get("count").and_then(|v| v.as_u64()).unwrap_or(4) as usize;
    let max_open = p.get("max_open").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    let content = world.resource::<Content>().clone();
    let Some(job) = content.jobs.values().find(|j| j.handler == "dig").map(|j| j.id.clone()) else { return };
    let open = world.resource::<sim_core::jobs::JobBoard>().jobs.values().filter(|j| j.job == job && j.faction == faction).count();
    if open >= max_open {
        return;
    }
    let map = world.resource::<WorldMap>().clone();
    // Only rock next to floor connected to the entrance (`from`: [x, y]), never isolated cave pockets.
    let from = p.get("from").and_then(|v| v.as_array()).map(|a| (a[0].as_i64().unwrap_or(3) as i32, a[1].as_i64().unwrap_or(3) as i32)).unwrap_or((3, 3));
    let mut reach = std::collections::BTreeSet::new();
    let mut frontier = Vec::new();
    for zi in map.resolve_zones(&zone) {
        let z = &map.zones[zi];
        let start = Position::new(z.layer, z.x + from.0, z.y + from.1);
        let mut queue = std::collections::VecDeque::from([start]);
        reach.insert(start);
        while let Some(c) = queue.pop_front() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let n = Position::new(c.layer, c.x + dx, c.y + dy);
                if map.tile(&n).is_some() && !map.blocked(&n) && reach.insert(n) {
                    queue.push_back(n);
                }
            }
        }
        for y in z.y..z.y + z.h {
            for x in z.x..z.x + z.w {
                let cell = Position::new(z.layer, x, y);
                if !map.tile(&cell).is_some_and(|c| map.diggable.contains_key(&c)) {
                    continue;
                }
                let touches_floor = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|(dx, dy)| reach.contains(&Position::new(z.layer, x + dx, y + dy)));
                if touches_floor {
                    frontier.push(cell);
                }
            }
        }
    }
    let taken: std::collections::BTreeSet<Position> = world
        .resource::<sim_core::jobs::JobBoard>()
        .jobs
        .values()
        .filter_map(|j| match j.target {
            sim_core::jobs::JobTarget::Cell(c) => Some(c),
            _ => None,
        })
        .collect();
    frontier.retain(|c| !taken.contains(c));
    for _ in 0..count.min(max_open - open) {
        let Some(i) = world.resource_mut::<SimRng>().index(frontier.len()) else { break };
        let cell = frontier.swap_remove(i);
        sim_core::jobs::post_job(world, &job, faction.clone(), sim_core::jobs::JobTarget::Cell(cell), 0, None);
    }
}

/// The subject climbs on the nearest mount of its faction (`{"template": "dinosauro"}`): the mount
/// sticks to the rider (strict follow) until `dismount`.
fn mount(world: &mut World, ctx: &EffectCtx, p: &serde_json::Value) {
    let Some(rider) = ctx.subject else { return };
    let template = p.get("template").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let (Some(pos), faction) = (world.get::<Position>(rider).copied(), world.get::<FactionMember>(rider).map(|m| m.faction.clone())) else { return };
    let Some(rider_id) = world.get::<SimId>(rider).copied() else { return };
    let mount = sim_core::sorted_entities::<TemplateId>(world).into_iter().find(|e| {
        world.get::<TemplateId>(*e).is_some_and(|t| t.0 == template)
            && world.get::<Dead>(*e).is_none()
            && world.get::<FactionMember>(*e).map(|m| m.faction.clone()) == faction
            && world.get::<Position>(*e).is_some_and(|q| q.within(&pos, 3))
    });
    if let Some(m) = mount {
        sim_core::jobs::release_task(world, m);
        world.entity_mut(m).insert(sim_core::player::Follow { target: rider_id, distance: 0, strict: true });
        world.entity_mut(m).remove::<sim_core::dungeon::Tethered>();
    }
}

fn dismount(world: &mut World, ctx: &EffectCtx, _p: &serde_json::Value) {
    let Some(rider) = ctx.subject.and_then(|e| world.get::<SimId>(e).copied()) else { return };
    for e in sim_core::sorted_entities::<sim_core::player::Follow>(world) {
        if world.get::<sim_core::player::Follow>(e).is_some_and(|f| f.strict && f.target == rider) {
            world.entity_mut(e).remove::<sim_core::player::Follow>();
        }
    }
}

/// `{"template": "...", "range": n}`: a living entity of that template of the subject's faction is close.
fn near(world: &mut World, ctx: &EffectCtx, p: &serde_json::Value) -> bool {
    let Some(me) = ctx.subject else { return false };
    let template = p.get("template").and_then(|v| v.as_str()).unwrap_or_default();
    let range = p.get("range").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
    let (Some(pos), faction) = (world.get::<Position>(me).copied(), world.get::<FactionMember>(me).map(|m| m.faction.clone())) else {
        return false;
    };
    let mut q = world.query_filtered::<(&TemplateId, &Position, Option<&FactionMember>), Without<Dead>>();
    q.iter(world).any(|(t, p, m)| t.0 == template && p.within(&pos, range) && m.map(|m| m.faction.clone()) == faction)
}

/// The hacker rewrites the firmware: the robot or drone joins the hacker's faction.
fn hack(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = ctx.target.filter(|t| world.get::<Dead>(*t).is_none()) else { return JobResult::fail("nessuna macchina") };
    let is_machine = world
        .get::<sim_core::stats::Tags>(t)
        .is_some_and(|tags| tags.has("robot") || tags.has("drone"));
    if !is_machine {
        return JobResult::fail("non è una macchina");
    }
    let Some(f) = world.get::<FactionMember>(ctx.actor).map(|m| m.faction.clone()) else { return JobResult::fail("senza fazione") };
    sim_core::social::change_faction(world, t, &f, "firmware riscritto da un hacker");
    world.entity_mut(t).remove::<sim_core::dungeon::Tethered>();
    JobResult::ok_msg(format!(
        "{} hackera {}",
        sim_core::infiltration::apparent_name(world, ctx.actor),
        sim_core::infiltration::apparent_name(world, t)
    ))
}

/// The simple web client served on `/ui/` (see `client/index.html`).
/// The Unity client's sprites (tiles, chibi sheets, buildings), served to the admin console when the
/// repository layout is there.
pub fn sprites_dir() -> Option<std::path::PathBuf> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../unity-client/Assets/Resources/Sprites");
    p.is_dir().then_some(p)
}

pub fn client_html() -> &'static str {
    include_str!("../client/index.html")
}

/// Races that can have children together (besides each race with itself).
const MIXED_RACES: &[&str] = &["fidentino", "salsese", "nano"];

/// Whether subject and target can conceive: Rettiliani are asexual and breed with any Rettiliano; Salsesi,
/// Fidentini and Nani breed with each other; the others within their race. Outside the Rettiliani it takes
/// a man and a woman; machines never.
fn can_conceive(world: &World, a: Entity, b: Entity) -> bool {
    use sim_core::stats::{Race, Sex};
    let (Some(ra), Some(rb)) = (world.get::<Race>(a), world.get::<Race>(b)) else { return false };
    let content = world.resource::<Content>();
    let sexless = |r: &str| content.races.get(r).is_none_or(|d| d.sexless);
    if sexless(&ra.0) || sexless(&rb.0) {
        return false;
    }
    if ra.0 == "rettiliano" || rb.0 == "rettiliano" {
        return ra.0 == rb.0;
    }
    let races_ok = ra.0 == rb.0 || (MIXED_RACES.contains(&ra.0.as_str()) && MIXED_RACES.contains(&rb.0.as_str()));
    let sexes = (world.get::<Sex>(a).copied(), world.get::<Sex>(b).copied());
    races_ok && matches!(sexes, (Some(Sex::Male), Some(Sex::Female)) | (Some(Sex::Female), Some(Sex::Male)))
}

fn fertile_couple(world: &mut World, ctx: &EffectCtx, _p: &serde_json::Value) -> bool {
    match (ctx.subject, ctx.target) {
        (Some(a), Some(b)) => can_conceive(world, a, b),
        _ => false,
    }
}

/// After sex: maybe a child is born (fertility of both; a condom makes it rare). It takes the mother's race
/// (or the subject's, for Rettiliani) and faction.
fn conceive(world: &mut World, ctx: &EffectCtx, _p: &serde_json::Value) {
    use sim_core::stats::{Race, Sex};
    let (Some(a), Some(b)) = (ctx.subject, ctx.target) else { return };
    if !can_conceive(world, a, b) {
        return;
    }
    let fert = |e: Entity| world.get::<Stats>(e).map_or(50.0, |s| s.get("fertilita"));
    let mut chance = (fert(a) + fert(b)) / 200.0 * 0.15;
    for e in [a, b] {
        if sim_core::inventory_ops::count(world, e, "preservativo") > 0 {
            sim_core::inventory_ops::take(world, e, "preservativo", 1);
            chance *= 0.05;
            break;
        }
    }
    if !world.resource_mut::<SimRng>().chance(chance) {
        return;
    }
    let mother = if world.get::<Sex>(b).copied() == Some(Sex::Female) { b } else { a };
    let race = world.get::<Race>(mother).map(|r| r.0.clone()).unwrap_or_default();
    let content = world.resource::<Content>().clone();
    let Some(t) = content
        .templates
        .values()
        .filter(|t| t.race == race && !t.unique && !t.virtual_entity)
        .min_by_key(|t| (t.classes != ["normie"], t.id.clone()))
        .map(|t| t.id.clone())
    else {
        return;
    };
    let pos = world.get::<Position>(mother).copied();
    let faction = world.get::<FactionMember>(mother).map(|m| m.faction.clone());
    let mname = sim_core::effects::name_of(world, mother);
    let ov = sim_core::lifecycle::SpawnOverrides { name: Some(format!("Figlio di {mname}")), faction, ..Default::default() };
    if let Some(child) = sim_core::lifecycle::spawn_template(world, &t, pos, &ov) {
        let bounds = content.stat_bounds("eta");
        if let Some(mut s) = world.get_mut::<Stats>(child) {
            s.set_base("eta", 0.0, bounds);
            s.set_base("verginita", 1.0, (0.0, 1.0));
        }
        let tick = world.resource::<SimClock>().tick;
        let id = world.get::<SimId>(child).copied();
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new("birth", format!("Fiocco azzurro e rosa: è nato il figlio di {mname}")).target(id).pos(pos).news(0.6).tags(["nascita"]),
        );
    }
}

/// Where an item is: the pawn or building holding it and the zone it is in.
fn locate_item(world: &mut World, item: &str) -> Option<(String, Option<String>)> {
    let map = world.resource::<WorldMap>().clone();
    let mut found = None;
    for e in sim_core::sorted_entities::<SimId>(world) {
        if world.get::<Dead>(e).is_some() || sim_core::inventory_ops::count(world, e, item) == 0 {
            continue;
        }
        let who = sim_core::infiltration::apparent_name(world, e);
        let zone = world.get::<Position>(e).and_then(|p| map.zone_name_at(p).map(String::from));
        found = Some((who, zone));
        break;
    }
    found
}

/// The passages (doors, stairs, manholes) along the way from `from` to a zone, and how many steps.
fn route(world: &World, from: Position, zone: &str) -> Option<(Vec<String>, usize)> {
    let map = world.resource::<WorldMap>();
    let z = map.zone(zone)?;
    let goal = z.center();
    let max = world.resource::<Params>().get("move.max_path_nodes", 20000.0) as usize * 4;
    let path = map.find_path(from, goal, (z.w.min(z.h) / 2).max(0), &|_| true, max)?;
    let mut doors = Vec::new();
    let mut prev = from;
    for p in &path {
        if p.layer != prev.layer
            && let Some(portal) = map.portals.iter().find(|x| (x.a == prev && x.b == *p) || (x.b == prev && x.a == *p))
        {
            doors.push(portal.name.clone());
        }
        prev = *p;
    }
    Some((doors, path.len()))
}

/// `{"topic": "reliquia" | "strada" | "infiltrati" | "ricercati" | "ruoli", "item": id, "tag": tag, "zone": id}`:
/// the subject investigates and writes what it found in its journal. Perception helps.
fn investigate(world: &mut World, ctx: &EffectCtx, p: &serde_json::Value) {
    let Some(me) = ctx.subject else { return };
    let topic = p.get("topic").and_then(|v| v.as_str()).unwrap_or("reliquia");
    let content = world.resource::<Content>().clone();
    let perception = world.get::<Stats>(me).map_or(10.0, |s| s.get("percezione"));
    let text = match topic {
        "reliquia" => {
            let tag = p.get("tag").and_then(|v| v.as_str()).unwrap_or("reliquia_maggiore");
            let items: Vec<String> = match p.get("item").and_then(|v| v.as_str()) {
                Some(i) => vec![i.to_string()],
                None => content.items.values().filter(|i| i.tags.iter().any(|t| t == tag)).map(|i| i.id.clone()).collect(),
            };
            let Some(item) = world.resource_mut::<SimRng>().pick(&items).cloned() else { return };
            let name = content.items.get(&item).map_or(item.clone(), |d| d.name.clone());
            match locate_item(world, &item) {
                Some((who, Some(zone))) => format!("Indagine: {name} ce l'ha {who}, in {zone}"),
                Some((who, None)) => format!("Indagine: {name} ce l'ha {who}"),
                None => format!("Indagine: di {name} non c'è traccia, forse è andata perduta"),
            }
        }
        "strada" => {
            let zone = p.get("zone").and_then(|v| v.as_str()).unwrap_or("gallerie");
            let zname = content.zone(zone).map_or(zone.to_string(), |z| z.name.clone());
            let Some(from) = world.get::<Position>(me).copied() else { return };
            match route(world, from, zone) {
                Some((doors, _)) if doors.is_empty() => format!("Indagine: per {zname} non servono passaggi, basta camminare"),
                Some((doors, steps)) => format!("Indagine: per arrivare a {zname} passa da {} (circa {steps} passi)", doors.join(", poi ")),
                None => format!("Indagine: nessuna strada nota per {zname}; forse bisogna scavare"),
            }
        }
        "infiltrati" => {
            let Some(pos) = world.get::<Position>(me).copied() else { return };
            let radius = 6 + (perception / 3.0) as i32;
            let found: Vec<Entity> = sim_core::sorted_entities::<Disguise>(world)
                .into_iter()
                .filter(|e| world.get::<Dead>(*e).is_none() && world.get::<Position>(*e).is_some_and(|p| p.within(&pos, radius)))
                .collect();
            if found.is_empty() {
                "Indagine: qui intorno nessuno sembra travestito".to_string()
            } else {
                let names: Vec<String> = found
                    .iter()
                    .map(|e| {
                        let real = sim_core::effects::name_of(world, *e);
                        let shown = sim_core::infiltration::apparent_name(world, *e);
                        format!("{shown} è in realtà {real}")
                    })
                    .collect();
                format!("Indagine: {}", names.join("; "))
            }
        }
        "ricercati" => {
            let mut wanted: Vec<(f32, String)> = Vec::new();
            for e in sim_core::sorted_entities::<Wanted>(world) {
                let lvl = world.get::<Wanted>(e).map_or(0.0, |w| w.level);
                if lvl >= 1.0 && world.get::<Dead>(e).is_none() {
                    wanted.push((lvl, sim_core::infiltration::apparent_name(world, e)));
                }
            }
            wanted.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            if wanted.is_empty() {
                "Indagine: al momento nessuno è ricercato".to_string()
            } else {
                let list: Vec<String> = wanted.iter().take(5).map(|(l, n)| format!("{n} ({l:.0})")).collect();
                format!("Indagine: i più ricercati sono {}", list.join(", "))
            }
        }
        "ruoli" => {
            let mut parts = Vec::new();
            for t in content.titles.values() {
                let who = sim_core::titles::holder_entity(world, &t.id).map(|h| sim_core::infiltration::apparent_name(world, h));
                parts.push(format!("{}: {}", t.name, who.unwrap_or_else(|| "vacante".into())));
            }
            format!("Indagine sui ruoli: {}", parts.join("; "))
        }
        _ => return,
    };
    sim_core::stats::write_journal(world, me, text.clone());
    let tick = world.resource::<SimClock>().tick;
    let (id, pos) = (world.get::<SimId>(me).copied(), world.get::<Position>(me).copied());
    let faction = world.get::<FactionMember>(me).map(|m| m.faction.clone());
    world.resource_mut::<EventLog>().push(tick, EventBuilder::new("investigation", text).actor(id).pos(pos).faction(faction).tags(["indagine"]));
}
