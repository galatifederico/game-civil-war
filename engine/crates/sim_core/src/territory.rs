//! Territory: factions conquer the quarters of the map.
//!
//! Quarters are the zones tagged `territory` (by default the tag is "quartiere"). Every
//! `territory.interval` ticks each faction gains influence in a quarter from its members standing
//! there (`territory.presence` each), the buildings it owns there (`territory.building`, scaled by
//! their health) and its banners (buildings tagged "stendardo": `territory.banner` each, at most
//! `territory.max_banners` counted); influence fades by `territory.decay`. A quarter is taken when a
//! faction's influence reaches `territory.threshold` and beats the current owner by
//! `territory.margin` (so control does not flip back and forth). Owners earn `territory.income` and
//! `territory.victory_points` per quarter every day.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::buildings::{Building, Owner};
use crate::content::Content;
use crate::events::{EventBuilder, EventLog};
use crate::factions::{FactionMember, Factions};
use crate::map::{Position, WorldMap};
use crate::params::Params;
use crate::stats::{Dead, Pawn, Virtual};
use crate::time::SimClock;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TerritoryState {
    pub owner: Option<String>,
    pub influence: BTreeMap<String, f32>,
    /// Tick of the last change of owner.
    pub since: u64,
}

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Territories {
    pub zones: BTreeMap<String, TerritoryState>,
}

impl Territories {
    pub fn owned_by(&self, faction: &str) -> Vec<String> {
        self.zones.iter().filter(|(_, t)| t.owner.as_deref() == Some(faction)).map(|(z, _)| z.clone()).collect()
    }
}

/// Indices of the quarter zones in the map.
pub fn quarters(map: &WorldMap) -> Vec<usize> {
    map.zones.iter().enumerate().filter(|(_, z)| z.tags.iter().any(|t| t == "quartiere")).map(|(i, _)| i).collect()
}

pub fn conquest(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let p = world.resource::<Params>().clone();
    let interval = p.get("territory.interval", 6.0).max(1.0) as u64;
    if tick == 0 || !tick.is_multiple_of(interval) {
        return;
    }
    let map = world.resource::<WorldMap>().clone();
    let quarters = quarters(&map);
    if quarters.is_empty() {
        return;
    }
    let content = world.resource::<Content>().clone();
    let (w_presence, w_building, w_banner) = (p.get("territory.presence", 1.0) as f32, p.get("territory.building", 2.0) as f32, p.get("territory.banner", 6.0) as f32);
    let max_banners = p.get("territory.max_banners", 2.0) as u32;
    let decay = p.get("territory.decay", 0.1).clamp(0.0, 1.0) as f32;
    let (threshold, margin) = (p.get("territory.threshold", 20.0) as f32, p.get("territory.margin", 1.25) as f32);
    let quarter_of = |pos: &Position| quarters.iter().copied().find(|i| map.zones[*i].contains(pos));
    let active = |f: &str, fs: &Factions| fs.states.get(f).is_some_and(|s| s.absorbed_into.is_none());

    let mut gain: BTreeMap<usize, BTreeMap<String, f32>> = BTreeMap::new();
    let mut banners: BTreeMap<(usize, String), u32> = BTreeMap::new();
    for e in crate::sorted_entities::<Pawn>(world) {
        if world.get::<Dead>(e).is_some() || world.get::<Virtual>(e).is_some() {
            continue;
        }
        let (Some(pos), Some(m)) = (world.get::<Position>(e), world.get::<FactionMember>(e)) else { continue };
        if let Some(q) = quarter_of(pos) {
            *gain.entry(q).or_default().entry(m.faction.clone()).or_default() += w_presence;
        }
    }
    for e in crate::sorted_entities::<Building>(world) {
        let b = world.get::<Building>(e).unwrap();
        let (Owner::Faction(f), Some(pos)) = (&b.owner, world.get::<Position>(e)) else { continue };
        if b.hp <= 0.0 {
            continue;
        }
        let Some(q) = quarter_of(pos) else { continue };
        let health = (b.hp / b.max_hp.max(1.0)).clamp(0.0, 1.0);
        let banner = content.buildings.get(&b.def).is_some_and(|d| d.tags.iter().any(|t| t == "stendardo"));
        let mut w = w_building * health;
        if banner {
            let n = banners.entry((q, f.clone())).or_default();
            *n += 1;
            w = if *n <= max_banners { w_banner * health } else { 0.0 };
        }
        *gain.entry(q).or_default().entry(f.clone()).or_default() += w;
    }

    let factions = world.resource::<Factions>().clone();
    let mut changes: Vec<(String, String, Option<String>, String)> = Vec::new();
    {
        let mut terr = world.resource_mut::<Territories>();
        for q in &quarters {
            let z = &map.zones[*q];
            let st = terr.zones.entry(z.id.clone()).or_default();
            for v in st.influence.values_mut() {
                *v *= 1.0 - decay;
            }
            for (f, g) in gain.get(q).into_iter().flatten() {
                if active(f, &factions) {
                    *st.influence.entry(f.clone()).or_default() += g;
                }
            }
            st.influence.retain(|f, v| *v >= 0.05 && active(f, &factions));
            // Strongest faction (ties: lowest id, deterministic).
            let best = st.influence.iter().fold(None::<(&String, f32)>, |acc, (f, v)| match acc {
                Some((_, bv)) if bv >= *v => acc,
                _ => Some((f, *v)),
            });
            let held = st.owner.as_ref().map_or(0.0, |o| st.influence.get(o).copied().unwrap_or(0.0));
            if let Some((f, v)) = best
                && st.owner.as_deref() != Some(f.as_str())
                && v >= threshold
                && v >= held * margin
            {
                let previous = st.owner.replace(f.clone());
                st.since = tick;
                changes.push((z.id.clone(), z.name.clone(), previous, f.clone()));
            }
            // An owner that lost every supporter lets the quarter go free.
            if let Some(o) = st.owner.clone()
                && st.influence.get(&o).copied().unwrap_or(0.0) < threshold * 0.1
            {
                st.owner = None;
                st.since = tick;
                changes.push((z.id.clone(), z.name.clone(), Some(o), String::new()));
            }
        }
    }
    let fname = |f: &str| content.factions.get(f).map_or(f.to_string(), |d| d.name.clone());
    for (zid, zname, previous, new) in changes {
        let msg = if new.is_empty() {
            format!("{zname} non è più di nessuno: {} ha perso il controllo", fname(previous.as_deref().unwrap_or("")))
        } else {
            match &previous {
                Some(p) => format!("{} strappa {zname} a {}", fname(&new), fname(p)),
                None => format!("{} conquista {zname}", fname(&new)),
            }
        };
        let faction = if new.is_empty() { previous.clone() } else { Some(new.clone()) };
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new("territory", msg).faction(faction).news(0.85).tags(["territory".to_string(), zid]),
        );
    }

    // Daily rent and victory points for every quarter held.
    if tick.is_multiple_of(24) {
        let (income, vp) = (p.get("territory.income", 15.0), p.get("territory.victory_points", 1.0) as i64);
        let owners: Vec<String> = world.resource::<Territories>().zones.values().filter_map(|t| t.owner.clone()).collect();
        let mut fs = world.resource_mut::<Factions>();
        for o in owners {
            if let Some(s) = fs.states.get_mut(&o) {
                s.treasury += income;
                s.victory_points += vp;
            }
        }
    }
}
