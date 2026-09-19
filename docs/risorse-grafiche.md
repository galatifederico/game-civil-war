# Risorse grafiche

Materiale che si può usare per migliorare la grafica. Raccolto il 2026-09-20 su indicazione dell'utente; ciò che è scritto qui **è stato verificato leggendo le pagine e, per Kenney, scaricando i pacchetti** (le voci non verificate sono dichiarate come tali). Oggi tutta la grafica del client è disegnata in codice (`PixelArt.cs`), senza asset importati.

## Regole d'uso

- **Si possono usare senza chiedere** le risorse **CC0** (pubblico dominio): uso commerciale, modifiche e ridistribuzione consentiti, attribuzione non richiesta. Qui lo sono Kenney e Kitbitz.
- Un pacchetto va **in una cartella sua**, `unity-client/Assets/Art/<autore>/<pacchetto>/`, insieme al suo file di licenza (`License.txt`). Si tengono nel repo **solo i file che il gioco usa** (i pacchetti interi pesano decine di MB; il pacchetto isometrico di Kenney è 14 MB).
- Tutto ciò che viene da fuori va elencato nella tabella "Asset in uso" in fondo a questo file (pacchetto, file usati, dove, licenza).
- Ciò che **non** è CC0 (per esempio immagini generate da un modello AI con i suoi termini) va controllato prima di finire nel repo.
- Le tre risorse esterne sotto sono tutte gratuite, ma **scaricare** un file da un sito è comunque una richiesta di rete: nessuno degli strumenti qui dentro richiede account per i file CC0.

## Cosa deve poter mostrare il client (vincoli)

- Vista **isometrica 2:1**: casella quadrata = rombo di 32 px di larghezza (`PixelArt.PixelsPerUnit = 32`, `GridMath.TileWidth = 1`), casella **esagonale** = esagono schiacciato con lo stesso rapporto. Un pacchetto senza esagoni non copre le board `hex` (l'Alveare del mondo di prova).
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

- **Non c'è un pacchetto isometrico in pixel art a 32 px** tra quelli visti: gli isometrici di Kenney sono in stile 3D/cartoon ad alta risoluzione (256 o 111 px per tile), quelli in pixel art sono dall'alto. Mescolarli con la grafica attuale (pixel art isometrica 32 px) darebbe due stili nella stessa scena.
- Non ho trovato in Kenney esagoni isometrici 2D (i pacchetti con "hex" che conosco sono 3D: da verificare prima di contarci).
- I pacchetti si scaricano come ZIP dalla pagina del pacchetto (link "Download"); i pochi file che servono si copiano nella cartella del progetto.

## 2. Kitbitz — https://kitbitz.art (CC0)

Libreria di **2.043 illustrazioni disegnate a mano** (dichiarato: non generate da AI) in 13 "kit": Medieval (341), Nature (309), Interior (281), Space (170), Cyberpunk (157), Pirate (145), Halloween (133), City (115), Ruins (103), Western (84), Winter (79), Dungeon (74), Barbieland (52). Formati **SVG e PNG** (le dimensioni sono nel catalogo, per esempio 301x293), quasi tutti "oggetti" singoli (piante, mobili, casse, insegne...). Licenza **CC0 1.0** per le illustrazioni; il sito, il plugin Figma e il codice del server MCP non lo sono.

- **Catalogo JSON completo**: https://kitbitz.art/catalog.v1.json (3,6 MB; per ogni asset `assetId`, nome, kit, categoria, tag, descrizione e URL di SVG/PNG su `assets.kitbitz.art`). Sorgente degli asset: repository `CaptExcellent/kits-library-assets`.
- **Server MCP** pubblico, in sola lettura, senza autenticazione: `https://mcp.kitbitz.art` (Streamable HTTP; stesso servizio di `https://kitbitz.art/api/mcp`). Strumenti: `search_illustrations`, `get_illustration`, `find_related_illustrations`, `curate_scene`, `prepare_asset_pack` (manifest di download). Non scrive file sul disco: dà URL e metadati. Documentazione: https://kitbitz.art/docs/mcp.
- **Non è collegato a questa sessione.** Per aggiungerlo in Claude Code: `claude mcp add --transport http kitbitz https://mcp.kitbitz.art` (comando standard per i server MCP HTTP; non provato qui). Senza MCP si può comunque usare il catalogo JSON con uno script.
- **Stile**: illustrazioni vettoriali a mano libera, non pixel art e non isometriche. Adatte a icone degli oggetti in grande (inventario, dettaglio oggetto), sfondi/illustrazioni della lobby e del login, ritratti; poco adatte al terreno isometrico.
- Il server dice di essere "in sviluppo attivo": non dipenderne per la build (scaricare i file che servono e salvarli nel repo).

## 3. Tiled — https://www.mapeditor.org (open source)

Editor di mappe a caselle, gratuito. L'editor è sotto **GPL-2.0** (con parti BSD/Apache nel repository); le mappe che si producono sono dell'autore. Gestisce mappe **ortogonali, isometriche ed esagonali** (le tre griglie che ci servono), livelli di caselle e di oggetti, automapping, ed esporta in **TMX** (formato nativo), **JSON**, Lua e altri.

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
| Terreno delle caselle (erba, pietra, acqua...) | `isometric-blocks` o `isometric-miniature-*` di Kenney | serve il dato terreno per casella e ridimensionare/riprogettare le caselle (oggi 32 px) |
| Icone degli oggetti | `board-game-icons` / `game-icons` / `tiny-dungeon` di Kenney, oppure Kitbitz (Medieval/Dungeon) | oggi sono 7 icone in pixel art in codice (`PixelArt.ItemIcon`); l'icona è un dato per oggetto (`board_items.icon`) |
| Pedine, NPC | sprite-sheet-creator (originali) oppure i personaggi di `isometric-miniature-dungeon` | le pedine sono tinte per squadra: servono sprite in grigi |
| Interfaccia (pannelli, pulsanti) | `pixel-ui-pack` / `ui-pack-pixel-adventure` di Kenney | oggi l'interfaccia è IMGUI con lo stile predefinito di Unity |
| Illustrazioni per lobby e login | Kitbitz | vettoriali, CC0 |
| Disegnare mappe a mano | Tiled | dopo il dato terreno |

## Asset in uso

Nessuno per ora (la grafica è ancora tutta disegnata in codice).

| Pacchetto | File usati | Dove | Licenza |
|---|---|---|---|
