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
    let state = AppState { sim: sim.clone(), control: Arc::new(Mutex::new(Control { paused: true, tick_ms: 1000 })), metrics: None, ui_html: None, factory: None, token: None, assets: None };
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
