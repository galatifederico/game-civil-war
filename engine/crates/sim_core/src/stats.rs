//! Identity components, statistics with modifiers, tags and needs.

use std::collections::{BTreeMap, BTreeSet};

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::Content;
use crate::inventory::Inventory;
use crate::status::StatusEffects;

/// Marker of agents (pawns). Buildings and virtual entities are not pawns.
#[derive(Component, Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct Pawn;

/// Entities that exist without a body or position (e.g. an AI in the network).
#[derive(Component, Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct Virtual;

#[derive(Component, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DisplayName(pub String);

#[derive(Component, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TemplateId(pub String);

#[derive(Component, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Race(pub String);

/// Sex of a pawn (races marked `sexless`, like machines, have none).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Sex {
    Male,
    Female,
    NonBinary,
}

#[derive(Component, Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Classes(pub Vec<String>);

#[derive(Component, Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct Immortal;

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Dead {
    pub tick: u64,
    pub cause: String,
}

#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Wallet(pub f64);

/// Tags: `base` from the template and effects, `effective` recomputed every tick adding race, classes,
/// statuses and faction role.
#[derive(Component, Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Tags {
    pub base: BTreeSet<String>,
    pub effective: BTreeSet<String>,
}

impl Tags {
    pub fn has(&self, t: &str) -> bool {
        self.effective.contains(t) || self.base.contains(t)
    }
}

/// Statistics: `base` holds permanent values (template + effects), `effective` adds every modifier.
#[derive(Component, Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Stats {
    pub base: BTreeMap<String, f32>,
    pub effective: BTreeMap<String, f32>,
}

impl Stats {
    pub fn get(&self, id: &str) -> f32 {
        self.effective.get(id).or_else(|| self.base.get(id)).copied().unwrap_or(0.0)
    }

    pub fn add_base(&mut self, id: &str, amount: f32, bounds: (f32, f32)) {
        let v = self.base.entry(id.to_string()).or_insert(0.0);
        *v = (*v + amount).clamp(bounds.0, bounds.1);
        // Keep effective in sync until the next recompute so same-tick readers see the change.
        let e = self.effective.entry(id.to_string()).or_insert(0.0);
        *e = (*e + amount).clamp(bounds.0, bounds.1);
    }

    pub fn set_base(&mut self, id: &str, value: f32, bounds: (f32, f32)) {
        let old = self.get(id);
        self.base.insert(id.to_string(), value.clamp(bounds.0, bounds.1));
        let e = self.effective.entry(id.to_string()).or_insert(old);
        *e = (*e + value - old).clamp(bounds.0, bounds.1);
    }
}

/// Needs in 0..1 (1 = satisfied).
#[derive(Component, Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Needs(pub BTreeMap<String, f32>);

impl Needs {
    pub fn get(&self, id: &str) -> f32 {
        self.0.get(id).copied().unwrap_or(1.0)
    }

    pub fn add(&mut self, id: &str, amount: f32) {
        let v = self.0.entry(id.to_string()).or_insert(1.0);
        *v = (*v + amount).clamp(0.0, 1.0);
    }
}

/// Recomputes effective stats and tags from race, classes, statuses and carried items.
pub fn recompute_stats(
    content: Res<Content>,
    mut q: Query<
        (&Race, &Classes, &StatusEffects, Option<&Inventory>, Option<&crate::factions::FactionMember>, &mut Stats, &mut Tags),
        Without<Dead>,
    >,
) {
    let police = &content.bindings.police_tag;
    let press = &content.bindings.press_tag;
    for (race, classes, statuses, inv, member, mut stats, mut tags) in &mut q {
        let mut eff = stats.base.clone();
        let mut t: BTreeSet<String> = tags.base.clone();
        let add = |m: &BTreeMap<String, f32>, eff: &mut BTreeMap<String, f32>| {
            for (k, v) in m {
                *eff.entry(k.clone()).or_insert(0.0) += v;
            }
        };
        if let Some(r) = content.races.get(&race.0) {
            add(&r.stats, &mut eff);
            t.extend(r.tags.iter().cloned());
            t.insert(format!("race:{}", r.id));
        }
        for c in &classes.0 {
            if let Some(cd) = content.classes.get(c) {
                add(&cd.stats, &mut eff);
                t.extend(cd.tags.iter().cloned());
                t.insert(format!("class:{c}"));
            }
        }
        for (sid, st) in &statuses.active {
            if let Some(sd) = content.statuses.get(sid) {
                add(&sd.stats, &mut eff);
                t.extend(sd.grants_tags.iter().cloned());
                t.extend(sd.tags.iter().map(|x| format!("status:{x}")));
                if let Some(stage) = st.stage.and_then(|i| sd.stages.get(i)) {
                    add(&stage.stats, &mut eff);
                    t.extend(stage.grants_tags.iter().cloned());
                }
            }
        }
        if let Some(inv) = inv {
            for (item, _) in inv.items() {
                if let Some(idef) = content.items.get(item) {
                    add(&idef.carried_stats, &mut eff);
                }
            }
        }
        if let Some(m) = member
            && let Some(f) = content.factions.get(&m.faction) {
                match f.role {
                    crate::content::FactionRole::Police => {
                        t.insert(police.clone());
                    }
                    crate::content::FactionRole::Press => {
                        t.insert(press.clone());
                    }
                    _ => {}
                }
                t.extend(f.tags.iter().cloned());
            }
        for (k, v) in eff.iter_mut() {
            let (lo, hi) = content.stat_bounds(k);
            *v = v.clamp(lo, hi);
        }
        stats.effective = eff;
        tags.effective = t;
    }
}

/// Stats with a rest value drift back towards it (morale recovers, anger fades).
pub fn stat_recovery(content: Res<Content>, mut q: Query<&mut Stats, (With<Pawn>, Without<Dead>)>) {
    let drifting: Vec<(&String, f32, f32)> =
        content.stats.values().filter_map(|s| s.rest_value.filter(|_| s.recovery > 0.0).map(|r| (&s.id, r, s.recovery))).collect();
    if drifting.is_empty() {
        return;
    }
    for mut stats in &mut q {
        for (id, rest, rate) in &drifting {
            let v = stats.base.get(*id).copied().unwrap_or(*rest);
            let nv = if v < *rest { (v + rate).min(*rest) } else { (v - rate).max(*rest) };
            if nv != v {
                stats.base.insert((*id).clone(), nv);
            }
        }
    }
}

/// Needs decay, modulated by statuses (`need_rates` multiply the base decay).
pub fn decay_needs(content: Res<Content>, mut q: Query<(&mut Needs, &StatusEffects, &Race), (With<Pawn>, Without<Dead>)>) {
    for (mut needs, statuses, race) in &mut q {
        let exempt = content.races.get(&race.0).map(|r| r.needs_exempt.clone()).unwrap_or_default();
        for (nid, nd) in &content.needs {
            if exempt.contains(nid) {
                needs.0.insert(nid.clone(), 1.0);
                continue;
            }
            let mut rate = 1.0;
            for (sid, st) in &statuses.active {
                if let Some(sd) = content.statuses.get(sid) {
                    rate += sd.need_rates.get(nid).copied().unwrap_or(0.0);
                    if let Some(stage) = st.stage.and_then(|i| sd.stages.get(i)) {
                        rate += stage.need_rates.get(nid).copied().unwrap_or(0.0);
                    }
                }
            }
            needs.add(nid, -nd.decay * rate.max(0.0));
        }
    }
}
