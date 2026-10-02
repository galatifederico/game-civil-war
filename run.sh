#!/usr/bin/env bash
# Avvia il server della simulazione e il client Unity (build Linux), senza aprire l'editor.
#   ./run.sh            usa l'ultima build del client (la crea se manca)
#   ./run.sh --build    ricompila il client (l'editor di Unity deve essere chiuso)
#   ./run.sh --web      niente Unity: apre il client web nel browser
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
PORT="${PORT:-8787}"
UNITY="${UNITY:-$HOME/Unity/Hub/Editor/6000.3.24f1/Editor/Unity}"
CLIENT="$ROOT/unity-client/Builds/Linux/FidenzaClient.x86_64"
mkdir -p "$ROOT/logs"

echo "▸ Compilo il server…"
(cd "$ROOT/engine" && CARGO_BUILD_JOBS=2 cargo build --release -j 2 -q -p fidenza_world)

if [[ "${1:-}" != "--web" ]] && { [[ "${1:-}" == "--build" ]] || [[ ! -x "$CLIENT" ]]; }; then
  if pgrep -f "unityhub-unity-edito[r]" >/dev/null; then
    echo "L'editor di Unity è aperto su un progetto: chiudilo per compilare il client da riga di comando." >&2
    exit 1
  fi
  echo "▸ Compilo il client Unity (qualche minuto)…"
  "$UNITY" -batchmode -quit -projectPath "$ROOT/unity-client" -executeMethod ProjectSetup.BuildLinux -logFile "$ROOT/logs/unity-build.log"
fi

echo "▸ Avvio il server su http://127.0.0.1:$PORT (log in logs/server.log)"
(cd "$ROOT/engine" && exec ./target/release/fidenza_world --ticks 0 --serve "127.0.0.1:$PORT" --tick-ms 1000 --compendium "$ROOT/logs/compendium.json") >"$ROOT/logs/server.log" 2>&1 &
SERVER=$!
trap 'kill $SERVER 2>/dev/null; echo "▸ Server fermato"' EXIT
for _ in $(seq 1 60); do curl -s "localhost:$PORT/healthz" >/dev/null && break; sleep 0.5; done

if [[ "${1:-}" == "--web" ]]; then
  xdg-open "http://127.0.0.1:$PORT/ui/" >/dev/null 2>&1 || echo "Apri http://127.0.0.1:$PORT/ui/"
  echo "▸ Ctrl+C per fermare il server"
  wait $SERVER
else
  echo "▸ Avvio il client (chiudi la finestra per uscire)"
  "$CLIENT" -screen-fullscreen 0 -screen-width 1600 -screen-height 950 --server="http://127.0.0.1:$PORT" -logFile "$ROOT/logs/client.log"
fi
