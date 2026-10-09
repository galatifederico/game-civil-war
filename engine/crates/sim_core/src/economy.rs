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

/// Price of an item in a shop: the owner's fixed price, or the local market price × markup.
pub fn shop_price(world: &World, shop: &Shop, stock: &Stock, item: &str) -> Option<f64> {
    let fixed = shop.catalog.get(item)?;
    let elasticity = world.resource::<Params>().f("market.elasticity");
    fixed.or_else(|| world.resource::<Market>().local_price(item, stock.count(item), elasticity).map(|p| p * shop.markup as f64))
}

/// Buys one unit of `item` from `shop_e` for `buyer`. Returns the price paid.
pub fn buy(world: &mut World, buyer: Entity, shop_e: Entity, item: &str) -> Result<f64, String> {
    let shop = world.get::<Shop>(shop_e).cloned().ok_or("non è un negozio")?;
    let stock = world.get::<Stock>(shop_e).cloned().unwrap_or_default();
    if stock.count(item) == 0 {
        return Err("esaurito".into());
    }
    let price = shop_price(world, &shop, &stock, item).ok_or("non in vendita")?;
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
    if let Some(c) = world.get::<crate::hygiene::Contaminated>(shop_e).cloned()
        && content.items.get(item).is_some_and(|d| d.tags.iter().any(|t| content.bindings.ingestible_tags.contains(t))) {
            crate::status::apply_status(world, buyer, &c.status, 1.0, None);
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

/// Exports and tourism: money entering the world. Every `economy.export_interval` ticks each working
/// building sells up to `economy.export_batch` units of its export goods at the market price and earns
/// its visitor income; both go to the owner (a faction's guild treasury).
pub fn exports(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let p = world.resource::<Params>().clone();
    let interval = p.get("economy.export_interval", 24.0).max(1.0) as u64;
    if tick == 0 || !tick.is_multiple_of(interval) {
        return;
    }
    let (batch, factor) = (p.get("economy.export_batch", 6.0) as u32, p.get("economy.export_price", 0.9));
    let content = world.resource::<Content>().clone();
    let mut totals: std::collections::BTreeMap<String, f64> = Default::default();
    for e in crate::sorted_entities::<Building>(world) {
        let b = world.get::<Building>(e).unwrap().clone();
        let Some(def) = content.buildings.get(&b.def) else { continue };
        if b.hp <= 0.0 || def.exports.is_empty() {
            continue;
        }
        let mut earned = 0.0;
        for item in &def.exports {
            let n = world.get::<crate::inventory::Stock>(e).map_or(0, |s| s.count(item)).min(batch);
            if n == 0 {
                continue;
            }
            let price = world.resource::<Market>().price(item).unwrap_or(0.0) * factor;
            world.get_mut::<crate::inventory::Stock>(e).unwrap().remove(item, n);
            earned += price * n as f64;
        }
        if earned > 0.0 {
            earn_owner(world, &b.owner, earned);
            if let Owner::Faction(f) = &b.owner {
                *totals.entry(f.clone()).or_default() += earned;
            }
        }
    }
    for (f, total) in totals {
        let name = content.factions.get(&f).map_or(f.clone(), |d| d.name.clone());
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new("exports", format!("{name}: incassi da export e visitatori {total:.0}")).faction(Some(f)).tags(["economy"]),
        );
    }
}

/// PayrollEngine, every `economy.payroll_period` ticks:
/// - salaries are paid from the guild treasury, scaled down (never below `economy.min_pay_ratio`) when the
///   treasury holds less than `economy.payroll_reserve` periods of payroll, instead of going bankrupt;
///   the missing part raises dissent proportionally;
/// - members whose savings exceed `economy.savings_cap` salaries pay `economy.guild_contribution` of the
///   excess back to the guild (the GuildTreasury centralizes the members' wealth). Champions are exempt.
pub fn payroll(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let p = world.resource::<Params>().clone();
    let period = p.get("economy.payroll_period", 24.0).max(1.0) as u64;
    if tick == 0 || !tick.is_multiple_of(period) {
        return;
    }
    let content = world.resource::<Content>().clone();
    let unpaid_dissent = p.f("economy.unpaid_dissent");
    let reserve = p.get("economy.payroll_reserve", 5.0).max(0.0);
    let min_ratio = p.get("economy.min_pay_ratio", 0.3).clamp(0.0, 1.0);
    let savings_cap = p.get("economy.savings_cap", 6.0).max(0.0);
    let contribution = p.get("economy.guild_contribution", 0.25).clamp(0.0, 1.0);
    let mut by_faction: std::collections::BTreeMap<String, Vec<(Entity, f64, bool)>> = Default::default();
    let mut q = world.query_filtered::<(Entity, &SimId, &FactionMember, Option<&crate::factions::Leader>), (Without<Dead>, Without<Virtual>)>();
    let mut members: Vec<(SimId, Entity, FactionMember, bool)> = q.iter(world).map(|(e, id, m, l)| (*id, e, m.clone(), l.is_some())).collect();
    members.sort_by_key(|m| m.0);
    for (_, e, m, champion) in members {
        let salary = world
            .resource::<Factions>()
            .states
            .get(&m.faction)
            .and_then(|s| s.salaries.get(&m.rank).copied())
            .or_else(|| content.rank(&m.faction, &m.rank).map(|r| r.salary))
            .unwrap_or(0.0);
        by_faction.entry(m.faction.clone()).or_default().push((e, salary, champion));
    }
    for (f, list) in by_faction {
        let total: f64 = list.iter().map(|(_, s, _)| *s).sum();
        let treasury = world.resource::<Factions>().treasury(&f);
        let bonus_reserve = p.get("economy.bonus_reserve", 20.0);
        let max_bonus = p.get("economy.max_pay_bonus", 1.5).max(1.0);
        let ratio = if total <= 0.0 || reserve == 0.0 {
            1.0
        } else if treasury >= total * bonus_reserve {
            // Prosperity is shared: a rich guild pays more (up to `economy.max_pay_bonus`).
            (treasury / (total * bonus_reserve)).min(max_bonus)
        } else if treasury >= total * reserve {
            1.0
        } else {
            (treasury / (total * reserve)).clamp(min_ratio, 1.0)
        };
        let (mut paid_total, mut paid, mut unpaid, mut contributed) = (0.0, 0u32, 0u32, 0.0);
        for (e, salary, champion) in list {
            if salary > 0.0 {
                let pay = (salary * ratio * 100.0).round() / 100.0;
                if world.resource_mut::<Factions>().spend(&f, pay) {
                    world.get_mut::<Wallet>(e).unwrap().0 += pay;
                    paid_total += pay;
                    paid += 1;
                    if ratio < 1.0 {
                        if let Some(mut d) = world.get_mut::<Dissent>(e) {
                            d.0 = (d.0 + unpaid_dissent * (1.0 - ratio) as f32).min(100.0);
                        }
                    }
                } else {
                    if let Some(mut d) = world.get_mut::<Dissent>(e) {
                        d.0 = (d.0 + unpaid_dissent).min(100.0);
                    }
                    unpaid += 1;
                }
            }
            // Savings above the cap flow back to the guild.
            let cap = savings_cap * salary.max(5.0);
            if !champion && contribution > 0.0 {
                let mut w = world.get_mut::<Wallet>(e).unwrap();
                if w.0 > cap {
                    let c = ((w.0 - cap) * contribution * 100.0).round() / 100.0;
                    w.0 -= c;
                    contributed += c;
                }
            }
        }
        if contributed > 0.0 {
            world.resource_mut::<Factions>().add_treasury(&f, contributed);
        }
        if paid == 0 && unpaid == 0 && contributed == 0.0 {
            continue;
        }
        let name = content.factions.get(&f).map_or(f.clone(), |d| d.name.clone());
        let share = if (ratio - 1.0).abs() > 0.005 { format!(" al {:.0}%", ratio * 100.0) } else { String::new() };
        let contrib = if contributed > 0.0 { format!(", contributi dei membri {contributed:.0}") } else { String::new() };
        let mut log = world.resource_mut::<EventLog>();
        log.push(
            tick,
            EventBuilder::new(kind::PAYROLL, format!("{name}: stipendi{share} pagati a {paid} membri ({paid_total:.0}){contrib}"))
                .faction(Some(f.clone()))
                .data(serde_json::json!({ "total": paid_total, "paid": paid, "unpaid": unpaid, "ratio": ratio, "contributions": contributed })),
        );
        if unpaid > 0 || ratio < 0.6 {
            log.push(
                tick,
                EventBuilder::new(kind::UNPAID, format!("{name}: casse in difficoltà, stipendi{share}{}", if unpaid > 0 { format!(", {unpaid} membri senza paga") } else { String::new() }))
                    .faction(Some(f.clone()))
                    .news(0.4)
                    .tags(["economy", "unpaid"]),
            );
        }
        if let Some(s) = world.resource_mut::<Factions>().states.get_mut(&f) {
            s.unpaid_periods = if unpaid > 0 || ratio < 1.0 { s.unpaid_periods + 1 } else { 0 };
        }
    }
}
