# 33 — Console del mondo (`/admin/`)

**Fatto.** La console admin è diventata lo strumento per osservare il mondo che stiamo costruendo:
- **Menu laterale** a sezioni: *Mondo* (Panoramica, Mappa, Pedine, Edifici in gioco, Eventi), *Catalogo* (tutto ciò che è
  configurato: Personaggi/NPC, Razze, Classi, Ruoli, Caratteristiche, Bisogni, Azioni dell'AI, Lavori ed effetti, Abilità,
  Stati e malattie, Oggetti, Edifici, Fazioni, Collezioni, Generatori, Eventi a condizione, Rifornimenti, Circostanze
  globali, Fonti di notizie, Corpi, Fluidi), *Strumenti* (Sprite, Parametri, Comandi, Contenuti grezzi). Accanto a ogni
  voce quante ce ne sono.
- **Stesso schema per tutte le sezioni con entità:** tabella con le informazioni principali e lo sprite (clic sullo sprite:
  ingrandito; le pedine sono composte da razza, sesso, vestiti col colore della fazione e accessorio della classe),
  ordinabile per colonna e filtrabile; clic per selezionare, doppio clic o «Apri il dettaglio» per la scheda.
- **Schede di dettaglio** con collegamenti tra le voci (una classe porta alle sue abilità e azioni, un NPC alla razza e
  alla fazione…), requisiti ed effetti scritti a parole (`GET /api/admin/describe`), «chi la usa / dove compare» e
  la parte viva: chi è di quella classe, chi tiene il ruolo, dove si trova un oggetto, chi sta facendo un'azione, chi
  ha una malattia. Per le **caratteristiche**: media, minimo, massimo, istogramma tra le pedine vive, i più alti e i
  più bassi, quali classi e ruoli la richiedono. La scheda di una pedina mostra tutte le caratteristiche per gruppo,
  bisogni, inventario con icone, equipaggiamento, ruoli, taccuino e Utility AI.
- **Panoramica:** pedine vive e morte, salute media, classi in gioco, ruoli assegnati, ricercati, malati; classi più
  diffuse, cosa stanno facendo, chi tiene i ruoli, fazioni.
- Indirizzi diretti: `/admin/#cat/classes`, `/admin/#cat/classes/goth`, `/admin/#ent/12`, `/admin/#list/pawns`…
- Resta tutto in un file (`sim_core/src/server/admin.html`), senza dipendenze esterne né build (niente React: la
  console la serve il motore stesso, anche offline).

**Non fatto.** Modifica delle definizioni dalla console (si cambiano i `.ron`); i gruppi delle caratteristiche sono in
ordine alfabetico (Principali per primi, Contatori per ultimi) perché il motore non conserva l'ordine dei file.
