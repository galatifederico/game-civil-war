//! Utility AI: every pawn scores the actions available to it (from data) with response curves, keeps
//! momentum on the current action to avoid oscillation, and turns the winner into a job.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{ActionDef, ActionKind, Content, Input};
use crate::crime::{Detained, Wanted};
use crate::effects::{eval_condition, EffectCtx};
use crate::extensions::Extensions;
use crate::factions::{Dissent, FactionMember};
use crate::inventory::Inventory;
use crate::jobs::{best_board_job, release_task, start_job, BoardJob, JobTarget, Task};
use crate::map::Position;
use crate::params::Params;
use crate::press::Notebook;
use crate::rng::SimRng;
use crate::stats::{Dead, Needs, Stats, Tags, Virtual, Wallet};
use crate::status::StatusEffects;
use crate::time::SimClock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoreEntry {
    pub action: String,
    pub score: f32,
    /// Per-consideration values after the curve.
    pub factors: Vec<(String, f32)>,
    pub momentum: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<JobTarget>,
}

#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Brain {
    pub actions: Vec<String>,
    pub current: Option<String>,
    pub momentum: f32,
    /// Action → tick until which it cannot be chosen.
    pub cooldowns: BTreeMap<String, u64>,
    pub think_acc: f32,
    /// Last evaluation, for the admin inspector.
    pub last: Vec<ScoreEntry>,
    pub last_tick: u64,
}

struct Candidate {
    action: ActionDef,
    score: f32,
    target: JobTarget,
    board: Option<BoardJob>,
}

fn input_value(world: &mut World, e: Entity, input: &Input, target: Option<&JobTarget>, board_score: Option<f32>) -> f32 {
    let content = world.resource::<Content>();
    match input {
        Input::Constant(c) => *c,
        Input::Need(n) => world.get::<Needs>(e).map_or(1.0, |x| x.get(n)),
        Input::Stat { stat, max } => world.get::<Stats>(e).map_or(0.0, |s| s.get(stat) / max.max(0.0001)),
        Input::StatusSeverity { status, max } => {
            world.get::<StatusEffects>(e).map_or(0.0, |s| s.severity(status) / max.max(0.0001))
        }
        Input::HasStatus(s) => world.get::<StatusEffects>(e).is_some_and(|x| x.has(s)) as u8 as f32,
        Input::HasTag(t) => world.get::<Tags>(e).is_some_and(|x| x.has(t)) as u8 as f32,
        Input::Money { max } => world.get::<Wallet>(e).map_or(0.0, |w| (w.0 / max.max(0.0001)) as f32),
        Input::Wanted { max } => world.get::<Wanted>(e).map_or(0.0, |w| w.level / max.max(0.0001)),
        Input::Dissent => world.get::<Dissent>(e).map_or(0.0, |d| d.0 / 100.0),
        Input::ItemCount { tag, max } => {
            world.get::<Inventory>(e).map_or(0.0, |i| i.count_tag(content, tag) as f32 / (*max).max(1) as f32)
        }
        Input::TargetExists => target.is_some_and(|t| *t != JobTarget::None) as u8 as f32,
        Input::TargetDistance { max } => {
            let me = world.get::<Position>(e).copied();
            match (me, target.and_then(|t| t.position(world))) {
                (Some(a), Some(b)) => {
                    let hop = world.resource::<Params>().get("move.map_hop_cost", 25.0) as i32;
                    world.resource::<crate::map::WorldMap>().travel_cost(&a, &b, hop) as f32 / max.max(0.0001)
                }
                _ => 1.0,
            }
        }
        Input::JobsAvailable { max } => board_score.map_or(0.0, |_| 1.0 / max.max(1.0)).max(board_score.unwrap_or(0.0)),
        Input::TimeOfDay => {
            let tpd = world.resource::<Params>().get("time.ticks_per_day", 24.0) as u64;
            world.resource::<SimClock>().hour_of_day(tpd) as f32 / tpd.max(1) as f32
        }
        Input::Random => world.resource_mut::<SimRng>().next_f32(),
        Input::Param { key, max } => world.resource::<Params>().f(key) / max.max(0.0001),
        Input::Scoops { max } => world.get::<Notebook>(e).map_or(0.0, |n| n.scoops.len() as f32 / max.max(1.0)),
        Input::Condition(c) => {
            let ctx = EffectCtx::new(Some(e), target.and_then(|t| t.entity(world)), "ai");
            eval_condition(world, &ctx, c) as u8 as f32
        }
        Input::Custom(id) => {
            let f = world.resource::<Extensions>().inputs.get(id).cloned();
            f.map_or(0.0, |f| f(world, e))
        }
    }
}

fn input_label(i: &Input) -> String {
    match i {
        Input::Need(n) => format!("need:{n}"),
        Input::Stat { stat, .. } => format!("stat:{stat}"),
        Input::StatusSeverity { status, .. } | Input::HasStatus(status) => format!("status:{status}"),
        Input::HasTag(t) => format!("tag:{t}"),
        Input::ItemCount { tag, .. } => format!("items:{tag}"),
        Input::Custom(c) => format!("custom:{c}"),
        other => format!("{other:?}").split(['(', ' ', '{']).next().unwrap_or("?").to_string(),
    }
}

fn score_action(world: &mut World, e: Entity, a: &ActionDef, tick: u64) -> Option<(Candidate, ScoreEntry)> {
    if world.get::<Brain>(e).and_then(|b| b.cooldowns.get(&a.id)).is_some_and(|until| *until > tick) {
        return None;
    }
    let ctx = EffectCtx::new(Some(e), None, "ai");
    if !eval_condition(world, &ctx, &a.requires) {
        return None;
    }
    // Cheap early exit: considerations that do not depend on the target are evaluated first.
    let targetless = |i: &Input| !matches!(i, Input::TargetExists | Input::TargetDistance { .. } | Input::Condition(_) | Input::JobsAvailable { .. } | Input::Random);
    for c in a.considerations.iter().filter(|c| targetless(&c.input)) {
        if c.curve.eval(input_value(world, e, &c.input, None, None)) <= 0.0 {
            return None;
        }
    }
    let (target, board) = match &a.kind {
        ActionKind::Job { job, target } => {
            let content = world.resource::<Content>().clone();
            let def = content.jobs.get(job)?;
            if !crate::jobs::can_do(world, e, def, false) {
                return None;
            }
            (crate::targeting::resolve(world, e, target)?, None)
        }
        ActionKind::Ability { ability, target } => {
            if !crate::abilities::ready(world, e, ability) {
                return None;
            }
            (crate::targeting::resolve(world, e, target)?, None)
        }
        ActionKind::Work => {
            let (_, j) = best_board_job(world, e)?;
            (j.target, Some(j))
        }
        ActionKind::Idle => (JobTarget::None, None),
    };
    let board_score = board.as_ref().map(|_| 1.0);
    let mut factors = Vec::new();
    let mut score = a.weight.max(0.0);
    let n = a.considerations.len().max(1) as f32;
    let mod_factor = 1.0 - 1.0 / n;
    for c in &a.considerations {
        let x = input_value(world, e, &c.input, Some(&target), board_score);
        let y = c.curve.eval(x);
        // Compensation so that many considerations do not drag the score to zero (Dave Mark).
        let comp = y + (1.0 - y) * mod_factor * y;
        factors.push((input_label(&c.input), y));
        score *= comp;
        if score <= 0.0 {
            break;
        }
    }
    let (current, momentum) = world.get::<Brain>(e).map_or((None, 0.0), |b| (b.current.clone(), b.momentum));
    let m = if current.as_deref() == Some(&a.id) && score > 0.0 { momentum } else { 0.0 };
    let total = score + m;
    let entry = ScoreEntry { action: a.id.clone(), score: total, factors, momentum: m, target: Some(target) };
    Some((Candidate { action: a.clone(), score: total, target, board }, entry))
}

/// Pawns in decision order: higher ranks first (first pick of board jobs), then by id.
fn decision_order(world: &mut World) -> Vec<Entity> {
    let content = world.resource::<Content>().clone();
    let mut v: Vec<(i64, crate::ids::SimId, Entity)> = Vec::new();
    let mut q = world.query_filtered::<(Entity, &crate::ids::SimId, Option<&FactionMember>), (With<Brain>, Without<Dead>, Without<Virtual>)>();
    for (e, id, m) in q.iter(world) {
        let lvl = m.and_then(|m| content.rank(&m.faction, &m.rank)).map_or(0, |r| r.level as i64);
        v.push((-lvl, *id, e));
    }
    v.sort();
    v.into_iter().map(|(_, _, e)| e).collect()
}

pub fn think(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let (interval, momentum0, decay) = {
        let p = world.resource::<Params>();
        (p.f("ai.think_interval").max(0.1), p.f("ai.momentum"), p.f("ai.momentum_decay"))
    };
    let content = world.resource::<Content>().clone();
    crate::targeting::rebuild_index(world);
    for e in decision_order(world) {
        if world.get::<Detained>(e).is_some() || world.get::<crate::player::KnockedOut>(e).is_some() {
            continue;
        }
        // Champions only look after their own needs, and only while the player has not given an order.
        let controlled = world.get::<crate::player::Controlled>(e).is_some();
        if controlled && world.get::<Task>(e).is_some_and(|t| t.job.is_some() && t.action.is_none()) {
            continue;
        }
        if world.get::<crate::player::Follow>(e).is_some_and(|f| f.strict) {
            continue;
        }
        crate::squads::apply_squad_order(world, e);
        let forced = world.get::<Task>(e).is_some_and(|t| t.forced && t.job.is_some());
        if forced {
            continue;
        }
        let speed = world.get::<StatusEffects>(e).map_or(1.0, |s| s.ai_speed(&content));
        {
            let mut b = world.get_mut::<Brain>(e).unwrap();
            b.momentum = (b.momentum - decay).max(0.0);
            b.think_acc = (b.think_acc + speed).min(interval * 4.0 + 1.0);
            if b.think_acc < interval {
                continue;
            }
            b.think_acc -= interval;
        }
        let mut action_ids = world.get::<Brain>(e).unwrap().actions.clone();
        if controlled {
            use crate::content::Selector;
            action_ids.retain(|a| {
                content.actions.get(a).is_some_and(|d| {
                    let wanders = matches!(&d.kind, ActionKind::Job { target: Selector::Zone(_) | Selector::Random(_) | Selector::OwnFactionZone(_), .. });
                    !wanders && d.considerations.iter().any(|c| matches!(c.input, Input::Need(_)))
                })
            });
        }
        let mut best: Option<Candidate> = None;
        let mut entries = Vec::new();
        for id in &action_ids {
            let Some(a) = content.actions.get(id) else { continue };
            if let Some((cand, entry)) = score_action(world, e, a, tick) {
                entries.push(entry);
                if cand.score > 0.0 && best.as_ref().is_none_or(|b| cand.score > b.score) {
                    best = Some(cand);
                }
            }
        }
        entries.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.action.cmp(&b.action)));
        entries.truncate(10);
        let (current, has_job) = {
            let b = world.get::<Brain>(e).unwrap();
            (b.current.clone(), world.get::<Task>(e).is_some_and(|t| t.job.is_some()))
        };
        {
            let mut b = world.get_mut::<Brain>(e).unwrap();
            b.last = entries;
            b.last_tick = tick;
        }
        let Some(best) = best else {
            if !has_job {
                release_task(world, e);
                world.get_mut::<Brain>(e).unwrap().current = None;
            }
            continue;
        };
        if current.as_deref() == Some(&best.action.id) && has_job {
            continue; // keep going: momentum won
        }
        // Same board job chosen again under another action label: keep the progress.
        let active_board = world.get::<Task>(e).and_then(|t| t.job.as_ref().and_then(|j| j.board_id));
        if best.board.as_ref().is_some_and(|b| Some(b.id) == active_board && b.id != 0) {
            continue;
        }
        release_task(world, e);
        {
            let mut b = world.get_mut::<Brain>(e).unwrap();
            b.current = Some(best.action.id.clone());
            b.momentum = momentum0;
        }
        let label = if best.action.label.is_empty() { best.action.name.clone() } else { best.action.label.clone() };
        {
            let mut t = world.get_mut::<Task>(e).unwrap();
            t.action = Some(best.action.id.clone());
            t.label = label;
        }
        match &best.action.kind {
            ActionKind::Job { job, .. } => start_job(world, e, job, best.target, None, None),
            ActionKind::Work => {
                if let Some(j) = best.board {
                    start_job(world, e, &j.job, j.target, Some(j.id), j.payload);
                }
            }
            ActionKind::Ability { ability, .. } => {
                start_job(world, e, "", best.target, None, None);
                if let Some(mut t) = world.get_mut::<Task>(e)
                    && let Some(j) = t.job.as_mut() {
                        j.ability = Some(ability.clone());
                        j.required = 0.0;
                    }
            }
            ActionKind::Idle => {}
        }
    }
}
