# 42 — Circostanze globali come stati del mondo

**Fatto.**
- Una **circostanza** (sciopero, nebbia, festa della birra, ordinanza…) è come uno stato, ma della città: intensità
  0–100, avviarla dà una dose (`intensity`, di default 100; l'effetto `GlobalModifier(id)` può darne più dosi), ogni
  ora cambia di `per_tick`, a 0 finisce. La durata è nella circostanza, non più nell'effetto che la avvia.
- **Finché è attiva:** modificatori di caratteristica su tutte le pedine (`stats`, es. morale: prima era un colpo
  solo all'avvio, ora vale per tutta la durata), prezzi per categoria di oggetto (`prices`), rifornimenti da fuori
  (`supply`, per oggetto o categoria, «*» = tutto), ritardi nelle consegne (`logistics_disruption`).
- Migrate le 10 circostanze (durate prese dagli effetti che le avviavano: −100/durata all'ora); l'Ordinanza del
  Sindaco, che era scritta solo dentro l'effetto, è una circostanza a sé. L'API dell'economia dà ancora id, nome,
  ore rimaste, ritardi e morale (più l'intensità).
- **Console:** elenco con ritmo, durata, effetti su tutte le pedine, prezzi, rifornimenti e ritardi; scheda con
  «Intensità» (e pulsante «Avviala ora») e «Finché è attiva».

**Da tarare.** Il morale ora dura quanto la circostanza: «Natale cancellato» (−20 per una settimana) è più pesante
di prima.
