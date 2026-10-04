# 34 — Modifica dei contenuti dalla console

**Fatto.**
- **Motore:** `PUT /api/admin/content/{tipo}/{id}` (comando `EditContent`) sostituisce una definizione mentre il gioco
  gira: viene validata come a un caricamento da zero (riferimenti, condizioni, effetti), applicata subito e salvata in
  `fidenza_world/data/99_console.json`, che si carica per ultimo e quindi vince sui `.ron` (che restano intatti). Si
  modificano caratteristiche, bisogni, corpi, razze, classi, stati, fluidi, oggetti, abilità, lavori, azioni, fazioni,
  edifici, personaggi, generatori, eventi a condizione, collezioni, ruoli, fonti di notizie, circostanze e rifornimenti.
- **Razze:** `stat_ranges` (minimo, massimo e valore iniziale della razza per ogni caratteristica: l'iniziale sostituisce
  il default alla nascita, minimo e massimo restringono quelli della caratteristica), `sexes` (sessi possibili; vuoto =
  senza sesso, non si riproduce; prende il posto di `sexless`), `need_rates` (bisogni consumati più o meno in fretta).
  Tolto `shapeshifter`: mutaforma è l'abilità `mutaforma`.
- **Scheda di una razza:** Caratteristiche (tabella di tutte con minimo, massimo, iniziale e bonus; vuoto = valore
  generale, mostrato in grigio; ricerca e «solo personalizzate»), Abilità e Immunità (tabelle con caratteristica e
  modificatore), Bisogni (quelli che la razza ha, con la variazione e il calo effettivo), Corpo (scelta del piano e sue
  parti), Sessi. Poi *Informazioni di gioco* (chi è in gioco, chi la usa) e *Dettaglio* (nome, descrizione, tag, stati
  innati, JSON completo).
- **Tutte le altre schede** con lo stesso schema, ricavato dai campi: Valori (numeri, sì/no, scelte da elenco),
  una tabella per ogni elenco di riferimenti (abilità, azioni, classi, stati… con quello che fanno) e per ogni mappa
  (bonus alle caratteristiche, oggetti e quantità, rapporti…), poi Informazioni di gioco e Dettaglio con condizioni ed
  effetti modificabili in JSON accanto alla loro descrizione a parole. Per aggiungere si sceglie tra i valori esistenti
  (casella con suggerimenti); i valori non validi sono rifiutati con il motivo.

**Non fatto.** Creare o cancellare voci (solo modificare); mappa, piazzamenti e collegamenti (`bindings`) non si
modificano dalla console; le modifiche non riscrivono i `.ron` (per renderle definitive vanno riportate a mano o si
committa `99_console.json`).
