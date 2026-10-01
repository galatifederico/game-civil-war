# 27 — Console di amministrazione (`/admin/`)

**Fatto.** Una pagina servita dal motore (generica, non legata a Fidenza):
- **Mappa:** caselle vere, zone e quartieri, edifici e pedine; ispeziona casella ed entità, dipingi caselle,
  piazza edifici, sposta entità, crea pedine.
- **Entità:** elenco filtrabile con scheda, inventario e Utility AI.
- **Oggetti:** dove si trovano (inventari e scorte).
- **Contenuti:** tutte le definizioni caricate (`/api/content`).
- **Sprite:** galleria delle immagini (`/api/assets`, `/assets/…`) e mappatura modificabile dal vivo.
- **Parametri** modificabili dal vivo, **Fazioni e quartieri**, **Eventi**, **Comandi** con esempi.
- Nuovi comandi: `place_building`, `move_entity`, `remove`.

Le schede si aprono anche direttamente: `/admin/#items`, `/admin/#factions`…

**Non fatto:** modifica delle definizioni (oggetti, edifici…) dalla console: si cambiano i file `.ron` e si
riavvia (vedi domanda 18).
