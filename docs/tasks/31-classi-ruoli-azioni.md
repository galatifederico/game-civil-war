# 31 — Caratteristiche, azioni, classi e ruoli (docs/azioni.md)

**Fatto.**
- **Motore (sim_core), tutto generico:**
  - Statistiche con `group` (sezione della scheda), `per_day` (crescita giornaliera: età, puzza…) e `spread`
    (ogni pedina nasce un po' diversa). Fedina penale: `bindings.crime_record` sale di 1 a ogni accusa.
  - Classi con `requires` (requisiti), `includes`, `replaces`, `innate`, `priority`, `group`. Sistema
    `classes::progression` (ogni `classes.check_every` tick): chi ha i requisiti prende la classe (le pedine da
    sole, con `classes.adopt_chance`; il campione riceve un'offerta e sceglie), chi scende sotto il 90% delle
    soglie (`classes.keep_ratio`) la perde e torna Normie (`bindings.default_class`). I personaggi unici e le
    classi date dal template non cambiano da sole, salvo classi che le sostituiscono (Jedi → Sith).
  - Ruoli (titoli) con `mode`: Seat, Succession, Election (con `term`), Appointment, Challenge, Coup, Purchase;
    `members_only`, `score`, `vacancy_ticks`, `grace` (tick sotto i requisiti prima di perderlo, con la stessa
    tolleranza del 90%), `challenge_stat`/`challenge_allies`, poteri (`stats`, `tags`, `abilities`, `actions`).
    Sistema `titles::roles_tick`; effetti `ClaimTitle`, `LeaveTitle`, `ChallengeTitle`.
  - Legami tra pedine (amicizia, attrazione): effetto `ModBond`, condizione `BondAtLeast`.
  - Condizioni nuove: `Contest` (risse, duelli), `HoldsTitle`, `HoldsAnyTitle`, `IsSex`, `StatusTagCount`.
    Effetti nuovi: `TransmuteFor` (trasformazione temporanea), `Note` (taccuino della pedina).
  - API: `GET /api/ui/classes/{player}`, `POST /api/ui/class`, `GET /api/ui/roles?player=`, `POST /api/ui/role`,
    `GET /api/ui/stats`; la scheda (`/api/entities/{id}`) ha anche `journal` e `titles`. Comandi
    `player_take_class`, `player_challenge_role`. Requisiti descritti a parole (`describe::condition`).
- **Fidenza (dati):** ~80 caratteristiche in 8 gruppi (`00_core.ron`), bisogno Sete; 66 classi con requisiti
  (`14_classi.ron`), compreso l'**Investigatore** e le classi oscure della **Cripta di San Vitale** (Goth,
  Satanista, Negromante, Strega, Medium, Profeta: magia nera e Satana); ~70 job e azioni della vita quotidiana
  (`45_vita_quotidiana.ron`); 27 ruoli con i loro poteri e le sfide dell'AI (`55_ruoli.ron`), Loggia Massonica;
  bevande, cibo e canne alimentano alcol, grasso, contatori; personaggi con statistiche coerenti con classe e ruolo.
  Plugin: chi può avere figli con chi e le nascite (`concepisci`), indagini dell'Investigatore (`indaga`:
  reliquie, strade verso una zona con i passaggi da prendere, infiltrati, ricercati, ruoli).
- **Client Unity:** finestre **Classi** (K: requisiti ✓/✗, "Diventa…") e **Ruoli** (U: chi li tiene, come si
  ottengono, "Prendi il posto"/"Sfida"/"Colpo di stato"); nella scheda ruoli, taccuino e caratteristiche per gruppo.
- Strumento: `cargo run --release -p fidenza_world --example azioni -- [giorni] [seed]` (cosa fanno le pedine,
  classi, ruoli).

**Non fatto / da rivedere.**
- **Bilanciamento** (rimandato su richiesta): poche pedine cambiano classe in 12 giorni; alcuni ruoli restano
  vacanti; il test `factions_hunt_relics` è `ignore` perché in 12 giorni nessuno ruba più una reliquia.
- La diarrea si diffonde moltissimo (5000+ contagi in 600 tick), ma era già così prima di queste modifiche.
- Il client Unity non è stato compilato qui (UnityMCP non raggiungibile e niente build Unity insieme a cargo).
- Non fatti: seppellire i cadaveri (domanda 21), reputazione verso le singole fazioni, parti del corpo per dita
  e denti, lavoro che aumenta lo stress, gravidanza con durata (la nascita è immediata).
