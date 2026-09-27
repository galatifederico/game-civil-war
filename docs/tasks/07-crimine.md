# 07 — Crimine, polizia neutra, tangenti

Priorità: 7 (serve alla demo).

- Job con `crime` (gravità, raggio dei testimoni): i testimoni che percepiscono l'autore (percezione vs
  stealth) sporgono denuncia → `Wanted.level` e capi d'accusa.
- Polizia (fazione con ruolo `police`): pattuglia le zone; se vede un ricercato sopra soglia lo perquisisce
  (sequestro del contrabbando), sopra la soglia d'arresto lo arresta e lo porta in cella (`Detained`).
- Perquisizioni casuali con probabilità configurabile.
- `SimCommand::Bribe`: il leader paga dal tesoro di gilda; se l'importo copre il costo (livello × tariffa
  × corruttibilità della polizia) accuse azzerate e rilascio; altrimenti tentata corruzione (+ricercato).
