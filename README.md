# The Game

Gioco multigiocatore online a griglia, mondo persistente. Concept e decisioni in
[docs/main.md](docs/main.md), [docs/design.md](docs/design.md), [docs/tecnico.md](docs/tecnico.md);
ordine di sviluppo e stato in [docs/roadmap.md](docs/roadmap.md).

Oggi funzionano le milestone M1 e M2: registrazione/login, una board 24x24 condivisa con NPC e
oggetti, ogni giocatore ha la sua squadra (1 Champion + 12 pedine). Le pedine si muovono,
attaccano i nemici (chi va a zero vita rinasce dopo 10 secondi), parlano con gli NPC, raccolgono
oggetti nell'inventario di squadra e costruiscono avamposti per conquistare caselle; ogni azione
dà punti e c'è una classifica. Il server valida tutto (proprietà, portata, cooldown, caselle
occupate) e salva su Postgres. Cliccando qualsiasi pedina si apre un menu con la descrizione e le
azioni possibili.

## Avvio rapido

Servono Docker (per Postgres, Redis e per compilare il backend Go) e Unity 6.

```bash
cp deploy/.env.example deploy/.env    # solo la prima volta; cambia JWT_SECRET
make -C backend up db                 # Postgres + Redis, crea il database the_game
make -C backend run                   # backend su http://localhost:8090 (Ctrl+C per fermarlo)
```

Poi in Unity Hub: **Add** -> cartella `unity-client/`, apri `Assets/Scenes/Main.unity` e premi Play.
Nella schermata di accesso usa "Crea un account", oppure "Accedi" se ne hai già uno. Per provare
più giocatori insieme, apri più istanze dell'app (o una build) con account diversi.

Test del backend: `make -C backend test`.

## Struttura

- `backend/` — server Go autoritativo (REST + WebSocket + simulazione della board)
- `unity-client/` — client Unity 6, "dumb": mostra lo stato del server e invia comandi
- `deploy/` — docker-compose per Postgres e Redis
- `docs/` — concept, design, decisioni tecniche, roadmap
