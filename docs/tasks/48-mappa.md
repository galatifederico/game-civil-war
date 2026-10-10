# 48 — Mappa più grande e più fedele, interni disegnati sulla superficie

**Fatto** (`tools/maps/build_maps.py`, rigenera `data/maps/*.map` e `data/70_mappa.ron`).
- **Più grande:** la superficie passa da 5×5 a 6×6 pezzi da 40×28 (240×168 caselle), disposti come la geografia
  vera: la **Stazione** (ferrovia Milano–Bologna che attraversa tutta la mappa) a nord, sotto **Piazza del Duomo**,
  poi **Piazza Garibaldi** sulla **Via Emilia** (asfalto da un capo all'altro della mappa); a est Quartiere Nerd,
  Borgo dei Templari, **Fidenza Village** con l'**A1**, il capannone e l'**Ospedale di Vaio**; a ovest l'Impero
  Vegano, le Rotonde, la Bassa e il **Bosco dello Stirone**; a sud la Provinciale sul torrente, poi
  **Salsomaggiore**, il Casinò, **Tabiano Terme** (con il castello sulla rupe) e le Colline di Salso.
  Nuovi pezzi: `stazione`, `ospedale_vaio`, `tabiano_terme`.
- **Interni dentro la mappa** (muri, pavimento, porta con zerbino, arredi):
  - Duomo: navata con le panche, il tappeto rosso fino al presbiterio, porta sul sagrato e porta laterale;
  - Comando di Polizia in Piazza Garibaldi, con il bancone e le celle dietro le sbarre;
  - Bar degli Ubriaconi nel Borgo, con il bancone e il trono sul tappeto in fondo;
  - Outlet con le botteghe e le casseforti del CdA;
  - Terme Berzieri con le vasche e la scala per la Cripta;
  - Sala da gioco del Casinò con i tavoli e il retrobottega;
  - Fumetteria con gli scaffali e la botola;
  - nuovi: atrio della Stazione con biglietteria ed edicola, Ospedale di Vaio con letti e pronto soccorso, Terme
    di Tabiano, Castello di Tabiano.
- **Stessi nomi:** le zone `duomo_interno`, `stazione_polizia`, `celle`, `bar_ubriaconi`, `trono_ubriaconi`,
  `outlet_interno`, `terme_interno`, `sala_casino` restano (ora sono zone dentro il pezzo di superficie), quindi
  fazioni, apparizioni, ordini, acquedotto e fogne funzionano come prima. Tolte le mappe separate degli interni e
  le loro porte.
- **Lo Scantinato della Fumetteria** è un pezzo del primo sottosuolo sotto il negozio: la botola diventa una
  scala.
- **Ingombro:** gli edifici con l'interno disegnato (cattedrale, stazione di polizia, bar, outlet, terme, sala da
  gioco, fumetteria) hanno ingombro 1×1, altrimenti l'ingombro bloccherebbe il pavimento dell'interno.

**Da rifinire.** Le celle di campagna fra i quartieri sono ancora generate a caso (prati, boschi, stagni); si
possono sostituire con altri luoghi veri (Fornio, Castione Marchesi, Soragna…) quando servono.
