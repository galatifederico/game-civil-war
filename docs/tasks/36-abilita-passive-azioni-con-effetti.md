# 36 — Abilità passive, azioni con effetti, ruoli come le classi

**Fatto.**
- **Azioni e abilità separate.** Un'**azione** è qualcosa che una pedina fa di proposito (lavori, attacchi,
  incantesimi, trasformazioni); un'**abilità** è un tratto permanente di chi ce l'ha.
- **Azioni con effetti** (`ActionKind::Effects { target, range, effects }`): le 82 abilità attive di prima sono
  diventate azioni. Le 80 azioni dell'IA che le «incartavano» portano ora effetti, gittata, attesa, requisiti e
  sospettosità; Mutaforma, Evoca Dinosauro e Rage quit sono azioni nuove con peso 0 (`48_azioni_speciali.ron`:
  l'IA non le sceglie, le usano il giocatore e gli ordini). Le liste di abilità di razze, classi e ruoli sono
  diventate azioni. L'ordine del giocatore resta `kind: "ability"` (con l'id dell'azione) per i client.
- **Abilità passive** (`12_abilita.ron`): modificatori di caratteristica e di consumo dei bisogni su chi le ha,
  tag, immunità e **aura** (raggio, condizione su chi colpisce, caratteristiche «finché resta vicino» e «ogni ora,
  si accumula e resta»). Esempi: Aura depressa (Goth), Tanfo (Barbone), Pace interiore (Prete), Sangue freddo
  (Rettiliano), Corazza metallica (Robot).
- **Ruoli come le classi:** «si perde se» esplicito (`loses_when`); la tolleranza è il tempo per cui la condizione
  deve valere. Tolta la regola generale del 90% (anche `classes.keep_ratio`). Casi evidenti messi su 20 ruoli: lascia
  la fazione (ruoli «solo membri»), perde la classe su cui si basa (Commissario, Direttore, Gran Boomer, Umarell
  Supremo, Boss dello spaccio), verginità (Gran Maestro Templari), carne (Gran Sacerdote del Tofu), reati
  (Commissario, Presidente della Repubblica), razza (Sindaci).
- **Console:** abilità in tabella (su chi, effetto, modificatore, quando), scheda dell'abilità con «Su chi ce l'ha»
  e «Aura»; azioni con effetti nella tabella delle azioni; nelle classi «Modificatori di caratteristica» subito dopo
  i requisiti; scheda dei ruoli come quella delle classi (requisiti, si perde se, come si ottiene e passa di mano,
  punteggio dei candidati, modificatori, abilità, azioni).

**Da decidere insieme.** Quali altre abilità passive servono e a chi; se dare un peso > 0 a Mutaforma ed Evoca
Dinosauro perché l'IA le usi da sola; le condizioni di perdita degli altri ruoli.
