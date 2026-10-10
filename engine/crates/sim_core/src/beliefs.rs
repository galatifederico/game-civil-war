//! What pawns believe about factions and people, and their values.
//!
//! A belief has a stance (−100 unreliable/enemy … 100 reliable/idol) and a strength (0..100). Beliefs spread
//! like a contagion between pawns that stand close, by the speaker's credibility, the listener's
//! suggestibility and how much the listener trusts the speaker; they fade with time. Values (stats) make
//! actions of some categories more or less attractive and make witnesses judge who does them. From beliefs
//! come a faction's public trust, the drift of relations between factions and public opinion in elections.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{About, Content};
use crate::factions::{Dissent, FactionMember, Factions};
use crate::ids::SimId;
use crate::map::Position;
use crate::params::Params;
use crate::stats::{Dead, Pawn, Stats};
use crate::time::SimClock;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Belief {
    pub stance: f32,
    pub strength: f32,
}

/// Beliefs of a pawn, by subject: "f:<faction>" or "p:<pawn id>".
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Beliefs(pub BTreeMap<String, Belief>);

pub fn faction_key(f: &str) -> String {
    format!("f:{f}")
}

pub fn pawn_key(id: SimId) -> String {
    format!("p:{}", id.0)
}

/// Adds (or reinforces) a belief: the stance moves towards the new one in proportion to its strength.
pub fn believe(world: &mut World, holder: Entity, key: String, stance: f32, strength: f32) {
    if strength <= 0.0 || world.get::<Dead>(holder).is_some() {
        return;
    }
    let max = world.resource::<Params>().get("beliefs.max", 12.0) as usize;
    if world.get::<Beliefs>(holder).is_none() {
        world.entity_mut(holder).insert(Beliefs::default());
    }
    let mut b = world.get_mut::<Beliefs>(holder).unwrap();
    let e = b.0.entry(key).or_default();
    let w = (strength / (e.strength + strength)).clamp(0.0, 1.0);
    e.stance = (e.stance + (stance - e.stance) * w).clamp(-100.0, 100.0);
    e.strength = (e.strength + strength).min(100.0);
    if b.0.len() > max {
        // Keep the strongest.
        let mut v: Vec<(String, Belief)> = b.0.iter().map(|(k, v)| (k.clone(), *v)).collect();
        v.sort_by(|a, b| b.1.strength.total_cmp(&a.1.strength).then(a.0.cmp(&b.0)));
        b.0 = v.into_iter().take(max).collect();
    }
}

/// What `holder` thinks of `key` (stance × strength / 100), 0 if nothing.
pub fn opinion(world: &World, holder: Entity, key: &str) -> f32 {
    world.get::<Beliefs>(holder).and_then(|b| b.0.get(key)).map_or(0.0, |b| b.stance * b.strength / 100.0)
}

/// The belief key an `About` points to, from the subject's (and target's) point of view.
pub fn resolve_about(world: &World, about: &About, subject: Option<Entity>, target: Option<Entity>) -> Option<String> {
    let faction = |e: Option<Entity>| e.and_then(|e| world.get::<FactionMember>(e).map(|m| m.faction.clone()));
    let pawn = |e: Option<Entity>| e.and_then(|e| world.get::<SimId>(e).copied());
    match about {
        About::Subject => pawn(subject).map(pawn_key),
        About::Target => pawn(target).map(pawn_key),
        About::SubjectFaction => faction(subject).map(|f| faction_key(&f)),
        About::TargetFaction => faction(target).map(|f| faction_key(&f)),
        About::Faction(f) => Some(faction_key(f)),
        About::SubjectEnemy | About::SubjectEnemyPawn => {
            let prefix = if *about == About::SubjectEnemy { "f:" } else { "p:" };
            let own = faction(subject).map(|f| faction_key(&f));
            let worst = subject.and_then(|e| world.get::<Beliefs>(e)).and_then(|b| {
                b.0.iter()
                    .filter(|(k, v)| k.starts_with(prefix) && Some(*k) != own.as_ref() && v.stance < 0.0)
                    .min_by(|a, b| (a.1.stance * a.1.strength).total_cmp(&(b.1.stance * b.1.strength)).then(a.0.cmp(b.0)))
                    .map(|(k, _)| k.clone())
            });
            if worst.is_some() || prefix == "p:" {
                return worst;
            }
            // No grudge of its own: its faction's worst relation.
            let mine = faction(subject)?;
            let fs = world.resource::<Factions>();
            fs.states
                .keys()
                .filter(|f| **f != mine)
                .min_by(|a, b| fs.relation(&mine, a).total_cmp(&fs.relation(&mine, b)).then(a.cmp(b)))
                .map(|f| faction_key(f))
        }
    }
}

fn stat(world: &World, e: Entity, id: &str, default: f32) -> f32 {
    world.get::<Stats>(e).map_or(default, |s| s.effective.get(id).copied().unwrap_or(default))
}

/// Multiplier of an action's utility from the pawn's values and the action's categories.
pub fn values_weight(content: &Content, stats: Option<&Stats>, tags: &[String]) -> f32 {
    let Some(stats) = stats else { return 1.0 };
    let mut w = 1.0f32;
    for v in content.values.values() {
        let x = (stats.effective.get(&v.stat).copied().unwrap_or(50.0) - 50.0) / 50.0;
        for t in tags {
            if let Some(like) = v.likes.get(t) {
                w *= 1.0 + like * x;
            }
        }
    }
    w.max(0.1)
}

/// How close a pawn's values are to a faction's (0 = opposite, 1 = the same).
pub fn affinity(world: &World, e: Entity, faction: &str) -> f32 {
    let content = world.resource::<Content>();
    let Some(f) = content.factions.get(faction) else { return 0.5 };
    if f.values.is_empty() {
        return 0.5;
    }
    let mut sum = 0.0;
    for (k, fv) in &f.values {
        let Some(v) = content.values.get(k) else { continue };
        sum += (stat(world, e, &v.stat, 50.0) - fv).abs() / 100.0;
    }
    1.0 - sum / f.values.len() as f32
}

/// Witnesses of an action judge who does it by their values: liked actions raise the actor (and a little
/// its faction) in their eyes, disliked ones lower them.
pub fn witness(world: &mut World, actor: Entity, action: &str) {
    let content = world.resource::<Content>().clone();
    let Some(tags) = content.actions.get(action).map(|a| a.tags.clone()).filter(|t| !t.is_empty()) else { return };
    let Some(p) = world.get::<Position>(actor).copied() else { return };
    let Some(aid) = world.get::<SimId>(actor).copied() else { return };
    let radius = world.resource::<Params>().get("values.witness_radius", 5.0) as i32;
    let afac = world.get::<FactionMember>(actor).map(|m| m.faction.clone());
    let others: Vec<Entity> = {
        let mut q = world.query_filtered::<(Entity, &Position), (With<Pawn>, Without<Dead>)>();
        q.iter(world).filter(|(e, q)| *e != actor && q.within(&p, radius)).map(|(e, _)| e).collect()
    };
    for w in others {
        let mut score = 0.0f32;
        for v in content.values.values() {
            let x = (stat(world, w, &v.stat, 50.0) - 50.0) / 50.0;
            for t in &tags {
                score += v.likes.get(t).copied().unwrap_or(0.0) * x;
            }
        }
        if score.abs() < 0.3 {
            continue;
        }
        let stance = (score * 60.0).clamp(-100.0, 100.0);
        believe(world, w, pawn_key(aid), stance, 15.0);
        if let Some(f) = &afac {
            believe(world, w, faction_key(f), stance * 0.5, 6.0);
        }
    }
}

/// Every `beliefs.every` ticks: beliefs spread between neighbours, fade, and move the world (public trust,
/// relations between factions, dissent of pawns whose values differ from their faction's).
pub fn tick(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let p = world.resource::<Params>().clone();
    let every = p.get("beliefs.every", 3.0).max(1.0) as u64;
    if !tick.is_multiple_of(every) {
        return;
    }
    let content = world.resource::<Content>().clone();
    let (cred_stat, sugg_stat) = (content.bindings.credibility.clone(), content.bindings.suggestibility.clone());
    let radius = p.get("beliefs.radius", 2.0) as i32;
    let spread = p.get("beliefs.spread", 0.15) as f32;
    let fade = p.get("beliefs.decay", 0.3) as f32 * every as f32;
    let pawns: Vec<(Entity, Position, SimId)> = {
        let mut q = world.query_filtered::<(Entity, &Position, &SimId), (With<Pawn>, Without<Dead>)>();
        let mut v: Vec<_> = q.iter(world).map(|(e, p, i)| (e, *p, *i)).collect();
        v.sort_by_key(|x| x.2);
        v
    };
    // 1. Spread: each speaker passes its strong beliefs to its neighbours.
    let mut told: Vec<(Entity, String, f32, f32)> = Vec::new();
    for (a, pa, ida) in &pawns {
        let Some(bs) = world.get::<Beliefs>(*a).cloned() else { continue };
        let cred = stat(world, *a, &cred_stat, 50.0) / 100.0;
        for (b, pb, _) in &pawns {
            if a == b || !pa.within(pb, radius) {
                continue;
            }
            let sugg = stat(world, *b, &sugg_stat, 40.0) / 100.0;
            let trust = (1.0 + opinion(world, *b, &pawn_key(*ida)) / 100.0).clamp(0.0, 2.0);
            let rate = spread * cred * sugg * trust;
            if rate <= 0.0 {
                continue;
            }
            for (k, bel) in bs.0.iter().filter(|(_, x)| x.strength >= 20.0) {
                told.push((*b, k.clone(), bel.stance, bel.strength * rate));
            }
        }
    }
    for (b, k, stance, strength) in told {
        believe(world, b, k, stance, strength);
    }
    // 2. Fade.
    for (e, _, _) in &pawns {
        if let Some(mut bs) = world.get_mut::<Beliefs>(*e) {
            for b in bs.0.values_mut() {
                b.strength -= fade;
            }
            bs.0.retain(|_, b| b.strength >= 1.0);
        }
    }
    // 3. Public trust of each faction (what outsiders believe), relations drifting towards what members believe.
    let ids: Vec<String> = world.resource::<Factions>().states.keys().cloned().collect();
    let mut trust: BTreeMap<String, (f32, f32)> = BTreeMap::new();
    let mut views: BTreeMap<(String, String), (f32, f32)> = BTreeMap::new();
    for (e, _, _) in &pawns {
        let mine = world.get::<FactionMember>(*e).map(|m| m.faction.clone());
        for f in &ids {
            let o = opinion(world, *e, &faction_key(f));
            if mine.as_deref() != Some(f.as_str()) {
                let t = trust.entry(f.clone()).or_default();
                t.0 += o;
                t.1 += 1.0;
            }
            if let Some(m) = &mine
                && m != f
                && o != 0.0
            {
                let v = views.entry((m.clone(), f.clone())).or_default();
                v.0 += o;
                v.1 += 1.0;
            }
        }
    }
    let drift = p.get("factions.relation_drift", 0.02) as f32 * every as f32;
    {
        let mut fs = world.resource_mut::<Factions>();
        for (f, (sum, n)) in &trust {
            if let Some(s) = fs.states.get_mut(f) {
                s.public_trust = if *n > 0.0 { sum / n } else { 0.0 };
            }
        }
        for ((a, b), (sum, n)) in &views {
            let view = sum / n;
            if let Some(s) = fs.states.get_mut(a) {
                let r = s.relations.entry(b.clone()).or_insert(0.0);
                *r = (*r + (view - *r) * drift).clamp(-100.0, 100.0);
            }
        }
    }
    // 4. Values against the faction: dissent grows slowly (the only way to disobey is to leave).
    let rate = p.get("values.dissent", 0.05) as f32 * every as f32;
    for (e, _, _) in &pawns {
        let Some(f) = world.get::<FactionMember>(*e).map(|m| m.faction.clone()) else { continue };
        let a = affinity(world, *e, &f);
        if a < 0.5
            && let Some(mut d) = world.get_mut::<Dissent>(*e)
        {
            d.0 += (0.5 - a) * 2.0 * rate;
        }
    }
}
