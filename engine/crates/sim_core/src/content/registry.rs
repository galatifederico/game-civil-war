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
    pub spawners: BTreeMap<Id, SpawnerDef>,
    pub triggers: BTreeMap<Id, TriggerDef>,
    pub collections: BTreeMap<Id, CollectionDef>,
    pub titles: BTreeMap<Id, TitleDef>,
    pub press: PressDef,
    pub news_sources: BTreeMap<Id, NewsSourceDef>,
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

impl Content {
    /// Merges packs in order (later definitions with the same id replace earlier ones) and validates
    /// every cross reference.
    pub fn from_packs(packs: Vec<ContentPack>) -> Result<Self, ContentError> {
        let mut c = ContentData::default();
        for p in packs {
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
            merge(&mut c.spawners, p.spawners, |d| &d.id);
            merge(&mut c.triggers, p.triggers, |d| &d.id);
            merge(&mut c.collections, p.collections, |d| &d.id);
            merge(&mut c.titles, p.titles, |d| &d.id);
            if let Some(press) = p.press {
                c.press = press;
            }
            merge(&mut c.news_sources, p.news_sources, |d| &d.id);
            c.news_impacts.extend(p.news_impacts);
            c.victory.extend(p.victory);
            merge(&mut c.global_modifiers, p.global_modifiers, |d| &d.id);
            merge(&mut c.supplies, p.supplies, |d| &d.id);
            c.sprites.extend(p.sprites);
            for (k, lines) in p.dialogue {
                c.dialogue.entry(k).or_default().extend(lines);
            }
        }
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

    pub fn stat_bounds(&self, stat: &str) -> (f32, f32) {
        self.stats.get(stat).map_or((f32::MIN, f32::MAX), |s| (s.min, s.max))
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
            self.effects(&n.effects_when_low, &format!("bisogno {}", n.id));
        }
        for bp in c.body_plans.values() {
            for p in &bp.parts {
                if let Some(parent) = &p.parent
                    && !bp.parts.iter().any(|q| &q.id == parent) {
                        self.errors.push(format!("piano corporeo {}: parte padre '{parent}' non esiste", bp.id));
                    }
            }
        }
        for r in c.races.values() {
            let ctx = format!("razza {}", r.id);
            self.check(&c.body_plans, "piano corporeo", &r.body_plan, &ctx);
            for s in r.stats.keys() {
                self.stat(s, &ctx);
            }
            for a in &r.abilities {
                self.check(&c.abilities, "abilità", a, &ctx);
            }
            for s in r.innate_statuses.iter().chain(&r.immunities) {
                self.check(&c.statuses, "status", s, &ctx);
            }
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
            for r in &cl.races {
                self.check(&c.races, "razza", r, &ctx);
            }
        }
        for s in c.statuses.values() {
            let ctx = format!("status {}", s.id);
            for k in s.stats.keys().chain(s.stages.iter().flat_map(|st| st.stats.keys())) {
                self.stat(k, &ctx);
            }
            for k in s.need_rates.keys().chain(s.stages.iter().flat_map(|st| st.need_rates.keys())) {
                self.check(&c.needs, "bisogno", k, &ctx);
            }
            if let Some(e) = &s.escalates_to {
                self.check(&c.statuses, "status", e, &ctx);
            }
            if let Some((f, _)) = &s.spills {
                self.check(&c.fluids, "fluido", f, &ctx);
            }
            self.effects(&s.on_apply, &ctx);
            self.effects(&s.per_tick, &ctx);
            self.effects(&s.on_expire, &ctx);
            for st in &s.stages {
                self.effects(&st.on_enter, &ctx);
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
            self.cond(&a.requires, &ctx);
            self.effects(&a.effects, &ctx);
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
                ActionKind::Ability { ability, target } => {
                    self.check(&c.abilities, "abilità", ability, &ctx);
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
            for r in &b.recipes {
                for i in r.inputs.keys().chain(r.outputs.keys()) {
                    self.check(&c.items, "oggetto", i, &ctx);
                }
            }
            for i in b.sells.keys().chain(b.stock.keys()).chain(b.cost.keys()).chain(b.passive.keys()) {
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
        for s in c.spawners.values() {
            let ctx = format!("spawner {}", s.id);
            self.check(&c.templates, "template", &s.template, &ctx);
            self.zone(&s.zone, &ctx);
            self.cond(&s.active_when, &ctx);
            if let Some(t) = &s.summoner {
                self.check(&c.templates, "template", t, &ctx);
            }
            if let Some(t) = &s.tether {
                self.zone(&t.zone, &ctx);
                self.cond(&t.release, &ctx);
            }
        }
        for t in c.triggers.values() {
            let ctx = format!("trigger {}", t.id);
            self.cond(&t.when, &ctx);
            self.effects(&t.effects, &ctx);
        }
        for col in c.collections.values() {
            for i in &col.items {
                self.check(&c.items, "oggetto", i, &format!("collezione {}", col.id));
            }
            self.effects(&col.bonus, &format!("collezione {}", col.id));
        }
        for t in c.titles.values() {
            let ctx = format!("titolo {}", t.id);
            self.check(&c.factions, "fazione", &t.faction, &ctx);
            self.zone(&t.seat_zone, &ctx);
            self.cond(&t.claim_requires, &ctx);
        }
        for n in c.news_sources.values() {
            if let Some(a) = &n.author {
                self.check(&c.templates, "template", a, &format!("fonte di notizie {}", n.id));
            }
            for h in &n.headlines {
                self.effects(&h.effects, &format!("fonte di notizie {}", n.id));
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
            Effect::All(v) => self.effects(v, ctx),
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
            Effect::Spawn { template, zone, .. } => {
                self.check(&c.templates, "template", template, ctx);
                if let Some(z) = zone {
                    self.zone(z, ctx);
                }
            }
            Effect::MarketShock { item: Some(i), .. } => self.check(&c.items, "oggetto", i, ctx),
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
            Condition::TitleVacant(t) => self.check(&c.titles, "titolo", t, ctx),
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
