//! Players: each one has a **champion** (its leader) moved by hand, and gives orders to the members of
//! its faction, who obey more or less depending on morale, dissent and rank. Squads can follow the champion.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::Content;
use crate::events::{EventBuilder, EventLog};
use crate::factions::{Dissent, FactionMember, Leader, Players};
use crate::ids::SimId;
use crate::jobs::{release_task, start_job, ActiveJob, JobTarget, Task};
use crate::map::Position;
use crate::params::Params;
use crate::rng::SimRng;
use crate::stats::{Dead, Stats};
use crate::time::SimClock;

/// Moved only by its player: the utility AI never picks actions for it.
#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Controlled;

/// Keeps an entity close to another. `strict` = glued to it (mounts): no own AI, same cell every tick.
#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Follow {
    pub target: SimId,
    pub distance: i32,
    pub strict: bool,
}

/// What a player can ask: go somewhere, run a job on a target, use an ability, or stop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Order {
    Move { pos: Position },
    Job { job: String, #[serde(default)] target: Option<SimId> },
    Ability { ability: String, #[serde(default)] target: Option<SimId> },
    Follow { target: SimId, #[serde(default = "two")] distance: i32 },
    Stop,
}

fn two() -> i32 {
    2
}

fn entity(world: &World, id: SimId) -> Result<Entity, String> {
    world.resource::<crate::ids::IdIndex>().get(id).ok_or_else(|| format!("entità {id} inesistente"))
}

/// Chance that a member obeys its player: base ± morale − dissent, lower for high ranks.
pub fn obedience(world: &World, e: Entity) -> f32 {
    let p = world.resource::<Params>();
    let base = p.get("player.obedience_base", 0.85) as f32;
    let morale_stat = &world.resource::<Content>().bindings.morale;
    let morale = world.get::<Stats>(e).map_or(50.0, |s| s.get(morale_stat));
    let dissent = world.get::<Dissent>(e).map_or(0.0, |d| d.0);
    let rank = world
        .get::<FactionMember>(e)
        .and_then(|m| world.resource::<Content>().rank(&m.faction, &m.rank).map(|r| r.level))
        .unwrap_or(1) as f32;
    (base + (morale - 50.0) / 200.0 - dissent / 100.0 * 0.8 - (rank - 1.0) * 0.03).clamp(0.05, 0.99)
}

/// Gives an order to a pawn on behalf of a player. The champion always obeys; other members of the
/// player's faction roll against [`obedience`].
pub fn give_order(world: &mut World, player: &str, id: SimId, order: Order) -> Result<String, String> {
    let e = entity(world, id)?;
    if world.get::<Dead>(e).is_some() {
        return Err("è morto".into());
    }
    if world.get::<KnockedOut>(e).is_some() {
        return Err("è svenuto, riprova tra poco".into());
    }
    let p = world.resource::<Players>().players.get(player).cloned().ok_or_else(|| format!("giocatore '{player}' inesistente"))?;
    let champion = world.get::<Leader>(e).is_some_and(|l| l.player == player);
    let member = world.get::<FactionMember>(e).is_some_and(|m| m.faction == p.faction);
    if !champion && !member {
        return Err("non è un membro della tua fazione".into());
    }
    let name = crate::infiltration::apparent_name(world, e);
    let tick = world.resource::<SimClock>().tick;
    if !champion {
        let chance = obedience(world, e);
        if !world.resource_mut::<SimRng>().chance(chance) {
            if let Some(mut d) = world.get_mut::<Dissent>(e) {
                d.0 = (d.0 + 2.0).min(100.0);
            }
            let pos = world.get::<Position>(e).copied();
            world.resource_mut::<EventLog>().push(
                tick,
                EventBuilder::new("order_refused", format!("{name} si rifiuta di obbedire ({:.0}% di obbedienza)", chance * 100.0))
                    .target(Some(id))
                    .faction(Some(p.faction.clone()))
                    .pos(pos)
                    .news(0.2),
            );
            return Err(format!("{name} si rifiuta di obbedire"));
        }
    }
    apply_order(world, e, order).map(|m| format!("{name}: {m}"))
}

/// Applies an order unconditionally (admin/tests).
pub fn apply_order(world: &mut World, e: Entity, order: Order) -> Result<String, String> {
    release_task(world, e);
    world.entity_mut(e).remove::<Follow>();
    let label = match &order {
        Order::Move { pos } => {
            let map = world.resource::<crate::map::WorldMap>().clone();
            let pos = map.clamp(*pos);
            start_job(world, e, "", JobTarget::Cell(pos), None, None);
            format!("va in ({}, {})", pos.x, pos.y)
        }
        Order::Job { job, target } => {
            if !world.resource::<Content>().jobs.contains_key(job) {
                return Err(format!("job '{job}' inesistente"));
            }
            let t = target.map_or(JobTarget::None, JobTarget::Entity);
            start_job(world, e, job, t, None, None);
            let name = world.resource::<Content>().jobs[job].name.clone();
            name.to_lowercase()
        }
        Order::Ability { ability, target } => {
            if !crate::abilities::ready(world, e, ability) {
                return Err("abilità non disponibile".into());
            }
            let t = target.map_or(JobTarget::None, JobTarget::Entity);
            start_job(world, e, "", t, None, None);
            if let Some(mut task) = world.get_mut::<Task>(e) {
                if let Some(j) = task.job.as_mut() {
                    j.ability = Some(ability.clone());
                    j.required = 0.0;
                }
            }
            format!("usa {ability}")
        }
        Order::Follow { target, distance } => {
            world.entity_mut(e).insert(Follow { target: *target, distance: (*distance).max(1), strict: false });
            "segue".to_string()
        }
        Order::Stop => "si ferma".to_string(),
    };
    if let Some(mut t) = world.get_mut::<Task>(e) {
        t.forced = !matches!(order, Order::Stop | Order::Follow { .. });
        t.label = format!("Ordine: {label}");
    }
    Ok(label)
}

/// Pure-move task towards an entity, stopping `range` cells away.
fn move_near(world: &mut World, e: Entity, target: SimId, range: i32) {
    let tick = world.resource::<SimClock>().tick;
    if let Some(mut t) = world.get_mut::<Task>(e) {
        t.job = Some(ActiveJob {
            job: String::new(),
            board_id: None,
            target: JobTarget::Entity(target),
            progress: 0.0,
            required: range as f32,
            started: tick,
            ability: None,
            payload: None,
        });
        t.forced = true;
        t.label = "Segue".into();
    }
}

/// Followers walk after their target; strict followers (mounts) stick to its cell.
pub fn follow_system(world: &mut World) {
    for e in crate::sorted_entities::<Follow>(world) {
        let f = world.get::<Follow>(e).unwrap().clone();
        let Some(t) = world.resource::<crate::ids::IdIndex>().get(f.target) else {
            world.entity_mut(e).remove::<Follow>();
            continue;
        };
        if world.get::<Dead>(t).is_some() || world.get::<Dead>(e).is_some() {
            world.entity_mut(e).remove::<Follow>();
            continue;
        }
        let (Some(tp), Some(me)) = (world.get::<Position>(t).copied(), world.get::<Position>(e).copied()) else { continue };
        if f.strict {
            world.entity_mut(e).insert(tp);
            continue;
        }
        let busy = world.get::<Task>(e).is_some_and(|t| t.job.is_some() && t.forced);
        if !me.within(&tp, f.distance) && !busy {
            move_near(world, e, f.target, f.distance);
        }
    }
}

/// A champion never dies: it is knocked out for a while instead.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct KnockedOut {
    pub until: u64,
}

/// What happens to a champion instead of dying: out of action for `player.knockout_ticks`, loses
/// `player.knockout_money_loss` of its money (to whoever knocked it out), wakes up with patched wounds.
pub fn knock_out(world: &mut World, e: Entity, cause: &str, by: Option<Entity>) {
    let tick = world.resource::<SimClock>().tick;
    let p = world.resource::<Params>();
    let (ticks, loss) = (p.get("player.knockout_ticks", 12.0) as u64, p.get("player.knockout_money_loss", 0.2).clamp(0.0, 1.0));
    release_task(world, e);
    world.entity_mut(e).remove::<Follow>();
    let lost = world.get::<crate::stats::Wallet>(e).map_or(0.0, |w| (w.0 * loss * 100.0).round() / 100.0);
    if let Some(mut w) = world.get_mut::<crate::stats::Wallet>(e) {
        w.0 -= lost;
    }
    if let Some(mut w) = by.filter(|b| *b != e).and_then(|b| world.get_mut::<crate::stats::Wallet>(b)) {
        w.0 += lost;
    }
    if let Some(mut b) = world.get_mut::<crate::anatomy::Body>(e) {
        for part in b.parts.iter_mut().filter(|p| !p.missing) {
            part.hp = part.hp.max(part.max_hp * 0.3);
        }
    }
    world.entity_mut(e).insert(KnockedOut { until: tick + ticks });
    let name = crate::effects::name_of(world, e);
    let (id, by_id) = (world.get::<SimId>(e).copied(), by.and_then(|b| world.get::<SimId>(b).copied()));
    let pos = world.get::<Position>(e).copied();
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new("knocked_out", format!("{name} va al tappeto ({cause}): fuori gioco per {ticks} tick, perde {lost:.0}"))
            .actor(by_id)
            .target(id)
            .pos(pos)
            .news(0.7)
            .tags(["champion", "violence"]),
    );
}

/// A leader or champion turned into something else (a pig, …): it gets its race back at `until`.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Transmuted {
    pub original: String,
    pub until: u64,
}

/// Knocked-out champions wake up; transmuted leaders return to their shape.
pub fn wake_up(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    for e in crate::sorted_entities::<Transmuted>(world) {
        let Some(t) = world.get::<Transmuted>(e).cloned() else { continue };
        if t.until > tick {
            continue;
        }
        world.entity_mut(e).remove::<Transmuted>();
        crate::status::transmute(world, e, &t.original);
        world.entity_mut(e).remove::<Transmuted>();
    }
    for e in crate::sorted_entities::<KnockedOut>(world) {
        if world.get::<KnockedOut>(e).is_some_and(|k| k.until <= tick) {
            world.entity_mut(e).remove::<KnockedOut>();
            let name = crate::effects::name_of(world, e);
            let id = world.get::<SimId>(e).copied();
            world.resource_mut::<EventLog>().push(tick, EventBuilder::new("recovered", format!("{name} si rialza")).target(id).tags(["champion"]));
        }
    }
}
