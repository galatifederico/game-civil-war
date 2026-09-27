# 03 — Determinismo e significato del tick

**Contesto.** Il motore deve essere deterministico; il design non dice quanto dura un tick.

**Decisione presa.**
- Executor ECS single-thread, sistemi in ordine esplicito, RNG unico seedato (SplitMix64 interno),
  mappe ordinate (`BTreeMap`) nello stato, entità processate in ordine di `SimId`.
- 1 tick = 1 ora di gioco (`time.ticks_per_day = 24`, parametro). Stipendi ogni 24 tick di default.
- Tutti gli input esterni diventano `SimCommand` applicati all'inizio del tick successivo.

**Alternative.** Executor multi-thread (più veloce, ma serve più disciplina per il determinismo);
tick di durata diversa (es. 10 minuti di gioco) con i ritmi scalati dai parametri.
