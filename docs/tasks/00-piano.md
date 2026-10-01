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
| 01 | [Fondamenta: workspace, tick, RNG, id, params, eventi, comandi, plugin](01-fondamenta.md) | ✅ fatto |
| 02 | [Contenuti data-driven: definizioni, loader RON, registry, validazione](02-contenuti.md) | ✅ fatto |
| 03 | [Mappa, posizioni, movimento, zone e livelli](03-mappa.md) | ✅ fatto (senza ostacoli, vedi domanda 04) |
| 04 | [Statistiche, bisogni, inventario](04-statistiche-inventario.md) | ✅ fatto |
| 05 | [Fazioni, gerarchia, tesoro di gilda, stipendi](05-fazioni-tesoro.md) | ✅ fatto |
| 06 | [Utility AI, JobQueue, WorkPriorityMatrix, squadre, ActivityState](06-ai-job.md) | ✅ fatto |
| 07 | [Crimine: WantedLevel, polizia neutra, perquisizione, arresto, tangenti](07-crimine.md) | ✅ fatto |
| 08 | [Mercato dinamico e negozi](08-mercato.md) | ✅ fatto (+ prezzi locali per negozio) |
| 09 | [Stampa e feed social](09-stampa.md) | ✅ fatto |
| 10 | [Demo `main.rs` (scenario crimine → arresto → tangente → scoop → prezzi)](10-demo.md) | ✅ fatto |
| 11 | [Anatomia, status, patogeni, mutazioni, medicina](11-salute.md) | ✅ fatto |
| 12 | [Igiene, fluidi, contagio ambientale e infrastrutture](12-igiene.md) | ✅ fatto |
| 13 | [Infiltrazione: mutaforma, furto d'identità, stealth, smascheramento](13-infiltrazione.md) | ✅ fatto |
| 14 | [Edifici di trasformazione, danni strutturali e conseguenze globali](14-edifici.md) | ✅ fatto (+ logistica delle catene) |
| 15 | [Dungeon, spawn, tethering, trigger](15-dungeon.md) | ✅ fatto |
| 16 | [Successione, fusione e diserzione, collezioni, vittoria](16-successione-vittoria.md) | ✅ fatto |
| 17 | [Telemetria, Admin HTTP, MCP, Compendium](17-server.md) | ✅ fatto |
| 18 | [Contenuti completi di Fidenza](18-contenuti-fidenza.md) | ✅ fatto (da bilanciare) |
| 19 | [Client grafico con sprite semplici (ultima)](19-integrazione.md) | ✅ web + Unity |
| 20 | [Campione, ordini, strategia, pathfinding, salvataggi, bilanciamento](20-giocatore-e-bilanciamento.md) | ✅ fatto |
| 21 | [Sprite chibi in pixel art (stile Pokémon)](21-sprite-chibi.md) | ✅ prima versione |
| 22 | [Regione in mappe contigue, terreno in pixel art](22-mappe-contigue.md) | ✅ prima versione |
| 23 | [Grafica più vicina a Pokémon](23-grafica-pokemon.md) | ✅ fatto |
| 24 | [Un mondo unico, come Dwarf Fortress](24-mondo-unico.md) | ✅ fatto |
| 25 | [Rifornimenti da fuori e luoghi di divertimento](25-rifornimenti-e-svago.md) | ✅ fatto |
| 26 | [Conquista dei quartieri](26-conquista-quartieri.md) | ✅ fatto |
| 27 | [Console di amministrazione](27-console-admin.md) | ✅ fatto |

Le task 01-10 formano la "fetta verticale" che fa girare la demo richiesta dal design; le 11-18 completano i
sottosistemi; la 19 è fuori scopo per ora.

## Stato al 2026-09-27

Tutte le task hanno una prima implementazione funzionante e testata (`cargo test`: unitari, pacchetto generico
"town", mondo di Fidenza, server HTTP/MCP su socket reale). La demo `cargo run --release -p fidenza_world`
verifica da sola i 4 punti richiesti dal design. Comandi e architettura: [../../engine/README.md](../../engine/README.md).

Prossimi passi possibili, in ordine di valore:
1. **Bilanciamento** del mondo di Fidenza (violenza, prezzi, produzione): tutto è nei file `data/*.ron` e nei
   parametri live; vedi [domanda 06](../questions/06-violenza-e-bilanciamento.md).
2. **Client**: scheda pedina più ricca, comandi del giocatore dal client (squadre, stipendi, ordini), sprite veri.
3. **Pathfinding** con ostacoli e indice spaziale a griglia per mondi grandi (domande 04 e 08).
4. **Salvataggio/caricamento** dello stato (oggi il replay deterministico basta per i test, non per le partite lunghe).
5. **Localizzazione** dei messaggi del motore (domanda 07).
