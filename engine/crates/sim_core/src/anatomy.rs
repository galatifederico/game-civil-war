//! AnatomySystem: hierarchical, configurable body parts with HP and efficiency. Mutilations and severe
//! wounds award heroism/honor points and change morale. Destroying a vital part kills.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{BodyPlanDef, Content};
use crate::events::{kind, EventBuilder, EventLog};
use crate::ids::SimId;
use crate::map::Position;
use crate::params::Params;
use crate::rng::SimRng;
use crate::stats::{Dead, DisplayName, Immortal, Stats};
use crate::time::SimClock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartState {
    pub id: String,
    pub name: String,
    pub parent: Option<usize>,
    pub hp: f32,
    pub max_hp: f32,
    pub missing: bool,
    pub vital: bool,
    pub coverage: f32,
    pub capacities: BTreeMap<String, f32>,
}

impl PartState {
    pub fn efficiency(&self) -> f32 {
        if self.missing { 0.0 } else { (self.hp / self.max_hp).clamp(0.0, 1.0) }
    }
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Body {
    pub plan: String,
    pub parts: Vec<PartState>,
}

impl Body {
    pub fn from_plan(plan: &BodyPlanDef) -> Self {
        let parts = plan
            .parts
            .iter()
            .map(|p| PartState {
                id: p.id.clone(),
                name: p.name.clone(),
                parent: p.parent.as_ref().and_then(|par| plan.parts.iter().position(|q| &q.id == par)),
                hp: p.hp,
                max_hp: p.hp,
                missing: false,
                vital: p.vital,
                coverage: p.coverage,
                capacities: p.capacities.clone(),
            })
            .collect();
        Self { plan: plan.id.clone(), parts }
    }

    fn effective(&self, i: usize) -> f32 {
        let mut eff = self.parts[i].efficiency();
        let mut cur = self.parts[i].parent;
        while let Some(p) = cur {
            if self.parts[p].missing {
                return 0.0;
            }
            cur = self.parts[p].parent;
        }
        eff = eff.clamp(0.0, 1.0);
        eff
    }

    /// Capacity (moving, manipulation, sight…) as the weighted efficiency of contributing parts.
    pub fn capacity(&self, cap: &str) -> f32 {
        let (mut num, mut den) = (0.0, 0.0);
        for (i, p) in self.parts.iter().enumerate() {
            if let Some(w) = p.capacities.get(cap) {
                num += w * self.effective(i);
                den += w;
            }
        }
        if den == 0.0 { 1.0 } else { num / den }
    }

    pub fn capacities(&self) -> BTreeMap<String, f32> {
        let mut caps: Vec<String> = self.parts.iter().flat_map(|p| p.capacities.keys().cloned()).collect();
        caps.sort();
        caps.dedup();
        caps.into_iter().map(|c| { let v = self.capacity(&c); (c, v) }).collect()
    }

    pub fn health_ratio(&self) -> f32 {
        let max: f32 = self.parts.iter().map(|p| p.max_hp).sum();
        let hp: f32 = self.parts.iter().filter(|p| !p.missing).map(|p| p.hp).sum();
        if max == 0.0 { 1.0 } else { hp / max }
    }

    pub fn missing_parts(&self) -> Vec<&str> {
        self.parts.iter().filter(|p| p.missing).map(|p| p.name.as_str()).collect()
    }

    fn mark_missing(&mut self, i: usize) {
        self.parts[i].missing = true;
        self.parts[i].hp = 0.0;
        let children: Vec<usize> = (0..self.parts.len()).filter(|j| self.parts[*j].parent == Some(i)).collect();
        for c in children {
            self.mark_missing(c);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DamageOutcome {
    Nothing,
    Hurt { part: String },
    Wounded { part: String },
    Mutilated { part: String },
    Killed { part: String },
}

/// Deals damage to a part (chosen by coverage when `part` is None).
pub fn damage(world: &mut World, e: Entity, amount: f32, part: Option<&str>, source: Option<Entity>) -> DamageOutcome {
    if world.get::<Dead>(e).is_some() || world.get::<crate::player::KnockedOut>(e).is_some() || amount <= 0.0 {
        return DamageOutcome::Nothing;
    }
    let Some(mut body) = world.get::<Body>(e).cloned() else { return DamageOutcome::Nothing };
    let alive: Vec<usize> = (0..body.parts.len()).filter(|i| !body.parts[*i].missing).collect();
    let idx = match part {
        Some(p) => body.parts.iter().position(|q| q.id == p && !q.missing),
        None => {
            let total: f32 = alive.iter().map(|i| body.parts[*i].coverage).sum();
            let mut roll = world.resource_mut::<SimRng>().next_f32() * total;
            let mut chosen = alive.first().copied();
            for i in &alive {
                roll -= body.parts[*i].coverage;
                if roll <= 0.0 {
                    chosen = Some(*i);
                    break;
                }
            }
            chosen
        }
    };
    let Some(i) = idx else { return DamageOutcome::Nothing };
    let immortal = world.get::<Immortal>(e).is_some() || world.get::<crate::factions::Leader>(e).is_some();
    let champion = world.get::<crate::factions::Leader>(e).is_some();
    let before = body.parts[i].hp;
    body.parts[i].hp -= amount;
    let part_name = body.parts[i].name.clone();
    let severe_ratio = world.resource::<Params>().f("health.bleed_threshold");
    let outcome = if body.parts[i].hp <= 0.0 {
        if body.parts[i].vital {
            if champion {
                body.parts[i].hp = 0.0;
                DamageOutcome::Killed { part: part_name.clone() }
            } else if immortal {
                body.parts[i].hp = 1.0;
                DamageOutcome::Wounded { part: part_name.clone() }
            } else {
                body.parts[i].hp = 0.0;
                DamageOutcome::Killed { part: part_name.clone() }
            }
        } else {
            body.mark_missing(i);
            DamageOutcome::Mutilated { part: part_name.clone() }
        }
    } else if body.parts[i].hp / body.parts[i].max_hp < severe_ratio && before / body.parts[i].max_hp >= severe_ratio {
        DamageOutcome::Wounded { part: part_name.clone() }
    } else {
        DamageOutcome::Hurt { part: part_name.clone() }
    };
    world.entity_mut(e).insert(body);

    let content = world.resource::<Content>();
    let (heroism, morale) = (content.bindings.heroism.clone(), content.bindings.morale.clone());
    let (hb, mb) = (content.stat_bounds(&heroism), content.stat_bounds(&morale));
    let params = world.resource::<Params>();
    let (hero_pts, morale_delta, ev_kind, news) = match &outcome {
        DamageOutcome::Mutilated { .. } => (params.f("anatomy.mutilation_heroism").max(10.0), -10.0, kind::MUTILATION, 0.6),
        DamageOutcome::Wounded { .. } => (params.get("anatomy.wound_heroism", 3.0) as f32, -3.0, kind::WOUND, 0.2),
        _ => (0.0, 0.0, "", 0.0),
    };
    if hero_pts > 0.0 {
        if let Some(mut s) = world.get_mut::<Stats>(e) {
            s.add_base(&heroism, hero_pts, hb);
            s.add_base(&morale, morale_delta, mb);
        }
        let tick = world.resource::<SimClock>().tick;
        let (target, actor) = (world.get::<SimId>(e).copied(), source.and_then(|s| world.get::<SimId>(s).copied()));
        let pos = world.get::<Position>(e).copied();
        let name = world.get::<DisplayName>(e).map_or("?".into(), |n| n.0.clone());
        let msg = match &outcome {
            DamageOutcome::Mutilated { part } => format!("{name} perde: {part} (+{hero_pts} {heroism})"),
            DamageOutcome::Wounded { part } => format!("{name} ferito gravemente: {part}"),
            _ => unreachable!(),
        };
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new(ev_kind, msg).actor(actor).target(target).pos(pos).news(news).tags(["violence"]),
        );
    }
    if let DamageOutcome::Killed { part } = &outcome {
        crate::lifecycle::kill(world, e, &format!("{part} distrutto"), source);
    }
    outcome
}

pub fn heal(world: &mut World, e: Entity, amount: f32) {
    if let Some(mut b) = world.get_mut::<Body>(e) {
        let mut left = amount;
        for p in b.parts.iter_mut().filter(|p| !p.missing) {
            let d = (p.max_hp - p.hp).min(left);
            p.hp += d;
            left -= d;
            if left <= 0.0 {
                break;
            }
        }
    }
}

/// Natural healing.
pub fn natural_healing(params: Res<Params>, mut q: Query<&mut Body, Without<Dead>>) {
    let rate = params.f("health.natural_heal");
    for mut b in &mut q {
        if b.parts.iter().any(|p| !p.missing && p.hp < p.max_hp) {
            for p in b.parts.iter_mut().filter(|p| !p.missing) {
                p.hp = (p.hp + rate).min(p.max_hp);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::BodyPartDef;

    #[test]
    fn capacities_follow_hierarchy() {
        let part = |id: &str, parent: Option<&str>, cap: Option<(&str, f32)>| BodyPartDef {
            id: id.into(),
            name: id.into(),
            parent: parent.map(Into::into),
            hp: 10.0,
            vital: false,
            coverage: 1.0,
            capacities: cap.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            tags: vec![],
        };
        let plan = BodyPlanDef {
            id: "h".into(),
            parts: vec![part("torso", None, None), part("arm", Some("torso"), None), part("hand", Some("arm"), Some(("manipulation", 1.0))), part("leg", Some("torso"), Some(("moving", 1.0)))],
        };
        let mut b = Body::from_plan(&plan);
        assert_eq!(b.capacity("manipulation"), 1.0);
        b.mark_missing(1);
        assert_eq!(b.capacity("manipulation"), 0.0);
        assert!(b.parts[2].missing);
        b.parts[3].hp = 5.0;
        assert_eq!(b.capacity("moving"), 0.5);
    }
}
