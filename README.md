# The Game — engine di simulazione

Engine di simulazione headless, deterministico e data-driven in Rust (ECS con `bevy_ecs`), stile colony sim
(Dwarf Fortress, RimWorld), più il plugin dell'ambientazione satirica di Fidenza e Salsomaggiore.

- Design: [docs/design.md](docs/design.md)
- Piano e stato delle task: [docs/tasks/00-piano.md](docs/tasks/00-piano.md)
- Decisioni prese in autonomia da rivedere: [docs/questions/](docs/questions/)

Il codice è in [engine/](engine/) (vedi il suo README per comandi e struttura). Il client Unity 6 è in [unity-client/](unity-client/): avvia il server con `cd engine && cargo run --release -p fidenza_world -- --serve`, poi apri il progetto in Unity Hub e premi Play.
