//! WantedLevelEngine & BribeSystem: crimes seen by witnesses raise the wanted level; the neutral police
//! searches, seizes contraband and arrests; a faction leader can pay bribes from the guild treasury.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{Content, CrimeDef, FactionRole};
use crate::events::{kind, EventBuilder, EventLog};
use crate::factions::{FactionMember, Factions};
use crate::ids::SimId;
use crate::inventory::{Inventory, Stock};
use crate::map::{Position, WorldMap};
use crate::params::Params;
use crate::rng::SimRng;
use crate::stats::{Dead, Pawn};
use crate::time::SimClock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Charge {
    pub crime: String,
    pub tick: u64,
    pub severity: f32,
    pub victim: Option<SimId>,
}

#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Wanted {
    pub level: f32,
    pub charges: Vec<Charge>,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Detained {
    pub until: u64,
    /// Police faction holding the pawn.
    pub by: String,
}

fn sid(world: &World, e: Entity) -> Option<SimId> {
    world.get::<SimId>(e).copied()
}

pub fn add_wanted(world: &mut World, e: Entity, amount: f32, crime: &str, victim: Option<SimId>) {
    let tick = world.resource::<SimClock>().tick;
    if let Some(mut w) = world.get_mut::<Wanted>(e) {
        w.level = (w.level + amount).max(0.0);
        if amount > 0.0 {
            w.charges.push(Charge { crime: crime.to_string(), tick, severity: amount, victim });
        }
    }
}

pub fn clear_wanted(world: &mut World, e: Entity) {
    if let Some(mut w) = world.get_mut::<Wanted>(e) {
        w.level = 0.0;
        w.charges.clear();
    }
}

/// Records a crime: every pawn nearby that notices the culprit reports it.
pub fn commit(world: &mut World, actor: Entity, crime: &CrimeDef, victim: Option<Entity>, pos: Option<Position>) {
    let tick = world.resource::<SimClock>().tick;
    let pos = pos.or_else(|| world.get::<Position>(actor).copied());
    let name = crate::infiltration::apparent_name(world, actor);
    let victim_name = victim.map(|v| crate::infiltration::apparent_name(world, v));
    let (actor_id, victim_id) = (sid(world, actor), victim.and_then(|v| sid(world, v)));
    let faction = crate::infiltration::apparent_faction(world, actor);
    let msg = match &victim_name {
        Some(v) => format!("{name} commette un reato ({}) ai danni di {v}", crime.id),
        None => format!("{name} commette un reato ({})", crime.id),
    };
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::CRIME, msg)
            .actor(actor_id)
            .target(victim_id)
            .faction(faction)
            .pos(pos)
            .news(crime.news)
            .tags(["crime".to_string(), crime.id.clone()]),
    );
    let Some(pos) = pos else { return };
    let witnesses: Vec<Entity> = crate::sorted_entities::<Pawn>(world)
        .into_iter()
        .filter(|w| *w != actor && world.get::<Dead>(*w).is_none() && world.get::<Detained>(*w).is_none())
        .filter(|w| world.get::<Position>(*w).is_some_and(|p| p.within(&pos, crime.witness_radius)))
        .collect();
    let mut reporters = Vec::new();
    for w in witnesses {
        if crate::infiltration::notices(world, w, actor) {
            reporters.push(w);
        }
    }
    let params = world.resource::<Params>();
    let (mult, unwitnessed) = (params.f("crime.report_multiplier"), params.f("crime.unwitnessed_multiplier"));
    if reporters.is_empty() {
        if unwitnessed > 0.0 {
            add_wanted(world, actor, crime.severity * unwitnessed, &crime.id, victim_id);
        }
        return;
    }
    add_wanted(world, actor, crime.severity * mult, &crime.id, victim_id);
    let names: Vec<String> = reporters.iter().map(|r| crate::infiltration::apparent_name(world, *r)).collect();
    let level = world.get::<Wanted>(actor).map_or(0.0, |w| w.level);
    let witness_ids: Vec<SimId> = reporters.iter().filter_map(|r| sid(world, *r)).collect();
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(
            kind::CRIME_REPORTED,
            format!("{} denunciano {name}: livello ricercato {level:.0}", names.join(", ")),
        )
        .actor(actor_id)
        .pos(Some(pos))
        .news(0.1)
        .tags(["crime"])
        .data(serde_json::json!({ "witnesses": witness_ids, "wanted": level })),
    );
}

fn police_faction_of(world: &World, officer: Entity) -> Option<String> {
    let m = world.get::<FactionMember>(officer)?;
    let c = world.resource::<Content>();
    (c.factions.get(&m.faction)?.role == FactionRole::Police).then(|| m.faction.clone())
}

/// Searches a suspect, seizing contraband. Returns the seized items.
pub fn search(world: &mut World, officer: Entity, suspect: Entity) -> Vec<(String, u32)> {
    let tick = world.resource::<SimClock>().tick;
    let content = world.resource::<Content>().clone();
    let tag = content.bindings.contraband_tag.clone();
    let seized: Vec<(String, u32)> = world
        .get::<Inventory>(suspect)
        .map(|inv| {
            inv.items()
                .filter(|(i, _)| content.items.get(*i).is_some_and(|d| d.tags.contains(&tag)))
                .map(|(i, n)| (i.clone(), n))
                .collect()
        })
        .unwrap_or_default();
    let evidence = find_jail_building(world);
    for (item, n) in &seized {
        if let Some(mut inv) = world.get_mut::<Inventory>(suspect) {
            inv.remove(item, *n);
        }
        if let Some(b) = evidence {
            if let Some(mut s) = world.get_mut::<Stock>(b) {
                s.add(item, *n);
            }
        }
    }
    let per_item = world.resource::<Params>().f("crime.contraband_wanted");
    let total: u32 = seized.iter().map(|(_, n)| n).sum();
    if total > 0 {
        add_wanted(world, suspect, per_item * total as f32, "contrabbando", None);
    }
    let (oid, sid_) = (sid(world, officer), sid(world, suspect));
    let pos = world.get::<Position>(suspect).copied();
    let (on, sn) = (crate::infiltration::apparent_name(world, officer), crate::infiltration::apparent_name(world, suspect));
    let faction = police_faction_of(world, officer);
    let mut log = world.resource_mut::<EventLog>();
    log.push(
        tick,
        EventBuilder::new(kind::SEARCH, format!("{on} perquisisce {sn}"))
            .actor(oid)
            .target(sid_)
            .faction(faction.clone())
            .pos(pos)
            .news(0.2)
            .tags(["police"]),
    );
    if total > 0 {
        let list: Vec<String> = seized.iter().map(|(i, n)| format!("{n}× {}", content.items.get(i).map_or(i.as_str(), |d| d.name.as_str()))).collect();
        log.push(
            tick,
            EventBuilder::new(kind::SEIZURE, format!("{on} sequestra a {sn}: {}", list.join(", ")))
                .actor(oid)
                .target(sid_)
                .faction(faction)
                .pos(pos)
                .news(0.6)
                .tags(["police".to_string(), "contraband".to_string()])
                .data(serde_json::json!({ "items": seized })),
        );
    }
    seized
}

fn find_jail_building(world: &mut World) -> Option<Entity> {
    let tag = world.resource::<Content>().bindings.jail_zone_tag.clone();
    let content = world.resource::<Content>().clone();
    let mut q = world.query::<(Entity, &SimId, &crate::buildings::Building)>();
    let mut v: Vec<(SimId, Entity)> = q
        .iter(world)
        .filter(|(_, _, b)| content.buildings.get(&b.def).is_some_and(|d| d.tags.contains(&tag)))
        .map(|(e, id, _)| (*id, e))
        .collect();
    v.sort();
    v.first().map(|x| x.1)
}

/// Arrests a suspect whose wanted level is at least the arrest threshold (searching them first).
pub fn arrest(world: &mut World, officer: Entity, suspect: Entity) -> bool {
    if world.get::<Detained>(suspect).is_some() || world.get::<Dead>(suspect).is_some() {
        return false;
    }
    search(world, officer, suspect);
    let threshold = world.resource::<Params>().f("crime.arrest_threshold");
    let level = world.get::<Wanted>(suspect).map_or(0.0, |w| w.level);
    if level < threshold {
        return false;
    }
    let tick = world.resource::<SimClock>().tick;
    let ticks = world.resource::<Params>().get("crime.detention_ticks", 48.0) as u64;
    let by = police_faction_of(world, officer).unwrap_or_default();
    crate::jobs::release_task(world, suspect);
    world.entity_mut(suspect).insert(Detained { until: tick + ticks, by: by.clone() });
    let jail = world.resource::<Content>().bindings.jail_zone_tag.clone();
    let map = world.resource::<WorldMap>().clone();
    if let Some(p) = map.random_cell(&format!("#{jail}"), &mut world.resource_mut::<SimRng>()) {
        world.entity_mut(suspect).insert(p);
    }
    let (oid, sid_) = (sid(world, officer), sid(world, suspect));
    let pos = world.get::<Position>(officer).copied();
    let (on, sn) = (crate::infiltration::apparent_name(world, officer), crate::infiltration::apparent_name(world, suspect));
    let suspect_faction = crate::infiltration::apparent_faction(world, suspect);
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::ARREST, format!("{on} arresta {sn} (ricercato {level:.0})"))
            .actor(oid)
            .target(sid_)
            .faction(suspect_faction)
            .pos(pos)
            .news(0.9)
            .tags(["police", "arrest", "crime"])
            .data(serde_json::json!({ "police": by, "wanted": level })),
    );
    true
}

/// Price the police asks to drop all charges against a pawn.
pub fn bribe_cost(world: &World, e: Entity) -> Option<f64> {
    let w = world.get::<Wanted>(e)?;
    let police = world
        .get::<Detained>(e)
        .map(|d| d.by.clone())
        .or_else(|| {
            world.resource::<Content>().factions.values().find(|f| f.role == FactionRole::Police).map(|f| f.id.clone())
        })?;
    let corr = world.resource::<Content>().factions.get(&police).map_or(1.0, |f| f.corruptibility);
    if corr <= 0.0 {
        return None;
    }
    let rate = world.resource::<Params>().get("crime.bribe_per_wanted", 10.0);
    Some((w.level.max(1.0) as f64 * rate / corr as f64).ceil())
}

/// Pays a bribe from a faction's treasury to clear a pawn's charges and release it.
pub fn bribe(world: &mut World, payer: &str, suspect: Entity, amount: Option<f64>) -> Result<f64, String> {
    let tick = world.resource::<SimClock>().tick;
    let Some(cost) = bribe_cost(world, suspect) else {
        return Err("la polizia non accetta tangenti".into());
    };
    let amount = amount.unwrap_or(cost);
    let police = world
        .get::<Detained>(suspect)
        .map(|d| d.by.clone())
        .or_else(|| world.resource::<Content>().factions.values().find(|f| f.role == FactionRole::Police).map(|f| f.id.clone()))
        .unwrap_or_default();
    let (sid_, name) = (sid(world, suspect), crate::infiltration::apparent_name(world, suspect));
    let pos = world.get::<Position>(suspect).copied();
    if !world.resource_mut::<Factions>().spend(payer, amount) {
        return Err(format!("fondo di gilda insufficiente ({amount:.0} richiesti)"));
    }
    world.resource_mut::<Factions>().add_treasury(&police, amount);
    if amount + 1e-9 < cost {
        let pen = world.resource::<Params>().f("crime.failed_bribe_wanted");
        add_wanted(world, suspect, pen, "tentata corruzione", None);
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new(kind::BRIBE_FAILED, format!("Tentata corruzione per {name}: {amount:.0} non bastano ({cost:.0})"))
                .target(sid_)
                .faction(Some(payer.to_string()))
                .pos(pos)
                .news(0.7)
                .tags(["bribe", "corruption"]),
        );
        return Err(format!("tangente troppo bassa: servono {cost:.0}"));
    }
    clear_wanted(world, suspect);
    let was_detained = world.entity_mut(suspect).take::<Detained>().is_some();
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(
            kind::BRIBE,
            format!("{payer} paga {amount:.0} a {police}: accuse contro {name} archiviate{}", if was_detained { ", rilasciato" } else { "" }),
        )
        .target(sid_)
        .faction(Some(payer.to_string()))
        .pos(pos)
        .news(0.8)
        .tags(["bribe", "corruption", "police"])
        .data(serde_json::json!({ "amount": amount, "police": police })),
    );
    Ok(amount)
}

/// Detention expiry and slow decay of wanted levels.
pub fn crime_upkeep(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let decay = world.resource::<Params>().f("crime.wanted_decay");
    let mut released = Vec::new();
    let mut q = world.query::<(Entity, &Detained)>();
    for (e, d) in q.iter(world) {
        if d.until <= tick {
            released.push(e);
        }
    }
    released.sort_by_key(|e| world.get::<SimId>(*e).copied());
    for e in released {
        world.entity_mut(e).remove::<Detained>();
        clear_wanted(world, e);
        let (id, name) = (sid(world, e), crate::infiltration::apparent_name(world, e));
        world.resource_mut::<EventLog>().push(tick, EventBuilder::new(kind::RELEASE, format!("{name} esce di prigione")).target(id));
    }
    let mut q = world.query_filtered::<&mut Wanted, Without<Detained>>();
    for mut w in q.iter_mut(world) {
        if w.level > 0.0 {
            w.level = (w.level - decay).max(0.0);
        }
    }
}
