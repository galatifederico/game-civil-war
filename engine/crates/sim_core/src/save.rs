//! Save and load: every component of every entity and every piece of simulation state is written to a
//! JSON file. Content is not saved (it is reloaded from the packs), only the ids of the packs used.
//! Loading a save and continuing gives exactly the same run as never having stopped.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ids::{IdIndex, SimId};

pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveGame {
    pub version: u32,
    pub packs: Vec<String>,
    pub tick: u64,
    pub next_id: u64,
    pub resources: BTreeMap<String, Value>,
    pub entities: Vec<SavedEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedEntity {
    pub id: SimId,
    pub components: BTreeMap<String, Value>,
}

macro_rules! components {
    ($($t:ty),* $(,)?) => {
        fn save_components(world: &World, e: Entity) -> BTreeMap<String, Value> {
            let mut m = BTreeMap::new();
            $(
                if let Some(c) = world.get::<$t>(e) {
                    m.insert(stringify!($t).to_string(), serde_json::to_value(c).expect("component serializes"));
                }
            )*
            m
        }

        fn load_components(world: &mut World, e: Entity, m: &BTreeMap<String, Value>) -> Result<(), String> {
            $(
                if let Some(v) = m.get(stringify!($t)) {
                    let c: $t = serde_json::from_value(v.clone()).map_err(|err| format!("{}: {err}", stringify!($t)))?;
                    world.entity_mut(e).insert(c);
                }
            )*
            Ok(())
        }
    };
}

use crate::abilities::AbilityCooldowns;
use crate::ai::Brain;
use crate::anatomy::Body;
use crate::buildings::{Building, Shop};
use crate::crime::{Detained, Wanted};
use crate::dungeon::{SpawnedBy, Tethered};
use crate::factions::{Dissent, FactionMember, Leader};
use crate::hygiene::Contaminated;
use crate::infiltration::{Cover, Disguise, StealthState};
use crate::inventory::{Inventory, Stock};
use crate::jobs::{PersonalQueue, Task, WorkPriorities};
use crate::map::Position;
use crate::movement::Movement;
use crate::player::{Controlled, Follow, KnockedOut};
use crate::press::Notebook;
use crate::snapshot::ActivityState;
use crate::stats::{Classes, Dead, DisplayName, Immortal, Needs, Pawn, Race, Stats, Tags, TemplateId, Virtual, Wallet};
use crate::status::StatusEffects;

components!(
    DisplayName, TemplateId, Race, Classes, Tags, Stats, Needs, StatusEffects, Inventory, Wallet, Dissent, Wanted,
    Detained, Brain, Task, PersonalQueue, WorkPriorities, ActivityState, Notebook, Movement, Pawn, Virtual, Position,
    Body, Immortal, Dead, FactionMember, Leader, Controlled, Follow, Disguise, Cover, StealthState, Tethered,
    SpawnedBy, AbilityCooldowns, Building, Shop, Stock, Contaminated, KnockedOut,
);

macro_rules! resources {
    ($($t:ty),* $(,)?) => {
        fn save_resources(world: &World) -> BTreeMap<String, Value> {
            let mut m = BTreeMap::new();
            $( m.insert(stringify!($t).to_string(), serde_json::to_value(world.resource::<$t>()).expect("resource serializes")); )*
            m
        }

        fn load_resources(world: &mut World, m: &BTreeMap<String, Value>) -> Result<(), String> {
            $(
                if let Some(v) = m.get(stringify!($t)) {
                    let r: $t = serde_json::from_value(v.clone()).map_err(|err| format!("{}: {err}", stringify!($t)))?;
                    world.insert_resource(r);
                }
            )*
            Ok(())
        }
    };
}

use crate::buildings::GlobalModifiers;
use crate::commands::{CommandQueue, SpriteMapping};
use crate::dungeon::TriggerState;
use crate::effects::Flags;
use crate::events::EventLog;
use crate::factions::{Factions, Players, Titles};
use crate::jobs::JobBoard;
use crate::map::{Environment, TerrainChanges};
use crate::market::Market;
use crate::params::Params;
use crate::press::{Feed, PressCursor};
use crate::rng::SimRng;
use crate::squads::Squads;
use crate::telemetry::TelemetryCursor;
use crate::time::SimClock;
use crate::victory::Progress;

resources!(
    SimRng, SimClock, Params, EventLog, CommandQueue, SpriteMapping, JobBoard, Factions, Players, Titles, Squads, Market,
    GlobalModifiers, Feed, PressCursor, TelemetryCursor, Flags, TriggerState, Progress, Environment, TerrainChanges,
);

pub fn save(world: &mut World) -> SaveGame {
    let packs = world.resource::<crate::content::Content>().packs.iter().map(|p| p.id.clone()).collect();
    let entities = world
        .resource::<IdIndex>()
        .entities()
        .into_iter()
        .map(|(id, e)| SavedEntity { id, components: save_components(world, e) })
        .collect();
    SaveGame {
        version: VERSION,
        packs,
        tick: world.resource::<SimClock>().tick,
        next_id: world.resource::<IdIndex>().next_id(),
        resources: save_resources(world),
        entities,
    }
}

/// Restores a save into a world built from the same content (without initial placements).
pub fn load(world: &mut World, save: &SaveGame) -> Result<(), String> {
    if save.version != VERSION {
        return Err(format!("versione del salvataggio {} non supportata (attesa {VERSION})", save.version));
    }
    let packs: Vec<String> = world.resource::<crate::content::Content>().packs.iter().map(|p| p.id.clone()).collect();
    if packs != save.packs {
        return Err(format!("pacchetti diversi: salvataggio {:?}, caricati {:?}", save.packs, packs));
    }
    load_resources(world, &save.resources)?;
    let mut index = IdIndex::default();
    for se in &save.entities {
        let e = world.spawn(se.id).id();
        index.insert(se.id, e);
        load_components(world, e, &se.components)?;
    }
    index.set_next(save.next_id);
    world.insert_resource(index);
    world.insert_resource(crate::targeting::TargetIndex::default());
    Ok(())
}
