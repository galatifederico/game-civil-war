#!/usr/bin/env bash
# Compila il client Unity come APK Android in unity-client/Builds/Android/FidenzaClient.apk.
# Con ~8 GB di RAM serve lo swap (IL2CPP da solo arriva a 2-3 GB):
#   sudo fallocate -l 8G /swapfile && sudo chmod 600 /swapfile && sudo mkswap /swapfile && sudo swapon /swapfile
# L'editor di Unity deve essere chiuso; meglio chiudere anche le app pesanti mentre compila.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
UNITY="${UNITY:-$HOME/Unity/Hub/Editor/6000.3.24f1/Editor/Unity}"
ANDROID="$(dirname "$UNITY")/Data/PlaybackEngines/AndroidPlayer"
APK="$ROOT/unity-client/Builds/Android/FidenzaClient.apk"
LOG="$ROOT/logs/unity-android.log"
mkdir -p "$ROOT/logs"

if pgrep -f "unityhub-unity-edito[r]" >/dev/null; then
  echo "L'editor di Unity è aperto su un progetto: chiudilo prima di compilare." >&2
  exit 1
fi
if pgrep -f "Unity.*-projectPath $ROOT/unity-clien[t]" >/dev/null; then
  echo "C'è già una build di Unity in corso su questo progetto: aspetta che finisca." >&2
  exit 1
fi

# RAM libera + swap: sotto i ~10 GB la build rischia di essere uccisa per memoria.
avail_mb=$(( $(awk '/MemAvailable/ {print $2}' /proc/meminfo) / 1024 ))
swap_mb=$(( $(awk '/SwapFree/ {print $2}' /proc/meminfo) / 1024 ))
echo "▸ Memoria disponibile: ${avail_mb} MB di RAM + ${swap_mb} MB di swap"
if (( avail_mb + swap_mb < 10000 )); then
  echo "  Attenzione: è poca, la build potrebbe fallire (vedi lo swap in cima a questo script)." >&2
fi

if [[ ! -d "$ANDROID/SDK/cmake/3.22.1" ]]; then
  echo "▸ Installo CMake 3.22.1 nell'SDK Android di Unity (serve alla build IL2CPP)…"
  yes | JAVA_HOME="$ANDROID/OpenJDK" "$ANDROID"/SDK/cmdline-tools/*/bin/sdkmanager --sdk_root="$ANDROID/SDK" --install "cmake;3.22.1" >/dev/null
fi

echo "▸ Compilo l'APK (la prima volta 15-30 minuti; log in logs/unity-android.log)…"
start=$SECONDS
rm -f "$APK"
"$UNITY" -batchmode -quit -buildTarget Android -projectPath "$ROOT/unity-client" \
  -executeMethod ProjectSetup.BuildAndroid -logFile "$LOG" || true

if [[ -f "$APK" ]]; then
  echo "▸ Fatto in $(( (SECONDS - start) / 60 )) min: $APK ($(du -h "$APK" | cut -f1))"
  echo "  Per installarlo e avviare il server: ./phone.sh"
else
  echo "✗ Build fallita. Ultimi errori dal log:" >&2
  grep -E "error CS|UnityException|What went wrong|Killed|Build Android" -A3 "$LOG" | tail -20 >&2 || true
  exit 1
fi
