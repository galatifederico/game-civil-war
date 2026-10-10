//! Roles (titles): who may hold them, how they change hands (seat, succession, election, appointment,
//! challenge, coup, purchase), the powers they give and when they are lost.

use bevy_ecs::prelude::*;

use crate::content::{Condition, Content, TitleDef, TitleMode};
use crate::effects::{eval_condition, EffectCtx};
use crate::events::{kind, EventBuilder, EventLog};
use crate::factions::{FactionMember, Factions, Leader, Players, Titles};
use crate::ids::{IdIndex, SimId};
use crate::map::Position;
use crate::params::Params;
use crate::rng::SimRng;
use crate::stats::{Dead, Pawn, Stats, Virtual, Wallet};
use crate::time::SimClock;

pub fn holder_entity(world: &World, title: &str) -> Option<Entity> {
    let id = world.resource::<Titles>().holder(title)?;
    world.resource::<IdIndex>().get(id)
}

/// Titles held by an entity.
pub fn held(world: &World, e: Entity) -> Vec<String> {
    world.get::<SimId>(e).map(|id| world.resource::<Titles>().held_by(*id)).unwrap_or_default()
}

/// Whether `e` meets the requirements of a role (alive, free, in the faction if needed, its condition).
pub fn eligible(world: &mut World, e: Entity, t: &TitleDef) -> bool {
    if world.get::<Dead>(e).is_some() || world.get::<Virtual>(e).is_some() || world.get::<crate::crime::Detained>(e).is_some() {
        return false;
    }
    if t.members_only && !t.faction.is_empty() && !world.get::<FactionMember>(e).is_some_and(|m| m.faction == t.faction) {
        return false;
    }
    eval_condition(world, &EffectCtx::new(Some(e), None, format!("title:{}", t.id)), &t.claim_requires)
}

/// Whether the holder keeps the role: the role's own loss condition does not hold (no general rule).
pub fn keeps(world: &mut World, e: Entity, t: &TitleDef) -> bool {
    match &t.loses_when {
        Some(c) => !eval_condition(world, &EffectCtx::new(Some(e), None, format!("title:{}", t.id)), c),
        None => true,
    }
}

/// Stats named by a condition (used as the default candidate score).
fn condition_stats(c: &Condition, out: &mut Vec<String>) {
    match c {
        Condition::All(v) | Condition::Any(v) => v.iter().for_each(|x| condition_stats(x, out)),
        Condition::StatAtLeast { stat, .. } => out.push(stat.clone()),
        _ => {}
    }
}

/// How good a candidate is: the weighted stats of `score` ("money" = the wallet), or the sum of the stats
/// the requirements ask for.
pub fn score(world: &World, e: Entity, t: &TitleDef) -> f32 {
    let stats = world.get::<Stats>(e);
    let stat = |id: &str| -> f32 {
        if id == "money" {
            world.get::<Wallet>(e).map_or(0.0, |w| w.0 as f32)
        } else {
            stats.map_or(0.0, |s| s.get(id))
        }
    };
    if t.mode == TitleMode::Purchase && t.score.is_empty() {
        return stat("money");
    }
    if t.score.is_empty() {
        let mut ids = Vec::new();
        condition_stats(&t.claim_requires, &mut ids);
        return ids.iter().map(|s| stat(s)).sum();
    }
    t.score.iter().map(|(k, w)| stat(k) * w).sum()
}

/// Gives a role to `e` (taking it from the current holder, if any). `how` explains it in the news.
pub fn assign(world: &mut World, title: &str, e: Entity, how: &str) {
    let Some(t) = world.resource::<Content>().titles.get(title).cloned() else { return };
    let Some(id) = world.get::<SimId>(e).copied() else { return };
    let tick = world.resource::<SimClock>().tick;
    let old = holder_entity(world, title);
    {
        let mut ts = world.resource_mut::<Titles>();
        ts.holders.insert(title.to_string(), Some(id));
        ts.vacant_since.remove(title);
        ts.since.insert(title.to_string(), tick);
        ts.failing.remove(title);
    }
    if let Some(o) = old.filter(|o| *o != e) {
        crate::lifecycle::refresh_role(world, o);
    }
    crate::lifecycle::refresh_role(world, e);
    let content = world.resource::<Content>().clone();
    let name = crate::effects::name_of(world, e);
    let player = world.get::<Leader>(e).map(|l| l.player.clone());
    let mut msg = match t.mode {
        TitleMode::Seat => format!("{name} sale sul trono: nuovo {}", t.name),
        _ if how.is_empty() => format!("{name} è il nuovo {}", t.name),
        _ => format!("{name} è il nuovo {} ({how})", t.name),
    };
    if let Some(p) = &player {
        let pf = world.resource::<Players>().players.get(p).map(|x| x.faction.clone());
        if let Some(pf) = pf {
            if t.victory_points != 0
                && let Some(s) = world.resource_mut::<Factions>().states.get_mut(&pf)
            {
                s.victory_points += t.victory_points;
                msg += &format!(" (+{} punti vittoria)", t.victory_points);
            }
        }
        let owned_by_other = world.resource::<Factions>().states.get(&t.faction).and_then(|s| s.controlled_by.clone()).is_some_and(|o| &o != p);
        if t.grants_faction_control && !owned_by_other {
            if let Some(s) = world.resource_mut::<Factions>().states.get_mut(&t.faction) {
                s.controlled_by = Some(p.clone());
            }
            msg += &format!("; il giocatore {p} controlla ora {}", content.factions.get(&t.faction).map_or("", |f| f.name.as_str()));
        }
    } else if t.mode == TitleMode::Seat {
        // A throne makes its AI claimant the leader of the faction.
        let rank = content.factions.get(&t.faction).and_then(|f| f.ranks.iter().filter(|r| r.unique).max_by_key(|r| r.level)).map(|r| r.id.clone());
        if let Some(rank) = rank {
            world.entity_mut(e).insert(FactionMember { faction: t.faction.clone(), rank, joined: tick });
        }
    }
    let pos = world.get::<Position>(e).copied();
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::SUCCESSION, msg)
            .actor(Some(id))
            .faction((!t.faction.is_empty()).then(|| t.faction.clone()))
            .pos(pos)
            .news(t.news.max(0.6))
            .tags(["succession", "politics", t.id.as_str()]),
    );
}

/// Takes a role away from its holder (it becomes vacant).
pub fn vacate(world: &mut World, title: &str, reason: &str) {
    let old = holder_entity(world, title);
    if let Some(o) = old {
        let tick = world.resource::<SimClock>().tick;
        let name = crate::effects::name_of(world, o);
        let tname = world.resource::<Content>().titles.get(title).map_or(title.to_string(), |t| t.name.clone());
        let id = world.get::<SimId>(o).copied();
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new(kind::ROLE_LOST, format!("{name} non è più {tname} ({reason})")).target(id).news(0.6).tags(["politics", title]),
        );
    }
    crate::social::vacate_title(world, title);
    world.resource_mut::<Titles>().failing.remove(title);
    if let Some(o) = old {
        crate::lifecycle::refresh_role(world, o);
    }
}

/// The eligible candidates for a role, best first. Pawns already holding another role stay out.
fn candidates(world: &mut World, t: &TitleDef, luck: f32) -> Vec<(f32, SimId, Entity)> {
    let mut out = Vec::new();
    for e in crate::sorted_entities::<Pawn>(world) {
        let others = held(world, e);
        if others.iter().any(|x| x != &t.id) || !eligible(world, e, t) {
            continue;
        }
        let mut s = score(world, e, t);
        // In elections public opinion counts: what everybody believes of the candidate.
        if t.mode == TitleMode::Election {
            let w = world.resource::<Params>().get("titles.opinion_weight", 0.5) as f32;
            s *= (1.0 + public_opinion(world, e) / 100.0 * w).max(0.1);
        }
        let roll = if luck > 0.0 { world.resource_mut::<SimRng>().next_f32() } else { 0.0 };
        let s = s * (1.0 - luck / 2.0 + luck * roll);
        out.push((s, *world.get::<SimId>(e).unwrap(), e));
    }
    out.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    out
}

fn mode_name(m: TitleMode) -> &'static str {
    match m {
        TitleMode::Seat => "si siede sul trono",
        TitleMode::Succession => "per successione",
        TitleMode::Election => "eletto",
        TitleMode::Appointment => "nominato",
        TitleMode::Challenge => "nessuno lo contende",
        TitleMode::Coup => "nessuno lo contende",
        TitleMode::Purchase => "compra il posto",
    }
}

/// Roles upkeep: holders below the requirements for too long lose them, mandates expire and are voted
/// again, vacancies are filled according to each role's mode (thrones are claimed in `social::succession`).
pub fn roles_tick(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let every = world.resource::<Params>().get("titles.check_every", 6.0).max(1.0) as u64;
    if !tick.is_multiple_of(every) {
        return;
    }
    let luck = world.resource::<Params>().f("titles.election_luck");
    let content = world.resource::<Content>().clone();
    for t in content.titles.values() {
        if !world.resource::<Titles>().holders.contains_key(&t.id) {
            continue;
        }
        if let Some(h) = holder_entity(world, &t.id) {
            if world.get::<Dead>(h).is_some() {
                vacate(world, &t.id, "è morto");
                continue;
            }
            if t.loses_when.is_some() {
                if keeps(world, h, t) {
                    world.resource_mut::<Titles>().failing.remove(&t.id);
                } else {
                    let since = *world.resource_mut::<Titles>().failing.entry(t.id.clone()).or_insert(tick);
                    if tick.saturating_sub(since) >= t.grace {
                        vacate(world, &t.id, "condizione di perdita");
                        continue;
                    }
                }
            }
            let start = world.resource::<Titles>().since.get(&t.id).copied().unwrap_or(0);
            if t.term > 0 && tick.saturating_sub(start) >= t.term && t.mode != TitleMode::Seat {
                let l = if t.mode == TitleMode::Election { luck } else { 0.0 };
                let best = candidates(world, t, l).first().map(|c| c.2);
                match best {
                    Some(b) if b != h => assign(world, &t.id, b, mode_name(t.mode)),
                    _ => {
                        world.resource_mut::<Titles>().since.insert(t.id.clone(), tick);
                        let name = crate::effects::name_of(world, h);
                        let id = world.get::<SimId>(h).copied();
                        world.resource_mut::<EventLog>().push(
                            tick,
                            EventBuilder::new(kind::SUCCESSION, format!("{name} è riconfermato {}", t.name)).actor(id).news(0.5).tags(["politics", t.id.as_str()]),
                        );
                    }
                }
            }
            continue;
        }
        if t.mode == TitleMode::Seat {
            continue;
        }
        let since = world.resource::<Titles>().vacant_since.get(&t.id).copied().unwrap_or(0);
        if tick.saturating_sub(since) < t.vacancy_ticks {
            continue;
        }
        let l = if t.mode == TitleMode::Election { luck } else { 0.0 };
        if let Some(&(_, _, e)) = candidates(world, t, l).first() {
            assign(world, &t.id, e, mode_name(t.mode));
        }
    }
}

/// Stat of an entity plus half the stat of its faction mates within `radius` (its supporters).
fn strength(world: &mut World, e: Entity, stat: &str, radius: i32) -> f32 {
    let own = world.get::<Stats>(e).map_or(0.0, |s| s.get(stat));
    if radius <= 0 {
        return own;
    }
    let (Some(pos), Some(f)) = (world.get::<Position>(e).copied(), world.get::<FactionMember>(e).map(|m| m.faction.clone())) else {
        return own;
    };
    let mut allies = 0.0;
    for o in crate::sorted_entities::<Pawn>(world) {
        if o == e || world.get::<Dead>(o).is_some() {
            continue;
        }
        if world.get::<FactionMember>(o).is_some_and(|m| m.faction == f) && world.get::<Position>(o).is_some_and(|p| p.within(&pos, radius)) {
            allies += world.get::<Stats>(o).map_or(0.0, |s| s.get(stat)) * 0.5;
        }
    }
    own + allies
}

/// `subject` tries to take `title`: if vacant it claims it (when eligible), otherwise it is a contest of
/// `stat` (plus supporters and luck) against the holder. Returns whether the subject holds it afterwards.
pub fn challenge(world: &mut World, subject: Entity, title: &str, stat: &str, allies_radius: i32) -> bool {
    let Some(t) = world.resource::<Content>().titles.get(title).cloned() else { return false };
    let tick = world.resource::<SimClock>().tick;
    let name = crate::effects::name_of(world, subject);
    let sid = world.get::<SimId>(subject).copied();
    if !eligible(world, subject, &t) {
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new(kind::ROLE_CHALLENGE, format!("{name} vorrebbe diventare {}, ma non ha i requisiti", t.name)).actor(sid).news(0.2),
        );
        return false;
    }
    let Some(h) = holder_entity(world, title) else {
        assign(world, title, subject, "si prende il posto vacante");
        return true;
    };
    if h == subject {
        return true;
    }
    let luck = world.resource::<Params>().f("titles.challenge_luck");
    let (a, b) = (strength(world, subject, stat, allies_radius), strength(world, h, stat, allies_radius));
    let (ra, rb) = {
        let mut rng = world.resource_mut::<SimRng>();
        (rng.next_f32(), rng.next_f32())
    };
    let (sa, sb) = (a.max(1.0) * (1.0 - luck / 2.0 + luck * ra), b.max(1.0) * (1.0 - luck / 2.0 + luck * rb));
    let hname = crate::effects::name_of(world, h);
    let hid = world.get::<SimId>(h).copied();
    let what = if t.mode == TitleMode::Coup { "un colpo di stato" } else { "una sfida" };
    if sa > sb {
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new(kind::ROLE_CHALLENGE, format!("{name} vince {what} contro {hname}")).actor(sid).target(hid).news(0.8).tags(["politics", title]),
        );
        assign(world, title, subject, &format!("ha battuto {hname}"));
        true
    } else {
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new(kind::ROLE_CHALLENGE, format!("{name} tenta {what} contro {hname} ({}) e perde", t.name)).actor(sid).target(hid).news(0.6).tags(["politics", title]),
        );
        false
    }
}

/// The player's champion goes for a role: a vacant one is claimed, a `Challenge` or `Coup` role is contested
/// with its holder; the other modes cannot be forced.
pub fn player_challenge(world: &mut World, player: &str, title: &str) -> Result<String, String> {
    let p = world.resource::<Players>().players.get(player).cloned().ok_or_else(|| format!("giocatore '{player}' inesistente"))?;
    let e = p.leader.and_then(|id| world.resource::<IdIndex>().get(id)).ok_or("nessun campione")?;
    let t = world.resource::<Content>().titles.get(title).cloned().ok_or_else(|| format!("ruolo '{title}' inesistente"))?;
    if !eligible(world, e, &t) {
        return Err(format!("il campione non ha i requisiti per essere {}", t.name));
    }
    match holder_entity(world, title) {
        Some(h) if h == e => Err(format!("il campione è già {}", t.name)),
        None => {
            assign(world, title, e, "si prende il posto vacante");
            Ok(format!("il campione è ora {}", t.name))
        }
        Some(_) if matches!(t.mode, TitleMode::Challenge | TitleMode::Coup) => {
            let stat = if t.challenge_stat.is_empty() { world.resource::<Content>().bindings.strength.clone() } else { t.challenge_stat.clone() };
            if challenge(world, e, title, &stat, t.challenge_allies) {
                Ok(format!("vittoria! il campione è ora {}", t.name))
            } else {
                Err("sfida persa".into())
            }
        }
        Some(_) => Err(format!("{} non si conquista con una sfida ({})", t.name, mode_name(t.mode))),
    }
}

/// What all living pawns believe of `e`, on average (−100…100).
pub fn public_opinion(world: &mut World, e: Entity) -> f32 {
    let Some(id) = world.get::<SimId>(e).copied() else { return 0.0 };
    let key = crate::beliefs::pawn_key(id);
    let pawns = crate::sorted_entities::<Pawn>(world);
    let n = pawns.len().max(1) as f32;
    pawns.into_iter().filter(|p| world.get::<Dead>(*p).is_none()).map(|p| crate::beliefs::opinion(world, p, &key)).sum::<f32>() / n
}
