//! Tunable numbers. Engine defaults live here, content packs override them and admins/MCP can change
//! them live (`set_parameter`).

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct Params {
    values: BTreeMap<String, f64>,
}

/// Every parameter the engine reads, with its default value and meaning.
pub const DEFAULTS: &[(&str, f64, &str)] = &[
    ("time.ticks_per_day", 24.0, "Tick in un giorno di gioco"),
    ("ai.momentum", 0.15, "Bonus di utilità dell'azione in corso (anti-oscillazione)"),
    ("ai.momentum_decay", 0.02, "Calo per tick del bonus di momentum"),
    ("ai.think_interval", 1.0, "Ogni quanti tick una pedina rivaluta le azioni"),
    ("ai.perception_range", 6.0, "Raggio di percezione di base (celle)"),
    ("move.base_speed", 1.0, "Passi per tick con speed = 1"),
    ("move.max_path_nodes", 20000.0, "Nodi massimi esplorati da A* per un percorso"),
    ("move.map_hop_cost", 25.0, "Costo (in caselle) di ogni passaggio da una mappa all'altra, per le scelte dell'AI"),
    ("crime.witness_base", 0.6, "Probabilità base che un testimone noti un crimine"),
    ("crime.report_multiplier", 1.0, "Moltiplicatore del livello di ricercato per crimine denunciato"),
    ("crime.unwitnessed_multiplier", 0.0, "Quota di ricercato anche senza testimoni"),
    ("crime.search_threshold", 5.0, "Livello di ricercato oltre cui la polizia perquisisce"),
    ("crime.arrest_threshold", 10.0, "Livello di ricercato oltre cui la polizia arresta"),
    ("crime.random_search_chance", 0.02, "Probabilità per tick di una perquisizione casuale"),
    ("crime.contraband_wanted", 3.0, "Ricercato aggiunto per ogni oggetto di contrabbando sequestrato"),
    ("crime.detention_ticks", 48.0, "Durata della detenzione"),
    ("crime.bribe_per_wanted", 10.0, "Costo della tangente per punto di ricercato"),
    ("crime.failed_bribe_wanted", 5.0, "Ricercato aggiunto per tentata corruzione"),
    ("crime.wanted_decay", 0.01, "Calo per tick del livello di ricercato"),
    ("economy.guild_income_share", 1.0, "Quota dei guadagni dei membri che va al fondo di gilda"),
    ("economy.payroll_period", 24.0, "Ogni quanti tick si pagano gli stipendi"),
    ("economy.unpaid_dissent", 10.0, "Dissenso per ogni stipendio non pagato"),
    ("economy.payroll_reserve", 5.0, "Periodi di stipendi che il fondo deve coprire per pagare al 100%"),
    ("economy.bonus_reserve", 20.0, "Periodi di stipendi in cassa oltre cui la gilda paga un bonus"),
    ("economy.max_pay_bonus", 1.5, "Moltiplicatore massimo degli stipendi quando la gilda è ricca"),
    ("economy.min_pay_ratio", 0.0, "Quota minima dello stipendio pagata quando le casse sono basse"),
    ("economy.export_interval", 24.0, "Ogni quanti tick gli edifici esportano e incassano dal turismo"),
    ("economy.export_batch", 6.0, "Unità massime di ogni merce esportata per volta"),
    ("economy.export_price", 0.9, "Prezzo di export come quota del prezzo di mercato"),
    ("economy.savings_cap", 10.0, "Risparmi (in stipendi) oltre cui si versa un contributo alla gilda"),
    ("economy.guild_contribution", 0.15, "Quota dei risparmi in eccesso versata alla gilda a ogni paga"),
    ("player.obedience_base", 0.85, "Probabilità base che un membro obbedisca al giocatore"),
    ("player.knockout_ticks", 12.0, "Tick in cui un campione resta a terra invece di morire"),
    ("player.knockout_money_loss", 0.2, "Quota dei soldi che il campione perde quando va al tappeto"),
    ("player.transmute_ticks", 24.0, "Tick dopo cui un leader o campione trasformato (es. in maiale) torna normale"),
    ("logistics.interval", 4.0, "Ogni quanti tick si pianificano i trasporti"),
    ("logistics.batch", 5.0, "Unità massime per viaggio di trasporto"),
    ("market.elasticity", 0.5, "Elasticità del prezzo rispetto a domanda/offerta"),
    ("market.smoothing", 0.2, "Velocità con cui il prezzo segue il valore teorico"),
    ("market.ema", 0.1, "Peso della media mobile di domanda e offerta"),
    ("market.min_mult", 0.25, "Prezzo minimo come multiplo del prezzo base"),
    ("market.max_mult", 5.0, "Prezzo massimo come multiplo del prezzo base"),
    ("market.change_event", 0.05, "Variazione relativa del prezzo che genera un evento"),
    ("press.perception_range", 8.0, "Raggio entro cui un giornalista registra un evento"),
    ("press.min_newsworthiness", 0.3, "Notiziabilità minima per uno scoop"),
    ("press.reputation_gain", 2.0, "Reputazione stampa guadagnata per articolo"),
    ("social.hostile_threshold", -30.0, "Relazione sotto cui due fazioni si considerano nemiche"),
    ("social.defection_threshold", 60.0, "Dissenso oltre cui una pedina diserta"),
    ("social.dissent_decay", 0.05, "Calo per tick del dissenso"),
    ("social.alliance_threshold", 40.0, "Relazione minima perché una fazione accetti un'alleanza dal giocatore"),
    ("social.merge_threshold", 90.0, "Relazione oltre cui due fazioni alleate si fondono"),
    ("social.ideology_violation_dissent", 5.0, "Dissenso per violazione ideologica vista"),
    ("health.bleed_threshold", 0.5, "Sotto questa frazione di HP una parte sanguina"),
    ("health.natural_heal", 0.05, "HP recuperati per tick da ogni parte"),
    ("hygiene.dirt_decay", 0.001, "Calo naturale dello sporco per tick"),
    ("hygiene.infection_scale", 0.02, "Probabilità di contagio per unità di carica patogena"),
    ("hygiene.network_flow", 0.1, "Quota di contaminazione che scorre lungo una rete per tick"),
    ("infiltration.suspicious_cover_loss", 10.0, "Copertura persa per azione sospetta vista"),
    ("infiltration.investigation_cover_loss", 25.0, "Copertura persa per inchiesta"),
    ("stealth.detection_scale", 0.1, "Peso della differenza percezione-stealth nel rilevamento"),
];

impl Default for Params {
    fn default() -> Self {
        Self {
            values: DEFAULTS.iter().map(|(k, v, _)| (k.to_string(), *v)).collect(),
        }
    }
}

impl Params {
    /// Value of `key`, or `fallback` for keys that neither the engine nor the content define.
    pub fn get(&self, key: &str, fallback: f64) -> f64 {
        self.values.get(key).copied().unwrap_or(fallback)
    }

    pub fn f(&self, key: &str) -> f32 {
        self.get(key, 0.0) as f32
    }

    pub fn set(&mut self, key: impl Into<String>, value: f64) -> Option<f64> {
        self.values.insert(key.into(), value)
    }

    pub fn all(&self) -> &BTreeMap<String, f64> {
        &self.values
    }

    pub fn describe(key: &str) -> Option<&'static str> {
        DEFAULTS.iter().find(|(k, _, _)| *k == key).map(|(_, _, d)| *d)
    }
}
