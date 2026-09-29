# 20 — Campione, ordini, strategia, pathfinding, salvataggi, bilanciamento

Richiesta del 2026-09-29. Stato: ✅ fatto.

- **Campione**: il leader di ogni giocatore (`Leader` + `Controlled`) si muove solo su ordine del giocatore
  (`PlayerOrder` → `Move`/`Job`/`Ability`/`Follow`/`Stop`); quando è libero pensa solo ai propri bisogni
  (mangiare, dormire, svagarsi), senza andare a spasso da solo.
- **Ordini ai membri** della propria fazione: obbediscono con probabilità
  `0,85 + (morale−50)/200 − dissenso·0,8/100 − 0,03·(livello del rango−1)` (tra 5% e 99%); chi rifiuta
  guadagna dissenso e genera l'evento `order_refused`.
- **Squadre del giocatore** (`PlayerCreateSquad`, `PlayerSquadOrder`), con il nuovo ordine `Follow` (segui il campione).
- **Strategia delle fazioni AI**: `goals` nei dati delle fazioni (a Fidenza: caccia alle reliquie maggiori).
- **Pathfinding A\*** con muri nei dati (`walls`, a Fidenza il torrente Stirone con tre ponti e le mura della
  Cattedrale), portali tra livelli, tether rispettato.
- **Salvataggio/caricamento** esatti: `POST /api/save`, `POST /api/load`, `--load file.json`; testato che una
  partita ricaricata prosegua identica.
- **Bilanciamento** misurato con `cargo run --release -p fidenza_world --example balance -- 30 3`
  (vedi [domanda 06](../questions/06-violenza-e-bilanciamento.md)).
- **Client Unity**: tasto destro muove la pedina selezionata, pulsanti degli ordini, scheda "La mia fazione"
  (membri e obbedienza, squadre, stipendi, priorità di lavoro), campione (C) e camera che lo segue, salva/carica.
