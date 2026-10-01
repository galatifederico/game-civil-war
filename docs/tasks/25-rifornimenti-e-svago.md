# 25 — Rifornimenti da fuori e luoghi di divertimento

**Fatto.**
- `supplies` nei pacchetti dati (`supply.rs`): ogni giorno i negozi che vendono un oggetto vengono riforniti fino a
  `per_shop` unità, al massimo `max_per_day`, con una variazione casuale; il proprietario paga `cost` × prezzo base.
  La produzione locale riempie prima i negozi, quindi copre una quota variabile di un totale stabile.
- Gli eventi globali hanno `supply: [(oggetto o tag, fattore)]`; sette circostanze casuali li attivano
  (sciopero dei camionisti, nebbia, piena dello Stirone, festa della birra, sagra del culatello, mercato contadino,
  camion di Lambrusco ribaltato) e il Proibizionismo vegano taglia alcol e carne.
- 19 luoghi di divertimento sparsi per la mappa e l'azione "Si diverte" (entro 40 passi).

**Effetto su 30 giorni:** cibo nei negozi stabile intorno a 300 (prima calava fino a 7), svago intorno a 0,62
(prima 0,33), morale intorno a 41.
