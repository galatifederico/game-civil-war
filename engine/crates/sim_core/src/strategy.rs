//! Strategic layer of AI-run factions: data-defined goals (e.g. "collect the relics") become faction
//! jobs on the board, targeting the pawns or buildings of other factions that hold the wanted items.

use bevy_ecs::prelude::*;

use crate::buildings::{Building, Owner};
use crate::content::Content;
use crate::effects::{eval_condition, EffectCtx};
use crate::factions::{FactionMember, Factions};
use crate::ids::SimId;
use crate::inventory::{Inventory, Stock};
use crate::jobs::{JobBoard, JobTarget};
use crate::rng::SimRng;
use crate::stats::Dead;
use crate::time::SimClock;

pub fn faction_goals(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let content = world.resource::<Content>().clone();
    if content.factions.values().all(|f| f.goals.is_empty()) {
        return;
    }
    let holdings = crate::inventory_ops::all_faction_holdings(world);
    // Every holder of every item: (item, holder id, holder faction).
    let mut holders: Vec<(String, SimId, Option<String>)> = Vec::new();
    let mut q = world.query_filtered::<(&SimId, &Inventory, Option<&FactionMember>), Without<Dead>>();
    for (id, inv, m) in q.iter(world) {
        for (i, _) in inv.items() {
            holders.push((i.clone(), *id, m.map(|m| m.faction.clone())));
        }
    }
    let mut qb = world.query::<(&SimId, &Stock, &Building)>();
    for (id, s, b) in qb.iter(world) {
        let owner = match &b.owner {
            Owner::Faction(f) => Some(f.clone()),
            _ => None,
        };
        for i in s.0.keys() {
            holders.push((i.clone(), *id, owner.clone()));
        }
    }
    holders.sort();
    let factions = world.resource::<Factions>().clone();
    for f in content.factions.values() {
        let Some(state) = factions.states.get(&f.id) else { continue };
        if state.controlled_by.is_some() || state.absorbed_into.is_some() {
            continue;
        }
        for g in &f.goals {
            let interval = g.interval.max(1);
            if tick == 0 || tick % interval != 0 {
                continue;
            }
            if !eval_condition(world, &EffectCtx::new(None, None, format!("goal:{}", f.id)), &g.requires) {
                continue;
            }
            let open = world.resource::<JobBoard>().jobs.values().filter(|j| j.job == g.job && j.faction.as_deref() == Some(f.id.as_str())).count() as u32;
            if open >= g.max_open.max(1) {
                continue;
            }
            let own = holdings.get(&f.id);
            let targets: Vec<SimId> = holders
                .iter()
                .filter(|(item, _, owner)| {
                    owner.as_deref() != Some(f.id.as_str())
                        && content.items.get(item).is_some_and(|d| d.tags.contains(&g.item_tag))
                        && own.is_none_or(|h| !h.contains_key(item))
                })
                .map(|(_, id, _)| *id)
                .collect();
            let Some(target) = world.resource_mut::<SimRng>().pick(&targets).copied() else { continue };
            crate::jobs::post_job(world, &g.job, Some(f.id.clone()), JobTarget::Entity(target), g.priority, None);
        }
    }
}
