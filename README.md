# The Game

Riavvio da zero come MVP: una scacchiera con alcune pedine, client in Unity. Clicca una pedina
per selezionarla, poi clicca una casella vuota per spostarla li'. Nessuna regola di movimento,
nessun backend: tutto e' locale nel client.

La visione a lungo termine (MMO realtime, mondo persistente) resta documentata in
[docs/main.md](docs/main.md), [docs/design.md](docs/design.md), [docs/tecnico.md](docs/tecnico.md)
e [docs/roadmap.md](docs/roadmap.md), ma il codice attuale riparte da questo MVP minimale — quei
documenti descrivono uno stato futuro, non l'implementazione presente.

## Avvio rapido

1. Apri Unity Hub, "Add" -> seleziona la cartella `unity-client/` (Unity 2022.3 LTS o successivo;
   se non hai quella patch esatta, Unity Hub ti propone comunque di aprire il progetto con la
   versione installata).
2. Apri la scena `Assets/Scenes/Main.unity`.
3. Premi Play: scacchiera 8x8 e pedine (rosse/blu) vengono generate a runtime dallo script
   `BoardManager` — non serve altro setup nella scena.

## Struttura

- `unity-client/` — client Unity (scacchiera, pedine, selezione/spostamento click-based)
- `backend/` — vuoto per ora: nessun backend necessario per l'MVP locale; da reintrodurre in Go
  quando servira' persistenza o multiplayer
- `docs/` — visione di lungo periodo e note di design/roadmap (non ancora implementate)
