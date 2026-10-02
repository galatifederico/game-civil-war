# 30 — Giornale più calmo, ritmo più lento, goth, sesso, collezioni, stato leggero

**Fatto.**
- **Giornale:** circa 7 volte meno notizie (0,44 → 0,064 per tick). I malanni comuni non fanno più notizia
  (`press.disease_news` 0,2 sotto la soglia di 0,3), i giornalisti aspettano 12 tick tra un articolo e l'altro,
  le bufale del Boomer passano da ogni 8 a ogni 48 tick, la propaganda da 36–48 a 120–144, l'Oracolo da 24 a 96.
  **Pulizia:** si tengono le ultime `press.main_keep` (60) notizie del canale principale; quelle dei canali
  secondari si cancellano dopo `press.secondary_ttl_ticks`, che il server imposta a 24 ore reali alla velocità
  corrente. Gli id degli articoli non si ripetono più (`Feed.next_id`).
- **Ritmo:** 1 secondo per tick di default (prima 0,4); velocità Lenta 2 s, Normale 1 s, Veloce 0,4 s, Turbo
  0,1 s. Le pedine camminano a passo costante (un passo occupa circa un tick) invece di scattare e fermarsi.
  Nuova azione **Si ferma** (a chiacchierare, 4 tick), e chi viene interpellato dal campione si ferma ad
  ascoltare. Il suggerimento "parla con…" vale fino a 2 caselle, come il server.
- **Stato per il client** (`/api/ui/state?lite=true`): solo ciò che serve alla mappa (la scheda chiede il resto),
  caselle sporche compatte `[layer, x, y, sporco × 10, patogeni]`, mappatura degli sprite solo alla prima
  richiesta, preparato una volta per tick e per fazione (cache condivisa tra i client), `since=<tick>` → risposta
  di poche decine di byte se il tick non è cambiato, compressione gzip (tower-http). Da 190 KB a circa 19 KB per
  tick compressi, zero tra un tick e l'altro. Il client Unity chiede gzip da sé (libcurl).
- **Sesso:** `Sex` (Male, Female, NonBinary) su ogni pedina; razze `sexless` (robot, drone, droide, IA) senza;
  si può fissare nel template (Waifu e Maid donne), altrimenti si estrae alla nascita
  (`population.nonbinary_share` 6%). Sprite: capelli lunghi per le donne, caschetto per i non binari
  (`chibi_<razza>_f/_nb.png`). Visibile nella scheda e nel Team.
- **Goth:** classe con abilità Sguardo Tenebroso (−umore e −svago al bersaglio, +svago a sé), malinconia notturna
  in Cripta, accessorio cappuccio nero con fiocco viola, 3 goth in piazza più Morticia del Borgo e Lord Tenebra,
  arrivi se sono meno di 2, frasi proprie.
- **Collezioni** (`data/32_collezioni.ron`), 20 pezzi ciascuna con un effetto e una descrizione:
  - Opere del Borgazzi (uniche, bonus a chi le porta): il Borgazzi ne dipinge una ogni 48 tick e la mette in
    vendita nella nuova **Galleria del Borgazzi** in Piazza Garibaldi; le pedine ricche ogni tanto ne comprano una.
  - Foto di Pag (bonus a chi le porta) e Santini (metà da usare, metà da tenere): si trovano nelle **bustine**
    (Fumetteria, Banchetto dei Santini, rifornite ogni giorno) che si aprono subito con un pezzo a caso (nuovo
    effetto `GiveRandomItem`). Anche le pedine collezionano.
  - Nell'Inventario: scheda **Collezioni** con posseduti/totale, punti e barra; ogni collezione apre la griglia dei
    pezzi (posseduti a colori con quantità, mancanti sbiaditi), ogni pezzo apre il dettaglio.
  - Icone diverse per ogni pezzo (`tools/sprites/items.py`), icona della galleria.

**TODO.**
- **Cadaveri:** oggi restano nel mondo per sempre (107 su 223 pedine dopo 10.000 tick). È una dinamica da
  progettare (sepoltura, becchini, malattie dai corpi, zombie, saccheggio, decomposizione) prima di partite lunghe.
- Le altre collezioni (fossili, kit dello sballo…) compaiono nell'elenco ma hanno ancora pochi pezzi.
