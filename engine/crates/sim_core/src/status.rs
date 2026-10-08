//! Statuses: an intensity 0..100 on a pawn. A dose adds the status's `intensity`; every tick the
//! intensity changes by `per_tick`, lowered by the resistance stat; at 0 the status ends. The highest
//! intensity threshold reached adds its modifiers and effects. No randomness: contagion is an aura (see
//! `hygiene`). Also immunities and race transmutation.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::Content;
use crate::effects::{apply_effects, EffectCtx};
use crate::events::{kind, EventBuilder, EventLog};
use crate::ids::SimId;
use crate::map::Position;
use crate::stats::{Dead, Race};
use crate::time::SimClock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActiveStatus {
    /// Intensity, 0..100.
    pub severity: f32,
    pub applied: u64,
    /// Index of the highest threshold reached.
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
}

fn stage_for(def: &crate::content::StatusDef, intensity: f32) -> Option<usize> {
    def.thresholds.iter().enumerate().filter(|(_, t)| intensity >= t.above).map(|(i, _)| i).next_back()
}

fn sim_id(world: &World, e: Entity) -> Option<SimId> {
    world.get::<SimId>(e).copied()
}

/// Whether `e` cannot catch `status` (race, abilities, temporary immunity).
pub fn immune(world: &World, e: Entity, status: &str) -> bool {
    let c = world.resource::<Content>();
    let tick = world.resource::<SimClock>().tick;
    world.get::<Race>(e).and_then(|r| c.races.get(&r.0)).is_some_and(|r| r.immunities.iter().any(|i| i == status))
        || world.get::<crate::stats::Tags>(e).is_some_and(|t| {
            t.effective.iter().filter_map(|x| x.strip_prefix("ability:")).filter_map(|a| c.abilities.get(a)).any(|a| a.immunities.iter().any(|i| i == status))
        })
        || world.get::<StatusEffects>(e).is_some_and(|s| s.immunities.get(status).is_some_and(|until| *until > tick))
}

/// Gives `doses` doses of a status (each adds the status's intensity). Returns false when the carrier is
/// immune or the status unknown.
pub fn apply_status(world: &mut World, e: Entity, status: &str, doses: f32, source: Option<Entity>) -> bool {
    let Some(def) = world.resource::<Content>().statuses.get(status).cloned() else { return false };
    add_intensity(world, e, status, def.dose() * doses, source)
}

/// Adds raw intensity to a status (contagion uses this), starting it if needed.
pub fn add_intensity(world: &mut World, e: Entity, status: &str, amount: f32, source: Option<Entity>) -> bool {
    let tick = world.resource::<SimClock>().tick;
    let Some(def) = world.resource::<Content>().statuses.get(status).cloned() else { return false };
    if world.get::<Dead>(e).is_some() || world.get::<StatusEffects>(e).is_none() || immune(world, e, status) || amount <= 0.0 {
        return false;
    }
    let existing = world.get::<StatusEffects>(e).and_then(|s| s.active.get(status).cloned());
    let state = match &existing {
        Some(cur) => ActiveStatus { severity: (cur.severity + amount).min(100.0), ..cur.clone() },
        None => ActiveStatus { severity: amount.min(100.0), applied: tick, stage: None },
    };
    world.get_mut::<StatusEffects>(e).unwrap().active.insert(status.to_string(), state);
    if existing.is_none() {
        let pos = world.get::<Position>(e).copied();
        let (actor, target) = (source.and_then(|s| sim_id(world, s)), sim_id(world, e));
        let disease_news = world.resource::<crate::params::Params>().get("press.disease_news", 0.2) as f32;
        let news = if def.tags.iter().any(|t| t == "malattia" || t == "mutazione") { disease_news } else { 0.0 };
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

/// Recomputes the highest threshold reached, running `on_enter` of a newly reached one.
fn update_stage(world: &mut World, e: Entity, status: &str) {
    let Some(def) = world.resource::<Content>().statuses.get(status).cloned() else { return };
    let Some(cur) = world.get::<StatusEffects>(e).and_then(|s| s.active.get(status).cloned()) else { return };
    let stage = stage_for(&def, cur.severity);
    if stage == cur.stage {
        return;
    }
    if let Some(mut se) = world.get_mut::<StatusEffects>(e)
        && let Some(a) = se.active.get_mut(status)
    {
        a.stage = stage;
    }
    if let Some(i) = stage.filter(|i| cur.stage.is_none_or(|c| *i > c)) {
        let th = &def.thresholds[i];
        let tick = world.resource::<SimClock>().tick;
        let target = sim_id(world, e);
        let pos = world.get::<Position>(e).copied();
        let who = world_name(world, e);
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new(kind::STATUS_STAGE, format!("{who}: {} → {}", def.name, th.name))
                .target(target)
                .pos(pos)
                .news(if def.tags.iter().any(|t| t == "mutazione") { 0.5 } else { 0.1 })
                .tags([status.to_string()]),
        );
        let ctx = EffectCtx::new(Some(e), None, format!("status:{status}"));
        apply_effects(world, &ctx, &th.on_enter);
    }
}

/// Every tick: effects, intensity change (and resistance), thresholds, end at 0.
pub fn tick_statuses(world: &mut World) {
    let entities: Vec<Entity> = crate::sorted_entities::<StatusEffects>(world);
    for e in entities {
        if world.get::<Dead>(e).is_some() {
            continue;
        }
        let actives: Vec<String> = world.get::<StatusEffects>(e).map(|s| s.active.keys().cloned().collect()).unwrap_or_default();
        for sid in actives {
            let Some(def) = world.resource::<Content>().statuses.get(&sid).cloned() else { continue };
            let Some(cur) = world.get::<StatusEffects>(e).and_then(|s| s.active.get(&sid).cloned()) else { continue };
            let ctx = EffectCtx::new(Some(e), None, format!("status:{sid}"));
            apply_effects(world, &ctx, &def.effects);
            if let Some(th) = cur.stage.and_then(|i| def.thresholds.get(i)) {
                apply_effects(world, &ctx, &th.effects);
            }
            let resist = def.resist_stat.as_ref().and_then(|s| world.get::<crate::stats::Stats>(e).map(|x| x.get(s))).unwrap_or(0.0).max(0.0);
            let change = def.per_tick - resist * def.resist_per_point;
            let ended = {
                let Some(mut se) = world.get_mut::<StatusEffects>(e) else { continue };
                let Some(a) = se.active.get_mut(&sid) else { continue };
                a.severity = (a.severity + change).min(100.0);
                // Below a hundredth it is over (durations written as 100/D do not add up exactly).
                a.severity < 0.01
            };
            if ended {
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
    // Leaders and champions are never stuck in another shape: they turn back after a while.
    let player_owned = world.get::<crate::player::Controlled>(e).is_some() || world.get::<crate::factions::Leader>(e).is_some();
    if player_owned && world.get::<crate::player::Transmuted>(e).is_none() {
        let until = world.resource::<SimClock>().tick + world.resource::<crate::params::Params>().get("player.transmute_ticks", 24.0) as u64;
        world.entity_mut(e).insert(crate::player::Transmuted { original: old.clone(), until });
    }
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
