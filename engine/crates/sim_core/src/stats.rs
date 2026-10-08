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

/// Sex of a pawn (races without `sexes`, like machines, have none).
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

/// Needs of older saves (0..1): turned into their stats when the save is loaded.
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
    titles: Res<crate::factions::Titles>,
    params: Res<crate::params::Params>,
    mut q: Query<
        (
            &Race,
            &Classes,
            &StatusEffects,
            Option<&Inventory>,
            Option<&crate::factions::FactionMember>,
            Option<&crate::ids::SimId>,
            Option<&crate::equipment::Equipment>,
            Option<&crate::abilities::AuraBonus>,
            Option<&crate::modes::Mode>,
            &mut Stats,
            &mut Tags,
        ),
        Without<Dead>,
    >,
) {
    let police = &content.bindings.police_tag;
    let press = &content.bindings.press_tag;
    // Roles held, by holder.
    let mut roles: BTreeMap<crate::ids::SimId, Vec<&crate::content::TitleDef>> = BTreeMap::new();
    for (t, h) in &titles.holders {
        if let (Some(h), Some(d)) = (h, content.titles.get(t)) {
            roles.entry(*h).or_default().push(d);
        }
    }
    for (race, classes, statuses, inv, member, sid, equipment, aura, mode, mut stats, mut tags) in &mut q {
        let mut eff = stats.base.clone();
        let mut t: BTreeSet<String> = tags.base.clone();
        // Abilities (from race, classes and roles): permanent modifiers and tags.
        let mut abilities: BTreeSet<&String> = BTreeSet::new();
        abilities.extend(content.races.get(&race.0).into_iter().flat_map(|r| r.abilities.iter()));
        abilities.extend(classes.0.iter().filter_map(|c| content.classes.get(c)).flat_map(|d| d.abilities.iter()));
        abilities.extend(sid.and_then(|id| roles.get(id)).into_iter().flatten().flat_map(|d| d.abilities.iter()));
        for a in abilities.iter().filter_map(|a| content.abilities.get(*a)) {
            for (k, v) in &a.stats {
                *eff.entry(k.clone()).or_insert(0.0) += v;
            }
            t.extend(a.tags.iter().cloned());
            t.insert(format!("ability:{}", a.id));
        }
        if let Some(b) = aura {
            for (k, v) in &b.0 {
                *eff.entry(k.clone()).or_insert(0.0) += v;
            }
        }
        if let Some(m) = mode.and_then(|m| content.modes.get(&m.current)) {
            for (k, v) in &m.stats {
                *eff.entry(k.clone()).or_insert(0.0) += v;
            }
            t.insert(format!("mode:{}", m.id));
        }
        // Needs below a threshold: its stat modifiers last while below.
        for n in content.needs.values() {
            let v = stats.base.get(&n.stat).copied().unwrap_or(f32::MAX);
            for th in n.thresholds.iter().filter(|th| v < th.below) {
                for (k, x) in &th.stats {
                    *eff.entry(k.clone()).or_insert(0.0) += x;
                }
            }
        }
        let add = |m: &BTreeMap<String, f32>, eff: &mut BTreeMap<String, f32>| {
            for (k, v) in m {
                *eff.entry(k.clone()).or_insert(0.0) += v;
            }
        };
        if let Some(r) = content.races.get(&race.0) {
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
                    // Weapons and clothes count only while in use (see `equipment`).
                    let in_use = !crate::equipment::is_equipment(idef) || equipment.is_some_and(|q| q.items.contains(item));
                    if in_use {
                        add(&idef.carried_stats, &mut eff);
                    }
                }
            }
        }
        // Too much weight slows down: capacity grows with strength.
        if let Some(q) = equipment.filter(|q| q.weight > 0.0) {
            let strength = eff.get(&content.bindings.strength).copied().unwrap_or(0.0);
            let capacity = params.get("inventory.base_capacity", 10.0) as f32 + strength * params.get("inventory.capacity_per_strength", 1.0) as f32;
            let over = q.weight - capacity;
            if over > 0.0 {
                *eff.entry(content.bindings.speed.clone()).or_insert(0.0) -= over * params.get("inventory.overweight_slowdown", 0.05) as f32;
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
        for d in sid.and_then(|id| roles.get(id)).into_iter().flatten() {
            add(&d.stats, &mut eff);
            t.extend(d.tags.iter().cloned());
            t.insert(format!("title:{}", d.id));
        }
        for (k, v) in eff.iter_mut() {
            let (lo, hi) = content.race_stat_bounds(&race.0, k);
            *v = v.clamp(lo, hi);
        }
        stats.effective = eff;
        tags.effective = t;
    }
}

/// Stats with a rest value drift back towards it (morale recovers, anger fades).
pub fn stat_recovery(content: Res<Content>, params: Res<crate::params::Params>, mut q: Query<&mut Stats, (With<Pawn>, Without<Dead>)>) {
    let drifting: Vec<(&String, f32, f32)> =
        content.stats.values().filter_map(|s| s.rest_value.filter(|_| s.recovery > 0.0).map(|r| (&s.id, r, s.recovery))).collect();
    let tpd = params.get("time.ticks_per_day", 24.0).max(1.0) as f32;
    let daily: Vec<(&String, f32, (f32, f32))> =
        content.stats.values().filter(|s| s.per_day != 0.0).map(|s| (&s.id, s.per_day / tpd, (s.min, s.max))).collect();
    if drifting.is_empty() && daily.is_empty() {
        return;
    }
    for mut stats in &mut q {
        for (id, step, bounds) in &daily {
            let v = stats.base.get(*id).copied().unwrap_or(0.0);
            stats.base.insert((*id).clone(), (v + step).clamp(bounds.0, bounds.1));
        }
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
pub fn decay_needs(content: Res<Content>, mut q: Query<(&mut Stats, &StatusEffects, &Race, Option<&Tags>), (With<Pawn>, Without<Dead>)>) {
    for (mut stats, statuses, race, tags) in &mut q {
        let rdef = content.races.get(&race.0);
        let abilities: Vec<&crate::content::AbilityDef> =
            tags.into_iter().flat_map(|t| t.effective.iter()).filter_map(|t| t.strip_prefix("ability:")).filter_map(|a| content.abilities.get(a)).collect();
        for (nid, nd) in &content.needs {
            let bounds = content.stat_bounds(&nd.stat);
            if rdef.is_some_and(|r| r.needs_exempt.contains(nid)) {
                stats.set_base(&nd.stat, bounds.1, bounds);
                continue;
            }
            let mut rate = 1.0 + rdef.and_then(|r| r.need_rates.get(nid)).copied().unwrap_or(0.0);
            rate += abilities.iter().filter_map(|a| a.need_rates.get(nid)).sum::<f32>();
            for (sid, st) in &statuses.active {
                if let Some(sd) = content.statuses.get(sid) {
                    rate += sd.need_rates.get(nid).copied().unwrap_or(0.0);
                    if let Some(stage) = st.stage.and_then(|i| sd.stages.get(i)) {
                        rate += stage.need_rates.get(nid).copied().unwrap_or(0.0);
                    }
                }
            }
            stats.add_base(&nd.stat, nd.per_tick * rate.max(0.0), bounds);
        }
    }
}

/// Ties of one pawn to others: friendship (grows with things done together) and attraction, 0..100.
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Bonds(pub BTreeMap<crate::ids::SimId, Bond>);

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct Bond {
    pub friendship: f32,
    pub attraction: f32,
}

impl Bonds {
    pub fn get(&self, other: crate::ids::SimId) -> Bond {
        self.0.get(&other).copied().unwrap_or_default()
    }
}

/// What a pawn found out (investigations, prophecies…), newest last; shown to its player.
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Journal(pub Vec<JournalEntry>);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JournalEntry {
    pub tick: u64,
    pub text: String,
}

/// Writes a line in a pawn's journal (the oldest go after 30).
pub fn write_journal(world: &mut World, e: Entity, text: String) {
    let tick = world.resource::<crate::time::SimClock>().tick;
    if world.get::<Journal>(e).is_none() {
        world.entity_mut(e).insert(Journal::default());
    }
    let mut j = world.get_mut::<Journal>(e).unwrap();
    j.0.push(JournalEntry { tick, text });
    if j.0.len() > 30 {
        j.0.remove(0);
    }
}
