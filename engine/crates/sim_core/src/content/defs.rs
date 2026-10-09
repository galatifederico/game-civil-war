//! Serde definitions of every kind of content. A content pack is a folder of RON (or JSON) files, each
//! holding a partial [`ContentPack`]; files are merged in name order.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::logic::{Condition, Curve, Effect, Selector};

pub type Id = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContentPack {
    pub meta: PackMeta,
    /// Names of the engine concepts this pack binds to its own stat/need ids.
    pub bindings: Option<Bindings>,
    pub params: BTreeMap<String, f64>,
    pub stats: Vec<StatDef>,
    pub needs: Vec<NeedDef>,
    pub body_plans: Vec<BodyPlanDef>,
    pub races: Vec<RaceDef>,
    pub action_sets: Vec<ActionSetDef>,
    pub modes: Vec<ModeDef>,
    pub classes: Vec<ClassDef>,
    pub statuses: Vec<StatusDef>,
    pub fluids: Vec<FluidDef>,
    pub items: Vec<ItemDef>,
    pub abilities: Vec<AbilityDef>,
    pub jobs: Vec<JobDef>,
    pub actions: Vec<ActionDef>,
    pub factions: Vec<FactionDef>,
    pub buildings: Vec<BuildingDef>,
    pub map: Option<MapDef>,
    pub templates: Vec<EntityTemplate>,
    pub placements: Vec<Placement>,
    pub events: Vec<EventDef>,
    pub collections: Vec<CollectionDef>,
    pub titles: Vec<TitleDef>,
    pub press: Option<PressDef>,
    pub news_impacts: Vec<NewsImpactDef>,
    pub victory: Vec<VictoryDef>,
    pub global_modifiers: Vec<GlobalModifierDef>,
    pub supplies: Vec<SupplyDef>,
    pub sprites: BTreeMap<Id, SpriteDef>,
    /// Stock phrases pawns say when a player talks to them, by key: "default", "race:<id>",
    /// "class:<id>", "faction:<id>", "template:<id>" (all matching keys are pooled; "default" only for
    /// races without lines of their own).
    /// Lines may use {name}, {faction}, {zone}, {player_name}.
    pub dialogue: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PackMeta {
    pub id: Id,
    pub name: String,
    pub version: String,
    pub description: String,
}

/// Maps engine concepts to the pack's stat and need ids, so the engine never hardcodes a world's words.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Bindings {
    pub morale: Id,
    pub perception: Id,
    pub stealth: Id,
    pub speed: Id,
    pub strength: Id,
    pub intellect: Id,
    pub heroism: Id,
    pub press_reputation: Id,
    /// Tag carried by police pawns (derived from the police faction role).
    pub police_tag: Id,
    /// Tag carried by journalists.
    pub press_tag: Id,
    /// Item tag that police seize.
    pub contraband_tag: Id,
    /// Zone tag of the detention cells.
    pub jail_zone_tag: Id,
    /// Item tags of things that are eaten or drunk (poisoned stock infects whoever buys them).
    pub ingestible_tags: Vec<String>,
    /// Fluid spilled by bleeding body parts.
    pub blood_fluid: Option<Id>,
    pub currency_name: String,
    /// Stat counting the crimes on a pawn's record (raised by one for every charge), if any.
    pub crime_record: Option<Id>,
    /// Class a pawn falls back to when it loses its last class.
    pub default_class: Option<Id>,
    /// Mode of pawns whose own choice finds none (see `ModeDef`).
    pub default_mode: Option<Id>,
    /// Combat stats: damage dealt grows with `attack`, damage taken shrinks with `defense` (both 0..100).
    pub attack: Option<Id>,
    pub defense: Option<Id>,
}

impl Default for Bindings {
    fn default() -> Self {
        Self {
            morale: "morale".into(),
            perception: "perception".into(),
            stealth: "stealth".into(),
            speed: "speed".into(),
            strength: "strength".into(),
            intellect: "intellect".into(),
            heroism: "heroism".into(),
            press_reputation: "press_reputation".into(),
            police_tag: "police".into(),
            press_tag: "press".into(),
            contraband_tag: "contraband".into(),
            jail_zone_tag: "jail".into(),
            ingestible_tags: vec!["food".into(), "drink".into(), "water".into()],
            blood_fluid: Some("blood".into()),
            currency_name: "crediti".into(),
            crime_record: None,
            default_class: None,
            default_mode: None,
            attack: None,
            defense: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatDef {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub min: f32,
    #[serde(default = "hundred")]
    pub max: f32,
    #[serde(default)]
    pub default: f32,
    /// Shown in the UI character sheet.
    #[serde(default = "yes")]
    pub visible: bool,
    /// Value the stat slowly returns to (e.g. morale recovering after bad news).
    #[serde(default)]
    pub rest_value: Option<f32>,
    /// Points per tick towards `rest_value`.
    #[serde(default)]
    pub recovery: f32,
    /// Points added every in-game day (age, days without washing…), spread over the day's ticks.
    #[serde(default)]
    pub per_day: f32,
    /// Section of the character sheet ("Fisiche", "Conoscenze", "Contatori"…).
    #[serde(default)]
    pub group: String,
    /// New pawns get `default` ± a random amount up to this (unless their template sets the stat).
    #[serde(default)]
    pub spread: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// A need moves a stat every tick (eating keeps "fame" up, time brings it down); below each threshold
/// something happens every tick and some stats change while the pawn stays below.
pub struct NeedDef {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// The stat this need moves (default: the need's own id; created 0..100, full at birth, if missing).
    #[serde(default)]
    pub stat: Id,
    /// Change of the stat every tick (negative: it drops), before race, ability and status rates.
    pub per_tick: f32,
    #[serde(default)]
    pub thresholds: Vec<NeedThreshold>,
}

/// What happens while a need's stat is below `below`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NeedThreshold {
    pub name: String,
    pub below: f32,
    /// Effects every tick while below (subject = the pawn).
    pub effects: Vec<Effect>,
    /// Stat modifiers that last while below.
    pub stats: BTreeMap<Id, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyPlanDef {
    pub id: Id,
    pub parts: Vec<BodyPartDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyPartDef {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub parent: Option<Id>,
    pub hp: f32,
    /// Destroying a vital part kills the owner.
    #[serde(default)]
    pub vital: bool,
    /// Relative chance of being hit.
    #[serde(default = "one_f")]
    pub coverage: f32,
    /// Capacity id → contribution weight (e.g. "moving": 0.5 for each leg).
    #[serde(default)]
    pub capacities: BTreeMap<Id, f32>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RaceDef {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub body_plan: Id,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub abilities: Vec<Id>,
    /// Statuses every member is born with (e.g. innate traits).
    #[serde(default)]
    pub innate_statuses: Vec<Id>,
    /// Statuses members can never catch.
    #[serde(default)]
    pub immunities: Vec<Id>,
    /// Needs that do not apply to this race (machines do not eat, beasts find their own food…).
    #[serde(default)]
    pub needs_exempt: Vec<Id>,
    /// Extra decay of a need for this race (0.5 = half again as fast, -0.5 = half as fast).
    #[serde(default)]
    pub need_rates: BTreeMap<Id, f32>,
    /// Sexes members can be born with; empty for races without sex (machines, programs…).
    #[serde(default = "all_sexes")]
    pub sexes: Vec<crate::stats::Sex>,
    /// Groups of base actions every member considers (see `ActionSetDef`).
    #[serde(default)]
    pub action_sets: Vec<Id>,
    /// Base actions of this race on top of its groups.
    #[serde(default)]
    pub actions: Vec<Id>,
    /// Per-race limits, starting value and variation at birth of stats (unset values: the stat's own).
    /// A race has no fixed bonus: members are born around its starting value and can change from there.
    #[serde(default)]
    pub stat_ranges: BTreeMap<Id, StatRange>,
}

fn all_sexes() -> Vec<crate::stats::Sex> {
    use crate::stats::Sex;
    vec![Sex::Male, Sex::Female, Sex::NonBinary]
}

/// A race's own bounds for a stat; unset values fall back to the stat definition.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StatRange {
    pub min: Option<f32>,
    pub max: Option<f32>,
    /// Value members are born with.
    pub initial: Option<f32>,
    /// Random variation at birth around the starting value (± spread).
    pub spread: Option<f32>,
}

/// A way of living a pawn is in (one at a time): working, conquering, searching… It weighs the AI's
/// choice of actions by their tags and sets which board jobs come first. The player's order wins, then
/// the faction's mode, then the pawn's own choice (`when`, highest `priority` first).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModeDef {
    pub id: Id,
    pub name: String,
    pub description: String,
    /// Utility multiplier of actions by tag (an action with several listed tags gets their product).
    pub weights: BTreeMap<String, f32>,
    /// Multiplier of actions with none of the listed tags.
    pub default_weight: f32,
    /// Stat modifiers while in this mode.
    pub stats: BTreeMap<Id, f32>,
    /// Board job priorities by work type (1 = first … 4 = last, 0 = never); unlisted types count 3.
    pub work: BTreeMap<Id, u8>,
    /// When a pawn picks this mode on its own.
    pub when: Condition,
    /// Higher first among the modes whose `when` holds.
    pub priority: i32,
}

impl Default for ModeDef {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            weights: BTreeMap::new(),
            default_weight: 1.0,
            stats: BTreeMap::new(),
            work: BTreeMap::new(),
            when: Condition::Never,
            priority: 0,
        }
    }
}

/// A named group of base actions shared by races (daily life, animal instincts…).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionSetDef {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub actions: Vec<Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassDef {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub stats: BTreeMap<Id, f32>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub abilities: Vec<Id>,
    /// Utility AI actions this class unlocks.
    #[serde(default)]
    pub actions: Vec<Id>,
    /// What a pawn needs to take the class on its own (see `classes`), races allowed included
    /// (`RaceIn`). `None`: only given by templates and effects, never acquired.
    #[serde(default)]
    pub requires: Option<Condition>,
    /// When a pawn that took the class on its own loses it. `None`: never (each class decides).
    #[serde(default)]
    pub loses_when: Option<Condition>,
    /// Classes lost when this one is taken, and not taken again while it is held (a Jedi turning Sith).
    #[serde(default)]
    pub replaces: Vec<Id>,
    /// Section of the class list in the UI.
    #[serde(default)]
    pub group: String,
}

/// A status is an intensity 0..100 on a pawn: taking it adds `intensity` per dose, every tick it changes by
/// `per_tick` (less each point of the resistance stat), at 0 it ends. Its modifiers last while it is active;
/// the highest threshold reached adds its own. Everything is deterministic.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StatusDef {
    pub id: Id,
    pub name: String,
    pub description: String,
    /// Categories ("malattia", "droga", "mutazione"…), also seen by conditions on status tags.
    pub tags: Vec<String>,
    /// Intensity one dose adds (default 100).
    pub intensity: Option<f32>,
    /// Intensity change every tick (negative: it heals by itself; positive: it gets worse).
    pub per_tick: f32,
    /// Stat that fights the status: each point lowers the intensity by `resist_per_point` every tick and
    /// cuts the contagion received by its share of 100.
    pub resist_stat: Option<Id>,
    pub resist_per_point: f32,
    /// Stat modifiers while active.
    pub stats: BTreeMap<Id, f32>,
    pub need_rates: BTreeMap<Id, f32>,
    /// Tags the carrier gets while active.
    pub grants_tags: Vec<String>,
    /// Effects every tick while active (subject = the carrier).
    pub effects: Vec<Effect>,
    pub on_apply: Vec<Effect>,
    pub on_expire: Vec<Effect>,
    /// Intensity thresholds, low to high: the highest one reached adds its modifiers and effects.
    pub thresholds: Vec<StatusThreshold>,
    pub contagion: Option<ContagionDef>,
}

impl StatusDef {
    pub fn dose(&self) -> f32 {
        self.intensity.unwrap_or(100.0)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StatusThreshold {
    pub name: String,
    /// Reached at this intensity or more.
    pub above: f32,
    pub stats: BTreeMap<Id, f32>,
    pub need_rates: BTreeMap<Id, f32>,
    /// Effects every tick while this is the highest threshold reached.
    pub effects: Vec<Effect>,
    /// Effects once, when the threshold is reached.
    pub on_enter: Vec<Effect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Vector {
    /// Pawns on the same or adjacent cells.
    Contact,
    /// Pawns within `radius`.
    Air,
    /// Contaminated cells and fluids.
    Fluid,
    /// Contaminated water networks.
    Water,
    /// Contaminated food items.
    Food,
    /// Only through explicit effects (inoculation, injection, bites…).
    Injection,
}

/// Contagion as an aura: pawns within `radius` of a carrier gain `per_tick` × (carrier's intensity / 100)
/// every tick, cut by their resistance; the carrier also leaves `shedding` on its cell (water and ground).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContagionDef {
    pub radius: i32,
    pub per_tick: f32,
    pub shedding: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FluidDef {
    pub id: Id,
    pub name: String,
    /// Dirt added per unit spilled.
    #[serde(default = "one_f")]
    pub dirtiness: f32,
    /// Pathogen status carried by the fluid, if any.
    #[serde(default)]
    pub carries: Option<Id>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ItemDef {
    pub id: Id,
    pub name: String,
    pub description: String,
    /// Stacking category: items of the same category share a slot kind and its stack limit.
    pub category: Id,
    pub stack_max: u32,
    pub base_price: f64,
    pub tags: Vec<String>,
    /// Effects when used/consumed (the item is spent unless `reusable`).
    pub on_use: Vec<Effect>,
    pub reusable: bool,
    /// Stat modifiers while carried (equipment).
    pub carried_stats: BTreeMap<Id, f32>,
    /// Victory points for the holder's faction while held (relics).
    pub victory_points: i64,
    /// Unique items exist once in the world.
    pub unique: bool,
    /// Hit points of one unit: it wears when used to hit, thrown or hit while worn, and breaks at zero
    /// (0 = never wears).
    pub durability: f32,
    /// Weight of one unit: past the carrying capacity the bearer slows down.
    pub weight: f32,
    /// Needed to wield or wear it (its bonuses count only then).
    pub requires: Condition,
    /// Hands it takes in use (a two-handed sword: 2). Hand items count only while there are hands free.
    pub hands: u8,
    /// Where it is worn ("corpo", "testa"…): only the best item for each place counts.
    pub wear_slot: Option<Id>,
    /// Damage when hitting with it (weapons) or throwing it.
    pub damage: f32,
    /// How far it can be thrown.
    pub range: i32,
    /// Effects on whoever is hit by it (thrown or wielded): subject = the victim, target = the attacker.
    pub on_hit: Vec<Effect>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
/// A permanent trait of whoever has it (from race, classes or roles): modifiers on the holder and,
/// optionally, an aura on the pawns around it. Things a pawn *does* are actions, not abilities.
pub struct AbilityDef {
    pub id: Id,
    pub name: String,
    pub description: String,
    /// Stat modifiers on the holder.
    pub stats: BTreeMap<Id, f32>,
    /// Extra decay of the holder's needs (0.5 = half again as fast).
    pub need_rates: BTreeMap<Id, f32>,
    /// Tags the holder carries.
    pub tags: Vec<String>,
    /// Statuses the holder cannot catch.
    pub immunities: Vec<Id>,
    pub aura: Option<AuraDef>,
}

/// What an ability does to the pawns around its holder.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuraDef {
    /// Cells around the holder.
    pub radius: i32,
    /// Who is affected (subject = the pawn near the holder, target = the holder). Default: everybody.
    pub affects: Condition,
    /// Stat modifiers that last while the pawn stays in range.
    pub stats: BTreeMap<Id, f32>,
    /// Stat changes added every tick (hour) the pawn is in range, kept after it leaves.
    pub stats_per_tick: BTreeMap<Id, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrimeDef {
    pub id: Id,
    pub severity: f32,
    #[serde(default = "five")]
    pub witness_radius: i32,
    /// Newsworthiness of the crime event.
    #[serde(default = "half")]
    pub news: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct JobDef {
    pub id: Id,
    pub name: String,
    pub description: String,
    /// Column of the WorkPriorityMatrix this job belongs to.
    pub work_type: Id,
    /// Engine handler that gives the job its behaviour (see `jobs::handlers`). Default: "effects".
    pub handler: Id,
    /// Work units to complete (one unit per tick at skill 1).
    pub duration: f32,
    /// Distance from the target needed to work.
    pub range: i32,
    /// Stat that speeds up the work (value / 10 = multiplier, min 0.2).
    pub skill: Option<Id>,
    pub requires: Condition,
    /// Required entity tags (from race, class, statuses).
    pub required_tags: Vec<String>,
    pub min_rank: u32,
    pub crime: Option<CrimeDef>,
    /// Newsworthiness of the completion event (0 = silent).
    pub news: f32,
    /// Effects on completion (subject = worker, target = job target).
    pub effects: Vec<Effect>,
    /// Handler-specific settings (e.g. item, recipe, amount).
    pub params: BTreeMap<String, serde_json::Value>,
    /// Suspicious jobs lower the cover of disguised workers when seen.
    pub suspicious: bool,
    /// Violates these ideology tags (factions that forbid them get dissent when they see it).
    pub ideology_tags: Vec<String>,
}

/// How an action is done, written inside the action (`how`): the engine turns it into the job with the
/// action's id (name and description from the action). Same fields as a job.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct JobSpec {
    pub work_type: Id,
    pub handler: Id,
    pub duration: f32,
    pub range: i32,
    pub skill: Option<Id>,
    pub requires: Condition,
    pub required_tags: Vec<String>,
    pub min_rank: u32,
    pub crime: Option<CrimeDef>,
    pub news: f32,
    pub effects: Vec<Effect>,
    pub params: BTreeMap<String, serde_json::Value>,
    pub suspicious: bool,
    pub ideology_tags: Vec<String>,
}

impl JobSpec {
    pub fn to_job(&self, id: &str, name: &str, description: &str) -> JobDef {
        JobDef {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            work_type: self.work_type.clone(),
            handler: self.handler.clone(),
            duration: self.duration,
            range: self.range,
            skill: self.skill.clone(),
            requires: self.requires.clone(),
            required_tags: self.required_tags.clone(),
            min_rank: self.min_rank,
            crime: self.crime.clone(),
            news: self.news,
            effects: self.effects.clone(),
            params: self.params.clone(),
            suspicious: self.suspicious,
            ideology_tags: self.ideology_tags.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ActionKind {
    /// Do a job: the action's own (`how`, when `job` is empty or the action's id) or a separate one.
    Job { #[serde(default)] job: Id, #[serde(default)] target: Selector },
    /// Take the best job from the faction/global JobBoard according to the WorkPriorityMatrix.
    #[default]
    Work,
    /// Apply the action's own effects (subject = the actor, target = the selected target): an attack, a
    /// spell, a transformation… `range` 0 means on itself or where it stands.
    Effects { #[serde(default)] target: Selector, #[serde(default)] range: i32, #[serde(default)] effects: Vec<Effect> },
    /// Stand still.
    Idle,
}

/// Input of a utility consideration; always normalized to 0..1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Input {
    Constant(f32),
    Need(Id),
    /// Stat value divided by `max`.
    Stat { stat: Id, max: f32 },
    StatusSeverity { status: Id, max: f32 },
    HasStatus(Id),
    HasTag(Id),
    Money { max: f64 },
    Wanted { max: f32 },
    Dissent,
    ItemCount { tag: Id, max: u32 },
    /// 1 when the action's selector finds a target, else 0.
    TargetExists,
    /// Distance to the selected target over `max`.
    TargetDistance { max: f32 },
    /// Number of available board jobs the pawn could take, over `max`.
    JobsAvailable { max: f32 },
    /// Hour of day / ticks per day.
    TimeOfDay,
    Random,
    Param { key: String, max: f32 },
    /// Number of scoops in the pawn's notebook, over `max`.
    Scoops { max: f32 },
    Condition(Condition),
    /// Registered by a plugin.
    Custom(Id),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consideration {
    pub input: Input,
    #[serde(default)]
    pub curve: Curve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ActionDef {
    pub id: Id,
    pub name: String,
    pub kind: ActionKind,
    pub weight: f32,
    pub considerations: Vec<Consideration>,
    /// Who may consider this action (default: everybody).
    pub requires: Condition,
    /// Ticks before the action can be picked again after it ends.
    pub cooldown: u64,
    /// Label for the UI while doing it.
    pub label: String,
    pub description: String,
    /// Doing it in view of others is suspicious (lowers cover).
    pub suspicious: bool,
    pub tags: Vec<String>,
    /// How it is done (duration, work type, effects, crime…): the action's own job.
    pub how: Option<JobSpec>,
}

impl Default for ActionDef {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            kind: ActionKind::default(),
            // A missing weight means "normal importance", not "never".
            weight: 1.0,
            considerations: Vec::new(),
            requires: Condition::default(),
            cooldown: 0,
            label: String::new(),
            description: String::new(),
            suspicious: false,
            tags: Vec::new(),
            how: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RankDef {
    pub id: Id,
    pub name: String,
    pub level: u32,
    #[serde(default)]
    pub salary: f64,
    /// Multiplier on job priorities for board assignment (higher ranks get first pick).
    #[serde(default = "one_f")]
    pub job_priority: f32,
    /// Unique rank (the faction leader).
    #[serde(default)]
    pub unique: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FactionRole {
    #[default]
    Regular,
    /// Neutral public-order faction: patrols, searches, arrests, takes bribes.
    Police,
    /// Neutral press: journalists publishing on the feed.
    Press,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FactionDef {
    pub id: Id,
    pub name: String,
    pub description: String,
    pub role: FactionRole,
    pub neutral: bool,
    /// Can be led by a player.
    pub playable: bool,
    /// Ideology axes (e.g. "carne": -1..1).
    pub ideology: BTreeMap<Id, f32>,
    /// Ideology tags this faction forbids: members that see a job with such a tag gain dissent.
    pub forbids: Vec<String>,
    /// Initial relation towards other factions, -100..100.
    pub relations: BTreeMap<Id, f32>,
    pub ranks: Vec<RankDef>,
    pub treasury: f64,
    /// For police: how easily it accepts bribes (0 = never, 1 = normal, >1 = cheap).
    pub corruptibility: f32,
    /// Zones this faction patrols / considers home.
    pub zones: Vec<Id>,
    pub tags: Vec<String>,
    /// Strategic goals pursued by AI-run factions (player factions decide by themselves).
    pub goals: Vec<GoalDef>,
    /// Mode of its members unless the player orders otherwise.
    pub mode: Option<Id>,
}

/// "Get items with this tag that we do not hold": posts faction jobs targeting whoever holds them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GoalDef {
    pub item_tag: String,
    pub job: Id,
    pub interval: u64,
    pub priority: i32,
    pub max_open: u32,
    /// Only while this holds (evaluated without subject: flags, ticks, titles…).
    pub requires: Condition,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RecipeDef {
    pub id: Id,
    pub name: String,
    pub inputs: BTreeMap<Id, u32>,
    pub outputs: BTreeMap<Id, u32>,
    /// Work units per batch.
    pub work: f32,
    /// Work type of the production job posted on the board.
    pub work_type: Id,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DamageConsequenceDef {
    /// Fires when HP / max HP drops below this ratio.
    pub below: f32,
    pub name: String,
    pub effects: Vec<Effect>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BuildingDef {
    pub id: Id,
    pub name: String,
    pub description: String,
    /// Cells it occupies around its anchor (door): width centred on it, height upwards. (1, 1) = just the anchor.
    pub footprint: Option<(i32, i32)>,
    pub hp: f32,
    pub tags: Vec<String>,
    pub recipes: Vec<RecipeDef>,
    /// Items it can sell (shop catalog) with an optional fixed price (None = market price × markup).
    pub sells: BTreeMap<Id, Option<f64>>,
    pub markup: f32,
    /// Stock it starts with.
    pub stock: BTreeMap<Id, u32>,
    /// Construction cost (items) for build jobs.
    pub cost: BTreeMap<Id, u32>,
    pub build_work: f32,
    pub consequences: Vec<DamageConsequenceDef>,
    /// Items produced passively every `passive_interval` ticks (fields, pens, generators).
    pub passive: BTreeMap<Id, u32>,
    pub passive_interval: u64,
    /// Removes fog of war around it for the owner's faction (radius in cells).
    pub vision: i32,
    /// Goods sold outside the world every `economy.export_interval` ticks (money comes in from outside).
    pub exports: Vec<Id>,
    /// Money earned every `economy.export_interval` ticks from visitors (tourism, gambling…).
    pub income: f64,
    /// A stockpile: logistics may take anything it stores, not only what it produces.
    pub stockpile: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapDef {
    pub layers: Vec<LayerDef>,
    #[serde(default)]
    pub zones: Vec<ZoneDef>,
    #[serde(default)]
    pub portals: Vec<PortalDef>,
    /// Infrastructure networks (water, sewers…) linking zones.
    #[serde(default)]
    pub networks: Vec<NetworkDef>,
    /// Impassable rectangles (walls, rivers…): (layer, x, y, width, height).
    #[serde(default)]
    pub walls: Vec<WallDef>,
    /// Maps side by side: walking off a border leads to the neighbouring map.
    #[serde(default)]
    pub edges: Vec<EdgeDef>,
    /// Characters of tile maps (`LayerDef::tiles`): terrain type and whether it can be walked on.
    #[serde(default)]
    pub legend: Vec<TileDef>,
    /// Static decorations drawn by clients (building facades, trees…); they block their footprint.
    #[serde(default)]
    pub props: Vec<PropDef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    North,
    South,
    East,
    West,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeDef {
    /// Map `a`'s `side` touches map `b`'s opposite side.
    pub a: Id,
    pub side: Side,
    pub b: Id,
    /// Shift of `b`'s coordinates along the border.
    #[serde(default)]
    pub offset: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileDef {
    pub ch: char,
    pub id: Id,
    #[serde(default)]
    pub name: String,
    #[serde(default = "yes")]
    pub walkable: bool,
    /// Digging turns this tile into `dig_to` (rock → floor), yielding `yields` (item, quantity).
    #[serde(default)]
    pub dig_to: Option<char>,
    #[serde(default)]
    pub yields: Option<(Id, u32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropDef {
    pub layer: Id,
    pub sprite: String,
    /// Anchor cell (bottom centre of the sprite).
    pub at: (i32, i32),
    /// Blocked cells around the anchor: width centred on it, height upwards. (0, 0) = not solid.
    #[serde(default)]
    pub footprint: (i32, i32),
    /// The anchor stays walkable (a door).
    #[serde(default)]
    pub door: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WallDef {
    pub name: String,
    pub layer: Id,
    pub rect: (i32, i32, i32, i32),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerDef {
    pub id: Id,
    pub name: String,
    /// Size in cells (taken from `tiles` when they are given).
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
    #[serde(default)]
    pub underground: bool,
    #[serde(default)]
    pub indoor: bool,
    /// Depth level (0 = surface, -1, -2… underground), for clients.
    #[serde(default)]
    pub depth: i32,
    /// Tags of the map itself (every map is also a zone with its own id).
    #[serde(default)]
    pub tags: Vec<String>,
    /// Tile rows, one character per cell (see `MapDef::legend`).
    #[serde(default)]
    pub tiles: Vec<String>,
    /// File with the tile rows, relative to the pack folder (read by the loader).
    #[serde(default)]
    pub tiles_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneDef {
    pub id: Id,
    pub name: String,
    pub layer: Id,
    /// x, y, width, height.
    pub rect: (i32, i32, i32, i32),
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortalDef {
    pub name: String,
    pub a: (Id, i32, i32),
    pub b: (Id, i32, i32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkDef {
    pub id: Id,
    pub name: String,
    pub zones: Vec<Id>,
    /// Vector the network carries (usually Water).
    #[serde(default = "water")]
    pub vector: Vector,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EntityTemplate {
    pub id: Id,
    pub name: String,
    pub description: String,
    pub race: Id,
    pub classes: Vec<Id>,
    pub faction: Option<Id>,
    pub rank: Option<Id>,
    pub stats: BTreeMap<Id, f32>,
    pub items: BTreeMap<Id, u32>,
    pub statuses: Vec<Id>,
    pub tags: Vec<String>,
    pub money: f64,
    /// Cannot die.
    pub immortal: bool,
    /// Only one may exist.
    pub unique: bool,
    /// Has no body or position (e.g. an AI living in the network).
    pub virtual_entity: bool,
    /// Extra actions on top of those from classes.
    pub actions: Vec<Id>,
    /// Inventory slot limit override.
    pub slots: Option<u32>,
    /// Starts disguised as this template's appearance (race, faction, name).
    pub disguise: Option<Disguise>,
    pub cover: Option<f32>,
    /// Titles held at spawn.
    pub titles: Vec<Id>,
    /// Fixed sex; when missing it is drawn at spawn (see `population.nonbinary_share`).
    pub sex: Option<crate::stats::Sex>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Disguise {
    pub race: Option<Id>,
    pub faction: Option<Id>,
    pub name: Option<String>,
}

/// Initial population and buildings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Placement {
    Entity {
        template: Id,
        #[serde(default = "one_u32")]
        count: u32,
        zone: Id,
        #[serde(default)]
        at: Option<(i32, i32)>,
        /// Name override (single entity).
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        faction: Option<Id>,
        #[serde(default)]
        tether: Option<Tether>,
        /// Player-controlled leader of this player id.
        #[serde(default)]
        leader_of: Option<Id>,
    },
    Building {
        building: Id,
        zone: Id,
        #[serde(default)]
        at: Option<(i32, i32)>,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        owner_faction: Option<Id>,
        #[serde(default)]
        owner_template: Option<Id>,
    },
    Player {
        id: Id,
        name: String,
        faction: Id,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Tether {
    pub zone: Id,
    /// Released when this becomes true (evaluated with the tethered pawn as subject).
    pub release: Condition,
}

/// Something that happens by itself: when `when` holds, every `every` ticks (0 = once), from `from_tick`.
/// With `by`, only while a pawn of that template is alive, and that pawn does the effects (the summoner, the
/// author of the news…). Spawning (`Spawn`), news (`Publish`, in turn with `Cycle`) and any other effect.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EventDef {
    pub id: Id,
    pub name: String,
    pub description: String,
    pub when: Condition,
    pub every: u64,
    pub from_tick: u64,
    pub by: Option<Id>,
    pub effects: Vec<Effect>,
    /// Newsworthiness of the event (named events are written in the log).
    pub news: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CollectionDef {
    pub id: Id,
    pub name: String,
    pub description: String,
    pub items: Vec<Id>,
    /// Alternatively: any `count` distinct items with this tag.
    pub tag: Option<String>,
    pub count: u32,
    /// Counted over a whole faction (true) or a single pawn (false).
    pub per_faction: bool,
    pub victory_points: i64,
    pub bonus: Vec<Effect>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TitleDef {
    pub id: Id,
    pub name: String,
    pub description: String,
    /// Faction the role belongs to (empty: a role of the whole world, like a mayor).
    pub faction: Id,
    /// How the role changes hands.
    pub mode: TitleMode,
    /// Zone of the throne (`Seat`): when vacant, the first eligible pawn standing here claims it.
    pub seat_zone: Id,
    /// Requirements to hold the role (to claim it, to be a candidate, to win a challenge).
    pub claim_requires: Condition,
    /// Only members of `faction` can hold it.
    pub members_only: bool,
    /// Candidate score: stat → weight (the key "money" weighs the wallet). Empty: the requirements' stats.
    pub score: BTreeMap<Id, f32>,
    /// Ticks a vacancy lasts before the role is assigned (campaign, mourning…). Not for `Seat`.
    pub vacancy_ticks: u64,
    /// Ticks of a mandate: at the end a new election (or appointment) is held. 0 = for life.
    pub term: u64,
    /// When the holder loses the role (besides death, challenges and the end of a mandate). `None`: never.
    pub loses_when: Option<Condition>,
    /// Ticks the loss condition must hold before the role is lost (0 = at once).
    pub grace: u64,
    /// Stat compared in a challenge or a coup (`Challenge`, `Coup`).
    pub challenge_stat: Id,
    /// Supporters within this radius add half their stat (a coup needs friends).
    pub challenge_allies: i32,
    /// Powers of whoever holds it.
    pub stats: BTreeMap<Id, f32>,
    pub tags: Vec<String>,
    pub abilities: Vec<Id>,
    pub actions: Vec<Id>,
    pub victory_points: i64,
    /// A player whose leader claims the title takes control of `faction`.
    pub grants_faction_control: bool,
    pub news: f32,
}

/// How a role (title) is obtained when vacant, and how it can be taken from its holder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TitleMode {
    /// Whoever sits on the seat while it is vacant.
    #[default]
    Seat,
    /// The best eligible member of the faction inherits it.
    Succession,
    /// Members vote: score plus luck; held again at the end of every term.
    Election,
    /// Chosen by the faction's higher-ups among the eligible: the best score.
    Appointment,
    /// Taken by beating the holder (a duel, a drinking contest…); when vacant, the best score.
    Challenge,
    /// Taken by a plot with enough supporters; when vacant, the best score.
    Coup,
    /// Bought: the richest eligible.
    Purchase,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PressDef {
    /// Name of the social feed.
    pub feed_name: String,
    /// Headline per event kind; placeholders {actor} {target} {faction} {message}.
    pub headlines: BTreeMap<String, Vec<String>>,
    pub default_headline: String,
    /// Work units needed to write an article.
    pub publish_work: f32,
    /// Feed categories in priority order: an article goes to the first one sharing a topic with it.
    pub categories: Vec<FeedCategoryDef>,
    /// Category of articles matching none of the above.
    pub default_category: String,
    /// Minimum importance shown by the "important" filter (articles about the reader always pass).
    pub important_threshold: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FeedCategoryDef {
    pub id: String,
    pub name: String,
    pub topics: Vec<String>,
    /// Importance multiplier for the category (e.g. gossip counts less).
    pub weight: Option<f32>,
}

/// How articles move the world, by topic (event kind or tag).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NewsImpactDef {
    pub topic: String,
    /// Market shocks on items with these tags: (tag, demand multiplier, supply multiplier).
    pub market: Vec<(String, f32, f32)>,
    pub duration: u64,
    /// Morale change for every pawn (fake news count at `fake_factor`).
    pub morale: f32,
    /// Reputation change for the subject of the article.
    pub subject_reputation: f32,
    /// Relation change between the article subject's faction and the author's.
    pub relation: f32,
    pub fake_factor: f32,
    /// Cover lost by a disguised subject.
    pub exposes: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum VictoryKind {
    /// A faction holds all these items (anywhere among its members and buildings).
    HoldItems(Vec<Id>),
    /// A player's leader holds this title.
    HoldTitle(Id),
    VictoryPoints(i64),
    Condition(Condition),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VictoryDef {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub kind: VictoryKind,
    #[serde(default)]
    pub primary: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GlobalModifierDef {
    pub id: Id,
    pub name: String,
    pub description: String,
    /// Intensity a start gives (default 100); every tick it changes by `per_tick`; at 0 it is over.
    pub intensity: Option<f32>,
    pub per_tick: f32,
    /// While active: stat modifiers on every pawn.
    pub stats: BTreeMap<Id, f32>,
    /// While active: price multipliers by item tag.
    pub prices: BTreeMap<String, f32>,
    /// While active: outside supply multipliers by item id or tag ("*" = everything).
    pub supply: BTreeMap<String, f32>,
    /// While active: delays in deliveries (0..1).
    pub logistics_disruption: f32,
}

/// Steady flow of goods from outside the world: every `interval` ticks the shops that sell `item` are
/// topped up towards `per_shop` units each, at most `max_per_day`. Local production fills shops first,
/// so it covers a variable share of the total and imports cover the rest. `variation` adds randomness
/// (±fraction), active global modifiers scale the flow, the owner faction pays `cost` × base price.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SupplyDef {
    pub id: Id,
    pub name: String,
    pub item: Id,
    pub per_shop: u32,
    pub max_per_day: u32,
    pub variation: f32,
    pub interval: u64,
    pub cost: f32,
    /// Only shops of buildings with this tag.
    pub shop_tag: Option<String>,
}

/// How the client should draw something. Kept deliberately simple: shape + colors + glyph, or a sheet.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct SpriteDef {
    pub shape: String,
    pub color: String,
    pub outline: String,
    pub glyph: String,
    pub sheet: Option<String>,
    pub frame: Option<u32>,
    pub icon: Option<String>,
}

fn hundred() -> f32 {
    100.0
}
fn yes() -> bool {
    true
}
fn one_f() -> f32 {
    1.0
}
fn one_u32() -> u32 {
    1
}
fn five() -> i32 {
    5
}
fn half() -> f32 {
    0.5
}
fn water() -> Vector {
    Vector::Water
}

/// Every id namespace, used by the validator and the compendium.
pub fn id_set<'a, I: IntoIterator<Item = &'a str>>(ids: I) -> BTreeSet<&'a str> {
    ids.into_iter().collect()
}
