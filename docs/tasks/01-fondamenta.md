# 01 — Fondamenta

Priorità: 1 (blocca tutto).

## Obiettivo
Workspace Cargo `engine/` con i crate `sim_core` e `fidenza_world`, e l'ossatura della simulazione.

## Contenuto
- `Simulation`: incapsula `bevy_ecs::World` + `Schedule` (executor single-thread), `tick()`, `run(n)`.
- `SimClock` (tick corrente, tick per giorno), `SimRng` (SplitMix64 seedato, niente dipendenze esterne).
- `SimId` stabile per entità + `IdIndex` (SimId → Entity).
- `Params`: mappa `String → f64` con default del motore sovrascrivibili dal pacchetto dati e a runtime.
- `EventLog`: eventi di simulazione tipizzati (`SimEvent`), con posizione e "notiziabilità" (serve alla stampa).
- `CommandQueue` di `SimCommand` applicati a inizio tick (giocatore, admin, MCP).
- `EffectQueue` + risoluzione degli `Effect` in un sistema esclusivo; registri di effetti/condizioni custom.
- `SimPlugin` + `SimBuilder`.
- Hash dello stato per il test di determinismo.

## Criteri di accettazione
- `cargo test` verde; due simulazioni con lo stesso seed producono lo stesso hash dopo N tick.
