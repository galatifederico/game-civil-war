# 26 — Conquista dei quartieri

**Fatto** (`territory.rs`, parametri `territory.*`).
- I quartieri sono le zone con tag `quartiere` (tutti quelli della superficie e i dungeon).
- Ogni 6 tick ogni fazione guadagna influenza da membri presenti, edifici posseduti (secondo la salute) e
  stendardi (al massimo 2 contano); l'influenza svanisce del 10% a ogni aggiornamento.
- Si conquista un quartiere superando la soglia (20) e battendo il padrone attuale del 25%.
- Il padrone riceve ogni giorno una rendita (15) e un punto vittoria, e vede tutto il quartiere.
- Il giocatore conquista stando nei quartieri con i suoi, possedendo edifici e piantando stendardi
  ("Pianta lo stendardo", costa 30); anche i capi delle fazioni AI ogni tanto ne piantano.
- Client: quartieri colorati in panoramica, padrone accanto al nome del luogo, elenco nella scheda della fazione.
