#!/usr/bin/env bash
# Lance le pipeline « mots » : word_service (gRPC) puis processing_service (HTTP + logique jeu).
# À lancer en premier avant le(s) web_server(s). Logs dans .run-logs/

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_DIR="$ROOT_DIR/.run-logs"
mkdir -p "$LOG_DIR"

WORD_HOST="${WORD_HOST:-127.0.0.1}"
WORD_PORT="${WORD_PORT:-50051}"
PROCESSING_HOST="${PROCESSING_HOST:-127.0.0.1}"
PROCESSING_PORT="${PROCESSING_PORT:-3001}"

export WORD_SERVICE_GRPC="${WORD_SERVICE_GRPC:-http://${WORD_HOST}:${WORD_PORT}}"
export PROCESSING_BIND="${PROCESSING_BIND:-${PROCESSING_HOST}:${PROCESSING_PORT}}"

pids=()

cleanup() {
  echo ""
  echo "[backend] Arrêt des services…"
  for pid in "${pids[@]:-}"; do
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  done
  wait 2>/dev/null || true
  echo "[backend] Terminé."
}

start_bg() {
  local name="$1"
  local workdir="$2"
  local logfile="$3"
  shift 3
  echo "[backend] Démarrage ${name} → ${logfile}"
  (
    cd "$workdir"
    "$@"
  ) >"$logfile" 2>&1 &
  pids+=("$!")
  echo "         pid=$! "
}

trap cleanup INT TERM EXIT

start_bg "word_service" "$ROOT_DIR/word_service" "$LOG_DIR/word_service.log" cargo run
sleep 1

start_bg "processing_service" "$ROOT_DIR/processing_service" "$LOG_DIR/processing_service.log" \
  env WORD_SERVICE_GRPC="$WORD_SERVICE_GRPC" PROCESSING_BIND="$PROCESSING_BIND" cargo run

echo ""
echo "[backend] OK — word_service : ${WORD_SERVICE_GRPC}"
echo "          processing_service : http://${PROCESSING_BIND}"
echo "          (pense à avoir PostgreSQL / DATABASE_URL si besoin)"
echo ""
echo "Ctrl+C pour tout couper."

wait
