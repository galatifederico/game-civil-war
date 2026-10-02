#!/usr/bin/env bash
# Gioca dal telefono Android: server raggiungibile dalla Wi-Fi di casa + client Unity come APK.
#   ./phone.sh            usa l'ultimo APK (lo crea se manca)
#   ./phone.sh --build    ricompila l'APK (l'editor di Unity deve essere chiuso)
# Con il telefono collegato via USB (debug USB attivo) l'APK viene installato da solo; altrimenti
# lo si scarica dal telefono all'indirizzo stampato qui sotto.
# Serve aver aperto le porte una volta: sudo ufw allow 8787:8788/tcp
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
PORT="${PORT:-8787}"
APK_PORT=$((PORT + 1))
UNITY="${UNITY:-$HOME/Unity/Hub/Editor/6000.3.24f1/Editor/Unity}"
ADB="${ADB:-$(dirname "$UNITY")/Data/PlaybackEngines/AndroidPlayer/SDK/platform-tools/adb}"
APK="$ROOT/unity-client/Builds/Android/FidenzaClient.apk"
mkdir -p "$ROOT/logs"

echo "▸ Compilo il server…"
(cd "$ROOT/engine" && CARGO_BUILD_JOBS=2 cargo build --release -j 2 -q -p fidenza_world)

if [[ "${1:-}" == "--build" ]] || [[ ! -f "$APK" ]]; then
  "$ROOT/build-android.sh"
fi

IP="$(ip -4 route get 1.1.1.1 2>/dev/null | sed -n 's/.* src \([0-9.]*\).*/\1/p')"
IP="${IP:-$(hostname -I | awk '{print $1}')}"

if [[ -x "$ADB" ]] && "$ADB" get-state >/dev/null 2>&1; then
  echo "▸ Telefono collegato via USB: installo l'APK…"
  "$ADB" install -r "$APK"
fi

echo "▸ Avvio il server su 0.0.0.0:$PORT (log in logs/server.log)"
(cd "$ROOT/engine" && exec ./target/release/fidenza_world --ticks 0 --serve "0.0.0.0:$PORT" --tick-ms 1000 --compendium "$ROOT/logs/compendium.json") >"$ROOT/logs/server.log" 2>&1 &
SERVER=$!
# Mini server per scaricare l'APK dal browser del telefono.
(cd "$(dirname "$APK")" && exec python3 -m http.server "$APK_PORT" --bind 0.0.0.0) >/dev/null 2>&1 &
FILES=$!
trap 'kill $SERVER $FILES 2>/dev/null; echo "▸ Server fermato"' EXIT
for _ in $(seq 1 60); do curl -s "localhost:$PORT/healthz" >/dev/null && break; sleep 0.5; done

cat <<EOF

  Sul telefono (stessa Wi-Fi):
    1. scarica e installa l'APK:   http://$IP:$APK_PORT/FidenzaClient.apk
    2. apri "Fidenza" e inserisci: $IP:$PORT

  Ctrl+C per fermare il server
EOF
wait $SERVER
