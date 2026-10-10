use std::collections::BTreeMap;
use std::sync::Arc;

use bevy_ecs::prelude::*;
use serde::Serialize;

use super::defs::*;
use super::loader::ContentError;
use super::logic::{Condition, Effect, Filter, Selector};

/// All loaded content, indexed by id. Inserted as a resource; read by every system. Cloning is cheap
/// (shared, immutable data), so systems clone it to release the world borrow.
#[derive(Resource, Debug, Clone, Default)]
pub struct Content(Arc<ContentData>);

impl std::ops::Deref for Content {
    type Target = ContentData;
    fn deref(&self) -> &ContentData {
        &self.0
    }
}

impl Serialize for Content {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ContentData {
    pub packs: Vec<PackMeta>,
    pub bindings: Bindings,
    pub params: BTreeMap<String, f64>,
    pub stats: BTreeMap<Id, StatDef>,
    pub needs: BTreeMap<Id, NeedDef>,
    pub body_plans: BTreeMap<Id, BodyPlanDef>,
    pub races: BTreeMap<Id, RaceDef>,
    pub action_sets: BTreeMap<Id, ActionSetDef>,
    pub modes: BTreeMap<Id, ModeDef>,
    pub classes: BTreeMap<Id, ClassDef>,
    pub statuses: BTreeMap<Id, StatusDef>,
    pub fluids: BTreeMap<Id, FluidDef>,
    pub items: BTreeMap<Id, ItemDef>,
    pub abilities: BTreeMap<Id, AbilityDef>,
    pub jobs: BTreeMap<Id, JobDef>,
    pub actions: BTreeMap<Id, ActionDef>,
    pub factions: BTreeMap<Id, FactionDef>,
    pub buildings: BTreeMap<Id, BuildingDef>,
    pub map: Option<MapDef>,
    pub templates: BTreeMap<Id, EntityTemplate>,
    pub placements: Vec<Placement>,
    pub events: BTreeMap<Id, EventDef>,
    pub collections: BTreeMap<Id, CollectionDef>,
    pub titles: BTreeMap<Id, TitleDef>,
    pub press: PressDef,
    pub news_impacts: Vec<NewsImpactDef>,
    pub victory: Vec<VictoryDef>,
    pub global_modifiers: BTreeMap<Id, GlobalModifierDef>,
    pub supplies: BTreeMap<Id, SupplyDef>,
    pub sprites: BTreeMap<Id, SpriteDef>,
    pub dialogue: BTreeMap<String, Vec<String>>,
}

fn merge<T, F: Fn(&T) -> &str>(dst: &mut BTreeMap<Id, T>, src: Vec<T>, id: F) {
    for v in src {
        dst.insert(id(&v).to_string(), v);
    }
}

/// Actions that say how they are done (`how`) become the job with the action's id.
fn action_jobs(c: &mut ContentData) {
    for a in c.actions.values_mut() {
        if let Some(how) = &a.how {
            c.jobs.insert(a.id.clone(), how.to_job(&a.id, &a.name, &a.description));
            if let ActionKind::Job { job, .. } = &mut a.kind
                && job.is_empty()
            {
                *job = a.id.clone();
            }
        }
    }
}

/// Every need names its stat; a missing one is created (0..100, full at birth, group "Bisogni").
fn need_stats(c: &mut ContentData) {
    for n in c.needs.values_mut() {
        if n.stat.is_empty() {
            n.stat = n.id.clone();
        }
        if !c.stats.contains_key(&n.stat) {
            c.stats.insert(n.stat.clone(), StatDef {
                id: n.stat.clone(), name: n.name.clone(), description: format!("Bisogno: {}", n.name), min: 0.0, max: 100.0,
                default: 100.0, visible: true, rest_value: None, recovery: 0.0, per_day: 0.0, group: "Bisogni".into(), spread: 0.0,
            });
        }
    }
}

/// Kinds of definitions (keyed by id) the admin console can replace while the game runs.
pub const EDITABLE_KINDS: &[&str] = &[
    "stats", "needs", "body_plans", "races", "action_sets", "modes", "classes", "statuses", "fluids", "items", "abilities", "jobs", "actions", "factions",
    "buildings", "templates", "events", "collections", "titles", "global_modifiers", "supplies",
];

/// Adds one pack on top of the content merged so far (same id = replaced).
fn merge_pack(c: &mut ContentData, p: ContentPack) {
    if !p.meta.id.is_empty() {
        c.packs.push(p.meta.clone());
    }
    if let Some(b) = p.bindings {
        c.bindings = b;
    }
    c.params.extend(p.params);
    merge(&mut c.stats, p.stats, |d| &d.id);
    merge(&mut c.needs, p.needs, |d| &d.id);
    merge(&mut c.body_plans, p.body_plans, |d| &d.id);
    merge(&mut c.races, p.races, |d| &d.id);
    merge(&mut c.action_sets, p.action_sets, |d| &d.id);
    merge(&mut c.modes, p.modes, |d| &d.id);
    merge(&mut c.classes, p.classes, |d| &d.id);
    merge(&mut c.statuses, p.statuses, |d| &d.id);
    merge(&mut c.fluids, p.fluids, |d| &d.id);
    merge(&mut c.items, p.items, |d| &d.id);
    merge(&mut c.abilities, p.abilities, |d| &d.id);
    merge(&mut c.jobs, p.jobs, |d| &d.id);
    merge(&mut c.actions, p.actions, |d| &d.id);
    merge(&mut c.factions, p.factions, |d| &d.id);
    merge(&mut c.buildings, p.buildings, |d| &d.id);
    if let Some(m) = p.map {
        match &mut c.map {
            None => c.map = Some(m),
            Some(cur) => {
                cur.layers.extend(m.layers);
                cur.zones.extend(m.zones);
                cur.portals.extend(m.portals);
                cur.networks.extend(m.networks);
                cur.walls.extend(m.walls);
                cur.edges.extend(m.edges);
                cur.legend.extend(m.legend);
                cur.props.extend(m.props);
            }
        }
    }
    merge(&mut c.templates, p.templates, |d| &d.id);
    c.placements.extend(p.placements);
    merge(&mut c.events, p.events, |d| &d.id);
    merge(&mut c.collections, p.collections, |d| &d.id);
    merge(&mut c.titles, p.titles, |d| &d.id);
    if let Some(press) = p.press {
        c.press = press;
    }
    c.news_impacts.extend(p.news_impacts);
    c.victory.extend(p.victory);
    merge(&mut c.global_modifiers, p.global_modifiers, |d| &d.id);
    merge(&mut c.supplies, p.supplies, |d| &d.id);
    c.sprites.extend(p.sprites);
    for (k, lines) in p.dialogue {
        c.dialogue.entry(k).or_default().extend(lines);
    }
}

impl Content {
    /// The content with one definition added or replaced (`kind` is one of [`EDITABLE_KINDS`], `def`
    /// its JSON form), validated like a fresh load.
    pub fn with_def(&self, kind: &str, def: serde_json::Value) -> Result<Content, ContentError> {
        if !EDITABLE_KINDS.contains(&kind) {
            return Err(ContentError::Invalid(vec![format!("i contenuti di tipo '{kind}' non si modificano da qui")]));
        }
        let pack: ContentPack = serde_json::from_value(serde_json::json!({ kind: [def] })).map_err(|e| ContentError::Invalid(vec![format!("{kind}: {e}")]))?;
        let mut c = (*self.0).clone();
        merge_pack(&mut c, pack);
        need_stats(&mut c);
        action_jobs(&mut c);
        let errors = c.validate();
        if errors.is_empty() { Ok(Content(Arc::new(c))) } else { Err(ContentError::Invalid(errors)) }
    }
}

impl Content {
    /// Merges packs in order (later definitions with the same id replace earlier ones) and validates
    /// every cross reference.
    pub fn from_packs(packs: Vec<ContentPack>) -> Result<Self, ContentError> {
        let mut c = ContentData::default();
        for p in packs {
            merge_pack(&mut c, p);
        }
        need_stats(&mut c);
        action_jobs(&mut c);
        if let Some(m) = c.map.as_mut() {
            for l in m.layers.iter_mut() {
                if !l.tiles.is_empty() {
                    l.height = l.tiles.len() as i32;
                    l.width = l.tiles.iter().map(|r| r.chars().count()).max().unwrap_or(0) as i32;
                }
            }
            // Every map is also a zone with its own id and tags.
            let extra: Vec<ZoneDef> = m
                .layers
                .iter()
                .filter(|l| !m.zones.iter().any(|z| z.id == l.id))
                .map(|l| ZoneDef { id: l.id.clone(), name: l.name.clone(), layer: l.id.clone(), rect: (0, 0, l.width, l.height), tags: l.tags.clone() })
                .collect();
            m.zones.extend(extra);
        }
        let errors = c.validate();
        if errors.is_empty() { Ok(Content(Arc::new(c))) } else { Err(ContentError::Invalid(errors)) }
    }
}

impl ContentData {

    /// The stat a need moves.
    pub fn need_stat<'a>(&'a self, need: &'a str) -> &'a str {
        self.needs.get(need).map_or(need, |n| n.stat.as_str())
    }

    /// How satisfied a need is, 0..1 (its stat within the stat's range).
    pub fn need_level(&self, stats: &crate::stats::Stats, need: &str) -> f32 {
        let stat = self.need_stat(need);
        let (lo, hi) = self.stat_bounds(stat);
        let v = stats.base.get(stat).copied().unwrap_or(hi);
        if hi > lo && hi < f32::MAX { ((v - lo) / (hi - lo)).clamp(0.0, 1.0) } else { 1.0 }
    }

    pub fn stat_bounds(&self, stat: &str) -> (f32, f32) {
        self.stats.get(stat).map_or((f32::MIN, f32::MAX), |s| (s.min, s.max))
    }

    /// Bounds of a stat for members of a race: the stat's own, narrowed by the race's `stat_ranges`.
    pub fn race_stat_bounds(&self, race: &str, stat: &str) -> (f32, f32) {
        let (lo, hi) = self.stat_bounds(stat);
        match self.races.get(race).and_then(|r| r.stat_ranges.get(stat)) {
            Some(r) => (r.min.map_or(lo, |m| m.max(lo)), r.max.map_or(hi, |m| m.min(hi))),
            None => (lo, hi),
        }
    }

    pub fn zone(&self, id: &str) -> Option<&ZoneDef> {
        self.map.as_ref()?.zones.iter().find(|z| z.id == id)
    }

    pub fn rank<'a>(&'a self, faction: &str, rank: &str) -> Option<&'a RankDef> {
        self.factions.get(faction)?.ranks.iter().find(|r| r.id == rank)
    }

    /// Lowest-level rank of a faction (new recruits).
    pub fn base_rank(&self, faction: &str) -> Option<&RankDef> {
        self.factions.get(faction)?.ranks.iter().filter(|r| !r.unique).min_by_key(|r| r.level)
    }

    fn validate(&self) -> Vec<String> {
        let mut v = Validator { c: self, errors: Vec::new() };
        v.run();
        v.errors
    }
}

struct Validator<'a> {
    c: &'a ContentData,
    errors: Vec<String>,
}

impl Validator<'_> {
    fn check<T>(&mut self, map: &BTreeMap<Id, T>, kind: &str, id: &str, ctx: &str) {
        if !map.contains_key(id) {
            self.errors.push(format!("{ctx}: {kind} '{id}' non esiste"));
        }
    }

    fn zone(&mut self, id: &str, ctx: &str) {
        if let Some(tag) = id.strip_prefix('#') {
            let ok = self.c.map.as_ref().is_some_and(|m| m.zones.iter().any(|z| z.tags.iter().any(|t| t == tag)));
            if !ok {
                self.errors.push(format!("{ctx}: nessuna zona con il tag '{tag}'"));
            }
        } else if self.c.zone(id).is_none() {
            self.errors.push(format!("{ctx}: zona '{id}' non esiste"));
        }
    }

    fn stat(&mut self, id: &str, ctx: &str) {
        let c = self.c;
        self.check(&c.stats, "statistica", id, ctx);
    }

    fn run(&mut self) {
        let c = self.c;
        for s in c.supplies.values() {
            if !c.items.contains_key(&s.item) {
                self.errors.push(format!("rifornimento {}: oggetto '{}' non esiste", s.id, s.item));
            }
        }
        let b = &c.bindings;
        for s in [&b.morale, &b.perception, &b.stealth, &b.speed, &b.strength, &b.heroism, &b.press_reputation] {
            if !c.stats.is_empty() {
                self.stat(s, "bindings");
            }
        }
        for n in c.needs.values() {
            let ctx = format!("bisogno {}", n.id);
            self.stat(&n.stat, &ctx);
            for t in &n.thresholds {
                self.effects(&t.effects, &ctx);
                for k in t.stats.keys() {
                    self.stat(k, &ctx);
                }
            }
        }
        for bp in c.body_plans.values() {
            for p in &bp.parts {
                if let Some(parent) = &p.parent
                    && !bp.parts.iter().any(|q| &q.id == parent) {
                        self.errors.push(format!("piano corporeo {}: parte padre '{parent}' non esiste", bp.id));
                    }
            }
        }
        for m in c.modes.values() {
            let ctx = format!("modalità {}", m.id);
            for s in m.stats.keys() {
                self.stat(s, &ctx);
            }
            self.cond(&m.when, &ctx);
        }
        if let Some(m) = &c.bindings.default_mode {
            self.check(&c.modes, "modalità", m, "bindings");
        }
        for f in c.factions.values() {
            for r in &f.ranks {
                let ctx = format!("rango {} di {}", r.id, f.id);
                if let Some(q) = &r.requires {
                    self.cond(q, &ctx);
                }
                for s in r.stats.keys() {
                    self.stat(s, &ctx);
                }
                for a in &r.abilities {
                    self.check(&c.abilities, "abilità", a, &ctx);
                }
                for a in &r.actions {
                    self.check(&c.actions, "azione", a, &ctx);
                }
            }
            if let Some(m) = &f.mode {
                self.check(&c.modes, "modalità", m, &format!("fazione {}", f.id));
            }
        }
        for s in c.action_sets.values() {
            for a in &s.actions {
                self.check(&c.actions, "azione", a, &format!("gruppo di azioni {}", s.id));
            }
        }
        for r in c.races.values() {
            let ctx = format!("razza {}", r.id);
            self.check(&c.body_plans, "piano corporeo", &r.body_plan, &ctx);
            for s in &r.action_sets {
                self.check(&c.action_sets, "gruppo di azioni", s, &ctx);
            }
            for a in &r.actions {
                self.check(&c.actions, "azione", a, &ctx);
            }
            for s in r.stat_ranges.keys() {
                self.stat(s, &ctx);
            }
            for (s, range) in &r.stat_ranges {
                if let (Some(lo), Some(hi)) = (range.min, range.max)
                    && lo > hi {
                        self.errors.push(format!("{ctx}: {s} ha il minimo ({lo}) sopra il massimo ({hi})"));
                    }
            }
            for n in r.needs_exempt.iter().chain(r.need_rates.keys()) {
                self.check(&c.needs, "bisogno", n, &ctx);
            }
            for a in &r.abilities {
                self.check(&c.abilities, "abilità", a, &ctx);
            }
            for s in r.innate_statuses.iter().chain(&r.immunities) {
                self.check(&c.statuses, "status", s, &ctx);
            }
        }
        for it in c.items.values() {
            let ctx = format!("oggetto {}", it.id);
            self.cond(&it.requires, &ctx);
            self.effects(&it.on_hit, &ctx);
            for s in it.carried_stats.keys() {
                self.stat(s, &ctx);
            }
        }
        for s in [&c.bindings.attack, &c.bindings.defense].into_iter().flatten() {
            self.stat(s, "bindings");
        }
        for cl in c.classes.values() {
            let ctx = format!("classe {}", cl.id);
            for s in cl.stats.keys() {
                self.stat(s, &ctx);
            }
            for a in &cl.abilities {
                self.check(&c.abilities, "abilità", a, &ctx);
            }
            for a in &cl.actions {
                self.check(&c.actions, "azione", a, &ctx);
            }
            for x in &cl.replaces {
                self.check(&c.classes, "classe", x, &ctx);
            }
            if let Some(req) = &cl.requires {
                self.cond(req, &ctx);
            }
            if let Some(lose) = &cl.loses_when {
                self.cond(lose, &ctx);
            }
        }
        if let Some(s) = &c.bindings.crime_record {
            self.stat(s, "bindings");
        }
        if let Some(cl) = &c.bindings.default_class {
            self.check(&c.classes, "classe", cl, "bindings");
        }
        for s in c.statuses.values() {
            let ctx = format!("status {}", s.id);
            for k in s.stats.keys().chain(s.thresholds.iter().flat_map(|st| st.stats.keys())).chain(s.resist_stat.iter()) {
                self.stat(k, &ctx);
            }
            for k in s.need_rates.keys().chain(s.thresholds.iter().flat_map(|st| st.need_rates.keys())) {
                self.check(&c.needs, "bisogno", k, &ctx);
            }
            self.effects(&s.on_apply, &ctx);
            self.effects(&s.effects, &ctx);
            self.effects(&s.on_expire, &ctx);
            for st in &s.thresholds {
                self.effects(&st.on_enter, &ctx);
                self.effects(&st.effects, &ctx);
            }
        }
        for f in c.fluids.values() {
            if let Some(s) = &f.carries {
                self.check(&c.statuses, "status", s, &format!("fluido {}", f.id));
            }
        }
        for i in c.items.values() {
            let ctx = format!("oggetto {}", i.id);
            self.effects(&i.on_use, &ctx);
            for s in i.carried_stats.keys() {
                self.stat(s, &ctx);
            }
        }
        for a in c.abilities.values() {
            let ctx = format!("abilità {}", a.id);
            for s in a.stats.keys().chain(a.aura.iter().flat_map(|x| x.stats.keys().chain(x.stats_per_tick.keys()))) {
                self.stat(s, &ctx);
            }
            for n in a.need_rates.keys() {
                self.check(&c.needs, "bisogno", n, &ctx);
            }
            for s in &a.immunities {
                self.check(&c.statuses, "status", s, &ctx);
            }
            if let Some(aura) = &a.aura {
                self.cond(&aura.affects, &ctx);
            }
        }
        for j in c.jobs.values() {
            let ctx = format!("job {}", j.id);
            if let Some(s) = &j.skill {
                self.stat(s, &ctx);
            }
            self.cond(&j.requires, &ctx);
            self.effects(&j.effects, &ctx);
        }
        for a in c.actions.values() {
            let ctx = format!("azione {}", a.id);
            match &a.kind {
                ActionKind::Job { job, target } => {
                    self.check(&c.jobs, "job", job, &ctx);
                    self.selector(target, &ctx);
                }
                ActionKind::Effects { target, effects, .. } => {
                    self.effects(effects, &ctx);
                    self.selector(target, &ctx);
                }
                ActionKind::Work | ActionKind::Idle => {}
            }
            self.cond(&a.requires, &ctx);
            for cons in &a.considerations {
                use super::defs::Input as I;
                match &cons.input {
                    I::Need(n) => self.check(&c.needs, "bisogno", n, &ctx),
                    I::Stat { stat, .. } => self.stat(stat, &ctx),
                    I::StatusSeverity { status, .. } | I::HasStatus(status) => {
                        self.check(&c.statuses, "status", status, &ctx)
                    }
                    I::Condition(cond) => self.cond(cond, &ctx),
                    _ => {}
                }
            }
        }
        for f in c.factions.values() {
            let ctx = format!("fazione {}", f.id);
            for other in f.relations.keys() {
                self.check(&c.factions, "fazione", other, &ctx);
            }
            for z in &f.zones {
                self.zone(z, &ctx);
            }
            for g in &f.goals {
                self.check(&c.jobs, "job", &g.job, &ctx);
                self.cond(&g.requires, &ctx);
            }
        }
        for b in c.buildings.values() {
            let ctx = format!("edificio {}", b.id);
            for r in &b.productions {
                for i in r.inputs.keys().chain(r.outputs.keys()) {
                    self.check(&c.items, "oggetto", i, &ctx);
                }
            }
            for i in b.sells.keys().chain(b.stock.keys()).chain(b.cost.keys()) {
                self.check(&c.items, "oggetto", i, &ctx);
            }
            for i in &b.exports {
                self.check(&c.items, "oggetto", i, &ctx);
            }
            for dc in &b.consequences {
                self.effects(&dc.effects, &ctx);
            }
        }
        if let Some(m) = &c.map {
            for z in &m.zones {
                if !m.layers.iter().any(|l| l.id == z.layer) {
                    self.errors.push(format!("zona {}: livello '{}' non esiste", z.id, z.layer));
                }
            }
            for p in &m.portals {
                for (l, _, _) in [&p.a, &p.b] {
                    if !m.layers.iter().any(|x| &x.id == l) {
                        self.errors.push(format!("portale {}: livello '{l}' non esiste", p.name));
                    }
                }
            }
            for e in &m.edges {
                for l in [&e.a, &e.b] {
                    if !m.layers.iter().any(|x| &x.id == l) {
                        self.errors.push(format!("bordo {}-{}: mappa '{l}' non esiste", e.a, e.b));
                    }
                }
            }
            for p in &m.props {
                if !m.layers.iter().any(|x| x.id == p.layer) {
                    self.errors.push(format!("decorazione {}: mappa '{}' non esiste", p.sprite, p.layer));
                }
            }
            for l in &m.layers {
                for row in &l.tiles {
                    for ch in row.chars() {
                        if !m.legend.iter().any(|t| t.ch == ch) {
                            self.errors.push(format!("mappa {}: carattere '{ch}' non è nella legenda", l.id));
                            break;
                        }
                    }
                }
            }
            for w in &m.walls {
                if !m.layers.iter().any(|l| l.id == w.layer) {
                    self.errors.push(format!("muro {}: livello '{}' non esiste", w.name, w.layer));
                }
            }
            for n in &m.networks {
                for z in &n.zones {
                    self.zone(z, &format!("rete {}", n.id));
                }
            }
        }
        for t in c.templates.values() {
            let ctx = format!("template {}", t.id);
            self.check(&c.races, "razza", &t.race, &ctx);
            for cl in &t.classes {
                self.check(&c.classes, "classe", cl, &ctx);
            }
            if let Some(f) = &t.faction {
                self.check(&c.factions, "fazione", f, &ctx);
                if let Some(r) = &t.rank
                    && c.rank(f, r).is_none() {
                        self.errors.push(format!("{ctx}: rango '{r}' non esiste nella fazione '{f}'"));
                    }
            }
            for s in t.stats.keys() {
                self.stat(s, &ctx);
            }
            for i in t.items.keys() {
                self.check(&c.items, "oggetto", i, &ctx);
            }
            for s in &t.statuses {
                self.check(&c.statuses, "status", s, &ctx);
            }
            for a in &t.actions {
                self.check(&c.actions, "azione", a, &ctx);
            }
            for ti in &t.titles {
                self.check(&c.titles, "titolo", ti, &ctx);
            }
            if let Some(d) = &t.disguise {
                if let Some(r) = &d.race {
                    self.check(&c.races, "razza", r, &ctx);
                }
                if let Some(f) = &d.faction {
                    self.check(&c.factions, "fazione", f, &ctx);
                }
            }
        }
        let mut players = Vec::new();
        for p in &c.placements {
            if let Placement::Player { id, faction, .. } = p {
                players.push(id.clone());
                self.check(&c.factions, "fazione", faction, &format!("giocatore {id}"));
            }
        }
        for p in &c.placements {
            match p {
                Placement::Entity { template, zone, faction, tether, leader_of, .. } => {
                    let ctx = format!("piazzamento {template}");
                    self.check(&c.templates, "template", template, &ctx);
                    self.zone(zone, &ctx);
                    if let Some(f) = faction {
                        self.check(&c.factions, "fazione", f, &ctx);
                    }
                    if let Some(t) = tether {
                        self.zone(&t.zone, &ctx);
                        self.cond(&t.release, &ctx);
                    }
                    if let Some(l) = leader_of
                        && !players.contains(l) {
                            self.errors.push(format!("{ctx}: giocatore '{l}' non esiste"));
                        }
                }
                Placement::Building { building, zone, owner_faction, owner_template, .. } => {
                    let ctx = format!("piazzamento edificio {building}");
                    self.check(&c.buildings, "edificio", building, &ctx);
                    self.zone(zone, &ctx);
                    if let Some(f) = owner_faction {
                        self.check(&c.factions, "fazione", f, &ctx);
                    }
                    if let Some(t) = owner_template {
                        self.check(&c.templates, "template", t, &ctx);
                    }
                }
                Placement::Player { .. } => {}
            }
        }
        for ev in c.events.values() {
            let ctx = format!("evento {}", ev.id);
            self.cond(&ev.when, &ctx);
            self.effects(&ev.effects, &ctx);
            if let Some(t) = &ev.by {
                self.check(&c.templates, "template", t, &ctx);
            }
        }
        for col in c.collections.values() {
            for i in &col.items {
                self.check(&c.items, "oggetto", i, &format!("collezione {}", col.id));
            }
            self.effects(&col.bonus, &format!("collezione {}", col.id));
        }
        for t in c.titles.values() {
            let ctx = format!("titolo {}", t.id);
            if !t.faction.is_empty() {
                self.check(&c.factions, "fazione", &t.faction, &ctx);
            }
            if t.mode == super::defs::TitleMode::Seat || !t.seat_zone.is_empty() {
                self.zone(&t.seat_zone, &ctx);
            }
            self.cond(&t.claim_requires, &ctx);
            if !t.challenge_stat.is_empty() {
                self.stat(&t.challenge_stat, &ctx);
            }
            for s in t.stats.keys().chain(t.score.keys().filter(|k| *k != "money")) {
                self.stat(s, &ctx);
            }
            for a in &t.abilities {
                self.check(&c.abilities, "abilità", a, &ctx);
            }
            for a in &t.actions {
                self.check(&c.actions, "azione", a, &ctx);
            }
        }
        for v in &c.victory {
            let ctx = format!("vittoria {}", v.id);
            match &v.kind {
                VictoryKind::HoldItems(items) => {
                    for i in items {
                        self.check(&c.items, "oggetto", i, &ctx);
                    }
                }
                VictoryKind::HoldTitle(t) => self.check(&c.titles, "titolo", t, &ctx),
                VictoryKind::Condition(cond) => self.cond(cond, &ctx),
                VictoryKind::VictoryPoints(_) => {}
            }
        }
    }

    fn selector(&mut self, s: &Selector, ctx: &str) {
        match s {
            Selector::Nearest(f) | Selector::Random(f) => self.filter(f, ctx),
            Selector::Zone(z) => self.zone(z, ctx),
            _ => {}
        }
    }

    fn filter(&mut self, f: &Filter, ctx: &str) {
        let c = self.c;
        if let Some(b) = &f.building {
            self.check(&c.buildings, "edificio", b, ctx);
        }
        if let Some(t) = &f.template {
            self.check(&c.templates, "template", t, ctx);
        }
        if let Some(fa) = &f.faction {
            self.check(&c.factions, "fazione", fa, ctx);
        }
        if let Some(s) = &f.has_status {
            self.check(&c.statuses, "status", s, ctx);
        }
        if let Some(cond) = &f.condition {
            self.cond(cond, ctx);
        }
    }

    fn effects(&mut self, effects: &[Effect], ctx: &str) {
        for e in effects {
            self.effect(e, ctx);
        }
    }

    fn effect(&mut self, e: &Effect, ctx: &str) {
        let c = self.c;
        match e {
            Effect::On(scope, inner) => {
                if let super::logic::Scope::Zone(z) = scope {
                    self.zone(z, ctx);
                }
                self.effect(inner, ctx)
            }
            Effect::All(v) | Effect::Cycle(v) => self.effects(v, ctx),
            Effect::Chance(_, inner) => self.effect(inner, ctx),
            Effect::If(cond, inner) => {
                self.cond(cond, ctx);
                self.effect(inner, ctx)
            }
            Effect::IfElse(cond, a, b) => {
                self.cond(cond, ctx);
                self.effect(a, ctx);
                self.effect(b, ctx)
            }
            Effect::ModStat { stat, .. } | Effect::SetStat { stat, .. } => self.stat(stat, ctx),
            Effect::ModNeed { need, .. } => self.check(&c.needs, "bisogno", need, ctx),
            Effect::ApplyStatus { status, .. }
            | Effect::RemoveStatus(status)
            | Effect::Immunize { status, .. }
            | Effect::Contaminate { status, .. } => self.check(&c.statuses, "status", status, ctx),
            Effect::GiveItem { item, .. } | Effect::TakeItem { item, .. } => {
                self.check(&c.items, "oggetto", item, ctx)
            }
            Effect::GiveRandomItem { items, .. } => {
                for item in items {
                    self.check(&c.items, "oggetto", item, ctx);
                }
            }
            Effect::ModRelation { faction, .. } | Effect::JoinFaction(faction) => {
                self.check(&c.factions, "fazione", faction, ctx)
            }
            Effect::Transmute(r) => self.check(&c.races, "razza", r, ctx),
            Effect::AddClass(cl) | Effect::RemoveClass(cl) => self.check(&c.classes, "classe", cl, ctx),
            Effect::Spawn { template, zone, faction, tether, .. } => {
                self.check(&c.templates, "template", template, ctx);
                if let Some(z) = zone {
                    self.zone(z, ctx);
                }
                if let Some(f) = faction {
                    self.check(&c.factions, "fazione", f, ctx);
                }
                if let Some(t) = tether {
                    self.zone(&t.zone, ctx);
                    self.cond(&t.release, ctx);
                }
            }
            Effect::MarketShock { item: Some(i), .. } => self.check(&c.items, "oggetto", i, ctx),
            Effect::GlobalModifier { id, .. } => self.check(&c.global_modifiers, "circostanza", id, ctx),
            Effect::Shapeshift { race, faction, .. } => {
                if let Some(r) = race {
                    self.check(&c.races, "razza", r, ctx);
                }
                if let Some(f) = faction {
                    self.check(&c.factions, "fazione", f, ctx);
                }
            }
            Effect::Spill { fluid, .. } => self.check(&c.fluids, "fluido", fluid, ctx),
            Effect::Teleport { zone } => self.zone(zone, ctx),
            Effect::PostJob { job, .. } => self.check(&c.jobs, "job", job, ctx),
            Effect::TransmuteFor { race, .. } => self.check(&c.races, "razza", race, ctx),
            Effect::ClaimTitle(t) | Effect::LeaveTitle(t) => self.check(&c.titles, "titolo", t, ctx),
            Effect::ChallengeTitle { title, stat, .. } => {
                self.check(&c.titles, "titolo", title, ctx);
                self.stat(stat, ctx);
            }
            _ => {}
        }
    }

    fn cond(&mut self, cond: &Condition, ctx: &str) {
        let c = self.c;
        match cond {
            Condition::All(v) | Condition::Any(v) => {
                for x in v {
                    self.cond(x, ctx);
                }
            }
            Condition::Not(x) | Condition::OnTarget(x) => self.cond(x, ctx),
            Condition::StatAtLeast { stat, .. } | Condition::StatBelow { stat, .. } => self.stat(stat, ctx),
            Condition::NeedBelow { need, .. } => self.check(&c.needs, "bisogno", need, ctx),
            Condition::HasStatus(s) => self.check(&c.statuses, "status", s, ctx),
            Condition::HasItem { item, .. } => self.check(&c.items, "oggetto", item, ctx),
            Condition::InZone(z) => self.zone(z, ctx),
            Condition::MemberOf(f) => self.check(&c.factions, "fazione", f, ctx),
            Condition::IsRace(r) => self.check(&c.races, "razza", r, ctx),
            Condition::RaceIn(list) => {
                for r in list {
                    self.check(&c.races, "razza", r, ctx);
                }
            }
            Condition::HasClass(cl) => self.check(&c.classes, "classe", cl, ctx),
            Condition::TemplateDead(t) | Condition::TemplateAlive(t) => {
                self.check(&c.templates, "template", t, ctx)
            }
            Condition::BuildingHpBelow { building, .. } => self.check(&c.buildings, "edificio", building, ctx),
            Condition::ZoneOccupied { zone, faction, .. } => {
                self.zone(zone, ctx);
                if let Some(f) = faction {
                    self.check(&c.factions, "fazione", f, ctx);
                }
            }
            Condition::Contest { stat, .. } => self.stat(stat, ctx),
            Condition::TitleVacant(t) | Condition::HoldsTitle(t) => self.check(&c.titles, "titolo", t, ctx),
            Condition::PopulationBelow { faction, template, .. } => {
                if let Some(f) = faction {
                    self.check(&c.factions, "fazione", f, ctx);
                }
                if let Some(t) = template {
                    self.check(&c.templates, "template", t, ctx);
                }
            }
            Condition::TreasuryAtLeast { faction, .. } => self.check(&c.factions, "fazione", faction, ctx),
            Condition::FactionHoldsItems { faction, items } => {
                self.check(&c.factions, "fazione", faction, ctx);
                for i in items {
                    self.check(&c.items, "oggetto", i, ctx);
                }
            }
            _ => {}
        }
    }
}
