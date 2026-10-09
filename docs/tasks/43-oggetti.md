# 43 — Oggetti: classificazione unica e scheda per sezioni

**Fatto.**
- **Da tre classificazioni a due, con ruoli chiari:** i *tipi* (arma, lanciabile, armatura, copricapo, accessorio,
  incendiario) sono entrati nelle **categorie** (tag) di 46 oggetti; la condizione «ha qualcosa di tipo X» è «ha
  qualcosa con la categoria X». Resta lo **scomparto** (`category`), che è una meccanica: decide in quale scomparto
  dell'inventario va l'oggetto e con cosa si impila. L'API dell'oggetto dà ancora `types` (= le categorie) per i
  client.
- **L'uso resta nell'oggetto** («Quando lo usi»): l'IA usa azioni generiche (Mangia, Bevi, Lancia…) che scelgono
  l'oggetto migliore e ne applicano gli effetti d'uso; un'azione per oggetto moltiplicherebbe le azioni senza
  vantaggi.
- **Console:** scheda dell'oggetto per sezioni — Cos'è (scomparto, quanti per scomparto, prezzo, peso, unico, punti
  vittoria, categorie), Finché lo porti, Come arma o indumento (danni, gittata, mani, dove si indossa, vita, effetti
  su chi viene colpito), Quando lo usi (effetti, non si consuma), Requisiti per usarlo; elenco con filtri per
  categoria e scomparto.
