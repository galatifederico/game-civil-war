# 05 — Fazioni, gerarchia, tesoro, stipendi

Priorità: 5.

- `FactionRegistry`: relazioni bidirezionali (asimmetriche), ideologia per assi, fazioni neutrali con ruolo
  (ordine pubblico, stampa).
- `FactionMember { faction, rank }`, ranghi con livello e stipendio.
- `GuildTreasury`: saldo per fazione; tutti i guadagni dei membri (quota configurabile) confluiscono nel fondo.
- `PayrollEngine`: pagamento periodico per rango; stipendi mancati → aumento del dissenso.
- Giocatori: `PlayerRegistry` (fazione, leader, punti vittoria); il leader è un'entità `Leader`.
