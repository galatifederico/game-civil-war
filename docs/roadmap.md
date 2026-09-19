# Roadmap (note di lavoro)

Ordine di sviluppo e stato reale. Le decisioni di gioco sono in [design.md](design.md), quelle tecniche in [tecnico.md](tecnico.md). Se riprendi il progetto in una nuova sessione, parti da "Stato attuale" e "Note operative".

## Struttura repo

```
the_game/
├── docs/          # concept, design, tecnico, questa roadmap
├── backend/       # server Go autoritativo (REST + WebSocket + simulazione board)
├── unity-client/  # client Unity 6 (client "dumb": mostra lo stato del server, invia comandi)
└── deploy/        # docker-compose (Postgres + Redis)
```

## Milestone

- **M0 — Scaffold**: backend Go che parte, migrazioni, `/healthz`, docker-compose.
- **M1 — Slice verticale** ✅: register/login (bcrypt + JWT), una board seedata con NPC e oggetti, alla registrazione ogni giocatore riceve la sua squadra (1 campione + 12 pedine), movimento autoritativo via WebSocket con limite di distanza e cooldown (velocità), posizioni persistite su Postgres. Client Unity: login, snapshot, click per muovere le proprie pedine, menu info di ogni pedina.
- **M2 — Tick loop, Redis, azioni a raggio**: stato caldo su Redis; attack / pickup / talk / build; morte e respawn (solo cooldown).
- **M3 — Multi-board e griglie miste**: board contigue di un'unica mappa, esagonale di prova, astrazione `Grid`.
- **M4 — Motore regole**: razze, compatibilità, regole di creazione pedine, riproduzione, inventario condiviso (solo campione), "vista" = visibilità + raggio.
- **M5 — Obiettivi, punteggio, condizioni di vittoria.**
- **M6 — Admin web app** (`admin-web/`) e lobby mondi lato giocatore.
- **M7 — Hardening**: riconnessione/afk, grafica 2D isometrica pixel art vera, Cloudflare Tunnel.

## Stato attuale

**M0 e M1 completati e verificati end-to-end** (client Unity vero contro backend e Postgres veri):

- Backend (`backend/`): `internal/game` (regole pure e testate: `Board.Move`, `PlanTeam`; `Loop` = una goroutine possiede lo stato della board, gli altri parlano con lei via canale), `internal/store` (pgx, migrazioni embedded applicate all'avvio), `internal/transport` (REST `/auth/register`, `/auth/login`, `/healthz`; WebSocket `/ws` con `auth` come primo messaggio). Protocollo JSON in `internal/protocol`: client→server `auth`, `move`; server→client `snapshot`, `delta`, `error`.
- Regole di movimento: solo le proprie pedine, casella libera e dentro la board, distanza (Chebyshev) ≤ velocità, cooldown dopo la mossa = distanza / velocità secondi. Campione velocità 3, pedine 2. NPC e oggetti non si muovono.
- Persistenza: posizioni salvate su Postgres in modo asincrono; il cooldown è stato effimero (non persistito). Al riavvio del server le pedine ritornano dove erano.
- Client Unity (`unity-client/`): `GameController` crea tutto a runtime (nessun setup nella scena); `NetworkClient` (REST via UnityWebRequest + `ClientWebSocket`), `BoardManager` (snapshot/delta, click), `Piece`, `InfoPanel`, `LoginScreen`, `Hud` (IMGUI, nessuna dipendenza da UGUI). Rendering con primitive 3D viste dall'alto: è un placeholder, la grafica vera arriva in M7.

**Non ancora fatto / da sapere:**

- Nessun test automatico per store e transport (solo per le regole in `internal/game`). La verifica end-to-end è stata manuale (curl, una sonda Go temporanea, Unity via MCP).
- Il click reale del mouse non è stato provato dall'automazione: si è simulato `OnMouseDown`. Il pannello ignora i click che cadono su di sé (`InfoPanel.BlocksPointer`).
- Ogni giocatore vede tutta la board (niente fog of war: arriva con la "vista" in M4). Le squadre nascono attorno a un'ancora per slot (`Board.PlanTeam`), 3 per riga.
- Redis è avviato ma il backend non lo usa ancora (M2). La grafica isometrica pixel art e il layout mobile (verticale) non sono iniziati.

## Prossimi passi consigliati

1. M2: tick loop, azioni a raggio (attack, pickup, talk, build), morte/respawn con solo cooldown. `talk` con gli NPC riutilizza il campo `description` come primo testo.
2. Test di integrazione per REST/WebSocket (DB temporaneo) prima che il protocollo cresca.
3. Provare il gioco con più giocatori reali (due istanze del client) per verificare broadcast e formazione delle squadre.

## Note operative

- **Go non è installato sull'host**: si compila e si testa con Docker (`golang:1.23-alpine`, vedi `backend/Makefile`).
- **Porte**: Postgres 5433 (la 5432 è di un altro progetto), Redis 6379, backend **8090** (la 8080 è il bridge MCP di Unity).
- **Database**: `the_game` nel container `deploy-postgres-1`. Il vecchio DB `thegame` (schema di una versione precedente) non è toccato.
- **Unity**: Editor 6000.3.24f1, pacchetto MCP for Unity installato. Con il bridge attivo (Window > MCP for Unity) posso aprire la scena, lanciare Play, leggere la console e fare screenshot.
- **Test Unity**: gli eventi mouse reali non si possono inviare via MCP; si simulano invocando `OnMouseDown` sugli oggetti.
- La cronologia git contiene la vecchia implementazione (client Godot + backend Go, commit `6bd11cd`): utile come riferimento, non da ripristinare.
