# 39 — Stati e malattie deterministici

**Fatto.**
- **Uno stato è un'intensità 0–100** su una pedina (valore interno dello stato, non una caratteristica). Una dose
  aggiunge `intensity` (di default 100; gli effetti possono dare più dosi), ogni ora cambia di `per_tick` (negativo
  = guarisce da solo, positivo = peggiora), meno `resist_per_point` per ogni punto della caratteristica di resistenza
  (`resist_stat`); a 0 lo stato finisce. Niente probabilità.
- **Finché è attivo:** modificatori di caratteristica, bisogni, tag dati alla pedina, effetti ogni ora; più
  `on_apply` / `on_expire`.
- **Soglie** («da intensità X»): vale la più alta raggiunta, con modificatori, bisogni, effetti ogni ora ed effetti
  quando la si raggiunge. Sostituiscono stadi ed escalation (Brillo e Fattone sopra 90 diventano Schifoso / Super Luca;
  Morbo sopra 75 trasforma in zombie; Mutazione Porcina a 100 in maiale).
- **Contagio come aura:** chi è nel raggio di un malato prende `per_tick` × (intensità del malato / 100) ogni ora,
  ridotto dalla propria resistenza; il malato lascia `shedding` sulla cella, e celle e acquedotti contaminati passano
  intensità in proporzione al carico. I modificatori dei malati contagiosi partono da 30 (incubazione).
- **Tolti:** tipo (ora una categoria fra i tag: malattia, droga, mutazione…), durata, accumulo, stadi, escalation,
  capacità, velocità dell'IA (Stordito/Avvinghiato rallentano con la velocità), fluidi versati (ora effetti ogni ora),
  nascosto, vettori di contagio.
- **Conversione dei 31 stati:** durata D → dose 100 e −100/D all'ora (stessa durata); progressione → dose 25 e soglie a
  25/50/75/100; Resistenza combatte Diarrea, Infezione, STD, Morbo e Avvelenato.
- **Console:** scheda dello stato con Intensità (dose, ritmo, resistenza e quanto dura una dose), Finché è attivo,
  Soglie (aggiungi/togli), Contagio (rendi/non più contagioso); elenco con categorie, ritmo, dose e soglie.

**Restano casuali** (da rivedere dopo, come deciso): le probabilità negli effetti di azioni e oggetti.
