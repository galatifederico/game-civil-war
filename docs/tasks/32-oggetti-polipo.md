# 32 — Caratteristiche degli oggetti, Forza e Difesa, il Polipo

**Fatto.**
- **Forza e Difesa** (`attacco`, `difesa`, gruppo Principali, `bindings.attack`/`defense`): ogni colpo di qualcuno fa
  danni × (1 + Forza/100) × 100 / (100 + Difesa). Classi da combattimento hanno bonus (Cavaliere, Templare, Sith…).
- **Oggetti** (motore, `ItemDef`): `types` (arma, lanciabile, armatura, copricapo, accessorio…), `durability` (vita:
  si consuma colpendo, lanciando o venendo colpiti e a zero si rompe), `weight` (oltre la capacità, che cresce con i
  Muscoli, si rallenta), `requires` (es. arti marziali per la katana, Forza della Star Wars per la spada laser),
  `hands` (mani occupate: si impugna finché ci sono mani libere, anche dopo aver perso una mano), `wear_slot`
  (corpo, testa: conta il migliore per posto), `damage`, `range`, `on_hit` (effetti su chi viene colpito).
  Cumulabile = `stack_max` dello slot: le armi stanno da sole, i petardi fino a 100 in uno slot. Le pedine hanno
  ora 5 slot (`inventory.slots`).
- Azioni per tipo: `lancia_oggetto` (handler `throw`) lancia la cosa più dannosa di tipo lanciabile; l'attacco usa
  la migliore arma impugnata. Condizione `HasItemType`. La scheda mostra cosa è "in uso"; la scheda dell'oggetto
  spiega tipo, danni, mani, requisiti, peso, durata.
- **~40 oggetti nuovi** (`35_equipaggiamento.ron`): spade, spadone, mazza, forcone, motosega, spada laser, bastone
  dell'Umarell, frusta; sassi, uova e pomodori marci, ciabatta della nonna (torna indietro), bottiglie, cotechino,
  boomerang, molotov (ustiona); giubbotto antiproiettile, armatura templare, gilet da pesca, piumino, tuta acetata,
  saio nero della Cripta; elmo, casco, cappello da alpino, coppola, cappello da strega, elmetto vichingo…; occhiali,
  Anello del potere, scarpe da corsa, catenone, rosario, pentacolo di San Vitale, zaino. In vendita all'Outlet, al
  mercato, alla Fumetteria e nel retro del Casinò; la Fucina forgia spade, spadoni, elmi e armature.
- **Scantinato della Fumetteria** (dungeon dei nerd, botola nel Quartiere Nerd) con il **Polipo**: corpo con 8
  tentacoli, tentacoli che avvinghiano, nube d'inchiostro quando è ferito; punta prima le donne (test
  `the_octopus_goes_for_women_first`). Nello scantinato c'è un baule con spada laser e Anello del potere; i nerd
  appassionati ci scendono a curiosare. Sprite `chibi_polipo` in `tools/sprites/chibi.py`.

**Da rivedere (bilanciamento).** In partita quasi nessuno lancia oggetti spontaneamente (servono nemici a portata);
il meccanismo è provato dal test `the_champion_throws_what_can_be_thrown`.
