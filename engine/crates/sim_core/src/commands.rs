//! Every input from outside the simulation (player UI, admin panel, MCP) is a [`SimCommand`] queued and
//! applied at the start of the next tick, which keeps runs replayable.

use std::collections::{BTreeMap, VecDeque};

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{Content, Effect, SpriteDef, Truth};
use crate::effects::{apply_effect, EffectCtx};
use crate::events::{kind, EventBuilder, EventLog};
use crate::factions::{FactionMember, Factions};
use crate::ids::{IdIndex, SimId};
use crate::jobs::{release_task, start_job, BoardJob, JobTarget, PersonalQueue, Task, WorkPriorities};
use crate::lifecycle::SpawnOverrides;
use crate::map::WorldMap;
use crate::params::Params;
use crate::rng::SimRng;
use crate::squads::{SquadOrder, Squads};
use crate::time::SimClock;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SimCommand {
    /// Pay the police from a faction's guild treasury to clear a pawn's charges (amount None = exact cost).
    Bribe { faction: String, target: SimId, #[serde(default)] amount: Option<f64> },
    SetParam { key: String, value: f64 },
    SetWorkPriority { entity: SimId, work_type: String, priority: u8 },
    /// The player's order: the mode of one pawn (`entity`) or of a whole faction (`faction`); no `mode` clears it.
    SetMode { #[serde(default)] entity: Option<SimId>, #[serde(default)] faction: Option<String>, #[serde(default)] mode: Option<String> },
    CreateSquad { name: String, #[serde(default)] faction: Option<String>, members: Vec<SimId>, #[serde(default)] role: String },
    SquadOrder { squad: u64, order: Option<SquadOrder> },
    SetSalary { faction: String, rank: String, amount: f64 },
    SetShopPrice { shop: SimId, item: String, price: Option<f64> },
    Spawn { template: String, #[serde(default)] zone: Option<String>, #[serde(default = "one")] count: u32, #[serde(default)] faction: Option<String>, #[serde(default)] name: Option<String> },
    FireTrigger { id: String },
    ApplyEffect { #[serde(default)] subject: Option<SimId>, #[serde(default)] target: Option<SimId>, effect: Effect },
    PostJob { job: String, #[serde(default)] faction: Option<String>, #[serde(default)] target: Option<SimId>, #[serde(default)] priority: i32, #[serde(default)] assignee: Option<SimId> },
    /// Direct order to one pawn: do this job now (move to a zone with job "move").
    Order { entity: SimId, job: String, #[serde(default)] target: Option<SimId>, #[serde(default)] zone: Option<String> },
    UseAbility { entity: SimId, ability: String, #[serde(default)] target: Option<SimId> },
    Promote { entity: SimId, rank: String },
    SetRelation { a: String, b: String, value: f32 },
    SetSprite { id: String, sprite: SpriteDef },
    /// Admin: adds or replaces one content definition (`kind` as in the packs: "races", "items"…)
    /// while the game runs; saved to the overrides file of the content folder.
    EditContent { kind: String, def: serde_json::Value },
    /// A player's order to its champion or to a member of its faction (members may refuse).
    PlayerOrder { player: String, entity: SimId, order: crate::player::Order },
    /// Walks the player's champion one cell right away (Pokémon-style, outside the tick); a passage
    /// (door, stairs) under the new cell is crossed.
    PlayerStep { player: String, dx: i32, dy: i32 },
    /// The champion talks to a pawn next to it: the answer is one of the content's stock phrases.
    PlayerTalk { player: String, target: SimId },
    /// The player's champion takes one of the classes it meets the requirements of.
    PlayerTakeClass { player: String, class: String },
    /// The player's champion challenges the holder of a role (or claims it if vacant).
    PlayerChallengeRole { player: String, title: String },
    /// Hands an item held by the team (members, then faction buildings) to one of its members.
    PlayerGiveItem { player: String, item: String, to: SimId, #[serde(default = "one")] qty: u32 },
    /// A member of the team uses an item (handed one by the team if it carries none).
    PlayerUseItem { player: String, entity: SimId, item: String },
    /// A player proposes an alliance: accepted when the other faction's relation is at least
    /// `social.alliance_threshold` (players accept each other's automatically).
    PlayerAlliance { player: String, faction: String },
    /// Dwarf-Fortress style designation: a job (default "dig") for every suitable cell of an area,
    /// posted on the board for the player's (or the given) faction.
    Designate {
        #[serde(default)] player: Option<String>,
        #[serde(default)] faction: Option<String>,
        layer: String,
        rect: (i32, i32, i32, i32),
        #[serde(default)] job: Option<String>,
    },
    /// Admin: paint a tile.
    SetTile { layer: String, x: i32, y: i32, tile: char },
    /// Admin: put a building on a cell (its door/anchor).
    PlaceBuilding { building: String, layer: String, x: i32, y: i32, #[serde(default)] owner_faction: Option<String>, #[serde(default)] name: Option<String> },
    /// Admin: move an entity to a cell.
    MoveEntity { entity: SimId, layer: String, x: i32, y: i32 },
    /// Admin: remove an entity (pawn or building) from the world.
    Remove { entity: SimId },
    /// A player's order to one of its squads.
    PlayerSquadOrder { player: String, squad: u64, order: Option<SquadOrder> },
    /// A player creates a squad from members of its faction (the champion leads it).
    PlayerCreateSquad { player: String, name: String, members: Vec<SimId> },
    Publish { headline: String, #[serde(default)] truth: Truth, #[serde(default)] topics: Vec<String>, #[serde(default)] author: Option<SimId> },
}

fn one() -> u32 {
    1
}

#[derive(Resource, Debug, Default, Serialize, Deserialize)]
pub struct CommandQueue {
    pub pending: VecDeque<(u64, SimCommand)>,
    next: u64,
}

impl CommandQueue {
    pub fn push(&mut self, c: SimCommand) -> u64 {
        self.next += 1;
        self.pending.push_back((self.next, c));
        self.next
    }
}

/// Results of applied commands, by sequence number (kept for the last 256).
#[derive(Resource, Debug, Default, Clone, Serialize)]
pub struct CommandResults(pub BTreeMap<u64, Result<String, String>>);

/// Sprite/UI mapping for the client (editable live from the admin panel).
#[derive(Resource, Debug, Default, Clone, Serialize, Deserialize)]
pub struct SpriteMapping(pub BTreeMap<String, SpriteDef>);

fn entity(world: &World, id: SimId) -> Result<Entity, String> {
    world.resource::<IdIndex>().get(id).ok_or_else(|| format!("entità {id} inesistente"))
}

pub fn apply_commands(world: &mut World) {
    let cmds: Vec<(u64, SimCommand)> = world.resource_mut::<CommandQueue>().pending.drain(..).collect();
    for (seq, cmd) in cmds {
        let res = apply(world, cmd.clone());
        let tick = world.resource::<SimClock>().tick;
        let summary = serde_json::to_string(&cmd).unwrap_or_default();
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new(kind::COMMAND, match &res {
                Ok(m) => format!("comando #{seq}: {m}"),
                Err(e) => format!("comando #{seq} rifiutato: {e}"),
            })
            .data(serde_json::json!({ "command": serde_json::from_str::<serde_json::Value>(&summary).ok(), "ok": res.is_ok() })),
        );
        let mut r = world.resource_mut::<CommandResults>();
        r.0.insert(seq, res);
        while r.0.len() > 256 {
            let first = *r.0.keys().next().unwrap();
            r.0.remove(&first);
        }
    }
}

pub fn apply(world: &mut World, cmd: SimCommand) -> Result<String, String> {
    match cmd {
        SimCommand::Bribe { faction, target, amount } => {
            let e = entity(world, target)?;
            crate::crime::bribe(world, &faction, e, amount).map(|paid| format!("tangente di {paid:.0} pagata"))
        }
        SimCommand::SetParam { key, value } => {
            let old = world.resource_mut::<Params>().set(key.clone(), value);
            Ok(format!("{key} = {value} (prima {})", old.map_or("non impostato".into(), |v| v.to_string())))
        }
        SimCommand::SetMode { entity: id, faction, mode } => {
            if let Some(m) = &mode
                && !world.resource::<Content>().modes.contains_key(m)
            {
                return Err(format!("modalità '{m}' inesistente"));
            }
            let what = mode.clone().unwrap_or_else(|| "la sua".into());
            let msg = match (id, faction) {
                (Some(id), _) => {
                    let e = entity(world, id)?;
                    let mut m = world.get::<crate::modes::Mode>(e).cloned().unwrap_or_default();
                    m.ordered = mode;
                    world.entity_mut(e).insert(m);
                    format!("{id} ora in modalità {what}")
                }
                (None, Some(f)) => {
                    let mut fs = world.resource_mut::<Factions>();
                    let st = fs.states.get_mut(&f).ok_or_else(|| format!("fazione '{f}' inesistente"))?;
                    st.mode = mode;
                    format!("fazione {f} ora in modalità {what}")
                }
                (None, None) => return Err("serve una pedina o una fazione".into()),
            };
            crate::modes::update(world);
            Ok(msg)
        }
        SimCommand::SetWorkPriority { entity: id, work_type, priority } => {
            let e = entity(world, id)?;
            let mut w = world.get_mut::<WorkPriorities>(e).ok_or("nessuna matrice di lavoro")?;
            w.overrides.insert(work_type.clone(), priority.min(4));
            Ok(format!("priorità {work_type} = {priority}"))
        }
        SimCommand::CreateSquad { name, faction, members, role } => {
            let id = world.resource_mut::<Squads>().create(name.clone(), faction, members, role);
            Ok(format!("squadra {id} '{name}' creata"))
        }
        SimCommand::SquadOrder { squad, order } => {
            let members = {
                let mut s = world.resource_mut::<Squads>();
                let sq = s.squads.get_mut(&squad).ok_or("squadra inesistente")?;
                sq.order = order.clone();
                sq.members.clone()
            };
            for m in members {
                if let Ok(e) = entity(world, m) {
                    release_task(world, e);
                }
            }
            Ok(format!("ordine alla squadra {squad}: {order:?}"))
        }
        SimCommand::SetSalary { faction, rank, amount } => {
            let mut f = world.resource_mut::<Factions>();
            let s = f.states.get_mut(&faction).ok_or("fazione inesistente")?;
            s.salaries.insert(rank.clone(), amount);
            Ok(format!("stipendio {faction}/{rank} = {amount}"))
        }
        SimCommand::SetShopPrice { shop, item, price } => {
            let e = entity(world, shop)?;
            let mut s = world.get_mut::<crate::buildings::Shop>(e).ok_or("non è un negozio")?;
            s.catalog.insert(item.clone(), price);
            Ok(format!("prezzo di {item} = {price:?}"))
        }
        SimCommand::Spawn { template, zone, count, faction, name } => {
            if !world.resource::<Content>().templates.contains_key(&template) {
                return Err(format!("template '{template}' inesistente"));
            }
            let map = world.resource::<WorldMap>().clone();
            let zone = zone.unwrap_or_else(|| map.zones[0].id.clone());
            let mut ids = Vec::new();
            for _ in 0..count.max(1) {
                let pos = map.random_cell(&zone, &mut world.resource_mut::<SimRng>());
                let ov = SpawnOverrides { name: name.clone(), faction: faction.clone(), ..Default::default() };
                if let Some(e) = crate::lifecycle::spawn_template(world, &template, pos, &ov) {
                    ids.push(world.get::<SimId>(e).unwrap().0);
                }
            }
            if ids.is_empty() { Err("nessuna entità creata (unica già esistente?)".into()) } else { Ok(format!("create {ids:?}")) }
        }
        SimCommand::FireTrigger { id } => {
            if crate::dungeon::fire_trigger(world, &id) { Ok(format!("evento {id} avvenuto")) } else { Err(format!("evento '{id}' inesistente")) }
        }
        SimCommand::ApplyEffect { subject, target, effect } => {
            let s = subject.map(|i| entity(world, i)).transpose()?;
            let t = target.map(|i| entity(world, i)).transpose()?;
            apply_effect(world, &EffectCtx::new(s, t, "command"), &effect);
            Ok("effetto applicato".into())
        }
        SimCommand::PostJob { job, faction, target, priority, assignee } => {
            if !world.resource::<Content>().jobs.contains_key(&job) {
                return Err(format!("job '{job}' inesistente"));
            }
            let target = target.map_or(JobTarget::None, JobTarget::Entity);
            match assignee {
                Some(a) => {
                    let e = entity(world, a)?;
                    let tick = world.resource::<SimClock>().tick;
                    let bj = BoardJob { id: 0, job: job.clone(), faction, target, priority, reserved_by: None, created: tick, posted_by: None, payload: None };
                    world.get_mut::<PersonalQueue>(e).ok_or("nessuna coda")?.0.push_back(bj);
                    Ok(format!("{job} in coda a {a}"))
                }
                None => {
                    let id = crate::jobs::post_job(world, &job, faction, target, priority, None);
                    Ok(format!("job {id} ({job}) pubblicato"))
                }
            }
        }
        SimCommand::Order { entity: id, job, target, zone } => {
            let e = entity(world, id)?;
            if !world.resource::<Content>().jobs.contains_key(&job) {
                return Err(format!("job '{job}' inesistente"));
            }
            let target = match (target, zone) {
                (Some(t), _) => JobTarget::Entity(t),
                (None, Some(z)) => {
                    let map = world.resource::<WorldMap>().clone();
                    JobTarget::Cell(map.random_cell(&z, &mut world.resource_mut::<SimRng>()).ok_or("zona inesistente")?)
                }
                _ => JobTarget::None,
            };
            release_task(world, e);
            start_job(world, e, &job, target, None, None);
            let mut t = world.get_mut::<Task>(e).ok_or("non può agire")?;
            t.forced = true;
            t.label = format!("Ordine: {job}");
            Ok(format!("{id} esegue {job}"))
        }
        SimCommand::UseAbility { entity: id, ability, target } => {
            let e = entity(world, id)?;
            let t = target.map(|i| entity(world, i)).transpose()?;
            // `ability` names an action with effects.
            if crate::abilities::ready(world, e, &ability) && crate::abilities::use_action(world, e, &ability, t) {
                Ok(format!("{ability} usata"))
            } else {
                Err("azione non disponibile".into())
            }
        }
        SimCommand::Promote { entity: id, rank } => {
            let e = entity(world, id)?;
            let f = world.get::<FactionMember>(e).ok_or("senza fazione")?.faction.clone();
            if world.resource::<Content>().rank(&f, &rank).is_none() {
                return Err(format!("rango '{rank}' inesistente in {f}"));
            }
            world.get_mut::<FactionMember>(e).unwrap().rank = rank.clone();
            Ok(format!("{id} promosso a {rank}"))
        }
        SimCommand::SetRelation { a, b, value } => {
            let mut f = world.resource_mut::<Factions>();
            let cur = f.relation(&a, &b);
            f.modify_relation(&a, &b, value - cur);
            Ok(format!("relazione {a}↔{b} = {value}"))
        }
        SimCommand::EditContent { kind, def } => {
            let id = def.get("id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).ok_or("manca l'id")?.to_string();
            let content = world.resource::<Content>().with_def(&kind, def).map_err(|e| e.to_string())?;
            if kind == "jobs"
                && let Some(j) = content.jobs.get(&id)
                && !j.handler.is_empty()
                && !world.resource::<crate::extensions::Extensions>().job_handlers.contains_key(&j.handler) {
                    return Err(format!("job {id}: handler '{}' non registrato", j.handler));
                }
            // Saved in canonical form (defaults filled in), as the engine reads it back.
            let saved = serde_json::to_value(&content).ok().and_then(|v| v.get(&kind)?.get(&id).cloned()).unwrap_or_default();
            world.insert_resource(content);
            match world.resource::<crate::content::ContentOverrides>().0.clone() {
                Some(path) => {
                    crate::content::save_override(&path, &kind, saved)?;
                    Ok(format!("{kind}/{id} aggiornato e salvato in {}", path.file_name().map(|f| f.to_string_lossy()).unwrap_or_default()))
                }
                None => Ok(format!("{kind}/{id} aggiornato (solo in memoria)")),
            }
        }
        SimCommand::SetSprite { id, sprite } => {
            world.resource_mut::<SpriteMapping>().0.insert(id.clone(), sprite);
            Ok(format!("sprite di {id} aggiornato"))
        }
        SimCommand::PlayerAlliance { player, faction } => {
            let mine = world.resource::<crate::factions::Players>().players.get(&player).map(|p| p.faction.clone()).ok_or("giocatore inesistente")?;
            if mine == faction || !world.resource::<Content>().factions.contains_key(&faction) {
                return Err("fazione non valida".into());
            }
            let f = world.resource::<Factions>();
            if f.allied(&mine, &faction) {
                return Ok("già alleati".into());
            }
            let needed = world.resource::<Params>().get("social.alliance_threshold", 40.0) as f32;
            let rel = f.relation(&faction, &mine);
            let other_player = f.states.get(&faction).is_some_and(|s| s.controlled_by.is_some());
            if rel < needed && !other_player {
                return Err(format!("rifiutata: relazione {rel:.0}, serve almeno {needed:.0}"));
            }
            crate::social::ally(world, &mine, &faction);
            Ok("alleanza stretta".into())
        }
        SimCommand::Designate { player, faction, layer, rect, job } => {
            let faction = match (faction, player) {
                (Some(f), _) => Some(f),
                (None, Some(p)) => Some(world.resource::<crate::factions::Players>().players.get(&p).ok_or("giocatore inesistente")?.faction.clone()),
                _ => None,
            };
            let content = world.resource::<Content>().clone();
            let job = job.unwrap_or_else(|| content.jobs.values().find(|j| j.handler == "dig").map(|j| j.id.clone()).unwrap_or_default());
            if !content.jobs.contains_key(&job) {
                return Err("nessun lavoro di scavo nei contenuti".into());
            }
            let map = world.resource::<WorldMap>().clone();
            let li = map.layers.iter().position(|l| l.id == layer).ok_or("mappa inesistente")? as u16;
            let existing: std::collections::BTreeSet<(i32, i32)> = world
                .resource::<crate::jobs::JobBoard>()
                .jobs
                .values()
                .filter_map(|j| match j.target {
                    JobTarget::Cell(p) if p.layer == li && j.job == job => Some((p.x, p.y)),
                    _ => None,
                })
                .collect();
            let mut n = 0;
            for y in rect.1..rect.1 + rect.3.min(64) {
                for x in rect.0..rect.0 + rect.2.min(64) {
                    let p = crate::map::Position::new(li, x, y);
                    if map.tile(&p).is_some_and(|c| map.diggable.contains_key(&c)) && !existing.contains(&(x, y)) {
                        crate::jobs::post_job(world, &job, faction.clone(), JobTarget::Cell(p), 0, None);
                        n += 1;
                    }
                }
            }
            Ok(format!("{n} celle designate per lo scavo"))
        }
        SimCommand::SetTile { layer, x, y, tile } => {
            let map = world.resource::<WorldMap>().clone();
            let li = map.layers.iter().position(|l| l.id == layer).ok_or("mappa inesistente")? as u16;
            if !map.legend.contains_key(&tile) {
                return Err(format!("'{tile}' non è nella legenda"));
            }
            crate::map::change_tile(world, crate::map::Position::new(li, x, y), tile);
            Ok(format!("({x},{y}) = '{tile}'"))
        }
        SimCommand::PlaceBuilding { building, layer, x, y, owner_faction, name } => {
            let content = world.resource::<Content>().clone();
            let def = content.buildings.get(&building).ok_or(format!("edificio '{building}' inesistente"))?;
            let map = world.resource::<WorldMap>().clone();
            let li = map.layers.iter().position(|l| l.id == layer).ok_or("mappa inesistente")? as u16;
            let pos = crate::map::Position::new(li, x, y);
            if map.tile(&pos).is_none() {
                return Err("fuori dalla mappa".into());
            }
            if let Some(f) = &owner_faction
                && !content.factions.contains_key(f)
            {
                return Err(format!("fazione '{f}' inesistente"));
            }
            let owner = owner_faction.map_or(crate::buildings::Owner::None, crate::buildings::Owner::Faction);
            let e = crate::buildings::spawn_building(world, &building, pos, name, owner).ok_or("edificio non creato")?;
            if let Some((w, h)) = def.footprint {
                let cells: Vec<_> = crate::map::footprint_cells(pos, w, h).into_iter().filter(|c| *c != pos).collect();
                world.resource_mut::<WorldMap>().make_mut().block(&cells);
            }
            Ok(format!("{} creato con id {}", def.name, world.get::<SimId>(e).unwrap().0))
        }
        SimCommand::MoveEntity { entity: id, layer, x, y } => {
            let e = entity(world, id)?;
            let map = world.resource::<WorldMap>().clone();
            let li = map.layers.iter().position(|l| l.id == layer).ok_or("mappa inesistente")? as u16;
            let pos = crate::map::Position::new(li, x, y);
            if map.tile(&pos).is_none() {
                return Err("fuori dalla mappa".into());
            }
            crate::jobs::release_task(world, e);
            world.entity_mut(e).insert(pos);
            world.entity_mut(e).remove::<crate::movement::Movement>();
            Ok(format!("spostato in ({x},{y})"))
        }
        SimCommand::Remove { entity: id } => {
            let e = entity(world, id)?;
            if let (Some(b), Some(p)) = (world.get::<crate::buildings::Building>(e).cloned(), world.get::<crate::map::Position>(e).copied())
                && let Some((w, h)) = world.resource::<Content>().buildings.get(&b.def).and_then(|d| d.footprint)
            {
                let cells = crate::map::footprint_cells(p, w, h);
                world.resource_mut::<WorldMap>().make_mut().unblock(&cells);
            }
            crate::jobs::release_task(world, e);
            crate::squads::remove_member(world, e);
            world.resource_mut::<crate::ids::IdIndex>().remove(id);
            world.despawn(e);
            Ok(format!("{} rimosso", id.0))
        }
        SimCommand::PlayerOrder { player, entity, order } => crate::player::give_order(world, &player, entity, order),
        SimCommand::PlayerTakeClass { player, class } => crate::classes::accept(world, &player, &class),
        SimCommand::PlayerChallengeRole { player, title } => crate::titles::player_challenge(world, &player, &title),
        SimCommand::PlayerTalk { player, target } => crate::player::talk(world, &player, target).map(|(n, l)| format!("{n}: «{l}»")),
        SimCommand::PlayerGiveItem { player, item, to, qty } => crate::player::give_item(world, &player, &item, to, qty.max(1)),
        SimCommand::PlayerUseItem { player, entity, item } => crate::player::use_item(world, &player, entity, &item),
        SimCommand::PlayerStep { player, dx, dy } => crate::player::step(world, &player, dx, dy).map(|p| format!("({}, {}, {})", p.layer, p.x, p.y)),
        SimCommand::PlayerSquadOrder { player, squad, order } => {
            let faction = world.resource::<crate::factions::Players>().players.get(&player).map(|p| p.faction.clone()).ok_or("giocatore inesistente")?;
            let own = world.resource::<Squads>().squads.get(&squad).is_some_and(|s| s.faction.as_deref() == Some(faction.as_str()));
            if !own {
                return Err("non è una tua squadra".into());
            }
            apply(world, SimCommand::SquadOrder { squad, order })
        }
        SimCommand::PlayerCreateSquad { player, name, members } => {
            let p = world.resource::<crate::factions::Players>().players.get(&player).cloned().ok_or("giocatore inesistente")?;
            let mut ok = Vec::new();
            for m in members {
                let e = entity(world, m)?;
                if world.get::<FactionMember>(e).is_some_and(|f| f.faction == p.faction) {
                    ok.push(m);
                }
            }
            if ok.is_empty() {
                return Err("nessun membro valido".into());
            }
            let id = world.resource_mut::<Squads>().create(name.clone(), Some(p.faction), ok.clone(), "squadra del giocatore".into());
            if let Some(s) = world.resource_mut::<Squads>().squads.get_mut(&id) {
                s.leader = p.leader;
            }
            Ok(format!("squadra {id} '{name}' con {} membri", ok.len()))
        }
        SimCommand::Publish { headline, truth, topics, author } => {
            let a = author.map(|i| entity(world, i)).transpose()?;
            let id = crate::press::publish(world, a, headline, truth, topics, None, None);
            Ok(format!("articolo {id} pubblicato"))
        }
    }
}
