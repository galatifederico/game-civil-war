//! ProcessingBuildingFramework, ShopFramework and StructuralDamageConsequenceEngine.

use std::collections::{BTreeMap, BTreeSet};

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::Content;
use crate::effects::{apply_effects, EffectCtx};
use crate::events::{kind, EventBuilder, EventLog};
use crate::ids::SimId;
use crate::inventory::Stock;
use crate::jobs::{JobBoard, JobTarget};
use crate::map::Position;
use crate::stats::{DisplayName, TemplateId};
use crate::time::SimClock;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum Owner {
    #[default]
    None,
    Faction(String),
    Entity(SimId),
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Building {
    pub def: String,
    pub hp: f32,
    pub max_hp: f32,
    pub owner: Owner,
    /// Indices of damage consequences already fired (reset when repaired above the threshold).
    pub fired: BTreeSet<usize>,
}

/// A building that sells goods. `catalog`: item → fixed price (None = market price × markup).
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Shop {
    pub catalog: BTreeMap<String, Option<f64>>,
    pub markup: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActiveModifier {
    pub id: String,
    pub name: String,
    pub until: u64,
    pub logistics_disruption: f32,
    pub morale: f32,
    /// Outside supply multipliers: (item id or tag, factor), "*" = everything.
    #[serde(default)]
    pub supply: Vec<(String, f32)>,
}

/// Global simulation events with a duration (delivery delays, morale crises, price spikes…).
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalModifiers {
    pub active: Vec<ActiveModifier>,
}

impl GlobalModifiers {
    pub fn logistics_disruption(&self) -> f32 {
        self.active.iter().map(|m| m.logistics_disruption).sum()
    }

    /// Combined outside-supply factor for an item (by id or one of its tags).
    pub fn supply_factor(&self, item: &str, tags: &[String]) -> f32 {
        self.active
            .iter()
            .flat_map(|m| m.supply.iter())
            .filter(|(k, _)| k == "*" || k == item || tags.contains(k))
            .map(|(_, f)| *f)
            .product()
    }
}

pub fn spawn_building(world: &mut World, def_id: &str, pos: Position, name: Option<String>, owner: Owner) -> Option<Entity> {
    let def = world.resource::<Content>().buildings.get(def_id)?.clone();
    let e = world.spawn_empty().id();
    let id = crate::lifecycle::register(world, e);
    let hp = if def.hp > 0.0 { def.hp } else { 100.0 };
    world.entity_mut(e).insert((
        DisplayName(name.unwrap_or_else(|| def.name.clone())),
        TemplateId(def.id.clone()),
        Building { def: def.id.clone(), hp, max_hp: hp, owner, fired: BTreeSet::new() },
        Stock(def.stock.clone()),
        pos,
    ));
    if !def.sells.is_empty() {
        world.entity_mut(e).insert(Shop { catalog: def.sells.clone(), markup: if def.markup > 0.0 { def.markup } else { 1.0 } });
    }
    let _ = id;
    Some(e)
}

pub fn damage_building(world: &mut World, e: Entity, amount: f32, source: Option<Entity>) {
    let Some(mut b) = world.get_mut::<Building>(e) else { return };
    let before = b.hp;
    b.hp = (b.hp - amount).max(0.0);
    let (hp, max) = (b.hp, b.max_hp);
    if before > 0.0 && hp < before {
        let tick = world.resource::<SimClock>().tick;
        let name = crate::effects::name_of(world, e);
        let (id, actor) = (world.get::<SimId>(e).copied(), source.and_then(|s| world.get::<SimId>(s).copied()));
        let pos = world.get::<Position>(e).copied();
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new(kind::STRUCTURE_DAMAGE, format!("{name} danneggiato: {:.0}/{:.0}", hp, max))
                .actor(actor)
                .target(id)
                .pos(pos)
                .news(if hp <= 0.0 { 0.8 } else { 0.4 })
                .tags(["structure", "sabotage"]),
        );
    }
}

pub fn repair_building(world: &mut World, e: Entity, amount: f32) {
    if let Some(mut b) = world.get_mut::<Building>(e) {
        b.hp = (b.hp + amount).min(b.max_hp);
    }
}

pub fn activate_modifier(world: &mut World, id: &str, name: &str, duration: u64, disruption: f32, morale: f32) {
    let tick = world.resource::<SimClock>().tick;
    let def = world.resource::<Content>().global_modifiers.get(id).cloned();
    let (name, disruption, morale) = match &def {
        Some(d) => (
            if name.is_empty() { d.name.clone() } else { name.to_string() },
            if disruption == 0.0 { d.logistics_disruption } else { disruption },
            if morale == 0.0 { d.morale } else { morale },
        ),
        None => (if name.is_empty() { id.to_string() } else { name.to_string() }, disruption, morale),
    };
    {
        let mut gm = world.resource_mut::<GlobalModifiers>();
        gm.active.retain(|m| m.id != id);
        let supply = def.as_ref().map(|d| d.supply.clone()).unwrap_or_default();
        gm.active.push(ActiveModifier { id: id.to_string(), name: name.clone(), until: tick + duration, logistics_disruption: disruption, morale, supply });
    }
    if morale != 0.0 {
        let stat = world.resource::<Content>().bindings.morale.clone();
        let ctx = EffectCtx::new(None, None, format!("modifier:{id}"));
        let eff = crate::content::Effect::On(crate::content::Scope::Everyone, Box::new(crate::content::Effect::ModStat { stat, amount: morale }));
        crate::effects::apply_effect(world, &ctx, &eff);
    }
    if let Some(d) = def {
        for (tag, mult) in &d.price_tags {
            crate::market::add_shock(world, None, Some(tag), *mult, 1.0, duration, id);
        }
    }
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::GLOBAL_MODIFIER, format!("Evento globale: {name} ({duration} tick)"))
            .news(0.7)
            .tags(["global".to_string(), id.to_string()]),
    );
}

/// Production posting, passive output, damage consequences, modifier expiry.
pub fn buildings_tick(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    world.resource_mut::<GlobalModifiers>().active.retain(|m| m.until > tick);
    let content = world.resource::<Content>().clone();
    let process_job = content.jobs.values().find(|j| j.handler == "process").map(|j| j.id.clone());
    let buildings = crate::sorted_entities::<Building>(world);
    for e in buildings {
        let b = world.get::<Building>(e).unwrap().clone();
        let Some(def) = content.buildings.get(&b.def) else { continue };
        let id = *world.get::<SimId>(e).unwrap();
        // Passive production (fields, pens, generators): halted when destroyed.
        if b.hp > 0.0 && def.passive_interval > 0 && tick.is_multiple_of(def.passive_interval) && tick > 0
            && let Some(mut s) = world.get_mut::<Stock>(e) {
                for (item, n) in &def.passive {
                    s.add(item, *n);
                }
            }
        // Post one production job per recipe whose inputs are available.
        if let (Some(job), true, Owner::Faction(f)) = (&process_job, b.hp > 0.0, &b.owner) {
            let stock = world.get::<Stock>(e).cloned().unwrap_or_default();
            for (ri, r) in def.recipes.iter().enumerate() {
                let payload = Some(crate::jobs::JobPayload::Recipe { building: id, index: ri });
                let open = world.resource::<JobBoard>().jobs.values().any(|j| j.payload == payload);
                let has = r.inputs.iter().all(|(i, n)| stock.count(i) >= *n);
                if !open && has {
                    let jid = crate::jobs::post_job(world, job, Some(f.clone()), JobTarget::Entity(id), 0, None);
                    if let Some(j) = world.resource_mut::<JobBoard>().jobs.get_mut(&jid) {
                        j.payload = payload;
                    }
                }
            }
        }
        // Structural damage consequences.
        let ratio = b.hp / b.max_hp;
        for (i, dc) in def.consequences.iter().enumerate() {
            let fired = b.fired.contains(&i);
            if ratio < dc.below && !fired {
                world.get_mut::<Building>(e).unwrap().fired.insert(i);
                let pos = world.get::<Position>(e).copied();
                world.resource_mut::<EventLog>().push(
                    tick,
                    EventBuilder::new(kind::STRUCTURE_DAMAGE, format!("{}: {}", def.name, dc.name))
                        .target(Some(id))
                        .pos(pos)
                        .news(0.9)
                        .tags(["structure", "consequence"]),
                );
                let ctx = EffectCtx::new(Some(e), None, format!("building:{}", def.id));
                apply_effects(world, &ctx, &dc.effects);
            } else if ratio >= dc.below && fired {
                world.get_mut::<Building>(e).unwrap().fired.remove(&i);
            }
        }
    }
}

/// Runs one batch of a recipe in a building. Returns false if inputs are missing.
pub fn process_recipe(world: &mut World, building: Entity, recipe: usize) -> Result<String, String> {
    let b = world.get::<Building>(building).cloned().ok_or("non è un edificio")?;
    let content = world.resource::<Content>().clone();
    let def = content.buildings.get(&b.def).ok_or("edificio sconosciuto")?;
    let r = def.recipes.get(recipe).ok_or("ricetta sconosciuta")?;
    let mut stock = world.get_mut::<Stock>(building).ok_or("nessun magazzino")?;
    if !r.inputs.iter().all(|(i, n)| stock.count(i) >= *n) {
        return Err("mancano gli ingredienti".into());
    }
    for (i, n) in &r.inputs {
        stock.remove(i, *n);
    }
    for (i, n) in &r.outputs {
        stock.add(i, *n);
    }
    let out: Vec<String> = r.outputs.iter().map(|(i, n)| format!("{n}× {}", content.items.get(i).map_or(i.as_str(), |d| d.name.as_str()))).collect();
    Ok(format!("{}: {} ({})", def.name, r.name, out.join(", ")))
}
