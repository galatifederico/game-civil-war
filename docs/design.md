# Documento di Design

Concept originale: [main.md](main.md). Questo documento raccoglie le decisioni di design (gameplay, meccaniche, tono) prese durante la definizione dei requisiti.

## Decisioni confermate

### Ritmo di gioco: Tempo reale
Il gioco si svolge in tempo reale: le pedine possono agire quando i giocatori vogliono e i cooldown (es. velocità) scorrono in base al tempo reale, non a turni.

### Stile visivo: 2D isometrico, pixel art
La board e le pedine sono rappresentate in 2D isometrico (vista a 3/4) in stile pixel art. È una combinazione pratica per un progetto di questa portata: Unity supporta bene i tilemap isometrici 2D e il rendering pixel-perfect (filtro Point sugli sprite, nessuna sfocatura), e lo stile pixel art è più veloce da produrre/estendere nel tempo (anche con asset pack open source, se utile) rispetto a un 2D vettoriale/flat più curato.

**Da definire più avanti (non bloccante):** risoluzione base degli sprite (es. 16x16 o 32x32 px per pedina — più piccola è più veloce da produrre ma meno dettagliata) e palette colori coerente col tono satirico/urbano scelto.

### Struttura della board: griglia libera/mista
La forma della griglia non è fissa: può variare da board a board (es. alcune quadrate, altre esagonali) a seconda del tipo di board/mondo. Massima flessibilità per chi configura i mondi, a costo di maggiore complessità di design e implementazione (pathfinding e calcolo del raggio d'azione dovranno essere generici rispetto al tipo di griglia).

### Tipo di mondo: persistente
Il gioco non è organizzato in sessioni/match chiusi, ma è un mondo sempre attivo in cui i giocatori entrano ed escono liberamente, coerente con la visione "open world su una board" del concept originale. Implica stato persistente lato server, bilanciamento continuo e gestione dei giocatori inattivi (afk).

### Condizioni di vittoria: multiple e configurabili, anche per singolo giocatore
Non c'è un'unica condizione di fine/vittoria. Sono previste più modalità, anche combinabili:
- **Punteggio**: accumulo punti squadra (accumulo perenne, vedi sotto).
- **Conquista territorio**: controllo delle caselle della mappa, ottenuto costruendo una struttura sulla casella (vedi dettaglio sotto).
- **Eliminazione campione → bonus punti, non vittoria istantanea**: eliminare il campione avversario non fa vincere all'istante (coerente col fatto che la morte pedina è solo un cooldown, senza permadeath): dà invece un grosso bonus di punti/territorio, alimentando la condizione "punteggio" invece di essere una condizione a sé.
- **Obiettivo configurabile dall'admin**: ogni mondo/board può definire condizioni di vittoria personalizzate.
- **Obiettivi individuali**: oltre a quelli di squadra/mondo, ogni giocatore può avere un proprio obiettivo personale, diverso da quello degli altri giocatori.

Questo significa che il "fine partita" non è un evento unico e globale per tutti: bisognerà progettare un sistema di obiettivi/quest per entità (mondo, squadra, singolo giocatore) che possono concludersi in momenti diversi e con esiti diversi.

### Conquista territorio: tramite costruzione
Una casella diventa territorio di una squadra quando vi si costruisce una struttura specifica. Non basta la presenza temporanea né eliminare le pedine nemiche presenti: serve l'azione di costruire.

### Ruolo NPC: solo informazioni/lore
Confermato che gli NPC restano limitati a fornire informazioni/lore tramite dialogo, senza missioni strutturate né commercio.

### Scope del primo MVP: ambizioso
Il primo MVP giocabile punta fin da subito a: multi-board, configurabilità admin (regole di mondo), obiettivi individuali e sistema di riproduzione/creazione pedine completi. Non si parte da una versione ridotta a singola board/singola razza.

### Tema e tono: satirico/adulto "vita di strada"
Ambientazione urbana contemporanea con umorismo nero e riferimenti irriverenti (alcol, droghe leggere), coerente con le caratteristiche già presenti nel concept (tasso alcolemico, thc, punti alpha, beatitudine). Il tono pieno è confermato, accettando un rating 17+/adulti sugli store.

### Collegamento tra board: aree contigue di un'unica mappa
Le board sono zone diverse di un mondo continuo: il giocatore si sposta camminando da una board all'altra, senza caricamenti separati o teletrasporti tra "stanze" isolate. Rinforza la sensazione di open world.

### Morte pedina: solo cooldown, nessuna penalità aggiuntiva
Quando una pedina esaurisce la vita, rinasce dopo un cooldown. Non ci sono penalità aggiuntive (nessuna perdita di punti, oggetti o posizione): il costo della morte è unicamente il tempo di attesa. Nessun permadeath, coerente con un mondo persistente sempre attivo.

### Struttura squadra: 1 giocatore = 1 squadra
Contrariamente a quanto lasciava intendere il concept iniziale, "squadra" non indica un gruppo di più giocatori umani, ma il roster di un singolo giocatore:

`1 giocatore → 1 squadra → 1 campione + N pedine minori`

Il numero di pedine minori non è fissato all'inizio: nuove pedine possono essere create durante la partita e aggiunte alla propria squadra. Il design è pensato per supportare scala larga (potenzialmente molte decine/centinaia di pedine per giocatore nel tempo), non un piccolo party fisso.

**Implicazioni:** la "squadra" nei sistemi di punteggio/territorio/vittoria descritti sopra coincide quindi con il singolo giocatore, salvo eventuali meccaniche di alleanza ancora da definire (vedi domande aperte). Il server dovrà gestire in modo efficiente un numero di entità (pedine) potenzialmente molto alto e crescente nel tempo.

### Raggio d'azione della pedina: tutte le interazioni core
Dentro il proprio raggio d'azione, una pedina può: attaccare altre pedine, raccogliere/spostare oggetti, parlare con NPC, conquistare/influenzare la casella (territorio). Tutte e quattro le interazioni sono confermate come meccaniche core, non alternative tra loro.

### Assegnazione obiettivo individuale: configurabile dall'admin
Non è un metodo unico e fisso: l'admin di ogni mondo decide se assegnare manualmente gli obiettivi individuali oppure lasciare che il sistema li generi in modo casuale. È quindi un parametro di configurazione del mondo, non una regola di design fissa.

### Alleanze: nessuna, ognuno per sé
Non sono previste alleanze formali o informali tra giocatori: ogni squadra (= giocatore) compete individualmente contro tutte le altre nel mondo.

### Creazione di nuove pedine: multipla e configurabile
Non esiste un unico metodo di creazione: sono previste più vie, tutte configurabili/modificabili per mondo (parametro amministrabile, non regola fissa nel codice):
- **Consumo di risorse raccolte** (es. "soldi" o altri oggetti/materiali presenti sulla board).
- **Consumo di caratteristiche del campione** (es. vita, mana o altre caratteristiche del campione come costo di creazione).
- **Interazione con la board** (es. edifici/palazzi specifici generano pedine).
- **Riproduzione tra pedine**: interazione tra due pedine (es. un "maschio" e una "femmina", o combinazioni tra razze diverse) può generare una nuova pedina, in modo autonomo, richiedendo solo tempo o altre circostanze da definire.

Questo sistema di creazione deve essere pensato come **configurabile e modificabile dall'admin di ogni mondo** fin dal design: non regole hard-coded, ma un insieme di "regole di creazione" che un admin può attivare, disattivare o parametrizzare per il proprio mondo.

### Limite pedine: nessuno
Non c'è un tetto massimo al numero di pedine minori per giocatore: crescita libera nel tempo. Conferma la scala larga già indicata nella struttura squadra — il server deve reggere un numero di entità potenzialmente molto alto e in continua crescita per singolo giocatore.

### Razze/tipi di pedine: sistema aperto, definito dall'admin
Non esiste un set fisso e globale di razze. Ogni admin di mondo definisce le proprie razze/tipi di pedine e le relative compatibilità di riproduzione, in coerenza con la configurabilità già decisa per la creazione delle pedine.

### Ereditarietà delle caratteristiche: minimi per razza + componente casuale
Ogni razza definisce un set di caratteristiche minime di base. La pedina generata da riproduzione parte da questi minimi, con un valore randomico aggiunto sopra la soglia. Questi minimi (e presumibilmente il range della componente casuale) variano da razza a razza, e quindi sono anch'essi parte della configurazione di mondo gestita dall'admin.

### Trigger riproduzione: azione esplicita del giocatore
La riproduzione tra due pedine compatibili non è automatica né temporizzata: richiede un comando/azione esplicita del giocatore.

### Classifica: accumulo perenne
I punti squadra si accumulano nel tempo senza reset periodici, coerente con il mondo persistente senza sessioni chiuse.

### Caratteristiche: stesso set per tutte le razze
Tutte le pedine, indipendentemente dalla razza/tipo, condividono lo stesso set di caratteristiche (velocità, vita, vista, soldi, tasso alcolemico, punti alpha, thc, beatitudine, mana, forza...). Ciò che varia da razza a razza sono i valori/minimi (vedi ereditarietà), non l'insieme di caratteristiche stesso.

### Inventario condiviso: gestito solo dal campione
Solo il campione può gestire/usare l'inventario condiviso della squadra. Le pedine minori possono raccogliere oggetti ma non amministrarli direttamente. Il trasferimento all'inventario condiviso avviene però in modo automatico/istantaneo alla raccolta, indipendentemente dalla distanza dal campione: non serve portare fisicamente l'oggetto vicino al campione.

### Raggio d'azione: dipende solo dalle caratteristiche della pedina stessa
A differenza di quanto ipotizzato inizialmente nel concept, il raggio d'azione di una pedina dipende esclusivamente dalle sue caratteristiche proprie (es. vista), non da quelle delle pedine circostanti (né alleate né nemiche). Nessun effetto di "supporto di gruppo" o "pressione" da pedine vicine.

### Caratteristica "vista": visibilità + raggio d'azione
La caratteristica "vista" governa insieme sia la visibilità della pedina sulla mappa (fog of war: quanto lontano il giocatore vede) sia il raggio d'azione/interazione (quanto lontano può agire), non sono due valori separati.

### Ruolo admin: può anche giocare nel proprio mondo
Chi amministra un mondo (definendone razze, obiettivi, regole di creazione/riproduzione, condizioni di vittoria) può anche avere una propria squadra e giocare insieme agli amici in quello stesso mondo. Non è un ruolo separato e non giocabile.

### Scala iniziale: piccolo gruppo di amici
Il gruppo iniziale previsto è piccolo (indicativamente 6-15 persone/squadre per mondo), utile per dimensionare l'MVP e l'infrastruttura self-hosted.

### Campione: abilità/azioni esclusive
Il campione non è solo una pedina con statistiche più alte: ha abilità/azioni che le pedine minori non possono compiere (oltre a gestire l'inventario condiviso e contare per il bonus punti da eliminazione). Il set esatto di queste abilità è materia di game design dei contenuti, da definire più avanti (non blocca l'architettura).

### Accesso al mondo: lista mondi disponibili
Un giocatore entra nel mondo di un amico tramite una lista/lobby dei mondi esistenti a cui può unirsi, non tramite codice di invito privato né indirizzo server manuale.

### Punteggio: ogni pedina contribuisce al punteggio della propria squadra
Confermato che non esistono squadre composte da più giocatori. I punti restano sempre "di un solo giocatore" (la sua squadra) e non sono mai condivisi con altri giocatori. All'interno della squadra, i punti guadagnati dalle azioni delle singole pedine (comprese quelle minori, non solo il campione) si sommano al punteggio totale della squadra — coerente con quanto già indicato nel concept originale ("ogni azione di ogni pedina può comportare un cambiamento dei punti della squadra").

## Domande aperte

### Configurabilità admin
- Dato che razze, ereditarietà, obiettivi individuali, regole di creazione/riproduzione, conquista territorio e condizioni di vittoria devono essere tutte configurabili per mondo, serve un sistema di regole/editor lato admin pensato fin dall'inizio dell'architettura (non solo un pannello a posteriori): riportato come requisito tecnico prioritario in [tecnico.md](tecnico.md).

### Contenuti (non bloccante per l'architettura)
- Set esatto di abilità esclusive del campione.
- Significato/effetto meccanico di ogni caratteristica (es. cosa fanno concretamente "punti alpha" e "beatitudine" in gioco).
- Tipologie di oggetti presenti sulla board e loro effetti.
