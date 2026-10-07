//! The three small data languages that make the engine data-driven:
//! [`Effect`] (verbs), [`Condition`] (predicates) and [`Curve`] (utility response curves),
//! plus [`Selector`]s that pick entities.

use serde::{Deserialize, Serialize};

/// Who an effect applies to, relative to the context that runs it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum Scope {
    /// The context subject (the actor of a job, the carrier of a status, the user of an item…).
    #[default]
    Subject,
    /// The context target (the victim of a job, the building being damaged…).
    Target,
    /// Every member of the subject's (true) faction.
    SubjectFaction,
    /// Every member of the target's faction.
    TargetFaction,
    /// Every living pawn.
    Everyone,
    /// Every living pawn inside a zone.
    Zone(String),
    /// Every living pawn within this many cells of the subject.
    Radius(i32),
    /// Every living pawn whose template id matches.
    Template(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Truth {
    #[default]
    Real,
    Propaganda,
    Fake,
}

/// A data-defined action on the simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    /// Re-target the inner effect.
    On(Scope, Box<Effect>),
    All(Vec<Effect>),
    Chance(f32, Box<Effect>),
    If(Condition, Box<Effect>),
    IfElse(Condition, Box<Effect>, Box<Effect>),

    ModStat { stat: String, amount: f32 },
    SetStat { stat: String, value: f32 },
    ModNeed { need: String, amount: f32 },
    ApplyStatus { status: String, #[serde(default = "one")] severity: f32 },
    RemoveStatus(String),
    /// Removes every status carrying this tag (e.g. a broad antidote).
    RemoveStatusTag(String),
    /// Grants immunity to a status for a number of ticks (0 = permanent).
    Immunize { status: String, #[serde(default)] ticks: u64 },
    AddTag(String),
    RemoveTag(String),
    GiveItem { item: String, #[serde(default = "one_u32")] qty: u32 },
    TakeItem { item: String, #[serde(default = "one_u32")] qty: u32 },
    /// One item drawn at random from `items` or from every item carrying `tag` (a pack of collectibles).
    GiveRandomItem { #[serde(default)] items: Vec<String>, #[serde(default)] tag: Option<String>, #[serde(default = "one_u32")] qty: u32 },
    ModMoney(f64),
    /// Adds to (or takes from) the treasury of the subject's faction.
    ModTreasury(f64),
    /// Changes the relation between the subject's faction and another (both directions).
    ModRelation { faction: String, amount: f32 },
    AddWanted { amount: f32, #[serde(default)] crime: String },
    ClearWanted,
    Damage { amount: f32, #[serde(default)] part: Option<String> },
    Heal(f32),
    Kill,
    Transmute(String),
    AddClass(String),
    RemoveClass(String),
    JoinFaction(String),
    VictoryPoints(i64),
    ModDissent(f32),
    Spawn { template: String, #[serde(default = "one_u32")] count: u32, #[serde(default)] zone: Option<String> },
    MarketShock {
        #[serde(default)] item: Option<String>,
        #[serde(default)] tag: Option<String>,
        #[serde(default = "one")] demand: f32,
        #[serde(default = "one")] supply: f32,
        duration: u64,
    },
    /// Activates a named global modifier (market disruption, morale…) for some ticks.
    GlobalModifier { id: String, #[serde(default)] name: String, duration: u64, #[serde(default)] logistics_disruption: f32, #[serde(default)] morale: f32 },
    SetFlag { flag: String, value: f64 },
    ModFlag { flag: String, amount: f64 },
    Publish { headline: String, #[serde(default)] truth: Truth, #[serde(default)] topics: Vec<String> },
    Emit { kind: String, message: String, #[serde(default)] news: f32 },
    ModCover(f32),
    Expose,
    Stealth { amount: f32, duration: u64 },
    ResetAggro,
    Shapeshift { #[serde(default)] race: Option<String>, #[serde(default)] faction: Option<String>, #[serde(default)] name: Option<String> },
    RevertForm,
    ReleaseTether,
    Contaminate { status: String, load: f32 },
    Spill { fluid: String, amount: f32 },
    Clean(f32),
    DamageBuilding(f32),
    RepairBuilding(f32),
    Teleport { zone: String },
    PostJob { job: String, #[serde(default)] priority: i32 },
    Log(String),
    /// Friendship between subject and target (both ways) and the target's attraction to the subject.
    ModBond { #[serde(default)] friendship: f32, #[serde(default)] attraction: f32 },
    /// Turns the subject into another race for a while, then back.
    TransmuteFor { race: String, ticks: u64 },
    /// A line in the subject's journal ({subject} and {target} are replaced).
    Note(String),
    /// Takes a role (title) if the subject meets its requirements and it is vacant.
    ClaimTitle(String),
    /// The subject leaves a role it holds.
    LeaveTitle(String),
    /// The subject tries to take a role from its holder: `stat` plus luck against the holder's (allies of
    /// each side within `allies_radius` add half of theirs: a coup). The winner keeps or takes the role.
    ChallengeTitle { title: String, stat: String, #[serde(default)] allies_radius: i32 },
    /// Implemented by a plugin (`SimBuilder::register_effect`).
    Custom { id: String, #[serde(default)] params: serde_json::Value },
}

fn one() -> f32 {
    1.0
}
fn one_u32() -> u32 {
    1
}

/// A data-defined predicate, evaluated against a subject/target context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum Condition {
    #[default]
    Always,
    Never,
    All(Vec<Condition>),
    Any(Vec<Condition>),
    Not(Box<Condition>),
    /// Evaluate the inner condition on the context target instead of the subject.
    OnTarget(Box<Condition>),
    TickAtLeast(u64),
    Every(u64),
    Chance(f32),
    Flag { flag: String, min: f64 },
    StatAtLeast { stat: String, value: f32 },
    StatBelow { stat: String, value: f32 },
    NeedBelow { need: String, value: f32 },
    HasStatus(String),
    HasStatusTag(String),
    HasTag(String),
    HasItem { item: String, #[serde(default = "one_u32")] qty: u32 },
    HasItemTag(String),
    MoneyAtLeast(f64),
    InZone(String),
    MemberOf(String),
    IsRace(String),
    /// The subject's (true) race is one of these.
    RaceIn(Vec<String>),
    HasClass(String),
    WantedAtLeast(f32),
    Detained,
    Disguised,
    TemplateDead(String),
    TemplateAlive(String),
    BuildingHpBelow { building: String, ratio: f32 },
    ZoneOccupied { zone: String, #[serde(default)] faction: Option<String>, #[serde(default = "one_u32")] min: u32 },
    /// Fewer than `count` living pawns (optionally of a faction and/or template).
    PopulationBelow { #[serde(default)] faction: Option<String>, #[serde(default)] template: Option<String>, count: u32 },
    /// Subject building HP ratio (or pawn health ratio) below this value.
    HpBelow(f32),
    TitleVacant(String),
    TreasuryAtLeast { faction: String, amount: f64 },
    FactionHoldsItems { faction: String, items: Vec<String> },
    VictoryPointsAtLeast(i64),
    ArticlesAtLeast(u32),
    /// Bond of the subject towards the target: friendship and attraction at least these values.
    BondAtLeast { #[serde(default)] friendship: f32, #[serde(default)] attraction: f32 },
    /// `stat` of the subject plus luck beats the target's (a brawl, a duel). `luck` 0..1, default 0.5.
    Contest { stat: String, #[serde(default)] luck: Option<f32> },
    /// The subject carries an item of this kind (see `ItemDef::types`).
    HasItemType(String),
    /// The subject holds this role (title).
    HoldsTitle(String),
    /// The subject holds any role.
    HoldsAnyTitle,
    IsSex(crate::stats::Sex),
    /// At least `count` active statuses carrying `tag` (e.g. three illnesses at once).
    StatusTagCount { tag: String, count: u32 },
    Custom { id: String, #[serde(default)] params: serde_json::Value },
}

/// Response curve mapping an input (normalized to 0..1) to a utility score in 0..1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum Curve {
    #[default]
    Identity,
    Constant(f32),
    /// `1 - x`.
    Inverse,
    Linear { slope: f32, intercept: f32 },
    /// `slope * (x - shift)^exponent + intercept`.
    Power { exponent: f32, #[serde(default = "one")] slope: f32, #[serde(default)] shift: f32, #[serde(default)] intercept: f32 },
    Logistic { steepness: f32, midpoint: f32 },
    /// 1 when x >= threshold, else 0 (or the reverse with `below: true`).
    Step { threshold: f32, #[serde(default)] below: bool },
}

impl Curve {
    pub fn eval(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        let y = match self {
            Curve::Identity => x,
            Curve::Constant(c) => *c,
            Curve::Inverse => 1.0 - x,
            Curve::Linear { slope, intercept } => slope * x + intercept,
            Curve::Power { exponent, slope, shift, intercept } => {
                slope * (x - shift).max(0.0).powf(*exponent) + intercept
            }
            Curve::Logistic { steepness, midpoint } => {
                1.0 / (1.0 + (-steepness * (x - midpoint)).exp())
            }
            Curve::Step { threshold, below } => {
                if (x >= *threshold) != *below { 1.0 } else { 0.0 }
            }
        };
        if y.is_nan() { 0.0 } else { y.clamp(0.0, 1.0) }
    }
}

/// Which pawn/building an AI action or job points at.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum Selector {
    /// No target: the job happens where the actor stands.
    #[default]
    None,
    SelfTarget,
    /// Nearest entity matching the filter (ties broken by id).
    Nearest(Filter),
    /// A random entity matching the filter.
    Random(Filter),
    /// A random cell inside a zone (or inside any zone with the tag, with `Zone("#tag")`).
    Zone(String),
    /// A random walkable cell within this many cells, on the chooser's own map (a stroll).
    Nearby(i32),
    /// The zone that holds the entity's post (e.g. the throne or a patrol route of its faction).
    OwnFactionZone(String),
}

/// Filter over entities used by selectors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Filter {
    /// Only pawns (true) or only buildings (false); `None` accepts both.
    pub pawn: Option<bool>,
    pub tags_any: Vec<String>,
    pub tags_all: Vec<String>,
    pub tags_none: Vec<String>,
    pub building: Option<String>,
    pub building_tag: Option<String>,
    pub template: Option<String>,
    /// Relation of the *apparent* faction of the candidate to the chooser's faction.
    pub relation: Option<Relation>,
    pub faction: Option<String>,
    pub has_item_tag: Option<String>,
    /// Buildings whose shop has in stock an item with this tag.
    pub sells_tag: Option<String>,
    pub min_money: Option<f64>,
    /// Minimum wanted level; a string key refers to a parameter (e.g. "crime.arrest_threshold").
    pub min_wanted: Option<Threshold>,
    pub has_status: Option<String>,
    pub max_distance: Option<i32>,
    /// Only candidates the chooser can currently perceive (stealth check).
    pub visible: bool,
    pub not_detained: bool,
    pub condition: Option<Condition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Threshold {
    Value(f32),
    Param(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Relation {
    Same,
    Other,
    Hostile,
    Friendly,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves_are_clamped() {
        assert_eq!(Curve::Inverse.eval(0.25), 0.75);
        assert_eq!(Curve::Linear { slope: 2.0, intercept: 0.0 }.eval(0.9), 1.0);
        assert_eq!(Curve::Step { threshold: 0.5, below: true }.eval(0.2), 1.0);
        let l = Curve::Logistic { steepness: 10.0, midpoint: 0.5 };
        assert!((l.eval(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn effects_parse_from_ron() {
        let e: Effect = ron::from_str(
            r#"All([On(Target, ApplyStatus(status: "drunk")), Chance(0.5, ModMoney(-3.0)), Custom(id: "x", params: {"a": 1})])"#,
        )
        .unwrap();
        match e {
            Effect::All(v) => assert_eq!(v.len(), 3),
            _ => panic!(),
        }
    }
}
