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

/// Moves `e` towards `goal` until within `range` or out of budget. Across maps it walks to the next
/// passage (door, stairs, map border) of the shortest map route, crosses it, and so on; inside a map it
/// follows an A* path, recomputed when the local goal moves. Returns true when within range.
pub fn move_towards(world: &mut World, e: Entity, goal: Position, range: i32) -> bool {
    let Some(mut pos) = world.get::<Position>(e).copied() else { return true };
    if pos.within(&goal, range) {
        return true;
    }
    let speed = speed_of(world, e);
    let map = world.resource::<WorldMap>().clone();
    let tether = world.get::<crate::dungeon::Tethered>(e).map(|t| t.0.zone.clone());
    let max = world.resource::<crate::params::Params>().get("move.max_path_nodes", 20000.0) as usize;
    let mut mv = world.get::<Movement>(e).cloned().unwrap_or_default();
    mv.budget += speed;
    let mut guard = 0;
    while mv.budget >= 1.0 && !pos.within(&goal, range) && guard < 16 {
        guard += 1;
        // Local goal: the target itself, or the passage towards its map.
        let (local, lrange) = if pos.layer == goal.layer {
            (goal, range)
        } else {
            match map.next_portal(pos, goal.layer) {
                Some(hop) if hop == pos => {
                    // Standing on a passage: cross it.
                    match map.portal_exit(pos) {
                        Some(exit) => {
                            pos = exit;
                            mv.budget -= 1.0;
                            mv.path.clear();
                            mv.goal = None;
                            continue;
                        }
                        None => break,
                    }
                }
                Some(hop) => (hop, 0),
                None => break,
            }
        };
        let stale = mv.path.is_empty() || mv.goal.is_none_or(|g| g.layer != local.layer || g.distance(&local).is_none_or(|d| d > 2));
        if stale {
            let layer = pos.layer;
            let allowed = |p: &Position| p.layer == layer && tether.as_ref().is_none_or(|z| map.in_zone(z, p));
            mv.path = map
                .find_path_local(pos, local, lrange, &allowed, max)
                .map(|mut p| {
                    p.reverse();
                    p
                })
                .unwrap_or_default();
            mv.goal = Some(local);
        }
        let next = match mv.path.pop() {
            Some(n) => n,
            None => {
                // No path: try a direct step, never through walls.
                let n = map.clamp(map.step_towards(pos, local));
                if n == pos || n.layer != pos.layer || map.blocked(&n) || tether.as_ref().is_some_and(|z| !map.in_zone(z, &n)) {
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
