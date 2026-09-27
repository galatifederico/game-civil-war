//! Model Context Protocol server (JSON-RPC 2.0): lets an external AI inspect and drive the live
//! simulation. Tools: `get_world_state`, `set_parameter`, `spawn_entity`, `trigger_event`, plus
//! `list_entities`, `inspect_entity`, `send_command`, `recent_events`.

use std::io::{BufRead, Write};

use serde_json::{json, Value};

use super::SharedSim;
use crate::prelude::*;

pub const PROTOCOL_VERSION: &str = "2025-06-18";

fn tools() -> Value {
    json!([
        {
            "name": "get_world_state",
            "description": "Stato del mondo: tick, fazioni, punteggi, mercato, feed e (con detail=true) tutte le entità. truth=true mostra le identità nascoste.",
            "inputSchema": { "type": "object", "properties": {
                "truth": { "type": "boolean", "default": false },
                "detail": { "type": "boolean", "default": false }
            }}
        },
        {
            "name": "set_parameter",
            "description": "Imposta un parametro di bilanciamento live (es. crime.arrest_threshold, market.elasticity).",
            "inputSchema": { "type": "object", "required": ["key", "value"], "properties": {
                "key": { "type": "string" }, "value": { "type": "number" }
            }}
        },
        {
            "name": "spawn_entity",
            "description": "Crea entità da un template dei contenuti in una zona.",
            "inputSchema": { "type": "object", "required": ["template"], "properties": {
                "template": { "type": "string" }, "zone": { "type": "string" },
                "count": { "type": "integer", "default": 1 }, "faction": { "type": "string" }, "name": { "type": "string" }
            }}
        },
        {
            "name": "trigger_event",
            "description": "Esegue un trigger dei contenuti (id), oppure emette un evento libero (kind + message) che la stampa può raccogliere.",
            "inputSchema": { "type": "object", "properties": {
                "id": { "type": "string" }, "kind": { "type": "string" }, "message": { "type": "string" },
                "newsworthiness": { "type": "number" }
            }}
        },
        {
            "name": "list_entities",
            "description": "Elenco compatto delle entità (id, nome, fazione apparente, attività).",
            "inputSchema": { "type": "object", "properties": { "faction": { "type": "string" } } }
        },
        {
            "name": "inspect_entity",
            "description": "Scheda completa di un'entità con la valutazione della Utility AI.",
            "inputSchema": { "type": "object", "required": ["id"], "properties": { "id": { "type": "integer" } } }
        },
        {
            "name": "send_command",
            "description": "Invia un SimCommand (JSON con campo type, es. {\"type\":\"bribe\",\"faction\":\"x\",\"target\":12}). Applicato subito.",
            "inputSchema": { "type": "object", "required": ["command"], "properties": { "command": { "type": "object" } } }
        },
        {
            "name": "recent_events",
            "description": "Ultimi eventi della simulazione.",
            "inputSchema": { "type": "object", "properties": { "limit": { "type": "integer", "default": 30 }, "kind": { "type": "string" } } }
        }
    ])
}

fn ok(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn err(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn text(v: Value, is_error: bool) -> Value {
    let t = if let Value::String(s) = &v { s.clone() } else { serde_json::to_string_pretty(&v).unwrap_or_default() };
    json!({ "content": [{ "type": "text", "text": t }], "isError": is_error })
}

/// Handles one JSON-RPC message. Returns None for notifications.
pub fn handle(sim: &SharedSim, req: Value) -> Option<Value> {
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or_default().to_string();
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    let id = match id {
        Some(id) => id,
        None => return None, // notification (e.g. notifications/initialized)
    };
    let resp = match method.as_str() {
        "initialize" => ok(
            &id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "sim_core", "version": env!("CARGO_PKG_VERSION") },
                "instructions": "Simulazione headless live. Usa get_world_state per orientarti, poi gli altri tool."
            }),
        ),
        "ping" => ok(&id, json!({})),
        "tools/list" => ok(&id, json!({ "tools": tools() })),
        "tools/call" => {
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or_default();
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            let (v, is_err) = match call(sim, name, &args) {
                Ok(v) => (v, false),
                Err(e) => (Value::String(e), true),
            };
            ok(&id, text(v, is_err))
        }
        _ => err(&id, -32601, &format!("metodo sconosciuto: {method}")),
    };
    Some(resp)
}

fn call(sim: &SharedSim, name: &str, a: &Value) -> Result<Value, String> {
    let mut sim = sim.lock().map_err(|_| "simulazione non disponibile")?;
    let s = |k: &str| a.get(k).and_then(|v| v.as_str()).map(str::to_string);
    match name {
        "get_world_state" => {
            let truth = a.get("truth").and_then(|v| v.as_bool()).unwrap_or(false);
            let detail = a.get("detail").and_then(|v| v.as_bool()).unwrap_or(false);
            let snap = sim.snapshot(truth);
            let mut v = serde_json::to_value(&snap).map_err(|e| e.to_string())?;
            if !detail {
                let alive = snap.entities.iter().filter(|e| !e.dead && e.kind == "pawn").count();
                v["entities"] = json!({ "pawns_alive": alive, "total": snap.entities.len(), "hint": "usa detail=true o list_entities" });
            }
            Ok(v)
        }
        "set_parameter" => {
            let key = s("key").ok_or("key mancante")?;
            let value = a.get("value").and_then(|v| v.as_f64()).ok_or("value mancante")?;
            sim.execute(SimCommand::SetParam { key, value }).map(Value::String)
        }
        "spawn_entity" => {
            let template = s("template").ok_or("template mancante")?;
            let count = a.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
            sim.execute(SimCommand::Spawn { template, zone: s("zone"), count, faction: s("faction"), name: s("name") }).map(Value::String)
        }
        "trigger_event" => {
            if let Some(id) = s("id") {
                return sim.execute(SimCommand::FireTrigger { id }).map(Value::String);
            }
            let kind = s("kind").ok_or("serve id (trigger) oppure kind + message")?;
            let message = s("message").unwrap_or_else(|| kind.clone());
            let news = a.get("newsworthiness").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
            let tick = sim.tick_count();
            let ev = sim.world.resource_mut::<EventLog>().push(tick, EventBuilder::new(kind, message).news(news).tags(["mcp"]));
            Ok(json!({ "event": ev }))
        }
        "list_entities" => {
            let snap = sim.snapshot(false);
            let f = s("faction");
            Ok(json!(snap
                .entities
                .iter()
                .filter(|e| f.is_none() || e.faction == f)
                .map(|e| json!({ "id": e.id, "name": e.name, "kind": e.kind, "faction": e.faction, "dead": e.dead,
                                 "activity": e.activity.as_ref().map(|x| &x.label) }))
                .collect::<Vec<_>>()))
        }
        "inspect_entity" => {
            let id = a.get("id").and_then(|v| v.as_u64()).ok_or("id mancante")?;
            let e = sim.entity(SimId(id)).ok_or("entità inesistente")?;
            let view = crate::snapshot::entity_view(&sim.world, e, true);
            Ok(json!({ "entity": view, "ai": crate::snapshot::ai_inspect(&sim.world, e) }))
        }
        "send_command" => {
            let cmd: SimCommand = serde_json::from_value(a.get("command").cloned().unwrap_or_default()).map_err(|e| e.to_string())?;
            sim.execute(cmd).map(Value::String)
        }
        "recent_events" => {
            let limit = a.get("limit").and_then(|v| v.as_u64()).unwrap_or(30) as usize;
            let k = s("kind");
            let evs: Vec<&SimEvent> = sim.events().all().iter().rev().filter(|e| k.as_ref().is_none_or(|x| &e.kind == x)).take(limit).collect();
            Ok(json!(evs))
        }
        _ => Err(format!("tool sconosciuto: {name}")),
    }
}

/// MCP over stdio: one JSON-RPC message per line on stdin, responses on stdout.
pub fn serve_stdio(sim: SharedSim) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let resp = match serde_json::from_str::<Value>(&line) {
            Ok(req) => handle(&sim, req),
            Err(e) => Some(err(&Value::Null, -32700, &format!("parse error: {e}"))),
        };
        if let Some(r) = resp {
            writeln!(out, "{}", serde_json::to_string(&r).unwrap_or_default())?;
            out.flush()?;
        }
    }
    Ok(())
}
