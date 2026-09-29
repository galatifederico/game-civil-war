# 22 — Regione in mappe contigue, terreno in pixel art

Richiesta del 2026-09-29/30. Stato: ✅ prima versione.

**Motore**
- Mappe a caselle (`LayerDef::tiles` o `tiles_file`) con una legenda (`MapDef::legend`: carattere → terreno,
  percorribile o no). Ogni mappa è anche una zona con il proprio id e i propri tag.
- `edges`: il bordo di una mappa porta alla mappa accanto (ogni casella libera del bordo diventa un passaggio).
  Le porte e le scale restano `portals`.
- Edifici solidi (`BuildingDef::footprint`) tranne la porta; decorazioni solide (`props`).
- Movimento gerarchico: prima la sequenza di mappe (grafo dei passaggi), poi A\* dentro la mappa corrente.
- Distanza di viaggio tra mappe per le scelte dell'AI (`move.map_hop_cost`, 25 caselle per passaggio).
- Selettore `Nearby(r)` per le passeggiate; `needs_exempt` sulle razze (macchine e bestie non mangiano).

**Fidenza e Salsomaggiore** (`python3 tools/maps/build_maps.py` rigenera mappe e `data/70_mappa.ron`)
```
Impero Vegano ─ Sagrato del Duomo ─ Quartiere Nerd
      │               │                   │
   Rotonde ──── Piazza Garibaldi ──── Borgo dei Templari ── Fidenza Village
      │               │
Campagna ────── Strada Provinciale ── Capannone di Babbo Natale
                      │
            Salsomaggiore e Terme ── Casinò
```
Interni: Duomo, Comando di Polizia (con le celle), Bar degli Ubriaconi (con la Sala del Trono), Negozi
dell'Outlet (casseforti del CdA), Stabilimento Termale, Sala da gioco. Sotterranei: Gallerie dei Nani (dal
tombino di Piazza Garibaldi), Cripta di San Vitale (dalle scale delle Terme).

**Grafica**: `python3 tools/sprites/tiles.py` genera 30 caselle 16×16 (erba, fiori, erba alta, lastricato,
sterrato, asfalto, sabbia, terra arata, ponte, acqua, alberi, cespugli, muri, staccionate, parquet, marmo,
tappeto, panche, banconi, piscina, sbarre, grotta…); il client Unity compone il terreno di ogni mappa, ha un
menu delle mappe e segue il campione quando cambia mappa.

**Bilanciamento sulle nuove mappe** (25 giorni, 2 seed): 94-102 abitanti, morale 36-50, fame 0,5-0,65,
cibo sempre nei negozi (forni e bancarelle anche in città, gestiti dalle fazioni locali).
