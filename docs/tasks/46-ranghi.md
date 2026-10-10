# 46 — Ranghi delle fazioni come classi e ruoli

**Deciso.** I ranghi non sono stati fusi nei ruoli: un ruolo ha un solo titolare (il sindaco, il commissario), un
rango è condiviso da molti membri (tutte le reclute). Hanno però ricevuto gli stessi strumenti.

**Fatto.**
- Un rango ha ora **requisiti per salire** (`requires`), **modificatori di caratteristica**, **tag**, **abilità** e
  **azioni**, come classi e ruoli. Ogni `factions.promotion_every` ore (24) i membri salgono al rango più alto di cui
  hanno i requisiti (mai a un rango unico; senza requisiti = solo per ordine o dal personaggio). La promozione va in
  cronaca.
- Esempi: Prete (è Prete, +5 santità), Sacerdote Oscuro (Satanista o Negromante), Templare (è Templare, +5 onore),
  Inquisitore (è Polizia Vegana), Coppiere (è Ubriacone).
- **Console:** nella scheda della fazione la tabella dei ranghi (livello, stipendio, per salire, poteri), modificabile
  nel JSON.
