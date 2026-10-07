# 35 — Classi cumulative, perdita esplicita, azioni di base della razza

**Fatto.**
- **Classi cumulative:** una pedina prende ogni classe di cui ha i requisiti (con la probabilità
  `classes.adopt_chance` a ogni controllo) e le tiene tutte: bonus, abilità e azioni si sommano. Le classi date dal
  personaggio restano; «Normie» resta solo finché non ce n'è un'altra. Il campione del giocatore può accettare più
  offerte. Tolti `priority`, `innate` («di razza»), `includes` («comprende») e `races` dalle classi.
- **Razze ammesse nei requisiti:** nuova condizione `RaceIn([...])`. Tutte le classi acquisibili hanno «Razza =
  Fidentino, Salsese, Nano, Elfo della Logistica, Rettiliano» (più Androide per le classi del gruppo Lavoro):
  bestie, macchine e zombie non prendono classi da soli. Da rivedere classe per classe.
- **Perdita esplicita (`loses_when`):** niente più regola generale del 90%. Vuoto = la classe non si perde mai.
  Casi evidenti messi: Massone, Poliziotto, Polizia Vegana, Prete, Riscossore, Templare e Vegano se lasciano la
  fazione; Templare e Mago Vergine se perdono la verginità; Vegano e Polizia Vegana se mangiano carne; Poliziotto e
  Santo se commettono un reato; Cosplayer senza costume; Untore con meno di 2 malattie; Barbone con almeno 100 €.
- **Sostituisce (`replaces`):** prendendo la classe si perdono quelle sostituite, che non si riprendono finché la si ha
  (il Sith al posto del Jedi).
- **Azioni di base dalla razza:** tolto `universal` dalle azioni; nuovi **gruppi di azioni** (`action_sets`,
  `47_gruppi_azioni.ron`): Vita quotidiana (80 azioni, razze umanoidi e Androide), Istinti animali (bestie e Maiale),
  Routine delle macchine (Robot, Drone, Droide, IA), Istinti dei non morti (Zombie). Ogni razza ha i suoi gruppi più
  le azioni sue (Monta in sella al Nano, Mordi umano al Rettiliano). Classi, ruoli e personaggio ne aggiungono altre.
- **Console:** in cima alla scheda di una classe «Requisiti per diventarlo» e «Si perde se» come righe con simboli
  (≥, <, =, !=, vero, falso), modalità «tutte (e)» / «almeno una (o)», aggiunta di condizioni per tipo (caratteristica,
  razza, classe, fazione, stato, tag, oggetto, ruolo, soldi, ricercato, bisogno, sesso); le condizioni più complesse
  restano modificabili nel JSON. Azioni in tabella (tipo, dove/su chi, effetti, peso, attesa, condizione) ovunque
  compaiano; nella razza la sezione «Azioni di base» (gruppi e azioni proprie); nuova pagina «Gruppi di azioni».

**Da decidere insieme.** Quali classi possono prendere gli Androidi e le altre razze; quali altre classi devono avere
una condizione di perdita; il bilanciamento ora che le classi si sommano.
