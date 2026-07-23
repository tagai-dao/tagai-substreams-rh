#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?Set DATABASE_URL to the PostgreSQL sink DSN}"

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGE_PATH="${PACKAGE_PATH:-${PROJECT_DIR}/tiptag-substreams-v0.2.0.spkg}"
ENDPOINT="${SUBSTREAMS_ENDPOINT:-robinhood.substreams.pinax.network:443}"
START_BLOCK="${START_BLOCK:-6922897}"
STOP_BLOCK="${STOP_BLOCK:-}"
MODULE_HASH_MISMATCH_POLICY="${MODULE_HASH_MISMATCH_POLICY:-error}"
CURSORS_TABLE="${CURSORS_TABLE:-cursors}"
HISTORY_TABLE="${HISTORY_TABLE:-substreams_history}"
MAX_RETRIES="${MAX_RETRIES:--1}"

if [[ -f "${PROJECT_DIR}/.substreams.env" ]]; then
  # shellcheck disable=SC1091
  source "${PROJECT_DIR}/.substreams.env"
fi

args=(
  run
  "${DATABASE_URL}"
  "${PACKAGE_PATH}"
  --start-block "${START_BLOCK}"
  --endpoint "${ENDPOINT}"
  --max-retries "${MAX_RETRIES}"
  --on-module-hash-mismatch "${MODULE_HASH_MISMATCH_POLICY}"
  --cursors-table "${CURSORS_TABLE}"
  --history-table "${HISTORY_TABLE}"
)

if [[ -n "${STOP_BLOCK}" ]]; then
  args+=(--stop-block "${STOP_BLOCK}")
fi

exec "${PROJECT_DIR}/bin/substreams-sink-sql" "${args[@]}"
