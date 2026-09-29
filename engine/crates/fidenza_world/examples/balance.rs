//! Balance report: `cargo run --release -p fidenza_world --example balance -- [days] [seeds]`.
//! Prints, per day and averaged over seeds, population, deaths, arrivals, morale, a price index,
//! treasuries and who holds the major relics.

use std::collections::BTreeMap;

use sim_core::prelude::*;

fn main() {
    let mut a = std::env::args().skip(1);
    let days: u64 = a.next().and_then(|v| v.parse().ok()).unwrap_or(30);
    let seeds: u64 = a.next().and_then(|v| v.parse().ok()).unwrap_or(3);
    let mut rows: BTreeMap<u64, Vec<[f64; 8]>> = BTreeMap::new();
    let mut endings = Vec::new();
    let mut hunger: BTreeMap<u64, Vec<(f64, f64, f64)>> = BTreeMap::new();
    let mut money: BTreeMap<u64, Vec<(f64, f64, f64)>> = BTreeMap::new();
    for seed in 1..=seeds {
        let mut sim = fidenza_world::build(seed).unwrap();
        let morale = sim.content().bindings.morale.clone();
        for day in 1..=days {
            sim.run(24);
            let snap = sim.snapshot(true);
            let pawns: Vec<_> = snap.entities.iter().filter(|e| e.kind == "pawn").collect();
            let alive = pawns.iter().filter(|e| !e.dead).count() as f64;
            let ev = sim.events().all();
            let today = |k: &str| ev.iter().filter(|e| e.kind == k && e.tick + 24 >= sim.tick_count()).count() as f64;
            let avg_morale = pawns.iter().filter(|e| !e.dead).map(|e| *e.stats.get(&morale).unwrap_or(&50.0) as f64).sum::<f64>() / alive.max(1.0);
            let price_index = snap.market.iter().map(|m| m.price / m.base).sum::<f64>() / snap.market.len().max(1) as f64;
            let treasury: f64 = snap.factions.iter().map(|f| f.treasury).sum();
            let refused = today("order_refused");
            let living: Vec<_> = pawns.iter().filter(|e| !e.dead && e.needs.contains_key("fame")).collect();
            let fame = living.iter().map(|e| e.needs["fame"] as f64).sum::<f64>() / living.len().max(1) as f64;
            let broke = living.iter().filter(|e| e.money < 3.0).count() as f64 / living.len().max(1) as f64;
            let food: f64 = snap.entities.iter().filter_map(|e| e.building.as_ref()).filter(|b| !b.sells.is_empty())
                .map(|b| b.stock.iter().filter(|(i, _)| ["pane", "salumi", "carne", "ortaggi", "fake_meat"].contains(&i.as_str())).map(|(_, n)| *n as f64).sum::<f64>()).sum();
            let svago = living.iter().map(|e| *e.needs.get("svago").unwrap_or(&1.0) as f64).sum::<f64>() / living.len().max(1) as f64;
            let riposo = living.iter().map(|e| *e.needs.get("riposo").unwrap_or(&1.0) as f64).sum::<f64>() / living.len().max(1) as f64;
            if day % 3 == 1 { println!("  [seed {seed} giorno {day}] svago {svago:.2} riposo {riposo:.2}"); }
            hunger.entry(day).or_default().push((fame, broke, food));
            let wallets_alive: f64 = pawns.iter().filter(|e| !e.dead).map(|e| e.money).sum();
            let wallets_dead: f64 = pawns.iter().filter(|e| e.dead).map(|e| e.money).sum();
            money.entry(day).or_default().push((treasury, wallets_alive, wallets_dead));
            rows.entry(day).or_default().push([alive, today("death"), today("spawn"), avg_morale, price_index, treasury, today("crime"), refused + today("article")]);
        }
        let holdings = sim_core::inventory_ops::all_faction_holdings(&mut sim.world);
        let mut relics = Vec::new();
        for (f, items) in holdings {
            let n = items.keys().filter(|i| sim.content().items.get(*i).is_some_and(|d| d.tags.iter().any(|t| t == "reliquia_maggiore"))).count();
            if n > 0 {
                relics.push(format!("{f}:{n}"));
            }
        }
        let winner = sim.world.resource::<Progress>().winner.clone();
        endings.push(format!("seed {seed}: reliquie maggiori {} · vincitore {:?}", relics.join(" "), winner.map(|w| (w.faction, w.victory, w.tick))));
    }
    println!("giorno  vivi  morti  arrivi  morale  prezzi  tesoro   crimini  articoli");
    for (day, v) in &rows {
        let n = v.len() as f64;
        let avg = |i: usize| v.iter().map(|r| r[i]).sum::<f64>() / n;
        println!("{day:>6} {:>5.0} {:>6.1} {:>7.1} {:>7.1} {:>7.2} {:>7.0} {:>8.1} {:>9.1}", avg(0), avg(1), avg(2), avg(3), avg(4), avg(5), avg(6), avg(7));
    }
    println!("giorno  fame  al_verde  cibo_nei_negozi");
    for (day, v) in hunger.iter().step_by(3) {
        let n = v.len() as f64;
        println!("{day:>6} {:>5.2} {:>9.2} {:>16.0}", v.iter().map(|x| x.0).sum::<f64>() / n, v.iter().map(|x| x.1).sum::<f64>() / n, v.iter().map(|x| x.2).sum::<f64>() / n);
    }
    println!("giorno  tesori  portafogli_vivi  portafogli_morti  totale");
    for (day, v) in money.iter().step_by(3) {
        let n = v.len() as f64;
        let (a, b, c) = (v.iter().map(|x| x.0).sum::<f64>() / n, v.iter().map(|x| x.1).sum::<f64>() / n, v.iter().map(|x| x.2).sum::<f64>() / n);
        println!("{day:>6} {a:>7.0} {b:>16.0} {c:>17.0} {:>7.0}", a + b + c);
    }
    for e in endings {
        println!("{e}");
    }
}
