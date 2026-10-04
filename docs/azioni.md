# Caratteristiche delle pedine
- Principali:
    - vita: 1000 -> 0
    - intelligenza: 0 -> 200
    - percezione: 0 -> 100
    - età: 0 -> 100
    - resistenza: 0 -> 100
    - Forza: 0 -> 100
    - Difesa: 0 -> 100
- Fisiche:
    - vista: 0 -> 100
    - velocità: 0 -> 100
    - muscoli: 0 -> 100
    - grasso: 0 -> 100
    - baffi: booleano
    - bellezza: 0 -> 100

- conoscenza: 0 -> 100
    - informatica
    - religione
    - lore nerd
    - alcol and beverage
    - medicina e chimica
    - magia nera
    - magia bianca
    - agricoltura
    - arti marziali
    - scherma
    - lavori manuali
    - cucina
    - storia del team
- Abilità:
    - furtività: 0 -> 100
    - mira: 0 -> 100
- Stato fisico
    - alcol: 0 -> 100
    - thc: 0 -> 100
    - pulizia: 100 -> 0
    - stanchezza: 0 -> 100
    - stress: 0 -> 100
    - divertimento: 0 -> 100
    - Malattie: lista (eventualmente vuota) di malattie
    - Per ogni parte del corpo:
        - sano: 100 -> 0
        - malattia: lista (eventualmente vuota) di malattie
        Le parti del corpo sono: fegato, cazzo o vagina, gamba destra, gamba sinistra, braccio destro, braccio sinista, mano destra, mano sinistra, le 10 dita dell emani, i due piedi, le 10 dita dei piedi, occhio dx, occhio sx, lingua, denti: 32, cappeli (100 -> 0 fino a diventare calvo), orecchie, intestino
- Spirituali:
    - santità: 0 -> 100
    - fortuna: 0 -> 100
    - iniziato: booleano per ogni religione
        - cristianesimo
        - sanatismo (san vitale)
- Personalità:
    - punti gay: 0 -> 100
    - punti alpha: 0 -> 100
    - curiosità: 0 -> 100
    - aggressività: 0 -> 100
    - discrimazione: 0 -> 100
        - una barra per ogni ra 
    - fanatismo: 0 -> 100
        - verso il leader
        - verso i fumetti
        - verso la chiesa
        - verso san vitale
        - verso il sesso
- Sessuali:
    - fertilità: 0 -> 100
    - capacità sessuale: 0 -> 100
    - verginità: booleano
    - orientamento sessuale: lista di tuple (razza, sesso)
- Speciali:
    - forza (star wars): 0 -> 100
    - mana: 0 -> 100
- Affiliazioni:
    - una per gruppo, booleano per dire se si fa parte di quella setta/fazione (es. anarchici, chiesa, massoni etc...)
- Caratteristiche tra personaggi singoli:
    - amicizia: 0 -> 100 (aumenta con attività fatte insieme)
    - attrazione: 0 -> 100

## Caratteristiche aggiunte
- Sociali:
    - soldi: 0 -> infinito
    - carisma: 0 -> 100
    - eloquenza: 0 -> 100 (convincere, predicare, vendere fumo)
    - reputazione: -100 -> 100 (cosa pensa di te il Borgo)
    - notorietà: 0 -> 100 (quanto ti conoscono, nel bene o nel male)
    - onore: 0 -> 100
    - fedina penale: lista di reati (furto, spaccio, vandalismo, incendio, rissa, ...)
    - ricercato: booleano
- Mentali:
    - sanità mentale: 100 -> 0
    - malinconia: 0 -> 100
    - creatività: 0 -> 100
    - pazienza: 0 -> 100
    - paranoia: 0 -> 100
- Fisiche:
    - fame: 0 -> 100
    - sete: 0 -> 100
    - puzza: 0 -> 100 (sale se pulizia è bassa, si sente a 2 caselle)
    - tatuaggi: numero
    - cicatrici: numero
    - tolleranza (all'alcol e al thc): 0 -> 100, sale bevendo e fumando
- Conoscenze aggiunte (0 -> 100):
    - teologia di San Vitale
    - norcineria (culatello, salame, spalla)
    - vinificazione (lambrusco, fortana)
    - edilizia (serve all'Umarell per criticare con cognizione)
    - giornalismo
    - legge
    - giochi d'azzardo
- Contatori (statistiche di carriera, non scendono mai): servono per i requisiti delle classi
    - risse vinte / perse
    - bevute totali
    - canne fumate
    - ore di studio per materia
    - ore di preghiera
    - ore passate a guardare cantieri
    - partner sessuali
    - oggetti rubati
    - incendi appiccati
    - graffiti fatti
    - persone curate
    - persone uccise
    - articoli pubblicati
    - ore ai videogiochi
    - vincite / perdite al Casinò
    - giorni senza lavarsi (consecutivi)
    - giorni da vergine (consecutivi)

# Azioni
- Andare a bere:
    requisti:
    - 
    effetto:
    + alcol
    + divertimento
    - stress
    + aggressività
    + malattia al fegato
    - soldi
    + punti alpha
    + grasso
    - percezione
- Andare a lavorare
    + soldi
    + stress
    + stanchezza
    -> ovviamente in base al lavoro possono succedere cose o puoi andare a lavorare un determinato posto, ma questo poi lo specifichiamo dopo)
- Studiare (una materia):
    + conoscenza in quella materia
    + stanchezza
    + stress
- Dormire
    - stress
    - stanchezza
    + salute
- Gioco d'azzardo:
    + divertimento (se vinci)
    + stress (se perdi)
    + o - soldi
- mangiare:
    + salute
    + grasso
    - stress
- Allenarsi:
    + muscoli
    - stress
    - grasso
- Fumare:
    - stress
    + percezione
- Pregare:
    + punti santità
    - stress
    + salute
- Rissa:
    + divertimento
    - salute (probabilmente)
    - stress
    + punti alpha (se vinci)
- Sesso:
    + punti alpha o punti gay a seconda del genere del partner
    + se la razza lo permette, possibilità di concepire (maschio e femmina per tutti, a parte i rettiliani che sono asessuati e possono riprodursi con tutti gli altri rettiliani. Salsesi, fidentini e nani possono riprodursi tra di loro) (percentuale ridotta se si usa anticoncezionale)
    + divertimento
    - stress
    + possibilità di malattia venerea se uno dei due è sporco
- Comprare qualcosa se sono vicino ad una attività commerciale
- Spacciare:
    + soldi
    - erba
    + possibilità di essere arrestato
- vandalismo:--> danni alla vittima sia oggetto che persona
    + divertimento
    - stress
    + oggetti in giro si rompono o prendono fuoco
- dare fuoco a qualcosa/qualcuno --> danni alla vittima sia oggetto che persona
- graffiti: puoi scrivere un messaggio su un muro e gli altri possono leggerlo
- pisciare: --> danni alla vittima sia oggetto che persona
- farsi crescere i baffi: 
    + punti alpha
    + punti gay
- coltivare erba
    + dopo un po' di tempo più erba nell'inventario
- rubare
- creare canne (azione sull'oggetto, posso creare una canna, una LUCA o una SUPER LUCA)
- giocare ai videogiochi:
    + divertimento
    - stress
    - igene
    + grasso
+ passa uno spinello (azione su qualcuno):
    + vita
    + thc
    + percezione
    - stress
- usa la forza (starwars):
    + danni all'avversario
    + danno a oggetti a distanza
- meditazione nella forza (starwars):
    + percezione
+ colpo con arma
+ lancia (oggetto)
+ prega per qualcuno
    + vita
    - stress
    - riposo
    + raramente può guarire una malattia, deve avere moltissimi punti santità
- pubblicizzare la propria fazione
    + fanatismo
- diffamare una fazione avversaria/razza
    + fanatismo
    + odio verso la fazione avversaria
- hackerare il piccione viaggiatore:
    - pubblica una notizia su un canale a scelta
- curare
    + vita
    - malattie
    - stress
    + riposo
    + igiene
- andare alle terme
    + igiene
    - stress
    + riposo
    + vita
    - soldi
    + divertimento
- guardare i cantieri
    + vita
    - stress
    + divertimento
- dare fuoco a qualcosa
- acquista oggetto
- raggio gay
    + punti gay alla vittima e all'utilizzatore
    + danni all'avversario
- fare la talpa
    + si fa finta di far parte di una fazione nemica, si seguono i suoi comandi ma si mantiene il controllo della pedina e la si può richiamare in qualsiasi momento
- abilità ninja
    + diventare invisibili?
+ esplorare:
    + vai ad esplorare la mappa e a scoprire posti nuovi e nuovi dungeon
- mutaforma:
    + prendi le sembianze di qualcuno
- diffondere malattia venerea
    + c'è la possibilità di attaccare una malattia venera senza rapporti
- trasformazione in maiale
    - la vittima viene trasformata in maiale per un tempo determinato
- palla di fuoco
    - magia del mago che infligge danni
- cura gli amici
    - magia del mago per curare i compagni

## Azioni aggiunte
- lavarsi:
    + pulizia
    - puzza
    - stress
- cucinare (consuma ingredienti):
    + piatto nell'inventario (qualità in base a cucina)
    + conoscenza cucina
    + divertimento
- fare il culatello / norcineria (serve un maiale, vivo o meno):
    + salumi nell'inventario
    + conoscenza norcineria
    - reputazione presso l'Impero Vegano
- vendemmiare / fare il lambrusco:
    + vino nell'inventario dopo un po' di tempo
    + conoscenza vinificazione
- digiunare:
    + santità
    - grasso
    + fame
    + stress
- confessarsi (serve un prete):
    - stress
    + santità
    il prete viene a sapere i tuoi reati, e un prete corrotto può rivenderli
- predicare (in piazza):
    + fanatismo negli ascoltatori
    + eloquenza
    + possibilità di convertire qualcuno
- esorcismo (su qualcuno):
    - malattie mistiche, possessioni, maledizioni
    - stress dell'esorcista + stanchezza
    rischio: se fallisce il demone passa all'esorcista
- rito di magia nera:
    + mana
    + maledizione su un bersaglio
    - santità
    - sanità mentale
- maledire (su qualcuno):
    - fortuna alla vittima
    + paranoia alla vittima
- benedire (su qualcuno):
    + fortuna alla vittima
    - stress alla vittima
- corteggiare (su qualcuno):
    + attrazione (in base a bellezza, carisma, baffi, puzza)
    rischio di due di picche: + stress, - punti alpha
- insultare (su qualcuno):
    - amicizia
    + aggressività della vittima
    + punti alpha se il pubblico ride
- raccontare una barzelletta:
    + divertimento a chi ascolta
    + amicizia
    rischio: se è una barzelletta da Boomer, - divertimento
- pedinare (su qualcuno):
    + vieni a sapere dove va e con chi parla
    + furtività
- origliare:
    + scopri le ultime chiacchiere (diventano notizie o ricatti)
- ricattare (serve un segreto sulla vittima):
    + soldi
    - amicizia
    + reato in fedina penale
- arrestare (solo forze dell'ordine):
    la vittima finisce in cella per un tempo pari ai reati
- multare (solo forze dell'ordine):
    - soldi alla vittima
    + soldi al comune
    - reputazione del poliziotto
- scrivere un articolo:
    + notizia sul Piccione Viaggiatore
    + conoscenza giornalismo
    + notorietà
- fare un tatuaggio (a sé o a qualcuno, serve un tatuatore):
    + tatuaggi
    + punti alpha
    + possibilità di infezione
- rasarsi / radersi i baffi:
    - baffi
    - punti alpha
    + bellezza (forse)
- fare jogging:
    - grasso
    + velocità
    + resistenza
    + stanchezza
- leggere fumetti:
    + lore nerd
    + divertimento
    + fanatismo verso i fumetti
- fare cosplay (serve un costume):
    + divertimento
    + notorietà
    + punti gay o alpha a seconda del costume
- giocare di ruolo (D&D, almeno 3 pedine al tavolo):
    + lore nerd
    + amicizia tra i giocatori
    + divertimento
    - igiene
- organizzare un rave:
    + divertimento a tutti i presenti
    + thc, alcol
    + notorietà
    + possibilità di intervento della polizia
- chiedere l'elemosina:
    + pochi soldi
    - reputazione
- scavare:
    + materiali
    + muscoli
    + possibilità di trovare un dungeon o un dinosauro
- riparare (robot, droni, oggetti):
    + vita della macchina / dell'oggetto
    + lavori manuali
- avvelenare (cibo o bevanda):
    + malattia o danni a chi la consuma
    + reato
- seppellire un cadavere:
    - prove di un omicidio
    + stanchezza
- resuscitare (magia nera, serve un cadavere):
    + uno zombie fedele a chi l'ha evocato
    - santità
    - sanità mentale
- fare un selfie / postare una foto:
    + notorietà
    + divertimento
    + bellezza percepita dagli altri (filtri)
- sfida a duello (su qualcuno):
    combattimento uno contro uno con regole; chi rifiuta perde onore
- guardare la partita al bar:
    + divertimento o stress in base al risultato
    + amicizia con chi tifa la stessa squadra
    + aggressività verso chi tifa l'altra

# Classi

## Regole generali
- Una pedina ha sempre una classe (Normie di default) e può averne una sola alla volta.
  Quando soddisfa i requisiti di un'altra classe le viene proposto il cambio (il campione sceglie, le pedine del team decidono da sole in base alla personalità).
- I requisiti si scrivono come condizioni sulle caratteristiche: `grasso >= 80 E muscoli < 30`, `santità >= 70 O fanatismo.chiesa >= 90`.
  Si possono usare anche: razza (o razze escluse), sesso, affiliazione, oggetti posseduti, contatori di carriera, iniziazioni, età.
- Mantenimento: se un requisito numerico scende sotto il 90% della soglia si perde la classe e si torna Normie (es. Ciccione si acquisisce a 80 di grasso e si perde sotto 72). Così non si cambia classe ogni tick.
- Alcune classi richiedono un "rito di passaggio": un'azione speciale da fare una volta (es. Templare: giuramento in chiesa). Le definiamo dopo.
- Le classi "di razza" (Bestia, Guardiano meccanico, Non morto) non si acquisiscono: vengono con la razza.

## Classi di base e di lavoro
- Normie
    requisiti: nessuno (classe di partenza)
    sblocca: tutti i lavori di base
- Agricoltore
    requisiti: agricoltura >= 40 E resistenza >= 30
    sblocca: coltivare (doppio raccolto), coltivare erba (raccolto migliore)
- Allevatore
    requisiti: agricoltura >= 30 E muscoli >= 30 E puzza >= 20
    sblocca: allevare, mungere, castrare il maiale
- Norcino (nuova)
    requisiti: norcineria >= 50 E muscoli >= 40 E contatore maiali macellati >= 5
    sblocca: fare il culatello (qualità migliore), taglio del prosciutto (+ amicizia a chi lo mangia)
    note: odiato dall'Impero Vegano, la Polizia Vegana lo smaschera subito
- Muratore
    requisiti: lavori manuali >= 40 E muscoli >= 40
    sblocca: costruire, riparare edifici
- Fabbro
    requisiti: lavori manuali >= 50 E muscoli >= 50 E resistenza >= 40
    sblocca: forgiare armi e armature
- Minatore
    requisiti: muscoli >= 40 E resistenza >= 50; i Nani lo sono di diritto
    sblocca: scavare (più materiali, più probabilità di dungeon)
- Addetto Logistica
    requisiti: velocità >= 40 E resistenza >= 40 E stress >= 50 (sì, serve)
    sblocca: consegne veloci, imballaggio
- Chef (nuova)
    requisiti: cucina >= 60 E grasso >= 30 E creatività >= 40
    sblocca: cucinare piatti con effetti (anolini: - stress a tutti; torta fritta: + grasso e + divertimento)
- Medico
    requisiti: medicina e chimica >= 60 E intelligenza >= 100
    sblocca: curare (più efficace), diagnosi, vaccinare
- Chimico
    requisiti: medicina e chimica >= 70 E intelligenza >= 120
    sblocca: sintetizzare farmaci, sieri, veleni
- Hacker
    requisiti: informatica >= 70 E intelligenza >= 100
    sblocca: hackerare il piccione viaggiatore, hackerare droni e robot
- Giornalista
    requisiti: giornalismo >= 40 E percezione >= 50 E curiosità >= 60
    sblocca: scrivere un articolo, inchiesta, origliare (più efficace)
- Influencer (nuova)
    requisiti: bellezza >= 70 E notorietà >= 50 E contatore selfie >= 30
    sblocca: fare un selfie (+ notorietà enorme), sponsorizzare un negozio (+ soldi)
- Tatuatore (nuova)
    requisiti: creatività >= 60 E tatuaggi >= 5 E mano destra sana >= 80
    sblocca: fare un tatuaggio agli altri

## Classi fisiche
- Ciccione (nuova)
    requisiti: grasso >= 80
    sblocca: Schiacciata (si siede sull'avversario: danni e lo immobilizza), Mangiata Colossale (svuota un buffet, - stress)
    malus: - velocità, + rischio malattie al fegato e al cuore
- Palestrato (nuova)
    requisiti: muscoli >= 80 E grasso <= 20 E contatore allenamenti >= 50
    sblocca: Sollevamento (lancia oggetti pesanti e persone), Posa Plastica (+ attrazione a chi guarda, + punti gay e alpha)
- Baffone (nuova)
    requisiti: baffi E punti alpha >= 60 E età >= 35
    sblocca: Sguardo da Baffone (la vittima si sente in soggezione: - aggressività), carisma +10
- Pelato (nuova)
    requisiti: capelli = 0 E età >= 30
    sblocca: Riflesso Accecante (al sole acceca chi è davanti), + punti alpha
- Rissaiolo (nuova)
    requisiti: aggressività >= 70 E muscoli >= 50 E risse vinte >= 10
    sblocca: Testata, provocare (costringe qualcuno a fare rissa)
- Cavaliere
    requisiti: scherma >= 60 E onore >= 60 E muscoli >= 50
    sblocca: sfida a duello, colpo con arma (+ danni con le spade)
- Spadaccino (nuova)
    requisiti: scherma >= 80 E velocità >= 60 E vista >= 50
    sblocca: Affondo (colpo veloce a 2 caselle), parata
- Monaco delle Arti Marziali (nuova)
    requisiti: arti marziali >= 80 E stress <= 20 E alcol = 0 E thc = 0
    sblocca: Pugno del Drago, disarmare, meditazione (- stress di più)
- Cecchino (nuova)
    requisiti: mira >= 80 E vista >= 80 E pazienza >= 60
    sblocca: tiro da lontano (lancia oggetto a 8 caselle), colpo mirato a una parte del corpo
- Ninja
    requisiti: furtività >= 80 E velocità >= 70 E arti marziali >= 50
    sblocca: abilità ninja (invisibilità), bomba fumogena, lama avvelenata

## Classi dei vizi
- Ubriacone (nuova)
    requisiti: tolleranza >= 60 E bevute totali >= 100
    sblocca: Rutto Tossico (danni ad area, - divertimento a chi è vicino), regge l'alcol (malus da alcol dimezzati)
    malus: fegato sale ogni giorno di meno
- Sommelier del Lambrusco (nuova)
    requisiti: alcol and beverage >= 70 E vinificazione >= 40 E percezione >= 60
    sblocca: riconoscere vino avvelenato, fare il lambrusco (qualità migliore), degustazione (+ amicizia a tutti i presenti)
- Fattone / Sciamano della Canna (nuova)
    requisiti: canne fumate >= 100 E tolleranza >= 60 E stress <= 30
    sblocca: SUPER LUCA (creare canne migliori), Visione (scopre una casella nascosta della mappa)
- Spacciatore (nuova)
    requisiti: contatore spaccio >= 20 E furtività >= 40 E eloquenza >= 30
    sblocca: spacciare (più soldi, meno rischio), rete di clienti (soldi passivi)
- Giocatore d'azzardo (nuova)
    requisiti: giochi d'azzardo >= 50 E vincite + perdite al Casinò >= 50
    sblocca: barare (+ probabilità di vincere, rischio di essere scoperti dal Riscossore)
- Hikikomori (nuova)
    requisiti: ore ai videogiochi >= 500 E carisma <= 20 E pulizia <= 30
    sblocca: Speedrun (+ divertimento massimo), Rage Quit (scappa da qualsiasi conversazione)
- Barbone (nuova)
    requisiti: soldi <= 10 E pulizia <= 20 E giorni senza lavarsi >= 14
    sblocca: chiedere l'elemosina (più soldi), Saggezza di Strada (conosce tutti i pettegolezzi)
- Casanova (nuova)
    requisiti: (bellezza >= 60 O carisma >= 70) E partner sessuali >= 10 E capacità sessuale >= 60
    sblocca: corteggiare (+ attrazione molto maggiore), spezzare cuori (- divertimento, + stress alla vittima)

## Classi criminali
- Ladro (nuova)
    requisiti: furtività >= 50 E velocità >= 40 E oggetti rubati >= 10
    sblocca: rubare (più probabilità), scassinare
- Vandalo / Writer (nuova)
    requisiti: graffiti fatti >= 20 E creatività >= 40 E reputazione <= 0
    sblocca: graffiti (messaggi più grandi, letti da più pedine), vandalismo
- Piromane (nuova)
    requisiti: incendi appiccati >= 5 E sanità mentale <= 50
    sblocca: dare fuoco a qualcosa/qualcuno (più danni, più rapido), molotov
- Spia / Infiltrato
    requisiti: furtività >= 60 E eloquenza >= 50 E carisma >= 40
    sblocca: fare la talpa, pedinare, avvelenare i depositi
- Untore (nuova)
    requisiti: almeno 3 malattie contemporaneamente E pulizia <= 20 E sanità mentale <= 40
    sblocca: diffondere malattia venerea (anche senza rapporti), starnuto (contagio ad area)
- Riscossore
    requisiti: muscoli >= 60 E aggressività >= 60 E affiliazione Casinò Diablo Tentator
    sblocca: riscuotere crediti

## Classi dell'ordine
- Poliziotto
    requisiti: legge >= 40 E percezione >= 50 E fedina penale vuota E affiliazione Polizia Neutra
    sblocca: arrestare, multare, perquisire, sequestrare
- Polizia Vegana (Inquisitore della Soia)
    requisiti: affiliazione Impero Vegano E fanatismo >= 70 E percezione >= 60 E contatore carne mangiata = 0
    sblocca: smascheramento delle aure (carne e formaggio)
- Umarell (nuova)
    requisiti: età >= 65 E ore passate a guardare cantieri >= 100
    sblocca: Critica Costruttiva (i muratori vicini lavorano più veloce, ma + stress), guardare i cantieri (effetti raddoppiati)
- Boomer
    requisiti: età >= 55 E informatica <= 20 E affiliazione Circolo dei Boomer
    sblocca: piazzare Gnomi da Giardino spia, diffondere bufale

## Classi religiose e mistiche
- Prete (nuova)
    requisiti: iniziato cristianesimo E religione >= 60 E santità >= 50 E affiliazione Chiesa
    sblocca: confessare, predicare, benedire, sposare due pedine
- Esorcista (nuova)
    requisiti: classe Prete E santità >= 80 E magia bianca >= 50
    sblocca: esorcismo
- Santo (nuova)
    requisiti: santità >= 95 E persone curate >= 50 E fedina penale vuota
    sblocca: prega per qualcuno (guarisce malattie), miracolo (una volta al giorno: risolve una malattia grave)
    note: rarissimo, alla morte diventa un santino della collezione
- Profeta di San Vitale (nuova)
    requisiti: iniziato sanatismo E fanatismo verso san vitale >= 90 E eloquenza >= 60
    sblocca: predicare (converte molto di più), Visione di San Vitale (una notizia sul futuro, a volte falsa)
- Templare (nuova)
    requisiti: affiliazione Templari del Borgo E scherma >= 50 E santità >= 40 E verginità
    sblocca: carica templare, giuramento (i compagni vicini guadagnano onore)
- Massone (nuova)
    requisiti: affiliazione Massoni E soldi >= 1000 E carisma >= 50 E reputazione >= 30
    sblocca: favori (un'Istituzione chiude un occhio su un reato), riunione segreta
- Goth
    requisiti: malinconia >= 70 E divertimento <= 30 E creatività >= 40
    sblocca: Sguardo Tenebroso, poesia triste
- Medium (nuova)
    requisiti: percezione >= 80 E magia bianca >= 40 E sanità mentale <= 60
    sblocca: parlare con i morti (scopre chi ha ucciso un cadavere), seduta spiritica
- Negromante (nuova)
    requisiti: magia nera >= 70 E mana >= 50 E santità <= 10
    sblocca: rito di magia nera, resuscitare (zombie), maledire
- Strega della Bassa (nuova)
    requisiti: magia nera >= 40 E agricoltura >= 40 E età >= 50
    sblocca: trasformazione in maiale, filtro d'amore (+ attrazione verso chi lo offre)
- Mago (nuova)
    requisiti: magia bianca >= 50 E magia nera >= 50 E mana >= 60 E intelligenza >= 120
    sblocca: palla di fuoco, cura gli amici
- Mago Vergine
    requisiti: verginità E giorni da vergine >= 10000 (circa 27 anni) E magia bianca >= 40
    sblocca: incantesimo verginale (cura gli alleati vicini)
    note: la classe si perde per sempre se perde la verginità
- Yag
    requisiti: punti gay >= 100
    sblocca: raggio gay, benedizione Yag
- Jedi
    requisiti: forza (star wars) >= 70 E aggressività <= 30 E stress <= 40
    sblocca: usa la forza, meditazione nella forza, spinta della forza
- Sith
    requisiti: forza (star wars) >= 70 E aggressività >= 70 E (stress >= 60 O contatore persone uccise >= 3)
    sblocca: usa la forza, fulmine Sith, strangolamento
    note: un Jedi che supera aggressività 70 passa al lato oscuro
- Vegano (Mistico della Pianta)
    requisiti: affiliazione Impero Vegano E contatore carne mangiata = 0 per 30 giorni E mana >= 40
    sblocca: levitazione, telecinesi

## Classi nerd
- Nerd (nuova)
    requisiti: lore nerd >= 70 E fanatismo verso i fumetti >= 50
    sblocca: citazione (+ amicizia con altri nerd, - divertimento ai normie), giocare di ruolo (Master)
- Cosplayer (nuova)
    requisiti: classe Nerd E creatività >= 60 E almeno un costume nell'inventario
    sblocca: fare cosplay (+ notorietà, copia una abilità del personaggio interpretato per un po')

## Classi di razza (non acquisibili)
- Bestia: razze animali (leone, talpa, verme, ragno, cinghiale, dinosauro selvatico)
- Guardiano meccanico: robot e droidi assegnati a una zona
- Non morto: zombie
- Mutaforma: abilità dei Rettiliani, non una classe, ma un Rettiliano può avere qualsiasi classe

## Da decidere
- Le soglie sono un primo giro: vanno tarate guardando quanto velocemente salgono le caratteristiche in partita.
- Una classe "avanzata" (Esorcista, Cosplayer) richiede la classe base: quando la si acquisisce si perde la base o la si ingloba? Proposta: la ingloba (l'Esorcista fa tutto quello che fa il Prete).
- Alcune caratteristiche (contatori, puzza, tolleranza) non esistono ancora nel motore.


# Ruoli

## Regole generali
- Un ruolo è un posto unico (o con pochi posti) nel mondo o in una fazione: c'è un solo Diablo Tentator, un solo Vescovo, un solo Re degli Ubriaconi.
- Il ruolo si aggiunge alla classe, non la sostituisce: il Vescovo può essere Prete, Esorcista o anche Normie.
- Per avere un ruolo servono i requisiti (come per le classi) E il posto deve essere libero, oppure bisogna prenderlo a chi lo occupa.
- Modi per ottenere un ruolo (ogni ruolo dice quale vale):
    - successione: il ruolo passa a un erede designato quando il titolare muore o si dimette
    - elezione: votano i membri della fazione, pesa amicizia e reputazione
    - nomina: lo assegna un ruolo superiore (il Commissario nomina gli agenti scelti)
    - sfida: si batte il titolare (duello, gara, rissa, gara di bevute...)
    - colpo di stato: complotto con almeno N membri della fazione dalla propria parte
    - sede: si diventa titolari restando sul trono / nella sede per un certo tempo (già così per il Re degli Ubriaconi)
- Si perde il ruolo se: si muore, si cambia fazione, si scende sotto i requisiti per più di 3 giorni, si perde una sfida o un colpo di stato.
- Ogni ruolo dà punti vittoria alla fazione e fa notizia sul Piccione Viaggiatore quando cambia titolare.

## Ruoli delle fazioni
- Diablo Tentator (Casinò Diablo Tentator)
    requisiti: giochi d'azzardo >= 80 E eloquenza >= 70 E soldi >= 5000 E santità <= 10
    si ottiene: sfida a carte con il Diablo in carica (posta: l'anima, cioè la pedina passa al Casinò se perde)
    poteri: Patto col Diablo (dà a qualcuno soldi o fortuna in cambio di un debito da riscuotere), truccare il Casinò, ordinare ai Riscossori chi visitare
- Capo dei Rivoluzionari (Anarchici del Commercio, oggi Gabriele del Commercio)
    requisiti: carisma >= 70 E eloquenza >= 60 E reputazione presso le Istituzioni <= -30 E almeno 3 reati in fedina penale
    si ottiene: colpo di stato o elezione in assemblea (gli anarchici votano tutti, anche quando non dovrebbero)
    poteri: Chiamata alla Rivolta (gli anarchici vicini guadagnano aggressività e fanatismo), Esproprio Proletario (svuota un negozio e distribuisce la merce), Comizio
- Re degli Ubriaconi (Fazione degli Ubriaconi, oggi Re Anolino)
    requisiti: tolleranza >= 80 E bevute totali >= 500 E baffi
    si ottiene: sede (restare sul trono) oppure gara di bevute col Re in carica
    poteri: Giro Offerto (tutti gli ubriaconi vicini: + divertimento, - stress), Editto del Bancone (prezzo dell'alcol dimezzato per la fazione)
- Gran Maestro dei Templari (Templari del Borgo, oggi Alex Innerfire)
    requisiti: classe Templare E onore >= 90 E scherma >= 80 E verginità
    si ottiene: sfida a duello col Gran Maestro in carica
    poteri: Crociata (i Templari puntano una fazione, + danni contro di lei), Giuramento Solenne (tutti i Templari: + onore)
- Il Vescovo (Chiesa)
    requisiti: classe Prete E santità >= 80 E religione >= 80 E età >= 50
    si ottiene: nomina (in futuro dal Papa, per ora elezione tra i Preti)
    poteri: Scomunica (la vittima perde santità e l'affiliazione alla Chiesa), Processione (tutti i fedeli vicini: - stress, + fanatismo), proclamare un Santo
- Gran Custode di San Vitale (Cripta di San Vitale)
    requisiti: classe Profeta di San Vitale O fanatismo verso san vitale >= 95 E iniziato sanatismo
    si ottiene: successione (il Custode sceglie l'erede in punto di morte), altrimenti visione mistica a caso tra i più fanatici
    poteri: aprire la Cripta, Reliquia (benedice un oggetto), Visione di San Vitale ogni giorno
- Gran Sacerdote del Tofu (Impero Vegano)
    requisiti: classe Vegano E mana >= 70 E carne mangiata = 0 da sempre
    si ottiene: elezione tra i Vegani
    poteri: Dieta Imposta (tutti i membri: - grasso, + mana), Crociata della Soia (comanda la Polizia Vegana)
- Capo dei Nani (Nani delle Miniere)
    requisiti: razza Nano E baffi E muscoli >= 70 E minerali estratti >= 1000
    si ottiene: sfida (rissa) o successione al figlio
    poteri: aprire una nuova miniera, Cavalcata del Dinosauro, Birra per Tutti
- Presidente del CdA (CdA del Fidenza Village)
    requisiti: soldi >= 10000 E eloquenza >= 70 E reputazione >= 40
    si ottiene: acquisto di quote (chi ha più soldi investiti) o colpo di stato in consiglio
    poteri: Saldi (tutti i negozi dell'outlet: - prezzi, + clienti), licenziare, comprare un edificio
    note: tradizionalmente è un Rettiliano travestito
- Consigliere del CdA (3 posti)
    requisiti: soldi >= 3000 E affiliazione CdA
    si ottiene: nomina del Presidente
    poteri: voto in consiglio, sapere chi sono i Rettiliani
- Commissario di Polizia (Polizia Neutra)
    requisiti: classe Poliziotto E legge >= 70 E arresti >= 20 E fedina penale vuota
    si ottiene: nomina (dalle Istituzioni) o anzianità
    poteri: Mandato di Perquisizione, Taglia (mette una taglia su un ricercato), nominare agenti scelti
- Direttore di Testata (Redazione della Gazzetta)
    requisiti: classe Giornalista E articoli pubblicati >= 50 E notorietà >= 60
    si ottiene: nomina o colpo di stato in redazione
    poteri: decide la notizia di prima pagina, censurare un articolo, Smentita (cancella una bufala)
- Babbo Natale Estivo (Capannone di Babbo Natale Estivo)
    requisiti: razza Elfo della Logistica O (grasso >= 70 E baffi E età >= 60)
    si ottiene: successione
    poteri: Consegna Regali (oggetti a caso alle pedine buone), Lista dei Cattivi (carbone e - fortuna)
- Gran Boomer (Circolo dei Boomer, tra gli umani; sopra c'è sempre la Boomer Overmind AI)
    requisiti: classe Boomer E età >= 70 E bufale diffuse >= 50
    si ottiene: elezione al circolo (vince chi offre più caffè corretti)
    poteri: Catena di Sant'Antonio (una bufala arriva a tutti), Gnomo Capo
- Presidente della Repubblica (Istituzioni)
    requisiti: età >= 50 E reputazione >= 80 E fedina penale vuota
    si ottiene: elezione (votano tutte le pedine) o arriva dall'esterno in visita
    poteri: Grazia (cancella la fedina penale di qualcuno), Stato d'Emergenza, inaugurare un cantiere (gli Umarell accorrono)

## Ruoli senza fazione
- Sindaco di Fidenza (nuovo)
    requisiti: età >= 30 E carisma >= 60 E reputazione >= 50
    si ottiene: elezione ogni 30 giorni di gioco, i candidati fanno campagna (comizi, promesse, bufale)
    poteri: Ordinanza (vieta un'azione in una zona: niente alcol in piazza, niente graffiti), aprire un cantiere, tasse
- Sindaco di Salsomaggiore (nuovo)
    come il Sindaco di Fidenza, ma votano solo i Salsesi, e può decidere chi entra alle Terme
- L'Oracolo
    requisiti: classe Mago Vergine E percezione >= 90 E sanità mentale <= 30
    si ottiene: alla morte dell'Oracolo il ruolo passa a una pedina a caso che ha i requisiti
    poteri: rivelare i segreti della mappa, profezia (a volte vera)
- Campione del Borgo (nuovo)
    requisiti: risse vinte >= 30 E muscoli >= 70
    si ottiene: torneo di lotta (ogni tanto, nella piazza) o battendo il Campione in carica
    poteri: + punti alpha a chi lo batte, notorietà alta, gli altri Rissaioli lo sfidano a vista
- Prima Donna / Primo Uomo del Borgo (nuovo)
    requisiti: bellezza >= 85 E notorietà >= 60
    si ottiene: concorso di bellezza alla sagra
    poteri: + attrazione di tutti verso di lei/lui, sponsorizza i negozi
- Il Pazzo del Paese (nuovo)
    requisiti: sanità mentale <= 10 E notorietà >= 40
    si ottiene: sede (chi ha la sanità mentale più bassa e ci sta da più tempo)
    poteri: dice la verità sui Rettiliani ma nessuno gli crede; ogni tanto ci azzecca e fa notizia
- Capo dei Cantieri / Umarell Supremo (nuovo)
    requisiti: classe Umarell E ore passate a guardare cantieri >= 1000
    si ottiene: sede (chi ne ha guardate di più)
    poteri: Sopralluogo (un cantiere finisce prima), mani dietro la schiena (+ pazienza a tutti gli Umarell)
- Re del Sottosuolo (nuovo)
    requisiti: aver esplorato il dungeon più profondo E vita >= 500
    si ottiene: arrivando per primo in fondo
    poteri: comanda la fauna delle profondità (talpe, vermi, ragni)
- Spacciatore Capo / Il Boss (nuovo)
    requisiti: classe Spacciatore E soldi >= 3000 E almeno 5 spacciatori che comprano da lui
    si ottiene: togliendo di mezzo il Boss in carica (arresto, rissa, morte)
    poteri: decide i prezzi dell'erba, protezione (la polizia lo arresta più difficilmente)


# Caratteristiche oggetti

- tipo: questo ci permette di sempificare l'utilizzo degli oggetti. Anche con alcune azioni che abbiamo usato prima, così non dobbiamo per forza collegare azione a singolo oggetto ma andiamo per classi, ad esempio posso lanciare un oggetto se è lanciabile, posso attaccare con un oggett (spada) se è lanciabile.
    - arma
    - lanciabile
    - armatura
    - copricapo
- vita
- peso
- costo
- cumulabile: ad esempio, negli slot, di spada ne posso avere una, di petardi posso averne 100 e mi occupano un solo slot.
- requisiti.
    - es: due mani libere
- bonus:
    + forza
    - velocità
    + mana etc...
