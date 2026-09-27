//! GuildTreasurySystem & PayrollEngine, plus the income path of shops and workers.

use bevy_ecs::prelude::*;

use crate::buildings::{Building, Owner, Shop};
use crate::content::Content;
use crate::events::{kind, EventBuilder, EventLog};
use crate::factions::{Dissent, FactionMember, Factions};
use crate::ids::{IdIndex, SimId};
use crate::inventory::Stock;
use crate::market::Market;
use crate::params::Params;
use crate::stats::{Dead, Virtual, Wallet};
use crate::time::SimClock;

/// Money earned by an entity: the guild share goes to its faction's treasury, the rest to its wallet.
pub fn earn(world: &mut World, e: Entity, amount: f64) {
    let share = world.resource::<Params>().get("economy.guild_income_share", 1.0).clamp(0.0, 1.0);
    let faction = world.get::<FactionMember>(e).map(|m| m.faction.clone());
    let to_guild = if faction.is_some() { amount * share } else { 0.0 };
    if let Some(f) = faction {
        world.resource_mut::<Factions>().add_treasury(&f, to_guild);
    }
    if let Some(mut w) = world.get_mut::<Wallet>(e) {
        w.0 += amount - to_guild;
    }
}

/// Income of a building owner.
pub fn earn_owner(world: &mut World, owner: &Owner, amount: f64) {
    match owner {
        Owner::Faction(f) => world.resource_mut::<Factions>().add_treasury(f, amount),
        Owner::Entity(id) => {
            if let Some(e) = world.resource::<IdIndex>().get(*id) {
                earn(world, e, amount);
            }
        }
        Owner::None => {}
    }
}

/// Price of an item in a shop (fixed price or market price × markup).
pub fn shop_price(world: &World, shop: &Shop, item: &str) -> Option<f64> {
    let fixed = shop.catalog.get(item)?;
    fixed.or_else(|| world.resource::<Market>().price(item).map(|p| p * shop.markup as f64))
}

/// Buys one unit of `item` from `shop_e` for `buyer`. Returns the price paid.
pub fn buy(world: &mut World, buyer: Entity, shop_e: Entity, item: &str) -> Result<f64, String> {
    let shop = world.get::<Shop>(shop_e).cloned().ok_or("non è un negozio")?;
    if world.get::<Stock>(shop_e).is_none_or(|s| s.count(item) == 0) {
        return Err("esaurito".into());
    }
    let price = shop_price(world, &shop, item).ok_or("non in vendita")?;
    let money = world.get::<Wallet>(buyer).map_or(0.0, |w| w.0);
    if money + 1e-9 < price {
        return Err("soldi insufficienti".into());
    }
    let content = world.resource::<Content>().clone();
    let fits = world.get::<crate::inventory::Inventory>(buyer).map_or(0, |i| i.room_for(&content, item));
    if fits == 0 {
        return Err("inventario pieno".into());
    }
    world.get_mut::<Stock>(shop_e).unwrap().remove(item, 1);
    world.get_mut::<Wallet>(buyer).unwrap().0 -= price;
    crate::inventory_ops::give(world, buyer, item, 1);
    world.resource_mut::<Market>().record_purchase(item, 1);
    let owner = world.get::<Building>(shop_e).map(|b| b.owner.clone()).unwrap_or(Owner::None);
    earn_owner(world, &owner, price);
    // Food poisoning / contaminated stock.
    if let Some(c) = world.get::<crate::hygiene::Contaminated>(shop_e).cloned() {
        if content.items.get(item).is_some_and(|d| d.tags.iter().any(|t| t == "food" || t == "drink" || t == "water")) {
            crate::status::apply_status(world, buyer, &c.status, 1.0, None);
        }
    }
    let tick = world.resource::<SimClock>().tick;
    let (bid, sid) = (world.get::<SimId>(buyer).copied(), world.get::<SimId>(shop_e).copied());
    let name = crate::infiltration::apparent_name(world, buyer);
    let iname = content.items.get(item).map_or(item.to_string(), |d| d.name.clone());
    world.resource_mut::<EventLog>().push(
        tick,
        EventBuilder::new(kind::PURCHASE, format!("{name} compra {iname} a {price:.1}"))
            .actor(bid)
            .target(sid)
            .tags(["market".to_string(), item.to_string()]),
    );
    Ok(price)
}

/// Pays salaries every `economy.payroll_period` ticks from each faction's treasury.
pub fn payroll(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let period = world.resource::<Params>().get("economy.payroll_period", 24.0).max(1.0) as u64;
    if tick == 0 || tick % period != 0 {
        return;
    }
    let content = world.resource::<Content>().clone();
    let unpaid_dissent = world.resource::<Params>().f("economy.unpaid_dissent");
    let mut members: Vec<(String, SimId, Entity, String)> = Vec::new();
    let mut q = world.query_filtered::<(Entity, &SimId, &FactionMember), (Without<Dead>, Without<Virtual>)>();
    for (e, id, m) in q.iter(world) {
        members.push((m.faction.clone(), *id, e, m.rank.clone()));
    }
    members.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
    let mut paid: std::collections::BTreeMap<String, (f64, u32, u32)> = Default::default();
    for (f, _, e, rank) in members {
        let salary = world
            .resource::<Factions>()
            .states
            .get(&f)
            .and_then(|s| s.salaries.get(&rank).copied())
            .or_else(|| content.rank(&f, &rank).map(|r| r.salary))
            .unwrap_or(0.0);
        if salary <= 0.0 {
            continue;
        }
        let entry = paid.entry(f.clone()).or_default();
        if world.resource_mut::<Factions>().spend(&f, salary) {
            world.get_mut::<Wallet>(e).unwrap().0 += salary;
            entry.0 += salary;
            entry.1 += 1;
        } else {
            if let Some(mut d) = world.get_mut::<Dissent>(e) {
                d.0 = (d.0 + unpaid_dissent).min(100.0);
            }
            entry.2 += 1;
        }
    }
    for (f, (total, n, unpaid)) in paid {
        let name = content.factions.get(&f).map_or(f.clone(), |d| d.name.clone());
        let mut log = world.resource_mut::<EventLog>();
        log.push(
            tick,
            EventBuilder::new(kind::PAYROLL, format!("{name}: stipendi pagati a {n} membri ({total:.0})"))
                .faction(Some(f.clone()))
                .data(serde_json::json!({ "total": total, "paid": n, "unpaid": unpaid })),
        );
        if unpaid > 0 {
            log.push(
                tick,
                EventBuilder::new(kind::UNPAID, format!("{name}: {unpaid} membri senza stipendio"))
                    .faction(Some(f.clone()))
                    .news(0.4)
                    .tags(["economy", "unpaid"]),
            );
        }
        if let Some(s) = world.resource_mut::<Factions>().states.get_mut(&f) {
            s.unpaid_periods = if unpaid > 0 { s.unpaid_periods + 1 } else { 0 };
        }
    }
}
