# 07 — Lingua di codice e contenuti

**Contesto.** Il repo era in italiano (documenti, commit), il codice Go in inglese.

**Decisione presa.**
- Codice Rust (`sim_core`): identificatori e commenti in inglese, come l'idioma Rust.
- Contenuti di Fidenza: id e nomi in italiano (`gamba_destra`, `polizia_neutra`), perché sono dati del mondo.
- Messaggi degli eventi del motore in italiano (li leggono giocatore e admin).
- Documenti in italiano.

**Alternative.** Tutto in inglese (più facile riusare il motore altrove); messaggi degli eventi localizzabili con
un file di traduzioni per pacchetto (utile se il motore verrà usato per un mondo non italiano).
