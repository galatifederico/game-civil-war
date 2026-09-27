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
            map.random_cell(z, &mut world.resource_mut::<SimRng>()).map(JobTarget::Cell)
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

/// Entities matching a filter from the chooser's point of view, with their positions.
pub fn candidates(world: &mut World, chooser: Entity, f: &Filter) -> Vec<(SimId, Option<Position>)> {
    let content = world.resource::<Content>().clone();
    let me_pos = world.get::<Position>(chooser).copied();
    let my_faction = world.get::<FactionMember>(chooser).map(|m| m.faction.clone());
    let min_wanted = f.min_wanted.as_ref().map(|t| threshold(world, t));
    let perception = world.resource::<Params>().get("ai.perception_range", 6.0) as i32;
    let ents = crate::sorted_entities::<SimId>(world);
    let mut out = Vec::new();
    for e in ents {
        if e == chooser || world.get::<Dead>(e).is_some() {
            continue;
        }
        let is_pawn = world.get::<Pawn>(e).is_some();
        let is_building = world.get::<Building>(e).is_some();
        if !is_pawn && !is_building {
            continue;
        }
        if let Some(p) = f.pawn {
            if p != is_pawn {
                continue;
            }
        }
        let pos = world.get::<Position>(e).copied();
        if let (Some(max), Some(a)) = (f.max_distance, me_pos) {
            if !pos.is_some_and(|b| a.within(&b, max)) {
                continue;
            }
        }
        if f.visible && !crate::infiltration::can_see(world, chooser, e, perception) {
            continue;
        }
        if f.not_detained && world.get::<Detained>(e).is_some() {
            continue;
        }
        let tags = crate::infiltration::visible_tags(world, e);
        if !f.tags_all.iter().all(|t| tags.contains(t))
            || (!f.tags_any.is_empty() && !f.tags_any.iter().any(|t| tags.contains(t)))
            || f.tags_none.iter().any(|t| tags.contains(t))
        {
            continue;
        }
        if let Some(b) = &f.building {
            if world.get::<Building>(e).is_none_or(|x| &x.def != b) {
                continue;
            }
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
        if let Some(t) = &f.template {
            if world.get::<TemplateId>(e).is_none_or(|x| &x.0 != t) {
                continue;
            }
        }
        let their = crate::infiltration::apparent_faction(world, e);
        if let Some(fa) = &f.faction {
            if their.as_ref() != Some(fa) {
                continue;
            }
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
        if let Some(t) = &f.has_item_tag {
            if world.get::<Inventory>(e).is_none_or(|i| i.count_tag(&content, t) == 0) {
                continue;
            }
        }
        if let Some(m) = f.min_money {
            if world.get::<Wallet>(e).is_none_or(|w| w.0 < m) {
                continue;
            }
        }
        if let Some(mw) = min_wanted {
            if world.get::<Wanted>(e).is_none_or(|w| w.level < mw) {
                continue;
            }
        }
        if let Some(s) = &f.has_status {
            if world.get::<StatusEffects>(e).is_none_or(|x| !x.has(s)) {
                continue;
            }
        }
        if let Some(c) = &f.condition {
            let ctx = EffectCtx::new(Some(e), Some(chooser), "filter");
            if !eval_condition(world, &ctx, c) {
                continue;
            }
        }
        let id = *world.get::<SimId>(e).unwrap();
        out.push((id, pos));
    }
    out
}
