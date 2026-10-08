# 37 — Bisogni come caratteristiche, con soglie

**Fatto.**
- **Un bisogno muove una caratteristica.** Fame, Sete, Riposo e Svago sono caratteristiche 0–100 (gruppo
  «Bisogni», piene alla nascita): si vedono nella scheda e classi, abilità, aure e stati possono modificarle. Ogni
  bisogno dice quale caratteristica muove (`stat`, di default il suo id: se manca la crea il motore) e di quanto ogni
  ora (`per_tick`, negativo = cala); razza, abilità e stati cambiano il ritmo come prima (`need_rates`), i bisogni
  assenti della razza (`needs_exempt`) restano pieni.
- **Soglie:** ogni bisogno ne ha quante vuole; ognuna ha un nome, «sotto quanto», effetti a ogni ora finché si è
  sotto e modificatori di caratteristica che valgono finché si è sotto.
  - Fame: Affamato (< 10, −0,6 morale ogni ora), **Denutrito** (< 3, −0,5 vita ogni ora, −5 Muscoli finché sotto).
  - Sete: Assetato (< 10, −0,4 morale e +0,2 stress ogni ora), **Disidratato** (< 3, −0,8 vita ogni ora, −0,3
    velocità finché sotto).
  - Svago: Annoiato (< 15, −0,6 umore e −0,3 morale ogni ora). Riposo: nessuna soglia.
- **Dati migrati:** soddisfare o peggiorare un bisogno ora modifica la sua caratteristica (×100), «bisogno sotto il
  X%» è «caratteristica < X», le considerazioni dell'IA leggono la caratteristica su 100. Il motore capisce ancora la
  forma vecchia (`ModNeed`, `NeedBelow`, `Need`) come quota della caratteristica. I salvataggi vecchi convertono i
  bisogni in caratteristiche al caricamento.
- **Console:** scheda del bisogno con «Caratteristica e ritmo» e «Soglie» (aggiungi/togli, nome, valore, effetti
  ogni ora, modificatori finché sotto); elenco dei bisogni con caratteristica, ritmo e soglie; tabella dei bisogni
  della razza con il ritmo orario.

**Da tarare.** I valori delle soglie nuove (Denutrito, Disidratato) sono di esempio.
