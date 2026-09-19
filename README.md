# The Game

Riavvio da zero come MVP, client in Unity: una scacchiera 10x10 con la tua squadra (1 Champion +
12 pedine), 5 NPC e 3 oggetti. Le pedine della tua squadra si muovono: clicca una pedina, poi una
casella vuota. NPC e oggetti non si muovono. Cliccando qualsiasi pedina si apre un menu con nome,
tipo e descrizione. Nessuna regola di movimento, nessun backend: tutto e' locale nel client.

La visione a lungo termine (MMO realtime, mondo persistente) resta documentata in
[docs/main.md](docs/main.md), [docs/design.md](docs/design.md), [docs/tecnico.md](docs/tecnico.md)
e [docs/roadmap.md](docs/roadmap.md), ma il codice attuale riparte da questo MVP minimale — quei
documenti descrivono uno stato futuro, non l'implementazione presente.

## Avvio rapido

1. Apri Unity Hub, "Add" -> seleziona la cartella `unity-client/` (Unity 2022.3 LTS o successivo;
   se non hai quella patch esatta, Unity Hub ti propone comunque di aprire il progetto con la
   versione installata).
2. Apri la scena `Assets/Scenes/Main.unity`.
3. Premi Play: scacchiera e pedine vengono generate a runtime dallo script `BoardManager` — non
   serve altro setup nella scena. Legenda: Champion = cilindro oro, pedine tue = capsule blu,
   NPC = sfere viola, oggetti = cubi verdi.

## Struttura

- `unity-client/` — client Unity (scacchiera, squadra/NPC/oggetti, spostamento click-based, menu info)
- `backend/` — vuoto per ora: nessun backend necessario per l'MVP locale; da reintrodurre in Go
  quando servira' persistenza o multiplayer
- `docs/` — visione di lungo periodo e note di design/roadmap (non ancora implementate)
