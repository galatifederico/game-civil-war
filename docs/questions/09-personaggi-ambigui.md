# 09 — Personaggi e classi con poca descrizione

**Contesto.** Alcuni elementi del design non hanno una descrizione meccanica.

**Decisioni prese.**
- **Yag**: mistico di supporto con "Benedizione Yag" (morale +5 e Punti Gay +1 a chi è vicino).
- **Mago Vergine**: guaritore ("Incantesimo Verginale" cura gli alleati vicini).
- **I Presidenti della Repubblica**: un unico NPC "Il Presidente della Repubblica" che arriva in visita al tick 30
  (trigger `visita_presidenziale`), fazione neutrale "Istituzioni".
- **Alex Innerfire**: Gran Maestro dei Templari del Borgo, Cavaliere e Jedi, custode di Gladio e Gonnella.
- **Gabriele del Commercio**: figura storica degli Anarchici del Commercio, custode del Grinder.
- **L'Oracolo**: immortale, all'inizio entra in una fazione a caso; ogni 24 tick smaschera un infiltrato o rivela
  sul Piccione Viaggiatore chi ha una reliquia maggiore.
- **Reliquie minori**: Boccale (Re Anolino), Gonnella e Gladio (Alex Innerfire), Fallo (Diablo), Grinder
  (Gabriele del Commercio).
- **Leader del giocatore**: "Il Capo", leader degli Anarchici del Commercio (fazione del giocatore nella demo).

**Alternative.** Ogni punto è modificabile nei file `data/*.ron`: dimmi come li immaginavi e li adeguo.
