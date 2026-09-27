//! Spatial model: stacked grid layers (surface, underground…), rectangular zones, portals between
//! layers, and sparse per-cell environmental state (dirt, fluids, pathogen load).

use std::collections::{BTreeMap, VecDeque};

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{MapDef, Vector};
use crate::rng::SimRng;

#[derive(
    Component, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct Position {
    pub layer: u16,
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub const fn new(layer: u16, x: i32, y: i32) -> Self {
        Self { layer, x, y }
    }

    /// Chebyshev distance on the same layer; `None` across layers.
    pub fn distance(&self, other: &Position) -> Option<i32> {
        (self.layer == other.layer).then(|| (self.x - other.x).abs().max((self.y - other.y).abs()))
    }

    /// Distance with a large penalty across layers (for sorting candidates).
    pub fn cost(&self, other: &Position) -> i32 {
        self.distance(other).unwrap_or(10_000)
    }

    pub fn within(&self, other: &Position, range: i32) -> bool {
        self.distance(other).is_some_and(|d| d <= range)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub underground: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Zone {
    pub id: String,
    pub name: String,
    pub layer: u16,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub tags: Vec<String>,
}

impl Zone {
    pub fn contains(&self, p: &Position) -> bool {
        p.layer == self.layer && p.x >= self.x && p.x < self.x + self.w && p.y >= self.y && p.y < self.y + self.h
    }

    pub fn center(&self) -> Position {
        Position::new(self.layer, self.x + self.w / 2, self.y + self.h / 2)
    }

    pub fn random_cell(&self, rng: &mut SimRng) -> Position {
        Position::new(
            self.layer,
            rng.range_i32(self.x, self.x + self.w - 1),
            rng.range_i32(self.y, self.y + self.h - 1),
        )
    }

    /// Nearest cell of the zone to `p` (clamped).
    pub fn clamp(&self, p: &Position) -> Position {
        Position::new(
            self.layer,
            p.x.clamp(self.x, self.x + self.w - 1),
            p.y.clamp(self.y, self.y + self.h - 1),
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Portal {
    pub name: String,
    pub a: Position,
    pub b: Position,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CellState {
    pub dirt: f32,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub fluids: BTreeMap<String, f32>,
    /// Status id → pathogen load.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub pathogens: BTreeMap<String, f32>,
}

impl CellState {
    pub fn is_clean(&self) -> bool {
        self.dirt < 0.01 && self.fluids.is_empty() && self.pathogens.is_empty()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Network {
    pub id: String,
    pub name: String,
    pub zones: Vec<usize>,
    pub vector: Vector,
    /// Status id → contamination load of the whole network.
    pub load: BTreeMap<String, f32>,
}

#[derive(Resource, Debug, Clone, Serialize)]
pub struct WorldMap {
    pub layers: Vec<Layer>,
    pub zones: Vec<Zone>,
    pub portals: Vec<Portal>,
    pub networks: Vec<Network>,
    /// Sparse environmental state: only non-clean cells are stored.
    pub cells: BTreeMap<Position, CellState>,
}

impl WorldMap {
    pub fn from_def(def: Option<&MapDef>) -> Self {
        let Some(def) = def else {
            return Self {
                layers: vec![Layer { id: "surface".into(), name: "Superficie".into(), width: 64, height: 64, underground: false }],
                zones: vec![Zone { id: "world".into(), name: "Mondo".into(), layer: 0, x: 0, y: 0, w: 64, h: 64, tags: vec![] }],
                portals: vec![],
                networks: vec![],
                cells: BTreeMap::new(),
            };
        };
        let layers: Vec<Layer> = def
            .layers
            .iter()
            .map(|l| Layer { id: l.id.clone(), name: l.name.clone(), width: l.width, height: l.height, underground: l.underground })
            .collect();
        let layer_idx = |id: &str| layers.iter().position(|l| l.id == id).unwrap_or(0) as u16;
        let zones: Vec<Zone> = def
            .zones
            .iter()
            .map(|z| Zone {
                id: z.id.clone(),
                name: z.name.clone(),
                layer: layer_idx(&z.layer),
                x: z.rect.0,
                y: z.rect.1,
                w: z.rect.2.max(1),
                h: z.rect.3.max(1),
                tags: z.tags.clone(),
            })
            .collect();
        let portals = def
            .portals
            .iter()
            .map(|p| Portal {
                name: p.name.clone(),
                a: Position::new(layer_idx(&p.a.0), p.a.1, p.a.2),
                b: Position::new(layer_idx(&p.b.0), p.b.1, p.b.2),
            })
            .collect();
        let mut map = Self { layers, zones, portals, networks: vec![], cells: BTreeMap::new() };
        map.networks = def
            .networks
            .iter()
            .map(|n| Network {
                id: n.id.clone(),
                name: n.name.clone(),
                zones: n.zones.iter().flat_map(|z| map.resolve_zones(z)).collect(),
                vector: n.vector,
                load: BTreeMap::new(),
            })
            .collect();
        map
    }

    pub fn zone(&self, id: &str) -> Option<&Zone> {
        self.zones.iter().find(|z| z.id == id)
    }

    pub fn zone_index(&self, id: &str) -> Option<usize> {
        self.zones.iter().position(|z| z.id == id)
    }

    /// Zone indices matching an id, or every zone with a tag when written as `#tag`.
    pub fn resolve_zones(&self, spec: &str) -> Vec<usize> {
        match spec.strip_prefix('#') {
            Some(tag) => self
                .zones
                .iter()
                .enumerate()
                .filter(|(_, z)| z.tags.iter().any(|t| t == tag))
                .map(|(i, _)| i)
                .collect(),
            None => self.zone_index(spec).into_iter().collect(),
        }
    }

    /// Whether `p` is inside the zone (or any zone of the tag).
    pub fn in_zone(&self, spec: &str, p: &Position) -> bool {
        self.resolve_zones(spec).into_iter().any(|i| self.zones[i].contains(p))
    }

    /// Zones containing `p`, most specific (smallest) first.
    pub fn zones_at(&self, p: &Position) -> Vec<&Zone> {
        let mut v: Vec<&Zone> = self.zones.iter().filter(|z| z.contains(p)).collect();
        v.sort_by_key(|z| z.w * z.h);
        v
    }

    pub fn zone_name_at(&self, p: &Position) -> Option<&str> {
        self.zones_at(p).first().map(|z| z.name.as_str())
    }

    pub fn random_cell(&self, spec: &str, rng: &mut SimRng) -> Option<Position> {
        let zones = self.resolve_zones(spec);
        let i = *rng.pick(&zones)?;
        Some(self.zones[i].random_cell(rng))
    }

    pub fn clamp(&self, p: Position) -> Position {
        let l = &self.layers[p.layer as usize % self.layers.len()];
        Position::new(p.layer, p.x.clamp(0, l.width - 1), p.y.clamp(0, l.height - 1))
    }

    /// Next cell on the way from `from` to `to`, crossing portals when the target is on another layer.
    pub fn step_towards(&self, from: Position, to: Position) -> Position {
        let goal = if from.layer == to.layer {
            to
        } else {
            match self.next_portal(from, to.layer) {
                Some(p) => p,
                None => return from,
            }
        };
        if from == goal {
            // Standing on a portal end: cross it.
            if let Some(other) = self.portal_exit(from) {
                return other;
            }
            return from;
        }
        Position::new(from.layer, from.x + (goal.x - from.x).signum(), from.y + (goal.y - from.y).signum())
    }

    fn portal_exit(&self, p: Position) -> Option<Position> {
        self.portals.iter().find_map(|po| {
            if po.a == p {
                Some(po.b)
            } else if po.b == p {
                Some(po.a)
            } else {
                None
            }
        })
    }

    /// Nearest portal end on `from`'s layer that starts a shortest layer path to `target_layer`.
    fn next_portal(&self, from: Position, target_layer: u16) -> Option<Position> {
        // BFS over layers to find which neighbour layer to go to first.
        let mut prev: BTreeMap<u16, u16> = BTreeMap::new();
        let mut q = VecDeque::from([from.layer]);
        prev.insert(from.layer, from.layer);
        while let Some(l) = q.pop_front() {
            if l == target_layer {
                break;
            }
            for po in &self.portals {
                for (x, y) in [(po.a, po.b), (po.b, po.a)] {
                    if x.layer == l && !prev.contains_key(&y.layer) {
                        prev.insert(y.layer, l);
                        q.push_back(y.layer);
                    }
                }
            }
        }
        let mut step = target_layer;
        prev.get(&step)?;
        while prev[&step] != from.layer {
            step = prev[&step];
        }
        self.portals
            .iter()
            .flat_map(|po| [(po.a, po.b), (po.b, po.a)])
            .filter(|(x, y)| x.layer == from.layer && y.layer == step)
            .map(|(x, _)| x)
            .min_by_key(|x| (from.cost(x), *x))
    }

    pub fn cell(&self, p: &Position) -> Option<&CellState> {
        self.cells.get(p)
    }

    pub fn cell_mut(&mut self, p: Position) -> &mut CellState {
        self.cells.entry(p).or_default()
    }

    /// Drops cells that became clean, keeping the sparse map small.
    pub fn compact(&mut self) {
        self.cells.retain(|_, c| !c.is_clean());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{LayerDef, PortalDef, ZoneDef};

    fn two_layers() -> WorldMap {
        WorldMap::from_def(Some(&MapDef {
            layers: vec![
                LayerDef { id: "up".into(), name: "Up".into(), width: 20, height: 20, underground: false },
                LayerDef { id: "down".into(), name: "Down".into(), width: 20, height: 20, underground: true },
            ],
            zones: vec![ZoneDef { id: "mine".into(), name: "Mine".into(), layer: "down".into(), rect: (0, 0, 5, 5), tags: vec!["dark".into()] }],
            portals: vec![PortalDef { name: "stairs".into(), a: ("up".into(), 10, 10), b: ("down".into(), 2, 2) }],
            networks: vec![],
        }))
    }

    #[test]
    fn walks_through_portals() {
        let m = two_layers();
        let mut p = Position::new(0, 0, 0);
        let goal = Position::new(1, 4, 4);
        for _ in 0..40 {
            if p == goal {
                break;
            }
            p = m.step_towards(p, goal);
        }
        assert_eq!(p, goal);
        assert!(m.in_zone("#dark", &goal));
        assert!(!m.in_zone("mine", &Position::new(0, 1, 1)));
    }
}
