//! SquadFramework: logical groups of pawns receiving macro orders that override the utility AI.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ids::SimId;
use crate::jobs::{release_task, start_job, JobTarget, Task};
use crate::map::WorldMap;
use crate::rng::SimRng;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SquadOrder {
    /// Go to a zone and stay there.
    MoveTo { zone: String },
    /// Walk around a zone using the given job (default "patrol").
    Patrol { zone: String, #[serde(default)] job: Option<String> },
    /// Run a job on a target (attack, arrest, sabotage…).
    Job { job: String, target: Option<SimId> },
    /// Stand still.
    Hold,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Squad {
    pub id: u64,
    pub name: String,
    pub faction: Option<String>,
    pub members: Vec<SimId>,
    pub leader: Option<SimId>,
    pub order: Option<SquadOrder>,
    /// Collective role label (e.g. "guardia", "commando").
    pub role: String,
}

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Squads {
    pub squads: BTreeMap<u64, Squad>,
    next: u64,
}

impl Squads {
    pub fn create(&mut self, name: String, faction: Option<String>, members: Vec<SimId>, role: String) -> u64 {
        self.next += 1;
        let leader = members.first().copied();
        self.squads.insert(self.next, Squad { id: self.next, name, faction, members, leader, order: None, role });
        self.next
    }

    pub fn of(&self, id: SimId) -> Option<&Squad> {
        self.squads.values().find(|s| s.members.contains(&id))
    }
}

pub fn remove_member(world: &mut World, e: Entity) {
    let Some(id) = world.get::<SimId>(e).copied() else { return };
    for s in world.resource_mut::<Squads>().squads.values_mut() {
        s.members.retain(|m| *m != id);
        if s.leader == Some(id) {
            s.leader = s.members.first().copied();
        }
    }
}

/// Turns the pawn's squad order into a forced task (called by the AI before thinking).
pub fn apply_squad_order(world: &mut World, e: Entity) {
    let Some(id) = world.get::<SimId>(e).copied() else { return };
    let Some(order) = world.resource::<Squads>().of(id).and_then(|s| s.order.clone()) else { return };
    let busy = world.get::<Task>(e).is_some_and(|t| t.forced && t.job.is_some());
    if busy {
        return;
    }
    let map = world.resource::<WorldMap>().clone();
    let (job, target) = match &order {
        SquadOrder::MoveTo { zone } => {
            if world.get::<crate::map::Position>(e).is_some_and(|p| map.in_zone(zone, p)) {
                return;
            }
            let Some(p) = map.random_cell(zone, &mut world.resource_mut::<SimRng>()) else { return };
            ("move".to_string(), JobTarget::Cell(p))
        }
        SquadOrder::Patrol { zone, job } => {
            let Some(p) = map.random_cell(zone, &mut world.resource_mut::<SimRng>()) else { return };
            (job.clone().unwrap_or_else(|| "patrol".into()), JobTarget::Cell(p))
        }
        SquadOrder::Job { job, target } => (job.clone(), target.map_or(JobTarget::None, JobTarget::Entity)),
        SquadOrder::Hold => {
            release_task(world, e);
            if let Some(mut t) = world.get_mut::<Task>(e) {
                t.forced = true;
                t.label = "In posizione".into();
            }
            return;
        }
    };
    if !world.resource::<crate::content::Content>().jobs.contains_key(&job) {
        return;
    }
    release_task(world, e);
    start_job(world, e, &job, target, None, None);
    if let Some(mut t) = world.get_mut::<Task>(e) {
        t.forced = true;
        t.label = format!("Ordine di squadra: {job}");
    }
}
