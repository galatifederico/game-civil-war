# Piano: engine di simulazione headless in Rust

Fonte: [../design.md](../design.md). Domande aperte e decisioni prese in autonomia: [../questions/](../questions/).

## Architettura in breve

Workspace Cargo in `engine/`. Il vecchio backend Go, il client Unity e l'admin web sono stati cancellati su
richiesta (restano nella storia git, vedi [domanda 01](../questions/01-repo-rust-vs-go.md)):

```
engine/
├── Cargo.toml                 # workspace
├── crates/sim_core/           # PARTE 1: motore agnostico (nessun nome dell'ambientazione nel codice)
│   └── src/ …                 # un modulo per sottosistema; server HTTP/MCP dietro la feature `server`
└── crates/fidenza_world/      # PARTE 2: plugin dell'ambientazione
    ├── data/*.ron             # contenuti (razze, classi, fazioni, oggetti, malattie, edifici, …)
    └── src/{lib.rs,main.rs}   # registrazione di effetti/condizioni/job custom + demo da 100 tick
```

Principi:
- **ECS con `bevy_ecs` 0.19**, executor single-thread e sistemi in ordine esplicito (`chain`) → deterministico.
- **Determinismo**: un solo RNG seedato (`SimRng`), iterazioni ordinate per `SimId`, niente `HashMap` nello
  stato osservabile (solo `BTreeMap`), hash dello stato per i test di replay.
- **Data-driven**: tutto ciò che è "contenuto" è un id stringa definito nei file RON (statistiche, parti del
  corpo, status, oggetti, job, azioni di Utility AI, fazioni, edifici, trigger…). Il comportamento è
  composto da tre linguaggi dati: `Effect` (verbi), `Condition` (predicati), `Curve` (utilità).
- **Estendibilità**: `SimPlugin` registra contenuti + effetti/condizioni/handler di job custom per nome.
- **Tutti gli input esterni** (giocatore, admin, MCP) sono `SimCommand` in coda, applicati a inizio tick
  → replay e test semplici.

## Task (ordine di priorità)

| # | Task | Stato |
|---|------|-------|
| 01 | [Fondamenta: workspace, tick, RNG, id, params, eventi, comandi, plugin](01-fondamenta.md) | |
| 02 | [Contenuti data-driven: definizioni, loader RON, registry, validazione](02-contenuti.md) | |
| 03 | [Mappa, posizioni, movimento, zone e livelli](03-mappa.md) | |
| 04 | [Statistiche, bisogni, inventario](04-statistiche-inventario.md) | |
| 05 | [Fazioni, gerarchia, tesoro di gilda, stipendi](05-fazioni-tesoro.md) | |
| 06 | [Utility AI, JobQueue, WorkPriorityMatrix, squadre, ActivityState](06-ai-job.md) | |
| 07 | [Crimine: WantedLevel, polizia neutra, perquisizione, arresto, tangenti](07-crimine.md) | |
| 08 | [Mercato dinamico e negozi](08-mercato.md) | |
| 09 | [Stampa e feed social](09-stampa.md) | |
| 10 | [Demo `main.rs` (scenario crimine → arresto → tangente → scoop → prezzi)](10-demo.md) | |
| 11 | [Anatomia, status, patogeni, mutazioni, medicina](11-salute.md) | |
| 12 | [Igiene, fluidi, contagio ambientale e infrastrutture](12-igiene.md) | |
| 13 | [Infiltrazione: mutaforma, furto d'identità, stealth, smascheramento](13-infiltrazione.md) | |
| 14 | [Edifici di trasformazione, danni strutturali e conseguenze globali](14-edifici.md) | |
| 15 | [Dungeon, spawn, tethering, trigger](15-dungeon.md) | |
| 16 | [Successione, fusione e diserzione, collezioni, vittoria](16-successione-vittoria.md) | |
| 17 | [Telemetria, Admin HTTP, MCP, Compendium](17-server.md) | |
| 18 | [Contenuti completi di Fidenza](18-contenuti-fidenza.md) | |
| 19 | [Client grafico (futuro)](19-integrazione.md) | |

Le task 01-10 formano la "fetta verticale" che fa girare la demo richiesta dal design; le 11-18 completano i
sottosistemi; la 19 è fuori scopo per ora.
