# 47 — Valori, convinzioni (informazioni che si propagano) e diplomazia

Richiesta: punti 1–4 (affidabilità delle fazioni e rapporti, diffamazione e informazioni che contagiano, valori
delle pedine e delle fazioni). Fatto in autonomia; le scelte sono qui.

## Valori (pedine e fazioni)
- Nuovo contenuto **Valori** (`15_valori.ron`): Fede, Tradizione, Lavoro, Capitale, Libertà, Onore, Vizio,
  Tecnologia, Carne, Morte, Magia nera (le stesse chiavi della vecchia «ideologia» delle fazioni). Ogni valore è una
  caratteristica della pedina 0–100 (gruppo «Valori», 50 ± 15 alla nascita; `v_<id>`) e dice quali **categorie di
  azioni** piacciono (+) o dispiacciono (−) a chi lo ha alto.
- **Le azioni:** l'utilità di un'azione si moltiplica anche per i valori della pedina sulle categorie dell'azione
  (come la modalità).
- **Le reazioni:** chi vede fare un'azione (entro `values.witness_radius`) la giudica con i suoi valori e si fa
  un'opinione di chi la fa (e un po' della sua fazione).
- **Le fazioni** hanno i loro **valori** (0–100, dall'ideologia di prima: 50 + 50 × valore). Pedine con valori lontani
  da quelli della loro fazione accumulano malcontento (`values.dissent`) e, quando disertano, scelgono la fazione
  più vicina per valori, per quello che credono di lei e per i rapporti.
- Alcune classi spostano i valori (Prete +Fede, Satanista −Fede +Magia nera, Vegano −Carne, Boomer +Tradizione e più
  influenzabile, Giornalista e Santo più credibili, Templare +Onore…).

## Convinzioni (informazioni che si propagano)
- Ogni pedina porta con sé delle **convinzioni** su fazioni e persone: un giudizio (−100 inaffidabile/nemico …
  100 affidabile/idolo) e una forza (0–100); ne tiene le 12 più forti.
- **Si propagano come un contagio:** ogni 3 ore chi sta vicino (2 caselle) passa le sue convinzioni forti, in
  proporzione alla **Credibilità** di chi parla, all'**Influenzabilità** di chi ascolta e alla fiducia che chi
  ascolta ha in chi parla. **Si affievoliscono** col tempo e si perdono.
- Effetti nuovi: `Tell` (chi parla convince il bersaglio) e `Believe` (ci si convince da soli), su `Subject`,
  `SubjectFaction`, `SubjectEnemy` (la fazione che chi parla detesta di più), `SubjectEnemyPawn`, `Target`,
  `TargetFaction`, `Faction(id)`.
- **Diffama:** convince una pedina vicina che la fazione e la persona che chi parla detesta di più sono
  inaffidabili (resta un reato). **Propaganda:** chi sta vicino si convince che la fazione di chi parla è affidabile.

## Diplomazia
- **Affidabilità pubblica** di una fazione: la media di ciò che ne pensano quelli che non ne fanno parte.
- **Rapporti tra fazioni:** si avvicinano a ciò che i membri credono dell'altra fazione (`factions.relation_drift`).
- **Elezioni:** al punteggio dei candidati si somma l'opinione pubblica su di loro (`titles.opinion_weight`).

## Console
- Pagina **Valori** (sezione Pedine); nella fazione i valori (modificabili), l'affidabilità pubblica e la modalità;
  nell'elenco delle fazioni la colonna «Affidabilità pubblica»; nella scheda della pedina la tabella delle
  **convinzioni**.

## Da tarare / da decidere
- Pesi dei valori, velocità di propagazione e oblio, e convinzioni iniziali (oggi si parte senza: nascono da
  diffamazioni, propaganda e da ciò che si vede fare).
