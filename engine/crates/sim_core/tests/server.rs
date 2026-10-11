//! Admin REST API, UI API with fog of war, and MCP over a real socket.

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use sim_core::prelude::*;
use sim_core::server::{mcp, AppState, Control};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn town() -> Simulation {
    let mut b = Simulation::builder(3);
    b.load_pack_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/town")).unwrap();
    b.build().unwrap()
}

async fn http(addr: std::net::SocketAddr, method: &str, path: &str, body: Option<Value>) -> (u16, Value) {
    let mut s = tokio::net::TcpStream::connect(addr).await.unwrap();
    let body = body.map(|b| b.to_string()).unwrap_or_default();
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    s.write_all(req.as_bytes()).await.unwrap();
    let mut buf = Vec::new();
    s.read_to_end(&mut buf).await.unwrap();
    let text = String::from_utf8_lossy(&buf).to_string();
    let status: u16 = text[9..12].parse().unwrap();
    let (_, rest) = text.split_once("\r\n\r\n").unwrap();
    // Chunked or plain: find the JSON object in the body.
    let start = rest.find(['{', '[']).unwrap_or(0);
    let end = rest.rfind(['}', ']']).map_or(rest.len(), |i| i + 1);
    (status, serde_json::from_str(&rest[start..end]).unwrap_or(Value::Null))
}

#[tokio::test]
async fn rest_api_and_mcp() {
    let sim = Arc::new(Mutex::new(town()));
    let state = AppState { sim: sim.clone(), control: Arc::new(Mutex::new(Control { paused: true, tick_ms: 1000 })), metrics: None, ui_html: None, factory: None, token: None, assets: None, ui_cache: Default::default() };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(sim_core::server::serve_on(state, listener));

    let (st, v) = http(addr, "GET", "/api/state?truth=true", None).await;
    assert_eq!(st, 200);
    assert_eq!(v["tick"], 0);
    let (_, v) = http(addr, "POST", "/api/control", Some(json!({ "step": 5 }))).await;
    assert_eq!(v["tick"], 5);

    let (_, ents) = http(addr, "GET", "/api/entities", None).await;
    let thief = ents.as_array().unwrap().iter().find(|e| e["name"].as_str().unwrap().starts_with("Thief")).unwrap()["id"].as_u64().unwrap();
    let (st, ai) = http(addr, "GET", &format!("/api/entities/{thief}/ai"), None).await;
    assert_eq!(st, 200);
    assert!(ai["scores"].is_array());

    let (st, _) = http(addr, "PUT", "/api/params/crime.arrest_threshold", Some(json!({ "value": 99.0 }))).await;
    assert_eq!(st, 200);
    assert_eq!(sim.lock().unwrap().world.resource::<Params>().get("crime.arrest_threshold", 0.0), 99.0);

    let (_, q) = http(addr, "POST", "/api/commands", Some(json!({ "type": "set_salary", "faction": "town", "rank": "member", "amount": 9 }))).await;
    assert!(q["queued"].as_u64().is_some());
    let (st, _) = http(addr, "PUT", "/api/sprites/race:human", Some(json!({ "shape": "square", "color": "#fff" }))).await;
    assert_eq!(st, 200);

    let (_, ui) = http(addr, "GET", "/api/ui/state?faction=press", None).await;
    let seen = ui["snapshot"]["entities"].as_array().unwrap().len();
    let (_, all) = http(addr, "GET", "/api/ui/state", None).await;
    assert!(seen <= all["snapshot"]["entities"].as_array().unwrap().len());
    assert!(ui["fog"]["observers"].as_array().is_some_and(|o| !o.is_empty()));

    let (st, pl) = http(addr, "GET", "/api/ui/player/p1", None).await;
    assert_eq!(st, 200);
    assert_eq!(pl["faction"], "town");
    assert!(pl["members"].as_array().unwrap().iter().any(|m| m["champion"] == true));
    let champ = pl["champion"].as_u64().unwrap();
    let (_, acts) = http(addr, "GET", &format!("/api/ui/actions/{champ}"), None).await;
    assert!(acts.as_array().unwrap().iter().any(|a| a["kind"] == "move"));
    let (_, r) = http(addr, "POST", "/api/commands?now=true",
        Some(json!({ "type": "player_order", "player": "p1", "entity": champ, "order": { "kind": "move", "pos": { "layer": 0, "x": 2, "y": 2 } } }))).await;
    assert!(r["ok"].is_string(), "{r}");
    let (_, comp) = http(addr, "GET", "/api/compendium", None).await;
    assert!(comp["items"].as_array().is_some_and(|i| i.len() == 3));

    // MCP over HTTP.
    let (_, init) = http(addr, "POST", "/mcp", Some(json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }))).await;
    assert_eq!(init["result"]["protocolVersion"], mcp::PROTOCOL_VERSION);
    let (_, list) = http(addr, "POST", "/mcp", Some(json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }))).await;
    let names: Vec<&str> = list["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    for t in ["get_world_state", "set_parameter", "spawn_entity", "trigger_event"] {
        assert!(names.contains(&t), "{t} missing");
    }
    let call = |id: u64, name: &str, args: Value| json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": { "name": name, "arguments": args } });
    let (_, r) = http(addr, "POST", "/mcp", Some(call(3, "spawn_entity", json!({ "template": "cop", "zone": "square", "count": 2 })))).await;
    assert_eq!(r["result"]["isError"], false, "{r}");
    let (_, r) = http(addr, "POST", "/mcp", Some(call(4, "set_parameter", json!({ "key": "market.elasticity", "value": 0.9 })))).await;
    assert_eq!(r["result"]["isError"], false);
    let (_, r) = http(addr, "POST", "/mcp", Some(call(5, "trigger_event", json!({ "kind": "festival", "message": "Festa in piazza", "newsworthiness": 0.9 })))).await;
    assert_eq!(r["result"]["isError"], false);
    let (_, r) = http(addr, "POST", "/mcp", Some(call(6, "get_world_state", json!({})))).await;
    let text = r["result"]["content"][0]["text"].as_str().unwrap();
    let state: Value = serde_json::from_str(text).unwrap();
    assert_eq!(state["tick"], 5);
    let (_, r) = http(addr, "POST", "/mcp", Some(call(7, "no_such_tool", json!({})))).await;
    assert_eq!(r["result"]["isError"], true);
    // Notifications get no body.
    let (st, _) = http(addr, "POST", "/mcp", Some(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))).await;
    assert_eq!(st, 202);
}

/// The console's map editor: reads the map file, refuses invalid edits, writes tile rows and restarts.
#[tokio::test]
async fn map_editor_saves_and_restarts() {
    // The town pack split in two files, since the editor rewrites the file holding the map.
    let dir = std::env::temp_dir().join(format!("sim_core_mapedit_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let src = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/town/town.ron"));
    let mut rest = sim_core::content::parse_pack(&std::fs::read_to_string(src).unwrap(), src).unwrap();
    let map_pack = sim_core::content::ContentPack { map: rest.map.take(), placements: std::mem::take(&mut rest.placements), ..Default::default() };
    std::fs::write(dir.join("10_town.json"), serde_json::to_string(&rest).unwrap()).unwrap();
    std::fs::write(dir.join("70_map.json"), serde_json::to_string(&map_pack).unwrap()).unwrap();
    let build = {
        let dir = dir.clone();
        move || -> Result<sim_core::sim::SimBuilder, String> {
            let mut b = Simulation::builder(3);
            b.load_pack_dir(&dir).map_err(|e| e.to_string())?;
            Ok(b)
        }
    };
    let sim = Arc::new(Mutex::new(build().unwrap().build().unwrap()));
    let state = AppState { sim: sim.clone(), control: Arc::new(Mutex::new(Control { paused: true, tick_ms: 1000 })), metrics: None, ui_html: None, factory: Some(Arc::new(build)), token: None, assets: None, ui_cache: Default::default() };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(sim_core::server::serve_on(state, listener));

    let (st, doc) = http(addr, "GET", "/api/admin/map", None).await;
    assert_eq!(st, 200, "{doc}");
    assert_eq!(doc["file"], "70_map.json");
    assert_eq!(doc["map"]["layers"][0]["width"], 40);
    let n = doc["placements"].as_array().unwrap().len();

    // A coloured tile type, the map painted with it, a hidden thing; then something invalid.
    let mut edit = json!({ "map": doc["map"], "placements": doc["placements"] });
    edit["map"]["legend"] = json!([{ "ch": ".", "id": "grass", "name": "Grass" }, { "ch": "z", "id": "blue_tiles", "sprite": "tiles", "color": "#3366cc", "walkable": true }]);
    let mut rows = vec![".".repeat(40); 40];
    rows[5] = "z".repeat(40);
    edit["map"]["layers"][0]["tiles"] = json!(rows);
    let mut bad = edit.clone();
    bad["placements"].as_array_mut().unwrap().push(json!({ "Entity": { "template": "cop", "zone": "nowhere" } }));
    let (st, _) = http(addr, "PUT", "/api/admin/map?dry=true", Some(bad)).await;
    assert_eq!(st, 400);
    assert!(!dir.join("maps").exists(), "a dry run writes nothing");

    edit["placements"].as_array_mut().unwrap().push(json!({ "Entity": { "template": "cop", "zone": "jail", "at": [31, 31], "name": "Warden" } }));
    let (st, v) = http(addr, "PUT", "/api/admin/map?restart=true", Some(edit)).await;
    assert_eq!(st, 200, "{v}");
    assert_eq!(std::fs::read_to_string(dir.join("maps/surface.map")).unwrap().lines().nth(5).unwrap(), "z".repeat(40));
    let (_, again) = http(addr, "GET", "/api/admin/map", None).await;
    assert_eq!(again["placements"].as_array().unwrap().len(), n + 1);
    assert_eq!(again["map"]["layers"][0]["tiles"][5], "z".repeat(40));
    let (_, ui) = http(addr, "GET", "/api/ui/map", None).await;
    assert_eq!(ui["legend"]["z"], json!(["blue_tiles", true, "tiles", "#3366cc"]));
    let (_, ents) = http(addr, "GET", "/api/entities", None).await;
    assert!(ents.as_array().unwrap().iter().any(|e| e["name"] == "Warden"), "the new game has the new NPC");

    // Many tiles at once in the running game.
    let (st, v) = http(addr, "POST", "/api/commands?now=true", Some(json!({ "type": "set_tiles", "layer": "surface", "cells": [[0, 0, "z"], [1, 0, "z"], [2, 5, "z"]] }))).await;
    assert_eq!(st, 200, "{v}");
    assert_eq!(sim.lock().unwrap().world.resource::<WorldMap>().tile(&Position::new(0, 1, 0)), Some('z'));
    let _ = std::fs::remove_dir_all(&dir);
}
