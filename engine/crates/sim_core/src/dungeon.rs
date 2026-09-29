//! Spawning & Tethering System and Trigger Conditions Engine.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{Content, Tether};
use crate::effects::{apply_effects, eval_condition, EffectCtx};
use crate::events::{kind, EventBuilder, EventLog};
use crate::ids::SimId;
use crate::lifecycle::{spawn_template, SpawnOverrides};
use crate::map::{Position, WorldMap};
use crate::rng::SimRng;
use crate::stats::{Dead, TemplateId};
use crate::time::SimClock;

/// Keeps a pawn inside a zone until its release condition holds.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Tethered(pub Tether);

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct SpawnedBy(pub String);

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriggerState {
    pub fired: BTreeMap<String, u64>,
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

pub fn spawners(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let spawners: Vec<_> = world.resource::<Content>().spawners.values().cloned().collect();
    for s in spawners {
        if s.interval == 0 || !tick.is_multiple_of(s.interval) {
            continue;
        }
        if let Some(summoner) = &s.summoner {
            let alive = crate::sorted_entities::<TemplateId>(world)
                .into_iter()
                .any(|e| world.get::<TemplateId>(e).is_some_and(|t| &t.0 == summoner) && world.get::<Dead>(e).is_none());
            if !alive {
                continue;
            }
        }
        if !eval_condition(world, &EffectCtx::new(None, None, format!("spawner:{}", s.id)), &s.active_when) {
            continue;
        }
        let alive = {
            let mut q = world.query_filtered::<&SpawnedBy, Without<Dead>>();
            q.iter(world).filter(|x| x.0 == s.id).count() as u32
        };
        if s.max_alive > 0 && alive >= s.max_alive {
            continue;
        }
        let map = world.resource::<WorldMap>().clone();
        let pos = map.random_cell(&s.zone, &mut world.resource_mut::<SimRng>());
        let ov = SpawnOverrides { faction: s.faction.clone(), tether: s.tether.clone(), ..Default::default() };
        if let Some(e) = spawn_template(world, &s.template, pos, &ov) {
            world.entity_mut(e).insert(SpawnedBy(s.id.clone()));
        }
    }
}

pub fn triggers(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let triggers: Vec<_> = world.resource::<Content>().triggers.values().cloned().collect();
    for t in triggers {
        let last = world.resource::<TriggerState>().fired.get(&t.id).copied();
        match last {
            Some(_) if !t.repeat => continue,
            Some(l) if tick < l + t.cooldown.max(1) => continue,
            _ => {}
        }
        let ctx = EffectCtx::new(None, None, format!("trigger:{}", t.id));
        if !eval_condition(world, &ctx, &t.when) {
            continue;
        }
        world.resource_mut::<TriggerState>().fired.insert(t.id.clone(), tick);
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new(kind::TRIGGER, if t.name.is_empty() { t.id.clone() } else { t.name.clone() })
                .news(t.news)
                .tags(["trigger", t.id.as_str()]),
        );
        apply_effects(world, &ctx, &t.effects);
    }
}

/// Fires a trigger now, ignoring its condition (admin/MCP `trigger_event`).
pub fn fire_trigger(world: &mut World, id: &str) -> bool {
    let Some(t) = world.resource::<Content>().triggers.get(id).cloned() else { return false };
    let tick = world.resource::<SimClock>().tick;
    world.resource_mut::<TriggerState>().fired.insert(t.id.clone(), tick);
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::TRIGGER, format!("{} (forzato)", if t.name.is_empty() { &t.id } else { &t.name }))
            .news(t.news)
            .tags(["trigger", t.id.as_str()]),
    );
    apply_effects(world, &EffectCtx::new(None, None, format!("trigger:{id}")), &t.effects);
    true
}
