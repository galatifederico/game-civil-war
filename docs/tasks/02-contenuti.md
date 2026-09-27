# 02 — Contenuti data-driven

Priorità: 2.

## Obiettivo
Definizioni serde di tutti i tipi di contenuto, caricamento da più file RON (un pacchetto = una cartella),
unione, validazione dei riferimenti incrociati, `ContentRegistry` come risorsa ECS.

## Contenuto
- Def: statistiche, bisogni, piani corporei, razze, classi, status (con stadi, contagio, escalation),
  oggetti, job, azioni di Utility AI, fazioni (ranghi, ideologia, relazioni), edifici (ricette, negozio,
  conseguenze dei danni), mappa (livelli, zone, portali, reti), template di entità (anche NPC unici),
  spawner, trigger, collezioni, titoli (successione), fonti di notizie, condizioni di vittoria, sprite.
- Linguaggi dati: `Effect`, `Condition`, `Curve`, `Target`.
- Validazione: ogni id referenziato esiste; errori chiari con il percorso del file.

## Criteri di accettazione
- Un pacchetto di prova minimale nei test si carica; un riferimento sbagliato dà un errore leggibile.
