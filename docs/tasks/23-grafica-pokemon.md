# 23 — Grafica più vicina a Pokémon

**Fatto.**
- Camera ravvicinata a zoom intero (pixel nitidi), che segue il campione o la pedina selezionata (F); panoramica con M.
- Alberi alti due caselle sopra le pedine, fronte delle rocce nelle grotte, rive dell'acqua, ombra ai piedi dei muri.
- Interfaccia a finestre bianche con bordo, nome del luogo e del quartiere quando ci si entra, riquadro di testo
  per l'ultima notizia, pannello laterale nascondibile (Tab), nebbia leggera.
- Caselle per grotte e profondità, sprite per i nuovi edifici e per talpa, verme, ragno e cinghiale.
- Terreno aggiornato dal vivo (`/api/ui/terrain`): solo le caselle cambiate vengono ridisegnate.
- Scavo trascinando un rettangolo (pulsante "Scava").

**Generatori:** `tools/sprites/tiles.py`, `buildings.py`, `chibi.py` (solo arte originale).
