//! InventoryComponent with hard slot limits and per-category stacking, plus building stock.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::Content;

/// A slot holds items of one category; the slot's capacity is the smallest `stack_max` among them.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Slot {
    pub category: String,
    pub items: BTreeMap<String, u32>,
}

impl Slot {
    pub fn total(&self) -> u32 {
        self.items.values().sum()
    }
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Inventory {
    /// Maximum number of distinct slots (categories). 0 = unlimited.
    pub max_slots: u32,
    pub slots: Vec<Slot>,
}

impl Default for Inventory {
    fn default() -> Self {
        Self { max_slots: 3, slots: Vec::new() }
    }
}

const UNLIMITED_STACK: u32 = 999;

fn cap_of(content: &Content, item: &str) -> (String, u32) {
    content.items.get(item).map_or((item.to_string(), UNLIMITED_STACK), |d| {
        let cat = if d.category.is_empty() { d.id.clone() } else { d.category.clone() };
        (cat, if d.stack_max == 0 { UNLIMITED_STACK } else { d.stack_max })
    })
}

impl Inventory {
    pub fn with_slots(max_slots: u32) -> Self {
        Self { max_slots, slots: Vec::new() }
    }

    /// How many units of `item` would fit.
    pub fn room_for(&self, content: &Content, item: &str) -> u32 {
        let (cat, cap) = cap_of(content, item);
        match self.slots.iter().find(|s| s.category == cat) {
            Some(slot) => {
                let slot_cap = slot.items.keys().map(|i| cap_of(content, i).1).min().unwrap_or(cap).min(cap);
                slot_cap.saturating_sub(slot.total())
            }
            None if self.max_slots == 0 || (self.slots.len() as u32) < self.max_slots => cap,
            None => 0,
        }
    }

    /// Adds up to `qty` units; returns how many were actually added.
    pub fn add(&mut self, content: &Content, item: &str, qty: u32) -> u32 {
        let n = qty.min(self.room_for(content, item));
        if n == 0 {
            return 0;
        }
        let (cat, _) = cap_of(content, item);
        let slot = match self.slots.iter_mut().position(|s| s.category == cat) {
            Some(i) => &mut self.slots[i],
            None => {
                self.slots.push(Slot { category: cat, items: BTreeMap::new() });
                self.slots.last_mut().expect("just pushed")
            }
        };
        *slot.items.entry(item.to_string()).or_insert(0) += n;
        n
    }

    /// Removes up to `qty` units; returns how many were removed.
    pub fn remove(&mut self, item: &str, qty: u32) -> u32 {
        let mut removed = 0;
        for slot in &mut self.slots {
            if let Some(c) = slot.items.get_mut(item) {
                removed = qty.min(*c);
                *c -= removed;
                if *c == 0 {
                    slot.items.remove(item);
                }
                break;
            }
        }
        self.slots.retain(|s| !s.items.is_empty());
        removed
    }

    pub fn count(&self, item: &str) -> u32 {
        self.slots.iter().filter_map(|s| s.items.get(item)).sum()
    }

    pub fn items(&self) -> impl Iterator<Item = (&String, u32)> + '_ {
        self.slots.iter().flat_map(|s| s.items.iter().map(|(k, v)| (k, *v)))
    }

    /// First item (in slot order) carrying `tag`.
    pub fn first_with_tag(&self, content: &Content, tag: &str) -> Option<String> {
        self.items()
            .find(|(i, _)| content.items.get(*i).is_some_and(|d| d.tags.iter().any(|t| t == tag)))
            .map(|(i, _)| i.clone())
    }

    pub fn count_tag(&self, content: &Content, tag: &str) -> u32 {
        self.items()
            .filter(|(i, _)| content.items.get(*i).is_some_and(|d| d.tags.iter().any(|t| t == tag)))
            .map(|(_, n)| n)
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

/// Unlimited storage of buildings (shops, warehouses, vaults).
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Stock(pub BTreeMap<String, u32>);

impl Stock {
    pub fn add(&mut self, item: &str, qty: u32) {
        if qty > 0 {
            *self.0.entry(item.to_string()).or_insert(0) += qty;
        }
    }

    pub fn remove(&mut self, item: &str, qty: u32) -> u32 {
        let Some(c) = self.0.get_mut(item) else { return 0 };
        let n = qty.min(*c);
        *c -= n;
        if *c == 0 {
            self.0.remove(item);
        }
        n
    }

    pub fn count(&self, item: &str) -> u32 {
        self.0.get(item).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ItemDef;

    fn content() -> Content {
        let mut pack = crate::content::ContentPack::default();
        for (id, cat, max) in [("pane", "cibo", 5), ("salame", "cibo", 3), ("katana", "arma", 1), ("santino", "coll", 10), ("vino", "alcol", 4)] {
            pack.items.push(ItemDef { id: id.into(), category: cat.into(), stack_max: max, ..Default::default() });
        }
        Content::from_packs(vec![pack]).unwrap()
    }

    #[test]
    fn slots_and_category_stacking() {
        let c = content();
        let mut inv = Inventory::default();
        assert_eq!(inv.add(&c, "pane", 4), 4);
        // Same category shares the slot; capacity is the smallest stack_max (3) → already full.
        assert_eq!(inv.add(&c, "salame", 2), 0);
        assert_eq!(inv.add(&c, "katana", 3), 1);
        assert_eq!(inv.add(&c, "santino", 2), 2);
        // Fourth distinct category does not fit in 3 slots.
        assert_eq!(inv.add(&c, "vino", 1), 0);
        assert_eq!(inv.remove("katana", 1), 1);
        assert_eq!(inv.add(&c, "vino", 1), 1);
        assert_eq!(inv.count("pane"), 4);
    }
}
