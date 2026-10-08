//! CompendiumExporter: builds `compendium.json` for the in-game wiki from the loaded content.

use serde_json::{json, Value};

use crate::content::Content;
use crate::params::{Params, DEFAULTS};

pub fn build(content: &Content, params: &Params) -> Value {
    let named = |id: &str, name: &str, desc: &str| json!({ "id": id, "name": name, "description": desc });
    json!({
        "packs": content.packs,
        "feed": content.press.feed_name,
        "currency": content.bindings.currency_name,
        "stats": content.stats.values().map(|s| json!({ "id": s.id, "name": s.name, "description": s.description, "min": s.min, "max": s.max })).collect::<Vec<_>>(),
        "needs": content.needs.values().map(|n| json!({ "id": n.id, "name": n.name, "stat": n.stat, "per_tick": n.per_tick })).collect::<Vec<_>>(),
        "races": content.races.values().map(|r| json!({
            "id": r.id, "name": r.name, "description": r.description, "stats": r.stat_ranges, "tags": r.tags,
            "abilities": r.abilities, "sexes": r.sexes, "immunities": r.immunities,
            "body": content.body_plans.get(&r.body_plan).map(|b| b.parts.iter().map(|p| p.name.clone()).collect::<Vec<_>>()),
        })).collect::<Vec<_>>(),
        "classes": content.classes.values().map(|c| json!({
            "id": c.id, "name": c.name, "description": c.description, "stats": c.stats, "abilities": c.abilities,
        })).collect::<Vec<_>>(),
        "statuses": content.statuses.values().filter(|s| !s.hidden).map(|s| json!({
            "id": s.id, "name": s.name, "description": s.description, "kind": s.kind, "tags": s.tags,
            "stages": s.stages.iter().map(|st| st.name.clone()).collect::<Vec<_>>(),
            "escalates_to": s.escalates_to.as_ref().and_then(|e| content.statuses.get(e)).map(|e| e.name.clone()),
            "contagious": s.contagion.is_some(),
        })).collect::<Vec<_>>(),
        "items": content.items.values().map(|i| json!({
            "id": i.id, "name": i.name, "description": i.description, "category": i.category, "price": i.base_price,
            "tags": i.tags, "victory_points": i.victory_points, "unique": i.unique,
        })).collect::<Vec<_>>(),
        "abilities": content.abilities.values().map(|a| named(&a.id, &a.name, &a.description)).collect::<Vec<_>>(),
        "jobs": content.jobs.values().map(|j| json!({ "id": j.id, "name": j.name, "description": j.description, "work_type": j.work_type, "crime": j.crime.as_ref().map(|c| &c.id) })).collect::<Vec<_>>(),
        "factions": content.factions.values().map(|f| json!({
            "id": f.id, "name": f.name, "description": f.description, "role": f.role, "neutral": f.neutral,
            "playable": f.playable, "ideology": f.ideology, "ranks": f.ranks.iter().map(|r| r.name.clone()).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "buildings": content.buildings.values().map(|b| json!({
            "id": b.id, "name": b.name, "description": b.description,
            "recipes": b.recipes.iter().map(|r| json!({ "name": r.name, "inputs": r.inputs, "outputs": r.outputs })).collect::<Vec<_>>(),
            "sells": b.sells.keys().collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "characters": content.templates.values().filter(|t| t.unique).map(|t| json!({
            "id": t.id, "name": t.name, "description": t.description, "race": t.race, "classes": t.classes,
            "faction": t.faction, "immortal": t.immortal,
        })).collect::<Vec<_>>(),
        "collections": content.collections.values().map(|c| json!({ "id": c.id, "name": c.name, "description": c.description, "items": c.items, "victory_points": c.victory_points })).collect::<Vec<_>>(),
        "titles": content.titles.values().map(|t| json!({ "id": t.id, "name": t.name, "faction": t.faction, "victory_points": t.victory_points })).collect::<Vec<_>>(),
        "victory": content.victory.iter().map(|v| json!({ "id": v.id, "name": v.name, "description": v.description, "primary": v.primary })).collect::<Vec<_>>(),
        "zones": content.map.as_ref().map(|m| m.zones.iter().map(|z| json!({ "id": z.id, "name": z.name, "layer": z.layer })).collect::<Vec<_>>()),
        "parameters": params.all().iter().map(|(k, v)| json!({ "key": k, "value": v, "description": DEFAULTS.iter().find(|d| d.0 == k).map(|d| d.2) })).collect::<Vec<_>>(),
    })
}

pub fn write(content: &Content, params: &Params, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
    std::fs::write(path, serde_json::to_string_pretty(&build(content, params)).unwrap_or_default())
}
