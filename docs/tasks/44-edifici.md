# 44 — Edifici: produzioni uniche

**Fatto.**
- **Produzioni** (`productions`) al posto di ricette, produzione passiva e incasso: ognuna ha ingressi, uscite e
  soldi; con `every` 0 la fa un lavoratore (`work`, `work_type`: si pubblica un lavoro sulla bacheca quando ci sono
  gli ingressi), altrimenti avviene da sola ogni `every` ore se ci sono gli ingressi (campi, recinti, visitatori), con
  i soldi al proprietario in proporzione all'integrità. Il lavoro «coltiva/alleva» su un campo dà le uscite delle sue
  produzioni automatiche senza ingressi.
- Migrati 11 ricettari, 11 produzioni passive («Produzione», ogni N ore) e 19 incassi («Visitatori», ogni 24 ore).
- Restano a parte le **esportazioni** (vendono fuori le scorte al prezzo di mercato) e le **soglie di integrità**
  (le conseguenze dei danni, già scritte come «sotto il X%: effetti»).
- **Console:** scheda dell'edificio per sezioni — Cos'è, Produzioni (come, usa, produce, soldi), Negozio e scorte,
  Esporta, Costruzione, Soglie di integrità; elenco con quello che produce e filtri per categoria e modo di produrre.
