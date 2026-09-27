# 04 — Statistiche, bisogni, inventario

Priorità: 4.

- `Stats`: valori base per id + modificatori (razza, classe, status, equipaggiamento) con min/max da def.
  Statistiche "speciali" (Baffi, Alopecia, Punti Onore, Reputazione Stampa…) sono solo dati.
- `Needs`: decadimento per tick, soddisfazione tramite effetti.
- `Inventory`: limite rigido di slot distinti (es. 3), stacking per categoria con massimo per stack,
  oggetti con tag (contrabbando, reliquia, collezionabile), uso degli oggetti (`on_use` = effetti).
- `Wallet` personale.
