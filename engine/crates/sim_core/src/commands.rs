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
    Publish { headline: String, #[serde(default)] truth: Truth, #[serde(default)] topics: Vec<String>, #[serde(default)] author: Option<SimId> },
}

fn one() -> u32 {
    1
}

#[derive(Resource, Debug, Default)]
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
            if crate::dungeon::fire_trigger(world, &id) { Ok(format!("trigger {id} eseguito")) } else { Err(format!("trigger '{id}' inesistente")) }
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
                    let bj = BoardJob { id: 0, job: job.clone(), faction, target, priority, reserved_by: None, created: tick, posted_by: None, recipe: None };
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
            if crate::abilities::use_ability(world, e, &ability, t) { Ok(format!("{ability} usata")) } else { Err("abilità non disponibile".into()) }
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
        SimCommand::SetSprite { id, sprite } => {
            world.resource_mut::<SpriteMapping>().0.insert(id.clone(), sprite);
            Ok(format!("sprite di {id} aggiornato"))
        }
        SimCommand::Publish { headline, truth, topics, author } => {
            let a = author.map(|i| entity(world, i)).transpose()?;
            let id = crate::press::publish(world, a, headline, truth, topics, None, None);
            Ok(format!("articolo {id} pubblicato"))
        }
    }
}
