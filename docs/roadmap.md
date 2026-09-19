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
- **M2 — Tick loop, azioni a raggio** ✅ (Redis escluso, vedi sotto): attack / pickup / talk / build / create entro la "vista" della pedina; morte e respawn (solo cooldown); punti e classifica; inventario condiviso; nebbia di guerra (anticipata da M4). Manca ancora lo stato caldo su Redis.
- **M3 — Multi-board e griglie miste**: board contigue di un'unica mappa, esagonale di prova, astrazione `Grid`.
- **M4 — Motore regole**: ✅ fondamenta fatte (parametri del mondo come dati, vedi "Stato attuale"); da fare razze, compatibilità, altre regole di creazione pedine, riproduzione, inventario condiviso (solo campione).
- **M5 — Obiettivi, punteggio, condizioni di vittoria.**
- **M6 — Admin web app** (`admin-web/`) e lobby mondi lato giocatore.
- **M7 — Hardening**: riconnessione/afk, grafica 2D isometrica pixel art vera, Cloudflare Tunnel.

## Stato attuale

**M0, M1 e M2 completati e verificati end-to-end** (client Unity vero contro backend e Postgres veri; test Go di regole, loop e integrazione con due giocatori):

- Backend (`backend/`): `internal/game` (regole pure e testate: `Board.Move`, `Board.Do` per le azioni, `PlanTeam`, `Tick`; `Loop` = una goroutine possiede lo stato della board a un tick di 250 ms, gli altri parlano con lei via canale; le scritture su Postgres passano da una coda ordinata), `internal/store` (pgx, migrazioni embedded applicate all'avvio), `internal/transport` (REST `/auth/register`, `/auth/login`, `/healthz`; WebSocket `/ws` con `auth` come primo messaggio). Protocollo JSON in `internal/protocol`: client→server `auth`, `move`, `attack`, `talk`, `pickup`, `build`; server→client `snapshot`, `delta`, `event`, `inventory`, `error`.
- **Movimento**: solo le proprie pedine, casella libera e dentro la board, distanza (Chebyshev) ≤ velocità, cooldown dopo la mossa = distanza / velocità secondi. Campione velocità 3, pedine 2.
- **Azioni** (raggio = "vista": campione 5, pedina 3; cooldown d'azione condiviso: attacco 1,5 s, raccolta 0,5 s, costruzione 3 s, creazione 5 s, parlare nessuno): *attack* solo contro pedine di altre squadre (NPC non attaccabili, come da design), danno = forza (campione 30, pedina 15); *talk* con un NPC restituisce una battuta a caso (dialoghi nella colonna `units.dialogue`); *pickup* toglie l'oggetto dalla board e lo mette nell'inventario condiviso di squadra, qualunque sia la distanza dal campione; *build* mette un avamposto su una casella libera, che diventa territorio della squadra; *move_item* sposta un oggetto su un'altra casella libera (oggetto e destinazione entro la vista, cooldown 0,5 s); *create* (solo il campione) paga 20 punti vita e fa comparire una nuova pedina su una casella libera accanto al campione (seconda via di creazione del design; nessuna rigenerazione di vita per ora, quindi è una risorsa finita: da bilanciare in M4).
- **Morte**: a vita 0 la pedina resta sulla casella fuori gioco e rinasce dopo 10 s con vita piena, senza altre penalità. Al riavvio del server le pedine già sconfitte rinascono dopo 10 s.
- **Punti** (design.md: ogni azione può cambiare i punti; eliminare il campione dà un grosso bonus, non la vittoria): colpo +5, sconfitta pedina +25, sconfitta campione +100, raccolta +5, costruzione +20. Classifica di tutti i giocatori sempre visibile.
- **Nebbia di guerra** (design.md: "vista" = visibilità + raggio d'azione): il server manda a ogni giocatore solo le entità entro la vista di una sua pedina ancora in gioco (più tutto ciò che è suo). `Board.VisibleTo` calcola l'insieme, `Loop.sync` invia a ogni giocatore connesso le entità entrate in vista (complete), gli aggiornamenti di quelle già in vista e le uscite come `removed`. La classifica è pubblica. Non c'è memoria delle zone esplorate: una cosa fuori vista sparisce. Il client scurisce le caselle fuori vista.
- **Partenza delle squadre**: `spawnAnchor` le distribuisce (prime sei ai lati, poi al centro) così che all'inizio siano fuori vista l'una dall'altra (test dedicato su 24x24).
- **Regole come dati** (tecnico.md: motore di regole configurabile per mondo): tutti i parametri di gioco (statistiche di campione e pedine, dimensione della squadra iniziale, respawn, cooldown, costo di creazione, punti per azione) stanno in `game.Rules`, con i valori predefiniti in `DefaultRules()`. Ogni mondo li sovrascrive nella colonna `worlds.rules` (JSON con solo le differenze, es. `{"minors_per_team": 8, "champion": {"speed": 4}}`). `ParseRules` rifiuta campi sconosciuti (refusi) e valori che romperebbero la simulazione, e il server non parte con regole sbagliate. Il client Unity non ha valori hardcoded: legge tutto dalle entità. È la base su cui l'editor admin (M6) scriverà.
- **Persistenza**: posizioni, vita, oggetti raccolti, inventario, strutture, nuove pedine e punti su Postgres. Il cooldown è stato effimero.
- **Client Unity** (`unity-client/`): `GameController` crea tutto a runtime (nessun setup nella scena); `NetworkClient` (REST via UnityWebRequest + `ClientWebSocket`), `BoardManager` (snapshot/delta, click, azioni, territorio colorato), `Piece`, `InfoPanel` (scheda + pulsanti azione con motivo del blocco), `LoginScreen`, `Hud` (classifica, inventario, notifiche/dialoghi), tutti in IMGUI senza dipendenze da UGUI. Rendering con primitive 3D viste dall'alto: è un placeholder, la grafica vera arriva in M7.
- **Come si gioca**: clic su una tua pedina la seleziona (poi una casella vuota la sposta). Con una pedina selezionata, clic su un NPC / oggetto / pedina nemica apre la sua scheda con "Parla" / "Raccogli" e "Sposta" / "Attacca" ("Sposta" e "Costruisci" chiedono poi di cliccare la casella di destinazione). Clic sulla pedina selezionata (scheda con "Costruisci avamposto") e poi su una casella libera costruisce.

**Non ancora fatto / da sapere:**

- Test: `make -C backend test` (regole e loop, senza DB) e `make -C backend test-integration` (REST + WebSocket su Postgres reale, database temporaneo per ogni esecuzione). Il client Unity non ha test automatici: si verifica a mano o via MCP.
- Il click reale del mouse non è stato provato dall'automazione: si è simulato `OnMouseDown` e l'esecuzione dei pulsanti. I pannelli ignorano i click che cadono su di sé (`BlocksPointer`).
- `Board.VisibleTo` è O(entità × pedine): va bene per pochi giocatori, da ottimizzare (indice spaziale) se le pedine crescono molto. La board di 24x24 regge bene 6 squadre fuori vista; per 6-15 giocatori serve una board più grande (parametro di mondo, M6).
- Il territorio è solo visivo (casella colorata): il perimetro, il suo valore in punti e la conquista contesa non sono definiti. Gli oggetti non si possono ancora spostare né usare; l'inventario si vede ma non si gestisce (solo il campione potrà, come da design).
- Redis è avviato ma il backend non lo usa ancora. La grafica isometrica pixel art e il layout mobile (verticale) non sono iniziati.
- Se il server viene fermato mentre un cooldown è in corso, il cooldown si azzera al riavvio (è effimero).

## Prossimi passi consigliati

1. Provare il gioco con più giocatori reali (due istanze del client) per verificare bilanciamento di danno, punti e costo di creazione, e come si sente la nebbia.
2. Altre regole di creazione (risorse raccolte, edifici, riproduzione tra pedine) e razze definite dall'admin: estendere `Rules` (o una tabella `races`) invece di aggiungere costanti. Le regole oggi sono per mondo e uguali per tutte le pedine dello stesso tipo.
3. Uso degli oggetti dell'inventario da parte del campione (gli oggetti oggi si possono raccogliere e spostare, non usare).
4. Redis per lo stato caldo quando le pedine per giocatore crescono davvero.
5. M3 (multi-board, griglie miste) e il resto di M4 (razze, riproduzione), poi grafica e layout mobile.
