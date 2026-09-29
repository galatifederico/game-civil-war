# 21 — Sprite chibi in pixel art (stile Pokémon)

Richiesta del 2026-09-29. Stato: ✅ prima versione.

- Generatore `tools/sprites/chibi.py` (Python + Pillow): tutto disegnato da template ASCII nel codice, quindi
  arte originale e rigenerabile (`python3 tools/sprites/chibi.py unity-client/Assets/Resources/Sprites --preview anteprima.png`).
- Fotogrammi 16×20 px, testa grande (metà altezza) e corpo piccolo, contorno scuro; foglio 3×4:
  pose (ferma, passo A, passo B) × direzioni (giù, sinistra, destra, su).
- Strati: `chibi_<razza>.png` (pelle, capelli, occhi, gambe), `chibi_<razza>_cloth.png` (vestito in grigi, colorato
  dal client con il colore della fazione), `acc_<nome>.png` (accessorio della classe: berretto della polizia,
  elmo, cappello da mago, fascia ninja, corona di foglie, cappucci Jedi/Sith, berretto medico, cappellino boomer,
  fedora della stampa, aureola, corona del campione).
- Razze umanoidi: Fidentino, Salsese, Nano (barba), Elfo (orecchie), Rettiliano (pelle verde, coda), Zombie,
  Androide (capelli lunghi). Creature con disegno proprio: maiale, dinosauro, leone, robot, drone, droide R2.
- Il collegamento è nei dati (`sprites` in `90_mondo.ron`: `sheet` per `race:*` e `class:*`), quindi si possono
  cambiare o aggiungere fogli senza toccare il client.
- Client Unity: animazione di camminata a 6 fps, direzione dal movimento, profondità per riga (chi è più in
  basso copre chi è dietro), ombra ovale del colore della fazione, corona sul campione, pedine a terra (morte o
  al tappeto) ruotate.

Prossimi passi: edifici e terreno in pixel art (oggi sono ancora quadrati colorati), ritratti per la scheda.
