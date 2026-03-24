#!/usr/bin/env bash
# Stack complète pour tests : word_service + processing + **deux** web_server (ports différents).
# Utile pour vérifier que plusieurs frontaux tapent le même backend (PROCESSING_BASE identique).

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_DIR="$ROOT_DIR/.run-logs"
mkdir -p "$LOG_DIR"

WORD_HOST="${WORD_HOST:-127.0.0.1}"
WORD_PORT="${WORD_PORT:-50051}"
PROCESSING_HOST="${PROCESSING_HOST:-127.0.0.1}"
PROCESSING_PORT="${PROCESSING_PORT:-3001}"
WEB1_HOST="${WEB1_HOST:-127.0.0.1}"
WEB1_PORT="${WEB1_PORT:-3000}"
WEB2_HOST="${WEB2_HOST:-127.0.0.1}"
WEB2_PORT="${WEB2_PORT:-3002}"

WORD_SERVICE_GRPC="${WORD_SERVICE_GRPC:-http://${WORD_HOST}:${WORD_PORT}}"
PROCESSING_BASE="${PROCESSING_BASE:-http://${PROCESSING_HOST}:${PROCESSING_PORT}}"
PROCESSING_BIND="${PROCESSING_BIND:-${PROCESSING_HOST}:${PROCESSING_PORT}}"
WEB1_BIND="${WEB1_BIND:-${WEB1_HOST}:${WEB1_PORT}}"
WEB2_BIND="${WEB2_BIND:-${WEB2_HOST}:${WEB2_PORT}}"

pids=()

cleanup() {
  echo
  echo "[multi] Arrêt des processus…"
  for pid in "${pids[@]:-}"; do
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  done
  wait || true
  echo "[multi] Fin."
}

start_service() {
  local name="$1"
  local workdir="$2"
  local logfile="$3"
  shift 3

  echo "Lancement ${name}..."
  (
    cd "$workdir"
    "$@"
  ) >"$logfile" 2>&1 &
  local pid=$!
  pids+=("$pid")
  echo "  PID=${pid} | logs: $logfile"
}

trap cleanup INT TERM EXIT

start_service \
  "word_service" \
  "$ROOT_DIR/word_service" \
  "$LOG_DIR/word_service.log" \
  cargo run

sleep 1

start_service \
  "processing_service" \
  "$ROOT_DIR/processing_service" \
  "$LOG_DIR/processing_service.log" \
  env WORD_SERVICE_GRPC="$WORD_SERVICE_GRPC" PROCESSING_BIND="$PROCESSING_BIND" cargo run

sleep 1

start_service \
  "web_server_1 (${WEB1_BIND})" \
  "$ROOT_DIR/web_server" \
  "$LOG_DIR/web_server_1.log" \
  env WEB_BIND="$WEB1_BIND" PROCESSING_BASE="$PROCESSING_BASE" cargo run

start_service \
  "web_server_2 (${WEB2_BIND})" \
  "$ROOT_DIR/web_server" \
  "$LOG_DIR/web_server_2.log" \
  env WEB_BIND="$WEB2_BIND" PROCESSING_BASE="$PROCESSING_BASE" cargo run

echo
echo "Services lances :"
echo "  - word_service       : ${WORD_SERVICE_GRPC}"
echo "  - processing_service : http://${PROCESSING_BIND}"
echo "  - web_server_1       : http://${WEB1_BIND}"
echo "  - web_server_2       : http://${WEB2_BIND}"
echo
echo "Ouvre par exemple :"
echo "  - http://${WEB1_BIND}"
echo "  - http://${WEB2_BIND}"
echo
echo "Ctrl+C pour tout arreter."

wait
