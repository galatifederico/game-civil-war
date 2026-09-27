//! [`Simulation`] facade, [`SimBuilder`] and the [`SimPlugin`] extension trait.

use std::path::Path;

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SingleThreadedExecutor;

use crate::content::{ContentError, ContentPack, Placement};
use crate::effects::{apply_effects, EffectCtx};
use crate::extensions::{ConditionFn, EffectFn, Extensions, InputFn, JobHandlerFn};
use crate::prelude::*;

/// Ordered phases of a tick. Plugins add their systems to a phase.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimSet {
    Input,
    Derive,
    Health,
    World,
    Ai,
    Act,
    Economy,
    Social,
    Press,
    Output,
}

/// A world plugin: registers content packs and code extensions.
pub trait SimPlugin {
    fn name(&self) -> &str;
    fn build(&self, builder: &mut SimBuilder) -> Result<(), ContentError>;
}

type SystemAdder = Box<dyn FnOnce(&mut Schedule) + Send>;

pub struct SimBuilder {
    seed: u64,
    packs: Vec<ContentPack>,
    ext: Extensions,
    systems: Vec<SystemAdder>,
    plugins: Vec<String>,
}

impl SimBuilder {
    pub fn new(seed: u64) -> Self {
        let mut ext = Extensions::default();
        crate::handlers::register_core(&mut ext);
        Self { seed, packs: Vec::new(), ext, systems: Vec::new(), plugins: Vec::new() }
    }

    pub fn add_pack(&mut self, pack: ContentPack) -> &mut Self {
        self.packs.push(pack);
        self
    }

    pub fn load_pack_dir(&mut self, dir: impl AsRef<Path>) -> Result<&mut Self, ContentError> {
        self.packs.extend(crate::content::load_pack_dir(dir)?);
        Ok(self)
    }

    pub fn add_plugin(&mut self, plugin: &dyn SimPlugin) -> Result<&mut Self, ContentError> {
        self.plugins.push(plugin.name().to_string());
        plugin.build(self)?;
        Ok(self)
    }

    pub fn register_effect(&mut self, id: &str, f: impl Fn(&mut World, &EffectCtx, &serde_json::Value) + Send + Sync + 'static) -> &mut Self {
        self.ext.effects.insert(id.to_string(), std::sync::Arc::new(f) as EffectFn);
        self
    }

    pub fn register_condition(&mut self, id: &str, f: impl Fn(&mut World, &EffectCtx, &serde_json::Value) -> bool + Send + Sync + 'static) -> &mut Self {
        self.ext.conditions.insert(id.to_string(), std::sync::Arc::new(f) as ConditionFn);
        self
    }

    pub fn register_input(&mut self, id: &str, f: impl Fn(&mut World, Entity) -> f32 + Send + Sync + 'static) -> &mut Self {
        self.ext.inputs.insert(id.to_string(), std::sync::Arc::new(f) as InputFn);
        self
    }

    pub fn register_job_handler(&mut self, id: &str, f: impl Fn(&mut World, &crate::jobs::JobCtx) -> crate::jobs::JobResult + Send + Sync + 'static) -> &mut Self {
        self.ext.job_handlers.insert(id.to_string(), std::sync::Arc::new(f) as JobHandlerFn);
        self
    }

    /// Adds systems to the schedule, e.g. `b.add_systems(|s| { s.add_systems(my_system.in_set(SimSet::World)); })`.
    pub fn add_systems(&mut self, f: impl FnOnce(&mut Schedule) + Send + 'static) -> &mut Self {
        self.systems.push(Box::new(f));
        self
    }

    pub fn build(self) -> Result<Simulation, ContentError> {
        let content = Content::from_packs(self.packs)?;
        let mut world = World::new();
        let mut params = Params::default();
        for (k, v) in &content.params {
            params.set(k.clone(), *v);
        }
        let titles = Titles { holders: content.titles.keys().map(|t| (t.clone(), None)).collect(), ..Default::default() };
        world.insert_resource(params);
        world.insert_resource(SimRng::new(self.seed));
        world.insert_resource(SimClock::default());
        world.insert_resource(IdIndex::default());
        world.insert_resource(EventLog::with_capacity(20_000));
        world.insert_resource(CommandQueue::default());
        world.insert_resource(CommandResults::default());
        world.insert_resource(SpriteMapping(content.sprites.clone()));
        world.insert_resource(JobBoard::default());
        world.insert_resource(Factions::from_content(&content));
        world.insert_resource(Players::default());
        world.insert_resource(titles);
        world.insert_resource(Squads::default());
        world.insert_resource(Market::from_content(&content));
        world.insert_resource(GlobalModifiers::default());
        world.insert_resource(Feed { name: content.press.feed_name.clone(), articles: vec![] });
        world.insert_resource(crate::press::PressCursor::default());
        world.insert_resource(crate::telemetry::TelemetryCursor::default());
        world.insert_resource(Flags::default());
        world.insert_resource(crate::dungeon::TriggerState::default());
        world.insert_resource(Progress::default());
        world.insert_resource(WorldMap::from_def(content.map.as_ref()));
        world.insert_resource(self.ext);
        world.insert_resource(content);

        let mut schedule = Schedule::default();
        schedule.set_executor(SingleThreadedExecutor::new());
        schedule.configure_sets(
            (SimSet::Input, SimSet::Derive, SimSet::Health, SimSet::World, SimSet::Ai, SimSet::Act, SimSet::Economy, SimSet::Social, SimSet::Press, SimSet::Output).chain(),
        );
        schedule.add_systems(crate::commands::apply_commands.in_set(SimSet::Input));
        schedule.add_systems((crate::stats::recompute_stats, crate::stats::decay_needs, low_needs).chain().in_set(SimSet::Derive));
        schedule.add_systems((crate::status::tick_statuses, crate::anatomy::natural_healing, crate::hygiene::hygiene_tick).chain().in_set(SimSet::Health));
        schedule.add_systems((crate::dungeon::tethers, crate::dungeon::spawners, crate::dungeon::triggers).chain().in_set(SimSet::World));
        schedule.add_systems(crate::ai::think.in_set(SimSet::Ai));
        schedule.add_systems((crate::jobs::run_jobs, crate::crime::crime_upkeep).chain().in_set(SimSet::Act));
        schedule.add_systems((crate::buildings::buildings_tick, crate::market::update_market, crate::economy::payroll).chain().in_set(SimSet::Economy));
        schedule.add_systems(
            (crate::social::defections, crate::social::merges, crate::social::succession, crate::victory::collections, crate::victory::check_victory)
                .chain()
                .in_set(SimSet::Social),
        );
        schedule.add_systems((crate::press::gather_scoops, crate::press::news_sources).chain().in_set(SimSet::Press));
        schedule.add_systems((crate::snapshot::update_activity, crate::telemetry::record_metrics).chain().in_set(SimSet::Output));
        for add in self.systems {
            add(&mut schedule);
        }

        let mut sim = Simulation { world, schedule, plugins: self.plugins };
        sim.place_initial();
        Ok(sim)
    }
}

/// Needs below their threshold run their `effects_when_low` every tick.
fn low_needs(world: &mut World) {
    let content = world.resource::<Content>().clone();
    if content.needs.values().all(|n| n.effects_when_low.is_empty()) {
        return;
    }
    for e in crate::sorted_entities::<Needs>(world) {
        if world.get::<Dead>(e).is_some() {
            continue;
        }
        for n in content.needs.values() {
            if !n.effects_when_low.is_empty() && world.get::<Needs>(e).is_some_and(|x| x.get(&n.id) < n.low_threshold) {
                apply_effects(world, &EffectCtx::new(Some(e), None, format!("need:{}", n.id)), &n.effects_when_low);
            }
        }
    }
}

pub struct Simulation {
    pub world: World,
    schedule: Schedule,
    pub plugins: Vec<String>,
}

impl Simulation {
    pub fn builder(seed: u64) -> SimBuilder {
        SimBuilder::new(seed)
    }

    fn place_initial(&mut self) {
        let placements = self.world.resource::<Content>().placements.clone();
        let world = &mut self.world;
        for p in &placements {
            if let Placement::Player { id, name, faction } = p {
                world.resource_mut::<Players>().players.insert(
                    id.clone(),
                    Player { id: id.clone(), name: name.clone(), faction: faction.clone(), leader: None },
                );
                if let Some(s) = world.resource_mut::<Factions>().states.get_mut(faction) {
                    s.controlled_by = Some(id.clone());
                }
            }
        }
        let pos_in = |world: &mut World, zone: &str, at: Option<(i32, i32)>| -> Option<Position> {
            let map = world.resource::<WorldMap>().clone();
            match at {
                Some((x, y)) => {
                    let layer = map.resolve_zones(zone).first().map_or(0, |i| map.zones[*i].layer);
                    Some(Position::new(layer, x, y))
                }
                None => map.random_cell(zone, &mut world.resource_mut::<SimRng>()),
            }
        };
        for p in &placements {
            if let Placement::Entity { template, count, zone, at, name, faction, tether, leader_of } = p {
                for _ in 0..*count {
                    let pos = pos_in(world, zone, *at);
                    let ov = SpawnOverrides {
                        name: name.clone(),
                        faction: faction.clone(),
                        tether: tether.clone(),
                        leader_of: leader_of.clone(),
                        ..Default::default()
                    };
                    crate::lifecycle::spawn_template(world, template, pos, &ov);
                }
            }
        }
        for p in &placements {
            if let Placement::Building { building, zone, at, name, owner_faction, owner_template } = p {
                let Some(pos) = pos_in(world, zone, *at) else { continue };
                let owner = match (owner_faction, owner_template) {
                    (Some(f), _) => Owner::Faction(f.clone()),
                    (None, Some(t)) => crate::sorted_entities::<TemplateId>(world)
                        .into_iter()
                        .find(|e| world.get::<TemplateId>(*e).is_some_and(|x| &x.0 == t))
                        .and_then(|e| world.get::<SimId>(e).copied())
                        .map_or(Owner::None, Owner::Entity),
                    _ => Owner::None,
                };
                crate::buildings::spawn_building(world, building, pos, name.clone(), owner);
            }
        }
        // Derived state ready before the first tick (stats, tags, activity).
        let mut init = Schedule::default();
        init.set_executor(SingleThreadedExecutor::new());
        init.add_systems((crate::stats::recompute_stats, crate::snapshot::update_activity).chain());
        init.run(world);
    }

    /// Runs one tick.
    pub fn tick(&mut self) {
        let start = std::time::Instant::now();
        self.schedule.run(&mut self.world);
        self.world.resource_mut::<SimClock>().tick += 1;
        metrics::histogram!("sim_tick_seconds").record(start.elapsed().as_secs_f64());
    }

    pub fn run(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.tick();
        }
    }

    pub fn tick_count(&self) -> u64 {
        self.world.resource::<SimClock>().tick
    }

    /// Queues a command for the next tick; returns its sequence number.
    pub fn enqueue(&mut self, cmd: SimCommand) -> u64 {
        self.world.resource_mut::<CommandQueue>().push(cmd)
    }

    /// Applies a command immediately (outside the tick).
    pub fn execute(&mut self, cmd: SimCommand) -> Result<String, String> {
        crate::commands::apply(&mut self.world, cmd)
    }

    pub fn content(&self) -> &Content {
        self.world.resource::<Content>()
    }

    pub fn events(&self) -> &EventLog {
        self.world.resource::<EventLog>()
    }

    pub fn snapshot(&mut self, truth: bool) -> WorldSnapshot {
        crate::snapshot::snapshot(&mut self.world, truth)
    }

    pub fn state_hash(&mut self) -> u64 {
        crate::snapshot::state_hash(&mut self.world)
    }

    pub fn entity(&self, id: SimId) -> Option<Entity> {
        self.world.resource::<IdIndex>().get(id)
    }

    /// First living entity created from a template.
    pub fn find_template(&mut self, template: &str) -> Option<(SimId, Entity)> {
        let world = &mut self.world;
        crate::sorted_entities::<TemplateId>(world)
            .into_iter()
            .find(|e| world.get::<TemplateId>(*e).is_some_and(|t| t.0 == template) && world.get::<Dead>(*e).is_none())
            .map(|e| (*world.get::<SimId>(e).unwrap(), e))
    }
}
