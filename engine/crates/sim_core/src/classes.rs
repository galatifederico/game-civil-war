//! Class progression: a pawn takes a class when it meets the class's requirements and loses it when it
//! falls well below them (the thresholds are relaxed by `classes.keep_ratio`, so nobody flips class every
//! tick). Players choose for their champion; everybody else decides alone.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{ClassDef, Condition, Content};
use crate::effects::{eval_condition, EffectCtx};
use crate::events::{kind, EventBuilder, EventLog};
use crate::factions::{FactionMember, Leader, Players};
use crate::ids::SimId;
use crate::map::Position;
use crate::params::Params;
use crate::rng::SimRng;
use crate::stats::{Classes, Dead, Pawn, Race, TemplateId, Virtual};
use crate::time::SimClock;

/// The class a pawn took by itself (not from its template) and the classes it could take now.
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClassState {
    pub acquired: Option<String>,
    /// Classes the pawn meets the requirements of (for a champion: what the player may choose).
    pub offers: Vec<String>,
    pub since: u64,
}

/// The same condition with every threshold loosened by `ratio` (0.9: "at least 80" becomes "at least 72",
/// "below 30" becomes "below 33.3").
pub fn relaxed(c: &Condition, ratio: f32) -> Condition {
    match c {
        Condition::All(v) => Condition::All(v.iter().map(|x| relaxed(x, ratio)).collect()),
        Condition::Any(v) => Condition::Any(v.iter().map(|x| relaxed(x, ratio)).collect()),
        Condition::StatAtLeast { stat, value } if *value > 0.0 => Condition::StatAtLeast { stat: stat.clone(), value: value * ratio },
        Condition::StatBelow { stat, value } if *value > 0.0 && ratio > 0.0 => Condition::StatBelow { stat: stat.clone(), value: value / ratio },
        Condition::MoneyAtLeast(m) => Condition::MoneyAtLeast(m * ratio as f64),
        other => other.clone(),
    }
}

fn race_allows(world: &World, e: Entity, d: &ClassDef) -> bool {
    d.races.is_empty() || world.get::<Race>(e).is_some_and(|r| d.races.contains(&r.0))
}

/// Whether `e` meets the requirements to take `class` now.
pub fn meets(world: &mut World, e: Entity, class: &str) -> bool {
    let Some(d) = world.resource::<Content>().classes.get(class).cloned() else { return false };
    let Some(req) = &d.requires else { return false };
    race_allows(world, e, &d) && eval_condition(world, &EffectCtx::new(Some(e), None, format!("class:{class}")), req)
}

/// Whether `e` still deserves a class it took (the relaxed requirements).
pub fn keeps(world: &mut World, e: Entity, class: &str) -> bool {
    let Some(d) = world.resource::<Content>().classes.get(class).cloned() else { return false };
    let Some(req) = &d.requires else { return true };
    let ratio = world.resource::<Params>().get("classes.keep_ratio", 0.9) as f32;
    eval_condition(world, &EffectCtx::new(Some(e), None, format!("class:{class}")), &relaxed(req, ratio))
}

/// Classes `e` could take now and does not have, best first (priority, then id).
pub fn eligible(world: &mut World, e: Entity) -> Vec<String> {
    let content = world.resource::<Content>().clone();
    let held = world.get::<Classes>(e).map(|c| c.0.clone()).unwrap_or_default();
    let mut out: Vec<&ClassDef> = Vec::new();
    for d in content.classes.values() {
        if d.requires.is_some() && !held.contains(&d.id) && meets(world, e, &d.id) {
            out.push(d);
        }
    }
    out.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.id.cmp(&b.id)));
    out.into_iter().map(|d| d.id.clone()).collect()
}

fn event(world: &mut World, e: Entity, kind: &str, msg: String, news: f32) {
    let tick = world.resource::<SimClock>().tick;
    let id = world.get::<SimId>(e).copied();
    let pos = world.get::<Position>(e).copied();
    let faction = world.get::<FactionMember>(e).map(|m| m.faction.clone());
    world.resource_mut::<EventLog>().push(tick, EventBuilder::new(kind, msg).target(id).pos(pos).faction(faction).news(news).tags(["class"]));
}

fn class_name(world: &World, class: &str) -> String {
    world.resource::<Content>().classes.get(class).map_or(class.to_string(), |d| d.name.clone())
}

/// `e` becomes `class` (with the classes it includes) and leaves the others.
pub fn take(world: &mut World, e: Entity, class: &str) {
    let content = world.resource::<Content>().clone();
    let Some(d) = content.classes.get(class) else { return };
    let mut list = vec![class.to_string()];
    list.extend(d.includes.iter().filter(|c| content.classes.contains_key(*c)).cloned());
    list.dedup();
    let tick = world.resource::<SimClock>().tick;
    if let Some(mut c) = world.get_mut::<Classes>(e) {
        c.0 = list;
    }
    world.entity_mut(e).insert(ClassState { acquired: Some(class.to_string()), offers: vec![], since: tick });
    crate::lifecycle::refresh_role(world, e);
    let name = crate::effects::name_of(world, e);
    event(world, e, kind::CLASS_GAINED, format!("{name} diventa {}", d.name), 0.3);
}

/// `e` loses the class it took and goes back to the default class.
pub fn drop_acquired(world: &mut World, e: Entity, reason: &str) {
    let content = world.resource::<Content>().clone();
    let Some(class) = world.get::<ClassState>(e).and_then(|s| s.acquired.clone()) else { return };
    let mut gone = vec![class.clone()];
    if let Some(d) = content.classes.get(&class) {
        gone.extend(d.includes.iter().cloned());
    }
    let default = content.bindings.default_class.clone().filter(|c| content.classes.contains_key(c));
    if let Some(mut c) = world.get_mut::<Classes>(e) {
        c.0.retain(|x| !gone.contains(x));
        if c.0.is_empty()
            && let Some(d) = &default
        {
            c.0.push(d.clone());
        }
    }
    if let Some(mut s) = world.get_mut::<ClassState>(e) {
        s.acquired = None;
    }
    crate::lifecycle::refresh_role(world, e);
    let name = crate::effects::name_of(world, e);
    event(world, e, kind::CLASS_LOST, format!("{name} non è più {} ({reason})", class_name(world, &class)), 0.2);
}

/// A pawn with only the default class (or a class it took itself) may change class freely; one with a
/// class from its template only when the new class replaces or includes it.
fn may_adopt(world: &World, held: &[String], new: &ClassDef, acquired: Option<&String>) -> bool {
    let content = world.resource::<Content>();
    let default = content.bindings.default_class.as_ref();
    let plain = held.iter().all(|c| Some(c) == default || Some(c) == acquired || new.includes.contains(c));
    if !plain {
        return held.iter().any(|c| new.replaces.contains(c) || new.includes.contains(c));
    }
    // Moving from a class it took to another one: only towards a more important class.
    match acquired.and_then(|a| content.classes.get(a)) {
        Some(cur) => new.priority > cur.priority || new.replaces.contains(&cur.id) || new.includes.contains(&cur.id),
        None => true,
    }
}

/// Periodic check: classes taken by pawns that no longer deserve them are lost; pawns that meet the
/// requirements of a class may take it (the champion gets offers instead).
pub fn progression(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let every = world.resource::<Params>().get("classes.check_every", 12.0).max(1.0) as u64;
    if !tick.is_multiple_of(every) {
        return;
    }
    let chance = world.resource::<Params>().get("classes.adopt_chance", 0.35) as f32;
    let content = world.resource::<Content>().clone();
    if content.classes.values().all(|c| c.requires.is_none()) {
        return;
    }
    for e in crate::sorted_entities::<Pawn>(world) {
        if world.get::<Dead>(e).is_some() || world.get::<Virtual>(e).is_some() {
            continue;
        }
        let held = world.get::<Classes>(e).map(|c| c.0.clone()).unwrap_or_default();
        if held.iter().any(|c| content.classes.get(c).is_some_and(|d| d.innate)) {
            continue;
        }
        let mut state = world.get::<ClassState>(e).cloned().unwrap_or_default();
        if let Some(a) = state.acquired.clone() {
            if !held.contains(&a) {
                state.acquired = None;
                world.entity_mut(e).insert(state.clone());
            } else if !keeps(world, e, &a) {
                drop_acquired(world, e, "non ha più i requisiti");
                continue;
            }
        }
        let options = eligible(world, e);
        let champion = world.get::<Leader>(e).is_some();
        if champion {
            let new: Vec<String> = options.iter().filter(|c| !state.offers.contains(c)).cloned().collect();
            state.offers = options;
            world.entity_mut(e).insert(state);
            if !new.is_empty() {
                let names: Vec<String> = new.iter().map(|c| class_name(world, c)).collect();
                let name = crate::effects::name_of(world, e);
                event(world, e, kind::CLASS_OFFER, format!("{name} ha i requisiti per diventare {}", names.join(", ")), 0.0);
            }
            continue;
        }
        // Named characters keep who they are.
        let unique = world.get::<TemplateId>(e).and_then(|t| content.templates.get(&t.0)).is_some_and(|t| t.unique);
        state.offers = options.clone();
        world.entity_mut(e).insert(state.clone());
        if unique {
            continue;
        }
        let pick = options
            .iter()
            .filter_map(|c| content.classes.get(c))
            .find(|d| may_adopt(world, &held, d, state.acquired.as_ref()))
            .map(|d| d.id.clone());
        if let Some(c) = pick
            && world.resource_mut::<SimRng>().chance(chance)
        {
            take(world, e, &c);
        }
    }
}

/// The player picks one of the classes its champion is offered.
pub fn accept(world: &mut World, player: &str, class: &str) -> Result<String, String> {
    let p = world.resource::<Players>().players.get(player).cloned().ok_or_else(|| format!("giocatore '{player}' inesistente"))?;
    let id = p.leader.ok_or("nessun campione")?;
    let e = world.resource::<crate::ids::IdIndex>().get(id).ok_or("campione inesistente")?;
    if world.get::<Dead>(e).is_some() {
        return Err("il campione è morto".into());
    }
    if world.get::<Classes>(e).is_some_and(|c| c.0.iter().any(|x| x == class)) {
        return Err("è già di questa classe".into());
    }
    if !meets(world, e, class) {
        return Err(format!("il campione non ha i requisiti per diventare {}", class_name(world, class)));
    }
    take(world, e, class);
    Ok(format!("il campione è ora {}", class_name(world, class)))
}
