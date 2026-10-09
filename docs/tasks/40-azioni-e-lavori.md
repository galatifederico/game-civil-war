# 40 — Azioni e lavori uniti

## Fatto: nella console
- Tolta la voce «Lavori ed effetti» dal menu. La scheda di un'azione che fa un lavoro ha la sezione **«Come si
  fa»** con i campi del lavoro (durata, tipo di lavoro, caratteristica che velocizza, gittata, rango minimo,
  notiziabilità, sospetto, requisiti, effetti, reato, tag richiesti): si modificano lì e si salvano sul lavoro (che
  vale per tutte le azioni che lo usano; la scheda dice quali).
- L'elenco delle **Azioni** mostra anche i 13 lavori che nessuna azione usa, cioè i **lavori della bacheca**
  (consegna, trasporta, pulisci, scava roccia, sabotaggio, ruba reliquia…), con la categoria «bacheca».
- Le schede dei lavori restano raggiungibili dai link; «Torna all'elenco» porta alle Azioni.

## Da fare: nel motore (piano)
Oggi un'azione dice *perché/quando* (peso, considerazioni, bersaglio, requisiti, attesa, categorie) e il lavoro
dice *come* (durata, tipo di lavoro, effetti, reato, gestore). 126 lavori su 139 sono usati da un'azione, quasi
sempre uno a uno; 5 sono condivisi; 13 sono solo della bacheca.

Proposta da decidere insieme:
1. L'azione porta direttamente i campi del lavoro (durata, tipo di lavoro, caratteristica che velocizza, effetti,
   reato, gestore): `ActionKind::Job { job }` diventa un'azione che «si fa» da sé.
2. I lavori della bacheca diventano azioni senza peso (l'IA non le sceglie da sola) che fazioni, edifici,
   designazioni e ordini pubblicano; «Lavora» prende dalla bacheca come oggi.
3. I 5 lavori condivisi diventano azioni separate (oppure un'azione con più bersagli).
4. Migrazione: i ~140 riferimenti nei dati (ricette degli edifici, obiettivi delle fazioni, eventi, designazioni,
   PostJob) e i 45 gestori nel codice puntano all'azione invece che al lavoro.
