//! Outside supply: a steady flow of goods from beyond the map that keeps shops from running dry.
//!
//! Every `SupplyDef::interval` ticks each supply looks at the shops selling its item, computes how far
//! their stock is below `per_shop` each and imports the gap (at most `max_per_day`), scaled by a random
//! `variation` and by the active global modifiers (strikes, festivals, floods…). Local production lands
//! in the shops first, so the more the world produces, the less is imported: locals cover a variable
//! share of a total that stays stable. Buildings that export the item are skipped (no import-export
//! arbitrage); the owner faction pays `cost` × base price per unit when it can afford it.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::buildings::{Building, GlobalModifiers, Owner, Shop};
use crate::content::Content;
use crate::events::{EventBuilder, EventLog};
use crate::factions::Factions;
use crate::inventory::Stock;
use crate::time::SimClock;

/// Last delivery per supply: (imported, units already in the shops before the delivery).
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct SupplyStats {
    pub last: BTreeMap<String, (u32, u32)>,
}

impl SupplyStats {
    /// Share of the shops' stock that came from local production at the last delivery (0..1).
    pub fn local_share(&self, supply: &str) -> Option<f32> {
        self.last.get(supply).map(|(imported, local)| {
            let total = imported + local;
            if total == 0 { 1.0 } else { *local as f32 / total as f32 }
        })
    }
}

pub fn imports(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    if tick == 0 {
        return;
    }
    let content = world.resource::<Content>().clone();
    let mut delivered: Vec<String> = Vec::new();
    for s in content.supplies.values() {
        let interval = if s.interval == 0 { 24 } else { s.interval };
        if !tick.is_multiple_of(interval) {
            continue;
        }
        let Some(item) = content.items.get(&s.item) else { continue };
        // Shops selling the item, emptiest first (ties by id: deterministic).
        let mut shops: Vec<(u32, Entity)> = Vec::new();
        for e in crate::sorted_entities::<Shop>(world) {
            let (Some(b), Some(shop)) = (world.get::<Building>(e), world.get::<Shop>(e)) else { continue };
            let Some(def) = content.buildings.get(&b.def) else { continue };
            if b.hp <= 0.0 || !shop.catalog.contains_key(&s.item) || def.exports.contains(&s.item) {
                continue;
            }
            if s.shop_tag.as_ref().is_some_and(|t| !def.tags.contains(t)) {
                continue;
            }
            shops.push((world.get::<Stock>(e).map_or(0, |st| st.count(&s.item)), e));
        }
        if shops.is_empty() {
            continue;
        }
        let local: u32 = shops.iter().map(|(n, _)| *n).sum();
        let gap: u32 = shops.iter().map(|(n, _)| s.per_shop.saturating_sub(*n)).sum();
        let factor = world.resource::<GlobalModifiers>().supply_factor(world.resource::<Content>(), &s.item, &item.tags);
        let noise = 1.0;
        let cap = if s.max_per_day == 0 { u32::MAX } else { s.max_per_day };
        let mut amount = ((gap.min(cap) as f32) * factor * noise).round().max(0.0) as u32;
        let wanted = amount;
        shops.sort_by_key(|(n, _)| *n);
        // Round-robin one unit at a time to the emptiest shops, paid by their owners.
        let unit_cost = item.base_price as f64 * s.cost as f64;
        let mut levels: Vec<u32> = shops.iter().map(|(n, _)| *n).collect();
        while amount > 0 {
            let Some(i) = (0..shops.len()).filter(|i| levels[*i] < s.per_shop.max(1) * 2).min_by_key(|i| levels[*i]) else { break };
            let e = shops[i].1;
            if unit_cost > 0.0
                && let Some(Owner::Faction(f)) = world.get::<Building>(e).map(|b| b.owner.clone())
            {
                let mut fs = world.resource_mut::<Factions>();
                let Some(st) = fs.states.get_mut(&f) else { break };
                if st.treasury < unit_cost {
                    levels[i] = u32::MAX; // this owner is broke: skip its shop
                    if levels.iter().all(|l| *l == u32::MAX) {
                        break;
                    }
                    continue;
                }
                st.treasury -= unit_cost;
            }
            world.get_mut::<Stock>(e).unwrap().add(&s.item, 1);
            levels[i] += 1;
            amount -= 1;
        }
        let imported = wanted - amount;
        world.resource_mut::<SupplyStats>().last.insert(s.id.clone(), (imported, local));
        if imported > 0 {
            delivered.push(format!("{imported} {}", item.name));
        }
    }
    if !delivered.is_empty() {
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new("supply", format!("Rifornimenti da fuori: {}", delivered.join(", "))).tags(["economy", "supply"]),
        );
    }
}
