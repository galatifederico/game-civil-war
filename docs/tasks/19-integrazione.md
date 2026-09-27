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
