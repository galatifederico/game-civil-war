# 10 — Trasporto del server MCP

**Contesto.** MCP (Model Context Protocol) si usa su stdio o su HTTP.

**Decisione presa.** Entrambi: `POST /mcp` (JSON-RPC 2.0, risposta JSON diretta, senza SSE) quando il server è
acceso con `--serve`, e `--mcp-stdio` per i client che lanciano il processo. Versione di protocollo dichiarata
`2025-06-18`. Tool richiesti dal design più quattro di servizio (`list_entities`, `inspect_entity`,
`send_command`, `recent_events`). Nessuna autenticazione: il server ascolta su 127.0.0.1 per default.

**Alternative.** Streamable HTTP completo con SSE e sessioni; autenticazione con token per esporlo in rete.
