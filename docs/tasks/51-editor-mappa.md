# 51 — Editor della mappa nella console

Richiesta: una pagina della console admin con un editor per modificare la mappa: muri, pavimenti (cemento,
sampietrini, erba, terra, piastrelle di cui si sceglie il colore), piante, fiume, campi; poter nascondere degli
oggetti e inserire degli NPC; il più completo possibile.

**Fatto.** Console → *Mondo* → **Editor della mappa** (`/admin/#mapedit`, codice in
`sim_core/src/server/mapeditor.js`, API in `server/mapedit.rs`).

## Cosa si fa dall'editor
- **Caselle:** pennello (grandezza 1–25, quadrato o tondo), gomma, linea (con la grandezza: strade e fiumi),
  rettangolo ed ellisse (pieni o solo bordo), riempimento (zona contigua o «sostituisci ovunque»), contagocce,
  **stanza** (muri, pavimento e porta in un gesto), selezione con copia/taglia/incolla, specchia e ruota.
  Tasto sinistro = casella primaria, destro = secondaria.
- **Tavolozza a gruppi:** Pavimenti, Terreno, Campi, Piante, Acqua, Muri e arredi, Sottosuolo, con ricerca.
  **Variante colorata:** si sceglie una casella (es. Piastrelle), un colore e un nome: nasce un nuovo tipo di
  casella con quella figura tinta di quel colore.
- **Legenda:** ogni tipo di casella si modifica (nome, id, calpestabile, gruppo, figura, colore, scavo e cosa
  rende); se ne creano di nuovi e si cancellano quelli non usati.
- **Edifici:** tipo, nome, proprietario, contenuto proprio (`stock`), **nascosto**.
- **NPC:** template, nome, fazione, quanti, posizione precisa o a caso nella zona, legame a una zona, capo di un
  giocatore.
- **Oggetti nascosti:** si sceglie l'oggetto e la quantità e si clicca dove nasconderlo; dalla scheda se ne
  aggiungono altri.
- **Zone:** si disegnano col rettangolo, si spostano e ridimensionano dagli angoli; nome, id (la rinomina
  aggiorna piazzamenti e reti), tag. **Reti** (acquedotto, fogne): zone collegate.
- **Passaggi** (scale, botole, porte) fra due punti, anche di mappe diverse; **bordi** fra mappe affiancate.
- **Mappe:** proprietà, ridimensionamento con ancoraggio (zone, passaggi e piazzamenti seguono), nuova mappa
  (es. un dungeon), duplica, elimina.
- **Vista:** zoom con la rotella, spostamento con spazio o tasto centrale, miniatura cliccabile, interruttori
  per griglia, zone, passaggi, edifici, NPC, nascosti, caselle solide (in rosso), piazzamenti degli altri file,
  nomi.
- **Sicurezza del lavoro:** annulla/rifai di tutto, bozza automatica nel browser (riproposta alla riapertura),
  avviso se si chiude con modifiche non salvate, esporta/importa in JSON, esporta la mappa come immagine.
- **Verifica:** controlli immediati (piazzamenti e passaggi su caselle solide o fuori mappa, zone che escono,
  mappe non collegate, nascondigli vuoti; un clic porta al punto) più la verifica del server, che ricarica
  tutti i contenuti con la mappa modificata come a un avvio.

## Salvare
- **Salva** scrive `data/maps/<mappa>.map` (le caselle) e `data/70_mappa.json` (legenda, mappe, zone, passaggi,
  bordi, reti, piazzamenti: una definizione per riga, senza i valori predefiniti). Al primo salvataggio
  `70_mappa.ron` (generato da `tools/maps/build_maps.py`) viene sostituito dal `.json`: da lì la mappa si modifica
  dall'editor, e `build_maps.py` si rifiuta di rigenerare (serve `--force`, che butta via le modifiche).
- **Salva e ricomincia** inizia subito una nuova partita con la mappa salvata.
- **Applica caselle alla partita** manda alla partita in corso solo le caselle cambiate (comando `SetTiles`),
  senza ricominciare; zone, edifici e NPC nuovi arrivano con la partita successiva.
- Giocatore e popolazione iniziale stanno con la mappa (prima erano in `90_mondo.ron`): `build_maps.py` li copia
  da `tools/maps/popolazione.ron`, così l'editor li modifica tutti. Piazzamenti messi in altri file si
  vedrebbero col lucchetto, in sola lettura.

## Nel motore
- **Caselle:** `TileDef` ha `sprite` (figura `tile_<sprite>.png`, default l'id), `color` (`#rrggbb`, tinge la
  figura) e `group` (gruppo della tavolozza). `/api/ui/map` manda la legenda come
  `carattere → [id, calpestabile, figura, colore]`; il client Unity (`TileMap.cs`) e la pagina Mappa della
  console disegnano figura e tinta.
- **Terreni nuovi** (figure da `tools/sprites/tiles.py`): cemento, sampietrini, piastrelle (chiare, da
  colorare; già pronte azzurre, rosse e verdi), ghiaia, binari, campo di grano, campo di mais, vigneto, orto,
  acqua bassa (guado, ci si cammina), palude, muro di mattoni, canneto, siepe, aiuola.
- **Edifici nascosti:** `Placement::Building` ha `hidden` e `stock`. Un edificio nascosto (componente `Hidden`)
  non compare nella vista dei giocatori e non è un bersaglio dell'AI. Una pedina entro `hidden.find_range`
  caselle lo trova con probabilità `hidden.find_chance` + percezione × `hidden.find_per_perception` per tick
  (evento `found_hidden`). Il **nascondiglio** (edificio `nascondiglio`) è il contenitore degli oggetti nascosti:
  chi lo trova prende tutto e il nascondiglio sparisce; gli altri edifici nascosti diventano solo visibili.
- **API:** `GET /api/admin/map` (mappa modificabile, con i piazzamenti degli altri file in sola lettura),
  `PUT /api/admin/map` (`?dry=true` solo verifica, `?restart=true` nuova partita); comando
  `SetTiles { layer, cells: [(x, y, casella)] }`.

## Aggiunte
- **Mano** (✋, tasto T): si trascina la mappa per spostarsi; con gli altri strumenti resta lo spazio premuto o
  il tasto centrale.
- **＋ Nuova mappa** nella barra in alto: nome (l'id si ricava dal nome), sotterranea o all'aperto, dimensioni
  (piccola 40×28, media 64×40, grande 240×168) e casella di riempimento.
- **⤡ Amplia** nella barra in alto: caselle da aggiungere (o togliere, con un numero negativo) a nord, sud, est e
  ovest, con la casella delle parti nuove; zone, passaggi e piazzamenti seguono il contenuto.

- **La selezione prende anche gli elementi** dentro l'area: edifici, NPC (quelli «a caso nella zona» al centro
  della zona, dove sono disegnati), oggetti nascosti, passaggi e le zone che ci stanno per intero; le spunte
  «Prende» scelgono cosa. *Elimina gli elementi* (o Canc), *Svuota le caselle*, *Elimina tutto*; le frecce
  spostano gli elementi; copia, taglia e incolla portano con sé edifici, NPC e oggetti nascosti (anche
  specchiati o ruotati). Eliminare una zona usata da fazioni, azioni o eventi fa fallire la Verifica, che
  dice da chi è usata.

- **Zone usate dal gioco:** l'editor sa quali zone (per id o per tag) sono nominate da fazioni, azioni ed
  eventi (`used_zones`, `used_tags` in `GET /api/admin/map`): non si eliminano e il loro id non si cambia
  (🔒 nella scheda), ma si spostano e ridimensionano; l'eliminazione in massa le salta e lo dice. Se il
  salvataggio è rifiutato perché ne mancano, la Verifica offre **Ripristina le zone mancanti**, che le rimette
  dai file salvati senza toccare il resto. La bozza automatica avvisa se i file sono cambiati dopo di lei.

**Da rifinire.** Le decorazioni (`props`) non hanno ancora un uso nei client e l'editor non le tocca; `Applica caselle` non porta nella partita in corso i
tipi di casella appena creati (serve ricominciare); i parametri della scoperta sono da bilanciare.
