# 06 — Quanto è violento il mondo

**Contesto.** Con le relazioni iniziali tra fazioni (molte sotto -30 = ostili) e le classi combattenti
(Cavalieri, Ninja, Sith, Jedi, Vegani con telecinesi), nei primi 100 tick muoiono circa 10-20 pedine su ~90.

**Decisione presa.** Ho lasciato una violenza "da colony sim" ma l'ho attenuata: attacco solo con cooldown e in
funzione della forza, furti d'identità solo per le spie vere, Oracolo e Borgazzi immortali. Tutti i valori sono
dati (`40_job_azioni.ron`, relazioni in `50_fazioni.ron`) o parametri live.

**Alternative.**
- Mondo più pacifico: relazioni iniziali più morbide, attacchi solo su ordine del giocatore o delle squadre.
- Più caos: tenere i valori originali (più morti, più articoli, più eroismo).

## Aggiornamento 2026-09-29

Bilanciato con il report `--example balance` (30 giorni, 3 seed). Prima: morale a 0 dal giorno 5, tutti
affamati, popolazione in calo. Cause trovate e corrette: peso di default delle azioni a 0 (nessuno mangiava),
filiera del cibo strozzata dalla logistica, notizie troppo pesanti sul morale, nessun recupero del morale,
nessun ricambio di popolazione. Ora (media 3 seed): 86-96 abitanti, ~1 morto al giorno compensato
dall'immigrazione, morale 30-46, prezzi a 1,05-1,2× il base, cibo sempre presente nei negozi.

**Da decidere**: tesori e soldi delle pedine calano lentamente dopo il giorno 20 (gli stipendi superano gli
incassi). Alternative: stipendi più bassi, tasse sugli acquisti, o lasciarlo come pressione economica
per il giocatore.
