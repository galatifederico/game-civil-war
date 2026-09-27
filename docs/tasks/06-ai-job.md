# 06 — Utility AI, JobQueue, WorkPriorityMatrix, squadre, ActivityState

Priorità: 6.

- Utility AI: azioni da dati con considerazioni (input: bisogno, statistica, status, soldi, ricercato, ora,
  job disponibili…) passate su curve (lineare, quadratica, logistica, gradino, inversa) e moltiplicate.
- Action Momentum: bonus all'azione corrente che decade; cooldown per azione.
- Ultima valutazione salvata per l'ispezione dall'admin.
- JobBoard: coda globale e per fazione; priorità, grado minimo, ruolo/classe richiesta, prenotazione.
- Esecuzione: raggiungi il bersaglio → progresso → completamento (effetti da def + handler per nome).
  Handler generici del motore: build, demolish, produce (coltivazione/allevamento/trasformazione), steal,
  sabotage, assassinate, infiltrate, identity_theft, patrol, arrest, search, seize, investigate, publish,
  treat, inoculate, psychic, transmute, clean.
- WorkPriorityMatrix per entità (0 = disabilitato, 1 = massima … 4).
- Squadre: membri, leader, ordini macro (muovi, pattuglia, attacca, tieni posizione) che prevalgono sull'AI.
- ActivityState: azione, job, progresso 0..1, umore, per la UI.
