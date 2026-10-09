//! Equipment: of the items a pawn carries, which ones it really uses — hand items while it has hands free,
//! the best item for each worn place, only if it meets their requirements — how much it all weighs, how
//! items wear out and break, and how attack and defense scale the damage of a blow.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{Content, ItemDef};
use crate::effects::{eval_condition, EffectCtx};
use crate::events::{EventBuilder, EventLog};
use crate::ids::SimId;
use crate::inventory::Inventory;
use crate::map::Position;
use crate::stats::{Dead, Stats};
use crate::time::SimClock;

/// Items in use (wielded or worn) and the total weight carried; recomputed every tick.
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Equipment {
    pub items: Vec<String>,
    pub weight: f32,
}

/// Items that count only while in use (held in the hands or worn).
pub fn is_equipment(d: &ItemDef) -> bool {
    d.hands > 0 || d.wear_slot.is_some()
}

/// Hands a pawn can use: two per full manipulation capacity (a lost hand is a lost hand).
fn free_hands(world: &World, e: Entity) -> u32 {
    world.get::<crate::anatomy::Body>(e).map_or(2, |b| (b.capacity("manipulation") * 2.0).round().max(0.0) as u32)
}

/// Picks the equipment of every pawn: the most valuable items first, as long as hands and places allow
/// and the requirements are met.
pub fn update(world: &mut World) {
    let content = world.resource::<Content>().clone();
    for e in crate::sorted_entities::<Inventory>(world) {
        if world.get::<Dead>(e).is_some() || world.get::<crate::buildings::Building>(e).is_some() {
            continue;
        }
        let carried: Vec<(String, u32)> = world.get::<Inventory>(e).map(|i| i.items().map(|(k, n)| (k.clone(), n)).collect()).unwrap_or_default();
        let weight: f32 = carried.iter().filter_map(|(k, n)| content.items.get(k).map(|d| d.weight * *n as f32)).sum();
        let mut gear: Vec<&ItemDef> = carried.iter().filter_map(|(k, _)| content.items.get(k)).filter(|d| is_equipment(d)).collect();
        gear.sort_by(|a, b| (b.damage + b.base_price as f32 / 100.0).total_cmp(&(a.damage + a.base_price as f32 / 100.0)).then(a.id.cmp(&b.id)));
        let hands = free_hands(world, e);
        let (mut used, mut places, mut items) = (0u32, Vec::<String>::new(), Vec::new());
        for d in gear {
            if d.hands as u32 > 0 && used + d.hands as u32 > hands {
                continue;
            }
            if let Some(p) = &d.wear_slot
                && places.contains(p)
            {
                continue;
            }
            if !eval_condition(world, &EffectCtx::new(Some(e), None, format!("item:{}", d.id)), &d.requires) {
                continue;
            }
            used += d.hands as u32;
            if let Some(p) = &d.wear_slot {
                places.push(p.clone());
            }
            items.push(d.id.clone());
        }
        let eq = Equipment { items, weight };
        if world.get::<Equipment>(e) != Some(&eq) {
            world.entity_mut(e).insert(eq);
        }
    }
}

/// The best weapon in use: (item, damage).
pub fn best_weapon(world: &World, e: Entity) -> Option<(String, f32)> {
    let content = world.resource::<Content>();
    world
        .get::<Equipment>(e)?
        .items
        .iter()
        .filter_map(|i| content.items.get(i))
        .filter(|d| d.hands > 0 && d.damage > 0.0)
        .max_by(|a, b| a.damage.total_cmp(&b.damage).then(b.id.cmp(&a.id)))
        .map(|d| (d.id.clone(), d.damage))
}

/// The carried item of a kind that hurts the most (e.g. the best thing to throw).
pub fn best_of_type(world: &mut World, e: Entity, kind: &str) -> Option<String> {
    let content = world.resource::<Content>().clone();
    let carried: Vec<String> = world.get::<Inventory>(e)?.items().map(|(k, _)| k.clone()).collect();
    let mut best: Option<&ItemDef> = None;
    for d in carried.iter().filter_map(|k| content.items.get(k)).filter(|d| d.tags.iter().any(|t| t == kind)) {
        if !eval_condition(world, &EffectCtx::new(Some(e), None, format!("item:{}", d.id)), &d.requires) {
            continue;
        }
        if best.is_none_or(|b| d.damage > b.damage || (d.damage == b.damage && d.id < b.id)) {
            best = Some(d);
        }
    }
    best.map(|d| d.id.clone())
}

/// Wears the unit in use of an item; at its durability it breaks (one unit is lost).
pub fn wear(world: &mut World, e: Entity, item: &str, amount: f32) {
    let Some(d) = world.resource::<Content>().items.get(item).cloned() else { return };
    if d.durability <= 0.0 || amount <= 0.0 {
        return;
    }
    let broken = {
        let Some(mut inv) = world.get_mut::<Inventory>(e) else { return };
        let w = inv.wear.entry(item.to_string()).or_insert(0.0);
        *w += amount;
        if *w >= d.durability {
            inv.wear.remove(item);
            true
        } else {
            false
        }
    };
    if broken {
        crate::inventory_ops::take(world, e, item, 1);
        let tick = world.resource::<SimClock>().tick;
        let name = crate::effects::name_of(world, e);
        let (id, pos) = (world.get::<SimId>(e).copied(), world.get::<Position>(e).copied());
        world.resource_mut::<EventLog>().push(tick, EventBuilder::new("item_broken", format!("A {name} si rompe: {}", d.name)).target(id).pos(pos));
    }
}

/// Everything a pawn wears takes a bit of each blow.
pub fn wear_armour(world: &mut World, e: Entity) {
    let content = world.resource::<Content>().clone();
    let worn: Vec<String> = world
        .get::<Equipment>(e)
        .map(|q| q.items.iter().filter(|i| content.items.get(*i).is_some_and(|d| d.wear_slot.is_some())).cloned().collect())
        .unwrap_or_default();
    for i in worn {
        wear(world, e, &i, 1.0);
    }
}

/// How much harder a blow lands: (1 + attack/100) × 100 / (100 + defense).
pub fn combat_multiplier(world: &World, attacker: Entity, victim: Entity) -> f32 {
    let b = &world.resource::<Content>().bindings;
    let stat = |e: Entity, s: &Option<String>| s.as_ref().and_then(|s| world.get::<Stats>(e).map(|x| x.get(s))).unwrap_or(0.0).max(0.0);
    (1.0 + stat(attacker, &b.attack) / 100.0) * 100.0 / (100.0 + stat(victim, &b.defense))
}
