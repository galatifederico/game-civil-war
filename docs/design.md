Sei uno Principal Software Architect ed esperto sviluppatore Rust specializzato in giochi Colony Sim / Management Sandbox sistemici, deterministici e data-driven (stile Dwarf Fortress e RimWorld).

Devi progettare e implementare in RUST un ENGINE DI SIMULAZIONE HEADLESS "Data-Driven" e completamente AGNOSTICO RISPETTO AL MONDO DI GIOCO, affinché possa essere riutilizzato per ambientazioni diverse senza dover riscrivere il codice del motore.
L'architettura deve basarsi sul pattern ECS (Entity Component System, ad esempio usando `bevy_ecs`) con uno stretto disaccoppiamento tra il motore di simulazione e i dati dell'ambientazione.

Il progetto deve essere diviso in DUE MACRO-MODULI (Crates / Moduli Rust):

===============================================================================
PARTE 1: CORE ENGINE (Framework Agnostico - `sim_core`)
===============================================================================
Implementa le seguenti strutture generiche, guidate da configurazioni esterne (JSON/RON/TOML):

1. SISTEMA ANATOMIA, SALUTE, IGIENE E GUERRA BIOLOGICA
   - AnatomySystem: Gestore di parti del corpo gerarchiche configurabili con HP ed efficienza. Mutilazioni e ferite gravi assegnano Punti Eroismo/Onore e modificatori di morale/prestazione.
   - StatusEffectSystem, PathogenEngine & MutationFramework: Framework per applicare, sintetizzare e curare buff/debuff temporanei o permanenti (patologie, vaccini, droghe sintetiche, mutazioni, transmutazioni di razza/specie, stati di alterazione).
   - Medical & Alchemy Framework: Sistema per analizzare agenti patogeni, produrre antidoti/cure, eseguire esami clinici ed effettuare inoculazioni di massa.
   - Hygiene & Environmental System: Gestione di fluidi, pulizia delle celle e propagazione di agenti/patogeni generici tramite infrastrutture.

2. UTILITY AI & JOB QUEUE FRAMEWORK
   - Engine decisionale generico basato su curve di utilità (Utility AI) e calcolo delle priorità dei bisogni.
   - Action Momentum per evitare oscillazioni dell'AI.
   - JobQueue Globale e locale per fazione/entità, con assegnazione priorità in base a gradi e ruoli. Include job generici di:
     * Costruzione, Demolizione, Coltivazione, Allevamento e Trasformazione Risorse.
     * Infiltrazione, Furto d'Identità, Sabotaggio e Assassinio Stealth.
     * Patrol, Arresto, Perquisizione e Sequestro Merci.
     * Investigazione, Ricerca Scoop e Reporting/Pubblicazione Notizie.
     * Trattamento Medico, Inoculazione Vaccinale, Magia Psichica e Transmutazione Razziale.

3. SPIONAGGIO, MUTAFORMISMO, COPERTURA E PERCEZIONE (InfiltrationEngine)
   - ShapeshiftFramework: Gestisce entità capaci di assumere forme e skin di altre razze/fazioni.
   - IdentityTheftSystem: Permette di rimpiazzare pedine esistenti e infiltrarsi nelle WorkPriorityMatrix e Squads nemiche (usato dalla Spia e dai Rettiliani).
   - Stealth & ShadowFramework: Gestisce la visibilità, l'invisibilità tattica, l'aggro reset tramite fumo e il posizionamento stealth (usato dal Ninja).
   - ExposureEngine: Gestisce la soglia di copertura (CoverLevel) e le condizioni che rivelano la vera forma delle spie, dei ninja o dei mutaforma (smascheramento tramite inchiesta del Giornalista o poteri dell'Inquisitore Vegano).

4. WORK MATRIX, SQUAD CONTROL & UI STATE API
   - WorkPriorityMatrix: Matrice di priorità dei compiti configurabile per ogni entità/pedina (stile Dwarf Fortress / RimWorld) per abilitare/disabilitare e dare priorità ai job.
   - SquadFramework: Gestore per raggruppare entità in gruppi/squadre logiche, con supporto all'invio di ordini macro e gestione dei ruoli collettivi.
   - ActivityStateAPI: Esporta lo stato e l'azione corrente di ogni pedina (azione, barra di progresso, stato emotivo) per essere facilmente consumato e mostrato nella UI del client.

5. DUNGEON, LOGISTICS IMPACT & SURFACE RAID SYSTEM
   - Spawning & Tethering System per entità confinate in strutture/dungeon sotterranei e di superficie.
   - Trigger Conditions Engine: Gestisce l'uscita delle pedine sotterranee in superficie al verificarsi di determinate condizioni di mappa.
   - StructuralDamageConsequenceEngine: Mappa i danni subiti dalle strutture chiave (es. Capannone di Babbo Natale Estivo) su eventi globali di simulazione (ritardi nelle consegne dei regali, cali di morale e picchi dei prezzi di mercato).

6. MERCATO DINAMICO, FONDO DI GILDA, STIPENDI ED EDIFICI DI TRASFORMAZIONE
   - GuildTreasurySystem & PayrollEngine: Centralizza tutti i guadagni dei membri della fazione nel Fondo di Gilda diretto dal Leader giocabile. Gestisce la matrice di configurazione degli stipendi mensili/periodici erogati alle pedine, che usano il salario personale per consumare beni autonomamente sul mercato.
   - MarketEngine: Formule dinamiche per il calcolo dei prezzi in base a Scarsità, Domanda, Offerta globale/locale e interruzioni logistiche/consegne.
   - ProcessingBuildingFramework: Gestione di strutture produttive e catene di trasformazione (Cantine, Distillatori, Birrifici, Mulini, Forni/Panifici, Macelli, Salumifici, Bar di Estratti, Cattedrali Idroponiche, Laboratori Fake Meat, Capannone Logistico Regali) che consumano e distribuiscono beni.
   - ShopFramework: Permette alle entità (o giocatori) di possedere strutture, definire un catalogo di vendita configurabile e impostare i prezzi dei beni.

7. FAZIONI, GERARCHIA, DIPLOMAZIA, GIUSTIZIA E SUCCESSIONE
   - FactionSystem & NeutralFactionEngine: Gestione di affinità bidirezionali, ideologie e matrici di relazione. Supporto per fazioni neutrali d'ordine pubblico (es. Polizia Neutra).
   - WantedLevelEngine & BribeSystem: Calcola il livello di ricercato/crimine delle pedine ed esegue arresti, perquisizioni e sequestri di contrabbando. Permette di pagare tangenti prelevate dal GuildTreasury per comprare l'impunità o azzerare il livello di criminalità.
   - HierarchySystem & SuccessionEngine: Assegnazione di gradi e ruoli militari/politici. Gestione della morte dei leader unici (es. Re Anolino) con sblocco meccanica di successione al trono; se un giocatore piazza il proprio leader sul trono ottiene una quota massiccia di Punti Vittoria e il controllo della fazione.
   - Merge & Defection Framework: Meccanica generica per l'annessione/inglobamento di fazioni alleate e diserzione autonoma degli individui (DissentLevel) scatenata da violazioni ideologiche o mancato pagamento dello stipendio. Il giocatore non può mai essere inglobato.

8. INVENTARIO E COLLEZIONI
   - InventoryComponent configurabile con limiti rigidi di slot eterogenei (es. cap massimo a 3 slot distinti per pedina) e stacking per categoria.
   - SetCollectionFramework: Generatore di bonus e punti vittoria al completamento di collezioni di oggetti registrate (comprese le Opere d'Arte e i Regali di Natale).

9. MEDIA, PRESS ENGINE & FEED SOCIAL
   - PressEngine: Gestisce la ricerca di notizie sul campo da parte dei Giornalisti, l'investigazione degli eventi di mappa e la pubblicazione di articoli (reali, di propaganda o Fake News) sul social "Il Piccione Viaggiatore", influenzando morale, reputazione e prezzi di mercato.

10. TELEMETRIA, INTERFACCIA ADMIN, MCP & CODEX
   - Telemetria (Grafana/Prometheus): Registrazione di metriche (Gauges, Counters, Histograms) tramite le crate `metrics` e `tracing` ed esposizione di un endpoint `/metrics`.
   - Admin Control Panel (Server HTTP `axum`): REST API per ispezionare l'Utility AI, bilanciare parametri in tempo reale e configurare il mapping degli Sprite/Grafiche e UI per il client.
   - MCP Server Integrato (Model Context Protocol): API JSON-RPC per consentire all'AI esterna di eseguire comandi live (`get_world_state`, `set_parameter`, `spawn_entity`, `trigger_event`).
   - CompendiumExporter: Generazione automatica di `compendium.json` partendo dalle configurazioni registrate per la Wiki in-game.

===============================================================================
PARTE 2: GAME WORLD PLUGIN (Configurazione e Dati del Mondo - `fidenza_world`)
===============================================================================
Mostra come estendere il motore `sim_core` tramite file di configurazione e registrazione di componenti/sistemi specifici per il mondo satirico di Fidenza e Salsomaggiore:

1. CONDIZIONI DI VITTORIA E RELIQUE
   - Reliquie Maggiori di San Donnino (Win Condition Primaria):
     1. Gamba Destra (Custodita dai Nani nelle gallerie sotterranee).
     2. Gamba Sinistra (Nelle viscere delle Terme di Salsomaggiore / Cripta San Vitale).
     3. Testa di San Donnino (Nelle mani del Vescovo presso la Cattedrale della Chiesa).
     4. Mano Destra (Nelle casseforti del CdA del Fidenza Village).
     5. Mano Sinistra (Posseduta dall'Overmind AI del Circolo dei Boomer).
   - Reliquie Minori: Boccale, Gonnella, Fallo, Grinder e Gladio di San Donnino.
   - Win Condition Politica Alternativa: Conquista del Trono degli Ubriaconi tramite Successione di Re Anolino.

2. RAZZE, CLASSI ED EVOLUZIONI
   - RAZZE:
     * Fidentino e Salsese (Umani base con tratti territoriali e storiche rivalità).
     * Nano (Specializzato nello scavo sotterraneo e nell'estrazione).
     * Elfo della Logistica (Specializzato nella gestione merci, imballaggio e velocità di movimento).
     * Rettiliano Mutaforma (Capace di assumere sembianze umane, infettare con malattie/droghe, rubare identità e trasformare gli umani in Maiali. Tutti i membri del CdA del Fidenza Village sono Rettiliani).
   - CLASSI:
     * Normie (Cittadino base).
     * Cavaliere & Fabbro (Combattimento da mischia e produzione armature/armi).
     * Muratore & Ingegnere Informatico / Hacker (Costruzioni strutturali e violazione sistemi/droni).
     * Agricoltore & Allevatore (Gestione di campi, idroponica, allevamento suini e bestiame).
     * Boomer (Piazzatore di Gnomi da Giardino/telecamere, diffusore di bufale).
     * Mago Vergine, Yag, Jedi/Sith (Utilizzatori di arti mistiche e forze speciali).
     * Medici / Chimici (Sintesi di farmaci, sieri, vaccini e cure per la Mutazione Porcina).
     * VEGANO (Mistico della Pianta) & POLIZIA VEGANA (Inquisitore della Soia): Poteri mistici, levitazione, telekinesi e smascheramento di aure di carne/formaggio.
     * SPIA / INFILTRATO: Furto d'identità, infiltrazione sociopolitica, avvelenamento depositi d'acqua/cibo.
     * NINJA: Attacchi acrobatici, veleni da lama, assalti silenziosi, bombe fumogene ed evasione tattica.
     * POLIZIOTTO (Fazione Neutra): Pattugliamento, perquisizioni casuali dell'inventario, sequestro contrabbando e arresti.
     * GIORNALISTA: Perlustrazione eventi, inchieste sul campo, pubblicazione articoli sul social "Il Piccione Viaggiatore" e smascheramento di Spie e Rettiliani.
   - Catene di Alterazione e Modificatori:
     * Alcolica: Brillo -> Schifoso (perdita di destrezza, aumento resistenza).
     * Sostanze: Fattone -> Super Luca (aumento percezione e velocizzazione Utility AI).
   - Statistiche Speciali & Requisiti: Baffi, Alopecia, Punti Gay, Punti Razzismo, Punti Onore, Livello Ricercato, Reputazione Stampa.

3. PEDINE DEI DUNGEON & ENTITÀ SPECIALI
   - Registra le pedine sotterranee/dungeon e le condizioni di uscita in superficie:
     * Dinosauri (Evocati dai Nani - usati come cavalcature o guardiani).
     * Robot & Droni (Evocati dall'Overmind AI dei Boomer - difesa rotonde e infrastrutture).
     * Zombie (Generati dal contagion di San Vitale).
     * Droidi R2D2, Waifu e Maid (Evocati dalla Gilda Fumetti/Nerd per supporto/difesa).
     * Leoni (Evocati dal Casinò / Diablo Tentator per la riscossione crediti).
     * Elfi della Logistica (In servizio permanente nel Capannone di Babbo Natale Estivo).

4. FAZIONI E NPC UNICI
   - FAZIONI:
     * Chiesa, Cripta di San Vitale, Nani delle Miniere, CdA Fidenza Village (Rettiliani), Anarchici del Commercio, Templari del Borgo, Contadini della Bassa, Casinò / Diablo Tentator, Circolo dei Boomer, Gilda Fumetti/Nerd, IMPERO VEGANO, FAZIONE DEGLI UBRIACONI.
     * FAZIONI SPECIALI/NEUTRE: Polizia Neutra (Comando di Fidenza) e Redazione della Gazzetta (Giornalisti).
   - NPC UNICI E LEGGENDRARI:
     * GEROLAMO BORGAZZI (Immortal NPC): Statistiche massimizzate (Alcolismo, Vandalismo, Eroismo, Umore, Carisma, Forza, Alpha Status). Autore unico delle "Opere d'Arte del Borgazzi".
     * Re Anolino (Monarca degli Ubriaconi - sblocca la meccanica di Successione al Trono).
     * Babbo Natale Estivo (Direttore logistico del Capannone dei Regali).
     * I Presidenti della Repubblica, Gabriele del Commercio, Alex Innerfire, Gran Sacerdote del Tofu, Commissario di Polizia, Direttore di Testata.
     * Boomer Overmind AI: Entità virtuale che genera Fake News sul social a partire dal Tick 0.
     * L'Oracolo: Entità con assegnazione casuale che rivela segreti di mappa.

5. PATOLOGIE, ATTREZZATURE, STRUTTURE PRODUTTIVE E DUNGEON
   - MALATTIE & PATOLOGIE: Diarrea, STD, Alopecia, Blue Balls, Mutazione Porcina (trasformazione progressiva in maiale), Carenza da B12.
   - RISORSE & STRUTTURE PRODUTTIVE:
     * Agricoltura/Allevamento: Uva, Luppolo, Orzo, Grano, Ortaggi, Soia, Maiali.
     * Strutture: Cantine, Birrifici, Distillatori, Mulini, Forni/Panifici, Macelli, Salumifici, Bar di Estratti, Cattedrali Idroponiche, Monoliti di Soia, Laboratori Fake Meat.
     * Infrastrutture Speciali: Stazione di Polizia con Celle di Detenzione, Redazione Giornalistica, Capannone Logistico Regali, Gnomi da Giardino (mini-computer spia dei Boomer per rimuovere la Fog of War).
   - CATALOGO OGGETTI & COLLEZIONI:
     * Consumabili & Cibo: Pane, Carne, Salumi, Fake Meat, Estratti di Soia, Vino, Birra, Grappa, Droghe Sintetiche, Vaccini, Sieri Mutageni, Antidoti.
     * Equipaggiamento & Armi: Shuriken, Katana, Bombe Fumogene, Manganello, Manette, Tesserino Stampa, Macchina Fotografica, Trattori, Strumenti da Cantiere, PC per Smart Working.
     * Collezionabili: Santini, Foto di Pag, Opere del Borgazzi, Regali di Natale, Corona di Re Anolino, Petardi, Canne, Preservativi.
   - FEED SOCIAL "IL PICCIONE VIAGGIATORE": Event Bus dinamico che raccoglie notizie vere dai Giornalisti e Fake News dall'Overmind AI, influenzando l'opinione pubblica.

6. INTEGRATION TEST / EXAMPLE
   - Fornisci un `main.rs` di esempio che avvia il `sim_core`, carica i dati di `fidenza_world`, esegue 100 tick di simulazione ed espone i comandi Admin, MCP e le API UI, dimostrando:
     1. Una pedina che commette un crimine aumentandosi il `WantedLevel`.
     2. Una pattuglia della Polizia Neutra che interviene, effettua una perquisizione e arresta la pedina.
     3. Il Leader che invia una tangente dal `GuildTreasury` per azzerare le accuse.
     4. Un Giornalista che assiste all'evento, scrive uno scoop su "Il Piccione Viaggiatore" e provoca una variazione nei prezzi del mercato nel `MarketEngine`.

Fornisci il codice in Rust idiomatico, modulare, ben strutturato e pronto alla compilazione.