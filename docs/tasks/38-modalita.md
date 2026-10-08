# 38 — Modalità (Lavora, Conquista, Ricerca, Svago)

**Fatto.**
- **Modalità** (`13_modalita.ron`): un modo di vivere alla volta. Ognuna ha i **pesi delle azioni per categoria**
  (l'utilità di un'azione si moltiplica per i pesi dei suoi tag; più tag: si moltiplicano; senza tag elencati vale
  `default_weight`), **modificatori di caratteristica** finché è attiva, **priorità di lavoro** della bacheca per tipo
  (1 prima … 4 ultima, 0 mai; i tipi non elencati valgono 3), e **quando la pedina la sceglie da sola** (condizione +
  priorità; altrimenti `bindings.default_mode`, Lavora).
  - Lavora (default): lavoro ×2,5, soldi ×2, sopravvivenza ×1,5, violenza ×0,3; lavori produttivi per primi.
  - Conquista (aggressività ≥ 70): violenza e territorio ×3, potere ×2, +15 aggressività; combattimento e
    sorveglianza per primi.
  - Ricerca (curiosità ≥ 75): esplorazione e informazioni ×3, studio ×2, +10 curiosità.
  - Svago (Svago < 20): svago ×3, sociale ×2.
- **Chi decide:** l'ordine del giocatore per la pedina vince su tutto, poi la modalità della fazione (dai dati,
  `FactionDef.mode`, o ordinata dal giocatore), poi la scelta della pedina. Razza, classe e ruolo non scelgono la
  modalità: contano solo tramite caratteristiche e bisogni. **Disobbedienza:** se la modalità imposta non è quella
  che la pedina sceglierebbe, il malcontento sale (`modes.dissent_per_tick`, 0,08 contro un calo di 0,05): l'unico
  modo di disobbedire è disertare (lasciare la fazione), come già succedeva col malcontento.
- **Priorità di lavoro:** tolte dalle classi, le dà la modalità; nella bacheca conta anche la bravura della pedina
  nel lavoro (la caratteristica `skill` del lavoro). Le priorità impostate a mano su una pedina restano e vincono.
- **Categorie delle azioni:** tag su tutte le 217 azioni (soldi, lavoro, sopravvivenza, riposo, violenza,
  territorio, potere, esplorazione, informazioni, studio, svago, sociale, fede, cura, crimine, fuga), ricavati dagli
  effetti, dai lavori e dai nomi, più 56 a mano.
- **Comando** `SetMode { entity | faction, mode }` (nessuna modalità = toglie l'ordine).
- **Console:** pagina Modalità (quando la sceglie, pesi per categoria, modificatori, priorità di lavoro, chi è in
  quella modalità); nella pedina la modalità attuale, da dove viene, quella che vorrebbe e un menu per ordinarla;
  nella fazione la modalità di default (e quella attuale); filtro e colonna «Categorie» nell'elenco delle azioni.

**Da rifinire.** Pesi, soglie e categorie sono una prima proposta; il client Unity non ha ancora il comando per
cambiare modalità.
