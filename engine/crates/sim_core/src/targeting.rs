//! Resolution of data [`Selector`]s and [`Filter`]s into concrete job targets.

use bevy_ecs::prelude::*;

use crate::buildings::{Building, Shop};
use crate::content::{Content, Filter, Relation, Selector, Threshold};
use crate::crime::{Detained, Wanted};
use crate::effects::{eval_condition, EffectCtx};
use crate::factions::{FactionMember, Factions};
use crate::ids::SimId;
use crate::inventory::{Inventory, Stock};
use crate::jobs::JobTarget;
use crate::map::{Position, WorldMap};
use crate::params::Params;
use crate::rng::SimRng;
use crate::stats::{Dead, Pawn, TemplateId, Wallet};
use bevy_ecs::query::Has;
use crate::status::StatusEffects;

pub fn resolve(world: &mut World, chooser: Entity, sel: &Selector) -> Option<JobTarget> {
    match sel {
        Selector::None => Some(JobTarget::None),
        Selector::SelfTarget => Some(JobTarget::of_entity(world, chooser)),
        Selector::Nearest(f) => {
            let me = world.get::<Position>(chooser).copied();
            let mut c = candidates(world, chooser, f);
            c.sort_by_key(|(id, p)| (me.zip(*p).map_or(0, |(a, b)| a.cost(&b)), *id));
            c.first().map(|(id, _)| JobTarget::Entity(*id))
        }
        Selector::Random(f) => {
            let c = candidates(world, chooser, f);
            let i = world.resource_mut::<SimRng>().index(c.len())?;
            Some(JobTarget::Entity(c[i].0))
        }
        Selector::Zone(z) => {
            let map = world.resource::<WorldMap>().clone();
            let cell = map.random_cell(z, &mut world.resource_mut::<SimRng>())?;
            // A tethered pawn walks to the nearest point of the target zone that is inside its own zone.
            match world.get::<crate::dungeon::Tethered>(chooser) {
                Some(t) if !map.in_zone(&t.0.zone, &cell) => None,
                _ => Some(JobTarget::Cell(cell)),
            }
        }
        Selector::OwnFactionZone(tag) => {
            let f = world.get::<FactionMember>(chooser).map(|m| m.faction.clone())?;
            let zones = world.resource::<Content>().factions.get(&f)?.zones.clone();
            let map = world.resource::<WorldMap>().clone();
            let spec: Vec<String> = zones
                .into_iter()
                .filter(|z| tag.is_empty() || map.zone(z).is_some_and(|zz| zz.tags.iter().any(|t| t == tag)))
                .collect();
            let z = world.resource_mut::<SimRng>().pick(&spec)?.clone();
            map.random_cell(&z, &mut world.resource_mut::<SimRng>()).map(JobTarget::Cell)
        }
    }
}

fn threshold(world: &World, t: &Threshold) -> f32 {
    match t {
        Threshold::Value(v) => *v,
        Threshold::Param(k) => world.resource::<Params>().f(k),
    }
}

/// Snapshot of targetable entities (alive pawns and buildings, in id order) rebuilt once per AI pass,
/// so that target searches do not re-query and re-sort the world for every action of every pawn.
#[derive(Resource, Debug, Default, Clone)]
pub struct TargetIndex {
    pub tick: Option<u64>,
    pub entries: Vec<(SimId, Entity, Option<Position>, bool)>,
}

pub fn rebuild_index(world: &mut World) {
    let tick = world.resource::<crate::time::SimClock>().tick;
    let mut q = world.query_filtered::<(Entity, &SimId, Option<&Position>, Has<Pawn>, Has<Building>), Without<Dead>>();
    let mut entries: Vec<(SimId, Entity, Option<Position>, bool)> = q
        .iter(world)
        .filter(|(_, _, _, pawn, building)| *pawn || *building)
        .map(|(e, id, p, pawn, _)| (*id, e, p.copied(), pawn))
        .collect();
    entries.sort_unstable_by_key(|x| x.0);
    world.insert_resource(TargetIndex { tick: Some(tick), entries });
}

/// Entities matching a filter from the chooser's point of view, with their positions.
pub fn candidates(world: &mut World, chooser: Entity, f: &Filter) -> Vec<(SimId, Option<Position>)> {
    let tick = world.resource::<crate::time::SimClock>().tick;
    if world.get_resource::<TargetIndex>().is_none_or(|i| i.tick != Some(tick)) {
        rebuild_index(world);
    }
    let content = world.resource::<Content>().clone();
    let me_pos = world.get::<Position>(chooser).copied();
    let my_faction = world.get::<FactionMember>(chooser).map(|m| m.faction.clone());
    let min_wanted = f.min_wanted.as_ref().map(|t| threshold(world, t));
    let perception = world.resource::<Params>().get("ai.perception_range", 6.0) as i32;
    let tether = world.get::<crate::dungeon::Tethered>(chooser).map(|t| t.0.zone.clone());
    let map = world.resource::<WorldMap>().clone();
    let index = std::mem::take(&mut world.resource_mut::<TargetIndex>().entries);
    let mut out = Vec::new();
    for &(id, e, pos, is_pawn) in &index {
        if e == chooser || world.get::<Dead>(e).is_some() {
            continue;
        }
        if let Some(p) = f.pawn
            && p != is_pawn {
                continue;
            }
        // Tethered pawns cannot leave their zone, so they only consider what is inside it.
        if let Some(z) = &tether
            && !pos.is_some_and(|p| map.in_zone(z, &p)) {
                continue;
            }
        // Positions in the index are from the start of the pass: good enough for choosing targets.
        if let (Some(max), Some(a)) = (f.max_distance, me_pos)
            && !pos.is_some_and(|b| a.within(&b, max)) {
                continue;
            }
        if f.visible && !crate::infiltration::can_see(world, chooser, e, perception) {
            continue;
        }
        if f.not_detained && world.get::<Detained>(e).is_some() {
            continue;
        }
        if !f.tags_all.is_empty() || !f.tags_any.is_empty() || !f.tags_none.is_empty() {
            let tags = crate::infiltration::visible_tags(world, e);
            if !f.tags_all.iter().all(|t| tags.contains(t))
                || (!f.tags_any.is_empty() && !f.tags_any.iter().any(|t| tags.contains(t)))
                || f.tags_none.iter().any(|t| tags.contains(t))
            {
                continue;
            }
        }
        if let Some(b) = &f.building
            && world.get::<Building>(e).is_none_or(|x| &x.def != b) {
                continue;
            }
        if let Some(bt) = &f.building_tag {
            let ok = world.get::<Building>(e).and_then(|x| content.buildings.get(&x.def)).is_some_and(|d| d.tags.contains(bt));
            if !ok {
                continue;
            }
        }
        if let Some(st) = &f.sells_tag {
            let ok = match (world.get::<Shop>(e), world.get::<Stock>(e)) {
                (Some(shop), Some(stock)) => shop.catalog.keys().any(|i| {
                    stock.count(i) > 0 && content.items.get(i).is_some_and(|d| d.tags.contains(st))
                }),
                _ => false,
            };
            if !ok {
                continue;
            }
        }
        if let Some(t) = &f.template
            && world.get::<TemplateId>(e).is_none_or(|x| &x.0 != t) {
                continue;
            }
        let their = if f.faction.is_some() || f.relation.is_some() { crate::infiltration::apparent_faction(world, e) } else { None };
        if let Some(fa) = &f.faction
            && their.as_ref() != Some(fa) {
                continue;
            }
        if let Some(rel) = f.relation {
            let fs = world.resource::<Factions>();
            // Pawns without a faction are "other" but neither hostile nor friendly.
            let ok = match (&my_faction, &their) {
                (Some(mine), Some(theirs)) => match rel {
                    Relation::Same => mine == theirs,
                    Relation::Other => mine != theirs,
                    Relation::Hostile => fs.hostile(mine, theirs),
                    Relation::Friendly => fs.friendly(mine, theirs),
                },
                _ => rel == Relation::Other,
            };
            if !ok {
                continue;
            }
        }
        if let Some(t) = &f.has_item_tag
            && world.get::<Inventory>(e).is_none_or(|i| i.count_tag(&content, t) == 0) {
                continue;
            }
        if let Some(m) = f.min_money
            && world.get::<Wallet>(e).is_none_or(|w| w.0 < m) {
                continue;
            }
        if let Some(mw) = min_wanted
            && world.get::<Wanted>(e).is_none_or(|w| w.level < mw) {
                continue;
            }
        if let Some(s) = &f.has_status
            && world.get::<StatusEffects>(e).is_none_or(|x| !x.has(s)) {
                continue;
            }
        if let Some(c) = &f.condition {
            let ctx = EffectCtx::new(Some(e), Some(chooser), "filter");
            if !eval_condition(world, &ctx, c) {
                continue;
            }
        }
        out.push((id, pos));
    }
    world.resource_mut::<TargetIndex>().entries = index;
    out
}
