# 50 — Quattro mappe grandi, un sottosuolo, dungeon separati

Richiesta: allargare ancora il mondo, una mappa per Fidenza, una per Salso, una per il Fidenza Village con i
campi, una per la Bassa (solo campi), tutte grandi come Fidenza; un solo sottosuolo (per i nani) e i dungeon come
mappe separate raggiungibili da alcuni luoghi. Generatore: `tools/maps/build_maps.py`.

## Superficie: 4 mappe da 240×168 (6×6 pezzi da 40×28), unite ai bordi
- **Fidenza** (`fidenza`): Stazione e ferrovia (passaggio a livello verso la Bassa), Piazza del Duomo, Piazza
  Garibaldi sulla Via Emilia, Quartiere Nerd, Borgo dei Templari, Impero Vegano, Rotonde, Ospedale di Vaio,
  Strada Provinciale; nuovi il **Teatro Magnani** (platea, palco, sipario) e il **Cimitero** (tombe in file,
  cappella, Mausoleo dei Vescovi). Intorno, isolati di **città**: strade, marciapiedi, case in cui si entra
  (porta, pavimento, tavolo), giardinetti con la fontana; più fuori i campi.
- **Salsomaggiore** (`salsomaggiore`): Bosco dello Stirone (da cui si arriva da Fidenza), Terme Berzieri, Casinò,
  **Palazzo dei Congressi** (salone moresco), Tabiano con il castello, Colline di Salso con la miniera, **Castello
  di Scipione** sulla rupe; vie di città vicino alle terme, colline con boschi, rocce e vigne intorno.
- **Fidenza Village e i campi** (`village`): Village con l'A1 che corre per tutta la mappa, capannone di Babbo
  Natale, Campi e cascine (mulino, forno, cantina, salumificio…), **Caseificio del Parmigiano**; la Via Emilia
  continua da Fidenza.
- **La Bassa** (`bassa`): solo campi: strisce di coltivazioni, siepi, pioppi lungo le carrarecce, fossi, boschi
  e stagni intatti, il **Po** lungo tutto il bordo nord. Zona `la_bassa`, con i cinghiali.
- **Confini:** Bassa a nord di Fidenza (3 passaggi), Village a est (3, tra cui la Via Emilia), Salso a sud (2).

## Sottosuolo: solo il Regno dei Nani (`sottosuolo_1`, sotto Fidenza)
Fortezza, Filone di ferro e Tana dei Dinosauri dal tombino di Piazza Garibaldi; Miniere Profonde, Caverne dei
Porcini e Cuore Termale sono sale dello stesso livello, unite da gallerie (prima erano 4 livelli). Tolti
`sottosuolo_2..4`.

## Dungeon: mappe a parte, con le scale
| Dungeon | Entrata | Cosa c'è |
|---|---|---|
| Cripta di San Donnino | scala nel presbiterio del Duomo, e Mausoleo dei Vescovi al Cimitero | tombe dei vescovi, ossario a labirinto (zombie), pozzo allagato, Cappella delle Reliquie con il **Reliquiario** e il **Vescovo Non Morto** |
| Cripta e Catacombe di San Vitale | scala nelle Terme Berzieri | come prima (labirinti, altare, zombie) |
| Scantinato della Fumetteria | botola in negozio | la stanza del polipo, **Sala LAN**, **Archivio dei Fumetti** a labirinto (ragni), **Tana del Dungeon Master** con il baule |
| **Cantine del Culatello** (i ciccioni) | botola accanto al Salumificio, nei campi del Village | culatelli appesi, Sala del Banchetto con la **Confraternita del Culatello**, trono del **Gran Mangione**, **Dispensa Proibita**, caveau dei formaggi |
| **Covo dei Rettiliani** | ascensore riservato del CdA nell'Outlet | sala server, incubatoio con le uova (guardie), Sala del Consiglio con la **Regina dei Rettiliani** e il **Caveau** |
| Miniera di Sale | bocca nelle Colline di Salso | come prima, con le gallerie che arrivano al deposito e alla fungaia |

Dati in `data/77_dungeon.ron`: fazione Confraternita del Culatello, boss e mostri (con apparizioni legate al
dungeon), tre forzieri, tre azioni per scendere (curiosi nella cripta, chi ha il grasso alto nelle cantine,
gli investigatori nel covo).

## Cambiato nel motore
- **Costo di viaggio fra mappe:** prima ogni passaggio valeva 25 caselle a prescindere da dove fosse, quindi un
  lavoro sull'altra mappa sembrava vicino e le pedine attraversavano tutta la città morendo di fame. Ora conta la
  strada fino al passaggio migliore e da lì alla meta.
- **Distanza massima di un lavoro della bacheca** (`jobs.max_distance`, 90 caselle; modificabile in console).
- I personaggi della scena iniziale in piazza hanno le coordinate della nuova piazza.

**Da rifinire:** la forza dei boss e quanto spesso ricompaiono; altri dungeon possibili (bunker dell'Overmind
sotto il Circolo dei Boomer, cantine del Castello di Scipione).
