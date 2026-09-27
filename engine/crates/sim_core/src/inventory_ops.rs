//! Item transfers that work on both pawns (slot-limited [`Inventory`]) and buildings ([`Stock`]).

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;

use crate::buildings::{Building, Owner};
use crate::content::Content;
use crate::factions::FactionMember;
use crate::inventory::{Inventory, Stock};
use crate::stats::Dead;

/// Gives items; returns how many were stored.
pub fn give(world: &mut World, e: Entity, item: &str, qty: u32) -> u32 {
    if let Some(mut s) = world.get_mut::<Stock>(e) {
        s.add(item, qty);
        return qty;
    }
    let content = world.resource::<Content>().clone();
    world.get_mut::<Inventory>(e).map_or(0, |mut i| i.add(&content, item, qty))
}

pub fn take(world: &mut World, e: Entity, item: &str, qty: u32) -> u32 {
    if let Some(mut s) = world.get_mut::<Stock>(e) {
        return s.remove(item, qty);
    }
    world.get_mut::<Inventory>(e).map_or(0, |mut i| i.remove(item, qty))
}

pub fn count(world: &World, e: Entity, item: &str) -> u32 {
    if let Some(s) = world.get::<Stock>(e) {
        return s.count(item);
    }
    world.get::<Inventory>(e).map_or(0, |i| i.count(item))
}

/// Moves items between two holders; returns how many moved (limited by what fits).
pub fn transfer(world: &mut World, from: Entity, to: Entity, item: &str, qty: u32) -> u32 {
    let have = count(world, from, item).min(qty);
    if have == 0 {
        return 0;
    }
    let moved = give(world, to, item, have);
    take(world, from, item, moved);
    moved
}

/// Everything a faction holds: members' inventories plus stock of buildings it owns.
pub fn faction_holdings(world: &mut World, faction: &str) -> BTreeMap<String, u32> {
    let mut out = BTreeMap::new();
    let mut q = world.query_filtered::<(&FactionMember, &Inventory), Without<Dead>>();
    for (m, inv) in q.iter(world) {
        if m.faction == faction {
            for (i, n) in inv.items() {
                *out.entry(i.clone()).or_insert(0) += n;
            }
        }
    }
    let mut qb = world.query::<(&Building, &Stock)>();
    for (b, s) in qb.iter(world) {
        if matches!(&b.owner, Owner::Faction(f) if f == faction) {
            for (i, n) in &s.0 {
                *out.entry(i.clone()).or_insert(0) += n;
            }
        }
    }
    out
}
