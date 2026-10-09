# 40 — Azioni e lavori uniti

## Fatto: nella console
- Tolta la voce «Lavori ed effetti» dal menu. La scheda di un'azione che fa un lavoro ha la sezione **«Come si
  fa»** con i campi del lavoro (durata, tipo di lavoro, caratteristica che velocizza, gittata, rango minimo,
  notiziabilità, sospetto, requisiti, effetti, reato, tag richiesti): si modificano lì e si salvano sul lavoro (che
  vale per tutte le azioni che lo usano; la scheda dice quali).
- I 13 **lavori della bacheca** (consegna, trasporta, pulisci, scava roccia, sabotaggio, ruba reliquia…) sono azioni
  con la categoria «bacheca»: si trovano nell'elenco delle Azioni con il filtro Categoria.
- Le schede dei lavori restano raggiungibili dai link; «Torna all'elenco» porta alle Azioni.

## Fatto: nel motore
- **Un'azione dice anche come si fa:** il campo `how` dell'azione (durata, tipo di lavoro, caratteristica che
  velocizza, gittata, requisiti, effetti, reato, gestore, parametri…). Al caricamento il motore ne ricava il lavoro con
  lo stesso id dell'azione, quindi bacheca, gestori nel codice, ricette, obiettivi delle fazioni e ordini funzionano
  come prima. `kind: Job(...)` senza `job` (o con l'id dell'azione) fa il lavoro dell'azione stessa; un'azione può
  anche usare il lavoro di un'altra (es. «Reclama il trono» usa «Si muove»).
- **Dati migrati:** i 139 lavori sono entrati nelle loro azioni (i 5 condivisi copiati in ciascuna); i 13 lavori della
  bacheca sono azioni con peso 0 e categoria «bacheca»; il movimento generico è l'azione «Si muove» (`muoviti`, peso 0,
  categoria «ordine»). 39 lavori hanno preso il nome della loro azione (es. «hackeraggio» → «hackera_macchina»);
  nessun riferimento esterno puntava a quei nomi. Non c'è più nessun elenco `jobs` nei dati di Fidenza.
- I pacchetti che scrivono i lavori a parte (`jobs: [...]`, come quello di prova dei test) funzionano ancora.
- **Console:** «Come si fa» modifica il `how` dell'azione; se l'azione usa il lavoro di un'altra, lo dice e salva su
  quella; la scheda di un lavoro che appartiene a un'azione porta all'azione.
