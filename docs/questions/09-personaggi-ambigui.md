# 09 — Personaggi e classi con poca descrizione

**Contesto.** Alcuni elementi del design non hanno una descrizione meccanica.

**Decisioni prese.**
- **Yag**: mistico di supporto con "Benedizione Yag" (morale +5 e Punti Gay +1 a chi è vicino) --> è semplicemente un gay super, che però ha anche tutti i punti alpha. è il punto che  unisce gay e uomini alpha. Uno yag riesce a riprodursi con tutti, anche con gli uomini.
- **Mago Vergine**: guaritore ("Incantesimo Verginale" cura gli alleati vicini). Un po' guaritore, un po' attaccante. NOn ho ancora nessuna idea specifica.
- **I Presidenti della Repubblica**: un unico NPC "Il Presidente della Repubblica" che arriva in visita al tick 30
  (trigger `visita_presidenziale`), fazione neutrale "Istituzioni". 
- **Alex Innerfire**: è un mago, è strano, ha una propria lore, conosce tantissime cose in realtà, con i giusti rapporti ti da molti consigli e ti fa dei regali e ti aiuta, è un ottimo supporter.
- **Gabriele del Commercio**: figura storica degli Anarchici del Commercio, custode del Grinder.
- **L'Oracolo**: immortale, all'inizio entra in una fazione a caso; ogni 24 tick smaschera un infiltrato o rivela
  sul Piccione Viaggiatore chi ha una reliquia maggiore.
- **Reliquie minori**: Boccale (Re Anolino), Gonnella e Gladio (Alex Innerfire), Fallo (Diablo), Grinder
  (Gabriele del Commercio). --> no, le reliquei minori possono essere assegnate ai giocatori iniziali, e dopo la scelta vengono sparse nel gioco.
- **Leader del giocatore**: il leader è la pedina che può muovere il giocatore, è il capo della fazione, è colui che può fare alcuni azioni, ad esempio è l'unico che può costruire, ha accesso all'inventario etc...
**Alternative.** Ogni punto è modificabile nei file `data/*.ron`: dimmi come li immaginavi e li adeguo.
