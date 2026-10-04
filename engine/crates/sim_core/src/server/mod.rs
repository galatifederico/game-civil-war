//! Admin Control Panel (REST on axum), Prometheus `/metrics`, MCP JSON-RPC and the UI state API.
//!
//! The simulation lives behind a mutex; a ticker task advances it in real time. Every write goes through
//! [`SimCommand`]s (queued for the next tick, or applied at once with `?now=true`).

pub mod mcp;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
pub use metrics_exporter_prometheus::PrometheusHandle;
use metrics_exporter_prometheus::PrometheusBuilder;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::prelude::*;

pub type SharedSim = Arc<Mutex<Simulation>>;

#[derive(Debug, Clone, Deserialize)]
pub struct Control {
    pub paused: bool,
    /// Real milliseconds per tick.
    pub tick_ms: u64,
}

/// Rebuilds an empty simulation (same content and plugins) to load saves into.
pub type Factory = Arc<dyn Fn() -> Result<crate::sim::SimBuilder, String> + Send + Sync>;

#[derive(Clone)]
pub struct AppState {
    pub sim: SharedSim,
    pub control: Arc<Mutex<Control>>,
    pub metrics: Option<PrometheusHandle>,
    /// Optional static client served on `/ui/` (index.html content).
    pub ui_html: Option<Arc<String>>,
    /// Needed by `POST /api/load`.
    pub factory: Option<Factory>,
    /// When set, every non-GET request needs `Authorization: Bearer <token>`.
    pub token: Option<String>,
    /// Folder of sprite images (PNG) shown by the admin console under `/assets/`.
    pub assets: Option<std::path::PathBuf>,
    /// Light UI state already serialized, per fog faction, for the tick it was made at.
    pub ui_cache: UiCache,
}

/// See [`AppState::ui_cache`]: faction ("" = no fog) → (tick, JSON).
pub type UiCache = Arc<Mutex<std::collections::HashMap<String, (u64, Arc<String>)>>>;

/// Installs the global Prometheus recorder (once per process).
pub fn install_metrics() -> Option<PrometheusHandle> {
    PrometheusBuilder::new().install_recorder().ok()
}

pub fn router(state: AppState) -> Router {
    router_inner(state).layer(tower_http::compression::CompressionLayer::new())
}

fn router_inner(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/admin/", get(admin_page))
        .route("/api/content", get(content_all))
        .route("/api/assets", get(assets_list))
        .route("/assets/{file}", get(asset_file))
        .route("/healthz", get(|| async { "ok" }))
        .route("/metrics", get(metrics))
        .route("/api/state", get(state_all))
        .route("/api/entities", get(entities))
        .route("/api/entities/{id}", get(entity))
        .route("/api/entities/{id}/ai", get(entity_ai))
        .route("/api/events", get(events))
        .route("/api/feed", get(feed))
        .route("/api/market", get(market))
        .route("/api/factions", get(factions))
        .route("/api/jobs", get(jobs))
        .route("/api/squads", get(squads))
        .route("/api/params", get(params))
        .route("/api/params/{key}", put(set_param))
        .route("/api/sprites", get(sprites))
        .route("/api/sprites/{id}", put(set_sprite))
        .route("/api/commands", post(command))
        .route("/api/commands/{seq}", get(command_result))
        .route("/api/control", get(control_get).post(control_set))
        .route("/api/compendium", get(compendium))
        .route("/api/extensions", get(extensions))
        .route("/api/ui/activity", get(ui_activity))
        .route("/api/ui/map", get(ui_map))
        .route("/api/ui/terrain", get(ui_terrain))
        .route("/api/ui/state", get(ui_state))
        .route("/ui/", get(ui_page))
        .route("/api/ui/player/{player}", get(ui_player))
        .route("/api/ui/feed", get(ui_feed))
        .route("/api/ui/actions/{id}", get(ui_actions))
        .route("/api/ui/step", post(ui_step))
        .route("/api/ui/talk", post(ui_talk))
        .route("/api/ui/interactions/{player}/{target}", get(ui_interactions))
        .route("/api/ui/item/{item}", get(ui_item))
        .route("/api/ui/collections", get(ui_collections))
        .route("/api/ui/economy", get(ui_economy))
        .route("/api/ui/classes/{player}", get(ui_classes))
        .route("/api/ui/class", post(ui_take_class))
        .route("/api/ui/roles", get(ui_roles))
        .route("/api/ui/stats", get(ui_stats))
        .route("/api/ui/role", post(ui_challenge_role))
        .route("/api/save", post(save_game))
        .route("/api/load", post(load_game))
        .route("/mcp", post(mcp_http))
        .layer(axum::middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state)
}

/// Runs the ticker and the HTTP server until Ctrl+C.
pub async fn serve(state: AppState, addr: SocketAddr) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("admin/API/MCP su http://{addr}");
    serve_on(state, listener).await
}

/// Like [`serve`] on an already bound listener.
pub async fn serve_on(state: AppState, listener: tokio::net::TcpListener) -> std::io::Result<()> {
    let ticker = {
        let (sim, control) = (state.sim.clone(), state.control.clone());
        tokio::spawn(async move {
            loop {
                let (paused, ms) = {
                    let c = control.lock().unwrap();
                    (c.paused, c.tick_ms.max(1))
                };
                if !paused {
                    let mut sim = sim.lock().unwrap();
                    // Secondary news last 24 real hours, whatever the speed.
                    let ttl = (86_400_000 / ms).max(24) as f64;
                    if sim.world.resource::<crate::params::Params>().get("press.secondary_ttl_ticks", 0.0) != ttl {
                        sim.world.resource_mut::<crate::params::Params>().set("press.secondary_ttl_ticks", ttl);
                    }
                    sim.tick();
                }
                tokio::time::sleep(Duration::from_millis(ms)).await;
            }
        })
    };
    let res = axum::serve(listener, router(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await;
    ticker.abort();
    res
}

type ApiResult = Result<Json<Value>, (StatusCode, String)>;

async fn auth(State(s): State<AppState>, req: axum::extract::Request, next: axum::middleware::Next) -> Response {
    if let Some(token) = &s.token {
        if req.method() != axum::http::Method::GET {
            let ok = req
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v == format!("Bearer {token}"));
            if !ok {
                return (StatusCode::UNAUTHORIZED, "token mancante o errato").into_response();
            }
        }
    }
    next.run(req).await
}

/// The player's view: champion, members (with obedience), squads, salary matrix, work types.
async fn ui_player(State(s): State<AppState>, Path(player): Path<String>) -> ApiResult {
    let mut sim = s.sim.lock().unwrap();
    let p = sim.world.resource::<Players>().players.get(&player).cloned().ok_or_else(|| not_found(&player))?;
    let content = sim.content().clone();
    let race_name = |id: &Option<String>| id.as_ref().map(|r| content.races.get(r).map_or(r.clone(), |d| d.name.clone()));
    let class_name = |id: &String| content.classes.get(id).map_or(id.clone(), |d| d.name.clone());
    let item_name = |id: &String| content.items.get(id).map_or(id.clone(), |d| d.name.clone());
    // Items the team holds: carried by the members and stored in the faction's buildings.
    let mut carried = std::collections::BTreeMap::<String, u32>::new();
    let mut stored = std::collections::BTreeMap::<String, u32>::new();
    let (mut pocket, mut morale_sum, mut health_sum) = (0.0f64, 0.0f32, 0.0f32);
    let members: Vec<Value> = crate::sorted_entities::<FactionMember>(&mut sim.world)
        .into_iter()
        .filter(|e| sim.world.get::<FactionMember>(*e).is_some_and(|m| m.faction == p.faction) && sim.world.get::<Dead>(*e).is_none())
        .filter_map(|e| {
            let v = crate::snapshot::entity_view(&sim.world, e, true)?;
            for (item, n) in &v.inventory {
                *carried.entry(item.clone()).or_default() += n;
            }
            let morale = v.activity.as_ref().map_or(50.0, |a| a.morale);
            pocket += v.money;
            morale_sum += morale;
            health_sum += v.health;
            Some(json!({
                "id": v.id, "name": v.name, "rank": v.rank, "classes": v.classes, "pos": v.pos,
                "race": v.race, "race_name": race_name(&v.race), "sex": v.sex,
                "class_names": v.classes.iter().map(class_name).collect::<Vec<_>>(),
                "health": v.health, "morale": morale, "mood": v.activity.as_ref().map(|a| a.mood.clone()),
                "money": v.money,
                "activity": v.activity.as_ref().map(|a| a.label.clone()),
                "obedience": crate::player::obedience(&sim.world, e),
                "champion": v.leader_of.as_deref() == Some(player.as_str()),
                "work": sim.world.get::<crate::jobs::WorkPriorities>(e).map(|w| w.matrix()),
            }))
        })
        .collect();
    for e in crate::sorted_entities::<crate::buildings::Building>(&mut sim.world) {
        let owned = sim.world.get::<crate::buildings::Building>(e).is_some_and(|b| b.owner == crate::buildings::Owner::Faction(p.faction.clone()));
        if let (true, Some(stock)) = (owned, sim.world.get::<crate::inventory::Stock>(e)) {
            for (item, n) in &stock.0 {
                *stored.entry(item.clone()).or_default() += n;
            }
        }
    }
    let mut ids: Vec<&String> = carried.keys().chain(stored.keys()).collect();
    ids.sort();
    ids.dedup();
    let inventory: Vec<Value> = ids
        .into_iter()
        .map(|i| {
            let (c, s) = (carried.get(i).copied().unwrap_or(0), stored.get(i).copied().unwrap_or(0));
            let category = content.items.get(i).map(|d| d.category.clone());
            json!({ "id": i, "name": item_name(i), "category": category, "carried": c, "stored": s, "total": c + s })
        })
        .filter(|v| v["total"].as_u64().unwrap_or(0) > 0)
        .collect();
    let n = members.len().max(1) as f32;
    let summary = json!({
        "members": members.len(), "pocket_money": pocket,
        "avg_morale": morale_sum / n, "avg_mood": crate::snapshot::mood_label(morale_sum / n), "avg_health": health_sum / n,
    });
    let squads: Vec<Value> = sim
        .world
        .resource::<Squads>()
        .squads
        .values()
        .filter(|q| q.faction.as_deref() == Some(p.faction.as_str()))
        .map(|q| serde_json::to_value(q).unwrap_or_default())
        .collect();
    let state = sim.world.resource::<Factions>().states.get(&p.faction).cloned().unwrap_or_default();
    let ranks: Vec<Value> = content.factions.get(&p.faction).map_or(vec![], |f| {
        f.ranks
            .iter()
            .map(|r| json!({ "id": r.id, "name": r.name, "level": r.level, "salary": state.salaries.get(&r.id).copied().unwrap_or(r.salary) }))
            .collect()
    });
    let mut work_types: Vec<String> = content.jobs.values().map(|j| j.work_type.clone()).filter(|w| !w.is_empty()).collect();
    for b in content.buildings.values() {
        work_types.extend(b.recipes.iter().map(|r| r.work_type.clone()).filter(|w| !w.is_empty()));
    }
    work_types.sort();
    work_types.dedup();
    Ok(Json(json!({
        "player": p, "faction": p.faction, "treasury": state.treasury, "victory_points": state.victory_points,
        "champion": p.leader, "members": members, "squads": squads, "ranks": ranks, "work_types": work_types,
        "summary": summary, "inventory": inventory,
    })))
}

#[derive(Deserialize)]
struct FeedQ {
    /// "important" (default), "mine", "all" or a category id.
    #[serde(default)]
    filter: Option<String>,
    player: Option<String>,
    #[serde(default = "twenty")]
    limit: usize,
}

fn twenty() -> usize {
    20
}

/// The social feed seen by a player: filtered, newest first, with fake news indistinguishable.
async fn ui_feed(State(s): State<AppState>, Query(q): Query<FeedQ>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    let content = sim.content();
    let feed = sim.world.resource::<Feed>();
    let faction = q.player.as_ref().and_then(|p| sim.world.resource::<Players>().players.get(p)).map(|p| p.faction.clone());
    let threshold = if content.press.important_threshold > 0.0 { content.press.important_threshold } else { 0.6 };
    let filter = q.filter.clone().unwrap_or_else(|| "important".into());
    let params = sim.world.resource::<crate::params::Params>();
    let (main_mine, main_world) = (params.get("press.main_mine_threshold", 0.5) as f32, params.get("press.main_world_threshold", 0.9) as f32);
    let mine = |a: &crate::press::Article| faction.as_ref().is_some_and(|f| a.concerns(f));
    let articles: Vec<Value> = feed
        .articles
        .iter()
        .rev()
        .filter(|a| match filter.as_str() {
            "all" => true,
            "important" => a.importance >= threshold || mine(a),
            // The pigeon's main channel: few messages, what touches the player plus the big world news.
            "main" => (mine(a) && a.importance >= main_mine) || a.importance >= main_world,
            "mine" => mine(a),
            cat => a.category == cat,
        })
        .take(q.limit.min(200))
        .map(|a| {
            json!({
                "id": a.id, "tick": a.tick, "headline": a.headline, "author_name": a.author_name,
                "category": a.category, "importance": a.importance, "mine": mine(a),
                // Propaganda is signed by its faction; fake news look real.
                "propaganda": a.truth == crate::content::Truth::Propaganda,
            })
        })
        .collect();
    let mut counts = std::collections::BTreeMap::<&str, usize>::new();
    for a in &feed.articles {
        *counts.entry(a.category.as_str()).or_default() += 1;
    }
    let categories: Vec<Value> = content
        .press
        .categories
        .iter()
        .map(|c| json!({ "id": c.id, "name": c.name, "count": counts.get(c.id.as_str()).copied().unwrap_or(0) }))
        .collect();
    Ok(Json(json!({ "name": feed.name, "filter": filter, "categories": categories, "articles": articles })))
}

/// Orders a pawn can receive: its jobs (from its AI actions) and abilities, plus moving and following.
async fn ui_actions(State(s): State<AppState>, Path(id): Path<u64>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    let e = sim.entity(SimId(id)).ok_or_else(|| not_found(id))?;
    let content = sim.content();
    let mut out = vec![json!({ "kind": "move", "id": "move", "name": "Vai qui", "needs_target": "cell" })];
    let actions = sim.world.get::<crate::ai::Brain>(e).map(|b| b.actions.clone()).unwrap_or_default();
    let mut seen = std::collections::BTreeSet::new();
    for a in actions.iter().filter_map(|a| content.actions.get(a)) {
        if let crate::content::ActionKind::Job { job, target } = &a.kind {
            if seen.insert(job.clone()) {
                let name = content.jobs.get(job).map_or(job.clone(), |j| j.name.clone());
                let needs = if *target == crate::content::Selector::None { "none" } else { "entity" };
                out.push(json!({ "kind": "job", "id": job, "name": name, "needs_target": needs }));
            }
        }
    }
    for ab in crate::abilities::known(&sim.world, e) {
        if let Some(d) = content.abilities.get(&ab) {
            out.push(json!({ "kind": "ability", "id": ab, "name": d.name, "needs_target": if d.range > 0 { "entity" } else { "none" } }));
        }
    }
    out.push(json!({ "kind": "follow", "id": "follow", "name": "Segui", "needs_target": "entity" }));
    out.push(json!({ "kind": "stop", "id": "stop", "name": "Fermati", "needs_target": "none" }));
    Ok(Json(json!(out)))
}

#[derive(Deserialize)]
struct StepBody {
    player: String,
    dx: i32,
    dy: i32,
}

/// One step of the champion, applied at once: answers with where it stands now.
async fn ui_step(State(s): State<AppState>, Json(b): Json<StepBody>) -> ApiResult {
    let mut sim = s.sim.lock().unwrap();
    let pos = crate::player::step(&mut sim.world, &b.player, b.dx, b.dy).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok(Json(json!({ "pos": pos })))
}

#[derive(Deserialize)]
struct TalkBody {
    player: String,
    target: u64,
}

/// The champion talks to someone next to it: who answers and what it says.
async fn ui_talk(State(s): State<AppState>, Json(b): Json<TalkBody>) -> ApiResult {
    let mut sim = s.sim.lock().unwrap();
    let (name, line) = crate::player::talk(&mut sim.world, &b.player, SimId(b.target)).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok(Json(json!({ "name": name, "line": line })))
}

/// What the player's champion can do with a pawn or building: talk (pawns), then the champion's jobs whose
/// target filter accepts it, its abilities with a range, following. With the distance from the champion.
async fn ui_interactions(State(s): State<AppState>, Path((player, target)): Path<(String, u64)>) -> ApiResult {
    let mut sim = s.sim.lock().unwrap();
    let p = sim.world.resource::<Players>().players.get(&player).cloned().ok_or_else(|| not_found(&player))?;
    let me = p.leader.and_then(|id| sim.entity(id)).ok_or_else(|| not_found("campione"))?;
    let t = sim.entity(SimId(target)).ok_or_else(|| not_found(target))?;
    let content = sim.content().clone();
    let building = sim.world.get::<crate::buildings::Building>(t).is_some();
    let dead = sim.world.get::<Dead>(t).is_some();
    let distance = match (sim.world.get::<Position>(me), sim.world.get::<Position>(t)) {
        (Some(a), Some(b)) => a.distance(b),
        _ => None,
    };
    let mut out = Vec::new();
    if t != me && !building && !dead {
        out.push(json!({ "kind": "talk", "id": "talk", "name": "Parla", "needs_target": "entity" }));
    }
    if t != me && !dead {
        let actions = sim.world.get::<crate::ai::Brain>(me).map(|b| b.actions.clone()).unwrap_or_default();
        let mut seen = std::collections::BTreeSet::new();
        for a in actions.iter().filter_map(|a| content.actions.get(a)) {
            let crate::content::ActionKind::Job { job, target: sel } = &a.kind else { continue };
            let filter = match sel {
                crate::content::Selector::Nearest(f) | crate::content::Selector::Random(f) => f,
                _ => continue,
            };
            if seen.contains(job) {
                continue;
            }
            // The target must fit the job (a shop for shopping, a pawn for a theft…); distance and sight
            // do not count: the champion walks there.
            let mut f = filter.clone();
            f.max_distance = None;
            f.visible = false;
            if crate::targeting::candidates(&mut sim.world, me, &f).iter().any(|(id, _)| id.0 == target) {
                seen.insert(job.clone());
                let name = content.jobs.get(job).map_or(job.clone(), |j| j.name.clone());
                out.push(json!({ "kind": "job", "id": job, "name": name, "needs_target": "entity" }));
            }
        }
        if !building {
            for ab in crate::abilities::known(&sim.world, me) {
                if let Some(d) = content.abilities.get(&ab).filter(|d| d.range > 0) {
                    out.push(json!({ "kind": "ability", "id": ab, "name": d.name, "needs_target": "entity" }));
                }
            }
            out.push(json!({ "kind": "follow", "id": "follow", "name": "Segui", "needs_target": "entity" }));
        }
    }
    let name = crate::infiltration::apparent_name(&sim.world, t);
    Ok(Json(json!({ "target": target, "name": name, "building": building, "distance": distance, "actions": out })))
}

/// A requirement split in lines, each with whether `who` meets it.
fn requirement_lines(sim: &mut Simulation, who: Option<Entity>, cond: &crate::content::Condition) -> Vec<Value> {
    let content = sim.content().clone();
    crate::describe::condition_parts(cond)
        .iter()
        .map(|p| {
            let ok = who.is_some_and(|e| crate::effects::eval_condition(&mut sim.world, &crate::effects::EffectCtx::new(Some(e), None, "ui"), p));
            json!({ "text": crate::describe::condition(&content, p), "ok": ok })
        })
        .collect()
}

fn ability_and_action_names(content: &crate::content::Content, abilities: &[String], actions: &[String]) -> Vec<String> {
    let mut out: Vec<String> = abilities.iter().filter_map(|a| content.abilities.get(a).map(|d| d.name.clone())).collect();
    for a in actions.iter().filter_map(|a| content.actions.get(a)) {
        let n = match &a.kind {
            crate::content::ActionKind::Job { job, .. } => content.jobs.get(job).map_or(a.name.clone(), |j| j.name.clone()),
            _ => a.name.clone(),
        };
        if !out.contains(&n) {
            out.push(n);
        }
    }
    out
}

/// The characteristics of the pawns, in content order: id, name, section of the card and range.
async fn ui_stats(State(s): State<AppState>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    let out: Vec<Value> = sim
        .content()
        .stats
        .values()
        .filter(|d| d.visible)
        .map(|d| json!({ "id": d.id, "name": d.name, "group": d.group, "min": d.min, "max": d.max, "description": d.description }))
        .collect();
    Ok(Json(json!(out)))
}

/// Every class that can be acquired, with its requirements checked on the player's champion.
async fn ui_classes(State(s): State<AppState>, Path(player): Path<String>) -> ApiResult {
    let mut sim = s.sim.lock().unwrap();
    let p = sim.world.resource::<Players>().players.get(&player).cloned().ok_or_else(|| not_found(&player))?;
    let me = p.leader.and_then(|id| sim.entity(id));
    let content = sim.content().clone();
    let held = me.and_then(|e| sim.world.get::<crate::stats::Classes>(e)).map(|c| c.0.clone()).unwrap_or_default();
    let mut out = Vec::new();
    for d in content.classes.values() {
        let Some(req) = &d.requires else { continue };
        let lines = requirement_lines(&mut sim, me, req);
        let eligible = me.is_some_and(|e| crate::classes::meets(&mut sim.world, e, &d.id));
        out.push(json!({
            "id": d.id, "name": d.name, "description": d.description, "group": d.group, "priority": d.priority,
            "held": held.contains(&d.id), "eligible": eligible, "requirements": lines,
            "powers": ability_and_action_names(&content, &d.abilities, &d.actions),
        }));
    }
    let names: Vec<String> = held.iter().map(|c| content.classes.get(c).map_or(c.clone(), |d| d.name.clone())).collect();
    Ok(Json(json!({ "current": names, "classes": out })))
}

#[derive(Deserialize)]
struct ClassBody {
    player: String,
    class: String,
}

async fn ui_take_class(State(s): State<AppState>, Json(b): Json<ClassBody>) -> ApiResult {
    let mut sim = s.sim.lock().unwrap();
    let msg = crate::classes::accept(&mut sim.world, &b.player, &b.class).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok(Json(json!({ "ok": msg })))
}

/// The roles of the world: who holds them, how they are won, their powers and (with `player`) whether the
/// champion meets the requirements.
async fn ui_roles(State(s): State<AppState>, Query(q): Query<ItemQ>) -> ApiResult {
    let mut sim = s.sim.lock().unwrap();
    let me = q.player.as_ref().and_then(|p| sim.world.resource::<Players>().players.get(p).cloned()).and_then(|p| p.leader).and_then(|id| sim.entity(id));
    let content = sim.content().clone();
    let mut out = Vec::new();
    for t in content.titles.values() {
        let holder = crate::titles::holder_entity(&sim.world, &t.id);
        let holder_json = holder.map(|h| json!({ "id": sim.world.get::<SimId>(h), "name": crate::infiltration::apparent_name(&sim.world, h) }));
        let lines = requirement_lines(&mut sim, me, &t.claim_requires);
        let eligible = me.is_some_and(|e| crate::titles::eligible(&mut sim.world, e, t));
        let mine = me.is_some() && holder == me;
        let mode = format!("{:?}", t.mode);
        let can_act = eligible && !mine && (holder.is_none() || matches!(t.mode, crate::content::TitleMode::Challenge | crate::content::TitleMode::Coup));
        let mut powers = ability_and_action_names(&content, &t.abilities, &t.actions);
        powers.extend(t.stats.iter().map(|(k, v)| format!("{} {:+.0}", content.stats.get(k).map_or(k.as_str(), |s| s.name.as_str()), v)));
        out.push(json!({
            "id": t.id, "name": t.name, "description": t.description,
            "faction": content.factions.get(&t.faction).map(|f| f.name.clone()),
            "mode": mode, "holder": holder_json, "mine": mine, "eligible": eligible, "can_act": can_act,
            "requirements": lines, "powers": powers, "victory_points": t.victory_points,
        }));
    }
    Ok(Json(json!(out)))
}

#[derive(Deserialize)]
struct RoleBody {
    player: String,
    title: String,
}

async fn ui_challenge_role(State(s): State<AppState>, Json(b): Json<RoleBody>) -> ApiResult {
    let mut sim = s.sim.lock().unwrap();
    match crate::titles::player_challenge(&mut sim.world, &b.player, &b.title) {
        Ok(m) => Ok(Json(json!({ "ok": m }))),
        Err(e) => Ok(Json(json!({ "error": e }))),
    }
}

#[derive(Deserialize)]
struct ItemQ {
    player: Option<String>,
}

/// An item explained: description, what it does, prices, and who in the player's team holds it.
async fn ui_item(State(s): State<AppState>, Path(item): Path<String>, Query(q): Query<ItemQ>) -> ApiResult {
    let mut sim = s.sim.lock().unwrap();
    let content = sim.content().clone();
    let d = content.items.get(&item).cloned().ok_or_else(|| not_found(&item))?;
    let price = sim.world.resource::<crate::market::Market>().price(&item);
    let mut holders = Vec::new();
    if let Some(p) = q.player.as_ref().and_then(|p| sim.world.resource::<Players>().players.get(p).cloned()) {
        for e in crate::sorted_entities::<FactionMember>(&mut sim.world) {
            if !sim.world.get::<FactionMember>(e).is_some_and(|m| m.faction == p.faction) || sim.world.get::<Dead>(e).is_some() {
                continue;
            }
            let n = crate::inventory_ops::count(&sim.world, e, &item);
            if n > 0 {
                let id = sim.world.get::<SimId>(e).copied();
                holders.push(json!({ "id": id, "name": crate::effects::name_of(&sim.world, e), "qty": n, "building": false }));
            }
        }
        for e in crate::sorted_entities::<crate::buildings::Building>(&mut sim.world) {
            let owned = sim.world.get::<crate::buildings::Building>(e).is_some_and(|b| b.owner == crate::buildings::Owner::Faction(p.faction.clone()));
            let n = if owned { sim.world.get::<crate::inventory::Stock>(e).map_or(0, |s| s.count(&item)) } else { 0 };
            if n > 0 {
                let id = sim.world.get::<SimId>(e).copied();
                holders.push(json!({ "id": id, "name": crate::effects::name_of(&sim.world, e), "qty": n, "building": true }));
            }
        }
    }
    Ok(Json(json!({
        "id": d.id, "name": d.name, "description": d.description, "category": d.category, "tags": d.tags,
        "base_price": d.base_price, "price": price, "usable": !d.on_use.is_empty(), "reusable": d.reusable,
        "stack_max": d.stack_max, "victory_points": d.victory_points, "unique": d.unique,
        "effects": crate::describe::item_effects(&content, &item), "holders": holders,
    })))
}

/// The collections with how many of their pieces the player's team holds (members and faction buildings).
async fn ui_collections(State(s): State<AppState>, Query(q): Query<ItemQ>) -> ApiResult {
    let mut sim = s.sim.lock().unwrap();
    let content = sim.content().clone();
    let faction = q.player.as_ref().and_then(|p| sim.world.resource::<Players>().players.get(p)).map(|p| p.faction.clone());
    let held = faction.as_ref().map(|f| crate::inventory_ops::faction_holdings(&mut sim.world, f)).unwrap_or_default();
    let out: Vec<Value> = content
        .collections
        .values()
        .map(|c| {
            // A collection lists its pieces, or asks for `count` different items with a tag.
            let mut pieces: Vec<String> = c.items.clone();
            if let Some(t) = &c.tag {
                pieces.extend(content.items.values().filter(|d| d.tags.contains(t)).map(|d| d.id.clone()));
            }
            let items: Vec<Value> = pieces
                .iter()
                .map(|i| {
                    let d = content.items.get(i);
                    json!({ "id": i, "name": d.map_or(i.clone(), |d| d.name.clone()), "category": d.map(|d| d.category.clone()),
                            "qty": held.get(i).copied().unwrap_or(0) })
                })
                .collect();
            let owned = items.iter().filter(|i| i["qty"].as_u64().unwrap_or(0) > 0).count();
            let total = if c.items.is_empty() && c.count > 0 { c.count as usize } else { items.len() };
            json!({
                "id": c.id, "name": c.name, "description": c.description, "victory_points": c.victory_points,
                "per_faction": c.per_faction, "total": total, "owned": owned.min(total), "complete": owned >= total && total > 0,
                "items": items,
            })
        })
        .collect();
    Ok(Json(json!(out)))
}

/// State of the world's resources: price index and inflation, money in circulation, disruptions and
/// circumstances in force, every good with its price and trend.
async fn ui_economy(State(s): State<AppState>) -> ApiResult {
    let mut sim = s.sim.lock().unwrap();
    let content = sim.content().clone();
    let tick = sim.tick_count();
    let market = sim.world.resource::<crate::market::Market>().clone();
    // Index = mean of price/base; inflation = mean change over the last day (24 ticks of history).
    let (mut index, mut day, mut n, mut nd) = (0.0f64, 0.0f64, 0usize, 0usize);
    let mut goods: Vec<Value> = Vec::new();
    for (id, m) in &market.items {
        index += m.price / m.base.max(0.01);
        n += 1;
        let ago = (m.history.len() > 24).then(|| m.history[m.history.len() - 25]);
        if let Some(a) = ago.filter(|a| *a > 0.0) {
            day += m.price / a - 1.0;
            nd += 1;
        }
        let item = content.items.get(id);
        goods.push(json!({
            "id": id, "name": item.map_or(id.clone(), |d| d.name.clone()), "category": item.map(|d| d.category.clone()),
            "price": m.price, "base": m.base, "day_change": ago.map(|a| if a > 0.0 { m.price / a - 1.0 } else { 0.0 }),
            "supply": m.supply, "demand": m.demand, "shocks": m.shocks.iter().map(|s| s.source.clone()).collect::<Vec<_>>(),
        }));
    }
    let wallets: f64 = sim.world.query::<&crate::stats::Wallet>().iter(&sim.world).map(|w| w.0).sum();
    let treasuries: f64 = sim.world.resource::<Factions>().states.values().map(|f| f.treasury).sum();
    let mods = sim.world.resource::<crate::buildings::GlobalModifiers>().clone();
    let circumstances: Vec<Value> = mods
        .active
        .iter()
        .map(|m| json!({ "id": m.id, "name": m.name, "ticks_left": m.until.saturating_sub(tick), "disruption": m.logistics_disruption, "morale": m.morale }))
        .collect();
    let supply = sim.world.resource::<crate::supply::SupplyStats>().clone();
    let local: Vec<Value> = content
        .supplies
        .keys()
        .filter_map(|k| supply.local_share(k).map(|v| json!({ "id": k, "item": content.supplies[k].item, "local_share": v })))
        .collect();
    Ok(Json(json!({
        "tick": tick, "price_index": if n > 0 { index / n as f64 } else { 1.0 },
        "inflation_day": if nd > 0 { day / nd as f64 } else { 0.0 },
        "money": { "wallets": wallets, "treasuries": treasuries, "total": wallets + treasuries },
        "disruption": market.disruption, "circumstances": circumstances, "local_production": local, "goods": goods,
    })))
}

#[derive(Deserialize)]
struct PathBody {
    #[serde(default = "default_save")]
    path: String,
}

fn default_save() -> String {
    "saves/quick.json".into()
}

async fn save_game(State(s): State<AppState>, Json(b): Json<PathBody>) -> ApiResult {
    if let Some(dir) = std::path::Path::new(&b.path).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut sim = s.sim.lock().unwrap();
    sim.save_to_file(&b.path).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "ok": format!("partita salvata in {} (tick {})", b.path, sim.tick_count()) })))
}

async fn load_game(State(s): State<AppState>, Json(b): Json<PathBody>) -> ApiResult {
    let factory = s.factory.clone().ok_or((StatusCode::NOT_IMPLEMENTED, "caricamento non disponibile".to_string()))?;
    let save = Simulation::read_save(&b.path).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let builder = factory().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let new = builder.build_from_save(&save).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let tick = new.tick_count();
    *s.sim.lock().unwrap() = new;
    Ok(Json(json!({ "ok": format!("partita caricata da {} (tick {tick})", b.path) })))
}

fn not_found(what: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("{what} non trovato"))
}

#[derive(Deserialize, Default)]
struct TruthQ {
    #[serde(default)]
    truth: bool,
}

async fn index() -> Html<&'static str> {
    Html(concat!(
        "<h1>sim_core admin</h1><p><a href='/admin/'><b>Console di amministrazione</b></a></p><ul>",
        "<li><a href='/ui/'>/ui/</a> client</li>",
        "<li><a href='/api/state'>/api/state</a> (?truth=true per la vista admin)</li>",
        "<li><a href='/api/entities'>/api/entities</a>, /api/entities/{id}, /api/entities/{id}/ai</li>",
        "<li><a href='/api/events'>/api/events</a> (?since=&amp;kind=)</li>",
        "<li><a href='/api/feed'>/api/feed</a>, <a href='/api/market'>/api/market</a>, <a href='/api/factions'>/api/factions</a>, <a href='/api/jobs'>/api/jobs</a>, <a href='/api/squads'>/api/squads</a></li>",
        "<li><a href='/api/params'>/api/params</a> (PUT /api/params/{key} {\"value\": x})</li>",
        "<li><a href='/api/sprites'>/api/sprites</a> (PUT /api/sprites/{id})</li>",
        "<li>POST /api/commands (SimCommand JSON, ?now=true), GET /api/commands/{seq}</li>",
        "<li><a href='/api/control'>/api/control</a> (POST {\"paused\":bool,\"tick_ms\":n})</li>",
        "<li><a href='/api/compendium'>/api/compendium</a>, <a href='/api/extensions'>/api/extensions</a></li>",
        "<li><a href='/api/ui/activity'>/api/ui/activity</a>, <a href='/api/ui/map'>/api/ui/map</a>, <a href='/api/ui/state'>/api/ui/state</a></li>",
        "<li><a href='/metrics'>/metrics</a> (Prometheus)</li>",
        "<li>POST /mcp (MCP JSON-RPC 2.0)</li></ul>"
    ))
}

async fn admin_page() -> Html<&'static str> {
    Html(include_str!("admin.html"))
}

/// Every loaded definition (items, buildings, races, jobs, actions, factions, templates, placements…).
async fn content_all(State(s): State<AppState>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    let c: &crate::content::ContentData = sim.content();
    Ok(Json(serde_json::to_value(c).unwrap_or_default()))
}

async fn assets_list(State(s): State<AppState>) -> ApiResult {
    let mut files: Vec<String> = s
        .assets
        .as_ref()
        .and_then(|d| std::fs::read_dir(d).ok())
        .map(|rd| rd.filter_map(|e| e.ok()?.file_name().into_string().ok()).filter(|n| n.ends_with(".png")).collect())
        .unwrap_or_default();
    files.sort();
    Ok(Json(json!({ "available": s.assets.is_some(), "files": files })))
}

async fn asset_file(State(s): State<AppState>, Path(file): Path<String>) -> Response {
    let ok_name = file.ends_with(".png") && !file.contains('/') && !file.contains('\\') && !file.contains("..");
    match s.assets.as_ref().filter(|_| ok_name).and_then(|d| std::fs::read(d.join(&file)).ok()) {
        Some(bytes) => ([(header::CONTENT_TYPE, "image/png"), (header::CACHE_CONTROL, "max-age=60")], bytes).into_response(),
        None => (StatusCode::NOT_FOUND, "immagine non trovata").into_response(),
    }
}

async fn metrics(State(s): State<AppState>) -> Response {
    match &s.metrics {
        Some(h) => ([(header::CONTENT_TYPE, "text/plain; version=0.0.4")], h.render()).into_response(),
        None => (StatusCode::SERVICE_UNAVAILABLE, "recorder non installato").into_response(),
    }
}

async fn state_all(State(s): State<AppState>, Query(q): Query<TruthQ>) -> ApiResult {
    let snap = s.sim.lock().unwrap().snapshot(q.truth);
    Ok(Json(serde_json::to_value(snap).unwrap_or_default()))
}

async fn entities(State(s): State<AppState>, Query(q): Query<TruthQ>) -> ApiResult {
    let snap = s.sim.lock().unwrap().snapshot(q.truth);
    let list: Vec<Value> = snap
        .entities
        .iter()
        .map(|e| json!({ "id": e.id, "name": e.name, "kind": e.kind, "faction": e.faction, "race": e.race, "pos": e.pos, "dead": e.dead,
                         "activity": e.activity.as_ref().map(|a| &a.label) }))
        .collect();
    Ok(Json(json!(list)))
}

async fn entity(State(s): State<AppState>, Path(id): Path<u64>, Query(q): Query<TruthQ>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    let e = sim.entity(SimId(id)).ok_or_else(|| not_found(id))?;
    let v = crate::snapshot::entity_view(&sim.world, e, q.truth).ok_or_else(|| not_found(id))?;
    Ok(Json(serde_json::to_value(v).unwrap_or_default()))
}

async fn entity_ai(State(s): State<AppState>, Path(id): Path<u64>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    let e = sim.entity(SimId(id)).ok_or_else(|| not_found(id))?;
    Ok(Json(crate::snapshot::ai_inspect(&sim.world, e)))
}

#[derive(Deserialize)]
struct EventsQ {
    #[serde(default)]
    since: u64,
    kind: Option<String>,
    #[serde(default = "hundred")]
    limit: usize,
}

fn hundred() -> usize {
    100
}

async fn events(State(s): State<AppState>, Query(q): Query<EventsQ>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    let evs: Vec<&SimEvent> = sim
        .events()
        .since(q.since)
        .iter()
        .filter(|e| q.kind.as_ref().is_none_or(|k| &e.kind == k))
        .collect();
    let start = evs.len().saturating_sub(q.limit);
    Ok(Json(json!({ "last_id": sim.events().last_id(), "events": evs[start..] })))
}

async fn feed(State(s): State<AppState>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    let f = sim.world.resource::<Feed>();
    Ok(Json(json!({ "name": f.name, "articles": f.articles.iter().rev().collect::<Vec<_>>() })))
}

async fn market(State(s): State<AppState>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    Ok(Json(serde_json::to_value(sim.world.resource::<Market>()).unwrap_or_default()))
}

async fn factions(State(s): State<AppState>) -> ApiResult {
    let snap = s.sim.lock().unwrap().snapshot(true);
    Ok(Json(json!({ "factions": snap.factions, "scores": snap.scores, "players": snap.players, "titles": snap.titles })))
}

async fn jobs(State(s): State<AppState>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    Ok(Json(serde_json::to_value(sim.world.resource::<JobBoard>()).unwrap_or_default()))
}

async fn squads(State(s): State<AppState>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    Ok(Json(serde_json::to_value(sim.world.resource::<Squads>()).unwrap_or_default()))
}

async fn params(State(s): State<AppState>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    let p = sim.world.resource::<Params>();
    let list: Vec<Value> = p
        .all()
        .iter()
        .map(|(k, v)| json!({ "key": k, "value": v, "description": Params::describe(k) }))
        .collect();
    Ok(Json(json!(list)))
}

#[derive(Deserialize)]
struct ValueBody {
    value: f64,
}

async fn set_param(State(s): State<AppState>, Path(key): Path<String>, Json(b): Json<ValueBody>) -> ApiResult {
    let res = s.sim.lock().unwrap().execute(SimCommand::SetParam { key, value: b.value });
    res.map(|m| Json(json!({ "ok": m }))).map_err(|e| (StatusCode::BAD_REQUEST, e))
}

async fn sprites(State(s): State<AppState>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    Ok(Json(serde_json::to_value(sim.world.resource::<SpriteMapping>()).unwrap_or_default()))
}

async fn set_sprite(State(s): State<AppState>, Path(id): Path<String>, Json(sprite): Json<crate::content::SpriteDef>) -> ApiResult {
    let res = s.sim.lock().unwrap().execute(SimCommand::SetSprite { id, sprite });
    res.map(|m| Json(json!({ "ok": m }))).map_err(|e| (StatusCode::BAD_REQUEST, e))
}

#[derive(Deserialize, Default)]
struct NowQ {
    #[serde(default)]
    now: bool,
}

async fn command(State(s): State<AppState>, Query(q): Query<NowQ>, Json(cmd): Json<SimCommand>) -> ApiResult {
    let mut sim = s.sim.lock().unwrap();
    if q.now {
        return sim.execute(cmd).map(|m| Json(json!({ "ok": m }))).map_err(|e| (StatusCode::BAD_REQUEST, e));
    }
    let seq = sim.enqueue(cmd);
    Ok(Json(json!({ "queued": seq, "applies_at_tick": sim.tick_count() })))
}

async fn command_result(State(s): State<AppState>, Path(seq): Path<u64>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    match sim.world.resource::<CommandResults>().0.get(&seq) {
        Some(Ok(m)) => Ok(Json(json!({ "seq": seq, "ok": m }))),
        Some(Err(e)) => Ok(Json(json!({ "seq": seq, "error": e }))),
        None => Ok(Json(json!({ "seq": seq, "pending": true }))),
    }
}

async fn control_get(State(s): State<AppState>) -> ApiResult {
    let c = s.control.lock().unwrap().clone();
    let tick = s.sim.lock().unwrap().tick_count();
    Ok(Json(json!({ "paused": c.paused, "tick_ms": c.tick_ms, "tick": tick })))
}

#[derive(Deserialize)]
struct ControlBody {
    paused: Option<bool>,
    tick_ms: Option<u64>,
    /// Run this many ticks right now (works while paused).
    step: Option<u64>,
}

async fn control_set(State(s): State<AppState>, Json(b): Json<ControlBody>) -> ApiResult {
    {
        let mut c = s.control.lock().unwrap();
        if let Some(p) = b.paused {
            c.paused = p;
        }
        if let Some(ms) = b.tick_ms {
            c.tick_ms = ms.max(1);
        }
    }
    if let Some(n) = b.step {
        s.sim.lock().unwrap().run(n.min(10_000));
    }
    control_get(State(s)).await
}

async fn compendium(State(s): State<AppState>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    Ok(Json(crate::compendium::build(sim.content(), sim.world.resource::<Params>())))
}

async fn extensions(State(s): State<AppState>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    Ok(Json(json!({ "plugins": sim.plugins, "registered": sim.world.resource::<crate::extensions::Extensions>().names() })))
}

async fn ui_activity(State(s): State<AppState>) -> ApiResult {
    let snap = s.sim.lock().unwrap().snapshot(false);
    let list: Vec<Value> = snap
        .entities
        .iter()
        .filter(|e| e.kind != "building")
        .map(|e| json!({ "id": e.id, "name": e.name, "pos": e.pos, "activity": e.activity }))
        .collect();
    Ok(Json(json!({ "tick": snap.tick, "pawns": list })))
}

async fn ui_map(State(s): State<AppState>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    let m = sim.world.resource::<WorldMap>();
    let env = sim.world.resource::<crate::map::Environment>();
    Ok(Json(json!({ "layers": m.layers, "zones": m.zones, "portals": m.portals, "networks": m.networks, "network_load": env.network_load,
                     "legend": m.legend, "props": m.props })))
}

#[derive(Deserialize, Default)]
struct SinceQ {
    #[serde(default)]
    since: usize,
}

/// Terrain changed since the client's last look (digging, admin painting): `[[layer, x, y, "ch"], …]`.
/// When `total` is lower than `since` the world was reloaded and the client should fetch the whole map.
async fn ui_terrain(State(s): State<AppState>, Query(q): Query<SinceQ>) -> ApiResult {
    let sim = s.sim.lock().unwrap();
    let ch = &sim.world.resource::<crate::map::TerrainChanges>().0;
    let changes: Vec<Value> = ch.iter().skip(q.since).map(|(p, c)| json!([p.layer, p.x, p.y, c.to_string()])).collect();
    Ok(Json(json!({ "total": ch.len(), "changes": changes })))
}

#[derive(Deserialize, Default)]
struct UiQ {
    /// Apply the fog of war of this faction (default: no fog).
    faction: Option<String>,
    /// Light state for the game client: what the map needs of each entity (the card asks the rest),
    /// only the dirty cells as `[layer, x, y, dirt × 10, pathogens]`, no sprite mapping (unless `since` is
    /// missing), no chronicle.
    #[serde(default)]
    lite: bool,
    /// Light state only: the tick the client already has; same tick → `{"unchanged": true}`.
    since: Option<u64>,
}

/// Everything the client needs each frame: apparent entities, feed, market, factions, sprites.
async fn ui_state(State(s): State<AppState>, Query(q): Query<UiQ>) -> Response {
    if q.lite {
        return ui_state_lite(&s, &q);
    }
    let mut sim = s.sim.lock().unwrap();
    let mut snap = sim.snapshot(false);
    let fog = q.faction.as_ref().map(|f| crate::snapshot::fog_of_war(&mut sim.world, f));
    if let Some(f) = &fog {
        snap.entities.retain(|e| f.visible.contains(&e.id));
    }
    let sprites = sim.world.resource::<SpriteMapping>().clone();
    let cells: Vec<Value> = sim
        .world
        .resource::<crate::map::Environment>()
        .cells
        .iter()
        .map(|(p, c)| json!({ "pos": p, "dirt": c.dirt, "pathogens": c.pathogens.len() }))
        .collect();
    let recent: Vec<SimEvent> = sim
        .events()
        .all()
        .iter()
        .rev()
        .filter(|e| e.newsworthiness >= 0.3 || e.kind == kind::ARTICLE)
        .take(30)
        .cloned()
        .collect();
    Json(json!({ "snapshot": snap, "sprites": sprites, "cells": cells, "recent_events": recent,
                 "fog": fog.map(|f| json!({ "faction": f.faction, "observers": f.observers })) }))
    .into_response()
}

/// The light state, built once per tick and fog faction and shared by every client asking for it.
fn ui_state_lite(s: &AppState, q: &UiQ) -> Response {
    let key = q.faction.clone().unwrap_or_default();
    let tick = s.sim.lock().unwrap().tick_count();
    if q.since == Some(tick) {
        return Json(json!({ "unchanged": true, "tick": tick })).into_response();
    }
    let cached = s.ui_cache.lock().unwrap().get(&key).filter(|(t, _)| *t == tick).map(|(_, j)| j.clone());
    let body = match cached {
        Some(j) => j,
        None => {
            let j = Arc::new(build_lite(&mut s.sim.lock().unwrap(), q.faction.as_deref()).to_string());
            s.ui_cache.lock().unwrap().insert(key, (tick, j.clone()));
            j
        }
    };
    let body = if q.since.is_none() {
        // First request: the sprite mapping too (it rarely changes, so later requests go without it).
        let sprites = serde_json::to_string(s.sim.lock().unwrap().world.resource::<SpriteMapping>()).unwrap_or_else(|_| "{}".into());
        format!("{{\"sprites\":{sprites},{}", &body[1..])
    } else {
        body.as_str().to_string()
    };
    ([(header::CONTENT_TYPE, "application/json")], body).into_response()
}

fn build_lite(sim: &mut Simulation, faction: Option<&str>) -> Value {
    let mut snap = sim.snapshot(false);
    let fog = faction.map(|f| crate::snapshot::fog_of_war(&mut sim.world, f));
    if let Some(f) = &fog {
        snap.entities.retain(|e| f.visible.contains(&e.id));
    }
    let entities: Vec<Value> = snap
        .entities
        .iter()
        .map(|e| {
            json!({
                "id": e.id, "kind": e.kind, "name": e.name, "template": e.template, "race": e.race, "classes": e.classes,
                "faction": e.faction, "pos": e.pos, "dead": e.dead, "sex": e.sex,
                "activity": e.activity.as_ref().map(|a| json!({ "label": a.label, "progress": a.progress, "flags": a.flags, "mood": a.mood })),
                "building": e.building.as_ref().map(|b| json!({ "def": b.def, "hp": b.hp, "max_hp": b.max_hp, "owner": b.owner })),
            })
        })
        .collect();
    let cells: Vec<Value> = sim
        .world
        .resource::<crate::map::Environment>()
        .cells
        .iter()
        .filter(|(_, c)| c.dirt >= 0.3 || !c.pathogens.is_empty())
        // Compact: [layer, x, y, dirt × 10, pathogens].
        .map(|(p, c)| json!([p.layer, p.x, p.y, (c.dirt * 10.0).round() as i32, c.pathogens.len()]))
        .collect();
    json!({
        "tick": snap.tick,
        "snapshot": {
            "tick": snap.tick, "entities": entities, "factions": snap.factions, "scores": snap.scores,
            "territories": snap.territories, "players": snap.players, "winner": snap.winner, "titles": snap.titles,
        },
        "cells": cells,
        "fog": fog.map(|f| json!({ "faction": f.faction, "observers": f.observers })),
    })
}

async fn ui_page(State(s): State<AppState>) -> Response {
    match &s.ui_html {
        Some(h) => Html(h.as_str().to_string()).into_response(),
        None => (StatusCode::NOT_FOUND, "nessun client configurato").into_response(),
    }
}

async fn mcp_http(State(s): State<AppState>, Json(req): Json<Value>) -> Response {
    match mcp::handle(&s.sim, req) {
        Some(resp) => Json(resp).into_response(),
        None => StatusCode::ACCEPTED.into_response(),
    }
}
