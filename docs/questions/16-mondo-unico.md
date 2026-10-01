# 16 — Mappa unica o mappe separate?

**Contesto.** Hai chiesto mappe più grandi, "idealmente una mappa unica, come Dwarf Fortress".

**Decisioni prese.**
- Come in Dwarf Fortress: una sola grande mappa in superficie con i livelli sotterranei sotto (z-level), tutti
  della stessa grandezza. Ho tenuto le vecchie mappe come quartieri (stessi nomi e id) e gli interni come mappe
  separate, perché un interno dentro la mappa grande avrebbe richiesto di ridisegnare gli edifici come stanze.
- Superficie di 200×140 caselle (5×5 quartieri da 40×28). Ingrandirla ancora si può (il generatore lo
  permette), ma il costo per tick cresce: oggi 6,5 ms.

**Alternative.**
- Interni dentro la mappa grande (tetti che spariscono quando entri, come in RimWorld).
- Superficie ancora più grande (7×7 quartieri, 280×196) con più campagna e paesi intorno.
- Più livelli sotterranei (oggi 4).
