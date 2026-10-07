//! Class progression: classes add up. A pawn takes every class whose requirements it meets (races allowed
//! included) and loses a class it took only when the class's own `loses_when` holds (no general rule:
//! a farmer who forgets farming may stay a farmer). Classes from the template stay, unless a new class
//! replaces them. Players choose for their champion; everybody else decides alone.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{Condition, Content};
use crate::effects::{eval_condition, EffectCtx};
use crate::events::{kind, EventBuilder, EventLog};
use crate::factions::{FactionMember, Leader, Players};
use crate::ids::SimId;
use crate::map::Position;
use crate::params::Params;
use crate::rng::SimRng;
use crate::stats::{Classes, Dead, Pawn, TemplateId, Virtual};
use crate::time::SimClock;

/// The classes a pawn took by itself (not from its template) and the classes it could take now.
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClassState {
    /// Older saves have one class (or none) here.
    #[serde(default, deserialize_with = "one_or_many")]
    pub acquired: Vec<String>,
    /// Classes the pawn meets the requirements of (for a champion: what the player may choose).
    pub offers: Vec<String>,
    pub since: u64,
}

fn one_or_many<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(Option<String>),
        Many(Vec<String>),
    }
    Ok(match OneOrMany::deserialize(d)? {
        OneOrMany::One(o) => o.into_iter().collect(),
        OneOrMany::Many(v) => v,
    })
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

/// Whether `e` meets the requirements to take `class` now.
pub fn meets(world: &mut World, e: Entity, class: &str) -> bool {
    let Some(d) = world.resource::<Content>().classes.get(class).cloned() else { return false };
    let Some(req) = &d.requires else { return false };
    eval_condition(world, &EffectCtx::new(Some(e), None, format!("class:{class}")), req)
}

/// Whether `e` keeps a class it took (the class's loss condition does not hold).
pub fn keeps(world: &mut World, e: Entity, class: &str) -> bool {
    let Some(d) = world.resource::<Content>().classes.get(class).cloned() else { return false };
    let Some(lose) = &d.loses_when else { return true };
    !eval_condition(world, &EffectCtx::new(Some(e), None, format!("class:{class}")), lose)
}

/// Classes `e` could take now: not held, not replaced by one it holds, requirements met.
pub fn eligible(world: &mut World, e: Entity) -> Vec<String> {
    let content = world.resource::<Content>().clone();
    let held = world.get::<Classes>(e).map(|c| c.0.clone()).unwrap_or_default();
    let replaced: Vec<&String> = held.iter().filter_map(|h| content.classes.get(h)).flat_map(|d| d.replaces.iter()).collect();
    let mut out = Vec::new();
    for d in content.classes.values() {
        if d.requires.is_some() && !held.contains(&d.id) && !replaced.contains(&&d.id) && meets(world, e, &d.id) {
            out.push(d.id.clone());
        }
    }
    out
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

/// `e` takes `class` on top of its others; the classes it replaces (and the default class) go.
pub fn take(world: &mut World, e: Entity, class: &str) {
    let content = world.resource::<Content>().clone();
    let Some(d) = content.classes.get(class) else { return };
    let default = content.bindings.default_class.clone();
    let tick = world.resource::<SimClock>().tick;
    if let Some(mut c) = world.get_mut::<Classes>(e) {
        c.0.retain(|x| !d.replaces.contains(x) && Some(x) != default.as_ref());
        if !c.0.iter().any(|x| x == class) {
            c.0.push(class.to_string());
        }
    }
    let mut state = world.get::<ClassState>(e).cloned().unwrap_or_default();
    state.acquired.retain(|x| !d.replaces.contains(x));
    if !state.acquired.iter().any(|x| x == class) {
        state.acquired.push(class.to_string());
    }
    state.offers.retain(|x| x != class);
    state.since = tick;
    world.entity_mut(e).insert(state);
    crate::lifecycle::refresh_role(world, e);
    let name = crate::effects::name_of(world, e);
    event(world, e, kind::CLASS_GAINED, format!("{name} diventa {}", d.name), 0.2);
}

/// `e` loses a class it took; with no class left it goes back to the default class.
pub fn drop_class(world: &mut World, e: Entity, class: &str, reason: &str) {
    let content = world.resource::<Content>().clone();
    let default = content.bindings.default_class.clone().filter(|c| content.classes.contains_key(c));
    if let Some(mut c) = world.get_mut::<Classes>(e) {
        c.0.retain(|x| x != class);
        if c.0.is_empty()
            && let Some(d) = &default
        {
            c.0.push(d.clone());
        }
    }
    if let Some(mut s) = world.get_mut::<ClassState>(e) {
        s.acquired.retain(|x| x != class);
    }
    crate::lifecycle::refresh_role(world, e);
    let name = crate::effects::name_of(world, e);
    event(world, e, kind::CLASS_LOST, format!("{name} non è più {} ({reason})", class_name(world, class)), 0.1);
}

/// Periodic check: classes taken by pawns whose loss condition holds are lost; pawns take each class
/// whose requirements they meet (with some chance per check; the champion gets offers instead).
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
        let mut state = world.get::<ClassState>(e).cloned().unwrap_or_default();
        let before = state.acquired.len();
        state.acquired.retain(|a| held.contains(a));
        if state.acquired.len() != before {
            world.entity_mut(e).insert(state.clone());
        }
        for a in state.acquired.clone() {
            if !keeps(world, e, &a) {
                drop_class(world, e, &a, "condizione di perdita");
            }
        }
        let options = eligible(world, e);
        let mut state = world.get::<ClassState>(e).cloned().unwrap_or_default();
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
        state.offers = options.clone();
        world.entity_mut(e).insert(state);
        // Named characters keep who they are.
        let unique = world.get::<TemplateId>(e).and_then(|t| content.templates.get(&t.0)).is_some_and(|t| t.unique);
        if unique {
            continue;
        }
        for c in options {
            // A class taken in this same check may have replaced this one.
            let replaced = world.get::<Classes>(e).is_some_and(|h| h.0.iter().filter_map(|x| content.classes.get(x)).any(|d| d.replaces.contains(&c)));
            if !replaced && world.resource_mut::<SimRng>().chance(chance) {
                take(world, e, &c);
            }
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
    if !eligible(world, e).iter().any(|c| c == class) {
        return Err(format!("{} è sostituita da una classe che il campione ha già", class_name(world, class)));
    }
    take(world, e, class);
    Ok(format!("il campione è ora anche {}", class_name(world, class)))
}
