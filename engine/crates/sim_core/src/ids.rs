//! Stable simulation ids. Bevy `Entity` values are an implementation detail; everything that leaves the
//! engine (API, MCP, events, snapshots) talks in [`SimId`].

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct SimId(pub u64);

impl std::fmt::Display for SimId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Maps [`SimId`] to live entities and hands out new ids in creation order.
#[derive(Resource, Debug, Default)]
pub struct IdIndex {
    next: u64,
    map: BTreeMap<SimId, Entity>,
}

impl IdIndex {
    pub fn next_id(&self) -> u64 {
        self.next
    }

    /// Restores the counter after loading a save.
    pub fn set_next(&mut self, next: u64) {
        self.next = next;
    }

    pub fn allocate(&mut self) -> SimId {
        self.next += 1;
        SimId(self.next)
    }

    pub fn insert(&mut self, id: SimId, entity: Entity) {
        self.map.insert(id, entity);
    }

    pub fn remove(&mut self, id: SimId) {
        self.map.remove(&id);
    }

    pub fn get(&self, id: SimId) -> Option<Entity> {
        self.map.get(&id).copied()
    }

    /// All live entities in id order (the canonical deterministic iteration order).
    pub fn entities(&self) -> Vec<(SimId, Entity)> {
        self.map.iter().map(|(k, v)| (*k, *v)).collect()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}
