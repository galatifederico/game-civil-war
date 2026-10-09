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

/// Circumstances of the whole world (strikes, fog, festivals…): intensity 0..100 each, like statuses.
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalModifiers {
    #[serde(default)]
    pub levels: BTreeMap<String, f32>,
}

impl GlobalModifiers {
    fn active<'a>(&'a self, content: &'a Content) -> impl Iterator<Item = &'a crate::content::GlobalModifierDef> {
        self.levels.keys().filter_map(|id| content.global_modifiers.get(id))
    }

    pub fn logistics_disruption(&self, content: &Content) -> f32 {
        self.active(content).map(|m| m.logistics_disruption).sum()
    }

    /// Combined outside-supply factor for an item (by id or one of its tags).
    pub fn supply_factor(&self, content: &Content, item: &str, tags: &[String]) -> f32 {
        self.active(content)
            .flat_map(|m| m.supply.iter())
            .filter(|(k, _)| *k == "*" || *k == item || tags.contains(k))
            .map(|(_, f)| *f)
            .product()
    }

    /// Stat modifiers every pawn gets from the active circumstances.
    pub fn stats(&self, content: &Content) -> BTreeMap<String, f32> {
        let mut out = BTreeMap::new();
        for m in self.active(content) {
            for (k, v) in &m.stats {
                *out.entry(k.clone()).or_insert(0.0) += v;
            }
        }
        out
    }

    pub fn names(&self, content: &Content) -> Vec<String> {
        self.active(content).map(|m| m.name.clone()).collect()
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

/// Starts (or strengthens) a circumstance: `doses` × its intensity (default 100), at most 100.
pub fn activate_modifier(world: &mut World, id: &str, doses: f32) {
    let tick = world.resource::<SimClock>().tick;
    let Some(def) = world.resource::<Content>().global_modifiers.get(id).cloned() else { return };
    let new = {
        let mut gm = world.resource_mut::<GlobalModifiers>();
        let lvl = gm.levels.entry(id.to_string()).or_insert(0.0);
        let new = *lvl <= 0.0;
        *lvl = (*lvl + def.intensity.unwrap_or(100.0) * doses).min(100.0);
        new
    };
    if new {
        let lasts = if def.per_tick < 0.0 { format!(" (circa {:.0} ore)", 100.0 / -def.per_tick) } else { String::new() };
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new(kind::GLOBAL_MODIFIER, format!("Evento globale: {}{lasts}", def.name))
                .news(0.7)
                .tags(["global".to_string(), id.to_string()]),
        );
    }
}

/// Circumstances: intensity moves by `per_tick`, they end at 0; while active their prices hold.
pub fn circumstances_tick(world: &mut World) {
    let content = world.resource::<Content>().clone();
    let levels = world.resource::<GlobalModifiers>().levels.clone();
    for (id, lvl) in levels {
        let Some(def) = content.global_modifiers.get(&id) else {
            world.resource_mut::<GlobalModifiers>().levels.remove(&id);
            continue;
        };
        let next = (lvl + def.per_tick).min(100.0);
        if next < 0.01 {
            world.resource_mut::<GlobalModifiers>().levels.remove(&id);
            continue;
        }
        world.resource_mut::<GlobalModifiers>().levels.insert(id.clone(), next);
        for (tag, mult) in &def.prices {
            crate::market::add_shock(world, None, Some(tag), *mult, 1.0, 2, &format!("circostanza:{id}"));
        }
    }
}

/// Production posting, passive output, damage consequences, modifier expiry.
pub fn buildings_tick(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let content = world.resource::<Content>().clone();
    let process_job = content.jobs.values().find(|j| j.handler == "process").map(|j| j.id.clone());
    let buildings = crate::sorted_entities::<Building>(world);
    for e in buildings {
        let b = world.get::<Building>(e).unwrap().clone();
        let Some(def) = content.buildings.get(&b.def) else { continue };
        let id = *world.get::<SimId>(e).unwrap();
        // Productions that happen by themselves (fields, pens, visitors…): halted when destroyed.
        if b.hp > 0.0 && tick > 0 {
            for p in def.productions.iter().filter(|p| p.every > 0 && tick.is_multiple_of(p.every)) {
                let Some(mut s) = world.get_mut::<Stock>(e) else { break };
                if !p.inputs.iter().all(|(i, n)| s.count(i) >= *n) {
                    continue;
                }
                for (i, n) in &p.inputs {
                    s.remove(i, *n);
                }
                for (i, n) in &p.outputs {
                    s.add(i, *n);
                }
                if p.money > 0.0 {
                    crate::economy::earn_owner(world, &b.owner, p.money * (b.hp / b.max_hp) as f64);
                }
            }
        }
        // Post one production job per recipe whose inputs are available.
        if let (Some(job), true, Owner::Faction(f)) = (&process_job, b.hp > 0.0, &b.owner) {
            let stock = world.get::<Stock>(e).cloned().unwrap_or_default();
            for (ri, r) in def.productions.iter().enumerate().filter(|(_, r)| r.every == 0) {
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
    let r = def.productions.get(recipe).ok_or("ricetta sconosciuta")?;
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
