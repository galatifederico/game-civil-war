//! Abilities are permanent traits (modifiers on the holder, auras on the pawns around it), from race,
//! classes and roles. Things a pawn does on purpose (attacks, spells, transformations) are actions with
//! their own effects (`ActionKind::Effects`); this module also runs those.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;

use crate::content::{ActionKind, Content};
use crate::effects::{apply_effects, eval_condition, EffectCtx};
use crate::events::{kind, EventBuilder, EventLog};
use crate::ids::SimId;
use crate::map::Position;
use crate::stats::{Classes, Dead, Pawn, Race, Stats};
use crate::time::SimClock;

/// Kept so that older saves load; cooldowns now live in the AI's action cooldowns.
#[derive(Component, Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AbilityCooldowns(pub BTreeMap<String, u64>);

/// Stat modifiers a pawn gets from the auras it stands in (recomputed every tick).
#[derive(Component, Debug, Clone, Default)]
pub struct AuraBonus(pub BTreeMap<String, f32>);

/// Abilities an entity has from race, classes and roles.
pub fn known(world: &World, e: Entity) -> Vec<String> {
    let c = world.resource::<Content>();
    let mut v = Vec::new();
    if let Some(r) = world.get::<Race>(e).and_then(|r| c.races.get(&r.0)) {
        v.extend(r.abilities.iter().cloned());
    }
    if let Some(cl) = world.get::<Classes>(e) {
        for x in &cl.0 {
            if let Some(d) = c.classes.get(x) {
                v.extend(d.abilities.iter().cloned());
            }
        }
    }
    for t in crate::titles::held(world, e) {
        if let Some(d) = c.titles.get(&t) {
            v.extend(d.abilities.iter().cloned());
        }
    }
    v.sort();
    v.dedup();
    v
}

/// Whether `e` can do the action with effects `action` now: it has it, it is not waiting and meets its requirements.
pub fn ready(world: &mut World, e: Entity, action: &str) -> bool {
    let Some(def) = world.resource::<Content>().actions.get(action).cloned() else { return false };
    if !matches!(def.kind, ActionKind::Effects { .. }) {
        return false;
    }
    let tick = world.resource::<SimClock>().tick;
    let Some(brain) = world.get::<crate::ai::Brain>(e) else { return false };
    if !brain.actions.iter().any(|a| a == action) || brain.cooldowns.get(action).is_some_and(|until| *until > tick) {
        return false;
    }
    eval_condition(world, &EffectCtx::new(Some(e), None, "action"), &def.requires)
}

/// Applies the effects of an action with effects (the actor is the subject).
pub fn use_action(world: &mut World, e: Entity, action: &str, target: Option<Entity>) -> bool {
    let Some(def) = world.resource::<Content>().actions.get(action).cloned() else { return false };
    let ActionKind::Effects { effects, .. } = &def.kind else { return false };
    let tick = world.resource::<SimClock>().tick;
    if def.cooldown > 0
        && let Some(mut b) = world.get_mut::<crate::ai::Brain>(e)
    {
        b.cooldowns.insert(action.to_string(), tick + def.cooldown);
    }
    let ctx = EffectCtx::new(Some(e), target, format!("action:{action}"));
    apply_effects(world, &ctx, effects);
    if def.suspicious {
        crate::infiltration::suspicious_act(world, e);
    }
    let pos = world.get::<Position>(e).copied();
    let actor = world.get::<SimId>(e).copied();
    let tid = target.and_then(|t| world.get::<SimId>(t).copied());
    let name = crate::infiltration::apparent_name(world, e);
    let on = target.filter(|t| *t != e).map(|t| format!(" su {}", crate::infiltration::apparent_name(world, t))).unwrap_or_default();
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::ABILITY, format!("{name} usa {}{on}", def.name))
            .actor(actor)
            .target(tid)
            .pos(pos)
            .news(if def.suspicious { 0.4 } else { 0.1 })
            .tags(def.tags.iter().cloned().chain([action.to_string()])),
    );
    true
}

/// Auras: pawns near a holder get the aura's lasting modifiers (while in range) and its per-tick changes.
pub fn auras(world: &mut World) {
    let content = world.resource::<Content>().clone();
    let with_aura: Vec<&crate::content::AbilityDef> = content.abilities.values().filter(|a| a.aura.as_ref().is_some_and(|x| x.radius > 0)).collect();
    let pawns: Vec<(Entity, Position)> = {
        let mut q = world.query_filtered::<(Entity, &Position), (With<Pawn>, Without<Dead>)>();
        let mut v: Vec<(Entity, Position)> = q.iter(world).map(|(e, p)| (e, *p)).collect();
        v.sort_by_key(|(e, _)| *e);
        v
    };
    let mut bonus: BTreeMap<Entity, BTreeMap<String, f32>> = BTreeMap::new();
    if !with_aura.is_empty() {
        for &(holder, hp) in &pawns {
            let has = known(world, holder);
            for a in with_aura.iter().filter(|a| has.contains(&a.id)) {
                let aura = a.aura.as_ref().expect("filtered");
                for &(other, op) in &pawns {
                    if other == holder || op.layer != hp.layer || (op.x - hp.x).abs().max((op.y - hp.y).abs()) > aura.radius {
                        continue;
                    }
                    if !eval_condition(world, &EffectCtx::new(Some(other), Some(holder), format!("aura:{}", a.id)), &aura.affects) {
                        continue;
                    }
                    let b = bonus.entry(other).or_default();
                    for (k, v) in &aura.stats {
                        *b.entry(k.clone()).or_insert(0.0) += v;
                    }
                    if !aura.stats_per_tick.is_empty()
                        && let Some(mut s) = world.get_mut::<Stats>(other)
                    {
                        for (k, v) in &aura.stats_per_tick {
                            let (lo, hi) = content.stat_bounds(k);
                            let cur = s.base.get(k).copied().unwrap_or(0.0);
                            s.base.insert(k.clone(), (cur + v).clamp(lo, hi));
                        }
                    }
                }
            }
        }
    }
    for (e, _) in pawns {
        let b = bonus.remove(&e).unwrap_or_default();
        if b.is_empty() && world.get::<AuraBonus>(e).is_none() {
            continue;
        }
        world.entity_mut(e).insert(AuraBonus(b));
    }
}
