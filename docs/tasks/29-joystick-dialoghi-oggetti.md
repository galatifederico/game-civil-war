# 29 — Joystick, dialoghi, schede e oggetti

**Fatto.**
- **Joystick** sul telefono al posto delle frecce: cerchio con una manopola da trascinare, 8 direzioni, zona
  morta al centro. Il **tasto A** è attivo quando accanto al campione c'è qualcuno o un edificio (sopra compare
  "parla con …") e apre la schermata delle azioni. Su PC lo stesso con Spazio/E/Invio, con il suggerimento in basso.
- **Menu aperto = campione fermo:** con una finestra aperta (o qualcuno che parla) joystick e A spariscono e
  frecce/WASD non muovono il campione.
- **Parla con chiunque:** `POST /api/ui/talk`, comando `player_talk`; entro 2 caselle. Le frasi sono in
  `fidenza_world/data/95_dialoghi.ron` per razza, classe e fazione (più quelle generiche per chi non ha una razza
  "propria", così gli animali grugniscono e basta); segnaposto `{name}`, `{faction}`, `{zone}`, `{player_name}`.
  La scelta varia senza toccare il generatore casuale della simulazione (le partite restano riproducibili).
  Riquadro in stile Pokémon in basso; si chiude con un tocco, A o Spazio.
- **Schermata azioni** (`GET /api/ui/interactions`): Parla, i job del campione il cui filtro accetta quel
  bersaglio (es. "Fa la spesa" su un banco del mercato), le sue abilità a distanza, Segui; per i tuoi membri anche
  gli ordini.
- **Scheda della pedina** grande come le altre finestre: vita e bisogni a barre, soldi, ricercato, dissenso,
  status, **i 3 slot** dell'inventario (icona, nome, quantità; "vuoto" se libero) e il pulsante **AZIONI**. Per gli
  edifici: integrità, magazzino a riquadri, AZIONI. Navigazione con "Indietro" (Team → scheda → oggetto).
- **Oggetti cliccabili** (inventario e slot): icona, categoria, valore, "Cosa fa" generato dagli effetti
  (`sim_core::describe`), chi ce l'ha nel team; scegli un membro e **Assegna** (`player_give_item`: lo prende dagli
  altri membri, poi dagli edifici della fazione) o **Usa** (`player_use_item`: se non ce l'ha, il team gliene passa
  uno; gli oggetti da portare addosso non si usano).
- **Icone degli oggetti** 16×16 per categoria (`tools/sprites/items.py`, 23 icone `item_<categoria>.png`); il
  client cerca prima `item_<id>`, poi la categoria, poi `item_varie`.
- Screenshot: `--touch-ui` (layout del telefono su PC), `--open=scheda|vicino|oggetto:<id>`.

**Non fatto.**
- Gli indizi nei dialoghi (previsti: andranno nelle stesse chiavi o in chiavi nuove).
- Le descrizioni a parole degli oggetti: solo 8 su 69 ne hanno una; "Cosa fa" però c'è per tutti.
- Assegnare un oggetto è istantaneo, anche da lontano; e il team può prendere merce dai negozi della fazione.
