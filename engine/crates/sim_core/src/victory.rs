//! SetCollectionFramework and win conditions.

use std::collections::{BTreeMap, BTreeSet};

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{Content, VictoryKind};
use crate::effects::{apply_effects, eval_condition, EffectCtx};
use crate::events::{kind, EventBuilder, EventLog};
use crate::factions::{FactionMember, Factions, Leader, Players, Titles};
use crate::inventory::Inventory;
use crate::stats::Dead;
use crate::time::SimClock;

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Progress {
    /// (collection, holder key) already completed.
    pub completed: BTreeSet<(String, String)>,
    pub winner: Option<Winner>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Winner {
    pub faction: String,
    pub player: Option<String>,
    pub victory: String,
    pub tick: u64,
}

fn has_all(held: &BTreeMap<String, u32>, col: &crate::content::CollectionDef, content: &Content) -> bool {
    if !col.items.is_empty() {
        return col.items.iter().all(|i| held.get(i).copied().unwrap_or(0) > 0);
    }
    if let Some(tag) = &col.tag {
        let n = held.keys().filter(|i| content.items.get(*i).is_some_and(|d| d.tags.contains(tag))).count() as u32;
        return n >= col.count.max(1);
    }
    false
}

pub fn collections(world: &mut World) {
    let content = world.resource::<Content>().clone();
    if content.collections.is_empty() {
        return;
    }
    let tick = world.resource::<SimClock>().tick;
    let pawns = crate::sorted_entities::<Inventory>(world);
    let factions: Vec<String> = world.resource::<Factions>().states.keys().cloned().collect();
    let holdings = crate::inventory_ops::all_faction_holdings(world);
    let empty = BTreeMap::new();
    for col in content.collections.values() {
        let mut done: Vec<(String, Option<Entity>, Option<String>)> = Vec::new();
        if col.per_faction {
            for f in &factions {
                let held = holdings.get(f).unwrap_or(&empty);
                if has_all(held, col, &content) {
                    done.push((format!("faction:{f}"), None, Some(f.clone())));
                }
            }
        } else {
            for e in &pawns {
                if world.get::<Dead>(*e).is_some() {
                    continue;
                }
                let held: BTreeMap<String, u32> = world.get::<Inventory>(*e).unwrap().items().map(|(i, n)| (i.clone(), n)).collect();
                if has_all(&held, col, &content) {
                    let id = world.get::<crate::ids::SimId>(*e).copied().map_or(0, |i| i.0);
                    let f = world.get::<FactionMember>(*e).map(|m| m.faction.clone());
                    done.push((format!("entity:{id}"), Some(*e), f));
                }
            }
        }
        for (key, holder, faction) in done {
            if !world.resource_mut::<Progress>().completed.insert((col.id.clone(), key)) {
                continue;
            }
            if let Some(f) = &faction {
                if let Some(s) = world.resource_mut::<Factions>().states.get_mut(f) {
                    s.victory_points += col.victory_points;
                }
            }
            let who = holder.map(|h| crate::effects::name_of(world, h)).or(faction.clone()).unwrap_or_default();
            world.resource_mut::<EventLog>().push(
                tick,
                EventBuilder::new(kind::COLLECTION, format!("{who} completa la collezione {} (+{} PV)", col.name, col.victory_points))
                    .faction(faction.clone())
                    .news(0.6)
                    .tags(["collection", col.id.as_str()]),
            );
            let subject = holder.or_else(|| faction.as_ref().and_then(|f| leader_of(world, f)));
            apply_effects(world, &EffectCtx::new(subject, None, format!("collection:{}", col.id)), &col.bonus);
        }
    }
}

fn leader_of(world: &mut World, faction: &str) -> Option<Entity> {
    crate::sorted_entities::<FactionMember>(world)
        .into_iter()
        .find(|e| world.get::<FactionMember>(*e).is_some_and(|m| m.faction == faction) && world.get::<Dead>(*e).is_none())
}

/// Victory points from relics currently held, per faction.
pub fn relic_points(world: &mut World) -> BTreeMap<String, i64> {
    let content = world.resource::<Content>().clone();
    let factions: Vec<String> = world.resource::<Factions>().states.keys().cloned().collect();
    let holdings = crate::inventory_ops::all_faction_holdings(world);
    let mut out = BTreeMap::new();
    for f in factions {
        let pts: i64 = holdings.get(&f).map_or(0, |held| held.keys().filter_map(|i| content.items.get(i)).map(|d| d.victory_points).sum());
        out.insert(f, pts);
    }
    out
}

/// Total score of a faction: accumulated points plus relics held.
pub fn scores(world: &mut World) -> BTreeMap<String, i64> {
    let relics = relic_points(world);
    let f = world.resource::<Factions>();
    f.states.iter().map(|(k, s)| (k.clone(), s.victory_points + relics.get(k).copied().unwrap_or(0))).collect()
}

pub fn check_victory(world: &mut World) {
    if world.resource::<Progress>().winner.is_some() {
        return;
    }
    let content = world.resource::<Content>().clone();
    let tick = world.resource::<SimClock>().tick;
    let factions: Vec<String> = world.resource::<Factions>().states.keys().cloned().collect();
    let scores = scores(world);
    for v in &content.victory {
        let mut winner: Option<String> = None;
        match &v.kind {
            VictoryKind::HoldItems(items) => {
                let holdings = crate::inventory_ops::all_faction_holdings(world);
                for f in &factions {
                    let Some(held) = holdings.get(f) else { continue };
                    if items.iter().all(|i| held.get(i).copied().unwrap_or(0) > 0) {
                        winner = Some(f.clone());
                        break;
                    }
                }
            }
            VictoryKind::HoldTitle(t) => {
                if let Some(holder) = world.resource::<Titles>().holder(t) {
                    if let Some(e) = crate::lifecycle::entity_of(world, holder) {
                        if let Some(l) = world.get::<Leader>(e) {
                            winner = world.resource::<Players>().players.get(&l.player).map(|p| p.faction.clone());
                        }
                    }
                }
            }
            VictoryKind::VictoryPoints(n) => {
                winner = scores.iter().filter(|(_, s)| **s >= *n).max_by_key(|(_, s)| **s).map(|(f, _)| f.clone());
            }
            VictoryKind::Condition(c) => {
                if eval_condition(world, &EffectCtx::new(None, None, "victory"), c) {
                    winner = Some(String::new());
                }
            }
        }
        if let Some(f) = winner {
            let player = world.resource::<Players>().by_faction(&f).map(|p| p.id.clone());
            let fname = content.factions.get(&f).map_or(f.clone(), |d| d.name.clone());
            world.resource_mut::<Progress>().winner = Some(Winner { faction: f.clone(), player, victory: v.id.clone(), tick });
            world.resource_mut::<EventLog>().push(
                tick,
                EventBuilder::new(kind::VICTORY, format!("VITTORIA: {fname} — {}", v.name)).faction(Some(f)).news(1.0).tags(["victory"]),
            );
            return;
        }
    }
}
