//! # fidenza_world
//!
//! The satirical world of Fidenza and Salsomaggiore as a `sim_core` plugin. Almost everything is data
//! (`data/*.ron`); this crate only adds what data alone cannot express:
//!
//! - effect `oracle_assign`: the Oracle joins a random faction at tick 0;
//! - effect `oracle_reveal`: the Oracle reveals a map secret (a disguised infiltrator or where a relic is);
//! - effect `borgazzi_masterpiece`: Gerolamo Borgazzi paints the next of his unique artworks;
//! - condition `intruders`: pawns of other factions inside a zone (dungeon exit conditions);
//! - job handler `hack`: a hacker takes control of a robot or drone.

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
