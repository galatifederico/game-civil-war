# engine — sim_core + fidenza_world

Engine di simulazione **headless, deterministico e data-driven** in Rust (ECS con `bevy_ecs` 0.19), più il
plugin dell'ambientazione satirica di Fidenza e Salsomaggiore. Design: [../docs/design.md](../docs/design.md),
piano: [../docs/tasks/00-piano.md](../docs/tasks/00-piano.md).

## Comandi

Serve solo Rust (stable, edition 2024). Tutto da questa cartella:

```bash
cargo run --release -p fidenza_world                 # demo: 100 tick, cronaca e verifica dei 4 punti del design
cargo run --release -p fidenza_world -- --verbose    # stampa tutti gli eventi
cargo run --release -p fidenza_world -- --serve      # dopo la demo resta acceso su http://127.0.0.1:8787
cargo run --release -p fidenza_world -- --serve 0.0.0.0:8787 --tick-ms 250
cargo run --release -p fidenza_world -- --mcp-stdio  # server MCP su stdin/stdout (per un client MCP)
cargo run --release -p fidenza_world --example profile   # millisecondi per sistema
cargo test                                            # test unitari e di integrazione
```

Altre opzioni: `--seed N`, `--ticks N`, `--compendium PATH` (default `compendium.json`, generato a ogni run).

## Struttura

```
crates/sim_core/          PARTE 1: motore agnostico (nessun nome dell'ambientazione nel codice)
  src/content/            definizioni serde, loader RON/JSON, registry con validazione dei riferimenti
      logic.rs            i linguaggi dati: Effect (verbi), Condition (predicati), Curve, Selector/Filter
  src/sim.rs              Simulation, SimBuilder, SimPlugin, ordine dei sistemi (SimSet)
  src/effects.rs          interprete di Effect e Condition
  src/ai.rs targeting.rs  Utility AI con momentum, ricerca dei bersagli
  src/jobs.rs handlers.rs JobBoard globale/per fazione, coda personale, WorkPriorityMatrix, handler generici
  src/squads.rs           squadre e ordini macro
  src/anatomy.rs status.rs hygiene.rs    corpo, status/patogeni/mutazioni, igiene e contagio
  src/infiltration.rs     mutaforma, furto d'identità, stealth, copertura e smascheramento
  src/crime.rs            ricercato, testimoni, perquisizioni, sequestri, arresti, tangenti
  src/market.rs economy.rs buildings.rs  mercato dinamico, fondo di gilda, stipendi, negozi, trasformazione
  src/factions.rs social.rs              fazioni, gerarchie, titoli, successione, diserzione, fusioni
  src/press.rs            scoop, feed social, impatti degli articoli, fonti di fake news
  src/dungeon.rs          spawner, tethering, trigger
  src/victory.rs          collezioni e condizioni di vittoria
  src/snapshot.rs         ActivityState, viste per UI/admin, nebbia di guerra, hash dello stato
  src/server/             Admin REST (axum), /metrics, MCP JSON-RPC  (feature `server`, attiva di default)
  tests/                  pacchetto generico "town" + test end-to-end e del server
crates/fidenza_world/     PARTE 2: il mondo come plugin
  data/*.ron              tutti i contenuti (caricati in ordine di nome file)
  src/lib.rs              FidenzaPlugin + estensioni custom (Oracolo, Borgazzi, intrusi, vicinanza, hacking)
  src/main.rs             la demo
  client/index.html       client web servito su /ui/
```

## Come funziona un tick

Fasi in ordine fisso, eseguite da un executor single-thread (determinismo):

1. **Input**: i `SimCommand` in coda (giocatore, admin, MCP) vengono applicati.
2. **Derive**: statistiche e tag effettivi (razza + classi + status + oggetti portati), bisogni.
3. **Health**: status (durate, stadi, escalation), guarigione, igiene e contagio.
4. **World**: tether dei dungeon, spawner, trigger.
5. **Ai**: ogni pedina (prima i gradi alti) valuta le azioni con le curve di utilità e sceglie un job.
6. **Act**: movimento e lavoro sui job; al completamento handler + effetti + crimini + testimoni.
7. **Economy**: edifici (produzione, conseguenze dei danni), mercato, stipendi.
8. **Social**: diserzioni, fusioni, successione, collezioni, vittoria.
9. **Press**: i giornalisti raccolgono scoop, le fonti automatiche pubblicano.
10. **Output**: ActivityState e metriche.

Ogni input esterno è un comando, l'RNG è unico e seedato, le entità sono sempre processate in ordine di `SimId`:
due run con lo stesso seed e gli stessi comandi producono lo stesso hash di stato (`Simulation::state_hash`).

## Aggiungere contenuti

Tutto è dato: una nuova malattia, classe, fazione o edificio non richiede codice. Esempio (una droga con escalation):

```ron
(
    statuses: [
        (id: "sballo", name: "Sballo", kind: Drug, duration: 6, stacking: Intensify,
         stats: {"percezione": 2}, ai_speed: 1.2, escalates_to: "trip", escalate_at: 3),
    ],
    items: [
        (id: "caramella", name: "Caramella strana", category: "sostanze", stack_max: 5, base_price: 12,
         tags: ["droga", "contrabbando"], on_use: [ApplyStatus(status: "sballo")]),
    ],
)
```

Un comportamento nuovo dell'AI è un'azione con considerazioni:

```ron
(id: "scippa", name: "Scippa", weight: 1.0, cooldown: 36,
 kind: Job(job: "scippo", target: Nearest((pawn: true, min_money: 20, relation: Other, max_distance: 12))),
 considerations: [(input: Money(max: 60), curve: Inverse)])
```

Il loader rifiuta riferimenti inesistenti con errori leggibili (`razza fidentino: piano corporeo 'x' non esiste`).
I parametri numerici (`crime.arrest_threshold`, `market.elasticity`, …) sono elencati in `src/params.rs` e si
cambiano da dati (`params: {...}`), da API o da MCP mentre la simulazione gira.

## Estendere con codice (plugin)

Quando i dati non bastano, un plugin registra per nome effetti, condizioni, input dell'AI e handler di job,
poi i dati li usano (`Custom(id: "...")`, `handler: "..."`):

```rust
impl SimPlugin for MioMondo {
    fn name(&self) -> &str { "mio_mondo" }
    fn build(&self, b: &mut SimBuilder) -> Result<(), ContentError> {
        b.load_pack_dir("data")?;
        b.register_effect("terremoto", |world, ctx, params| { /* ... */ });
        b.register_job_handler("hack", |world, job| JobResult::ok());
        b.add_systems(|s| { s.add_systems(mio_sistema.in_set(SimSet::World)); });
        Ok(())
    }
}
```

## API (con `--serve`)

| Metodo e percorso | Cosa fa |
|---|---|
| `GET /api/state?truth=true` | snapshot completo (con `truth` le identità nascoste) |
| `GET /api/entities`, `/api/entities/{id}`, `/api/entities/{id}/ai` | entità, scheda, ispezione della Utility AI (punteggi per azione e fattore) |
| `GET /api/events?since=&kind=&limit=` | log degli eventi |
| `GET /api/feed`, `/api/market`, `/api/factions`, `/api/jobs`, `/api/squads` | feed social, mercato, fazioni/punteggi, bacheca job, squadre |
| `GET /api/params`, `PUT /api/params/{key}` `{"value": x}` | parametri di bilanciamento live |
| `GET /api/sprites`, `PUT /api/sprites/{id}` | mapping grafico per il client |
| `POST /api/commands` (`?now=true`) , `GET /api/commands/{seq}` | invio di `SimCommand` (JSON con `type`) |
| `GET/POST /api/control` `{"paused", "tick_ms", "step"}` | pausa, velocità, avanzamento manuale |
| `GET /api/compendium`, `/api/extensions` | compendium per la wiki, estensioni registrate |
| `GET /api/ui/state?faction=`, `/api/ui/activity`, `/api/ui/map` | API per il client (con nebbia di guerra per fazione) |
| `GET /api/ui/terrain?since=N` | caselle cambiate dopo la N-esima modifica (scavi, pittura) |
| `GET /api/ui/player/{id}` | vista del giocatore: membri (razza, classe, salute, umore, soldi, attività, obbedienza), riepilogo e inventario del team, squadre, stipendi |
| `GET /api/ui/feed?filter=&player=` | il Piccione: `main` (canale principale: notizie sulla fazione del giocatore e grandi fatti, soglie `press.main_*`), `mine`, `all`, una categoria |
| `POST /api/ui/step` `{"player", "dx", "dy"}` | un passo del campione, subito (anche in pausa); risponde con la posizione |
| `GET /api/ui/economy` | indice dei prezzi, inflazione delle ultime 24 ore, moneta in circolazione, circostanze, quota locale, merci |
| `GET /admin/` | console di amministrazione |
| `GET /api/content`, `/api/assets`, `/assets/{file}.png` | tutte le definizioni caricate, elenco e file delle immagini |
| `GET /metrics` | metriche Prometheus |
| `POST /mcp` | MCP JSON-RPC 2.0: `get_world_state`, `set_parameter`, `spawn_entity`, `trigger_event`, `list_entities`, `inspect_entity`, `send_command`, `recent_events` |

Esempio di comando (la tangente della demo):

```bash
curl -X POST localhost:8787/api/commands -H 'content-type: application/json' \
  -d '{"type":"bribe","faction":"anarchici_commercio","target":2}'
```

## Prestazioni

Mondo di Fidenza (~120 pedine, ~75 edifici, superficie 200×140 più 4 livelli sotterranei): circa 6,5 ms per tick in release. Oltre qualche migliaio di pedine
servirà un indice spaziale a griglia per la ricerca dei bersagli (vedi `docs/questions/08-prestazioni.md`).
