# 15 — La regione in mappe

**Contesto.** Hai chiesto Fidenza divisa in più mappe contigue, interni come mappe proprie, e mi hai lasciato
scegliere la lista.

**Decisioni prese.**
- 20 mappe: 11 esterne da 40×28 caselle collegate dai bordi, 6 interni, 2 sotterranei (lista e schema nella task 22).
- Gli edifici con interno hanno la porta sulla stessa casella dove le pedine comprano o lavorano: si entra
  camminandoci sopra. Il Capannone di Babbo Natale è un cortile esterno (gli elfi ci lavorano senza uscire).
- Le pedine si muovono tra mappe quando serve (spesa, lavoro, furti, arresti): conviene loro restare vicine,
  perché ogni passaggio pesa come 25 caselle nelle scelte dell'AI.
- Ogni centro ha i suoi negozi (forni e bancarelle in città, gestiti da fazioni locali): senza, gli abitanti
  della piazza morivano di fame aspettando il pane dalla campagna.
- Mappe descritte da un generatore Python (non a mano): per cambiarle si modifica `tools/maps/build_maps.py`.

**Alternative / da decidere.**
- Mappe disegnate a mano (o con l'editor gratuito Tiled) invece che dal generatore.
- Più interni (negozi, case, Circolo dei Boomer, Fumetteria), più mappe di campagna, Salsomaggiore più grande.
- Transizione "a schermo intero" come in Pokémon (una mappa alla volta con dissolvenza) invece della vista libera.
