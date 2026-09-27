# 02 — Formato dei file di configurazione

**Contesto.** Il design ammette JSON, RON o TOML.

**Decisione presa.** RON per i dati del mondo (`crates/fidenza_world/data/*.ron`): supporta commenti, enum
Rust nativi (utili per `Effect`/`Condition`/`Curve`) ed è leggibile. JSON per tutto ciò che esce
(API, MCP, `compendium.json`). Il loader accetta anche file `.json` con la stessa struttura.

**Alternative.** Solo JSON (più universale, niente commenti); TOML (verboso con liste di strutture annidate).
