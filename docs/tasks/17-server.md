# 17 — Telemetria, Admin, MCP, Compendium

Priorità: 17 (una versione base arriva con la demo).

- `metrics` + `metrics-exporter-prometheus`, `tracing`; endpoint `/metrics`.
- Admin REST (`axum`): stato, entità, ispezione Utility AI, parametri (lettura/scrittura live), sprite/UI mapping,
  comandi, feed, mercato, fazioni, compendium.
- MCP: JSON-RPC 2.0 su HTTP `POST /mcp` e su stdio (`--mcp-stdio`); tool `get_world_state`,
  `set_parameter`, `spawn_entity`, `trigger_event`.
- `CompendiumExporter` → `compendium.json`.
