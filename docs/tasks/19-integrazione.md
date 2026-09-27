# 19 — Client grafico (futuro)

Priorità: 19 (non iniziata).

Il vecchio client Unity, l'admin web e il backend Go sono stati rimossi (sono nella storia git fino al
commit `3077ea5`). Un futuro client legge le API UI dell'engine (`/api/ui/*`: ActivityState, mappa, sprite
mapping) e invia `SimCommand` tramite `POST /api/commands`. Vedi [domanda 01](../questions/01-repo-rust-vs-go.md).
