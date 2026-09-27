//! HierarchySystem & SuccessionEngine, Merge & Defection Framework.

use bevy_ecs::prelude::*;

use crate::content::{Content, FactionRole, JobDef};
use crate::effects::{eval_condition, EffectCtx};
use crate::events::{kind, EventBuilder, EventLog};
use crate::factions::{Dissent, FactionMember, Factions, Leader, Players, Titles};
use crate::ids::SimId;
use crate::map::{Position, WorldMap};
use crate::params::Params;
use crate::stats::{Dead, Pawn};
use crate::time::SimClock;

pub fn change_faction(world: &mut World, e: Entity, faction: &str, reason: &str) {
    let content = world.resource::<Content>();
    if !content.factions.contains_key(faction) {
        return;
    }
    let rank = content.base_rank(faction).map(|r| r.id.clone()).unwrap_or_default();
    let tick = world.resource::<SimClock>().tick;
    let old = world.get::<FactionMember>(e).map(|m| m.faction.clone());
    if old.as_deref() == Some(faction) {
        return;
    }
    world.entity_mut(e).insert(FactionMember { faction: faction.to_string(), rank, joined: tick });
    if let Some(mut d) = world.get_mut::<Dissent>(e) {
        d.0 = 0.0;
    }
    crate::squads::remove_member(world, e);
    let name = crate::effects::name_of(world, e);
    let id = world.get::<SimId>(e).copied();
    let fname = world.resource::<Content>().factions.get(faction).map_or(faction.to_string(), |f| f.name.clone());
    let pos = world.get::<Position>(e).copied();
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::DEFECTION, format!("{name} passa a {fname} ({reason})"))
            .target(id)
            .faction(Some(faction.to_string()))
            .pos(pos)
            .news(0.5)
            .tags(["defection"])
            .data(serde_json::json!({ "from": old, "to": faction })),
    );
}

/// Members of a faction that forbids one of the job's ideology tags, and who see it happen, gain dissent.
pub fn ideology_witnesses(world: &mut World, actor: Entity, job: &JobDef) {
    if job.ideology_tags.is_empty() {
        return;
    }
    let Some(pos) = world.get::<Position>(actor).copied() else { return };
    let Some(actor_faction) = world.get::<FactionMember>(actor).map(|m| m.faction.clone()) else { return };
    let content = world.resource::<Content>().clone();
    let Some(fdef) = content.factions.get(&actor_faction) else { return };
    if !job.ideology_tags.iter().any(|t| fdef.forbids.contains(t)) {
        return;
    }
    let amount = world.resource::<Params>().f("social.ideology_violation_dissent");
    let range = world.resource::<Params>().get("ai.perception_range", 6.0) as i32;
    for w in crate::sorted_entities::<Pawn>(world) {
        if w == actor || world.get::<Dead>(w).is_some() {
            continue;
        }
        let same = world.get::<FactionMember>(w).is_some_and(|m| m.faction == actor_faction);
        if same && world.get::<Position>(w).is_some_and(|p| p.within(&pos, range)) {
            if let Some(mut d) = world.get_mut::<Dissent>(w) {
                d.0 = (d.0 + amount).min(100.0);
            }
        }
    }
}

/// Dissent decay and defection of the most unhappy members to the most compatible faction.
pub fn defections(world: &mut World) {
    let p = world.resource::<Params>();
    let (threshold, decay) = (p.f("social.defection_threshold"), p.f("social.dissent_decay"));
    let content = world.resource::<Content>().clone();
    let pawns = crate::sorted_entities::<Dissent>(world);
    for e in pawns {
        if world.get::<Dead>(e).is_some() || world.get::<Leader>(e).is_some() {
            continue;
        }
        let d = {
            let mut d = world.get_mut::<Dissent>(e).unwrap();
            d.0 = (d.0 - decay).max(0.0);
            d.0
        };
        if d < threshold {
            continue;
        }
        let Some(cur) = world.get::<FactionMember>(e).map(|m| m.faction.clone()) else { continue };
        // Leaders of their faction never desert; unique ranks either.
        if world.get::<FactionMember>(e).and_then(|m| content.rank(&m.faction, &m.rank)).is_some_and(|r| r.unique) {
            continue;
        }
        let factions = world.resource::<Factions>().clone();
        let cur_ideo = content.factions.get(&cur).map(|f| f.ideology.clone()).unwrap_or_default();
        let best = content
            .factions
            .values()
            .filter(|f| f.id != cur && f.role == FactionRole::Regular && factions.states.get(&f.id).is_some_and(|s| s.absorbed_into.is_none()))
            .map(|f| {
                let ideo: f32 = f.ideology.iter().map(|(k, v)| 1.0 - (v - cur_ideo.get(k).copied().unwrap_or(0.0)).abs()).sum();
                (factions.relation(&cur, &f.id) + ideo * 20.0, f.id.clone())
            })
            .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)));
        if let Some((_, to)) = best {
            change_faction(world, e, &to, "dissenso");
        }
    }
}

/// Allied factions above the merge threshold merge; a player's faction is never absorbed.
pub fn merges(world: &mut World) {
    let threshold = world.resource::<Params>().f("social.merge_threshold");
    let content = world.resource::<Content>().clone();
    let factions = world.resource::<Factions>().clone();
    let ids: Vec<&String> = factions.states.keys().filter(|f| factions.states[*f].absorbed_into.is_none()).collect();
    let mut count = std::collections::BTreeMap::<String, usize>::new();
    let mut q = world.query_filtered::<&FactionMember, Without<Dead>>();
    for m in q.iter(world) {
        *count.entry(m.faction.clone()).or_default() += 1;
    }
    for (i, a) in ids.iter().enumerate() {
        for b in &ids[i + 1..] {
            let regular = |f: &str| content.factions.get(f).is_some_and(|d| d.role == FactionRole::Regular);
            if !regular(a) || !regular(b) {
                continue;
            }
            if factions.relation(a, b) < threshold || factions.relation(b, a) < threshold {
                continue;
            }
            let (pa, pb) = (factions.states[*a].controlled_by.is_some(), factions.states[*b].controlled_by.is_some());
            let (absorber, absorbed) = match (pa, pb) {
                (true, true) => continue,
                (true, false) => (a, b),
                (false, true) => (b, a),
                _ => {
                    let (ca, cb) = (count.get(*a).copied().unwrap_or(0), count.get(*b).copied().unwrap_or(0));
                    if ca >= cb { (a, b) } else { (b, a) }
                }
            };
            merge(world, absorber, absorbed);
            return; // one merge per tick keeps things readable and deterministic
        }
    }
}

pub fn merge(world: &mut World, absorber: &str, absorbed: &str) {
    if world.resource::<Factions>().states.get(absorbed).is_some_and(|s| s.controlled_by.is_some()) {
        return; // the player can never be absorbed
    }
    let tick = world.resource::<SimClock>().tick;
    let content = world.resource::<Content>().clone();
    let rank = content.base_rank(absorber).map(|r| r.id.clone()).unwrap_or_default();
    let members: Vec<Entity> = crate::sorted_entities::<FactionMember>(world)
        .into_iter()
        .filter(|e| world.get::<FactionMember>(*e).is_some_and(|m| m.faction == absorbed))
        .collect();
    for e in &members {
        world.entity_mut(*e).insert(FactionMember { faction: absorber.to_string(), rank: rank.clone(), joined: tick });
    }
    {
        let mut f = world.resource_mut::<Factions>();
        let t = f.states.get(absorbed).map_or(0.0, |s| s.treasury);
        f.add_treasury(absorber, t);
        if let Some(s) = f.states.get_mut(absorbed) {
            s.treasury = 0.0;
            s.absorbed_into = Some(absorber.to_string());
        }
    }
    let n = |f: &str| content.factions.get(f).map_or(f.to_string(), |d| d.name.clone());
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::MERGE, format!("{} ingloba {} ({} membri)", n(absorber), n(absorbed), members.len()))
            .faction(Some(absorber.to_string()))
            .news(0.9)
            .tags(["merge", "politics"]),
    );
}

pub fn vacate_title(world: &mut World, title: &str) {
    let tick = world.resource::<SimClock>().tick;
    {
        let mut t = world.resource_mut::<Titles>();
        t.holders.insert(title.to_string(), None);
        t.vacant_since.insert(title.to_string(), tick);
    }
    let name = world.resource::<Content>().titles.get(title).map_or(title.to_string(), |t| t.name.clone());
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::SUCCESSION_OPEN, format!("Il titolo di {name} è vacante: la successione è aperta!"))
            .news(1.0)
            .tags(["succession", "politics", title]),
    );
}

/// Vacant titles are claimed by the first eligible pawn sitting on the seat (player leaders first).
pub fn succession(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let content = world.resource::<Content>().clone();
    let map = world.resource::<WorldMap>().clone();
    for t in content.titles.values() {
        if world.resource::<Titles>().holder(&t.id).is_some() {
            continue;
        }
        // Titles that were never held are vacant from the start only if listed as such.
        if !world.resource::<Titles>().holders.contains_key(&t.id) {
            continue;
        }
        let mut claimants: Vec<(bool, SimId, Entity)> = Vec::new();
        for e in crate::sorted_entities::<Pawn>(world) {
            if world.get::<Dead>(e).is_some() || world.get::<crate::crime::Detained>(e).is_some() {
                continue;
            }
            if !world.get::<Position>(e).is_some_and(|p| map.in_zone(&t.seat_zone, p)) {
                continue;
            }
            if !eval_condition(world, &EffectCtx::new(Some(e), None, "succession"), &t.claim_requires) {
                continue;
            }
            claimants.push((world.get::<Leader>(e).is_none(), *world.get::<SimId>(e).unwrap(), e));
        }
        claimants.sort();
        let Some((_, id, e)) = claimants.first().copied() else { continue };
        world.resource_mut::<Titles>().holders.insert(t.id.clone(), Some(id));
        let name = crate::effects::name_of(world, e);
        let player = world.get::<Leader>(e).map(|l| l.player.clone());
        let mut msg = format!("{name} sale sul trono: nuovo {}", t.name);
        if let Some(p) = &player {
            let pf = world.resource::<Players>().players.get(p).map(|x| x.faction.clone());
            if let Some(pf) = pf {
                if let Some(s) = world.resource_mut::<Factions>().states.get_mut(&pf) {
                    s.victory_points += t.victory_points;
                }
                msg += &format!(" (+{} punti vittoria)", t.victory_points);
            }
            if t.grants_faction_control {
                if let Some(s) = world.resource_mut::<Factions>().states.get_mut(&t.faction) {
                    s.controlled_by = Some(p.clone());
                }
                msg += &format!("; il giocatore {p} controlla ora {}", content.factions.get(&t.faction).map_or("", |f| f.name.as_str()));
            }
        } else {
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
                .faction(Some(t.faction.clone()))
                .pos(pos)
                .news(t.news.max(0.9))
                .tags(["succession", "politics", t.id.as_str()]),
        );
    }
}
