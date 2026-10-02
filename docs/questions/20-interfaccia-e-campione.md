# 20 — Interfaccia nuova e campione mosso a mano

**Contesto.** Hai chiesto i numeri in alto, i pulsanti a destra e a sinistra, un pulsante per nascondere tutto e
il campione da muovere come in Pokémon per esplorare e decidere con chi parlare. Le risposte che mi hai dato:
comandi in un pulsante Opzioni, passi immediati anche in pausa, canale principale con solo ciò che ti tocca,
soldi = fondo di gilda.

**Decisioni prese.**
- **"Parlare":** nel gioco non esiste ancora un'azione di dialogo. Davanti a qualcuno, A/Spazio apre la sua scheda
  con le azioni che il campione può fare su di lui. Va deciso cosa fa "Parla" (vedi sotto).
- **Il campione aspetta:** dopo un passo dato a mano la sua AI si ferma per un giorno di gioco
  (`player.manual_hold_ticks` = 24), poi torna a mangiare, dormire e svagarsi da solo. Senza questo, appena ti
  fermavi a leggere un pannello se ne andava al bar.
- **Punti in alto:** i punti che contano per la vittoria (punti vittoria + reliquie possedute), come in classifica.
- **Inventario del team:** oggetti addosso ai membri + scorte degli edifici posseduti dalla fazione.
- **Mappa:** mostra la superficie; le mappe interne e sotterranee compaiono nell'elenco quando c'è qualcuno dei
  tuoi (o da "Tutte le mappe").
- **Pulsante nascondi:** nasconde numeri, colonne, finestre e scheda; sul telefono la croce e il tasto A restano,
  altrimenti a interfaccia nascosta non potresti muovere il campione.
- **Canali del piccione:** Principale, Su di te, le categorie del giornale (Chiacchiere, Politica, Cronaca,
  Salute, Economia, Eventi), Tutto. Il popup in basso compare solo per i messaggi nuovi del Principale.

**Domande aperte.**
- Cosa deve fare **Parla**? Proposte: (a) chiacchierata che migliora la relazione con la sua fazione e a volte dà
  una voce/notizia; (b) reclutamento se l'umore è basso e la relazione alta; (c) commercio (compra/vendi);
  (d) missioni dai personaggi con nome.
- Il campione deve **bloccarsi per sempre** quando lo muovi a mano (solo tu lo muovi), o va bene che dopo un giorno
  di gioco torni autonomo?
- Quali **altri canali** del piccione vuoi abilitare per primi (es. un canale per fazione alleata, uno per il
  mercato, messaggi privati tra giocatori)?
