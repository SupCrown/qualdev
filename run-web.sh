#!/usr/bin/env bash
# Lance un seul web_server sur le port (ou l’adresse complète) voulu.
# Usage :
#   ./run-web.sh              → 127.0.0.1:3000 par défaut
#   ./run-web.sh 8080         → 127.0.0.1:8080
#   ./run-web.sh 0.0.0.0:9000 → bind explicite (LAN, VM, etc.)
#
# Le proxy pointe vers PROCESSING_BASE (défaut http://127.0.0.1:3001).

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_DIR="$ROOT_DIR/.run-logs"
mkdir -p "$LOG_DIR"

RAW="${1:-3000}"
if [[ "$RAW" == *:* ]]; then
  WEB_BIND="$RAW"
else
  WEB_BIND="127.0.0.1:${RAW}"
fi

export PROCESSING_BASE="${PROCESSING_BASE:-http://127.0.0.1:3001}"

echo "[web] WEB_BIND=${WEB_BIND}"
echo "[web] PROCESSING_BASE=${PROCESSING_BASE}"
echo "[web] Ouvre http://${WEB_BIND} (adapter si besoin)"
echo ""

cd "$ROOT_DIR/web_server"
LOGFILE="$LOG_DIR/web_server_${RAW//:/_}.log"
echo "[web] log : $LOGFILE"
env WEB_BIND="$WEB_BIND" PROCESSING_BASE="$PROCESSING_BASE" cargo run 2>&1 | tee "$LOGFILE"
