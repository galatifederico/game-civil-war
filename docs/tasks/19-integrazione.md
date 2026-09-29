# 19 — Client grafico (ultima task)

Priorità: 19 (si fa alla fine, su tua indicazione).

Il vecchio client Unity, l'admin web e il backend Go sono stati rimossi (sono nella storia git fino al
commit `3077ea5`). Il nuovo client:
- è una pagina web servita dall'engine stesso (`/ui/`), niente toolchain aggiuntive;
- legge le API UI dell'engine (`/api/ui/*`: ActivityState, mappa, sprite mapping) e invia `SimCommand`
  tramite `POST /api/commands`;
- **sprite semplicissimi per iniziare**: forme e colori generati dal mapping sprite dei dati (cerchio
  colorato per razza, bordo per fazione, lettera/icona per classe, quadrati per edifici), sostituibili in
  seguito con vere tavole di sprite senza cambiare l'engine.

## Aggiornamento 2026-09-29: client Unity

Su tua richiesta c'è anche un client **Unity 6** in `unity-client/` (il progetto vecchio non c'è più: questo è
nuovo e parla solo con le API dell'engine Rust). Script in `Assets/Scripts/`:
- `SimApi` (HTTP + JSON Newtonsoft), `SimView` (mappa, zone, edifici, pedine, sporco, nebbia, camera, selezione),
  `SimHud` (barra comandi, scheda pedina con Utility AI e tangente, feed, cronaca, mercato, fazioni),
  `Shapes` (sprite procedurali semplicissimi), `Bootstrap` (crea il client al Play in qualunque scena).
- Menu `Fidenza > Crea scena Main` e `Fidenza > Build Linux`; da riga di comando il player accetta
  `--server=http://host:porta` e `--screenshot=file.png --shot-after=8` (per le verifiche automatiche).
