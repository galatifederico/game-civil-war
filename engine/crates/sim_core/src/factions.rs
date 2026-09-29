//! FactionSystem: membership, ranks, relations, ideology, guild treasuries, players and titles.

use std::collections::{BTreeMap, BTreeSet};

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::Content;
use crate::ids::SimId;

#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FactionMember {
    pub faction: String,
    pub rank: String,
    pub joined: u64,
}

/// A player-controlled leader.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Leader {
    pub player: String,
}

#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Dissent(pub f32);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FactionState {
    pub treasury: f64,
    pub relations: BTreeMap<String, f32>,
    /// Player controlling this faction, if any. Player factions are never absorbed.
    pub controlled_by: Option<String>,
    pub absorbed_into: Option<String>,
    /// Pathogens this faction has analyzed (medical framework).
    pub known_pathogens: BTreeSet<String>,
    pub victory_points: i64,
    pub reputation: f32,
    /// Salary overrides per rank (PayrollEngine configuration matrix).
    pub salaries: BTreeMap<String, f64>,
    pub unpaid_periods: u32,
    /// Allied factions: never hostile to each other (player factions ally instead of merging).
    #[serde(default)]
    pub allies: BTreeSet<String>,
}

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Factions {
    pub states: BTreeMap<String, FactionState>,
}

impl Factions {
    pub fn from_content(c: &Content) -> Self {
        let mut states = BTreeMap::new();
        for f in c.factions.values() {
            states.insert(
                f.id.clone(),
                FactionState { treasury: f.treasury, relations: f.relations.clone(), ..Default::default() },
            );
        }
        // Make relations bidirectional: a missing reverse entry mirrors the declared one.
        let ids: Vec<String> = states.keys().cloned().collect();
        for a in &ids {
            for b in &ids {
                if a == b {
                    continue;
                }
                let ab = states[a].relations.get(b).copied();
                let ba = states[b].relations.get(a).copied();
                if let (Some(v), None) = (ab, ba) {
                    states.get_mut(b).unwrap().relations.insert(a.clone(), v);
                }
            }
        }
        Self { states }
    }

    pub fn relation(&self, a: &str, b: &str) -> f32 {
        if a == b {
            return 100.0;
        }
        self.states.get(a).and_then(|s| s.relations.get(b)).copied().unwrap_or(0.0)
    }

    /// Changes the relation in both directions.
    pub fn modify_relation(&mut self, a: &str, b: &str, amount: f32) {
        if a == b {
            return;
        }
        for (x, y) in [(a, b), (b, a)] {
            if let Some(s) = self.states.get_mut(x) {
                let r = s.relations.entry(y.to_string()).or_insert(0.0);
                *r = (*r + amount).clamp(-100.0, 100.0);
            }
        }
    }

    pub fn treasury(&self, f: &str) -> f64 {
        self.states.get(f).map_or(0.0, |s| s.treasury)
    }

    pub fn add_treasury(&mut self, f: &str, amount: f64) {
        if let Some(s) = self.states.get_mut(f) {
            s.treasury += amount;
        }
    }

    /// Takes `amount` if available.
    pub fn spend(&mut self, f: &str, amount: f64) -> bool {
        match self.states.get_mut(f) {
            Some(s) if s.treasury + 1e-9 >= amount => {
                s.treasury -= amount;
                true
            }
            _ => false,
        }
    }

    /// Final faction after merges.
    pub fn resolve(&self, f: &str) -> String {
        let mut cur = f.to_string();
        for _ in 0..16 {
            match self.states.get(&cur).and_then(|s| s.absorbed_into.clone()) {
                Some(n) => cur = n,
                None => break,
            }
        }
        cur
    }

    pub fn hostile(&self, a: &str, b: &str) -> bool {
        self.hostile_at(a, b, -30.0)
    }

    /// Hostile when the relation is at or below `threshold` (parameter `social.hostile_threshold`).
    pub fn hostile_at(&self, a: &str, b: &str, threshold: f32) -> bool {
        a != b && !self.allied(a, b) && self.relation(a, b) <= threshold
    }

    pub fn allied(&self, a: &str, b: &str) -> bool {
        self.states.get(a).is_some_and(|s| s.allies.contains(b))
    }

    pub fn friendly(&self, a: &str, b: &str) -> bool {
        a == b || self.allied(a, b) || self.relation(a, b) >= 30.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub faction: String,
    pub leader: Option<SimId>,
}

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Players {
    pub players: BTreeMap<String, Player>,
}

impl Players {
    pub fn by_faction(&self, faction: &str) -> Option<&Player> {
        self.players.values().find(|p| p.faction == faction)
    }
}

/// Holders of unique titles (e.g. a throne), handled by the SuccessionEngine.
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Titles {
    pub holders: BTreeMap<String, Option<SimId>>,
    /// Tick at which each title became vacant.
    pub vacant_since: BTreeMap<String, u64>,
}

impl Titles {
    pub fn holder(&self, t: &str) -> Option<SimId> {
        self.holders.get(t).copied().flatten()
    }

    pub fn held_by(&self, id: SimId) -> Vec<String> {
        self.holders.iter().filter(|(_, h)| **h == Some(id)).map(|(t, _)| t.clone()).collect()
    }
}
