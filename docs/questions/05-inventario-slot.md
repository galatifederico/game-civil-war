# 05 — Interpretazione di "slot eterogenei" e "stacking per categoria"

**Contesto.** Il design chiede "limiti rigidi di slot eterogenei (es. cap massimo a 3 slot distinti per pedina)
e stacking per categoria".

**Decisione presa.** Uno slot contiene oggetti di **una sola categoria** (cibo, arma, reliquia…), anche di tipi
diversi (pane e salumi insieme nello slot "cibo"). La capienza dello slot è il più piccolo `stack_max` tra gli
oggetti che contiene. Il numero di slot è 3 per default, modificabile per template (`slots`); 0 = illimitato
(usato per creature ed entità virtuali). Gli edifici hanno magazzini illimitati (`Stock`).

**Alternative.**
- Uno slot per tipo di oggetto (3 tipi di oggetto in tutto), stack per tipo.
- Peso/volume invece degli slot.
