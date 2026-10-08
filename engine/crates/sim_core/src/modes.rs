//! Modes: one way of living at a time (work, conquest, search…). The player's order for the pawn wins,
//! then its faction's mode, then the pawn's own choice. A pawn kept in a mode it would not choose grows
//! dissent (`modes.dissent_per_tick`): its only way to disobey is leaving the faction.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{Content, ModeDef};
use crate::effects::{eval_condition, EffectCtx};
use crate::factions::{Dissent, FactionMember, Factions};
use crate::jobs::WorkPriorities;
use crate::stats::{Dead, Pawn, Virtual};

/// Where a pawn's current mode comes from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModeSource {
    #[default]
    Own,
    Faction,
    Player,
}

#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Mode {
    /// The mode the player ordered for this pawn.
    pub ordered: Option<String>,
    /// The mode the pawn would choose by itself.
    pub own: String,
    /// The mode it is in.
    pub current: String,
    pub source: ModeSource,
}

/// The pawn's own choice: the highest-priority mode whose condition holds, else the default mode.
fn own_choice(world: &mut World, e: Entity, content: &Content) -> String {
    let mut modes: Vec<&ModeDef> = content.modes.values().collect();
    modes.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.id.cmp(&b.id)));
    for m in modes {
        if eval_condition(world, &EffectCtx::new(Some(e), None, format!("mode:{}", m.id)), &m.when) {
            return m.id.clone();
        }
    }
    content.bindings.default_mode.clone().filter(|m| content.modes.contains_key(m)).or_else(|| content.modes.keys().next().cloned()).unwrap_or_default()
}

/// Recomputes every pawn's mode, its work priorities and the dissent of pawns kept against their will.
pub fn update(world: &mut World) {
    let content = world.resource::<Content>().clone();
    if content.modes.is_empty() {
        return;
    }
    let dissent = world.resource::<crate::params::Params>().get("modes.dissent_per_tick", 0.08) as f32;
    for e in crate::sorted_entities::<Pawn>(world) {
        if world.get::<Dead>(e).is_some() || world.get::<Virtual>(e).is_some() {
            continue;
        }
        let own = own_choice(world, e, &content);
        let mut m = world.get::<Mode>(e).cloned().unwrap_or_default();
        let faction_mode = world.get::<FactionMember>(e).and_then(|f| world.resource::<Factions>().states.get(&f.faction).and_then(|s| s.mode.clone()));
        let (current, source) = match (&m.ordered, faction_mode) {
            (Some(o), _) if content.modes.contains_key(o) => (o.clone(), ModeSource::Player),
            (_, Some(f)) if content.modes.contains_key(&f) => (f, ModeSource::Faction),
            _ => (own.clone(), ModeSource::Own),
        };
        if source != ModeSource::Own
            && current != own
            && let Some(mut d) = world.get_mut::<Dissent>(e)
        {
            d.0 += dissent;
        }
        let changed = m.current != current;
        m.own = own;
        m.current = current;
        m.source = source;
        if changed && let Some(def) = content.modes.get(&m.current) {
            if let Some(mut w) = world.get_mut::<WorkPriorities>(e) {
                w.defaults = def.work.clone();
            }
        }
        world.entity_mut(e).insert(m);
    }
}

/// Multiplier of an action's utility in the pawn's current mode.
pub fn weight(content: &Content, mode: Option<&Mode>, tags: &[String]) -> f32 {
    let Some(def) = mode.and_then(|m| content.modes.get(&m.current)) else { return 1.0 };
    let hits: Vec<f32> = tags.iter().filter_map(|t| def.weights.get(t).copied()).collect();
    if hits.is_empty() { def.default_weight } else { hits.iter().product() }
}
