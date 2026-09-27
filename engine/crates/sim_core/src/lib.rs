//! # sim_core
//!
//! Headless, deterministic, data-driven simulation engine for colony-sim / management-sandbox games,
//! built on `bevy_ecs`. The engine knows no setting: races, classes, diseases, factions, items, jobs,
//! AI behaviours, buildings, triggers and win conditions are all content loaded from RON/JSON packs, and
//! world plugins add named effects, conditions, AI inputs and job handlers ([`sim::SimPlugin`]).

pub mod abilities;
pub mod ai;
pub mod anatomy;
pub mod buildings;
pub mod commands;
pub mod compendium;
pub mod content;
pub mod crime;
pub mod dungeon;
pub mod economy;
pub mod effects;
pub mod events;
pub mod extensions;
pub mod factions;
pub mod handlers;
pub mod hygiene;
pub mod ids;
pub mod infiltration;
pub mod inventory;
pub mod inventory_ops;
pub mod jobs;
pub mod lifecycle;
pub mod logistics;
pub mod map;
pub mod market;
pub mod movement;
pub mod params;
pub mod press;
pub mod rng;
pub mod sim;
pub mod snapshot;
pub mod social;
pub mod squads;
pub mod stats;
pub mod status;
pub mod targeting;
pub mod telemetry;
pub mod time;
pub mod victory;

#[cfg(feature = "server")]
pub mod server;

pub use bevy_ecs;

pub mod prelude {
    pub use crate::buildings::{Building, GlobalModifiers, Owner, Shop};
    pub use crate::commands::{CommandQueue, CommandResults, SimCommand, SpriteMapping};
    pub use crate::content::{Content, ContentError, ContentPack, Effect, Condition};
    pub use crate::crime::{Detained, Wanted};
    pub use crate::effects::Flags;
    pub use crate::events::{kind, EventBuilder, EventLog, SimEvent};
    pub use crate::factions::{FactionMember, Factions, Leader, Player, Players, Titles};
    pub use crate::ids::{IdIndex, SimId};
    pub use crate::inventory::{Inventory, Stock};
    pub use crate::jobs::JobBoard;
    pub use crate::lifecycle::SpawnOverrides;
    pub use crate::map::{Position, WorldMap};
    pub use crate::market::Market;
    pub use crate::params::Params;
    pub use crate::press::Feed;
    pub use crate::rng::SimRng;
    pub use crate::sim::{SimBuilder, SimPlugin, SimSet, Simulation};
    pub use crate::snapshot::{WorldSnapshot, ActivityState};
    pub use crate::squads::Squads;
    pub use crate::stats::{Dead, DisplayName, Needs, Pawn, Stats, TemplateId, Wallet};
    pub use crate::time::SimClock;
    pub use crate::victory::Progress;
    pub use bevy_ecs::prelude::*;
}

use bevy_ecs::prelude::*;

/// Entities having component `T`, in [`ids::SimId`] order: the canonical deterministic iteration order.
pub fn sorted_entities<T: Component>(world: &mut World) -> Vec<Entity> {
    let mut q = world.query_filtered::<(Entity, &ids::SimId), With<T>>();
    let mut v: Vec<(ids::SimId, Entity)> = q.iter(world).map(|(e, id)| (*id, e)).collect();
    v.sort_unstable_by_key(|x| x.0);
    v.into_iter().map(|x| x.1).collect()
}
