# Roadmap tecnica e stato di avanzamento

Questo file traduce le decisioni di [design.md](design.md) e [tecnico.md](tecnico.md) in un ordine di sviluppo concreto, e tiene traccia di cosa è già stato costruito. Se riprendi il progetto in una sessione futura (io o l'utente), parti da qui: la sezione "Stato attuale" dice esattamente cosa funziona e cosa manca.

## Struttura repo

```
the_game/
├── docs/            # documentazione (concept, design, tecnico, questa roadmap)
├── backend/         # server Go autoritativo (REST + WebSocket + simulazione)
├── godot-client/     # client Godot 4.x (mobile, isometrico — per ora rendering placeholder)
├── admin-web/       # pannello admin (web app separata) — non ancora iniziato, arriva a M6
└── deploy/          # docker-compose (Postgres + Redis) per lo sviluppo/self-hosting
```

## Milestone

- **M0 — Scaffold**: repo git, scheletro dei tre progetti, `docker-compose.yml`, endpoint `/healthz`, progetto Godot avviabile. **✅ Fatto.**
- **M1 — Slice verticale**: login reale (JWT), un'unica board quadrata seedata via migrazione, un campione per giocatore che si muove via WebSocket, persistito su Postgres. **✅ Fatto e verificato end-to-end** (due account, due connessioni WS, movimento di un client visibile in tempo reale sull'altro).
- **M2 — Redis + tick loop + le 4 azioni a raggio**: stato caldo (posizione/cooldown) spostato su Redis con flush periodico; pedine minori oltre al campione; attack/pickup/talk/build con comportamento hardcoded (nessuna regola admin ancora). **🚧 Parzialmente fatto** — vedi dettaglio sotto: movimento con cooldown reale, pedine minori, attacco e ciclo morte/respawn sono fatti e testati; Redis, pickup, talk, build restano da fare.
- **M3 — Multi-board + griglie miste**: `board_links`, passaggio di board tra goroutine, board esagonale di prova. **⬜ Da fare.** (Nota: l'astrazione `Grid`/`SquareGrid`/`HexGrid` in `backend/internal/world/` esiste già e supporta entrambe le forme — manca solo l'orchestrazione multi-board.)
- **M4 — Motore regole (seed via SQL, no UI)**: `races`, `race_compatibility`, `creation_rules`, riproduzione con trigger esplicito, inventario condiviso solo-campione. **⬜ Da fare.**
- **M5 — Obiettivi, punteggio, condizioni di vittoria (seed via SQL)**. **⬜ Da fare.**
- **M6 — Admin web app**: CRUD reale (`admin-web/`) + handler REST `/admin/*`, lobby mondi lato giocatore. È qui che si raggiunge l'MVP "ambizioso" descritto in design.md. **⬜ Da fare.**
- **M7 — Hardening**: riconnessione/afk, tileset pixel art reale, recovery per board, Cloudflare Tunnel per l'accesso remoto. **⬜ Da fare.**

## Stato attuale in dettaglio (fine sessione)

### Cosa gira davvero
- `docker-compose` in `deploy/` avvia Postgres (host port **5433**, non 5432 — occupata da un altro progetto su questa macchina) e Redis (6379, non ancora usato dal backend).
- Migrazione `backend/migrations/0001_init.sql` applicata: schema `players/worlds/boards/player_world_membership/units`, con un mondo e una board (32x32, griglia quadrata) seedati.
- Backend Go compila e gira (verificato con `make build` / `make run` in `backend/`, usando l'immagine Docker `golang:1.23-alpine` con `GOTOOLCHAIN=auto` — **Go non è installato sull'host**, vedi "Note operative" sotto).
- Testato manualmente end-to-end: `POST /auth/register`, `POST /auth/login`, connessione WebSocket con handshake `auth`, ricezione `world_snapshot` (con `your_unit_id` per farsi riconoscere dal client), invio `move_command`, ricezione `state_delta` broadcast a tutti i client sulla board, rimozione dell'unità dal broadcast alla disconnessione.
- Progetto Godot (`godot-client/`) con scena di login (REST register/login), scena di gioco (griglia disegnata, click sinistro per muovere il proprio campione, unità renderizzate come rettangoli colorati — oro per il campione, blu per le pedine minori). Verificato che il progetto si importa e si avvia senza errori in `godot --headless` (non testato con interazione utente reale/mouse, dato l'ambiente headless).

### M2 — cosa è stato aggiunto e testato in questa sessione
- **Migrazione `0002_units_combat_state.sql`**: aggiunte `max_health`, `alive`, `respawn_at` alla tabella `units`.
- **Cooldown di movimento reale**: `speed` ora è "celle al secondo" — dopo una mossa l'unità non può muoversi di nuovo finché non passa `distanza/speed` secondi (`sim.BoardLoop.applyMove`, campo `MoveReadyAt`, non persistito — è stato realtime effimero). `BoardLoop.Run` ora ha un vero ticker (100ms) oltre a processare i comandi in coda.
- **Creazione pedine minori** (`create_unit_command`): un solo metodo hardcoded per ora — il campione paga 20 punti vita (`store.ChampionCostPerMinorUnit`) e genera una pedina minore (speed 2, vita 50) accanto a sé. Gli altri 3 metodi previsti dal design (risorse raccolte, interazione con edifici, riproduzione tra pedine) restano da fare in M4 insieme al motore regole.
- **Azione "attack"** (`action_command`): raggio d'azione hardcoded a 1 casella (diventerà dipendente dalla caratteristica "vista" in M4), danno fisso 25. Verificato: non si può attaccare una propria pedina; attaccare un bersaglio senza unità in quella casella dà errore.
- **Morte e respawn**: alla vita ≤ 0 l'unità diventa `alive=false` e torna in vita dopo 10s con vita piena, **senza altre penalità** (coerente con design.md: nessuna perdita di punti/oggetti/posizione, nessun permadeath). Verificato end-to-end con un test che aspetta il respawn reale.
- Test manuali (script Node.js in scratchpad, non versionati nel repo) hanno validato l'intera catena: creazione pedina → broadcast a entrambi i client → attacco ripetuto → morte → attesa 10s → respawn a vita piena, visibile su entrambe le connessioni.
- **`Makefile` alla radice del repo**: `make start-local` (o `make start:local`) avvia Postgres/Redis, applica tutte le migrazioni in ordine e fa partire il backend in foreground; `make stop-local` ferma tutto. Le migrazioni in `backend/migrations/` sono scritte per essere **idempotenti** (`CREATE TABLE IF NOT EXISTS`, `ON CONFLICT DO NOTHING`, `ADD COLUMN IF NOT EXISTS`) proprio per poter essere rilanciate sempre senza tracking separato — ogni nuova migrazione va scritta con lo stesso criterio.

### Cosa manca prima di M2 completo
- **Pickup, talk, build**: definiti nel protocollo (`protocol.ActionPickup/Talk/Build`) e nello switch di `BoardLoop.HandleAction`, ma rispondono esplicitamente "not implemented yet" — nessuna logica di oggetti, dialogo NPC o conquista territorio ancora.
- **Nessun uso di Redis**: la posizione/salute viene scritta su Postgres ad ogni mossa/azione in modo asincrono "fire and forget" (eccetto la creazione pedina, sincrona). Va bene per pochi giocatori; da rivedere quando il numero di pedine per giocatore cresce molto (vedi tecnico.md).
- Riproduzione tra pedine, razze, oggetti sulla board, inventario condiviso: non ancora iniziati (arrivano con M4/M5).

### Note operative importanti
- **Go non è installato sull'host** (Manjaro, niente sudo passwordless disponibile in questa sessione). Tutti i comandi Go passano per Docker — vedi `backend/Makefile` (`make build`, `make vet`, `make tidy`, `make run`). Se in futuro si installa Go nativamente, gli stessi comandi (`go build ./...`, `go run ./cmd/server`) funzionano direttamente in `backend/`.
- Porta Postgres locale: **5433** (non 5432, già occupata da `main-db-1`, un container non legato a questo progetto).
- `make run` collega il container del backend alla rete Docker `deploy_default` per raggiungere Postgres/Redis by hostname — richiede che `docker compose up -d` sia già stato eseguito in `deploy/`.
- Nessun commit git è stato fatto: il repo è stato inizializzato (`git init`) ma i file restano solo in working tree, in attesa che l'utente chieda esplicitamente di committare.

## Prossimi passi consigliati

Per finire M2:
1. Implementare `pickup` (serve una tabella `board_items` minima + colonna/tabella inventario condiviso per giocatore) e `build` (tabella `territory_claims` minima). `talk` può restare un semplice messaggio "event" con del testo hardcoded finché non esistono NPC veri.
2. Valutare se introdurre Redis ora o rimandare a quando le pedine per giocatore crescono davvero (per ora, con al più poche decine di unità totali, la scrittura diretta su Postgres per ogni azione è più che sufficiente).

Poi, in ordine, M3 (multi-board — l'astrazione `Grid` è già pronta, manca solo `board_links` e l'instradamento dei comandi tra goroutine di board diverse), M4 (motore regole reale, non più hardcoded) e M5 (obiettivi/punteggio/vittoria) come descritto sopra.
