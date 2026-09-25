# The Game

Gioco multigiocatore online a griglia, mondo persistente. Concept e decisioni in
[docs/main.md](docs/main.md), [docs/design.md](docs/design.md), [docs/tecnico.md](docs/tecnico.md);
ordine di sviluppo e stato in [docs/roadmap.md](docs/roadmap.md).

Stato: le milestone M0-M7 di [docs/roadmap.md](docs/roadmap.md) sono fatte (il deploy su internet è predisposto ma non
provato). Registrazione e login, una lobby di mondi persistenti (ognuno con le sue regole, board quadrate
collegate da passaggi, razze, NPC, oggetti e obiettivi), squadre di 1 campione + 12 pedine che si
muovono, attaccano, parlano, raccolgono, costruiscono e si riproducono, nebbia di guerra, classifica, obiettivi
individuali e di mondo, gestione dei giocatori assenti. Il server è autoritativo (valida tutto e salva su
Postgres); il client Unity è "dumb" e disegna in pixel art vista dall'alto (stile Pokémon), anche in verticale su telefono. Ogni
mondo si modifica da un'app web di amministrazione.

## Avvio rapido

Servono Docker (per Postgres, Redis e per compilare il backend Go) e Unity 6.

```bash
cp deploy/.env.example deploy/.env    # solo la prima volta; cambia JWT_SECRET
make -C backend up db                 # Postgres + Redis, crea il database the_game
make -C backend run                   # costruisce l'immagine e avvia il backend in background su http://localhost:8090
```

Il backend gira come container `thegame-server` (immagine `thegame-backend:dev`, costruita da `backend/Dockerfile`).
`make -C backend run` aspetta che risponda e ti dice se qualcosa non va; poi:

```bash
make -C backend restart               # ferma, ricostruisce e riavvia (dopo aver cambiato il codice)
make -C backend stop                  # ferma il server e verifica che la porta sia libera
make -C backend logs                  # segue i log (Ctrl+C esce, il server resta acceso)
```

Poi in Unity Hub: **Add** -> cartella `unity-client/`, apri `Assets/Scenes/Main.unity` e premi Play.
Nella schermata di accesso usa "Crea un account", oppure "Accedi" se ne hai già uno; poi scegli un mondo
nella lobby ("Unisciti" la prima volta crea la tua squadra) o creane uno nuovo. Per provare
più giocatori insieme, apri più istanze dell'app (o una build) con account diversi.

**App admin** (regole, board, passaggi, razze, obiettivi, NPC, oggetti, giocatori): con il backend avviato apri
http://localhost:8090/admin/ e accedi con l'account admin del mondo (chi crea un mondo ne è l'admin; il primo
account registrato amministra il mondo di prova). Dalla lobby del client il pulsante "Editor" ci porta lì.

**Su internet** (Cloudflare Tunnel): [docs/deploy.md](docs/deploy.md).

Test del backend: `make -C backend test` (regole e loop) e `make -C backend test-integration`
(REST + WebSocket su Postgres reale, richiede `make -C backend up`).

## Struttura

- `backend/` — server Go autoritativo (REST + WebSocket + simulazione della board)
- `unity-client/` — client Unity 6, "dumb": mostra lo stato del server e invia comandi
- `tools/sprites/` — script che costruisce la tavola degli sprite dei personaggi dall'eroe standard (`art-src/hero/`)
- `admin-web/` — app web di amministrazione (HTML/JS statico, servita dal backend su `/admin/`)
- `deploy/` — docker-compose: Postgres, Redis e, a richiesta, backend e Cloudflare Tunnel
- `docs/` — concept, design, decisioni tecniche, roadmap, risorse grafiche utilizzabili
