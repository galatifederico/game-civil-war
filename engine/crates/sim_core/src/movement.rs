//! Movement along the grid with fractional speed, capacity penalties and tethering.

use bevy_ecs::prelude::*;

use crate::anatomy::Body;
use crate::content::Content;
use crate::map::{Position, WorldMap};
use crate::params::Params;
use crate::stats::Stats;

#[derive(Component, Debug, Clone, Default)]
pub struct Movement {
    pub budget: f32,
}

/// Steps per tick for an entity.
pub fn speed_of(world: &World, e: Entity) -> f32 {
    let base = world.resource::<Params>().f("move.base_speed");
    let stat = &world.resource::<Content>().bindings.speed;
    let s = world.get::<Stats>(e).map_or(1.0, |s| {
        let v = s.get(stat);
        if v <= 0.0 { 1.0 } else { v }
    });
    let cap = world.get::<Body>(e).map_or(1.0, |b| b.capacity("moving"));
    (base * s * cap).max(0.1)
}

/// Moves `e` towards `goal` until within `range` or out of budget. Returns true when within range.
pub fn move_towards(world: &mut World, e: Entity, goal: Position, range: i32) -> bool {
    let Some(mut pos) = world.get::<Position>(e).copied() else { return true };
    if pos.within(&goal, range) {
        return true;
    }
    let speed = speed_of(world, e);
    let mut budget = world.get::<Movement>(e).map_or(0.0, |m| m.budget) + speed;
    let map = world.resource::<WorldMap>().clone();
    let tether = world.get::<crate::dungeon::Tethered>(e).map(|t| t.0.zone.clone());
    while budget >= 1.0 && !pos.within(&goal, range) {
        let next = map.clamp(map.step_towards(pos, goal));
        if next == pos {
            break;
        }
        if let Some(z) = &tether
            && !map.in_zone(z, &next) {
                break;
            }
        pos = next;
        budget -= 1.0;
    }
    budget = budget.min(1.0);
    world.entity_mut(e).insert(pos);
    if let Some(mut m) = world.get_mut::<Movement>(e) {
        m.budget = budget;
    }
    pos.within(&goal, range)
}
