//! Logistics of production chains: haul jobs carry inputs from producers (fields, pens, workshops) to the
//! buildings whose recipes need them, and restock shops. Goods changing owner are paid wholesale at the
//! market price by the receiving faction.

use std::collections::{BTreeMap, BTreeSet};

use bevy_ecs::prelude::*;

use crate::buildings::{Building, Owner, Shop};
use crate::content::Content;
use crate::ids::SimId;
use crate::inventory::Stock;
use crate::jobs::{BoardJob, JobBoard, JobPayload, JobTarget, PersonalQueue, Task};
use crate::map::Position;
use crate::params::Params;
use crate::time::SimClock;

struct Site {
    id: SimId,
    owner: Owner,
    pos: Option<Position>,
    stock: Stock,
    def: String,
    alive: bool,
}

fn produces(content: &Content, def: &str, item: &str) -> bool {
    content.buildings.get(def).is_some_and(|d| d.passive.contains_key(item) || d.recipes.iter().any(|r| r.outputs.contains_key(item)))
}

/// Haul deliveries already planned (on the board, in personal queues or being carried out).
fn pending(world: &mut World) -> BTreeSet<(SimId, String)> {
    let mut out = BTreeSet::new();
    let mut add = |p: &Option<JobPayload>| {
        if let Some(JobPayload::Haul { to, item, .. }) = p {
            out.insert((*to, item.clone()));
        }
    };
    for j in world.resource::<JobBoard>().jobs.values() {
        add(&j.payload);
    }
    let mut q = world.query::<(&Task, &PersonalQueue)>();
    for (t, pq) in q.iter(world) {
        if let Some(j) = &t.job {
            add(&j.payload);
        }
        for j in &pq.0 {
            add(&j.payload);
        }
    }
    out
}

/// Posts haul jobs for missing recipe inputs and empty shop shelves (every `logistics.interval` ticks).
pub fn post_logistics(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let interval = world.resource::<Params>().get("logistics.interval", 4.0).max(1.0) as u64;
    if tick % interval != 0 {
        return;
    }
    let content = world.resource::<Content>().clone();
    let Some(haul_job) = content.jobs.values().find(|j| j.handler == "haul").map(|j| j.id.clone()) else { return };
    let batch = world.resource::<Params>().get("logistics.batch", 5.0) as u32;
    let mut sites: Vec<Site> = Vec::new();
    let mut q = world.query::<(&SimId, &Building, &Stock, Option<&Position>)>();
    for (id, b, s, p) in q.iter(world) {
        sites.push(Site { id: *id, owner: b.owner.clone(), pos: p.copied(), stock: s.clone(), def: b.def.clone(), alive: b.hp > 0.0 });
    }
    sites.sort_by_key(|s| s.id);
    let shops: BTreeMap<SimId, Vec<String>> = {
        let mut q = world.query::<(&SimId, &Shop)>();
        q.iter(world).map(|(id, s)| (*id, s.catalog.keys().cloned().collect())).collect()
    };
    let mut planned = pending(world);
    let mut needs: Vec<(usize, String, u32)> = Vec::new();
    for (i, s) in sites.iter().enumerate() {
        if !s.alive {
            continue;
        }
        if let Some(d) = content.buildings.get(&s.def) {
            for r in &d.recipes {
                for (item, n) in &r.inputs {
                    if s.stock.count(item) < *n {
                        needs.push((i, item.clone(), n * 2 - s.stock.count(item)));
                    }
                }
            }
        }
        for item in shops.get(&s.id).into_iter().flatten() {
            if !produces(&content, &s.def, item) && s.stock.count(item) < 3 {
                needs.push((i, item.clone(), batch.max(3) - s.stock.count(item)));
            }
        }
    }
    for (di, item, qty) in needs {
        let dest = &sites[di];
        if planned.contains(&(dest.id, item.clone())) {
            continue;
        }
        let source = sites
            .iter()
            .filter(|s| s.id != dest.id && s.alive && s.stock.count(&item) > 0 && produces(&content, &s.def, &item))
            .min_by_key(|s| (s.owner != dest.owner, dest.pos.zip(s.pos).map_or(0, |(a, b)| a.cost(&b)), s.id));
        let Some(src) = source else { continue };
        let qty = qty.min(src.stock.count(&item)).min(batch).max(1);
        let faction = match &dest.owner {
            Owner::Faction(f) => Some(f.clone()),
            _ => None,
        };
        let payload = JobPayload::Haul { from: src.id, to: dest.id, item: item.clone(), qty };
        let jid = crate::jobs::post_job(world, &haul_job, faction, JobTarget::Entity(src.id), 0, None);
        if let Some(j) = world.resource_mut::<JobBoard>().jobs.get_mut(&jid) {
            j.payload = Some(payload);
        }
        planned.insert((dest.id, item));
    }
}

fn owner_of(world: &World, id: SimId) -> Owner {
    world
        .resource::<crate::ids::IdIndex>()
        .get(id)
        .and_then(|e| world.get::<Building>(e))
        .map_or(Owner::None, |b| b.owner.clone())
}

/// Pick-up leg: loads the goods (paying the producer if it has another owner) and queues the delivery.
pub fn haul(world: &mut World, ctx: &crate::jobs::JobCtx) -> crate::jobs::JobResult {
    use crate::jobs::JobResult;
    let Some(JobPayload::Haul { from, to, item, qty }) = ctx.active.payload.clone() else { return JobResult::fail("nessun carico") };
    let Some(src) = crate::lifecycle::entity_of(world, from) else { return JobResult::fail("magazzino sparito") };
    let content = world.resource::<Content>().clone();
    let room = world.get::<crate::inventory::Inventory>(ctx.actor).map_or(0, |i| i.room_for(&content, &item));
    let n = qty.min(crate::inventory_ops::count(world, src, &item)).min(room);
    if n == 0 {
        return JobResult::fail("niente da caricare");
    }
    let (src_owner, dst_owner) = (owner_of(world, from), owner_of(world, to));
    if src_owner != dst_owner {
        let price = world.resource::<crate::market::Market>().price(&item).unwrap_or(0.0) * n as f64;
        if let Owner::Faction(f) = &dst_owner {
            if !world.resource_mut::<crate::factions::Factions>().spend(f, price) {
                return JobResult::fail("la fazione non può pagare la merce");
            }
        }
        crate::economy::earn_owner(world, &src_owner, price);
    }
    crate::inventory_ops::take(world, src, &item, n);
    crate::inventory_ops::give(world, ctx.actor, &item, n);
    let Some(deliver) = content.jobs.values().find(|j| j.handler == "deliver").map(|j| j.id.clone()) else {
        return JobResult::ok();
    };
    let tick = world.resource::<SimClock>().tick;
    let job = BoardJob {
        id: 0,
        job: deliver,
        faction: None,
        target: JobTarget::Entity(to),
        priority: 1,
        reserved_by: None,
        created: tick,
        posted_by: None,
        payload: Some(JobPayload::Haul { from, to, item, qty: n }),
    };
    if let Some(mut q) = world.get_mut::<PersonalQueue>(ctx.actor) {
        q.0.push_front(job);
    }
    let iname = content.items.get(&ctx.active.payload.as_ref().map_or(String::new(), |p| match p { JobPayload::Haul { item, .. } => item.clone(), _ => String::new() })).map_or(String::new(), |d| d.name.clone());
    JobResult::ok_msg(format!("{} carica {n}× {iname} da {}", crate::infiltration::apparent_name(world, ctx.actor), crate::effects::name_of(world, src)))
}

/// Delivery leg: unloads the goods into the destination's stock.
pub fn deliver(world: &mut World, ctx: &crate::jobs::JobCtx) -> crate::jobs::JobResult {
    use crate::jobs::JobResult;
    let Some(JobPayload::Haul { to, item, qty, .. }) = ctx.active.payload.clone() else { return JobResult::fail("nessun carico") };
    let Some(dst) = crate::lifecycle::entity_of(world, to) else { return JobResult::fail("destinazione sparita") };
    let n = crate::inventory_ops::transfer(world, ctx.actor, dst, &item, qty);
    if n == 0 {
        return JobResult::fail("carico perso");
    }
    let iname = world.resource::<Content>().items.get(&item).map_or(item.clone(), |d| d.name.clone());
    JobResult::ok_msg(format!("{} consegna {n}× {iname} a {}", crate::infiltration::apparent_name(world, ctx.actor), crate::effects::name_of(world, dst)))
}
