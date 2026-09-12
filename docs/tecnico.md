# Documentazione Tecnica

Questo documento raccoglie le decisioni tecniche/architetturali prese durante la definizione dei requisiti. Le decisioni di gameplay sono in [design.md](design.md).

## Vincoli

### Budget: solo soluzioni open source / gratuite
Nessun costo per licenze software o tool: si useranno esclusivamente engine, framework, librerie e database open source e gratuiti.

**Aggiornamento:** per ora il gioco non verrà pubblicato sugli store, si gioca privatamente con un gruppo di amici. Questo rimuove per il momento la necessità delle fee Apple/Google — restano solo come eventualità futura se si deciderà di pubblicare pubblicamente. Eventuali costi di hosting/infrastruttura (server) sono da valutare a parte più avanti (self-hosting possibile per azzerarli in fase di sviluppo/MVP).

## Stack scelto

### Piattaforma: app mobile nativa, distribuzione privata
iOS/Android, non browser né desktop. Per ora nessuna pubblicazione su store: distribuzione privata a un gruppo di amici.

**Nota:** Android permette il sideload di un APK gratuitamente senza vincoli. iOS invece richiede comunque una forma di firma dell'app anche per uso privato (account Apple Developer gratuito con resign ogni 7 giorni, oppure Apple Developer Program a pagamento per TestFlight/durata più lunga) — da chiarire se il gruppo di amici include utenti iOS o solo Android.

### Client: Godot
Godot è stato scelto per la parte client (rendering, tilemap isometriche, input, export mobile iOS/Android). È open source (licenza MIT), completamente gratuito anche per uso commerciale, senza royalty.

**Nota architetturale:** il multiplayer built-in di Godot non è adatto a un server autoritativo persistente su larga scala — verrà usato solo come client "dumb" che comunica via WebSocket con un backend custom.

### Backend: Go
Linguaggio scelto per il server autoritativo realtime: Go. Open source, gratuito, ottime performance e concorrenza nativa (goroutine) adatte a gestire molte connessioni WebSocket simultanee con basso consumo di risorse.

## Decisioni confermate

### Scala attesa
Piccola in numero di giocatori (indicativamente 6-15 persone/squadre per mondo, gioco privato tra amici), ma potenzialmente **molto alta in numero di entità**: ogni giocatore/squadra può creare pedine senza limite (crescita libera, anche tramite riproduzione autonoma tra pedine — vedi [design.md](design.md)). Il server deve reggere un numero di pedine attive potenzialmente molto grande anche con pochi giocatori connessi.

### Pannello admin: editor visuale fin dal primo MVP
Confermato un editor visuale (non solo file di configurazione a mano) per creare/modificare mondi, board, razze, regole di creazione/riproduzione e condizioni di vittoria senza programmare — coerente con lo scope MVP "ambizioso" già scelto in design.md.

**Proposta implementativa:** un pannello admin come applicazione **web separata** (non dentro Godot/l'app mobile), che parla con lo stesso backend Go tramite API REST — più veloce da costruire di un editor visuale dentro un motore di gioco, e riutilizza il backend esistente come unica fonte di verità. Il motore di regole configurabile (vedi sotto) è ciò che l'editor va effettivamente a modificare.

### Motore di regole configurabile (priorità architetturale)
Più meccaniche di design (razze, obiettivi individuali, regole di creazione/riproduzione delle pedine, conquista territorio, condizioni di vittoria) devono essere configurabili e modificabili per mondo dall'admin, non hard-coded. Il backend Go deve trattare queste regole come **dati/configurazione per mondo** (caricati es. da database), non come logica fissa nel codice — l'editor admin scrive su questa configurazione.

### Autenticazione: account con login vero
Email/password, con gestione sicura (hashing password con bcrypt o argon2, non testo in chiaro). Serve un meccanismo di sessione/token (es. JWT) sia per le chiamate API sia per autenticare la connessione WebSocket di gioco.

### Hosting: PC personale sempre acceso
Il server gira su una macchina fisica a casa, non su un provider cloud. Per ora resta raggiungibile solo in rete locale (nessuna esposizione su internet) — vedi "Accesso da remoto" più sotto per quando servirà aprirlo agli amici. Da considerare comunque l'uptime (se il PC si spegne o la connessione cade, il gioco va offline).

### Team: sviluppo in solitaria con supporto AI
Nessun team di collaboratori previsto per ora.

### Libreria WebSocket: nhooyr.io/websocket (github.com/coder/websocket)
Scelta rispetto a gorilla/websocket (ormai in sola manutenzione) per API più moderna e pulita, supporto nativo al `context` di Go (utile per gestire in modo pulito timeout/cancellazione delle connessioni realtime) e sviluppo attivo.

### Database: PostgreSQL
Preferito a SQLite nonostante l'hosting su singolo PC personale, perché: supporta bene JSONB (utile per le regole di mondo configurabili gestite dall'editor admin), gestisce meglio scritture concorrenti (rilevante con potenzialmente moltissime pedine attive, anche se Redis assorbe la maggior parte del carico realtime — vedi sotto), e regge meglio un'eventuale crescita futura (più mondi, più admin). Gira come processo separato (es. via Docker) sulla stessa macchina del backend Go, restando comunque open source e gratuito.

### Cache realtime: Redis, da subito
Redis tiene lo stato "caldo" (posizioni, cooldown, pedine attive) in memoria per le letture/scritture ad alta frequenza del server realtime; PostgreSQL resta la fonte di verità persistente (config di mondo, account, stato "a riposo" di pedine/oggetti/territorio), aggiornato con minore frequenza. Questo riduce anche la pressione di scrittura su Postgres.

### Accesso da remoto: da definire in seguito, nessuna azione ora
Per il momento il server resta solo in rete locale per i test, senza esposizione su internet. Quando si vorrà farlo giocare agli amici da remoto, il candidato consigliato è **Cloudflare Tunnel** (gratuito, non richiede di aprire porte sul router né esporre l'IP di casa) rispetto al classico DNS dinamico + port forwarding — da rivalutare comunque al momento opportuno.
