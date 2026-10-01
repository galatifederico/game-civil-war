//! Spatial model: stacked grid layers (surface, underground…), rectangular zones, portals between
//! layers, and sparse per-cell environmental state (dirt, fluids, pathogen load).

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, VecDeque};
use std::sync::Arc;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{MapDef, Side, Vector};
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
    pub indoor: bool,
    pub depth: i32,
    pub tags: Vec<String>,
    /// Tile rows (one character per cell), for clients.
    pub tiles: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Prop {
    pub layer: u16,
    pub sprite: String,
    pub x: i32,
    pub y: i32,
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
}

/// Map geometry: immutable after loading and shared, so cloning [`WorldMap`] is free.
#[derive(Resource, Debug, Clone)]
pub struct WorldMap(Arc<MapData>);

impl std::ops::Deref for WorldMap {
    type Target = MapData;
    fn deref(&self) -> &MapData {
        &self.0
    }
}

impl Serialize for WorldMap {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MapData {
    pub layers: Vec<Layer>,
    pub zones: Vec<Zone>,
    pub portals: Vec<Portal>,
    pub networks: Vec<Network>,
    /// Impassable rectangles: (name, layer, x, y, w, h).
    pub walls: Vec<(String, u16, i32, i32, i32, i32)>,
    /// Tile legend: character → (terrain id, walkable).
    pub legend: BTreeMap<char, (String, bool)>,
    /// Diggable tiles: character → (tile after digging, item yielded).
    pub diggable: BTreeMap<char, (char, Option<(String, u32)>)>,
    pub props: Vec<Prop>,
    /// Per layer, row-major: true = impassable.
    #[serde(skip)]
    blocked: Vec<Vec<bool>>,
    /// Map-to-map hop counts through portals and edges (u16::MAX = unreachable).
    #[serde(skip)]
    hops: Vec<Vec<u16>>,
}

/// Cells of a footprint around an anchor: `w` wide centred on it, `h` tall upwards.
pub fn footprint_cells(anchor: Position, w: i32, h: i32) -> Vec<Position> {
    let mut v = Vec::new();
    for dy in 0..h.max(0) {
        for dx in -(w - 1) / 2..=w / 2 {
            v.push(Position::new(anchor.layer, anchor.x + dx, anchor.y - dy));
        }
    }
    v
}

/// Mutable environmental state: sparse dirty cells and contamination of infrastructure networks.
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Environment {
    /// Only non-clean cells are stored.
    #[serde(with = "cells_as_list")]
    pub cells: BTreeMap<Position, CellState>,
    /// Network id → status id → contamination load.
    pub network_load: BTreeMap<String, BTreeMap<String, f32>>,
}

impl Environment {
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


mod cells_as_list {
    use super::{CellState, Position};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<S: Serializer>(m: &BTreeMap<Position, CellState>, s: S) -> Result<S::Ok, S::Error> {
        m.iter().collect::<Vec<_>>().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<Position, CellState>, D::Error> {
        Ok(Vec::<(Position, CellState)>::deserialize(d)?.into_iter().collect())
    }
}

/// Every tile changed since the world was built (digging, painting): kept so saves can replay them.
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct TerrainChanges(pub Vec<(Position, char)>);

/// Changes a tile and records it.
pub fn change_tile(world: &mut World, p: Position, ch: char) {
    world.resource_mut::<WorldMap>().make_mut().set_tile(p, ch);
    world.resource_mut::<TerrainChanges>().0.push((p, ch));
    world.insert_resource(crate::targeting::TargetIndex::default());
}

/// A random cell of a zone spec, using the shared RNG.
pub fn random_cell(world: &mut World, spec: &str) -> Option<Position> {
    let map = world.resource::<WorldMap>().clone();
    map.random_cell(spec, &mut world.resource_mut::<SimRng>())
}

impl WorldMap {
    pub fn from_def(def: Option<&MapDef>) -> Self {
        Self(Arc::new(MapData::from_def(def)))
    }

    /// Mutable access (copy-on-write): only used while building the world.
    pub fn make_mut(&mut self) -> &mut MapData {
        Arc::make_mut(&mut self.0)
    }
}

impl MapData {
    pub fn from_def(def: Option<&MapDef>) -> Self {
        let Some(def) = def else {
            return Self {
                layers: vec![Layer {
                    id: "surface".into(),
                    name: "Superficie".into(),
                    width: 64,
                    height: 64,
                    underground: false,
                    indoor: false,
                    depth: 0,
                    tags: vec![],
                    tiles: vec![],
                }],
                zones: vec![Zone { id: "world".into(), name: "Mondo".into(), layer: 0, x: 0, y: 0, w: 64, h: 64, tags: vec![] }],
                portals: vec![],
                networks: vec![],
                walls: vec![],
                legend: BTreeMap::new(),
                diggable: BTreeMap::new(),
                props: vec![],
                blocked: vec![vec![false; 64 * 64]],
                hops: vec![vec![0]],
            };
        };
        let layers: Vec<Layer> = def
            .layers
            .iter()
            .map(|l| Layer {
                id: l.id.clone(),
                name: l.name.clone(),
                width: l.width,
                height: l.height,
                underground: l.underground,
                indoor: l.indoor,
                depth: l.depth,
                tags: l.tags.clone(),
                tiles: l.tiles.clone(),
            })
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
        let mut portals: Vec<Portal> = def
            .portals
            .iter()
            .map(|p| Portal {
                name: p.name.clone(),
                a: Position::new(layer_idx(&p.a.0), p.a.1, p.a.2),
                b: Position::new(layer_idx(&p.b.0), p.b.1, p.b.2),
            })
            .collect();
        let legend: BTreeMap<char, (String, bool)> = def.legend.iter().map(|t| (t.ch, (t.id.clone(), t.walkable))).collect();
        let diggable: BTreeMap<char, (char, Option<(String, u32)>)> =
            def.legend.iter().filter_map(|t| t.dig_to.map(|to| (t.ch, (to, t.yields.clone())))).collect();
        let props: Vec<Prop> = def
            .props
            .iter()
            .map(|p| Prop { layer: layer_idx(&p.layer), sprite: p.sprite.clone(), x: p.at.0, y: p.at.1 })
            .collect();
        let walls: Vec<(String, u16, i32, i32, i32, i32)> =
            def.walls.iter().map(|w| (w.name.clone(), layer_idx(&w.layer), w.rect.0, w.rect.1, w.rect.2, w.rect.3)).collect();
        let mut blocked: Vec<Vec<bool>> = layers.iter().map(|l| vec![false; (l.width * l.height).max(0) as usize]).collect();
        for (_, l, x, y, w, h) in &walls {
            let lw = layers[*l as usize].width;
            let lh = layers[*l as usize].height;
            for yy in (*y).max(0)..(y + h).min(lh) {
                for xx in (*x).max(0)..(x + w).min(lw) {
                    blocked[*l as usize][(yy * lw + xx) as usize] = true;
                }
            }
        }
        // Impassable tiles.
        for (li, l) in layers.iter().enumerate() {
            for (y, row) in l.tiles.iter().enumerate() {
                for (x, ch) in row.chars().enumerate() {
                    if legend.get(&ch).is_some_and(|(_, walk)| !walk) && (x as i32) < l.width {
                        blocked[li][y * l.width as usize + x] = true;
                    }
                }
            }
        }
        // Solid decorations (a door keeps its anchor open).
        for p in &def.props {
            let li = layer_idx(&p.layer);
            let anchor = Position::new(li, p.at.0, p.at.1);
            for c in footprint_cells(anchor, p.footprint.0, p.footprint.1) {
                if p.door && c == anchor {
                    continue;
                }
                let l = &layers[li as usize];
                if c.x >= 0 && c.y >= 0 && c.x < l.width && c.y < l.height {
                    blocked[li as usize][(c.y * l.width + c.x) as usize] = true;
                }
            }
        }
        // Edges: every walkable border cell facing a walkable cell of the neighbour becomes a passage.
        let walkable = |blocked: &Vec<Vec<bool>>, l: u16, x: i32, y: i32| {
            let ly = &layers[l as usize];
            x >= 0 && y >= 0 && x < ly.width && y < ly.height && !blocked[l as usize][(y * ly.width + x) as usize]
        };
        for e in &def.edges {
            let (a, b) = (layer_idx(&e.a), layer_idx(&e.b));
            let (la, lb) = (&layers[a as usize], &layers[b as usize]);
            let pairs: Vec<(Position, Position)> = match e.side {
                Side::East => (0..la.height).map(|y| (Position::new(a, la.width - 1, y), Position::new(b, 0, y + e.offset))).collect(),
                Side::West => (0..la.height).map(|y| (Position::new(a, 0, y), Position::new(b, lb.width - 1, y + e.offset))).collect(),
                Side::South => (0..la.width).map(|x| (Position::new(a, x, la.height - 1), Position::new(b, x + e.offset, 0))).collect(),
                Side::North => (0..la.width).map(|x| (Position::new(a, x, 0), Position::new(b, x + e.offset, lb.height - 1))).collect(),
            };
            for (pa, pb) in pairs {
                if walkable(&blocked, pa.layer, pa.x, pa.y) && walkable(&blocked, pb.layer, pb.x, pb.y) {
                    portals.push(Portal { name: format!("{} → {}", e.a, e.b), a: pa, b: pb });
                }
            }
        }
        let mut map = Self { layers, zones, portals, networks: vec![], walls, legend, diggable, props, blocked, hops: vec![] };
        map.compute_hops();
        map.networks = def
            .networks
            .iter()
            .map(|n| Network {
                id: n.id.clone(),
                name: n.name.clone(),
                zones: n.zones.iter().flat_map(|z| map.resolve_zones(z)).collect(),
                vector: n.vector,
            })
            .collect();
        map
    }

    fn compute_hops(&mut self) {
        let n = self.layers.len();
        let mut adj: Vec<std::collections::BTreeSet<usize>> = vec![Default::default(); n];
        for p in &self.portals {
            let (a, b) = (p.a.layer as usize, p.b.layer as usize);
            if a != b {
                adj[a].insert(b);
                adj[b].insert(a);
            }
        }
        self.hops = (0..n)
            .map(|s| {
                let mut d = vec![u16::MAX; n];
                d[s] = 0;
                let mut q = VecDeque::from([s]);
                while let Some(u) = q.pop_front() {
                    for &v in &adj[u] {
                        if d[v] == u16::MAX {
                            d[v] = d[u] + 1;
                            q.push_back(v);
                        }
                    }
                }
                d
            })
            .collect();
    }

    /// Approximate walking cost between two positions, also across maps (each map change costs
    /// `hop_cost` cells). Unreachable = 100 000.
    pub fn travel_cost(&self, a: &Position, b: &Position, hop_cost: i32) -> i32 {
        if a.layer == b.layer {
            return a.distance(b).unwrap_or(0);
        }
        match self.hops.get(a.layer as usize).and_then(|h| h.get(b.layer as usize)) {
            Some(&h) if h != u16::MAX => h as i32 * hop_cost,
            _ => 100_000,
        }
    }

    pub fn tile(&self, p: &Position) -> Option<char> {
        let l = self.layers.get(p.layer as usize)?;
        l.tiles.get(p.y as usize)?.chars().nth(p.x as usize)
    }

    /// Changes one tile (digging, building, admin painting) and its passability.
    pub fn set_tile(&mut self, p: Position, ch: char) {
        let Some(l) = self.layers.get_mut(p.layer as usize) else { return };
        if p.x < 0 || p.y < 0 || p.x >= l.width || p.y >= l.height {
            return;
        }
        while l.tiles.len() <= p.y as usize {
            l.tiles.push(String::new());
        }
        let mut row: Vec<char> = l.tiles[p.y as usize].chars().collect();
        while row.len() <= p.x as usize {
            row.push('.');
        }
        row[p.x as usize] = ch;
        l.tiles[p.y as usize] = row.into_iter().collect();
        let walk = self.legend.get(&ch).is_none_or(|(_, w)| *w);
        let w = l.width;
        self.blocked[p.layer as usize][(p.y * w + p.x) as usize] = !walk;
    }

    /// Marks extra cells as impassable (building footprints).
    pub fn block(&mut self, cells: &[Position]) {
        for c in cells {
            if let Some(l) = self.layers.get(c.layer as usize) {
                if c.x >= 0 && c.y >= 0 && c.x < l.width && c.y < l.height {
                    self.blocked[c.layer as usize][(c.y * l.width + c.x) as usize] = true;
                }
            }
        }
    }

    pub fn blocked(&self, p: &Position) -> bool {
        let Some(l) = self.layers.get(p.layer as usize) else { return true };
        if p.x < 0 || p.y < 0 || p.x >= l.width || p.y >= l.height {
            return true;
        }
        self.blocked[p.layer as usize][(p.y * l.width + p.x) as usize]
    }

    /// A* over the layered grid (8 directions, no corner cutting, portals as edges). Returns the steps
    /// after `from` until a cell within `range` of `goal`, or None if unreachable within `max_nodes`.
    pub fn find_path(&self, from: Position, goal: Position, range: i32, allowed: &dyn Fn(&Position) -> bool, max_nodes: usize) -> Option<Vec<Position>> {
        if from.layer == goal.layer {
            return self.find_path_local(from, goal, range, allowed, max_nodes);
        }
        let portal_ends: Vec<(Position, Position)> = self.portals.iter().flat_map(|p| [(p.a, p.b), (p.b, p.a)]).collect();
        let h = |p: &Position| -> i32 {
            if p.layer == goal.layer {
                (p.distance(&goal).unwrap_or(0) - range).max(0)
            } else {
                portal_ends.iter().filter(|(a, _)| a.layer == p.layer).map(|(a, _)| a.cost(p) + 1).min().unwrap_or(0)
            }
        };
        // Dense arrays over all layers: index = layer offset + y * width + x.
        let offsets: Vec<usize> = self
            .layers
            .iter()
            .scan(0usize, |acc, l| {
                let o = *acc;
                *acc += (l.width * l.height).max(0) as usize;
                Some(o)
            })
            .collect();
        let total: usize = self.layers.iter().map(|l| (l.width * l.height).max(0) as usize).sum();
        let idx = |p: &Position| offsets[p.layer as usize] + (p.y * self.layers[p.layer as usize].width + p.x) as usize;
        let mut g = vec![i32::MAX; total];
        let mut came = vec![usize::MAX; total];
        let mut open = BinaryHeap::new();
        g[idx(&from)] = 0;
        open.push(Reverse((h(&from), 0, from)));
        let mut expanded = 0;
        while let Some(Reverse((_, cost, cur))) = open.pop() {
            let ci = idx(&cur);
            if cost > g[ci] {
                continue;
            }
            if cur.within(&goal, range) {
                let mut path = Vec::new();
                let mut c = cur;
                while c != from {
                    path.push(c);
                    let pi = came[idx(&c)];
                    if pi == usize::MAX {
                        break;
                    }
                    c = self.position_of(pi, &offsets);
                }
                path.reverse();
                return Some(path);
            }
            expanded += 1;
            if expanded > max_nodes {
                return None;
            }
            let mut next: [Option<Position>; 12] = [None; 12];
            let mut k = 0;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let n = Position::new(cur.layer, cur.x + dx, cur.y + dy);
                    if self.blocked(&n) || !allowed(&n) {
                        continue;
                    }
                    if dx != 0 && dy != 0 && (self.blocked(&Position::new(cur.layer, cur.x + dx, cur.y)) || self.blocked(&Position::new(cur.layer, cur.x, cur.y + dy))) {
                        continue;
                    }
                    next[k] = Some(n);
                    k += 1;
                }
            }
            for (a, b) in &portal_ends {
                if *a == cur && allowed(b) && k < next.len() {
                    next[k] = Some(*b);
                    k += 1;
                }
            }
            for n in next.iter().take(k).flatten() {
                let ni = idx(n);
                let nc = cost + 1;
                if nc < g[ni] {
                    g[ni] = nc;
                    came[ni] = ci;
                    open.push(Reverse((nc + h(n), nc, *n)));
                }
            }
        }
        None
    }

    /// A* inside one map (8 directions, no corner cutting): arrays sized to that map only.
    pub fn find_path_local(&self, from: Position, goal: Position, range: i32, allowed: &dyn Fn(&Position) -> bool, max_nodes: usize) -> Option<Vec<Position>> {
        let l = self.layers.get(from.layer as usize)?;
        let (w, h) = (l.width, l.height);
        if from.x < 0 || from.y < 0 || from.x >= w || from.y >= h {
            return None;
        }
        let idx = |x: i32, y: i32| (y * w + x) as usize;
        let mut g = vec![i32::MAX; (w * h) as usize];
        let mut came = vec![u32::MAX; (w * h) as usize];
        let hfn = |x: i32, y: i32| ((x - goal.x).abs().max((y - goal.y).abs()) - range).max(0);
        let mut open = BinaryHeap::new();
        g[idx(from.x, from.y)] = 0;
        open.push(Reverse((hfn(from.x, from.y), 0, from.x, from.y)));
        let mut expanded = 0;
        while let Some(Reverse((_, cost, x, y))) = open.pop() {
            let ci = idx(x, y);
            if cost > g[ci] {
                continue;
            }
            if (x - goal.x).abs().max((y - goal.y).abs()) <= range {
                let mut path = Vec::new();
                let (mut cx, mut cy) = (x, y);
                while (cx, cy) != (from.x, from.y) {
                    path.push(Position::new(from.layer, cx, cy));
                    let p = came[idx(cx, cy)];
                    if p == u32::MAX {
                        break;
                    }
                    cx = p as i32 % w;
                    cy = p as i32 / w;
                }
                path.reverse();
                return Some(path);
            }
            expanded += 1;
            if expanded > max_nodes {
                return None;
            }
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let (nx, ny) = (x + dx, y + dy);
                    let n = Position::new(from.layer, nx, ny);
                    if self.blocked(&n) || !allowed(&n) {
                        continue;
                    }
                    if dx != 0 && dy != 0 && (self.blocked(&Position::new(from.layer, x + dx, y)) || self.blocked(&Position::new(from.layer, x, y + dy))) {
                        continue;
                    }
                    let ni = idx(nx, ny);
                    let nc = cost + 1;
                    if nc < g[ni] {
                        g[ni] = nc;
                        came[ni] = ci as u32;
                        open.push(Reverse((nc + hfn(nx, ny), nc, nx, ny)));
                    }
                }
            }
        }
        None
    }

    fn position_of(&self, index: usize, offsets: &[usize]) -> Position {
        let layer = offsets.iter().rposition(|o| *o <= index).unwrap_or(0);
        let local = (index - offsets[layer]) as i32;
        let w = self.layers[layer].width;
        Position::new(layer as u16, local % w, local / w)
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

    /// A random walkable cell of a zone spec (a blocked one only if the zone has no free cell).
    pub fn random_cell(&self, spec: &str, rng: &mut SimRng) -> Option<Position> {
        let zones = self.resolve_zones(spec);
        let i = *rng.pick(&zones)?;
        let mut last = self.zones[i].random_cell(rng);
        for _ in 0..40 {
            if !self.blocked(&last) {
                return Some(last);
            }
            last = self.zones[i].random_cell(rng);
        }
        Some(last)
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

    pub fn portal_exit(&self, p: Position) -> Option<Position> {
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
    pub fn next_portal(&self, from: Position, target_layer: u16) -> Option<Position> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{LayerDef, PortalDef, ZoneDef};

    fn two_layers() -> WorldMap {
        WorldMap::from_def(Some(&MapDef {
            layers: vec![
                LayerDef { id: "up".into(), name: "Up".into(), width: 20, height: 20, underground: false, ..Default::default() },
                LayerDef { id: "down".into(), name: "Down".into(), width: 20, height: 20, underground: true, ..Default::default() },
            ],
            zones: vec![ZoneDef { id: "mine".into(), name: "Mine".into(), layer: "down".into(), rect: (0, 0, 5, 5), tags: vec!["dark".into()] }],
            portals: vec![PortalDef { name: "stairs".into(), a: ("up".into(), 10, 10), b: ("down".into(), 2, 2) }],
            networks: vec![],
            walls: vec![],
            ..Default::default()
        }))
    }

    #[test]
    fn astar_goes_around_walls() {
        let m = MapData::from_def(Some(&MapDef {
            layers: vec![LayerDef { id: "up".into(), name: "Up".into(), width: 20, height: 20, underground: false, ..Default::default() }],
            zones: vec![],
            portals: vec![],
            networks: vec![],
            walls: vec![crate::content::WallDef { name: "muro".into(), layer: "up".into(), rect: (10, 0, 1, 18) }],
            ..Default::default()
        }));
        let path = m.find_path(Position::new(0, 5, 5), Position::new(0, 15, 5), 0, &|_| true, 10_000).unwrap();
        assert!(path.iter().all(|p| !m.blocked(p)));
        assert!(path.iter().any(|p| p.y >= 18), "must go around the wall's end");
        assert_eq!(*path.last().unwrap(), Position::new(0, 15, 5));
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
