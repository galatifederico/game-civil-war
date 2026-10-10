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
    skip_placements: bool,
    /// Edits from the admin console go to the overrides file of the last pack folder loaded.
    overrides: Option<std::path::PathBuf>,
}

impl SimBuilder {
    pub fn new(seed: u64) -> Self {
        let mut ext = Extensions::default();
        crate::handlers::register_core(&mut ext);
        Self { seed, packs: Vec::new(), ext, systems: Vec::new(), plugins: Vec::new(), skip_placements: false, overrides: None }
    }

    pub fn add_pack(&mut self, pack: ContentPack) -> &mut Self {
        self.packs.push(pack);
        self
    }

    pub fn load_pack_dir(&mut self, dir: impl AsRef<Path>) -> Result<&mut Self, ContentError> {
        self.packs.extend(crate::content::load_pack_dir(&dir)?);
        self.overrides = Some(dir.as_ref().join(crate::content::OVERRIDES_FILE));
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

    /// Builds the simulation and restores a save instead of placing the initial population.
    pub fn build_from_save(mut self, save: &crate::save::SaveGame) -> Result<Simulation, ContentError> {
        self.skip_placements = true;
        let mut sim = self.build()?;
        crate::save::load(&mut sim.world, save).map_err(|e| ContentError::Invalid(vec![e]))?;
        block_building_footprints(&mut sim.world);
        // Replay dug and painted tiles.
        let changes = sim.world.resource::<crate::map::TerrainChanges>().0.clone();
        for (p, ch) in changes {
            sim.world.resource_mut::<WorldMap>().make_mut().set_tile(p, ch);
        }
        Ok(sim)
    }

    pub fn build(self) -> Result<Simulation, ContentError> {
        let content = Content::from_packs(self.packs)?;
        let missing: Vec<String> = content
            .jobs
            .values()
            .filter(|j| !j.handler.is_empty() && !self.ext.job_handlers.contains_key(&j.handler))
            .map(|j| format!("job {}: handler '{}' non registrato", j.id, j.handler))
            .collect();
        if !missing.is_empty() {
            return Err(ContentError::Invalid(missing));
        }
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
        world.insert_resource(crate::supply::SupplyStats::default());
        world.insert_resource(crate::territory::Territories::default());
        world.insert_resource(Feed { name: content.press.feed_name.clone(), articles: vec![], next_id: 1 });
        world.insert_resource(crate::press::PressCursor::default());
        world.insert_resource(crate::telemetry::TelemetryCursor::default());
        world.insert_resource(Flags::default());
        world.insert_resource(crate::dungeon::TriggerState::default());
        world.insert_resource(crate::effects::Odds::default());
        world.insert_resource(Progress::default());
        world.insert_resource(WorldMap::from_def(content.map.as_ref()));
        world.insert_resource(crate::map::Environment::default());
        world.insert_resource(crate::map::TerrainChanges::default());
        world.insert_resource(self.ext);
        world.insert_resource(content);
        world.insert_resource(crate::content::ContentOverrides(self.overrides.clone()));

        let mut schedule = Schedule::default();
        schedule.set_executor(SingleThreadedExecutor::new());
        schedule.configure_sets(
            (SimSet::Input, SimSet::Derive, SimSet::Health, SimSet::World, SimSet::Ai, SimSet::Act, SimSet::Economy, SimSet::Social, SimSet::Press, SimSet::Output).chain(),
        );
        schedule.add_systems(crate::commands::apply_commands.in_set(SimSet::Input));
        schedule.add_systems((crate::stats::stat_recovery, crate::equipment::update, crate::modes::update, crate::abilities::auras, crate::stats::recompute_stats, crate::stats::decay_needs, low_needs).chain().in_set(SimSet::Derive));
        schedule.add_systems((crate::status::tick_statuses, crate::anatomy::natural_healing, crate::hygiene::hygiene_tick).chain().in_set(SimSet::Health));
        schedule.add_systems((crate::dungeon::tethers, crate::dungeon::events).chain().in_set(SimSet::World));
        schedule.add_systems((crate::player::wake_up, crate::player::follow_system, crate::ai::think).chain().in_set(SimSet::Ai));
        schedule.add_systems((crate::jobs::run_jobs, crate::crime::crime_upkeep).chain().in_set(SimSet::Act));
        schedule.add_systems((crate::buildings::buildings_tick, crate::buildings::circumstances_tick, crate::logistics::post_logistics, crate::market::update_market, crate::supply::imports, crate::economy::exports, crate::economy::payroll).chain().in_set(SimSet::Economy));
        schedule.add_systems(
            (crate::strategy::faction_goals, crate::beliefs::tick, crate::social::defections, crate::social::promotions, crate::social::merges, crate::social::succession, crate::titles::roles_tick, crate::classes::progression, crate::territory::conquest, crate::victory::collections, crate::victory::check_victory)
                .chain()
                .in_set(SimSet::Social),
        );
        schedule.add_systems((crate::press::gather_scoops, crate::press::prune_feed).chain().in_set(SimSet::Press));
        schedule.add_systems((crate::snapshot::update_activity, crate::telemetry::record_metrics).chain().in_set(SimSet::Output));
        for add in self.systems {
            add(&mut schedule);
        }

        let mut sim = Simulation { world, schedule, plugins: self.plugins };
        if !self.skip_placements {
            sim.place_initial();
        }
        Ok(sim)
    }
}

/// Buildings are solid except for their door (the anchor cell).
fn block_building_footprints(world: &mut World) {
    let content = world.resource::<Content>().clone();
    let mut cells = Vec::new();
    let mut q = world.query::<(&Building, &Position)>();
    for (b, p) in q.iter(world) {
        if let Some((w, h)) = content.buildings.get(&b.def).and_then(|d| d.footprint) {
            cells.extend(crate::map::footprint_cells(*p, w, h).into_iter().filter(|c| c != p));
        }
    }
    if !cells.is_empty() {
        world.resource_mut::<WorldMap>().make_mut().block(&cells);
    }
}

/// Needs below a threshold run that threshold's effects every tick.
fn low_needs(world: &mut World) {
    let content = world.resource::<Content>().clone();
    if content.needs.values().all(|n| n.thresholds.iter().all(|t| t.effects.is_empty())) {
        return;
    }
    for e in crate::sorted_entities::<crate::stats::Pawn>(world) {
        if world.get::<Dead>(e).is_some() {
            continue;
        }
        for n in content.needs.values() {
            let Some(v) = world.get::<crate::stats::Stats>(e).and_then(|s| s.base.get(&n.stat).copied()) else { continue };
            for th in n.thresholds.iter().filter(|th| v < th.below && !th.effects.is_empty()) {
                apply_effects(world, &EffectCtx::new(Some(e), None, format!("need:{}", n.id)), &th.effects);
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
        block_building_footprints(world);
        // Derived state ready before the first tick (stats, tags, activity).
        let mut init = Schedule::default();
        init.set_executor(SingleThreadedExecutor::new());
        init.add_systems((crate::equipment::update, crate::modes::update, crate::stats::recompute_stats, crate::snapshot::update_activity).chain());
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

    pub fn save(&mut self) -> crate::save::SaveGame {
        crate::save::save(&mut self.world)
    }

    pub fn save_to_file(&mut self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        let json = serde_json::to_vec(&self.save()).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    pub fn read_save(path: impl AsRef<std::path::Path>) -> std::io::Result<crate::save::SaveGame> {
        serde_json::from_slice(&std::fs::read(path)?).map_err(std::io::Error::other)
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
