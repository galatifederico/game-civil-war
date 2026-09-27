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

#[derive(Clone)]
pub struct AppState {
    pub sim: SharedSim,
    pub control: Arc<Mutex<Control>>,
    pub metrics: Option<PrometheusHandle>,
    /// Optional static client served on `/ui/` (index.html content).
    pub ui_html: Option<Arc<String>>,
}

/// Installs the global Prometheus recorder (once per process).
pub fn install_metrics() -> Option<PrometheusHandle> {
    PrometheusBuilder::new().install_recorder().ok()
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
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
        .route("/api/ui/state", get(ui_state))
        .route("/ui/", get(ui_page))
        .route("/mcp", post(mcp_http))
        .with_state(state)
}

/// Runs the ticker and the HTTP server until Ctrl+C.
pub async fn serve(state: AppState, addr: SocketAddr) -> std::io::Result<()> {
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
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("admin/API/MCP su http://{addr}");
    let res = axum::serve(listener, router(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await;
    ticker.abort();
    res
}

type ApiResult = Result<Json<Value>, (StatusCode, String)>;

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
        "<h1>sim_core admin</h1><ul>",
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
    Ok(Json(json!({ "layers": m.layers, "zones": m.zones, "portals": m.portals, "networks": m.networks, "network_load": env.network_load })))
}

/// Everything the client needs each frame: apparent entities, feed, market, factions, sprites.
async fn ui_state(State(s): State<AppState>) -> ApiResult {
    let mut sim = s.sim.lock().unwrap();
    let snap = sim.snapshot(false);
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
    Ok(Json(json!({ "snapshot": snap, "sprites": sprites, "cells": cells, "recent_events": recent })))
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
