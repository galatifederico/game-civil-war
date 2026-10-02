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

Nel client tutti i comandi sono nel menu laterale a destra, che si può ridurre a una colonna di pulsanti (▸)
o nascondere dietro "≡ Menu" (Tab). Trascinare la mappa sposta la vista, un clic seleziona; C torna al campione,
F segue la pedina selezionata, M passa tra vista ravvicinata e panoramica; "Scava" e poi trascinare un rettangolo
fa scavare la tua fazione (vale per un rettangolo). Esc annulla l'ordine o lo scavo in corso, altrimenti chiede
se uscire (Invio conferma, anche Ctrl+Q). La freccia verde sopra la testa segna le pedine della tua fazione.

### Dal telefono (Android)

```bash
sudo ufw allow 8787:8788/tcp   # una volta sola: apre le porte del server e del download dell'APK
./phone.sh                     # server sulla Wi-Fi di casa + APK del client (./phone.sh --build lo ricompila)
```

Lo script stampa l'indirizzo da cui il telefono scarica l'APK (`unity-client/Builds/Android/FidenzaClient.apk`;
se il telefono è collegato via USB con il debug attivo lo installa da solo) e quello da inserire nell'app al
primo avvio (resta salvato; il pulsante "Server" nel menu lo cambia). Sul telefono: un dito trascina la mappa,
un tocco seleziona o sceglie il bersaglio di un ordine, una pressione lunga manda lì la tua pedina selezionata,
due dita zoomano; il menu scorre col dito; il tasto indietro annulla ordine o scavo, altrimenti chiede se uscire.

Il codice è in [engine/](engine/) (vedi il suo README per comandi e struttura). Il client Unity 6 è in [unity-client/](unity-client/): avvia il server con `cd engine && cargo run --release -p fidenza_world -- --serve`, poi apri il progetto in Unity Hub e premi Play.
