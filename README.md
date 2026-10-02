# The Game — engine di simulazione

Engine di simulazione headless, deterministico e data-driven in Rust (ECS con `bevy_ecs`), stile colony sim
(Dwarf Fortress, RimWorld), più il plugin dell'ambientazione satirica di Fidenza e Salsomaggiore.

- Design: [docs/design.md](docs/design.md)
- Piano e stato delle task: [docs/tasks/00-piano.md](docs/tasks/00-piano.md)
- Decisioni prese in autonomia da rivedere: [docs/questions/](docs/questions/)

## Avvio

```bash
./run.sh            # compila e avvia il server, poi apre il client Unity già compilato (senza aprire l'editor)
./run.sh --build    # come sopra ma ricompila prima il client Unity (l'editor deve essere chiuso)
./run.sh --web      # niente Unity: apre il client web nel browser su http://127.0.0.1:8787/ui/
```

I log finiscono in `logs/`. Chiudendo la finestra del client il server si ferma.

Con il server acceso la console di amministrazione è su http://127.0.0.1:8787/admin/ (mappa, entità, oggetti,
contenuti, sprite, parametri, fazioni e quartieri, eventi, comandi).

Nel client, in alto al centro ci sono sempre i punti vittoria, il fondo di gilda e quanti siete nel team. A destra:
Mappa (M, la panoramica del mondo con quanti dei tuoi ci sono in ogni area), Inventario (I, cosa ha raccolto il
team), Team (T, il riepilogo del team e poi ogni membro con razza, classe, vita, umore, soldi e attività) e Opzioni
(O: tempo, vista, scavo, salvataggi, nebbia, server, esci). L'inventario ha anche le Collezioni (Opere del
Borgazzi, Foto di Pag, Santini…) con i pezzi posseduti e quelli che mancano. A sinistra: il Piccione Viaggiatore (P, i canali delle
notizie; il numero rosso conta le non lette del canale principale), la Classifica ufficiale (L), le Risorse mondiali
(R: prezzi, inflazione, moneta, circostanze) e il Manuale (H). Il pulsante in alto a sinistra (o Tab) nasconde e fa
ricomparire tutto.

Il campione si muove a mano, una casella alla volta come in Pokémon: frecce o WASD (in panoramica spostano la
vista); porte e scale si attraversano camminandoci sopra. Quando gli sta accanto qualcuno, Spazio (o E) apre le
azioni possibili, a cominciare da Parla. Un clic su una pedina o un edificio apre la sua scheda (vita, bisogni, i
tre slot degli oggetti, AZIONI); un clic su un oggetto ne mostra il dettaglio, da cui assegnarlo o usarlo. Con una
finestra aperta il campione sta fermo. Trascinare la mappa sposta la vista, un clic seleziona; C torna al campione,
F segue la pedina selezionata; "Scava" (nelle Opzioni) e poi trascinare un rettangolo fa scavare la tua fazione.
Esc chiude la finestra o la scheda aperta e annulla l'ordine o lo scavo in corso, altrimenti chiede se uscire
(Invio conferma, anche Ctrl+Q). La freccia verde sopra la testa segna le pedine della tua fazione.

### Dal telefono (Android)

```bash
sudo ufw allow 8787:8788/tcp   # una volta sola: apre le porte del server e del download dell'APK
./phone.sh                     # server sulla Wi-Fi di casa + APK del client (./phone.sh --build lo ricompila)
```

Lo script stampa l'indirizzo da cui il telefono scarica l'APK (`unity-client/Builds/Android/FidenzaClient.apk`;
se il telefono è collegato via USB con il debug attivo lo installa da solo) e quello da inserire nell'app al
primo avvio (resta salvato; il pulsante "Server" nelle Opzioni lo cambia). Sul telefono: la croce in basso a
sinistra è un joystick che muove il campione e il tasto A apre le azioni con chi gli sta accanto (con una finestra
aperta spariscono); un dito trascina la mappa, un tocco
seleziona o sceglie il bersaglio di un ordine, una pressione lunga manda lì la tua pedina selezionata, due dita
zoomano; finestre e schede scorrono col dito; il tasto indietro chiude o annulla, altrimenti chiede se uscire.

Il codice è in [engine/](engine/) (vedi il suo README per comandi e struttura). Il client Unity 6 è in [unity-client/](unity-client/): avvia il server con `cd engine && cargo run --release -p fidenza_world -- --serve`, poi apri il progetto in Unity Hub e premi Play.
