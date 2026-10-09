//! Generic job handlers shipped with the engine. Content picks one with `handler: "<name>"`; plugins can
//! add more with `SimBuilder::register_job_handler`.

use std::sync::Arc;

use bevy_ecs::prelude::*;

use crate::buildings::{Building, Owner};
use crate::content::Content;
use crate::extensions::{Extensions, JobHandlerFn};
use crate::factions::{FactionMember, Factions};
use crate::inventory::Inventory;
use crate::jobs::{JobCtx, JobResult};
use crate::map::Position;
use crate::stats::{Dead, Stats, Wallet};
use crate::status::StatusEffects;

fn h(f: fn(&mut World, &JobCtx) -> JobResult) -> JobHandlerFn {
    Arc::new(f)
}

pub fn register_core(ext: &mut Extensions) {
    let list: &[(&str, fn(&mut World, &JobCtx) -> JobResult)] = &[
        ("effects", |_, _| JobResult::ok()),
        ("consume", consume),
        ("buy", buy),
        ("steal", steal),
        ("attack", attack),
        ("throw", throw),
        ("assassinate", assassinate),
        ("sabotage", sabotage),
        ("build", build),
        ("demolish", demolish),
        ("produce", produce),
        ("process", process),
        ("search", search),
        ("arrest", arrest),
        ("investigate", investigate),
        ("publish", publish),
        ("analyze", analyze),
        ("synthesize", synthesize),
        ("treat", treat),
        ("inoculate", inoculate),
        ("transmute", transmute),
        ("infiltrate", infiltrate),
        ("identity_theft", identity_theft),
        ("poison", poison),
        ("clean", clean),
        ("loot", loot),
        ("haul", crate::logistics::haul),
        ("dig", dig),
        ("store", store),
        ("deliver", crate::logistics::deliver),
    ];
    for (name, f) in list {
        ext.job_handlers.insert((*name).to_string(), h(*f));
    }
}

fn target_alive(world: &World, ctx: &JobCtx) -> Option<Entity> {
    ctx.target.filter(|t| world.get::<Dead>(*t).is_none())
}

/// Uses the first carried item with tag `tag` (param) — eating, drinking, taking drugs.
fn consume(world: &mut World, ctx: &JobCtx) -> JobResult {
    let content = world.resource::<Content>().clone();
    let tag = ctx.param_str("tag").unwrap_or("food");
    let Some(item) = world.get::<Inventory>(ctx.actor).and_then(|i| i.first_with_tag(&content, tag)) else {
        return JobResult::fail("niente da consumare");
    };
    use_item(world, ctx.actor, &item)
}

pub fn use_item(world: &mut World, e: Entity, item: &str) -> JobResult {
    let Some(def) = world.resource::<Content>().items.get(item).cloned() else { return JobResult::fail("oggetto sconosciuto") };
    if crate::inventory_ops::count(world, e, item) == 0 {
        return JobResult::fail("oggetto non posseduto");
    }
    if !def.reusable {
        crate::inventory_ops::take(world, e, item, 1);
    }
    crate::effects::apply_effects(world, &crate::effects::EffectCtx::new(Some(e), None, format!("item:{item}")), &def.on_use);
    JobResult::ok()
}

/// Buys at the target shop the cheapest affordable item with tag `tag` (or item `item`).
fn buy(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(shop) = ctx.target else { return JobResult::fail("nessun negozio") };
    let content = world.resource::<Content>().clone();
    let Some(s) = world.get::<crate::buildings::Shop>(shop).cloned() else { return JobResult::fail("non è un negozio") };
    let stock = world.get::<crate::inventory::Stock>(shop).cloned().unwrap_or_default();
    let money = world.get::<Wallet>(ctx.actor).map_or(0.0, |w| w.0);
    let wanted_item = ctx.param_str("item");
    let tag = ctx.param_str("tag");
    let mut options: Vec<(f64, String)> = s
        .catalog
        .keys()
        .filter(|i| stock.count(i) > 0)
        .filter(|i| wanted_item.is_none_or(|w| w == i.as_str()))
        .filter(|i| tag.is_none_or(|t| content.items.get(*i).is_some_and(|d| d.tags.iter().any(|x| x == t))))
        .filter_map(|i| crate::economy::shop_price(world, &s, &stock, i).map(|p| (p, i.clone())))
        .filter(|(p, _)| *p <= money)
        .collect();
    options.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let Some((_, item)) = options.first().cloned() else { return JobResult::fail("niente di accessibile") };
    match crate::economy::buy(world, ctx.actor, shop, &item) {
        Ok(_) => {
            if ctx.def.params.get("use").and_then(|v| v.as_bool()).unwrap_or(false) {
                use_item(world, ctx.actor, &item);
            }
            JobResult::ok()
        }
        Err(e) => JobResult::fail(e),
    }
}

/// Takes an item with tag `tag` (param) from a pawn or a building; otherwise money (param `amount`)
/// or, failing that, a random item.
fn steal(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = target_alive(world, ctx) else { return JobResult::fail("nessuna vittima") };
    if let Some(tag) = ctx.param_str("tag") {
        let content = world.resource::<Content>().clone();
        let items: Vec<String> = match world.get::<crate::inventory::Stock>(t) {
            Some(s) => s.0.keys().cloned().collect(),
            None => world.get::<Inventory>(t).map(|i| i.items().map(|(k, _)| k.clone()).collect()).unwrap_or_default(),
        };
        let Some(item) = items.into_iter().find(|i| content.items.get(i).is_some_and(|d| d.tags.iter().any(|x| x == tag))) else {
            return JobResult::fail("l'oggetto non è più qui");
        };
        if crate::inventory_ops::transfer(world, t, ctx.actor, &item, 1) == 0 {
            return JobResult::fail("non ha spazio per portarlo via");
        }
        let (thief, victim) = (crate::infiltration::apparent_name(world, ctx.actor), crate::effects::name_of(world, t));
        return JobResult::ok_msg(format!("{thief} sottrae {} a {victim}", content.items[&item].name));
    }
    let amount = ctx.param_f("amount", 20.0);
    let taken = world.get::<Wallet>(t).map_or(0.0, |w| w.0.min(amount));
    let (thief, victim) = (crate::infiltration::apparent_name(world, ctx.actor), crate::infiltration::apparent_name(world, t));
    if taken > 0.0 {
        world.get_mut::<Wallet>(t).unwrap().0 -= taken;
        world.get_mut::<Wallet>(ctx.actor).unwrap().0 += taken;
        return JobResult::ok_msg(format!("{thief} ruba {taken:.0} a {victim}"));
    }
    let items: Vec<String> = world.get::<Inventory>(t).map(|i| i.items().map(|(k, _)| k.clone()).collect()).unwrap_or_default();
    let Some(item) = world.resource_mut::<crate::rng::SimRng>().pick(&items).cloned() else {
        return JobResult::fail("vittima al verde");
    };
    let moved = crate::inventory_ops::transfer(world, t, ctx.actor, &item, 1);
    if moved == 0 {
        return JobResult::fail("inventario pieno");
    }
    JobResult::ok_msg(format!("{thief} ruba {item} a {victim}"))
}

fn strength(world: &World, e: Entity) -> f32 {
    let s = &world.resource::<Content>().bindings.strength;
    world.get::<Stats>(e).map_or(10.0, |x| x.get(s)).max(1.0)
}

fn attack(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = target_alive(world, ctx) else { return JobResult::fail("nessun bersaglio") };
    let mut dmg = strength(world, ctx.actor) / 10.0 * ctx.param_f("damage", 5.0) as f32;
    // With a weapon in hand the blow hurts more, and the weapon wears.
    let weapon = crate::equipment::best_weapon(world, ctx.actor);
    if let Some((w, extra)) = &weapon {
        dmg += extra;
        crate::equipment::wear(world, ctx.actor, w, 1.0);
    }
    if world.get::<Building>(t).is_some() {
        crate::buildings::damage_building(world, t, dmg, Some(ctx.actor));
    } else {
        crate::anatomy::damage(world, t, dmg, None, Some(ctx.actor));
    }
    if let Some(d) = weapon.and_then(|(w, _)| world.resource::<Content>().items.get(&w).cloned()) {
        crate::effects::apply_effects(world, &crate::effects::EffectCtx::new(Some(t), Some(ctx.actor), format!("item:{}", d.id)), &d.on_hit);
    }
    JobResult::ok()
}

/// Throws the most harmful carried item of a kind (param `type`, default "lanciabile") at the target: it
/// is spent unless reusable (then it only wears).
fn throw(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = target_alive(world, ctx) else { return JobResult::fail("nessun bersaglio") };
    let kind = ctx.param_str("type").unwrap_or("lanciabile").to_string();
    let Some(item) = crate::equipment::best_of_type(world, ctx.actor, &kind) else { return JobResult::fail("niente da lanciare") };
    let Some(d) = world.resource::<Content>().items.get(&item).cloned() else { return JobResult::fail("oggetto sconosciuto") };
    if let (Some(a), Some(b)) = (world.get::<Position>(ctx.actor).copied(), world.get::<Position>(t).copied())
        && d.range > 0
        && !a.within(&b, d.range)
    {
        return JobResult::fail("troppo lontano");
    }
    if d.reusable {
        crate::equipment::wear(world, ctx.actor, &item, 1.0);
    } else {
        crate::inventory_ops::take(world, ctx.actor, &item, 1);
    }
    let dmg = d.damage.max(1.0);
    if world.get::<Building>(t).is_some() {
        crate::buildings::damage_building(world, t, dmg, Some(ctx.actor));
    } else {
        crate::anatomy::damage(world, t, dmg, None, Some(ctx.actor));
    }
    crate::effects::apply_effects(world, &crate::effects::EffectCtx::new(Some(t), Some(ctx.actor), format!("item:{item}")), &d.on_hit);
    JobResult::ok()
}

/// Silent kill: if the victim does not notice the assassin, a vital part takes massive damage.
fn assassinate(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = target_alive(world, ctx) else { return JobResult::fail("nessun bersaglio") };
    let seen = crate::infiltration::notices(world, t, ctx.actor);
    if seen {
        return attack(world, ctx);
    }
    let vital = world.get::<crate::anatomy::Body>(t).and_then(|b| b.parts.iter().find(|p| p.vital && !p.missing).map(|p| p.id.clone()));
    let dmg = ctx.param_f("damage", 100.0) as f32;
    crate::anatomy::damage(world, t, dmg, vital.as_deref(), Some(ctx.actor));
    JobResult::ok()
}

fn sabotage(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = ctx.target.filter(|t| world.get::<Building>(*t).is_some()) else { return JobResult::fail("nessun edificio") };
    let dmg = ctx.param_f("damage", 30.0) as f32;
    crate::buildings::damage_building(world, t, dmg, Some(ctx.actor));
    JobResult::ok()
}

/// Builds `building` (param) at the target cell for the worker's faction, or repairs a target building.
fn build(world: &mut World, ctx: &JobCtx) -> JobResult {
    if let Some(t) = ctx.target.filter(|t| world.get::<Building>(*t).is_some()) {
        crate::buildings::repair_building(world, t, ctx.param_f("repair", 25.0) as f32);
        return JobResult::ok();
    }
    let Some(def) = ctx.param_str("building") else { return JobResult::fail("nessun progetto") };
    let Some(pos) = ctx.target_pos.or_else(|| world.get::<Position>(ctx.actor).copied()) else { return JobResult::fail("dove?") };
    let cost = world.resource::<Content>().buildings.get(def).map(|b| b.cost.clone()).unwrap_or_default();
    for (i, n) in &cost {
        if crate::inventory_ops::count(world, ctx.actor, i) < *n {
            return JobResult::fail("mancano i materiali");
        }
    }
    for (i, n) in &cost {
        crate::inventory_ops::take(world, ctx.actor, i, *n);
    }
    let owner = world.get::<FactionMember>(ctx.actor).map_or(Owner::None, |m| Owner::Faction(m.faction.clone()));
    crate::buildings::spawn_building(world, def, pos, None, owner);
    JobResult::ok_msg(format!("Costruito: {def}"))
}

fn demolish(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = ctx.target.filter(|t| world.get::<Building>(*t).is_some()) else { return JobResult::fail("nessun edificio") };
    let hp = world.get::<Building>(t).unwrap().hp;
    crate::buildings::damage_building(world, t, hp, Some(ctx.actor));
    JobResult::ok()
}

/// Farming, breeding, mining: `item` × `qty` into the target building's stock or the worker's inventory.
fn produce(world: &mut World, ctx: &JobCtx) -> JobResult {
    let dest = ctx.target.filter(|t| world.get::<crate::inventory::Stock>(*t).is_some()).unwrap_or(ctx.actor);
    let outputs: Vec<(String, u32)> = match ctx.param_str("item") {
        Some(item) => vec![(item.to_string(), ctx.param_f("qty", 1.0) as u32)],
        // Without an explicit item, work the target building (a field, a pen): yields its passive output.
        None => world
            .get::<Building>(dest)
            .and_then(|b| world.resource::<Content>().buildings.get(&b.def))
            .map(|d| d.productions.iter().filter(|p| p.every > 0 && p.inputs.is_empty()).flat_map(|p| p.outputs.iter().map(|(k, v)| (k.clone(), *v))).collect())
            .unwrap_or_default(),
    };
    if outputs.is_empty() {
        return JobResult::fail("cosa produrre?");
    }
    let mut n = 0;
    for (item, qty) in outputs {
        n += crate::inventory_ops::give(world, dest, &item, qty);
    }
    if n == 0 { JobResult::fail("nessuno spazio") } else { JobResult::ok() }
}

fn process(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(crate::jobs::JobPayload::Recipe { building: bid, index: ri }) = ctx.active.payload else {
        return JobResult::fail("nessuna ricetta");
    };
    let Some(b) = crate::lifecycle::entity_of(world, bid) else { return JobResult::fail("edificio sparito") };
    match crate::buildings::process_recipe(world, b, ri) {
        Ok(msg) => JobResult::ok_msg(msg),
        Err(e) => JobResult::fail(e),
    }
}

fn search(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = target_alive(world, ctx) else { return JobResult::fail("nessun sospetto") };
    crate::crime::search(world, ctx.actor, t);
    JobResult::ok()
}

fn arrest(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = target_alive(world, ctx) else { return JobResult::fail("nessun sospetto") };
    if crate::crime::arrest(world, ctx.actor, t) { JobResult::ok() } else { JobResult::fail("nessun motivo per arrestare") }
}

/// Journalist inquiry: lowers the cover of a disguised target and records a scoop about them.
fn investigate(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = target_alive(world, ctx) else { return JobResult::fail("nessun soggetto") };
    let loss = world.resource::<crate::params::Params>().f("infiltration.investigation_cover_loss");
    crate::infiltration::mod_cover(world, t, -loss, "inchiesta giornalistica");
    JobResult::ok()
}

fn publish(world: &mut World, ctx: &JobCtx) -> JobResult {
    match crate::press::publish_best_scoop(world, ctx.actor) {
        Some(h) => JobResult::ok_msg(format!("Pubblicato: «{h}»")),
        None => JobResult::fail("nessuno scoop"),
    }
}

/// Clinical exam: the worker's faction learns the hidden statuses of the target.
fn analyze(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = target_alive(world, ctx) else { return JobResult::fail("nessun paziente") };
    let Some(f) = world.get::<FactionMember>(ctx.actor).map(|m| m.faction.clone()) else { return JobResult::fail("senza fazione") };
    let found: Vec<String> = world.get::<StatusEffects>(t).map(|s| s.active.keys().cloned().collect()).unwrap_or_default();
    let content = world.resource::<Content>().clone();
    let diseases: Vec<String> = found.into_iter().filter(|s| content.statuses.get(s).is_some_and(|d| d.contagion.is_some() || d.tags.iter().any(|t| t == "malattia"))).collect();
    if let Some(s) = world.resource_mut::<Factions>().states.get_mut(&f) {
        s.known_pathogens.extend(diseases.iter().cloned());
    }
    let names: Vec<String> = diseases.iter().map(|d| content.statuses[d].name.clone()).collect();
    JobResult::ok_msg(format!("Esame clinico: {}", if names.is_empty() { "nulla".into() } else { names.join(", ") }))
}

/// Produces a cure (`item`) for `pathogen` if the faction has analyzed it; consumes `inputs` if given.
fn synthesize(world: &mut World, ctx: &JobCtx) -> JobResult {
    let (Some(item), Some(pathogen)) = (ctx.param_str("item"), ctx.param_str("pathogen")) else { return JobResult::fail("ricetta incompleta") };
    let Some(f) = world.get::<FactionMember>(ctx.actor).map(|m| m.faction.clone()) else { return JobResult::fail("senza fazione") };
    if !world.resource::<Factions>().states.get(&f).is_some_and(|s| s.known_pathogens.contains(pathogen)) {
        return JobResult::fail("patogeno sconosciuto: serve un'analisi");
    }
    let qty = ctx.param_f("qty", 1.0) as u32;
    let dest = ctx.target.filter(|t| world.get::<crate::inventory::Stock>(*t).is_some()).unwrap_or(ctx.actor);
    crate::inventory_ops::give(world, dest, item, qty);
    JobResult::ok_msg(format!("Sintetizzato {item}"))
}

/// Uses a medicine item (param `item`) from the worker's inventory on the target.
fn treat(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = target_alive(world, ctx) else { return JobResult::fail("nessun paziente") };
    let Some(item) = ctx.param_str("item") else { return JobResult::fail("nessuna cura") };
    if crate::inventory_ops::take(world, ctx.actor, item, 1) == 0 {
        return JobResult::fail("cura esaurita");
    }
    let def = world.resource::<Content>().items.get(item).cloned();
    if let Some(d) = def {
        crate::effects::apply_effects(world, &crate::effects::EffectCtx::new(Some(t), Some(ctx.actor), format!("treat:{item}")), &d.on_use);
    }
    JobResult::ok()
}

/// Mass inoculation: applies a vaccine item to every pawn of the worker's faction within `radius`.
fn inoculate(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(item) = ctx.param_str("item") else { return JobResult::fail("nessun vaccino") };
    let radius = ctx.param_f("radius", 5.0) as i32;
    let Some(def) = world.resource::<Content>().items.get(item).cloned() else { return JobResult::fail("vaccino sconosciuto") };
    let Some(pos) = world.get::<Position>(ctx.actor).copied() else { return JobResult::fail("dove?") };
    let f = world.get::<FactionMember>(ctx.actor).map(|m| m.faction.clone());
    let mut n = 0;
    for e in crate::sorted_entities::<crate::stats::Pawn>(world) {
        if world.get::<Dead>(e).is_some() || !world.get::<Position>(e).is_some_and(|p| p.within(&pos, radius)) {
            continue;
        }
        if f.is_some() && world.get::<FactionMember>(e).map(|m| m.faction.clone()) != f {
            continue;
        }
        if crate::inventory_ops::take(world, ctx.actor, item, 1) == 0 {
            break;
        }
        crate::effects::apply_effects(world, &crate::effects::EffectCtx::new(Some(e), Some(ctx.actor), "inoculate"), &def.on_use);
        n += 1;
    }
    if n == 0 { JobResult::fail("nessuno inoculato") } else { JobResult::ok_msg(format!("Inoculazione di massa: {n} vaccinati")) }
}

fn transmute(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = target_alive(world, ctx) else { return JobResult::fail("nessun soggetto") };
    let Some(race) = ctx.param_str("race") else { return JobResult::fail("in cosa?") };
    crate::status::transmute(world, t, race);
    JobResult::ok()
}

/// Joins the target's faction under cover.
fn infiltrate(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = target_alive(world, ctx) else { return JobResult::fail("nessun contatto") };
    let Some(f) = crate::infiltration::apparent_faction(world, t) else { return JobResult::fail("il contatto non ha fazione") };
    let cur = world.get::<crate::infiltration::Disguise>(ctx.actor).cloned().unwrap_or_default();
    let d = crate::content::Disguise { race: cur.race, faction: Some(f), name: cur.name };
    crate::infiltration::shapeshift(world, ctx.actor, d, None);
    JobResult::ok()
}

fn identity_theft(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = target_alive(world, ctx) else { return JobResult::fail("nessuna vittima") };
    match crate::infiltration::steal_identity(world, ctx.actor, t) {
        Ok(m) => JobResult::ok_msg(m),
        Err(e) => JobResult::fail(e),
    }
}

/// Poisons a building's stock or network with `status` (param).
fn poison(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = ctx.target else { return JobResult::fail("nessun bersaglio") };
    let Some(status) = ctx.param_str("status") else { return JobResult::fail("quale veleno?") };
    let load = ctx.param_f("load", 1.0) as f32;
    crate::hygiene::contaminate(world, t, status, load);
    JobResult::ok()
}

fn clean(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(p) = ctx.target_pos.or_else(|| world.get::<Position>(ctx.actor).copied()) else { return JobResult::fail("dove?") };
    let radius = ctx.param_f("radius", 2.0) as i32;
    crate::hygiene::clean(world, p, radius, ctx.param_f("amount", 1.0) as f32);
    JobResult::ok()
}

/// Digs the target cell (rock → floor), giving the worker what the rock yields (stone, ore, gems).
fn dig(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(p) = ctx.target_pos else { return JobResult::fail("dove scavare?") };
    let map = world.resource::<crate::map::WorldMap>().clone();
    let Some(ch) = map.tile(&p) else { return JobResult::fail("fuori mappa") };
    let Some((to, yields)) = map.diggable.get(&ch).cloned() else { return JobResult::fail("qui non si scava") };
    crate::map::change_tile(world, p, to);
    if let Some((item, n)) = yields {
        crate::inventory_ops::give(world, ctx.actor, &item, n);
    }
    let what = map.legend.get(&ch).map_or("roccia".to_string(), |(id, _)| id.clone());
    let tick = world.resource::<crate::time::SimClock>().tick;
    let (who, id) = (crate::infiltration::apparent_name(world, ctx.actor), world.get::<crate::ids::SimId>(ctx.actor).copied());
    world.resource_mut::<crate::events::EventLog>().push(
        tick,
        crate::events::EventBuilder::new("dug", format!("{who} scava {what}"))
            .actor(id)
            .pos(Some(p))
            .tags(["dig"]),
    );
    JobResult::ok()
}

/// Puts every carried item with tag `tag` (param) into the target building (a stockpile).
fn store(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = ctx.target.filter(|t| world.get::<crate::inventory::Stock>(*t).is_some()) else { return JobResult::fail("nessun deposito") };
    let content = world.resource::<Content>().clone();
    let tag = ctx.param_str("tag").unwrap_or("materiale");
    let items: Vec<String> = world
        .get::<Inventory>(ctx.actor)
        .map(|i| i.items().filter(|(k, _)| content.items.get(*k).is_some_and(|d| d.tags.iter().any(|x| x == tag))).map(|(k, _)| k.clone()).collect())
        .unwrap_or_default();
    let mut n = 0;
    for i in items {
        n += crate::inventory_ops::transfer(world, ctx.actor, t, &i, 999);
    }
    if n == 0 { JobResult::fail("niente da depositare") } else { JobResult::ok() }
}

/// Takes items from a corpse or a building's stock.
fn loot(world: &mut World, ctx: &JobCtx) -> JobResult {
    let Some(t) = ctx.target else { return JobResult::fail("niente da prendere") };
    let items: Vec<String> = if let Some(s) = world.get::<crate::inventory::Stock>(t) {
        s.0.keys().cloned().collect()
    } else {
        world.get::<Inventory>(t).map(|i| i.items().map(|(k, _)| k.clone()).collect()).unwrap_or_default()
    };
    let mut n = 0;
    for i in items {
        n += crate::inventory_ops::transfer(world, t, ctx.actor, &i, 99);
    }
    if n == 0 { JobResult::fail("niente preso") } else { JobResult::ok() }
}
