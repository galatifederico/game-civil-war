//! Spawning entities from templates, death, and role refresh after class changes.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;

use crate::ai::Brain;
use crate::anatomy::Body;
use crate::content::{Content, Disguise, Tether};
use crate::crime::Wanted;
use crate::events::{kind, EventBuilder, EventLog};
use crate::factions::{Dissent, FactionMember, Factions, Leader, Players, Titles};
use crate::ids::{IdIndex, SimId};
use crate::inventory::Inventory;
use crate::jobs::{release_task, PersonalQueue, Task, WorkPriorities};
use crate::map::Position;
use crate::movement::Movement;
use crate::press::Notebook;
use crate::snapshot::ActivityState;
use crate::stats::{Classes, Dead, DisplayName, Immortal, Needs, Pawn, Race, Stats, Tags, TemplateId, Virtual, Wallet};
use crate::status::StatusEffects;
use crate::time::SimClock;

#[derive(Debug, Clone, Default)]
pub struct SpawnOverrides {
    pub name: Option<String>,
    pub faction: Option<String>,
    pub rank: Option<String>,
    pub tether: Option<Tether>,
    pub leader_of: Option<String>,
    pub disguise: Option<Disguise>,
}

/// Registers a new entity id for `e`.
pub fn register(world: &mut World, e: Entity) -> SimId {
    let id = world.resource_mut::<IdIndex>().allocate();
    world.resource_mut::<IdIndex>().insert(id, e);
    world.entity_mut(e).insert(id);
    id
}

pub fn entity_of(world: &World, id: SimId) -> Option<Entity> {
    world.resource::<IdIndex>().get(id)
}

/// Spawns a pawn (or virtual entity) from a template. Returns None for unknown templates or when a
/// unique template already has a living instance.
pub fn spawn_template(world: &mut World, template: &str, pos: Option<Position>, ov: &SpawnOverrides) -> Option<Entity> {
    let content = world.resource::<Content>().clone();
    let t = content.templates.get(template)?.clone();
    if t.unique {
        let mut q = world.query_filtered::<&TemplateId, Without<Dead>>();
        if q.iter(world).any(|x| x.0 == t.id) {
            return None;
        }
    }
    let tick = world.resource::<SimClock>().tick;
    let e = world.spawn_empty().id();
    let id = register(world, e);
    let name = ov.name.clone().unwrap_or_else(|| {
        if t.unique { t.name.clone() } else { format!("{} {}", t.name, id.0) }
    });
    let mut base: BTreeMap<String, f32> = content.stats.values().map(|s| (s.id.clone(), s.default)).collect();
    for (k, v) in &t.stats {
        base.insert(k.clone(), *v);
    }
    let mut inv = Inventory::with_slots(t.slots.unwrap_or(3));
    for (item, qty) in &t.items {
        inv.add(&content, item, *qty);
    }
    let needs = Needs(content.needs.keys().map(|n| (n.clone(), 1.0)).collect());
    world.entity_mut(e).insert((
        DisplayName(name.clone()),
        TemplateId(t.id.clone()),
        Race(t.race.clone()),
        Classes(t.classes.clone()),
        Tags { base: t.tags.iter().cloned().collect(), effective: Default::default() },
        Stats { effective: base.clone(), base },
        needs,
        StatusEffects::default(),
        inv,
        Wallet(t.money),
        Dissent::default(),
        Wanted::default(),
    ));
    world.entity_mut(e).insert((
        Brain::default(),
        Task::default(),
        PersonalQueue::default(),
        WorkPriorities::default(),
        ActivityState::default(),
        Notebook::default(),
        Movement::default(),
    ));
    if t.virtual_entity {
        world.entity_mut(e).insert(Virtual);
    } else {
        world.entity_mut(e).insert(Pawn);
        if let Some(p) = pos {
            world.entity_mut(e).insert(p);
        }
        if let Some(plan) = content.races.get(&t.race).and_then(|r| content.body_plans.get(&r.body_plan)) {
            world.entity_mut(e).insert(Body::from_plan(plan));
        }
    }
    if t.immortal {
        world.entity_mut(e).insert(Immortal);
    }
    let faction = ov.faction.clone().or(t.faction.clone());
    if let Some(f) = faction.filter(|f| content.factions.contains_key(f)) {
        let rank = ov
            .rank
            .clone()
            .or(t.rank.clone())
            .filter(|r| content.rank(&f, r).is_some())
            .or_else(|| content.base_rank(&f).map(|r| r.id.clone()))
            .unwrap_or_default();
        world.entity_mut(e).insert(FactionMember { faction: f, rank, joined: tick });
    }
    if let Some(d) = ov.disguise.clone().or(t.disguise.clone()) {
        crate::infiltration::shapeshift(world, e, d, None);
        if let Some(c) = t.cover {
            world.entity_mut(e).insert(crate::infiltration::Cover(c));
        }
    }
    if let Some(tether) = ov.tether.clone() {
        world.entity_mut(e).insert(crate::dungeon::Tethered(tether));
    }
    if let Some(player) = &ov.leader_of {
        world.entity_mut(e).insert((Leader { player: player.clone() }, crate::player::Controlled));
        let fac = world.get::<FactionMember>(e).map(|m| m.faction.clone());
        if let Some(p) = world.resource_mut::<Players>().players.get_mut(player) {
            p.leader = Some(id);
        }
        if let Some(f) = fac
            && let Some(s) = world.resource_mut::<Factions>().states.get_mut(&f) {
                s.controlled_by = Some(player.clone());
            }
    }
    for title in &t.titles {
        world.resource_mut::<Titles>().holders.insert(title.clone(), Some(id));
    }
    let race = content.races.get(&t.race).cloned();
    for s in t.statuses.iter().chain(race.iter().flat_map(|r| r.innate_statuses.iter())) {
        crate::status::apply_status(world, e, s, 1.0, None);
    }
    refresh_role(world, e);
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::SPAWN, format!("Compare {name}")).target(Some(id)).pos(pos).tags([t.id.clone()]),
    );
    Some(e)
}

/// Recomputes the actions a pawn can consider and its default work priorities from classes, race and
/// template. Explicit priority overrides are kept.
pub fn refresh_role(world: &mut World, e: Entity) {
    let content = world.resource::<Content>();
    let classes = world.get::<Classes>(e).map(|c| c.0.clone()).unwrap_or_default();
    let template = world.get::<TemplateId>(e).and_then(|t| content.templates.get(&t.0));
    let mut actions: Vec<String> = content.actions.values().filter(|a| a.universal).map(|a| a.id.clone()).collect();
    let mut work: BTreeMap<String, u8> = BTreeMap::new();
    for c in &classes {
        if let Some(cd) = content.classes.get(c) {
            actions.extend(cd.actions.iter().cloned());
            for (wt, p) in &cd.work {
                let cur = work.entry(wt.clone()).or_insert(*p);
                if *p != 0 && (*cur == 0 || *p < *cur) {
                    *cur = *p;
                }
            }
        }
    }
    if let Some(t) = template {
        actions.extend(t.actions.iter().cloned());
    }
    actions.sort();
    actions.dedup();
    if let Some(mut b) = world.get_mut::<Brain>(e) {
        b.actions = actions;
    }
    if let Some(mut w) = world.get_mut::<WorkPriorities>(e) {
        w.defaults = work;
    }
}

/// Kills an entity (immortals survive with a scratch). Vacates titles and releases jobs.
pub fn kill(world: &mut World, e: Entity, cause: &str, killer: Option<Entity>) {
    if world.get::<Dead>(e).is_some() {
        return;
    }
    if world.get::<Immortal>(e).is_some() {
        if let Some(mut b) = world.get_mut::<Body>(e) {
            for p in b.parts.iter_mut().filter(|p| p.vital) {
                p.hp = p.hp.max(1.0);
            }
        }
        return;
    }
    let tick = world.resource::<SimClock>().tick;
    release_task(world, e);
    world.entity_mut(e).insert(Dead { tick, cause: cause.to_string() });
    world.entity_mut(e).remove::<crate::crime::Detained>();
    let id = world.get::<SimId>(e).copied();
    let name = crate::effects::name_of(world, e);
    let unique = world
        .get::<TemplateId>(e)
        .and_then(|t| world.resource::<Content>().templates.get(&t.0))
        .is_some_and(|t| t.unique);
    let pos = world.get::<Position>(e).copied();
    let killer_id = killer.and_then(|k| world.get::<SimId>(k).copied());
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::DEATH, format!("{name} muore ({cause})"))
            .actor(killer_id)
            .target(id)
            .pos(pos)
            .news(if unique { 1.0 } else { 0.5 })
            .tags(["death"]),
    );
    crate::squads::remove_member(world, e);
    if let Some(id) = id {
        let titles = world.resource::<Titles>().held_by(id);
        for t in titles {
            crate::social::vacate_title(world, &t);
        }
    }
}
