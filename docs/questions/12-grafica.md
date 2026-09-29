# 12 — Client grafico

**Contesto.** Su tua indicazione la grafica si fa alla fine, con sprite semplicissimi.

**Decisione presa.** Client web in un solo file HTML servito dall'engine su `/ui/` (canvas 2D, nessuna build):
forme e colori presi dal mapping sprite dei dati (`sprites` in `90_mondo.ron`, modificabile live da
`PUT /api/sprites/{id}`), bordo del colore della fazione, lettera della razza, barra di progresso dell'azione,
nebbia di guerra della fazione del giocatore, feed del Piccione Viaggiatore e pannello della pedina selezionata.

**Aggiornamento 2026-09-29:** fatto anche un client Unity 6 nuovo in `unity-client/` che usa le stesse API.

**Alternative.** client in Bevy (Rust)
che riusa direttamente i tipi del motore; vere tavole di sprite (i campi `sheet`/`frame` esistono già).
