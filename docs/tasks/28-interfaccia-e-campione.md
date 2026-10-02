# 28 — Nuova interfaccia e campione mosso a mano

**Fatto.**
- **In alto al centro, sempre visibili:** punti vittoria (gli stessi che contano per vincere: punti + reliquie
  possedute), fondo di gilda, numero di membri del team; sotto, giorno e ora.
- **A destra:** Mappa (panoramica della superficie con quanti dei tuoi ci sono in ogni area, elenco delle mappe
  con i tuoi e delle aree occupate, "Torna dal campione"), Inventario (oggetti del team: addosso ai membri e negli
  edifici della fazione, per categoria), Team (in cima il riepilogo: fondo, punti, soldi in tasca, umore e salute
  medi, quartieri; poi ogni membro con razza, classe, vita, umore, soldi, attività; sotto influenza, squadre,
  stipendi e priorità di lavoro), Opzioni (tempo, vista, nebbia, scavo, salva/carica, server, esci).
- **A sinistra:** Piccione Viaggiatore (canale Principale + Su di te + le categorie + Tutto, contatore rosso delle
  non lette del Principale, ricordato sul dispositivo), Classifica ufficiale, Risorse mondiali (indice dei prezzi,
  inflazione delle ultime 24 ore, moneta in circolazione, circostanze, quota prodotta in zona, merci), Manuale.
- **Un pulsante sempre visibile** (in alto a sinistra, o Tab) nasconde tutto e lo fa ricomparire.
- **Scheda della pedina** in basso (tocco o clic, oppure A/Spazio davanti al campione), con gli ordini se è tua e
  con quello che il campione può farle se non lo è.
- **Campione stile Pokémon:** frecce/WASD o croce sul telefono, una casella per pressione, tenendo premuto cammina;
  il passo è immediato (`POST /api/ui/step`, comando `player_step`), anche in pausa; porte e scale si attraversano
  camminandoci sopra; niente tagli d'angolo. Dopo un passo a mano la sua AI resta ferma per
  `player.manual_hold_ticks` (24 tick), poi torna a badare ai bisogni.
- **Canale principale:** notizie sulla fazione del giocatore con importanza ≥ `press.main_mine_threshold` (0,5) e
  notizie qualsiasi con importanza ≥ `press.main_world_threshold` (0,9). Su 300 tick: 15 notizie contro le 34 di
  "Importanti" e le 174 totali.
- Server: `/api/ui/player` con membri arricchiti, `summary` e `inventory`; `/api/ui/economy`; filtro `main`.
- Screenshot automatici: `--open=team|mappa|…`, `--walk=RRDD`, `--select-near`.

**Non fatto.**
- Un'azione "Parla" vera e propria (vedi domanda 20): oggi davanti a una pedina si apre la sua scheda con le azioni
  del campione su di lei (attacca, segui, scippa…), ma nessuna è un dialogo.
- Il client web (`/ui/`) ha ancora la vecchia interfaccia.
- La croce sul telefono non è provata su un dispositivo vero (solo build Linux con screenshot).
