# 41 — Eventi (al posto di generatori, eventi a condizione e fonti di notizie)

**Fatto.**
- **Un solo concetto, l'evento** (`events`): succede da solo quando vale la sua condizione (`when`), una volta o
  ogni `every` ore, da `from_tick`; con `by` solo mentre è vivo un personaggio di quel template, che compie gli
  effetti. Gli eventi con un nome finiscono nella cronaca.
- **Generatori → eventi con l'effetto `Spawn`**, che ora sa anche la fazione, il legame a una zona (`tether`) e il
  massimo di vivi (`max_alive`, contati per chi li ha generati).
- **Fonti di notizie → eventi dell'autore** con il nuovo effetto **`Cycle`** («a turno»): un titolo diverso ogni
  volta, in ordine (prima era a caso). `Publish` sostituisce ora {subject} e {target} nel titolo.
- **Eventi a condizione → eventi** (una volta sola, o ripetuti con l'attesa di prima).
- Migrati 31 generatori, 18 eventi e 4 fonti; il comando `fire_trigger` (e lo strumento MCP `trigger_event`) fa
  succedere un evento subito.
- **Console:** pagina «Eventi» (sezione Fazioni e mondo) con filtri (evoca / notizie / altro, ripetuto), scheda con
  «Quando succede» (condizioni a righe, ogni quante ore, dall'ora, da chi, notiziabilità), «Cosa succede» (tabella
  degli effetti, anche a turno) e il pulsante «Fallo succedere ora».
