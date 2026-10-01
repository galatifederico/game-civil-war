# 24 — Un mondo unico, come Dwarf Fortress

**Fatto.**
- Le 14 mappe esterne sono diventate quartieri di un'unica superficie di 200×140 caselle; tra i luoghi, campagna
  generata (prati, boschi, stagni, sterrati).
- Quattro livelli sotterranei grandi quanto la superficie, di roccia scavabile con vene e sacche; i dungeon
  (Fortezza dei Nani, Cripta, Miniera di Sale, Miniere Profonde, Catacombe, Caverne, Cuore Termale) sono scavati
  sotto i loro ingressi e collegati da scale.
- I 6 interni restano mappe separate a cui si entra dalle porte.
- Ogni vecchia mappa è ancora una zona con lo stesso id: i contenuti che la nominano funzionano come prima.
- Generatore deterministico (`tools/maps/build_maps.py`, niente `hash()` di Python).

**Prestazioni:** circa 6,5 ms per tick (prima 2,6).
