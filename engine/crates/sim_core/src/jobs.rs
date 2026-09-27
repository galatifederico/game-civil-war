//! JobQueue framework: a global board (jobs optionally restricted to a faction), local per-entity
//! queues, the WorkPriorityMatrix, and job execution (walk to the target, work, complete).

use std::collections::{BTreeMap, VecDeque};

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{Content, JobDef};
use crate::effects::{apply_effects, eval_condition, EffectCtx};
use crate::events::{kind, EventBuilder, EventLog};
use crate::extensions::Extensions;
use crate::factions::FactionMember;
use crate::ids::{IdIndex, SimId};
use crate::map::Position;
use crate::stats::{Dead, Stats, Tags};
use crate::time::SimClock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum JobTarget {
    #[default]
    None,
    Entity(SimId),
    Cell(Position),
}

impl JobTarget {
    pub fn of_entity(world: &World, e: Entity) -> Self {
        world.get::<SimId>(e).map_or(JobTarget::None, |id| JobTarget::Entity(*id))
    }

    pub fn entity(&self, world: &World) -> Option<Entity> {
        match self {
            JobTarget::Entity(id) => world.resource::<IdIndex>().get(*id),
            _ => None,
        }
    }

    pub fn position(&self, world: &World) -> Option<Position> {
        match self {
            JobTarget::None => None,
            JobTarget::Cell(p) => Some(*p),
            JobTarget::Entity(_) => self.entity(world).and_then(|e| world.get::<Position>(e).copied()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardJob {
    pub id: u64,
    pub job: String,
    /// Only members of this faction may take it (None = anyone).
    pub faction: Option<String>,
    pub target: JobTarget,
    pub priority: i32,
    pub reserved_by: Option<SimId>,
    pub created: u64,
    pub posted_by: Option<SimId>,
    /// Production jobs: building and recipe index.
    pub recipe: Option<(SimId, usize)>,
}

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobBoard {
    pub jobs: BTreeMap<u64, BoardJob>,
    next: u64,
}

impl JobBoard {
    pub fn post(&mut self, mut job: BoardJob) -> u64 {
        self.next += 1;
        job.id = self.next;
        self.jobs.insert(job.id, job);
        self.next
    }
}

/// Local queue of jobs assigned to one entity (orders from the player, squad leader…).
#[derive(Component, Debug, Clone, Default)]
pub struct PersonalQueue(pub VecDeque<BoardJob>);

/// WorkPriorityMatrix row: work type → 1 (highest) … 4 (lowest); 0 disables.
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkPriorities {
    pub defaults: BTreeMap<String, u8>,
    pub overrides: BTreeMap<String, u8>,
}

impl WorkPriorities {
    pub fn get(&self, work_type: &str) -> u8 {
        self.overrides.get(work_type).or_else(|| self.defaults.get(work_type)).copied().unwrap_or(0)
    }

    pub fn matrix(&self) -> BTreeMap<String, u8> {
        let mut m = self.defaults.clone();
        m.extend(self.overrides.clone());
        m
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActiveJob {
    pub job: String,
    pub board_id: Option<u64>,
    pub target: JobTarget,
    pub progress: f32,
    pub required: f32,
    pub started: u64,
    /// Set when the task is an ability use rather than a job.
    pub ability: Option<String>,
    pub recipe: Option<(SimId, usize)>,
}

/// What a pawn is doing right now.
#[derive(Component, Debug, Clone, Default)]
pub struct Task {
    pub action: Option<String>,
    pub label: String,
    pub job: Option<ActiveJob>,
    /// Imposed by a squad order or a command: the utility AI does not override it.
    pub forced: bool,
}

pub struct JobCtx {
    pub actor: Entity,
    pub target: Option<Entity>,
    pub target_pos: Option<Position>,
    pub def: JobDef,
    pub active: ActiveJob,
}

impl JobCtx {
    pub fn param_str(&self, key: &str) -> Option<&str> {
        self.def.params.get(key).and_then(|v| v.as_str())
    }

    pub fn param_f(&self, key: &str, default: f64) -> f64 {
        self.def.params.get(key).and_then(|v| v.as_f64()).unwrap_or(default)
    }
}

#[derive(Debug, Clone, Default)]
pub struct JobResult {
    pub ok: bool,
    pub message: Option<String>,
}

impl JobResult {
    pub fn ok() -> Self {
        Self { ok: true, message: None }
    }
    pub fn ok_msg(m: impl Into<String>) -> Self {
        Self { ok: true, message: Some(m.into()) }
    }
    pub fn fail(m: impl Into<String>) -> Self {
        Self { ok: false, message: Some(m.into()) }
    }
}

pub fn post_job(world: &mut World, job: &str, faction: Option<String>, target: JobTarget, priority: i32, by: Option<Entity>) -> u64 {
    let tick = world.resource::<SimClock>().tick;
    let posted_by = by.and_then(|e| world.get::<SimId>(e).copied());
    world.resource_mut::<JobBoard>().post(BoardJob {
        id: 0,
        job: job.to_string(),
        faction,
        target,
        priority,
        reserved_by: None,
        created: tick,
        posted_by,
        recipe: None,
    })
}

/// Can this pawn do this job at all (tags, rank, requirements, matrix)?
pub fn can_do(world: &mut World, e: Entity, def: &JobDef, check_matrix: bool) -> bool {
    if let Some(tags) = world.get::<Tags>(e) {
        if !def.required_tags.iter().all(|t| tags.has(t)) {
            return false;
        }
    }
    if def.min_rank > 0 {
        let lvl = world
            .get::<FactionMember>(e)
            .and_then(|m| world.resource::<Content>().rank(&m.faction, &m.rank).map(|r| r.level))
            .unwrap_or(0);
        if lvl < def.min_rank {
            return false;
        }
    }
    if check_matrix && !def.work_type.is_empty() {
        let p = world.get::<WorkPriorities>(e).map_or(0, |w| w.get(&def.work_type));
        if p == 0 {
            return false;
        }
    }
    let ctx = EffectCtx::new(Some(e), None, "job-check");
    eval_condition(world, &ctx, &def.requires)
}

fn recipe_work_type(world: &World, j: &BoardJob) -> Option<String> {
    let (bid, ri) = j.recipe?;
    let b = world.resource::<IdIndex>().get(bid)?;
    let def = world.get::<crate::buildings::Building>(b)?.def.clone();
    let r = world.resource::<Content>().buildings.get(&def)?.recipes.get(ri)?;
    (!r.work_type.is_empty()).then(|| r.work_type.clone())
}

/// Picks the best job for a pawn from its personal queue or the board. Returns (score, job).
pub fn best_board_job(world: &mut World, e: Entity) -> Option<(f32, BoardJob)> {
    if let Some(j) = world.get::<PersonalQueue>(e).and_then(|q| q.0.front().cloned()) {
        return Some((1.0, j));
    }
    let content = world.resource::<Content>().clone();
    let my_factions = crate::infiltration::factions_seen_as_member(world, e);
    let rank_mult = world
        .get::<FactionMember>(e)
        .and_then(|m| content.rank(&m.faction, &m.rank).map(|r| r.job_priority))
        .unwrap_or(1.0);
    let pos = world.get::<Position>(e).copied();
    let jobs: Vec<BoardJob> = world.resource::<JobBoard>().jobs.values().filter(|j| j.reserved_by.is_none()).cloned().collect();
    let mut best: Option<(f32, BoardJob)> = None;
    for j in jobs {
        if let Some(f) = &j.faction {
            if !my_factions.contains(f) {
                continue;
            }
        }
        let Some(def) = content.jobs.get(&j.job) else { continue };
        // Production jobs use the work type of their recipe (brewing, baking, forging…).
        let mut def = def.clone();
        if let Some(wt) = recipe_work_type(world, &j) {
            def.work_type = wt;
        }
        if !can_do(world, e, &def, true) {
            continue;
        }
        let prio = world.get::<WorkPriorities>(e).map_or(4, |w| w.get(&def.work_type)).max(1);
        let dist = match (pos, j.target.position(world)) {
            (Some(a), Some(b)) => a.cost(&b) as f32,
            _ => 0.0,
        };
        let score = (5 - prio as i32) as f32 / 4.0 * (1.0 + j.priority as f32 * 0.1) * rank_mult / (1.0 + dist / 20.0);
        if best.as_ref().is_none_or(|(s, _)| score > *s) {
            best = Some((score.clamp(0.0, 1.0), j));
        }
    }
    best
}

/// Starts a job for a pawn (reserving board jobs).
pub fn start_job(world: &mut World, e: Entity, job: &str, target: JobTarget, board_id: Option<u64>, recipe: Option<(SimId, usize)>) {
    let tick = world.resource::<SimClock>().tick;
    let required = world.resource::<Content>().jobs.get(job).map_or(1.0, |d| d.duration.max(0.0));
    let me = world.get::<SimId>(e).copied();
    if let Some(id) = board_id {
        if let Some(j) = world.resource_mut::<JobBoard>().jobs.get_mut(&id) {
            j.reserved_by = me;
        }
    }
    if let Some(mut t) = world.get_mut::<Task>(e) {
        t.job = Some(ActiveJob { job: job.to_string(), board_id, target, progress: 0.0, required, started: tick, ability: None, recipe });
    }
}

/// Cancels the current job, freeing its board reservation.
pub fn release_task(world: &mut World, e: Entity) {
    let Some(mut t) = world.get_mut::<Task>(e) else { return };
    let job = t.job.take();
    t.action = None;
    t.forced = false;
    t.label.clear();
    if let Some(id) = job.and_then(|j| j.board_id) {
        if let Some(j) = world.resource_mut::<JobBoard>().jobs.get_mut(&id) {
            j.reserved_by = None;
        }
    }
}

fn work_rate(world: &World, e: Entity, def: &JobDef) -> f32 {
    let skill = def.skill.as_ref().map_or(1.0, |s| {
        let v = world.get::<Stats>(e).map_or(10.0, |st| st.get(s));
        (v / 10.0).max(0.2)
    });
    let manip = world.get::<crate::anatomy::Body>(e).map_or(1.0, |b| b.capacity("manipulation").max(0.1));
    skill * manip
}

/// Executes the active job of every pawn: move into range, then work until done.
pub fn run_jobs(world: &mut World) {
    let pawns = crate::sorted_entities::<Task>(world);
    for e in pawns {
        if world.get::<Dead>(e).is_some() || world.get::<crate::crime::Detained>(e).is_some() {
            continue;
        }
        let Some(active) = world.get::<Task>(e).and_then(|t| t.job.clone()) else { continue };
        let content = world.resource::<Content>();
        let (range, def) = match &active.ability {
            Some(a) => (content.abilities.get(a).map_or(1, |d| d.range), None),
            None => match content.jobs.get(&active.job) {
                Some(d) => (d.range, Some(d.clone())),
                None => {
                    release_task(world, e);
                    continue;
                }
            },
        };
        // Target gone (despawned) → cancel.
        if matches!(active.target, JobTarget::Entity(_)) && active.target.entity(world).is_none() {
            release_task(world, e);
            continue;
        }
        if let Some(goal) = active.target.position(world) {
            if world.get::<Position>(e).is_some() && !crate::movement::move_towards(world, e, goal, range) {
                continue;
            }
        }
        if let Some(ab) = &active.ability {
            crate::abilities::use_ability(world, e, ab, active.target.entity(world));
            finish(world, e, &active);
            continue;
        }
        let def = def.expect("job def");
        let rate = work_rate(world, e, &def);
        let done = {
            let mut t = world.get_mut::<Task>(e).unwrap();
            let j = t.job.as_mut().unwrap();
            j.progress += rate;
            j.progress >= j.required
        };
        if done {
            complete_job(world, e, &active, &def);
        }
    }
}

fn finish(world: &mut World, e: Entity, active: &ActiveJob) {
    let tick = world.resource::<SimClock>().tick;
    let action = world.get::<Task>(e).and_then(|t| t.action.clone());
    if let Some(id) = active.board_id {
        world.resource_mut::<JobBoard>().jobs.remove(&id);
    }
    if let Some(mut q) = world.get_mut::<PersonalQueue>(e) {
        if active.board_id.is_some() && q.0.front().is_some_and(|j| Some(j.id) == active.board_id) {
            q.0.pop_front();
        }
    }
    if let Some(mut t) = world.get_mut::<Task>(e) {
        t.job = None;
        t.forced = false;
    }
    if let Some(a) = action {
        let cd = world.resource::<Content>().actions.get(&a).map_or(0, |d| d.cooldown);
        if let Some(mut b) = world.get_mut::<crate::ai::Brain>(e) {
            b.cooldowns.insert(a, tick + cd);
            b.current = None;
            b.think_acc = f32::MAX / 4.0; // think again next tick
        }
    }
}

fn complete_job(world: &mut World, e: Entity, active: &ActiveJob, def: &JobDef) {
    let target = active.target.entity(world);
    let target_pos = active.target.position(world);
    let handler_id = if def.handler.is_empty() { "effects".to_string() } else { def.handler.clone() };
    let handler = world.resource::<Extensions>().job_handlers.get(&handler_id).cloned();
    let ctx = JobCtx { actor: e, target, target_pos, def: def.clone(), active: active.clone() };
    let result = match handler {
        Some(h) => h(world, &ctx),
        None => JobResult::fail(format!("handler '{handler_id}' non registrato")),
    };
    let tick = world.resource::<SimClock>().tick;
    let pos = world.get::<Position>(e).copied().or(target_pos);
    let actor_id = world.get::<SimId>(e).copied();
    let target_id = target.and_then(|t| world.get::<SimId>(t).copied());
    if result.ok {
        let ectx = EffectCtx::new(Some(e), target, format!("job:{}", def.id));
        apply_effects(world, &ectx, &def.effects);
        if let Some(crime) = &def.crime {
            crate::crime::commit(world, e, crime, target, pos);
        }
        crate::social::ideology_witnesses(world, e, def);
        if def.suspicious {
            crate::infiltration::suspicious_act(world, e);
        }
        if def.news > 0.0 || result.message.is_some() {
            let msg = result.message.clone().unwrap_or_else(|| {
                format!("{}: {}", crate::infiltration::apparent_name(world, e), def.name)
            });
            let faction = world.get::<FactionMember>(e).map(|m| m.faction.clone());
            world.resource_mut::<EventLog>().push(
                tick,
                EventBuilder::new(kind::JOB_DONE, msg)
                    .actor(actor_id)
                    .target(target_id)
                    .faction(faction)
                    .pos(pos)
                    .news(def.news)
                    .tags([def.id.clone(), def.work_type.clone()]),
            );
        }
    } else if let Some(m) = &result.message {
        tracing::debug!(target: "sim::jobs", "{} fallito: {m}", def.id);
    }
    finish(world, e, active);
}
