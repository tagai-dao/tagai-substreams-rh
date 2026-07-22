#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?Set DATABASE_URL to the PostgreSQL sink DSN}"

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGE_PATH="${PACKAGE_PATH:-${PROJECT_DIR}/tiptag-substreams-v0.1.0.spkg}"
ENDPOINT="${SUBSTREAMS_ENDPOINT:-robinhood.substreams.pinax.network:443}"
START_BLOCK="${START_BLOCK:-6922897}"
STOP_BLOCK="${STOP_BLOCK:-}"

if [[ -f "${PROJECT_DIR}/.substreams.env" ]]; then
  # shellcheck disable=SC1091
  source "${PROJECT_DIR}/.substreams.env"
fi

exec "${PROJECT_DIR}/bin/substreams-sink-sql" run \
  "${DATABASE_URL}" \
  "${PACKAGE_PATH}" \
  "${START_BLOCK}:${STOP_BLOCK}" \
  -e "${ENDPOINT}"
