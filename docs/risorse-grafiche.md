# Risorse grafiche

Materiale che si può usare per migliorare la grafica. Raccolto il 2026-09-20 su indicazione dell'utente; ciò che è scritto qui **è stato verificato leggendo le pagine e, per Kenney, scaricando i pacchetti** (le voci non verificate sono dichiarate come tali). Oggi tutta la grafica del client è disegnata in codice (`PixelArt.cs`), senza asset importati.

## Direzione grafica scelta dall'utente (2026-09-20)

Riferimento: la grafica dei giochi **Pokémon (generazioni 3-5)**, vista in due schermate (un gioco amatoriale con la mappa e un riquadro di dialogo con il ritratto del personaggio; la schermata della Pokédex della versione GBA):

- **Mappa dall'alto a 3/4**, a caselle quadrate piccole (16 px), non isometrica: alberi, erba, sentieri, staccionate e case disegnati con un lato frontale visibile; personaggi piccoli che camminano nelle 4 direzioni.
- **Interfaccia con cornici in pixel art**: riquadro di dialogo in basso con bordo azzurro/blu, nome del parlante in una linguetta sopra, ritratto grande del personaggio che parla; elenchi con riga selezionata evidenziata e piccole icone; sfondo scuro a bande.
- Colori saturi, contorni scuri, ombre morbide sotto alberi e personaggi.

**Attenzione alla provenienza**: quelle immagini sono di Nintendo/Game Freak (una è di un gioco amatoriale che usa sprite ufficiali). Sono solo un **riferimento di stile**: gli sprite, i tileset e i ritratti di Pokémon o dei giochi amatoriali **non si copiano né si mettono nel repo**. Si usano risorse CC0 (Kenney), disegni originali o generati (con licenza controllata) e non si usano nomi o personaggi di Pokémon.

**Cosa cambia rispetto a oggi** (il client è isometrico 2:1 a 32 px):

- Il rendering passa da isometrico a **dall'alto ortogonale**: `GridMath` (cella ↔ scena) diventa una semplice griglia, la selezione delle caselle e l'ordinamento in profondità si semplificano (ordine per y, come oggi). Non cambia niente lato server.
- **Board esagonali: eliminate** (decisione dell'utente, 2026-09-20). Tutte le board sono quadrate; la migrazione `0009` ha trasformato l'Alveare del mondo di prova.
- Serve il dato **terreno per casella** (erba, sentiero, acqua, alberi, muri...) nel mondo e nell'app admin; senza, ogni board resterebbe una distesa uniforme.
- Serve un **riquadro di dialogo** con nome e ritratto (oggi i dialoghi con gli NPC sono un messaggio nel menu) e **cornici di pannello** per il menu.
- Le pedine possono restare tinte per squadra solo se lo sprite è in grigi o ha zone da colorare; con sprite già colorati si distingue la squadra con un anello/bandierina sotto la pedina.

**Pacchetti Kenney più vicini a questo stile** (tutti CC0, 16x16, dall'alto a 3/4; ho visto le immagini di esempio dei pacchetti):

- `tiny-town` — **il più vicino**: prati, sentieri, alberi (verdi e autunnali), case con tetti rossi e grigi, staccionate, alveari, cartelli, castello, fiori e personaggi. Base consigliata per l'esterno.
- `tiny-dungeon` — interni e sotterranei, eroi, mostri, pozioni, forzieri, spade: oggetti e nemici.
- `roguelike-rpg-pack` — interni di edifici (pavimenti, mobili, finestre, scale): case ed edifici visitabili.
- `roguelike-characters` — personaggi componibili (corpo, vestiti, capelli) per avere tante pedine diverse.
- `rpg-urban-pack` — città moderna (strade, auto, palazzi): fuori tema per un mondo fantasy, ma nello stesso stile.
- `pixel-ui-pack`, `ui-pack-pixel-adventure` — pannelli, pulsanti e cornici da adattare al riquadro di dialogo e ai menu.
- I ritratti grandi in stile anime come quello dell'immagine **non esistono** in questi pacchetti: si possono fare con Kitbitz (non è pixel art), con sprite-sheet-creator (originali, da controllare) o disegnarli.

## Regole d'uso

- **Si possono usare senza chiedere** le risorse **CC0** (pubblico dominio): uso commerciale, modifiche e ridistribuzione consentiti, attribuzione non richiesta. Qui lo sono Kenney e Kitbitz.
- Un pacchetto va **in una cartella sua**, `unity-client/Assets/Art/<autore>/<pacchetto>/`, insieme al suo file di licenza (`License.txt`). Si tengono nel repo **solo i file che il gioco usa** (i pacchetti interi pesano decine di MB; il pacchetto isometrico di Kenney è 14 MB).
- Tutto ciò che viene da fuori va elencato nella tabella "Asset in uso" in fondo a questo file (pacchetto, file usati, dove, licenza).
- Ciò che **non** è CC0 (per esempio immagini generate da un modello AI con i suoi termini) va controllato prima di finire nel repo.
- Le tre risorse esterne sotto sono tutte gratuite, ma **scaricare** un file da un sito è comunque una richiesta di rete: nessuno degli strumenti qui dentro richiede account per i file CC0.

## Cosa deve poter mostrare il client (vincoli)

- (Prima del refactor) vista **isometrica 2:1**: casella quadrata = rombo di 32 px di larghezza (`PixelArt.PixelsPerUnit = 32`, `GridMath.TileWidth = 1`). Ora: caselle quadrate dall'alto, da 16 px.
- Le pedine sono sprite in **scala di grigi colorati per squadra** (`SpriteRenderer.color`): un asset a colori fissi non si tinge bene, va bene per terreno, oggetti, NPC e icone.
- Ordinamento per profondità in base alla y (`BoardManager.SortOrder`), quindi gli sprite alti (muri, alberi) funzionano con il pivot ai piedi.
- Oggi non esiste un **tipo di terreno** per casella: ogni board è una scacchiera chiara/scura. Usare tile diversi (erba, acqua, pietra...) richiede un dato in più nel mondo (per esempio `board_cells`) e la sua modifica nell'app admin.

## 1. Kenney — https://kenney.nl (CC0)

Migliaia di asset gratuiti (2D, 3D, texture, UI, suoni). Licenza dichiarata dal sito: *"all game assets on the asset pages are public domain licensed (CC0). You're free to use them, even in commercial projects."* Attribuzione non richiesta (gradita); non si usa il loro logo. Ogni pacchetto contiene un `License.txt` con la stessa dicitura.

Pacchetti visti, con il link diretto alla pagina (`https://kenney.nl/assets/<nome>`):

| Pacchetto | Cosa c'è (verificato) | Stile | Adatto a |
|---|---|---|---|
| `isometric-miniature-dungeon` | 70 PNG nella scheda; il file scaricato ha 289 pezzi (muri, pavimenti, tavoli, botti, casse, scale...) in **4 angoli di vista** (N/E/S/W), sprite da 256x512, più 169 personaggi maschili. CC0, 2019. Serie *Isometric Miniature*: esistono anche `-bases`, `-farm`, `-library`, `-prototype` | isometrico, rendering 3D "miniatura" ad alta risoluzione | terreno, oggetti a terra, NPC, strutture (gli avamposti) |
| `isometric-blocks` | 163 file, tile da 111x128 px in tre serie ("Voxel", "Platformer", "Abstract"): blocchi con erba, terra, sabbia, casse, monete, chiave, cactus... | isometrico, vettoriale cartoon | terreno per tipo di casella, oggetti |
| `isometric-roads` | esiste (pagina verificata, contenuto non ispezionato) | isometrico | strade tra caselle |
| `tiny-dungeon` | 130 asset, **16x16 px**, tilemap; eroi, mostri, oggetti, pozioni, forziere, spade | pixel art **dall'alto** (non isometrica) | icone degli oggetti e dei personaggi, se si accetta la vista dall'alto |
| `tiny-town` | esiste (pagina verificata, contenuto non ispezionato) | pixel art dall'alto | come sopra |
| `roguelike-rpg-pack`, `1-bit-pack` | esistono (pagine verificate, contenuto non ispezionato) | pixel art dall'alto | icone, personaggi |
| `board-game-icons` | 780 file: 513 PNG e 256 icone vettoriali (dadi, carte, pedine, frecce...) | icone piatte | icone dell'interfaccia e degli oggetti |
| `game-icons` | 105 icone (2014) | icone piatte | interfaccia |
| `pixel-ui-pack`, `ui-pack-pixel-adventure` | esistono (pagine verificate, contenuto non ispezionato) | pixel art | pannelli, pulsanti, cornici del menu |

Note per l'uso:

- **Non c'è un pacchetto isometrico in pixel art a 32 px** tra quelli visti: gli isometrici di Kenney sono in stile 3D/cartoon ad alta risoluzione (256 o 111 px per tile), quelli in pixel art sono dall'alto. Con la direzione scelta (vista dall'alto stile Pokémon, sezione sopra) i pacchetti isometrici non servono.
- I pacchetti si scaricano come ZIP dalla pagina del pacchetto (link "Download"); i pochi file che servono si copiano nella cartella del progetto.

## 2. Kitbitz — https://kitbitz.art (CC0)

Libreria di **2.043 illustrazioni disegnate a mano** (dichiarato: non generate da AI) in 13 "kit": Medieval (341), Nature (309), Interior (281), Space (170), Cyberpunk (157), Pirate (145), Halloween (133), City (115), Ruins (103), Western (84), Winter (79), Dungeon (74), Barbieland (52). Formati **SVG e PNG** (le dimensioni sono nel catalogo, per esempio 301x293), quasi tutti "oggetti" singoli (piante, mobili, casse, insegne...). Licenza **CC0 1.0** per le illustrazioni; il sito, il plugin Figma e il codice del server MCP non lo sono.

- **Catalogo JSON completo**: https://kitbitz.art/catalog.v1.json (3,6 MB; per ogni asset `assetId`, nome, kit, categoria, tag, descrizione e URL di SVG/PNG su `assets.kitbitz.art`). Sorgente degli asset: repository `CaptExcellent/kits-library-assets`.
- **Server MCP** pubblico, in sola lettura, senza autenticazione: `https://mcp.kitbitz.art` (Streamable HTTP; stesso servizio di `https://kitbitz.art/api/mcp`). Strumenti: `search_illustrations`, `get_illustration`, `find_related_illustrations`, `curate_scene`, `prepare_asset_pack` (manifest di download). Non scrive file sul disco: dà URL e metadati. Documentazione: https://kitbitz.art/docs/mcp.
- **Non è collegato a questa sessione.** Per aggiungerlo in Claude Code: `claude mcp add --transport http kitbitz https://mcp.kitbitz.art` (comando standard per i server MCP HTTP; non provato qui). Senza MCP si può comunque usare il catalogo JSON con uno script.
- **Stile**: illustrazioni vettoriali a mano libera, non pixel art e non isometriche. Adatte a icone degli oggetti in grande (inventario, dettaglio oggetto), sfondi/illustrazioni della lobby e del login, ritratti; poco adatte al terreno isometrico.
- Il server dice di essere "in sviluppo attivo": non dipenderne per la build (scaricare i file che servono e salvarli nel repo).

## 3. Tiled — https://www.mapeditor.org (open source)

Editor di mappe a caselle, gratuito. L'editor è sotto **GPL-2.0** (con parti BSD/Apache nel repository); le mappe che si producono sono dell'autore. Gestisce mappe **ortogonali** (quella che ci serve), isometriche ed esagonali, livelli di caselle e di oggetti, automapping, ed esporta in **TMX** (formato nativo), **JSON**, Lua e altri.

- Nel nostro caso le board sono definite nel database e modificate dall'app admin (`admin-web/`), non caricate da file. Tiled servirebbe come **strumento per disegnare** il terreno di una board e poi **importarlo** (un convertitore TMX/JSON → `board_cells` + NPC + oggetti + passaggi). Prima serve il dato "terreno per casella" (vedi vincoli).
- **Unity**: la pagina non parla di integrazione; i pacchetti di importazione TMX per Unity (per esempio SuperTiled2Unity) sono di terzi e **non li ho verificati**. Il client non usa le Tilemap di Unity (le caselle sono sprite creati a runtime), quindi la strada più semplice sarebbe importare il JSON di Tiled **nel backend**, non in Unity.

## 4. sprite-sheet-creator — repository locale `/home/g/projects/sprite-sheet-creator`

Applicazione Next.js dell'utente (`npm run dev`, http://localhost:3000) che genera sprite sheet in pixel art con i modelli di immagini di **fal.ai** (Nano Banana Pro / Lite, GPT Image 2). Ha due modalità: *side-scroller* (camminata, salto, attacco, idle e sfondi parallasse) e **isometrica** (camminata in 3 direzioni, attacco, idle e una mappa dall'alto). Estrae i frame con divisori regolabili, anteprima animata, rimozione dello sfondo (Bria).

- **Richiede `FAL_KEY`** in `.env.local` (esiste solo `.env.local.example`): a pagamento e manda i prompt (e le immagini caricate) a un servizio esterno. **Non l'ho avviato**: serve la chiave dell'utente e la sua conferma a spendere.
- Cosa si può fare con lui: generare **personaggi/NPC/pedine** coerenti in vari fotogrammi da un testo. Rischi: coerenza di stile tra un'immagine e l'altra, risoluzione non a 32 px (va ridimensionata senza sfocare, filtro Point), colori fissi (per le pedine di squadra servirebbe la versione in scala di grigi).
- **Licenza dei risultati**: dipende dai termini del modello scelto su fal.ai; da controllare prima di distribuire il gioco.

## Come si incastrano con quello che c'è

| Serve | Miglior candidato | Nota |
|---|---|---|
| Terreno delle caselle (erba, sentiero, acqua, alberi...) | `tiny-town` (esterni) e `roguelike-rpg-pack` (interni) di Kenney | serve il dato terreno per casella e il rendering dall'alto (vedi la sezione sulla direzione) |
| Icone degli oggetti | `board-game-icons` / `game-icons` / `tiny-dungeon` di Kenney, oppure Kitbitz (Medieval/Dungeon) | oggi sono 7 icone in pixel art in codice (`PixelArt.ItemIcon`); l'icona è un dato per oggetto (`board_items.icon`) |
| Pedine, NPC | `roguelike-characters`, `tiny-town`/`tiny-dungeon` (personaggi 16x16) oppure sprite-sheet-creator (originali) | per la squadra: anello/bandierina colorata sotto la pedina |
| Interfaccia (pannelli, riquadro di dialogo, pulsanti) | `pixel-ui-pack` / `ui-pack-pixel-adventure` di Kenney | oggi l'interfaccia è IMGUI con lo stile predefinito di Unity: servono uno `GUISkin` con cornici a 9 fette |
| Illustrazioni per lobby e login, ritratti | Kitbitz (vettoriale, non pixel art) o originali | CC0 |
| Disegnare mappe a mano | Tiled | dopo il dato terreno |

## Pixellab (generatore di sprite, account dell'utente)

Servizio a pagamento di sprite in pixel art; ha un **server MCP** (`https://api.pixellab.ai/mcp`, documentazione: https://api.pixellab.ai/mcp/docs, 94 strumenti: personaggi con 4/8 direzioni e animazioni, ritratti, tileset dall'alto, oggetti, pannelli UI, mappe). L'utente lo ha aggiunto alla propria configurazione locale di Claude Code (`claude mcp add pixellab https://api.pixellab.ai/mcp -t http -H "Authorization: Bearer <chiave>"`): **la chiave sta lì e non va mai scritta nel repo o nei docs**. Gli strumenti compaiono nelle sessioni avviate dopo l'aggiunta (o dopo `/mcp`); in una sessione che non li vede si può parlare con lo stesso server via HTTP (JSON-RPC MCP, header `Authorization` letto dalla configurazione) — è quello che si è fatto per i personaggi attuali.

- **Piano**: trial con **40 generazioni** (rimaste 6 dopo l'eroe standard umanoide del 2026-09-25). Costi: personaggio standard con `create_character_pro_flash` (32x32, testo, nessuna immagine di riferimento) = 6 (5 immagine + 1 rotazioni), camminata v3 = 1 per direzione, `create_portrait_character` = 20, `create_ui_asset` = 20-40, personaggio "pro" = 20-40. Chiedere prima di spendere.
- **Dimensione**: il parametro `size` è l'altezza del personaggio in pixel e la tela viene ingrandita (28x28 per `size=20`). Con caselle da 16 px va bene **`size=20`** (~23 px di altezza, 1,4 caselle); `size=32` esce alto due caselle.
- **La pedina standard, prima versione** (2026-09-24, sostituita il 2026-09-25): l'immagine di riferimento dell'utente (`img/pawn.png`, pixel da ~17 px: in realtà 72x72) è stata riportata alla sua risoluzione nativa (28x27 px), ripulita e portata su una tela 32x32; `create_character_pro_flash` con `first_frame_base64` (l'immagine viene **conservata** e si pagano solo le rotazioni: **1 generazione** per 8 direzioni) e poi `animate_character` in modalità v3 con `action_description` (1 generazione per direzione per 8 fotogrammi, 32 px). Fotogrammi in `art-src/pawn/` (non più usati dallo script, ma lasciati sul disco).
- **L'eroe standard, versione attuale** (2026-09-25): l'utente ha chiesto di seguire lo stile di un'immagine di riferimento (`img/personaggi.png`), che si è rivelata artwork Pokémon autentico (confermato da uno screenshot di gioco, `img/full_picture.png`, nella stessa cartella). Coerente con la regola del progetto "mai arte Nintendo ripresa", **l'immagine non è stata usata come input di generazione** (né `first_frame`, né `style_image`): l'utente, messo di fronte alla scelta, ha optato per un personaggio originale descritto solo a parole. `create_character_pro_flash` (32x32, `view="low top-down"`, nessuna immagine, solo testo: avventuriero chibi con cappuccio verdeacqua) → 6 generazioni per le 8 rotazioni; `animate_character` in modalità v3, 4 direzioni, 8 fotogrammi → 4 generazioni. Fotogrammi in `art-src/hero/`. Il resto (colori, corona, copricapo) resta `tools/sprites/build_characters.py`, senza altre generazioni: ricolora spostando la **tonalità** dei pixel del cappuccio/mantello (l'unica parte nella fascia ~140-200°) verso quella della razza, lasciando intatte pelle/capelli/cintura/stivali (fascia ~15-45°).
- **I personaggi precedenti** (cavalieri e NPC umanoidi): `create_character` (humanoid, 4 direzioni, `view` "low top-down", contorno nero, ombreggiatura base) e `animate_character` con `template_animation_id` = `walking-4-frames`. Tempi: 1-4 minuti, a lavoro asincrono (massimo 8 lavori in coda). I PNG si scaricano dagli URL di `get_character` (serve un `User-Agent` da browser). I fotogrammi sono stati impacchettati in una tavola (`characters.bytes`, celle 28x28, piedi alla riga 25) con `characters-layout.json`.
- **Licenza dei risultati**: dipende dai termini di Pixellab (**non verificati**); sono personaggi originali (nessuna proprietà intellettuale di terzi nella richiesta), ma va controllato prima di distribuire il gioco.

## Asset in uso

| Pacchetto | File usati | Dove | Licenza |
|---|---|---|---|
| Kenney, Tiny Town 1.1 | `Tilemap/tilemap_packed.png` (192x176, tile 16x16; usati: erba 0-2, alberi 3-5 e 15-16, sentiero 12-14 / 24-26 / 36-38, pietra 43, staccionate 59 e 80-82, muro 126) | `unity-client/Assets/Resources/Art/kenney-tiny-town.bytes` (+ `kenney-tiny-town-license.txt`); letti da `TileSheet.cs` | CC0 |
| Pixellab (account dell'utente) | l'eroe standard: 8 rotazioni e camminata (fotogrammi sorgente in `art-src/hero/`), personaggio originale descritto a parole (nessuna immagine di riferimento in input) | `unity-client/Assets/Resources/Art/characters.bytes` e `characters-layout.json` (19 varianti costruite da `tools/sprites/build_characters.py`); letti da `CharacterSprites.cs` | termini Pixellab (da verificare) |
