//! Extension points for world plugins: custom effects, conditions, AI inputs and job handlers,
//! registered by name and referenced from data (`Custom(id: ...)`, `handler: "..."`).

use std::collections::BTreeMap;
use std::sync::Arc;

use bevy_ecs::prelude::*;

use crate::effects::EffectCtx;
use crate::jobs::{JobCtx, JobResult};

pub type EffectFn = Arc<dyn Fn(&mut World, &EffectCtx, &serde_json::Value) + Send + Sync>;
pub type ConditionFn = Arc<dyn Fn(&mut World, &EffectCtx, &serde_json::Value) -> bool + Send + Sync>;
pub type InputFn = Arc<dyn Fn(&mut World, Entity) -> f32 + Send + Sync>;
pub type JobHandlerFn = Arc<dyn Fn(&mut World, &JobCtx) -> JobResult + Send + Sync>;

#[derive(Resource, Default, Clone)]
pub struct Extensions {
    pub effects: BTreeMap<String, EffectFn>,
    pub conditions: BTreeMap<String, ConditionFn>,
    pub inputs: BTreeMap<String, InputFn>,
    pub job_handlers: BTreeMap<String, JobHandlerFn>,
}

impl Extensions {
    pub fn names(&self) -> serde_json::Value {
        serde_json::json!({
            "effects": self.effects.keys().collect::<Vec<_>>(),
            "conditions": self.conditions.keys().collect::<Vec<_>>(),
            "inputs": self.inputs.keys().collect::<Vec<_>>(),
            "job_handlers": self.job_handlers.keys().collect::<Vec<_>>(),
        })
    }
}
