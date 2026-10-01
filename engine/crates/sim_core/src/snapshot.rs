//! ActivityStateAPI and world snapshots for the UI, the admin panel and MCP; state hashing for replay tests.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ai::{Brain, ScoreEntry};
use crate::anatomy::Body;
use crate::buildings::{Building, GlobalModifiers, Owner, Shop};
use crate::content::Content;
use crate::crime::{Detained, Wanted};
use crate::factions::{Dissent, FactionMember, Factions, Leader, Players, Titles};
use crate::ids::SimId;
use crate::infiltration::{Cover, Disguise};
use crate::inventory::{Inventory, Stock};
use crate::jobs::{JobBoard, Task, WorkPriorities};
use crate::map::{Position, WorldMap};
use crate::market::Market;
use crate::press::{Article, Feed, Notebook};
use crate::stats::{Classes, Dead, DisplayName, Needs, Race, Stats, TemplateId, Virtual, Wallet};
use crate::status::StatusEffects;
use crate::time::SimClock;
use crate::victory::Progress;

/// What a pawn is doing, ready for the client: action, progress bar, emotional state.
#[derive(Component, Debug, Clone, Default, Serialize, PartialEq, Deserialize)]
pub struct ActivityState {
    pub action: Option<String>,
    pub label: String,
    pub job: Option<String>,
    /// 0..1
    pub progress: f32,
    pub mood: String,
    pub morale: f32,
    pub target: Option<SimId>,
    pub zone: Option<String>,
    pub flags: Vec<String>,
}

pub fn mood_label(morale: f32) -> &'static str {
    match morale {
        m if m < 20.0 => "disperato",
        m if m < 40.0 => "giù di morale",
        m if m < 60.0 => "tranquillo",
        m if m < 80.0 => "contento",
        _ => "euforico",
    }
}

pub fn update_activity(world: &mut World) {
    let morale_stat = world.resource::<Content>().bindings.morale.clone();
    let map = world.resource::<WorldMap>().clone();
    let mut q = world.query::<(
        &Task,
        &Stats,
        Option<&Position>,
        Option<&Dead>,
        Option<&Detained>,
        Option<&Disguise>,
        &StatusEffects,
        Option<&crate::player::KnockedOut>,
        &mut ActivityState,
    )>();
    for (task, stats, pos, dead, detained, disguise, statuses, knocked, mut act) in q.iter_mut(world) {
        let morale = stats.get(&morale_stat);
        let (progress, job, target) = task.job.as_ref().map_or((0.0, None, None), |j| {
            let p = if j.required > 0.0 { (j.progress / j.required).clamp(0.0, 1.0) } else { 0.0 };
            let target = match j.target {
                crate::jobs::JobTarget::Entity(id) => Some(id),
                _ => None,
            };
            (p, Some(if j.job.is_empty() { j.ability.clone().unwrap_or_default() } else { j.job.clone() }), target)
        });
        let mut flags = Vec::new();
        if dead.is_some() {
            flags.push("dead".to_string());
        }
        if detained.is_some() {
            flags.push("detained".to_string());
        }
        if disguise.is_some() {
            flags.push("disguised".to_string());
        }
        if knocked.is_some() {
            flags.push("knocked_out".to_string());
        }
        flags.extend(statuses.active.keys().cloned());
        let label = if dead.is_some() {
            "Morto".to_string()
        } else if knocked.is_some() {
            "Al tappeto".to_string()
        } else if detained.is_some() {
            "In cella".to_string()
        } else if task.label.is_empty() {
            "Ozia".to_string()
        } else {
            task.label.clone()
        };
        *act = ActivityState {
            action: task.action.clone(),
            label,
            job,
            progress,
            mood: mood_label(morale).to_string(),
            morale,
            target,
            zone: pos.and_then(|p| map.zone_name_at(p)).map(str::to_string),
            flags,
        };
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityView {
    pub id: SimId,
    pub name: String,
    pub kind: &'static str,
    pub template: Option<String>,
    pub race: Option<String>,
    pub classes: Vec<String>,
    pub faction: Option<String>,
    pub rank: Option<String>,
    pub pos: Option<Position>,
    pub activity: Option<ActivityState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truth: Option<TrueIdentity>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub stats: BTreeMap<String, f32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub needs: BTreeMap<String, f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub statuses: Vec<StatusView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inventory: Vec<(String, u32)>,
    pub money: f64,
    pub wanted: f32,
    pub dissent: f32,
    pub health: f32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub missing_parts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub building: Option<BuildingView>,
    pub leader_of: Option<String>,
    pub dead: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrueIdentity {
    pub name: String,
    pub race: String,
    pub faction: Option<String>,
    pub cover: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusView {
    pub id: String,
    pub name: String,
    pub severity: f32,
    pub stage: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildingView {
    pub def: String,
    pub hp: f32,
    pub max_hp: f32,
    pub owner: Owner,
    pub stock: BTreeMap<String, u32>,
    pub sells: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FactionView {
    pub id: String,
    pub name: String,
    pub treasury: f64,
    pub members: usize,
    pub victory_points: i64,
    pub controlled_by: Option<String>,
    pub absorbed_into: Option<String>,
    pub relations: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PriceView {
    pub item: String,
    pub name: String,
    pub base: f64,
    pub price: f64,
    pub supply: f32,
    pub demand: f32,
    pub shocks: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorldSnapshot {
    pub tick: u64,
    pub entities: Vec<EntityView>,
    pub factions: Vec<FactionView>,
    pub market: Vec<PriceView>,
    pub disruption: f32,
    pub feed_name: String,
    pub feed: Vec<Article>,
    pub players: serde_json::Value,
    pub titles: BTreeMap<String, Option<SimId>>,
    pub modifiers: Vec<String>,
    pub open_jobs: usize,
    pub winner: Option<crate::victory::Winner>,
    pub scores: BTreeMap<String, i64>,
    /// Quarters: owner and influence per faction.
    pub territories: BTreeMap<String, crate::territory::TerritoryState>,
}

/// One entity. With `truth` the admin view shows hidden identities; otherwise the apparent one.
pub fn entity_view(world: &World, e: Entity, truth: bool) -> Option<EntityView> {
    let id = *world.get::<SimId>(e)?;
    let content = world.resource::<Content>();
    let disguise = world.get::<Disguise>(e);
    let hide = disguise.is_some() && !truth;
    let building = world.get::<Building>(e).map(|b| BuildingView {
        def: b.def.clone(),
        hp: b.hp,
        max_hp: b.max_hp,
        owner: b.owner.clone(),
        stock: world.get::<Stock>(e).map(|s| s.0.clone()).unwrap_or_default(),
        sells: world.get::<Shop>(e).map(|s| s.catalog.keys().cloned().collect()).unwrap_or_default(),
    });
    let kind = if building.is_some() { "building" } else if world.get::<Virtual>(e).is_some() { "virtual" } else { "pawn" };
    let statuses = world
        .get::<StatusEffects>(e)
        .map(|s| {
            s.active
                .iter()
                .filter_map(|(k, v)| {
                    let d = content.statuses.get(k)?;
                    if d.hidden && !truth {
                        return None;
                    }
                    Some(StatusView {
                        id: k.clone(),
                        name: d.name.clone(),
                        severity: v.severity,
                        stage: v.stage.and_then(|i| d.stages.get(i)).map(|s| s.name.clone()),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let visible_stats: BTreeMap<String, f32> = world
        .get::<Stats>(e)
        .map(|s| {
            content
                .stats
                .values()
                .filter(|d| d.visible || truth)
                .map(|d| (d.id.clone(), (s.get(&d.id) * 10.0).round() / 10.0))
                .collect()
        })
        .unwrap_or_default();
    let member = world.get::<FactionMember>(e);
    Some(EntityView {
        id,
        name: crate::infiltration::apparent_name(world, e),
        kind,
        template: if hide { None } else { world.get::<TemplateId>(e).map(|t| t.0.clone()) },
        race: crate::infiltration::apparent_race(world, e).filter(|_| building.is_none()),
        classes: world.get::<Classes>(e).map(|c| c.0.clone()).unwrap_or_default(),
        faction: crate::infiltration::apparent_faction(world, e).or_else(|| match building.as_ref().map(|b| &b.owner) {
            Some(Owner::Faction(f)) => Some(f.clone()),
            _ => None,
        }),
        rank: if hide { None } else { member.map(|m| m.rank.clone()) },
        pos: world.get::<Position>(e).copied(),
        activity: world.get::<crate::snapshot::ActivityState>(e).cloned(),
        truth: (truth && disguise.is_some()).then(|| TrueIdentity {
            name: world.get::<DisplayName>(e).map_or("?".into(), |n| n.0.clone()),
            race: world.get::<Race>(e).map_or("?".into(), |r| r.0.clone()),
            faction: member.map(|m| m.faction.clone()),
            cover: world.get::<Cover>(e).map_or(0.0, |c| c.0),
        }),
        stats: visible_stats,
        needs: world.get::<Needs>(e).map(|n| n.0.iter().map(|(k, v)| (k.clone(), (v * 100.0).round() / 100.0)).collect()).unwrap_or_default(),
        statuses,
        inventory: world.get::<Inventory>(e).map(|i| i.items().map(|(k, v)| (k.clone(), v)).collect()).unwrap_or_default(),
        money: world.get::<Wallet>(e).map_or(0.0, |w| (w.0 * 100.0).round() / 100.0),
        wanted: world.get::<Wanted>(e).map_or(0.0, |w| (w.level * 10.0).round() / 10.0),
        dissent: world.get::<Dissent>(e).map_or(0.0, |d| (d.0 * 10.0).round() / 10.0),
        health: world.get::<Body>(e).map_or(1.0, |b| (b.health_ratio() * 100.0).round() / 100.0),
        missing_parts: world.get::<Body>(e).map(|b| b.missing_parts().into_iter().map(String::from).collect()).unwrap_or_default(),
        building,
        leader_of: world.get::<Leader>(e).map(|l| l.player.clone()),
        dead: world.get::<Dead>(e).is_some(),
    })
}

pub fn snapshot(world: &mut World, truth: bool) -> WorldSnapshot {
    let ents = crate::sorted_entities::<SimId>(world);
    let entities: Vec<EntityView> = ents.iter().filter_map(|e| entity_view(world, *e, truth)).collect();
    let scores = crate::victory::scores(world);
    let content = world.resource::<Content>();
    let factions = world.resource::<Factions>();
    let faction_views = factions
        .states
        .iter()
        .map(|(id, s)| FactionView {
            id: id.clone(),
            name: content.factions.get(id).map_or(id.clone(), |f| f.name.clone()),
            treasury: (s.treasury * 100.0).round() / 100.0,
            members: entities.iter().filter(|e| !e.dead && e.kind != "building" && e.faction.as_deref() == Some(id.as_str())).count(),
            victory_points: s.victory_points,
            controlled_by: s.controlled_by.clone(),
            absorbed_into: s.absorbed_into.clone(),
            relations: s.relations.clone(),
        })
        .collect();
    let market = world.resource::<Market>();
    let feed = world.resource::<Feed>();
    WorldSnapshot {
        tick: world.resource::<SimClock>().tick,
        entities,
        factions: faction_views,
        market: market
            .items
            .iter()
            .map(|(k, m)| PriceView {
                item: k.clone(),
                name: content.items.get(k).map_or(k.clone(), |d| d.name.clone()),
                base: m.base,
                price: (m.price * 100.0).round() / 100.0,
                supply: m.supply,
                demand: m.demand,
                shocks: m.shocks.len(),
            })
            .collect(),
        disruption: market.disruption,
        feed_name: feed.name.clone(),
        feed: feed
            .articles
            .iter()
            .rev()
            .take(20)
            .map(|a| {
                let mut a = a.clone();
                if !truth && a.truth == crate::content::Truth::Fake {
                    a.truth = crate::content::Truth::Real;
                }
                a
            })
            .collect(),
        players: serde_json::to_value(&world.resource::<Players>().players).unwrap_or_default(),
        titles: world.resource::<Titles>().holders.clone(),
        modifiers: world.resource::<GlobalModifiers>().active.iter().map(|m| m.name.clone()).collect(),
        open_jobs: world.resource::<JobBoard>().jobs.len(),
        winner: world.resource::<Progress>().winner.clone(),
        scores,
        territories: world.resource::<crate::territory::Territories>().zones.clone(),
    }
}

/// What a faction can see: its own members and buildings always; other pawns only within the
/// perception range of its pawns or the vision radius of its buildings (e.g. garden-gnome cameras).
#[derive(Debug, Clone, Serialize)]
pub struct FogView {
    pub faction: String,
    /// (position, radius) of every observer, for the client to shade the fog.
    pub observers: Vec<(Position, i32)>,
    pub visible: std::collections::BTreeSet<SimId>,
}

pub fn fog_of_war(world: &mut World, faction: &str) -> FogView {
    let content = world.resource::<Content>().clone();
    let base = world.resource::<crate::params::Params>().get("ai.perception_range", 6.0) as i32;
    let mut observers = Vec::new();
    let mut visible = std::collections::BTreeSet::new();
    for e in crate::sorted_entities::<SimId>(world) {
        let id = *world.get::<SimId>(e).unwrap();
        let own = world.get::<FactionMember>(e).is_some_and(|m| m.faction == faction && world.get::<Dead>(e).is_none());
        let building = world.get::<Building>(e);
        let owned_building = building.is_some_and(|b| matches!(&b.owner, Owner::Faction(f) if f == faction));
        if building.is_some() {
            visible.insert(id); // landmarks are on the map
        }
        let Some(pos) = world.get::<Position>(e).copied() else { continue };
        if own {
            visible.insert(id);
            observers.push((pos, base));
        } else if owned_building {
            let vision = building.and_then(|b| content.buildings.get(&b.def)).map_or(0, |d| d.vision);
            if vision > 0 {
                observers.push((pos, vision));
            }
        }
    }
    // Quarters held by the faction are watched over entirely.
    let map = world.resource::<crate::map::WorldMap>().clone();
    for (zid, t) in &world.resource::<crate::territory::Territories>().zones {
        if t.owner.as_deref() == Some(faction)
            && let Some(z) = map.zones.iter().find(|z| &z.id == zid)
        {
            observers.push((Position::new(z.layer, z.x + z.w / 2, z.y + z.h / 2), z.w.max(z.h) / 2 + 2));
        }
    }
    for e in crate::sorted_entities::<crate::stats::Pawn>(world) {
        let id = *world.get::<SimId>(e).unwrap();
        if visible.contains(&id) || crate::infiltration::invisible(world, e) {
            continue;
        }
        if let Some(p) = world.get::<Position>(e)
            && observers.iter().any(|(o, r)| o.within(p, *r)) {
                visible.insert(id);
            }
    }
    FogView { faction: faction.to_string(), observers, visible }
}

/// Utility AI inspection for one entity.
pub fn ai_inspect(world: &World, e: Entity) -> serde_json::Value {
    let b = world.get::<Brain>(e);
    let t = world.get::<Task>(e);
    serde_json::json!({
        "current": b.and_then(|b| b.current.clone()),
        "momentum": b.map(|b| b.momentum),
        "evaluated_at": b.map(|b| b.last_tick),
        "scores": b.map(|b| b.last.clone()).unwrap_or_default() as Vec<ScoreEntry>,
        "cooldowns": b.map(|b| b.cooldowns.clone()),
        "task": t.map(|t| serde_json::json!({ "label": t.label, "forced": t.forced, "job": t.job })),
        "work_priorities": world.get::<WorkPriorities>(e).map(|w| w.matrix()),
        "notebook": world.get::<Notebook>(e).map(|n| n.scoops.clone()),
    })
}

/// FNV-1a hash of the full (truth) snapshot plus the RNG state.
pub fn state_hash(world: &mut World) -> u64 {
    let snap = snapshot(world, true);
    let json = serde_json::to_string(&snap).unwrap_or_default();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in json.as_bytes().iter().chain(world.resource::<crate::rng::SimRng>().state().to_le_bytes().iter()) {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}
