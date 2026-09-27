# 08 — Obiettivo di prestazioni

**Contesto.** Il design non indica quante pedine deve reggere il motore.

**Decisione presa.** Ottimizzato fino a ~2,5 ms per tick con ~90 pedine e ~40 edifici (release), cioè molto sotto
il tempo reale anche a 10 tick al secondo. La ricerca dei bersagli dell'AI è lineare nel numero di entità:
va bene fino a qualche migliaio di entità.

**Alternative.**
- Indice spaziale a griglia per le ricerche per distanza (necessario sopra le ~2000 pedine).
- Executor multi-thread di bevy per i sistemi indipendenti (richiede più disciplina per il determinismo).
- AI a frequenza variabile (le pedine lontane dal giocatore pensano meno spesso).
