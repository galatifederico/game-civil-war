# 10 — Demo `main.rs`

Priorità: 10.

`cargo run -p fidenza_world` : carica `sim_core` + dati di Fidenza, esegue 100 tick e stampa la cronaca:
1. una pedina commette un crimine (sale il `WantedLevel`);
2. una pattuglia della Polizia Neutra perquisisce e arresta;
3. il Leader paga una tangente dal `GuildTreasury` e le accuse si azzerano;
4. un Giornalista assiste, pubblica uno scoop sul "Piccione Viaggiatore" e il prezzo di mercato cambia.

Con `--serve` resta acceso ed espone Admin API, `/metrics`, MCP e le API UI.
Un test di integrazione verifica i 4 punti.
