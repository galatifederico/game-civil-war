//! MarketEngine: dynamic prices from scarcity (supply), demand, logistics disruption and temporary
//! shocks (news, global events, structural damage).

use std::collections::{BTreeMap, VecDeque};

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::buildings::{GlobalModifiers, Shop};
use crate::content::Content;
use crate::events::{kind, EventBuilder, EventLog};
use crate::inventory::Stock;
use crate::params::Params;
use crate::time::SimClock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Shock {
    pub demand: f32,
    pub supply: f32,
    pub until: u64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketItem {
    pub base: f64,
    pub price: f64,
    /// Purchases per tick (moving average) and its long-term normal level.
    pub demand: f32,
    pub normal_demand: f32,
    /// Units available in shops (moving average) and the reference level at start.
    pub supply: f32,
    pub reference_supply: f32,
    pub pending_demand: f32,
    pub shocks: Vec<Shock>,
    pub last_reported: f64,
    pub history: VecDeque<f64>,
}

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Market {
    pub items: BTreeMap<String, MarketItem>,
    /// Current global logistics disruption (0 = none).
    pub disruption: f32,
    initialized: bool,
}

impl Market {
    pub fn from_content(c: &Content) -> Self {
        let items = c
            .items
            .values()
            .filter(|i| i.base_price > 0.0)
            .map(|i| {
                (
                    i.id.clone(),
                    MarketItem {
                        base: i.base_price,
                        price: i.base_price,
                        demand: 0.0,
                        normal_demand: 0.0,
                        supply: 0.0,
                        reference_supply: 0.0,
                        pending_demand: 0.0,
                        shocks: vec![],
                        last_reported: i.base_price,
                        history: VecDeque::new(),
                    },
                )
            })
            .collect();
        Self { items, disruption: 0.0, initialized: false }
    }

    pub fn price(&self, item: &str) -> Option<f64> {
        self.items.get(item).map(|m| m.price)
    }

    pub fn record_purchase(&mut self, item: &str, qty: u32) {
        if let Some(m) = self.items.get_mut(item) {
            m.pending_demand += qty as f32;
        }
    }
}

/// Adds a temporary shock to one item or to every item with a tag.
pub fn add_shock(world: &mut World, item: Option<&str>, tag: Option<&str>, demand: f32, supply: f32, duration: u64, source: &str) {
    let tick = world.resource::<SimClock>().tick;
    let content = world.resource::<Content>().clone();
    let mut market = world.resource_mut::<Market>();
    for (id, m) in market.items.iter_mut() {
        let hit = item == Some(id.as_str())
            || tag.is_some_and(|t| content.items.get(id).is_some_and(|d| d.tags.iter().any(|x| x == t)));
        if hit {
            // The same source (e.g. news about arrests) refreshes its shock instead of stacking it.
            m.shocks.retain(|s| s.source != source);
            m.shocks.push(Shock { demand, supply, until: tick + duration, source: source.to_string() });
        }
    }
}

/// Updates supply/demand averages and prices; emits `price_change` events on significant moves.
pub fn update_market(world: &mut World) {
    let tick = world.resource::<SimClock>().tick;
    let p = world.resource::<Params>().clone();
    let (elasticity, smoothing, ema) = (p.f("market.elasticity"), p.get("market.smoothing", 0.2), p.f("market.ema"));
    let (min_m, max_m, change) = (p.get("market.min_mult", 0.25), p.get("market.max_mult", 5.0), p.get("market.change_event", 0.05));
    let mut supply: BTreeMap<String, u32> = BTreeMap::new();
    let mut q = world.query::<(&Shop, &Stock)>();
    for (shop, stock) in q.iter(world) {
        for item in shop.catalog.keys() {
            *supply.entry(item.clone()).or_insert(0) += stock.count(item);
        }
    }
    let disruption = world.resource::<GlobalModifiers>().logistics_disruption();
    let content = world.resource::<Content>().clone();
    let mut events = Vec::new();
    {
        let mut market = world.resource_mut::<Market>();
        market.disruption = disruption;
        let init = !market.initialized;
        market.initialized = true;
        for (id, m) in market.items.iter_mut() {
            let s = supply.get(id).copied().unwrap_or(0) as f32;
            if init {
                m.supply = s;
                m.reference_supply = s;
            }
            m.supply += (s - m.supply) * ema;
            m.demand += (m.pending_demand - m.demand) * ema;
            m.normal_demand += (m.demand - m.normal_demand) * ema * 0.1;
            m.pending_demand = 0.0;
            m.shocks.retain(|sh| sh.until > tick);
            let (sd, ss) = m.shocks.iter().fold((1.0f32, 1.0f32), |(d, s), sh| (d * sh.demand, s * sh.supply));
            let scarcity = (m.reference_supply + 1.0) / (m.supply * ss + 1.0);
            let demand = sd * (m.demand + 0.1) / (m.normal_demand + 0.1);
            let target = m.base
                * (scarcity.powf(elasticity) as f64)
                * (demand.powf(elasticity) as f64)
                * (1.0 + disruption as f64);
            let target = target.clamp(m.base * min_m, m.base * max_m);
            m.price += (target - m.price) * smoothing;
            m.history.push_back((m.price * 100.0).round() / 100.0);
            if m.history.len() > 72 {
                m.history.pop_front();
            }
            let rel = (m.price - m.last_reported) / m.last_reported.max(0.01);
            if rel.abs() >= change {
                let name = content.items.get(id).map_or(id.clone(), |d| d.name.clone());
                events.push((id.clone(), name, m.last_reported, m.price));
                m.last_reported = m.price;
            }
        }
    }
    let currency = content.bindings.currency_name.clone();
    for (id, name, old, new) in events {
        let pct = (new - old) / old * 100.0;
        world.resource_mut::<EventLog>().push(
            tick,
            EventBuilder::new(kind::PRICE_CHANGE, format!("Prezzo di {name}: {old:.1} → {new:.1} {currency} ({pct:+.0}%)"))
                .tags(["market".to_string(), id.clone()])
                .data(serde_json::json!({ "item": id, "old": old, "new": new })),
        );
    }
}
