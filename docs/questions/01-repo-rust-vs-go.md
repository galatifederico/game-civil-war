# 01 — Cosa fare del codice esistente (Go, Unity, admin web)

**Contesto.** La repo conteneva un gioco multigiocatore a griglia (backend Go + Postgres, client Unity,
admin web). Il nuovo design chiede un engine di simulazione headless in Rust, completamente diverso
(colony sim deterministico, ECS, data-driven).

**Decisione presa.** Su tua indicazione ho cancellato tutto tranne `docs/` (anche `img/`, `art-src/`,
`tools/`, `deploy/`, e le modifiche non committate in `backend/`). Nulla è riutilizzabile direttamente
(linguaggio diverso); le idee utili (griglia, zone, fazioni/obiettivi come dati, admin web) sono state
riprese nel nuovo design. Il codice vecchio resta nella storia git fino al commit `3077ea5`.

**Alternative.**
- Tenere il backend Go come gateway multigiocatore e usare l'engine Rust come libreria/servizio di simulazione.
- Riportare il client Unity e farlo parlare con le API UI del nuovo engine (task 19).
