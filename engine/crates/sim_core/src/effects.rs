//! Interpreter of the [`Effect`] and [`Condition`] data languages.
//!
//! Effects run in a context with a *subject* and an optional *target*. `On(scope, e)` runs `e` once per
//! scoped entity; inside it the subject is the scoped entity and the target is the previous subject
//! (so `On(Target, ...)` means "the victim, with the actor as the other party").

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{Condition, Content, Effect, Scope};
use crate::events::{EventBuilder, EventLog};
use crate::factions::{FactionMember, Factions, Titles};
use crate::ids::SimId;
use crate::inventory::Inventory;
use crate::map::{Position, WorldMap};
use crate::rng::SimRng;
use crate::stats::{Classes, Dead, DisplayName, Needs, Pawn, Race, Stats, Tags, TemplateId, Wallet};
use crate::status::StatusEffects;
use crate::time::SimClock;

#[derive(Debug, Clone)]
pub struct EffectCtx {
    pub subject: Option<Entity>,
    pub target: Option<Entity>,
    /// What caused this (for logs): "job:steal", "status:drunk", "trigger:x", "command"…
    pub origin: String,
}

impl EffectCtx {
    pub fn new(subject: Option<Entity>, target: Option<Entity>, origin: impl Into<String>) -> Self {
        Self { subject, target, origin: origin.into() }
    }
}

/// Global named numeric flags, set by effects and read by conditions (triggers, quests…).
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Flags(pub BTreeMap<String, f64>);

pub fn apply_effects(world: &mut World, ctx: &EffectCtx, effects: &[Effect]) {
    for e in effects {
        apply_effect(world, ctx, e);
    }
}

/// Living pawns selected by a scope, in id order.
pub fn resolve_scope(world: &mut World, ctx: &EffectCtx, scope: &Scope) -> Vec<Entity> {
    let alive_pawns = |world: &mut World| -> Vec<Entity> {
        crate::sorted_entities::<Pawn>(world).into_iter().filter(|e| world.get::<Dead>(*e).is_none()).collect()
    };
    match scope {
        Scope::Subject => ctx.subject.into_iter().collect(),
        Scope::Target => ctx.target.into_iter().collect(),
        Scope::SubjectFaction | Scope::TargetFaction => {
            let who = if matches!(scope, Scope::SubjectFaction) { ctx.subject } else { ctx.target };
            let Some(f) = who.and_then(|e| world.get::<FactionMember>(e)).map(|m| m.faction.clone()) else {
                return vec![];
            };
            alive_pawns(world).into_iter().filter(|e| world.get::<FactionMember>(*e).is_some_and(|m| m.faction == f)).collect()
        }
        Scope::Everyone => alive_pawns(world),
        Scope::Zone(z) => {
            let pawns = alive_pawns(world);
            let map = world.resource::<WorldMap>();
            pawns.into_iter().filter(|e| world.get::<Position>(*e).is_some_and(|p| map.in_zone(z, p))).collect()
        }
        Scope::Radius(r) => {
            let Some(center) = ctx.subject.and_then(|s| world.get::<Position>(s).copied()) else { return vec![] };
            alive_pawns(world)
                .into_iter()
                .filter(|e| Some(*e) != ctx.subject && world.get::<Position>(*e).is_some_and(|p| p.within(&center, *r)))
                .collect()
        }
        Scope::Template(t) => alive_pawns(world)
            .into_iter()
            .filter(|e| world.get::<TemplateId>(*e).is_some_and(|x| &x.0 == t))
            .collect(),
    }
}

fn faction_of(world: &World, e: Option<Entity>) -> Option<String> {
    e.and_then(|e| world.get::<FactionMember>(e)).map(|m| m.faction.clone())
}

pub fn apply_effect(world: &mut World, ctx: &EffectCtx, effect: &Effect) {
    let subj = ctx.subject;
    let tick = world.resource::<SimClock>().tick;
    match effect {
        Effect::On(scope, inner) => {
            for e in resolve_scope(world, ctx, scope) {
                let c = EffectCtx { subject: Some(e), target: ctx.subject.or(ctx.target), origin: ctx.origin.clone() };
                apply_effect(world, &c, inner);
            }
        }
        Effect::All(v) => apply_effects(world, ctx, v),
        Effect::Chance(p, inner) => {
            if world.resource_mut::<SimRng>().chance(*p) {
                apply_effect(world, ctx, inner);
            }
        }
        Effect::If(c, inner) => {
            if eval_condition(world, ctx, c) {
                apply_effect(world, ctx, inner);
            }
        }
        Effect::IfElse(c, a, b) => {
            let branch = if eval_condition(world, ctx, c) { a } else { b };
            apply_effect(world, ctx, branch);
        }
        Effect::ModStat { stat, amount } => {
            let bounds = world.resource::<Content>().stat_bounds(stat);
            if let Some(mut s) = subj.and_then(|e| world.get_mut::<Stats>(e)) {
                s.add_base(stat, *amount, bounds);
            }
        }
        Effect::SetStat { stat, value } => {
            let bounds = world.resource::<Content>().stat_bounds(stat);
            if let Some(mut s) = subj.and_then(|e| world.get_mut::<Stats>(e)) {
                s.set_base(stat, *value, bounds);
            }
        }
        Effect::ModNeed { need, amount } => {
            if let Some(mut n) = subj.and_then(|e| world.get_mut::<Needs>(e)) {
                n.add(need, *amount);
            }
        }
        Effect::ApplyStatus { status, severity } => {
            if let Some(e) = subj {
                crate::status::apply_status(world, e, status, *severity, ctx.target);
            }
        }
        Effect::RemoveStatus(s) => {
            if let Some(e) = subj {
                crate::status::remove_status(world, e, s, false);
            }
        }
        Effect::RemoveStatusTag(tag) => {
            if let Some(e) = subj {
                let content = world.resource::<Content>();
                let ids: Vec<String> = world
                    .get::<StatusEffects>(e)
                    .map(|s| {
                        s.active
                            .keys()
                            .filter(|id| content.statuses.get(*id).is_some_and(|d| d.tags.contains(tag)))
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
                for id in ids {
                    crate::status::remove_status(world, e, &id, false);
                }
            }
        }
        Effect::Immunize { status, ticks } => {
            if let Some(mut s) = subj.and_then(|e| world.get_mut::<StatusEffects>(e)) {
                let until = if *ticks == 0 { u64::MAX } else { tick + ticks };
                s.immunities.insert(status.clone(), until);
            }
        }
        Effect::AddTag(t) => {
            if let Some(mut tags) = subj.and_then(|e| world.get_mut::<Tags>(e)) {
                tags.base.insert(t.clone());
                tags.effective.insert(t.clone());
            }
        }
        Effect::RemoveTag(t) => {
            if let Some(mut tags) = subj.and_then(|e| world.get_mut::<Tags>(e)) {
                tags.base.remove(t);
                tags.effective.remove(t);
            }
        }
        Effect::GiveItem { item, qty } => {
            if let Some(e) = subj {
                crate::inventory_ops::give(world, e, item, *qty);
            }
        }
        Effect::GiveRandomItem { items, tag, qty } => {
            if let Some(e) = subj {
                let content = world.resource::<Content>().clone();
                let mut pool: Vec<String> = items.clone();
                if let Some(t) = tag {
                    pool.extend(content.items.values().filter(|d| d.tags.contains(t)).map(|d| d.id.clone()));
                }
                pool.sort();
                pool.dedup();
                for _ in 0..*qty {
                    if let Some(item) = world.resource_mut::<crate::rng::SimRng>().pick(&pool).cloned() {
                        crate::inventory_ops::give(world, e, &item, 1);
                    }
                }
            }
        }
        Effect::TakeItem { item, qty } => {
            if let Some(e) = subj {
                crate::inventory_ops::take(world, e, item, *qty);
            }
        }
        Effect::ModMoney(amount) => {
            if let Some(mut w) = subj.and_then(|e| world.get_mut::<Wallet>(e)) {
                w.0 = (w.0 + amount).max(0.0);
            }
        }
        Effect::ModTreasury(amount) => {
            if let Some(f) = faction_of(world, subj) {
                world.resource_mut::<Factions>().add_treasury(&f, *amount);
            }
        }
        Effect::ModRelation { faction, amount } => {
            if let Some(f) = faction_of(world, subj) {
                world.resource_mut::<Factions>().modify_relation(&f, faction, *amount);
            }
        }
        Effect::AddWanted { amount, crime } => {
            if let Some(e) = subj {
                crate::crime::add_wanted(world, e, *amount, if crime.is_empty() { "reato" } else { crime }, None);
            }
        }
        Effect::ClearWanted => {
            if let Some(e) = subj {
                crate::crime::clear_wanted(world, e);
            }
        }
        Effect::Damage { amount, part } => {
            if let Some(e) = subj {
                if world.get::<crate::buildings::Building>(e).is_some() {
                    crate::buildings::damage_building(world, e, *amount, ctx.target);
                } else {
                    crate::anatomy::damage(world, e, *amount, part.as_deref(), ctx.target);
                }
            }
        }
        Effect::Heal(amount) => {
            if let Some(e) = subj {
                crate::anatomy::heal(world, e, *amount);
            }
        }
        Effect::Kill => {
            if let Some(e) = subj {
                crate::lifecycle::kill(world, e, &ctx.origin, ctx.target);
            }
        }
        Effect::Transmute(race) => {
            if let Some(e) = subj {
                crate::status::transmute(world, e, race);
            }
        }
        Effect::AddClass(c) => {
            if let Some(e) = subj {
                if let Some(mut cl) = world.get_mut::<Classes>(e)
                    && !cl.0.contains(c) {
                        cl.0.push(c.clone());
                    }
                crate::lifecycle::refresh_role(world, e);
            }
        }
        Effect::RemoveClass(c) => {
            if let Some(e) = subj {
                if let Some(mut cl) = world.get_mut::<Classes>(e) {
                    cl.0.retain(|x| x != c);
                }
                crate::lifecycle::refresh_role(world, e);
            }
        }
        Effect::JoinFaction(f) => {
            if let Some(e) = subj {
                crate::social::change_faction(world, e, f, "cambio di fazione");
            }
        }
        Effect::VictoryPoints(n) => {
            if let Some(f) = faction_of(world, subj)
                && let Some(s) = world.resource_mut::<Factions>().states.get_mut(&f) {
                    s.victory_points += n;
                }
        }
        Effect::ModDissent(amount) => {
            if let Some(mut d) = subj.and_then(|e| world.get_mut::<crate::factions::Dissent>(e)) {
                d.0 = (d.0 + amount).clamp(0.0, 100.0);
            }
        }
        Effect::Spawn { template, count, zone } => {
            let pos = match zone {
                Some(z) => {
                    let map = world.resource::<WorldMap>().clone();
                    map.random_cell(z, &mut world.resource_mut::<SimRng>())
                }
                None => subj.and_then(|e| world.get::<Position>(e).copied()),
            };
            for _ in 0..*count {
                let p = pos.or_else(|| {
                    let map = world.resource::<WorldMap>().clone();
                    map.random_cell(&map.zones[0].id.clone(), &mut world.resource_mut::<SimRng>())
                });
                crate::lifecycle::spawn_template(world, template, p, &Default::default());
            }
        }
        Effect::MarketShock { item, tag, demand, supply, duration } => {
            crate::market::add_shock(world, item.as_deref(), tag.as_deref(), *demand, *supply, *duration, &ctx.origin);
        }
        Effect::GlobalModifier { id, name, duration, logistics_disruption, morale } => {
            crate::buildings::activate_modifier(world, id, name, *duration, *logistics_disruption, *morale);
        }
        Effect::SetFlag { flag, value } => {
            world.resource_mut::<Flags>().0.insert(flag.clone(), *value);
        }
        Effect::ModFlag { flag, amount } => {
            *world.resource_mut::<Flags>().0.entry(flag.clone()).or_insert(0.0) += amount;
        }
        Effect::Publish { headline, truth, topics } => {
            crate::press::publish(world, subj, headline.clone(), *truth, topics.clone(), ctx.target, None);
        }
        Effect::Emit { kind, message, news } => {
            let pos = subj.and_then(|e| world.get::<Position>(e).copied());
            let actor = subj.and_then(|e| world.get::<SimId>(e).copied());
            let target = ctx.target.and_then(|e| world.get::<SimId>(e).copied());
            let msg = substitute(world, message, subj, ctx.target);
            world.resource_mut::<EventLog>().push(
                tick,
                EventBuilder::new(kind.clone(), msg).actor(actor).target(target).pos(pos).news(*news),
            );
        }
        Effect::ModCover(amount) => {
            if let Some(e) = subj {
                let reason = origin_label(world, &ctx.origin);
                crate::infiltration::mod_cover(world, e, *amount, &reason);
            }
        }
        Effect::Expose => {
            if let Some(e) = subj {
                let reason = origin_label(world, &ctx.origin);
                crate::infiltration::expose(world, e, &reason, ctx.target);
            }
        }
        Effect::Stealth { amount, duration } => {
            if let Some(e) = subj {
                crate::infiltration::grant_stealth(world, e, *amount, *duration);
            }
        }
        Effect::ResetAggro => {
            if let Some(e) = subj {
                crate::infiltration::reset_aggro(world, e);
            }
        }
        Effect::Shapeshift { race, faction, name } => {
            if let Some(e) = subj {
                let d = crate::content::Disguise { race: race.clone(), faction: faction.clone(), name: name.clone() };
                crate::infiltration::shapeshift(world, e, d, ctx.target);
            }
        }
        Effect::RevertForm => {
            if let Some(e) = subj {
                crate::infiltration::revert(world, e);
            }
        }
        Effect::ReleaseTether => {
            if let Some(e) = subj {
                crate::dungeon::release(world, e, &ctx.origin);
            }
        }
        Effect::Contaminate { status, load } => {
            if let Some(e) = subj {
                crate::hygiene::contaminate(world, e, status, *load);
            }
        }
        Effect::Spill { fluid, amount } => {
            if let Some(p) = subj.and_then(|e| world.get::<Position>(e).copied()) {
                crate::hygiene::spill(world, p, fluid, *amount);
            }
        }
        Effect::Clean(amount) => {
            if let Some(p) = subj.and_then(|e| world.get::<Position>(e).copied()) {
                crate::hygiene::clean(world, p, 1, *amount);
            }
        }
        Effect::DamageBuilding(amount) => {
            if let Some(e) = subj {
                crate::buildings::damage_building(world, e, *amount, ctx.target);
            }
        }
        Effect::RepairBuilding(amount) => {
            if let Some(e) = subj {
                crate::buildings::repair_building(world, e, *amount);
            }
        }
        Effect::Teleport { zone } => {
            if let Some(e) = subj {
                let map = world.resource::<WorldMap>().clone();
                if let Some(p) = map.random_cell(zone, &mut world.resource_mut::<SimRng>()) {
                    world.entity_mut(e).insert(p);
                }
            }
        }
        Effect::PostJob { job, priority } => {
            let faction = faction_of(world, subj);
            let target = ctx.target.or(subj).map(|e| crate::jobs::JobTarget::of_entity(world, e)).unwrap_or_default();
            crate::jobs::post_job(world, job, faction, target, *priority, subj);
        }
        Effect::Log(msg) => {
            let msg = substitute(world, msg, subj, ctx.target);
            world.resource_mut::<EventLog>().push(tick, EventBuilder::new("log", msg));
        }
        Effect::Custom { id, params } => {
            let f = world.resource::<crate::extensions::Extensions>().effects.get(id).cloned();
            match f {
                Some(f) => f(world, ctx, params),
                None => tracing::warn!("effetto custom '{id}' non registrato"),
            }
        }
    }
}

/// Human readable cause from an effect origin ("ability:x" → the ability's name).
pub fn origin_label(world: &World, origin: &str) -> String {
    let c = world.resource::<Content>();
    let (kind, id) = origin.split_once(':').unwrap_or(("", origin));
    let name = match kind {
        "ability" => c.abilities.get(id).map(|d| d.name.clone()),
        "job" => c.jobs.get(id).map(|d| d.name.clone()),
        "status" => c.statuses.get(id).map(|d| d.name.clone()),
        "item" => c.items.get(id).map(|d| d.name.clone()),
        "trigger" => c.triggers.get(id).map(|d| if d.name.is_empty() { d.id.clone() } else { d.name.clone() }),
        _ => None,
    };
    name.unwrap_or_else(|| origin.to_string())
}

/// Replaces {subject}, {target} and {flag:x} in messages.
pub fn substitute(world: &World, msg: &str, subj: Option<Entity>, target: Option<Entity>) -> String {
    let name = |e: Option<Entity>| e.map(|e| crate::infiltration::apparent_name(world, e)).unwrap_or_else(|| "qualcuno".into());
    let mut s = msg.replace("{subject}", &name(subj)).replace("{target}", &name(target));
    if s.contains("{flag:") {
        for (k, v) in &world.resource::<Flags>().0 {
            s = s.replace(&format!("{{flag:{k}}}"), &format!("{v}"));
        }
    }
    s
}

pub fn eval_conditions_all(world: &mut World, ctx: &EffectCtx, conds: &[Condition]) -> bool {
    conds.iter().all(|c| eval_condition(world, ctx, c))
}

pub fn eval_condition(world: &mut World, ctx: &EffectCtx, cond: &Condition) -> bool {
    let subj = ctx.subject;
    let tick = world.resource::<SimClock>().tick;
    match cond {
        Condition::Always => true,
        Condition::Never => false,
        Condition::All(v) => v.iter().all(|c| eval_condition(world, ctx, c)),
        Condition::Any(v) => v.iter().any(|c| eval_condition(world, ctx, c)),
        Condition::Not(c) => !eval_condition(world, ctx, c),
        Condition::OnTarget(c) => {
            let swapped = EffectCtx { subject: ctx.target, target: ctx.subject, origin: ctx.origin.clone() };
            eval_condition(world, &swapped, c)
        }
        Condition::TickAtLeast(t) => tick >= *t,
        Condition::Every(n) => *n > 0 && tick.is_multiple_of(*n),
        Condition::Chance(p) => world.resource_mut::<SimRng>().chance(*p),
        Condition::Flag { flag, min } => world.resource::<Flags>().0.get(flag).is_some_and(|v| v >= min),
        Condition::StatAtLeast { stat, value } => subj.and_then(|e| world.get::<Stats>(e)).is_some_and(|s| s.get(stat) >= *value),
        Condition::StatBelow { stat, value } => subj.and_then(|e| world.get::<Stats>(e)).is_some_and(|s| s.get(stat) < *value),
        Condition::NeedBelow { need, value } => subj.and_then(|e| world.get::<Needs>(e)).is_some_and(|n| n.get(need) < *value),
        Condition::HasStatus(s) => subj.and_then(|e| world.get::<StatusEffects>(e)).is_some_and(|x| x.has(s)),
        Condition::HasStatusTag(t) => {
            let content = world.resource::<Content>();
            subj.and_then(|e| world.get::<StatusEffects>(e)).is_some_and(|x| x.has_tag(content, t))
        }
        Condition::HasTag(t) => subj.and_then(|e| world.get::<Tags>(e)).is_some_and(|x| x.has(t)),
        Condition::HasItem { item, qty } => subj.is_some_and(|e| crate::inventory_ops::count(world, e, item) >= *qty),
        Condition::HasItemTag(t) => {
            let content = world.resource::<Content>();
            subj.and_then(|e| world.get::<Inventory>(e)).is_some_and(|i| i.count_tag(content, t) > 0)
        }
        Condition::MoneyAtLeast(m) => subj.and_then(|e| world.get::<Wallet>(e)).is_some_and(|w| w.0 >= *m),
        Condition::InZone(z) => {
            let map = world.resource::<WorldMap>();
            subj.and_then(|e| world.get::<Position>(e)).is_some_and(|p| map.in_zone(z, p))
        }
        Condition::MemberOf(f) => faction_of(world, subj).is_some_and(|x| &x == f),
        Condition::IsRace(r) => subj.and_then(|e| world.get::<Race>(e)).is_some_and(|x| &x.0 == r),
        Condition::HasClass(c) => subj.and_then(|e| world.get::<Classes>(e)).is_some_and(|x| x.0.contains(c)),
        Condition::WantedAtLeast(v) => subj.and_then(|e| world.get::<crate::crime::Wanted>(e)).is_some_and(|w| w.level >= *v),
        Condition::Detained => subj.is_some_and(|e| world.get::<crate::crime::Detained>(e).is_some()),
        Condition::Disguised => subj.is_some_and(|e| world.get::<crate::infiltration::Disguise>(e).is_some()),
        Condition::TemplateDead(t) => {
            let (alive, dead) = template_counts(world, t);
            dead > 0 && alive == 0
        }
        Condition::TemplateAlive(t) => template_counts(world, t).0 > 0,
        Condition::BuildingHpBelow { building, ratio } => {
            let mut q = world.query::<&crate::buildings::Building>();
            q.iter(world).any(|b| &b.def == building && b.hp / b.max_hp < *ratio)
        }
        Condition::ZoneOccupied { zone, faction, min } => {
            let map = world.resource::<WorldMap>().clone();
            let mut q = world.query_filtered::<(&Position, Option<&FactionMember>), (With<Pawn>, Without<Dead>)>();
            let n = q
                .iter(world)
                .filter(|(p, m)| map.in_zone(zone, p) && faction.as_ref().is_none_or(|f| m.is_some_and(|m| &m.faction == f)))
                .count();
            n as u32 >= *min
        }
        Condition::PopulationBelow { faction, template, count } => {
            let mut q = world.query_filtered::<(Option<&FactionMember>, &TemplateId), (With<Pawn>, Without<Dead>)>();
            let n = q
                .iter(world)
                .filter(|(m, t)| {
                    faction.as_ref().is_none_or(|f| m.is_some_and(|m| &m.faction == f)) && template.as_ref().is_none_or(|x| &t.0 == x)
                })
                .count();
            (n as u32) < *count
        }
        Condition::HpBelow(r) => subj.is_some_and(|e| {
            if let Some(b) = world.get::<crate::buildings::Building>(e) {
                b.hp / b.max_hp < *r
            } else {
                world.get::<crate::anatomy::Body>(e).is_some_and(|b| b.health_ratio() < *r)
            }
        }),
        Condition::TitleVacant(t) => world.resource::<Titles>().holder(t).is_none(),
        Condition::TreasuryAtLeast { faction, amount } => world.resource::<Factions>().treasury(faction) >= *amount,
        Condition::FactionHoldsItems { faction, items } => {
            let held = crate::inventory_ops::faction_holdings(world, faction);
            items.iter().all(|i| held.get(i).copied().unwrap_or(0) > 0)
        }
        Condition::VictoryPointsAtLeast(v) => faction_of(world, subj)
            .and_then(|f| world.resource::<Factions>().states.get(&f).map(|s| s.victory_points))
            .is_some_and(|x| x >= *v),
        Condition::ArticlesAtLeast(n) => world.resource::<crate::press::Feed>().articles.len() as u32 >= *n,
        Condition::Custom { id, params } => {
            let f = world.resource::<crate::extensions::Extensions>().conditions.get(id).cloned();
            match f {
                Some(f) => f(world, ctx, params),
                None => {
                    tracing::warn!("condizione custom '{id}' non registrata");
                    false
                }
            }
        }
    }
}

fn template_counts(world: &mut World, t: &str) -> (usize, usize) {
    let mut q = world.query::<(&TemplateId, Option<&Dead>)>();
    let (mut alive, mut dead) = (0, 0);
    for (tid, d) in q.iter(world) {
        if tid.0 == t {
            if d.is_some() { dead += 1 } else { alive += 1 }
        }
    }
    (alive, dead)
}

/// Convenience used by many modules.
pub fn name_of(world: &World, e: Entity) -> String {
    world.get::<DisplayName>(e).map_or_else(|| "?".into(), |n| n.0.clone())
}
