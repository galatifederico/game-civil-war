# 04 — Movimento e ostacoli

**Contesto.** Il design non parla di mappa fisica, ma molte meccaniche (pattuglie, testimoni, contagio,
dungeon) hanno bisogno di posizioni.

**Decisione presa.** Griglia a livelli (superficie + sotterranei) con zone rettangolari e portali tra livelli.
Movimento a passi diretti verso il bersaglio (un passo in diagonale conta come uno), senza ostacoli.
Velocità dalla statistica `speed` (passi per tick, con accumulo frazionario).

**Alternative.** A* su celle bloccanti (quando ci sarà un client con una mappa vera); grafo di zone astratto
senza coordinate.
