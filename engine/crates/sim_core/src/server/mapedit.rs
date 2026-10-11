//! The console's map editor: reads the pack file holding the map (legend, maps, zones, passages, borders,
//! networks) with its placements, validates an edited copy like a fresh load and writes it back: tile rows
//! in `maps/<map>.map`, the rest in `70_mappa.json` (which replaces a generated `70_mappa.ron`).

use std::path::{Path as FsPath, PathBuf};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use super::{ApiResult, AppState};
use crate::content::{load_pack_dir, parse_pack, Content, ContentPack, MapDef, Placement};

type Err = (StatusCode, String);

fn bad(e: impl std::fmt::Display) -> Err {
    (StatusCode::BAD_REQUEST, e.to_string())
}

/// Folder of the content packs: the one the console's content edits are saved in.
fn data_dir(s: &AppState) -> Result<PathBuf, Err> {
    let sim = s.sim.lock().unwrap();
    sim.world
        .resource::<crate::content::ContentOverrides>()
        .0
        .as_ref()
        .and_then(|p| p.parent().map(FsPath::to_path_buf))
        .ok_or((StatusCode::NOT_IMPLEMENTED, "cartella dei contenuti sconosciuta: l'editor della mappa non può salvare".into()))
}

/// Pack files of the folder, sorted like the loader does.
fn pack_files(dir: &FsPath) -> Result<Vec<PathBuf>, Err> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(bad)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "ron" || e == "json"))
        .collect();
    files.sort();
    Ok(files)
}

fn read_pack(dir: &FsPath, path: &FsPath) -> Result<ContentPack, Err> {
    let text = std::fs::read_to_string(path).map_err(bad)?;
    let mut pack = parse_pack(&text, path).map_err(bad)?;
    if let Some(map) = pack.map.as_mut() {
        for l in map.layers.iter_mut() {
            if let Some(f) = &l.tiles_file {
                let rows = std::fs::read_to_string(dir.join(f)).map_err(|e| bad(format!("{f}: {e}")))?;
                l.tiles = rows.lines().map(|r| r.trim_end().to_string()).filter(|r| !r.is_empty()).collect();
            }
        }
    }
    Ok(pack)
}

/// The file with the map; it must hold nothing but the map and placements, since the editor rewrites it.
fn map_file(dir: &FsPath) -> Result<(PathBuf, ContentPack), Err> {
    let mut found: Option<(PathBuf, ContentPack)> = None;
    for f in pack_files(dir)? {
        let pack = read_pack(dir, &f)?;
        if pack.map.is_some() {
            if let Some((other, _)) = &found {
                return Err(bad(format!("la mappa è divisa fra {} e {}: l'editor ne gestisce una sola", name(other), name(&f))));
            }
            found = Some((f, pack));
        }
    }
    let (f, pack) = found.ok_or_else(|| bad("nessun file dei contenuti contiene la mappa"))?;
    let mut rest = serde_json::to_value(&pack).map_err(bad)?;
    rest["map"] = Value::Null;
    rest["placements"] = json!([]);
    if rest != serde_json::to_value(ContentPack::default()).map_err(bad)? {
        return Err(bad(format!("{} contiene anche altri contenuti oltre a mappa e piazzamenti: spostali in un altro file", name(&f))));
    }
    Ok((f, pack))
}

fn name(p: &FsPath) -> String {
    p.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

/// `GET /api/admin/map`: the editable map with its placements, and the placements of other files (read only).
pub async fn get_map(State(s): State<AppState>) -> ApiResult {
    let dir = data_dir(&s)?;
    let (file, pack) = map_file(&dir)?;
    let mut others = Vec::new();
    for f in pack_files(&dir)? {
        if f == file {
            continue;
        }
        let pack = read_pack(&dir, &f)?;
        for p in pack.placements {
            others.push(json!({ "file": name(&f), "placement": p }));
        }
    }
    let mut map = pack.map.unwrap_or_default();
    for l in map.layers.iter_mut() {
        l.width = l.tiles.iter().map(|r| r.chars().count()).max().unwrap_or(l.width as usize) as i32;
        l.height = if l.tiles.is_empty() { l.height } else { l.tiles.len() as i32 };
    }
    Ok(Json(json!({ "file": name(&file), "map": map, "placements": pack.placements, "others": others })))
}

#[derive(Deserialize)]
pub struct Doc {
    map: MapDef,
    #[serde(default)]
    placements: Vec<Placement>,
}

#[derive(Deserialize, Default)]
pub struct SaveQ {
    /// Only check, write nothing.
    #[serde(default)]
    dry: bool,
    /// After saving, start a new game with the saved map.
    #[serde(default)]
    restart: bool,
}

/// `PUT /api/admin/map`: validates the whole content with this map and, unless `dry`, writes it.
pub async fn put_map(State(s): State<AppState>, Query(q): Query<SaveQ>, Json(mut doc): Json<Doc>) -> ApiResult {
    let dir = data_dir(&s)?;
    let (file, old) = map_file(&dir)?;
    // Same rules as the loader: rows padded to the map width, sizes taken from the rows.
    let mut warnings = Vec::new();
    for l in doc.map.layers.iter_mut() {
        if l.id.is_empty() || !l.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(bad(format!("mappa '{}': l'id può avere solo lettere, cifre e _", l.id)));
        }
        if l.tiles.is_empty() {
            if l.width <= 0 || l.height <= 0 {
                return Err(bad(format!("mappa {}: senza caselle servono larghezza e altezza", l.id)));
            }
            continue;
        }
        let w = l.tiles.iter().map(|r| r.chars().count()).max().unwrap_or(0);
        for r in l.tiles.iter_mut() {
            let n = r.chars().count();
            if n < w {
                warnings.push(format!("mappa {}: una riga corta è stata allungata", l.id));
                r.extend(std::iter::repeat_n('.', w - n));
            }
        }
        l.width = w as i32;
        l.height = l.tiles.len() as i32;
    }
    let mut packs = load_pack_dir(&dir).map_err(bad)?;
    let files = pack_files(&dir)?;
    let idx = files.iter().position(|f| *f == file).ok_or_else(|| bad("file della mappa sparito"))?;
    packs[idx] = ContentPack { map: Some(doc.map.clone()), placements: doc.placements.clone(), ..Default::default() };
    if let Err(e) = Content::from_packs(packs) {
        return Err(bad(e));
    }
    if q.dry {
        return Ok(Json(json!({ "ok": "la mappa è valida", "warnings": warnings })));
    }

    // Tile rows in their own files; stale ones of maps that were removed or renamed go away.
    let maps_dir = dir.join("maps");
    std::fs::create_dir_all(&maps_dir).map_err(bad)?;
    let mut written = Vec::new();
    for l in doc.map.layers.iter_mut() {
        if l.tiles.is_empty() {
            l.tiles_file = None;
            continue;
        }
        let rel = format!("maps/{}.map", l.id);
        std::fs::write(dir.join(&rel), l.tiles.join("\n") + "\n").map_err(bad)?;
        written.push(rel.clone());
        l.tiles_file = Some(rel);
        l.tiles.clear();
    }
    for l in old.map.iter().flat_map(|m| &m.layers) {
        if let Some(f) = &l.tiles_file
            && !written.contains(f)
        {
            let _ = std::fs::remove_file(dir.join(f));
        }
    }
    let out = file.with_extension("json");
    std::fs::write(&out, format_doc(&doc.map, &doc.placements)?).map_err(bad)?;
    if out != file {
        std::fs::remove_file(&file).map_err(bad)?;
    }
    let mut msg = format!("mappa salvata in {} e {} file di caselle", name(&out), written.len());
    if q.restart {
        let factory = s.factory.clone().ok_or((StatusCode::NOT_IMPLEMENTED, "la partita non si può ricominciare da qui".to_string()))?;
        let new = factory().map_err(bad)?.build().map_err(bad)?;
        *s.sim.lock().unwrap() = new;
        s.ui_cache.lock().unwrap().clear();
        msg += "; nuova partita iniziata";
    }
    Ok(Json(json!({ "ok": msg, "warnings": warnings })))
}

/// The pack as JSON with one definition per line and default values left out, so diffs stay readable.
fn format_doc(map: &MapDef, placements: &[Placement]) -> Result<String, Err> {
    let mut map = serde_json::to_value(map).map_err(bad)?;
    let mut placements = serde_json::to_value(placements).map_err(bad)?;
    strip_defaults(&mut map);
    strip_defaults(&mut placements);
    let line = |v: &Value| serde_json::to_string(v).unwrap_or_default();
    let list = |items: &[Value], indent: &str| -> String {
        if items.is_empty() {
            return "[]".into();
        }
        let body: Vec<String> = items.iter().map(|v| format!("{indent}  {}", line(v))).collect();
        format!("[\n{}\n{indent}]", body.join(",\n"))
    };
    let mut parts = Vec::new();
    if let Value::Object(m) = &map {
        for (k, v) in m {
            let text = match v {
                Value::Array(a) => list(a, "    "),
                other => line(other),
            };
            parts.push(format!("    {}: {text}", line(&json!(k))));
        }
    }
    let placements = list(placements.as_array().map_or(&[][..], |a| a.as_slice()), "  ");
    Ok(format!(
        "{{\n  \"map\": {{\n{}\n  }},\n  \"placements\": {placements}\n}}\n",
        parts.join(",\n")
    ))
}

/// Drops nulls, empty lists and objects, and fields equal to their default (everything that serde would
/// fill in anyway).
fn strip_defaults(v: &mut Value) {
    match v {
        Value::Array(a) => a.iter_mut().for_each(strip_defaults),
        Value::Object(m) => {
            m.values_mut().for_each(strip_defaults);
            let tiled = m.get("tiles_file").is_some_and(|f| !f.is_null());
            m.retain(|k, v| !is_default(k, v, tiled));
        }
        _ => {}
    }
}

fn is_default(k: &str, v: &Value, tiled: bool) -> bool {
    match v {
        Value::Null => true,
        Value::Array(a) => a.is_empty() || (k == "footprint" && a.iter().all(|x| x.as_i64() == Some(0))),
        Value::Object(o) => o.is_empty(),
        Value::Bool(b) => !b && k != "walkable" || *b && k == "walkable",
        Value::Number(n) => match k {
            "depth" | "offset" => n.as_i64() == Some(0),
            "count" => n.as_i64() == Some(1),
            "width" | "height" => tiled,
            _ => false,
        },
        _ => false,
    }
}
