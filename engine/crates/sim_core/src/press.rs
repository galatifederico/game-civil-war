//! PressEngine: journalists perceive newsworthy events, keep scoops, publish articles on the social feed;
//! articles (real, propaganda or fake) move morale, reputation, relations, cover and market prices.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{Content, Truth};
use crate::crime::Detained;
use crate::effects::{apply_effects, EffectCtx};
use crate::events::{kind, EventBuilder, EventLog, SimEvent};
use crate::factions::{FactionMember, Factions};
use crate::ids::{IdIndex, SimId};
use crate::map::Position;
use crate::params::Params;
use crate::rng::SimRng;
use crate::stats::{Dead, Stats, Tags, TemplateId};
use crate::time::SimClock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scoop {
    pub event: u64,
    pub kind: String,
    pub tick: u64,
    pub news: f32,
    pub actor: Option<SimId>,
    pub target: Option<SimId>,
    pub faction: Option<String>,
    pub message: String,
    pub tags: Vec<String>,
}

#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Notebook {
    pub scoops: Vec<Scoop>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Article {
    pub id: u64,
    pub tick: u64,
    pub author: Option<SimId>,
    pub author_name: String,
    pub outlet: Option<String>,
    pub headline: String,
    pub truth: Truth,
    pub topics: Vec<String>,
    pub subject: Option<SimId>,
    pub source_event: Option<u64>,
    /// Feed category (cronaca, politica, economia…), from the content pack.
    #[serde(default)]
    pub category: String,
    /// 0..1: how much it matters; the default feed view hides the unimportant ones.
    #[serde(default)]
    pub importance: f32,
    /// Factions the story is about (for the "about you" filter).
    #[serde(default)]
    pub factions: Vec<String>,
}

impl Article {
    /// Whether the article concerns a faction (its members, its leader, its deeds).
    pub fn concerns(&self, faction: &str) -> bool {
        self.factions.iter().any(|f| f == faction)
    }
}

/// The social feed (its name comes from the content pack).
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Feed {
    pub name: String,
    pub articles: Vec<Article>,
    /// Id of the next article (articles get pruned, ids never repeat).
    #[serde(default)]
    pub next_id: u64,
}

/// Whether an article belongs to the main channel: big news for anyone, or news about a player's faction.
pub fn is_main(params: &Params, a: &Article, player_factions: &[String]) -> bool {
    let (mine, world) = (params.get("press.main_mine_threshold", 0.5) as f32, params.get("press.main_world_threshold", 0.9) as f32);
    a.importance >= world || (a.importance >= mine && player_factions.iter().any(|f| a.concerns(f)))
}

/// Keeps the feed short: the newest `press.main_keep` articles of the main channel, and the others only
/// for `press.secondary_ttl_ticks` (the server sets it to 24 real hours at the current speed).
pub fn prune_feed(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    if !tick.is_multiple_of(6) {
        return;
    }
    let p = world.resource::<Params>().clone();
    let (keep, ttl) = (p.get("press.main_keep", 60.0) as usize, p.get("press.secondary_ttl_ticks", 2160.0) as u64);
    let players: Vec<String> = world.resource::<crate::factions::Players>().players.values().map(|p| p.faction.clone()).collect();
    let mut feed = world.resource_mut::<Feed>();
    let main_total = feed.articles.iter().filter(|a| is_main(&p, a, &players)).count();
    let mut main_seen = 0;
    let mut kept = Vec::with_capacity(feed.articles.len());
    // Oldest first: the main articles beyond the newest `keep` go.
    for a in std::mem::take(&mut feed.articles) {
        if is_main(&p, &a, &players) {
            main_seen += 1;
            if main_total - main_seen < keep {
                kept.push(a);
            }
        } else if a.tick + ttl > tick {
            kept.push(a);
        }
    }
    feed.articles = kept;
}

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct PressCursor(pub u64);

/// Journalists within range record newsworthy events they can see.
pub fn gather_scoops(world: &mut World) {
    let cursor = world.resource::<PressCursor>().0;
    let events: Vec<SimEvent> = world.resource::<EventLog>().since(cursor).to_vec();
    if let Some(last) = events.last() {
        world.resource_mut::<PressCursor>().0 = last.id;
    }
    let p = world.resource::<Params>();
    let (range, min_news) = (p.get("press.perception_range", 8.0) as i32, p.f("press.min_newsworthiness"));
    let press_tag = world.resource::<Content>().bindings.press_tag.clone();
    let journalists: Vec<Entity> = crate::sorted_entities::<Notebook>(world)
        .into_iter()
        .filter(|e| world.get::<Dead>(*e).is_none() && world.get::<Detained>(*e).is_none())
        .filter(|e| world.get::<Tags>(*e).is_some_and(|t| t.has(&press_tag)))
        .collect();
    for ev in events.iter().filter(|e| e.newsworthiness >= min_news && e.kind != kind::ARTICLE) {
        let Some(pos) = ev.pos else { continue };
        for j in &journalists {
            let me = world.get::<SimId>(*j).copied();
            if ev.actor == me && ev.kind != kind::JOB_DONE {
                continue;
            }
            if !world.get::<Position>(*j).is_some_and(|p| p.within(&pos, range)) {
                continue;
            }
            let scoop = Scoop {
                event: ev.id,
                kind: ev.kind.clone(),
                tick: ev.tick,
                news: ev.newsworthiness,
                actor: ev.actor,
                target: ev.target,
                faction: ev.faction.clone(),
                message: ev.message.clone(),
                tags: ev.tags.clone(),
            };
            let mut nb = world.get_mut::<Notebook>(*j).unwrap();
            if !nb.scoops.iter().any(|s| s.event == ev.id) {
                nb.scoops.push(scoop);
            }
        }
    }
    // Scoops get stale after two days.
    let tick = world.resource::<SimClock>().tick;
    for j in journalists {
        world.get_mut::<Notebook>(j).unwrap().scoops.retain(|s| s.tick + 48 > tick);
    }
}

fn name_of_id(world: &World, id: Option<SimId>) -> String {
    id.and_then(|i| world.resource::<IdIndex>().get(i))
        .map(|e| crate::infiltration::apparent_name(world, e))
        .unwrap_or_else(|| "qualcuno".into())
}

/// Writes an article from the journalist's best scoop. Returns the headline.
pub fn publish_best_scoop(world: &mut World, journalist: Entity) -> Option<String> {
    let published: std::collections::BTreeSet<u64> =
        world.resource::<Feed>().articles.iter().filter_map(|a| a.source_event).collect();
    let mut nb = world.get_mut::<Notebook>(journalist)?;
    // A story already published by someone else is no longer a scoop.
    nb.scoops.retain(|s| !published.contains(&s.event));
    let best = nb.scoops.iter().enumerate().max_by(|a, b| a.1.news.total_cmp(&b.1.news).then(b.0.cmp(&a.0)))?.0;
    let scoop = nb.scoops.remove(best);
    let press = world.resource::<Content>().press.clone();
    let templates = press.headlines.get(&scoop.kind).cloned().unwrap_or_default();
    let template = world.resource_mut::<SimRng>().pick(&templates).cloned().unwrap_or_else(|| {
        if press.default_headline.is_empty() { "{message}".into() } else { press.default_headline.clone() }
    });
    let faction_name = scoop
        .faction
        .as_ref()
        .and_then(|f| world.resource::<Content>().factions.get(f).map(|d| d.name.clone()))
        .unwrap_or_default();
    let headline = template
        .replace("{actor}", &name_of_id(world, scoop.actor))
        .replace("{target}", &name_of_id(world, scoop.target))
        .replace("{faction}", &faction_name)
        .replace("{message}", &scoop.message);
    let mut topics = vec![scoop.kind.clone()];
    topics.extend(scoop.tags.iter().cloned());
    let subject_entity = scoop.target.or(scoop.actor).and_then(|i| world.resource::<IdIndex>().get(i));
    let subject = match scoop.kind.as_str() {
        kind::ARREST | kind::SEARCH | kind::SEIZURE => scoop.target,
        _ => scoop.actor.or(scoop.target),
    }
    .and_then(|i| world.resource::<IdIndex>().get(i))
    .or(subject_entity);
    publish(world, Some(journalist), headline.clone(), Truth::Real, topics, subject, Some(scoop.event));
    Some(headline)
}

/// Publishes an article on the feed and applies its impacts.
pub fn publish(
    world: &mut World,
    author: Option<Entity>,
    headline: String,
    truth: Truth,
    topics: Vec<String>,
    subject: Option<Entity>,
    source_event: Option<u64>,
) -> u64 {
    let tick = world.resource::<SimClock>().tick;
    let author_id = author.and_then(|a| world.get::<SimId>(a).copied());
    let author_name = author.map_or_else(|| "Anonimo".into(), |a| crate::infiltration::apparent_name(world, a));
    let outlet = author.and_then(|a| world.get::<FactionMember>(a)).map(|m| m.faction.clone());
    let subject_id = subject.and_then(|s| world.get::<SimId>(s).copied());
    // Category and importance.
    let press = world.resource::<Content>().press.clone();
    let cat = press.categories.iter().find(|c| c.topics.iter().any(|t| topics.contains(t)));
    let category = cat.map(|c| c.id.clone()).unwrap_or_else(|| if press.default_category.is_empty() { "varie".into() } else { press.default_category.clone() });
    let base = source_event
        .and_then(|id| world.resource::<EventLog>().all().iter().rev().find(|e| e.id == id).map(|e| e.newsworthiness))
        .unwrap_or(match truth {
            Truth::Real => 0.6,
            Truth::Propaganda => 0.3,
            Truth::Fake => 0.25,
        });
    let importance = (base * cat.and_then(|c| c.weight).unwrap_or(1.0)).clamp(0.0, 1.0);
    let mut factions: Vec<String> = Vec::new();
    if let Some(f) = subject.and_then(|s| world.get::<FactionMember>(s)).map(|m| m.faction.clone()) {
        factions.push(f);
    }
    if let Some(ev) = source_event.and_then(|id| world.resource::<EventLog>().all().iter().rev().find(|e| e.id == id).cloned()) {
        if let Some(f) = ev.faction {
            factions.push(f);
        }
        for who in [ev.actor, ev.target].into_iter().flatten() {
            if let Some(f) = world.resource::<crate::ids::IdIndex>().get(who).and_then(|e| world.get::<FactionMember>(e)).map(|m| m.faction.clone()) {
                factions.push(f);
            }
        }
    }
    factions.sort();
    factions.dedup();
    let id = {
        let mut feed = world.resource_mut::<Feed>();
        // Saves from before `next_id` existed: continue after the last article.
        let id = feed.next_id.max(feed.articles.last().map_or(0, |a| a.id) + 1);
        feed.next_id = id + 1;
        feed.articles.push(Article {
            id,
            tick,
            author: author_id,
            author_name: author_name.clone(),
            outlet: outlet.clone(),
            headline: headline.clone(),
            truth,
            topics: topics.clone(),
            subject: subject_id,
            source_event,
            category,
            importance,
            factions,
        });
        id
    };
    let feed_name = world.resource::<Feed>().name.clone();
    let label = match truth {
        Truth::Real => "",
        Truth::Propaganda => " [propaganda]",
        // Fake news look like any other article: readers cannot tell.
        Truth::Fake => "",
    };
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::ARTICLE, format!("{feed_name} — «{headline}» di {author_name}{label}"))
            .actor(author_id)
            .target(subject_id)
            .faction(outlet.clone())
            .tags(topics.iter().cloned())
            .data(serde_json::json!({ "article": id, "truth": truth })),
    );
    apply_impacts(world, author, truth, &topics, subject);
    id
}

fn apply_impacts(world: &mut World, author: Option<Entity>, truth: Truth, topics: &[String], subject: Option<Entity>) {
    let content = world.resource::<Content>().clone();
    let b = &content.bindings;
    if truth == Truth::Real
        && let Some(a) = author {
            let gain = world.resource::<Params>().f("press.reputation_gain");
            let bounds = content.stat_bounds(&b.press_reputation);
            if let Some(mut s) = world.get_mut::<Stats>(a) {
                s.add_base(&b.press_reputation, gain, bounds);
            }
        }
    let author_faction = author.and_then(|a| world.get::<FactionMember>(a)).map(|m| m.faction.clone());
    let subject_faction = subject.and_then(|s| world.get::<FactionMember>(s)).map(|m| m.faction.clone());
    for imp in content.news_impacts.iter().filter(|i| topics.contains(&i.topic)) {
        let factor = if truth == Truth::Fake { if imp.fake_factor == 0.0 { 1.0 } else { imp.fake_factor } } else { 1.0 };
        for (tag, d, s) in &imp.market {
            let d = 1.0 + (d - 1.0) * factor;
            let s = 1.0 + (s - 1.0) * factor;
            crate::market::add_shock(world, None, Some(tag), d, s, imp.duration.max(1), &format!("news:{}", imp.topic));
        }
        if imp.morale != 0.0 {
            let eff = crate::content::Effect::On(
                crate::content::Scope::Everyone,
                Box::new(crate::content::Effect::ModStat { stat: b.morale.clone(), amount: imp.morale * factor }),
            );
            crate::effects::apply_effect(world, &EffectCtx::new(None, None, "news"), &eff);
        }
        if let Some(s) = subject {
            if imp.subject_reputation != 0.0 {
                let bounds = content.stat_bounds(&b.press_reputation);
                if let Some(mut st) = world.get_mut::<Stats>(s) {
                    st.add_base(&b.press_reputation, imp.subject_reputation * factor, bounds);
                }
                if let Some(f) = &subject_faction
                    && let Some(fs) = world.resource_mut::<Factions>().states.get_mut(f) {
                        fs.reputation += imp.subject_reputation * factor;
                    }
            }
            if imp.exposes > 0.0 {
                crate::infiltration::mod_cover(world, s, -imp.exposes * factor, "inchiesta giornalistica");
            }
        }
        if imp.relation != 0.0
            && let (Some(a), Some(s)) = (&author_faction, &subject_faction) {
                world.resource_mut::<Factions>().modify_relation(a, s, imp.relation * factor);
            }
    }
}

/// Automatic news sources (e.g. an AI spreading fake news from tick 0).
pub fn news_sources(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let sources: Vec<_> = world.resource::<Content>().news_sources.values().cloned().collect();
    for src in sources {
        if tick < src.start_tick || src.interval == 0 || !(tick - src.start_tick).is_multiple_of(src.interval) || src.headlines.is_empty() {
            continue;
        }
        let author = match &src.author {
            Some(t) => {
                let found = crate::sorted_entities::<TemplateId>(world)
                    .into_iter()
                    .find(|e| world.get::<TemplateId>(*e).is_some_and(|x| &x.0 == t) && world.get::<Dead>(*e).is_none());
                match found {
                    Some(e) => Some(e),
                    None => continue,
                }
            }
            None => None,
        };
        let Some(item) = world.resource_mut::<SimRng>().pick(&src.headlines).cloned() else { continue };
        let ctx = EffectCtx::new(author, None, format!("news:{}", src.id));
        apply_effects(world, &ctx, &item.effects);
        let headline = crate::effects::substitute(world, &item.headline, author, None);
        publish(world, author, headline, src.truth, item.topics.clone(), None, None);
    }
}
