//! Movement along the grid with fractional speed, capacity penalties and tethering.

use bevy_ecs::prelude::*;

use crate::anatomy::Body;
use crate::content::Content;
use crate::map::{Position, WorldMap};
use crate::params::Params;
use crate::stats::Stats;

#[derive(Component, Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Movement {
    pub budget: f32,
    /// Remaining steps of the current path (next step last).
    pub path: Vec<Position>,
    /// Goal the path was computed for.
    pub goal: Option<Position>,
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

/// Moves `e` towards `goal` along an A* path (recomputed when the goal moves) until within `range`
/// or out of budget. Returns true when within range.
pub fn move_towards(world: &mut World, e: Entity, goal: Position, range: i32) -> bool {
    let Some(mut pos) = world.get::<Position>(e).copied() else { return true };
    if pos.within(&goal, range) {
        return true;
    }
    let speed = speed_of(world, e);
    let map = world.resource::<WorldMap>().clone();
    let tether = world.get::<crate::dungeon::Tethered>(e).map(|t| t.0.zone.clone());
    let mut mv = world.get::<Movement>(e).cloned().unwrap_or_default();
    mv.budget += speed;
    let stale = match mv.goal {
        Some(g) => g.layer != goal.layer || g.distance(&goal).is_none_or(|d| d > 2) || mv.path.is_empty(),
        None => true,
    };
    if stale {
        let allowed = |p: &Position| tether.as_ref().is_none_or(|z| map.in_zone(z, p));
        let max = world.resource::<crate::params::Params>().get("move.max_path_nodes", 20000.0) as usize;
        mv.path = match map.find_path(pos, goal, range, &allowed, max) {
            Some(mut p) => {
                p.reverse();
                p
            }
            None => Vec::new(),
        };
        mv.goal = Some(goal);
    }
    while mv.budget >= 1.0 && !pos.within(&goal, range) {
        let next = match mv.path.pop() {
            Some(n) => n,
            None => {
                // No path (unreachable): try a direct step, never through walls.
                let n = map.clamp(map.step_towards(pos, goal));
                if n == pos || map.blocked(&n) || tether.as_ref().is_some_and(|z| !map.in_zone(z, &n)) {
                    break;
                }
                n
            }
        };
        if map.blocked(&next) {
            mv.path.clear();
            break;
        }
        pos = next;
        mv.budget -= 1.0;
    }
    mv.budget = mv.budget.min(1.0);
    world.entity_mut(e).insert(pos);
    world.entity_mut(e).insert(mv);
    pos.within(&goal, range)
}
