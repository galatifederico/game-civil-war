//! Active abilities (powers, gadgets, mystic arts) with cooldowns, defined in data.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;

use crate::content::Content;
use crate::effects::{apply_effects, eval_condition, EffectCtx};
use crate::events::{kind, EventBuilder, EventLog};
use crate::ids::SimId;
use crate::map::Position;
use crate::stats::{Classes, Race};
use crate::time::SimClock;

#[derive(Component, Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AbilityCooldowns(pub BTreeMap<String, u64>);

/// Abilities an entity has from race, classes and statuses' granted tags.
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
    v.sort();
    v.dedup();
    v
}

pub fn ready(world: &mut World, e: Entity, ability: &str) -> bool {
    if !known(world, e).iter().any(|a| a == ability) {
        return false;
    }
    let tick = world.resource::<SimClock>().tick;
    if world.get::<AbilityCooldowns>(e).and_then(|c| c.0.get(ability)).is_some_and(|until| *until > tick) {
        return false;
    }
    let Some(def) = world.resource::<Content>().abilities.get(ability).cloned() else { return false };
    eval_condition(world, &EffectCtx::new(Some(e), None, "ability"), &def.requires)
}

pub fn use_ability(world: &mut World, e: Entity, ability: &str, target: Option<Entity>) -> bool {
    if !ready(world, e, ability) {
        return false;
    }
    let def = world.resource::<Content>().abilities.get(ability).cloned().expect("checked");
    let tick = world.resource::<SimClock>().tick;
    if world.get::<AbilityCooldowns>(e).is_none() {
        world.entity_mut(e).insert(AbilityCooldowns::default());
    }
    world.get_mut::<AbilityCooldowns>(e).unwrap().0.insert(ability.to_string(), tick + def.cooldown);
    let ctx = EffectCtx::new(Some(e), target, format!("ability:{ability}"));
    apply_effects(world, &ctx, &def.effects);
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
            .tags(def.tags.iter().cloned().chain([ability.to_string()])),
    );
    true
}
