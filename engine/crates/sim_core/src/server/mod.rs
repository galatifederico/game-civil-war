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
}

/// Installs the global Prometheus recorder (once per process).
pub fn install_metrics() -> Option<PrometheusHandle> {
    PrometheusBuilder::new().install_recorder().ok()
}

pub fn router(state: AppState) -> Router {
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
                    sim.lock().unwrap().tick();
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
    let members: Vec<Value> = crate::sorted_entities::<FactionMember>(&mut sim.world)
        .into_iter()
        .filter(|e| sim.world.get::<FactionMember>(*e).is_some_and(|m| m.faction == p.faction) && sim.world.get::<Dead>(*e).is_none())
        .filter_map(|e| {
            let v = crate::snapshot::entity_view(&sim.world, e, true)?;
            Some(json!({
                "id": v.id, "name": v.name, "rank": v.rank, "classes": v.classes, "pos": v.pos,
                "activity": v.activity.as_ref().map(|a| a.label.clone()),
                "obedience": crate::player::obedience(&sim.world, e),
                "champion": v.leader_of.as_deref() == Some(player.as_str()),
                "work": sim.world.get::<crate::jobs::WorkPriorities>(e).map(|w| w.matrix()),
            }))
        })
        .collect();
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
    let mine = |a: &crate::press::Article| faction.as_ref().is_some_and(|f| a.concerns(f));
    let articles: Vec<Value> = feed
        .articles
        .iter()
        .rev()
        .filter(|a| match filter.as_str() {
            "all" => true,
            "important" => a.importance >= threshold || mine(a),
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
}

/// Everything the client needs each frame: apparent entities, feed, market, factions, sprites.
async fn ui_state(State(s): State<AppState>, Query(q): Query<UiQ>) -> ApiResult {
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
    Ok(Json(json!({ "snapshot": snap, "sprites": sprites, "cells": cells, "recent_events": recent,
                     "fog": fog.map(|f| json!({ "faction": f.faction, "observers": f.observers })) })))
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
