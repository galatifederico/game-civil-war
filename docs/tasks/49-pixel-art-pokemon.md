# 49 — Pixel art più vicina ai Pokémon

**Fatto** (`tools/sprites/chibi.py`, esempi in `docs/img`).
- **Frame più grandi:** 24×32 al posto di 16×20; le pedine sono alte circa 2 caselle, come gli allenatori nella
  mappa dei Pokémon. Testa grande e tonda, ciuffi e riflesso sui capelli, occhi grandi con il punto di luce, bocca.
- **Tre toni:** colore pieno, luce e ombra per capelli, pelle, vestiti (la parte tinta dal colore della fazione),
  pantaloni e scarpe; le tonalità di luce e ombra si ricavano dal colore della razza se non sono indicate.
- **Contorno colorato:** un bordo scuro nella tinta della forma che circonda (non più un nero unico), come nei
  giochi.
- **Passo:** nei frame di camminata il corpo scende di un pixel, le mani oscillano nelle viste di lato; gli
  accessori seguono la testa.
- **Capelli lunghi** che coprono la schiena (e i vestiti) vista da dietro; caschetto; barba dei nani; orecchie degli
  elfi; coda dei rettiliani.
- **Creature:** stesso contorno colorato e luce dall'alto a sinistra; quelle grandi (maiale, dinosauro, leone,
  cinghiale, robot, droide) sono ingrandite 1,5× per stare accanto alle persone.
- **Accessori** delle classi riportati sulla testa nuova, con contorno.
- **Client e console:** la dimensione del frame si ricava dal foglio (larghezza / 3, altezza / 4), quindi si può
  cambiare di nuovo senza toccare il codice.

**Da verificare.** Il client Unity non è stato aperto (Unity non era raggiungibile).
