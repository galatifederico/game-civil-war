//! Events (things that happen by themselves: spawns, news, triggers) and tethering.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{Content, Tether};
use crate::effects::{apply_effects, eval_condition, EffectCtx};
use crate::events::{kind, EventBuilder, EventLog};
use crate::ids::SimId;
use crate::map::{Position, WorldMap};
use crate::stats::{Dead, TemplateId};
use crate::time::SimClock;

/// Keeps a pawn inside a zone until its release condition holds.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Tethered(pub Tether);

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct SpawnedBy(pub String);

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriggerState {
    /// Event id → last tick it happened.
    pub fired: BTreeMap<String, u64>,
    /// Source → how many times its `Cycle` effects ran (which one comes next).
    #[serde(default)]
    pub cycles: BTreeMap<String, u64>,
}

pub fn release(world: &mut World, e: Entity, reason: &str) {
    if world.entity_mut(e).take::<Tethered>().is_none() {
        return;
    }
    let tick = world.resource::<SimClock>().tick;
    let (id, name) = (world.get::<SimId>(e).copied(), crate::effects::name_of(world, e));
    let pos = world.get::<Position>(e).copied();
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::RELEASED, format!("{name} esce in superficie ({reason})"))
            .target(id)
            .pos(pos)
            .news(0.7)
            .tags(["dungeon", "surface_raid"]),
    );
}

/// Tether upkeep: pull strays back, release when the condition holds.
pub fn tethers(world: &mut World) {
    let map = world.resource::<WorldMap>().clone();
    for e in crate::sorted_entities::<Tethered>(world) {
        if world.get::<Dead>(e).is_some() {
            continue;
        }
        let t = world.get::<Tethered>(e).unwrap().0.clone();
        if eval_condition(world, &EffectCtx::new(Some(e), None, "tether"), &t.release) {
            release(world, e, "condizione di uscita");
            continue;
        }
        if let Some(p) = world.get::<Position>(e).copied()
            && !map.in_zone(&t.zone, &p)
                && let Some(z) = map.resolve_zones(&t.zone).first() {
                    world.entity_mut(e).insert(map.zones[*z].clamp(&p));
                }
    }
}

/// Events whose time has come and whose condition holds happen (once, or every `every` ticks).
pub fn events(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let events: Vec<_> = world.resource::<Content>().events.values().cloned().collect();
    for ev in events {
        if tick < ev.from_tick {
            continue;
        }
        match world.resource::<TriggerState>().fired.get(&ev.id).copied() {
            Some(_) if ev.every == 0 => continue,
            Some(l) if tick < l + ev.every => continue,
            _ => {}
        }
        let by = match &ev.by {
            Some(t) => match by_template(world, t) {
                Some(e) => Some(e),
                None => continue,
            },
            None => None,
        };
        let ctx = EffectCtx::new(by, None, format!("event:{}", ev.id));
        if !eval_condition(world, &ctx, &ev.when) {
            continue;
        }
        happen(world, &ev, &ctx, false);
    }
}

fn by_template(world: &mut World, template: &str) -> Option<Entity> {
    crate::sorted_entities::<TemplateId>(world)
        .into_iter()
        .find(|e| world.get::<TemplateId>(*e).is_some_and(|t| t.0 == template) && world.get::<Dead>(*e).is_none())
}

fn happen(world: &mut World, ev: &crate::content::EventDef, ctx: &EffectCtx, forced: bool) {
    let tick = world.resource::<SimClock>().tick;
    world.resource_mut::<TriggerState>().fired.insert(ev.id.clone(), tick);
    // Named events go in the log (spawns and news speak for themselves).
    if !ev.name.is_empty() {
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new(kind::TRIGGER, if forced { format!("{} (forzato)", ev.name) } else { ev.name.clone() })
                .news(ev.news)
                .tags(["trigger", ev.id.as_str()]),
        );
    }
    apply_effects(world, ctx, &ev.effects);
}

/// Makes an event happen now, ignoring its condition and timing (admin/MCP `trigger_event`).
pub fn fire_trigger(world: &mut World, id: &str) -> bool {
    let Some(ev) = world.resource::<Content>().events.get(id).cloned() else { return false };
    let by = ev.by.as_deref().and_then(|t| by_template(world, t));
    let ctx = EffectCtx::new(by, None, format!("event:{id}"));
    happen(world, &ev, &ctx, true);
    true
}
