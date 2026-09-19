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
- **M2 — Tick loop, azioni a raggio** ✅ (Redis escluso, vedi sotto): attack / pickup / talk / build entro la "vista" della pedina; morte e respawn (solo cooldown); punti e classifica; inventario condiviso. Manca ancora lo stato caldo su Redis.
- **M3 — Multi-board e griglie miste**: board contigue di un'unica mappa, esagonale di prova, astrazione `Grid`.
- **M4 — Motore regole**: razze, compatibilità, regole di creazione pedine, riproduzione, inventario condiviso (solo campione), "vista" = visibilità + raggio.
- **M5 — Obiettivi, punteggio, condizioni di vittoria.**
- **M6 — Admin web app** (`admin-web/`) e lobby mondi lato giocatore.
- **M7 — Hardening**: riconnessione/afk, grafica 2D isometrica pixel art vera, Cloudflare Tunnel.

## Stato attuale

**M0, M1 e M2 completati e verificati end-to-end** (client Unity vero contro backend e Postgres veri, più una sonda Go temporanea con due giocatori):

- Backend (`backend/`): `internal/game` (regole pure e testate: `Board.Move`, `Board.Do` per le azioni, `PlanTeam`, `Tick`; `Loop` = una goroutine possiede lo stato della board a un tick di 250 ms, gli altri parlano con lei via canale; le scritture su Postgres passano da una coda ordinata), `internal/store` (pgx, migrazioni embedded applicate all'avvio), `internal/transport` (REST `/auth/register`, `/auth/login`, `/healthz`; WebSocket `/ws` con `auth` come primo messaggio). Protocollo JSON in `internal/protocol`: client→server `auth`, `move`, `attack`, `talk`, `pickup`, `build`; server→client `snapshot`, `delta`, `event`, `inventory`, `error`.
- **Movimento**: solo le proprie pedine, casella libera e dentro la board, distanza (Chebyshev) ≤ velocità, cooldown dopo la mossa = distanza / velocità secondi. Campione velocità 3, pedine 2.
- **Azioni** (raggio = "vista": campione 5, pedina 3; cooldown d'azione condiviso: attacco 1,5 s, raccolta 0,5 s, costruzione 3 s, parlare nessuno): *attack* solo contro pedine di altre squadre (NPC non attaccabili, come da design), danno = forza (campione 30, pedina 15); *talk* con un NPC restituisce una battuta a caso (dialoghi nella colonna `units.dialogue`); *pickup* toglie l'oggetto dalla board e lo mette nell'inventario condiviso di squadra, qualunque sia la distanza dal campione; *build* mette un avamposto su una casella libera, che diventa territorio della squadra.
- **Morte**: a vita 0 la pedina resta sulla casella fuori gioco e rinasce dopo 10 s con vita piena, senza altre penalità. Al riavvio del server le pedine già sconfitte rinascono dopo 10 s.
- **Punti** (design.md: ogni azione può cambiare i punti; eliminare il campione dà un grosso bonus, non la vittoria): colpo +5, sconfitta pedina +25, sconfitta campione +100, raccolta +5, costruzione +20. Classifica di tutti i giocatori sempre visibile.
- **Persistenza**: posizioni, vita, oggetti raccolti, inventario, strutture e punti su Postgres. Il cooldown è stato effimero.
- **Client Unity** (`unity-client/`): `GameController` crea tutto a runtime (nessun setup nella scena); `NetworkClient` (REST via UnityWebRequest + `ClientWebSocket`), `BoardManager` (snapshot/delta, click, azioni, territorio colorato), `Piece`, `InfoPanel` (scheda + pulsanti azione con motivo del blocco), `LoginScreen`, `Hud` (classifica, inventario, notifiche/dialoghi), tutti in IMGUI senza dipendenze da UGUI. Rendering con primitive 3D viste dall'alto: è un placeholder, la grafica vera arriva in M7.
- **Come si gioca**: clic su una tua pedina la seleziona (poi una casella vuota la sposta). Con una pedina selezionata, clic su un NPC / oggetto / pedina nemica apre la sua scheda con "Parla" / "Raccogli" / "Attacca". Clic sulla pedina selezionata (scheda con "Costruisci avamposto") e poi su una casella libera costruisce.

**Non ancora fatto / da sapere:**

- Nessun test automatico per store e transport (solo per le regole in `internal/game`). La verifica end-to-end è stata manuale.
- Il click reale del mouse non è stato provato dall'automazione: si è simulato `OnMouseDown` e l'esecuzione dei pulsanti. I pannelli ignorano i click che cadono su di sé (`BlocksPointer`).
- Ogni giocatore vede tutta la board (niente fog of war: arriva con la "vista" in M4). Le squadre nascono attorno a un'ancora per slot (`Board.PlanTeam`), 3 per riga.
- Il territorio è solo visivo (casella colorata): il perimetro, il suo valore in punti e la conquista contesa non sono definiti. Gli oggetti non si possono ancora spostare né usare; l'inventario si vede ma non si gestisce (solo il campione potrà, come da design).
- Redis è avviato ma il backend non lo usa ancora. La grafica isometrica pixel art e il layout mobile (verticale) non sono iniziati.
- Se il server viene fermato mentre un cooldown è in corso, il cooldown si azzera al riavvio (è effimero).

## Prossimi passi consigliati

1. Provare il gioco con più giocatori reali (due istanze del client) per verificare broadcast, formazione delle squadre e bilanciamento di danno/punti.
2. Test di integrazione per REST/WebSocket (DB temporaneo) prima che il protocollo cresca ancora.
3. Redis per lo stato caldo (posizioni/cooldown) quando le pedine per giocatore crescono davvero.
4. M3 (multi-board, griglie miste) oppure M4 (razze, riproduzione, fog of war con la "vista"): scegliere in base a cosa serve di più giocando.
