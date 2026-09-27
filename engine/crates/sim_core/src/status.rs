//! StatusEffectSystem, PathogenEngine and MutationFramework: timed or permanent buffs/debuffs with
//! severity, stages, escalation chains (e.g. tipsy → wasted), immunities and race transmutation.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{Content, Stacking};
use crate::effects::{apply_effects, EffectCtx};
use crate::events::{kind, EventBuilder, EventLog};
use crate::ids::SimId;
use crate::map::Position;
use crate::stats::{Dead, Race};
use crate::time::SimClock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActiveStatus {
    pub severity: f32,
    pub applied: u64,
    /// Ticks left (None = indefinite).
    pub remaining: Option<u64>,
    pub stage: Option<usize>,
}

#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatusEffects {
    pub active: BTreeMap<String, ActiveStatus>,
    /// Status id → tick until which the carrier is immune (u64::MAX = forever).
    pub immunities: BTreeMap<String, u64>,
}

impl StatusEffects {
    pub fn has(&self, id: &str) -> bool {
        self.active.contains_key(id)
    }

    pub fn severity(&self, id: &str) -> f32 {
        self.active.get(id).map_or(0.0, |s| s.severity)
    }

    pub fn has_tag(&self, content: &Content, tag: &str) -> bool {
        self.active.keys().any(|s| content.statuses.get(s).is_some_and(|d| d.tags.iter().any(|t| t == tag)))
    }

    /// Multiplier on AI thinking speed from active statuses.
    pub fn ai_speed(&self, content: &Content) -> f32 {
        self.active
            .keys()
            .filter_map(|s| content.statuses.get(s)?.ai_speed)
            .fold(1.0, |acc, x| acc * x)
    }
}

fn stage_for(def: &crate::content::StatusDef, severity: f32) -> Option<usize> {
    def.stages.iter().enumerate().filter(|(_, s)| severity >= s.at).map(|(i, _)| i).next_back()
}

fn sim_id(world: &World, e: Entity) -> Option<SimId> {
    world.get::<SimId>(e).copied()
}

/// Applies (or re-applies) a status. Returns false when the carrier is immune or the status unknown.
pub fn apply_status(world: &mut World, e: Entity, status: &str, severity: f32, source: Option<Entity>) -> bool {
    let tick = world.resource::<SimClock>().tick;
    let Some(def) = world.resource::<Content>().statuses.get(status).cloned() else { return false };
    if world.get::<Dead>(e).is_some() {
        return false;
    }
    let race_immune = world
        .get::<Race>(e)
        .and_then(|r| world.resource::<Content>().races.get(&r.0))
        .is_some_and(|r| r.immunities.iter().any(|i| i == status));
    let Some(se) = world.get::<StatusEffects>(e) else { return false };
    if race_immune || se.immunities.get(status).is_some_and(|until| *until > tick) {
        return false;
    }
    let existing = se.active.get(status).cloned();
    let new_state = match (&existing, def.stacking) {
        (Some(_), Stacking::Ignore) => return false,
        (Some(cur), Stacking::Refresh) => ActiveStatus {
            severity: cur.severity.max(severity),
            remaining: def.duration,
            ..cur.clone()
        },
        (Some(cur), Stacking::Intensify) => ActiveStatus {
            severity: cur.severity + severity,
            remaining: def.duration,
            ..cur.clone()
        },
        (None, _) => ActiveStatus { severity, applied: tick, remaining: def.duration, stage: None },
    };
    world.get_mut::<StatusEffects>(e).unwrap().active.insert(status.to_string(), new_state);
    if existing.is_none() {
        let pos = world.get::<Position>(e).copied();
        let (actor, target) = (source.and_then(|s| sim_id(world, s)), sim_id(world, e));
        let news = if def.kind == crate::content::StatusKind::Disease || def.kind == crate::content::StatusKind::Mutation { 0.3 } else { 0.0 };
        let who = world_name(world, e);
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new(kind::STATUS_APPLIED, format!("{} colpisce {who}", def.name))
                .actor(actor)
                .target(target)
                .pos(pos)
                .news(news)
                .tags(def.tags.iter().cloned().chain([status.to_string()])),
        );
        let ctx = EffectCtx::new(Some(e), source, format!("status:{status}"));
        apply_effects(world, &ctx, &def.on_apply);
    }
    update_stage(world, e, status);
    true
}

fn world_name(world: &World, e: Entity) -> String {
    world.get::<crate::stats::DisplayName>(e).map_or_else(|| "?".into(), |n| n.0.clone())
}

pub fn remove_status(world: &mut World, e: Entity, status: &str, expired: bool) {
    let Some(mut se) = world.get_mut::<StatusEffects>(e) else { return };
    if se.active.remove(status).is_none() {
        return;
    }
    let tick = world.resource::<SimClock>().tick;
    let def = world.resource::<Content>().statuses.get(status).cloned();
    if let Some(def) = def {
        let target = sim_id(world, e);
        let who = world_name(world, e);
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new(kind::STATUS_ENDED, format!("{who}: finito {}", def.name))
                .target(target)
                .tags([status.to_string()]),
        );
        if expired {
            let ctx = EffectCtx::new(Some(e), None, format!("status:{status}"));
            apply_effects(world, &ctx, &def.on_expire);
        }
    }
}

/// Recomputes the stage of a status, running `on_enter` of newly reached stages and escalating.
fn update_stage(world: &mut World, e: Entity, status: &str) {
    let Some(def) = world.resource::<Content>().statuses.get(status).cloned() else { return };
    let Some(cur) = world.get::<StatusEffects>(e).and_then(|s| s.active.get(status).cloned()) else { return };
    if let (Some(next), Some(at)) = (&def.escalates_to, def.escalate_at)
        && cur.severity >= at {
            remove_status(world, e, status, false);
            apply_status(world, e, next, 1.0, None);
            return;
        }
    let stage = stage_for(&def, cur.severity);
    if stage != cur.stage {
        if let Some(mut se) = world.get_mut::<StatusEffects>(e)
            && let Some(a) = se.active.get_mut(status) {
                a.stage = stage;
            }
        if let Some(i) = stage.filter(|i| cur.stage.is_none_or(|c| *i > c)) {
            let st = &def.stages[i];
            let tick = world.resource::<SimClock>().tick;
            let target = sim_id(world, e);
            let pos = world.get::<Position>(e).copied();
            let who = world_name(world, e);
            world.resource_mut::<EventLog>().push(
                tick,
                EventBuilder::new(kind::STATUS_STAGE, format!("{who}: {} → {}", def.name, st.name))
                    .target(target)
                    .pos(pos)
                    .news(if def.kind == crate::content::StatusKind::Mutation { 0.5 } else { 0.1 })
                    .tags([status.to_string()]),
            );
            let ctx = EffectCtx::new(Some(e), None, format!("status:{status}"));
            apply_effects(world, &ctx, &st.on_enter);
        }
    }
}

/// Tick of every active status: duration, progression, stages, per-tick effects, expiry.
pub fn tick_statuses(world: &mut World) {
    let entities: Vec<Entity> = crate::sorted_entities::<StatusEffects>(world);
    for e in entities {
        if world.get::<Dead>(e).is_some() {
            continue;
        }
        let actives: Vec<(String, ActiveStatus)> = world
            .get::<StatusEffects>(e)
            .map(|s| s.active.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        for (sid, _) in actives {
            let Some(def) = world.resource::<Content>().statuses.get(&sid).cloned() else { continue };
            if !def.per_tick.is_empty() {
                let ctx = EffectCtx::new(Some(e), None, format!("status:{sid}"));
                apply_effects(world, &ctx, &def.per_tick);
            }
            let mut expired = false;
            if let Some(mut se) = world.get_mut::<StatusEffects>(e) {
                if let Some(a) = se.active.get_mut(&sid) {
                    a.severity += def.progression;
                    if let Some(r) = a.remaining.as_mut() {
                        *r = r.saturating_sub(1);
                        expired |= *r == 0;
                    }
                    expired |= def.progression < 0.0 && a.severity <= 0.0;
                    if let Some(max) = def.max_severity {
                        a.severity = a.severity.min(max);
                    }
                } else {
                    continue;
                }
            }
            if expired {
                remove_status(world, e, &sid, true);
            } else {
                update_stage(world, e, &sid);
            }
        }
        // Expired immunities.
        let tick = world.resource::<SimClock>().tick;
        if let Some(mut se) = world.get_mut::<StatusEffects>(e) {
            se.immunities.retain(|_, until| *until > tick);
        }
    }
}

/// Changes an entity's race (transmutation), rebuilding the body if the plan changes.
pub fn transmute(world: &mut World, e: Entity, race: &str) {
    let content = world.resource::<Content>();
    let Some(rdef) = content.races.get(race).cloned() else { return };
    let old = world.get::<Race>(e).map(|r| r.0.clone()).unwrap_or_default();
    if old == race {
        return;
    }
    let old_name = content.races.get(&old).map_or(old.clone(), |r| r.name.clone());
    let plan = content.body_plans.get(&rdef.body_plan).cloned();
    world.entity_mut(e).insert(Race(race.to_string()));
    if let (Some(plan), Some(body)) = (plan, world.get::<crate::anatomy::Body>(e).cloned())
        && body.plan != plan.id {
            let ratio = body.health_ratio();
            let mut nb = crate::anatomy::Body::from_plan(&plan);
            for p in &mut nb.parts {
                p.hp = (p.max_hp * ratio).max(1.0);
            }
            world.entity_mut(e).insert(nb);
        }
    for s in rdef.innate_statuses.clone() {
        apply_status(world, e, &s, 1.0, None);
    }
    let tick = world.resource::<SimClock>().tick;
    let target = sim_id(world, e);
    let pos = world.get::<Position>(e).copied();
    let name = world_name(world, e);
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::TRANSMUTATION, format!("{name} si trasforma: da {old_name} a {}", rdef.name))
            .target(target)
            .pos(pos)
            .news(0.8)
            .tags(["transmutation".to_string(), race.to_string()]),
    );
}
