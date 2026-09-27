//! Hygiene & Environmental System: fluids and dirt on cells, pathogen loads, contagion by contact/air/
//! fluids/water networks/food, cleaning.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::anatomy::Body;
use crate::content::{Content, Vector};
use crate::map::{Environment, Position, WorldMap};
use crate::params::Params;
use crate::rng::SimRng;
use crate::stats::{Dead, Pawn};
use crate::status::{apply_status, StatusEffects};

/// Poisoned stock (food/water) of a building: buyers catch the status.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Contaminated {
    pub status: String,
    pub load: f32,
}

pub fn spill(world: &mut World, p: Position, fluid: &str, amount: f32) {
    let def = world.resource::<Content>().fluids.get(fluid).cloned();
    let mut env = world.resource_mut::<Environment>();
    let cell = env.cell_mut(p);
    *cell.fluids.entry(fluid.to_string()).or_insert(0.0) += amount;
    if let Some(d) = def {
        cell.dirt += amount * d.dirtiness;
        if let Some(s) = d.carries {
            *cell.pathogens.entry(s).or_insert(0.0) += amount;
        }
    }
}

pub fn clean(world: &mut World, p: Position, radius: i32, amount: f32) {
    let mut env = world.resource_mut::<Environment>();
    for (pos, c) in env.cells.iter_mut() {
        if pos.within(&p, radius) {
            c.dirt = (c.dirt - amount).max(0.0);
            for v in c.fluids.values_mut().chain(c.pathogens.values_mut()) {
                *v = (*v - amount).max(0.0);
            }
            c.fluids.retain(|_, v| *v > 0.01);
            c.pathogens.retain(|_, v| *v > 0.01);
        }
    }
    env.compact();
}

/// Contaminates what the entity is: a building's stock (food/water poisoning, and its network), or the
/// cell where a pawn stands.
pub fn contaminate(world: &mut World, e: Entity, status: &str, load: f32) {
    let Some(p) = world.get::<Position>(e).copied() else { return };
    if world.get::<crate::buildings::Building>(e).is_some() {
        world.entity_mut(e).insert(Contaminated { status: status.to_string(), load });
        let map = world.resource::<WorldMap>().clone();
        let zones: Vec<usize> = (0..map.zones.len()).filter(|i| map.zones[*i].contains(&p)).collect();
        let mut env = world.resource_mut::<Environment>();
        for n in map.networks.iter().filter(|n| n.zones.iter().any(|z| zones.contains(z))) {
            *env.network_load.entry(n.id.clone()).or_default().entry(status.to_string()).or_insert(0.0) += load;
        }
    } else {
        *world.resource_mut::<Environment>().cell_mut(p).pathogens.entry(status.to_string()).or_insert(0.0) += load;
    }
}

/// Spills, shedding, contagion and decay, once per tick.
pub fn hygiene_tick(world: &mut World) {
    let content = world.resource::<Content>().clone();
    let p = world.resource::<Params>().clone();
    let (scale, flow, dirt_decay) = (p.f("hygiene.infection_scale"), p.f("hygiene.network_flow"), p.f("hygiene.dirt_decay"));
    let bleed = p.f("health.bleed_threshold");
    let pawns: Vec<Entity> = crate::sorted_entities::<Pawn>(world).into_iter().filter(|e| world.get::<Dead>(*e).is_none()).collect();
    let blood = content.bindings.blood_fluid.clone().filter(|f| content.fluids.contains_key(f));
    // 1. Spills and shedding from carriers.
    for e in &pawns {
        let Some(pos) = world.get::<Position>(*e).copied() else { continue };
        if let Some(b) = &blood {
            let bleeding = world.get::<Body>(*e).map_or(0, |body| body.parts.iter().filter(|x| !x.missing && x.hp / x.max_hp < bleed).count());
            if bleeding > 0 {
                spill(world, pos, b, 0.1 * bleeding as f32);
            }
        }
        let active: Vec<String> = world.get::<StatusEffects>(*e).map(|s| s.active.keys().cloned().collect()).unwrap_or_default();
        for sid in active {
            let Some(sd) = content.statuses.get(&sid) else { continue };
            if let Some((fluid, amount)) = &sd.spills {
                spill(world, pos, fluid, *amount);
            }
            if let Some(c) = &sd.contagion {
                if c.shedding > 0.0 {
                    *world.resource_mut::<Environment>().cell_mut(pos).pathogens.entry(sid.clone()).or_insert(0.0) += c.shedding;
                }
            }
        }
    }
    // 2. Direct contagion (contact / air).
    let mut infections: Vec<(Entity, String, f32, Entity)> = Vec::new();
    for carrier in &pawns {
        let Some(cpos) = world.get::<Position>(*carrier).copied() else { continue };
        let active: Vec<String> = world.get::<StatusEffects>(*carrier).map(|s| s.active.keys().cloned().collect()).unwrap_or_default();
        for sid in active {
            let Some(c) = content.statuses.get(&sid).and_then(|d| d.contagion.clone()) else { continue };
            let radius = if c.vectors.contains(&Vector::Air) { c.radius } else if c.vectors.contains(&Vector::Contact) { 1 } else { continue };
            for other in &pawns {
                if other == carrier {
                    continue;
                }
                if world.get::<Position>(*other).is_some_and(|p| p.within(&cpos, radius))
                    && world.get::<StatusEffects>(*other).is_some_and(|s| !s.has(&sid))
                    && world.resource_mut::<SimRng>().chance(c.chance)
                {
                    infections.push((*other, sid.clone(), c.initial_severity, *carrier));
                }
            }
        }
    }
    // 3. Fluids and contaminated cells, water networks.
    let map = world.resource::<WorldMap>().clone();
    let env = world.resource::<Environment>().clone();
    for e in &pawns {
        let Some(pos) = world.get::<Position>(*e).copied() else { continue };
        if let Some(cell) = env.cell(&pos) {
            for (sid, load) in &cell.pathogens {
                let fluid_vector = content.statuses.get(sid).and_then(|d| d.contagion.as_ref()).is_none_or(|c| c.vectors.contains(&Vector::Fluid));
                if fluid_vector && world.resource_mut::<SimRng>().chance(load * scale) {
                    infections.push((*e, sid.clone(), 1.0, *e));
                }
            }
        }
        for n in &map.networks {
            if !n.zones.iter().any(|z| map.zones[*z].contains(&pos)) {
                continue;
            }
            for (sid, load) in env.network_load.get(&n.id).into_iter().flatten() {
                if world.resource_mut::<SimRng>().chance(load * scale) {
                    infections.push((*e, sid.clone(), 1.0, *e));
                }
            }
        }
    }
    for (e, sid, sev, src) in infections {
        if apply_status(world, e, &sid, sev, (src != e).then_some(src)) {
            let tick = world.resource::<crate::time::SimClock>().tick;
            let id = world.get::<crate::ids::SimId>(e).copied();
            let name = crate::effects::name_of(world, e);
            let sname = content.statuses.get(&sid).map_or(sid.clone(), |d| d.name.clone());
            world.resource_mut::<crate::events::EventLog>().push(
                tick,
                crate::events::EventBuilder::new(crate::events::kind::INFECTION, format!("{name} contagiato: {sname}"))
                    .target(id)
                    .tags(["contagion", sid.as_str()]),
            );
        }
    }
    // 4. Decay.
    let mut env = world.resource_mut::<Environment>();
    for c in env.cells.values_mut() {
        c.dirt = (c.dirt - dirt_decay).max(0.0);
        for v in c.fluids.values_mut().chain(c.pathogens.values_mut()) {
            *v *= 0.98;
        }
        c.fluids.retain(|_, v| *v > 0.01);
        c.pathogens.retain(|_, v| *v > 0.01);
    }
    for load in env.network_load.values_mut() {
        for v in load.values_mut() {
            *v *= 1.0 - flow;
        }
        load.retain(|_, v| *v > 0.01);
    }
    env.compact();
}
