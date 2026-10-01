//! InfiltrationEngine: ShapeshiftFramework (apparent race/faction/name), IdentityTheftSystem,
//! Stealth & ShadowFramework (visibility, invisibility, aggro reset) and ExposureEngine (cover level).

use std::collections::BTreeSet;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::Content;
use crate::events::{kind, EventBuilder, EventLog};
use crate::factions::FactionMember;
use crate::ids::SimId;
use crate::jobs::{release_task, JobTarget, Task, WorkPriorities};
use crate::map::Position;
use crate::params::Params;
use crate::rng::SimRng;
use crate::stats::{Dead, DisplayName, Pawn, Race, Stats, Tags};
use crate::time::SimClock;

/// Apparent identity shown to everybody else.
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Disguise {
    pub race: Option<String>,
    pub faction: Option<String>,
    pub name: Option<String>,
}

/// CoverLevel 0..100: at 0 the true identity is exposed.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Cover(pub f32);

#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct StealthState {
    pub bonus: f32,
    pub until: u64,
}

pub fn apparent_name(world: &World, e: Entity) -> String {
    world
        .get::<Disguise>(e)
        .and_then(|d| d.name.clone())
        .or_else(|| world.get::<DisplayName>(e).map(|n| n.0.clone()))
        .unwrap_or_else(|| "?".into())
}

pub fn apparent_faction(world: &World, e: Entity) -> Option<String> {
    world
        .get::<Disguise>(e)
        .and_then(|d| d.faction.clone())
        .or_else(|| world.get::<FactionMember>(e).map(|m| m.faction.clone()))
        // A building belongs to the faction that owns it.
        .or_else(|| match world.get::<crate::buildings::Building>(e).map(|b| &b.owner) {
            Some(crate::buildings::Owner::Faction(f)) => Some(f.clone()),
            _ => None,
        })
}

pub fn apparent_race(world: &World, e: Entity) -> Option<String> {
    world.get::<Disguise>(e).and_then(|d| d.race.clone()).or_else(|| world.get::<Race>(e).map(|r| r.0.clone()))
}

/// Factions whose board jobs this pawn can take: its own, plus the one it infiltrated.
pub fn factions_seen_as_member(world: &World, e: Entity) -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    if let Some(m) = world.get::<FactionMember>(e) {
        s.insert(m.faction.clone());
    }
    if let Some(f) = world.get::<Disguise>(e).and_then(|d| d.faction.clone()) {
        s.insert(f);
    }
    s
}

/// Tags others can see: the true race's tags are replaced by the apparent race's while disguised.
pub fn visible_tags(world: &World, e: Entity) -> BTreeSet<String> {
    let mut tags: BTreeSet<String> = world.get::<Tags>(e).map(|t| t.effective.union(&t.base).cloned().collect()).unwrap_or_default();
    if let Some(d) = world.get::<Disguise>(e) {
        let content = world.resource::<Content>();
        if let Some(true_race) = world.get::<Race>(e).and_then(|r| content.races.get(&r.0)) {
            for t in &true_race.tags {
                tags.remove(t);
            }
            tags.remove(&format!("race:{}", true_race.id));
        }
        if let Some(r) = d.race.as_ref().and_then(|r| content.races.get(r)) {
            tags.extend(r.tags.iter().cloned());
            tags.insert(format!("race:{}", r.id));
        }
    }
    tags
}

fn stealth_of(world: &World, e: Entity) -> f32 {
    let stat = &world.resource::<Content>().bindings.stealth;
    world.get::<Stats>(e).map_or(0.0, |s| s.get(stat))
        + world.get::<StealthState>(e).map_or(0.0, |s| {
            if s.until > world.resource::<SimClock>().tick { s.bonus } else { 0.0 }
        })
}

fn perception_of(world: &World, e: Entity) -> f32 {
    let stat = &world.resource::<Content>().bindings.perception;
    let cap = world.get::<crate::anatomy::Body>(e).map_or(1.0, |b| b.capacity("sight"));
    world.get::<Stats>(e).map_or(10.0, |s| s.get(stat)) * cap
}

pub fn invisible(world: &World, e: Entity) -> bool {
    let tick = world.resource::<SimClock>().tick;
    world.get::<StealthState>(e).is_some_and(|s| s.until > tick && s.bonus >= 100.0)
}

/// Deterministic visibility: in range, not invisible and perception at least stealth − 5.
pub fn can_see(world: &World, observer: Entity, target: Entity, range: i32) -> bool {
    let (Some(a), Some(b)) = (world.get::<Position>(observer), world.get::<Position>(target)) else { return false };
    a.within(b, range) && !invisible(world, target) && perception_of(world, observer) + 5.0 >= stealth_of(world, target)
}

/// Random perception roll (crimes, suspicious acts).
pub fn notices(world: &mut World, observer: Entity, target: Entity) -> bool {
    if invisible(world, target) {
        return false;
    }
    let p = world.resource::<Params>();
    let (base, scale) = (p.f("crime.witness_base"), p.f("stealth.detection_scale"));
    let diff = perception_of(world, observer) - stealth_of(world, target);
    let chance = (base + diff * scale / 10.0).clamp(0.05, 0.99);
    world.resource_mut::<SimRng>().chance(chance)
}

pub fn shapeshift(world: &mut World, e: Entity, d: crate::content::Disguise, copied_from: Option<Entity>) {
    let disguise = Disguise { race: d.race, faction: d.faction, name: d.name };
    world.entity_mut(e).insert(disguise.clone());
    if world.get::<Cover>(e).is_none() {
        world.entity_mut(e).insert(Cover(100.0));
    }
    let tick = world.resource::<SimClock>().tick;
    let id = world.get::<SimId>(e).copied();
    let src = copied_from.and_then(|c| world.get::<SimId>(c).copied());
    let name = world.get::<DisplayName>(e).map_or("?".into(), |n| n.0.clone());
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::SHAPESHIFT, format!("{name} assume l'aspetto di {}", disguise.name.clone().unwrap_or_else(|| "un altro".into())))
            .actor(id)
            .target(src)
            .tags(["shapeshift", "secret"]),
    );
}

pub fn revert(world: &mut World, e: Entity) {
    world.entity_mut(e).remove::<Disguise>();
}

pub fn mod_cover(world: &mut World, e: Entity, amount: f32, reason: &str) {
    if world.get::<Disguise>(e).is_none() {
        return;
    }
    let level = {
        let mut c = match world.get_mut::<Cover>(e) {
            Some(c) => c,
            None => return,
        };
        c.0 = (c.0 + amount).clamp(0.0, 100.0);
        c.0
    };
    if level <= 0.0 {
        expose(world, e, reason, None);
    }
}

/// Reveals the true identity of a disguised entity.
pub fn expose(world: &mut World, e: Entity, reason: &str, by: Option<Entity>) {
    let Some(d) = world.get::<Disguise>(e).cloned() else { return };
    let tick = world.resource::<SimClock>().tick;
    let fake = d.name.clone().unwrap_or_else(|| apparent_name(world, e));
    world.entity_mut(e).remove::<Disguise>();
    world.entity_mut(e).insert(Cover(0.0));
    let content = world.resource::<Content>();
    let real_race = world.get::<Race>(e).and_then(|r| content.races.get(&r.0)).map_or("?".into(), |r| r.name.clone());
    let real_faction = world.get::<FactionMember>(e).and_then(|m| content.factions.get(&m.faction)).map(|f| f.name.clone());
    let real_name = world.get::<DisplayName>(e).map_or("?".into(), |n| n.0.clone());
    let (id, by_id) = (world.get::<SimId>(e).copied(), by.and_then(|b| world.get::<SimId>(b).copied()));
    let pos = world.get::<Position>(e).copied();
    let faction = world.get::<FactionMember>(e).map(|m| m.faction.clone());
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(
            kind::EXPOSED,
            format!(
                "Smascherato! {fake} è in realtà {real_name} ({real_race}{}) — {reason}",
                real_faction.map(|f| format!(", {f}")).unwrap_or_default()
            ),
        )
        .actor(by_id)
        .target(id)
        .faction(faction)
        .pos(pos)
        .news(1.0)
        .tags(["exposed", "infiltration"]),
    );
    let wanted = world.resource::<Params>().get("infiltration.exposed_wanted", 8.0) as f32;
    crate::crime::add_wanted(world, e, wanted, "infiltrazione", None);
}

pub fn grant_stealth(world: &mut World, e: Entity, amount: f32, duration: u64) {
    let tick = world.resource::<SimClock>().tick;
    world.entity_mut(e).insert(StealthState { bonus: amount, until: tick + duration });
}

/// Smoke bomb: everybody targeting this entity loses track of it.
pub fn reset_aggro(world: &mut World, e: Entity) {
    let Some(me) = world.get::<SimId>(e).copied() else { return };
    let hunters: Vec<Entity> = crate::sorted_entities::<Task>(world)
        .into_iter()
        .filter(|h| {
            world.get::<Task>(*h).and_then(|t| t.job.as_ref()).is_some_and(|j| j.target == JobTarget::Entity(me))
        })
        .collect();
    let tick = world.resource::<SimClock>().tick;
    for h in hunters {
        release_task(world, h);
        if let Some(mut b) = world.get_mut::<crate::ai::Brain>(h) {
            b.current = None;
            for a in b.actions.clone() {
                b.cooldowns.entry(a).and_modify(|c| *c = (*c).max(tick + 2)).or_insert(tick + 2);
            }
        }
    }
}

/// A disguised pawn did something suspicious: witnesses that notice lower its cover.
pub fn suspicious_act(world: &mut World, e: Entity) {
    if world.get::<Disguise>(e).is_none() {
        return;
    }
    let Some(pos) = world.get::<Position>(e).copied() else { return };
    let loss = world.resource::<Params>().f("infiltration.suspicious_cover_loss");
    let range = world.resource::<Params>().get("ai.perception_range", 6.0) as i32;
    let witnesses: Vec<Entity> = crate::sorted_entities::<Pawn>(world)
        .into_iter()
        .filter(|w| *w != e && world.get::<Dead>(*w).is_none() && world.get::<Position>(*w).is_some_and(|p| p.within(&pos, range)))
        .collect();
    for w in witnesses {
        if notices(world, w, e) {
            mod_cover(world, e, -loss, "comportamento sospetto");
            if world.get::<Disguise>(e).is_none() {
                break;
            }
        }
    }
}

/// The thief takes the victim's place: name, apparent race and faction, work matrix and squad slot.
/// The victim disappears (kept as dead: "replaced").
pub fn steal_identity(world: &mut World, thief: Entity, victim: Entity) -> Result<String, String> {
    if world.get::<Dead>(victim).is_some() {
        return Err("la vittima è morta".into());
    }
    if world.get::<crate::factions::Leader>(victim).is_some() {
        return Err("un campione non si può sostituire".into());
    }
    let name = world.get::<DisplayName>(victim).map(|n| n.0.clone()).ok_or("vittima senza nome")?;
    let d = crate::content::Disguise {
        race: world.get::<Race>(victim).map(|r| r.0.clone()),
        faction: world.get::<FactionMember>(victim).map(|m| m.faction.clone()),
        name: Some(name.clone()),
    };
    let wp = world.get::<WorkPriorities>(victim).cloned();
    let (tid, vid) = (world.get::<SimId>(thief).copied(), world.get::<SimId>(victim).copied());
    // Squad slot.
    if let (Some(t), Some(v)) = (tid, vid) {
        let mut squads = world.resource_mut::<crate::squads::Squads>();
        for s in squads.squads.values_mut() {
            for m in s.members.iter_mut() {
                if *m == v {
                    *m = t;
                }
            }
            if s.leader == Some(v) {
                s.leader = Some(t);
            }
        }
    }
    let vpos = world.get::<Position>(victim).copied();
    crate::lifecycle::kill(world, victim, "sostituito da un impostore", Some(thief));
    if let Some(p) = vpos {
        world.entity_mut(victim).remove::<Position>();
        world.entity_mut(thief).insert(p);
    }
    shapeshift(world, thief, d, Some(victim));
    world.entity_mut(thief).insert(Cover(100.0));
    if let Some(mut w) = world.get_mut::<WorkPriorities>(thief)
        && let Some(v) = wp {
            w.overrides = v.matrix();
        }
    let tick = world.resource::<SimClock>().tick;
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::IDENTITY_STOLEN, format!("Qualcuno ha preso il posto di {name}"))
            .actor(tid)
            .target(vid)
            .pos(vpos)
            .tags(["infiltration", "secret"]),
    );
    Ok(format!("identità di {name} rubata"))
}
