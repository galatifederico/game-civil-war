# 19 — Console di amministrazione: cosa si può cambiare

**Contesto.** Volevi sapere dalla console com'è fatta la mappa, dove sono gli oggetti, quali oggetti e sprite
esistono, e configurare il mondo.

**Decisioni prese.**
- La console è nel motore (`/admin/`), quindi funziona con qualsiasi mondo, non solo Fidenza.
- Si cambia dal vivo: caselle, edifici (piazza, sposta, rimuovi), pedine (crea, sposta, rimuovi), parametri,
  mappatura degli sprite, comandi.
- Le definizioni (oggetti, edifici, azioni…) si vedono tutte ma si modificano nei file `.ron`, poi si riavvia: un
  editor delle definizioni con ricarica a caldo è possibile ma va progettato insieme ai salvataggi (un salvataggio
  fatto con definizioni diverse potrebbe non caricarsi).
- Le modifiche alle caselle fatte dalla console finiscono nei salvataggi, ma non nei file della mappa: per renderle
  permanenti vanno riportate in `tools/maps/build_maps.py`.

**Alternative.**
- Editor delle definizioni dalla console con un pacchetto "override" e ricarica.
- Esportare la mappa modificata in un file `.map` da tenere.
