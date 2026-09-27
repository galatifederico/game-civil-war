//! Simulation event log: everything that happens is recorded here. The press, telemetry, the admin API
//! and the demo all read from it.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{ids::SimId, map::Position};

/// Event kinds emitted by the engine itself. Content and plugins may emit any other string.
pub mod kind {
    pub const SPAWN: &str = "spawn";
    pub const DEATH: &str = "death";
    pub const CRIME: &str = "crime";
    pub const CRIME_REPORTED: &str = "crime_reported";
    pub const SEARCH: &str = "search";
    pub const SEIZURE: &str = "seizure";
    pub const ARREST: &str = "arrest";
    pub const RELEASE: &str = "release";
    pub const BRIBE: &str = "bribe";
    pub const BRIBE_FAILED: &str = "bribe_failed";
    pub const ARTICLE: &str = "article";
    pub const PRICE_CHANGE: &str = "price_change";
    pub const PURCHASE: &str = "purchase";
    pub const PAYROLL: &str = "payroll";
    pub const UNPAID: &str = "unpaid";
    pub const STATUS_APPLIED: &str = "status_applied";
    pub const STATUS_STAGE: &str = "status_stage";
    pub const STATUS_ENDED: &str = "status_ended";
    pub const INFECTION: &str = "infection";
    pub const MUTILATION: &str = "mutilation";
    pub const WOUND: &str = "wound";
    pub const TRANSMUTATION: &str = "transmutation";
    pub const JOB_DONE: &str = "job_done";
    pub const EXPOSED: &str = "exposed";
    pub const IDENTITY_STOLEN: &str = "identity_stolen";
    pub const SHAPESHIFT: &str = "shapeshift";
    pub const SUCCESSION_OPEN: &str = "succession_open";
    pub const SUCCESSION: &str = "succession";
    pub const DEFECTION: &str = "defection";
    pub const MERGE: &str = "merge";
    pub const COLLECTION: &str = "collection_completed";
    pub const VICTORY: &str = "victory";
    pub const TRIGGER: &str = "trigger";
    pub const STRUCTURE_DAMAGE: &str = "structure_damage";
    pub const GLOBAL_MODIFIER: &str = "global_modifier";
    pub const PRODUCTION: &str = "production";
    pub const RELEASED: &str = "tether_released";
    pub const COMMAND: &str = "command";
    pub const ABILITY: &str = "ability";
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimEvent {
    pub id: u64,
    pub tick: u64,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<SimId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<SimId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<Position>,
    /// 0..1: how interesting this is for journalists. 0 = not news.
    #[serde(default)]
    pub newsworthiness: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub message: String,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub data: serde_json::Value,
}

/// Builder for [`SimEvent`]s; `id` and `tick` are filled in by [`EventLog::push`].
#[derive(Debug, Clone)]
pub struct EventBuilder(SimEvent);

impl EventBuilder {
    pub fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self(SimEvent {
            id: 0,
            tick: 0,
            kind: kind.into(),
            actor: None,
            target: None,
            faction: None,
            pos: None,
            newsworthiness: 0.0,
            tags: Vec::new(),
            message: message.into(),
            data: serde_json::Value::Null,
        })
    }
    pub fn actor(mut self, id: Option<SimId>) -> Self {
        self.0.actor = id;
        self
    }
    pub fn target(mut self, id: Option<SimId>) -> Self {
        self.0.target = id;
        self
    }
    pub fn faction(mut self, f: Option<String>) -> Self {
        self.0.faction = f;
        self
    }
    pub fn pos(mut self, p: Option<Position>) -> Self {
        self.0.pos = p;
        self
    }
    pub fn news(mut self, n: f32) -> Self {
        self.0.newsworthiness = n;
        self
    }
    pub fn tags<I: IntoIterator<Item = S>, S: Into<String>>(mut self, t: I) -> Self {
        self.0.tags.extend(t.into_iter().map(Into::into));
        self
    }
    pub fn data(mut self, d: serde_json::Value) -> Self {
        self.0.data = d;
        self
    }
}

#[derive(Resource, Debug, Default)]
pub struct EventLog {
    events: Vec<SimEvent>,
    next_id: u64,
    /// Events older than this many are dropped from memory (0 = keep everything).
    pub capacity: usize,
}

impl EventLog {
    pub fn with_capacity(capacity: usize) -> Self {
        Self { capacity, ..Default::default() }
    }

    pub fn push(&mut self, tick: u64, ev: EventBuilder) -> u64 {
        let mut e = ev.0;
        self.next_id += 1;
        e.id = self.next_id;
        e.tick = tick;
        tracing::debug!(target: "sim::event", tick, kind = %e.kind, "{}", e.message);
        self.events.push(e);
        if self.capacity > 0 && self.events.len() > self.capacity * 2 {
            let drop = self.events.len() - self.capacity;
            self.events.drain(..drop);
        }
        self.next_id
    }

    /// Events with id strictly greater than `cursor` (use the last id you saw as cursor).
    pub fn since(&self, cursor: u64) -> &[SimEvent] {
        let start = self.events.partition_point(|e| e.id <= cursor);
        &self.events[start..]
    }

    pub fn last_id(&self) -> u64 {
        self.next_id
    }

    pub fn all(&self) -> &[SimEvent] {
        &self.events
    }

    pub fn of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a SimEvent> + 'a {
        self.events.iter().filter(move |e| e.kind == kind)
    }
}
