#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "$0")/.." && pwd)"
cd "$project_dir"

endpoint="${SUBSTREAMS_ENDPOINT:-robinhood.substreams.pinax.network:443}"
package="${SUBSTREAMS_PACKAGE:-./tiptag-substreams-v0.2.0.spkg}"
database_url="${DATABASE_URL:-postgres://dev:insecure@127.0.0.1:5432/tiptag_final_v5?sslmode=disable}"
noop_database_url="${NOOP_DATABASE_URL:-postgres://dev:insecure@127.0.0.1:5432/tiptag_noop_v2?sslmode=disable}"
postgres_container="${POSTGRES_CONTAINER:-postgres-substreams}"
postgres_user="${POSTGRES_USER:-dev}"
postgres_database="${POSTGRES_DATABASE:-tiptag_final_v5}"
start_block="${START_BLOCK:-6922897}"
target_block="${TARGET_BLOCK:-15217318}"
stop_block="$((target_block + 1))"
parallel_workers="${SUBSTREAMS_PARALLEL_WORKERS:-20}"

if [[ ! -f ./.substreams.env ]]; then
  echo "missing .substreams.env; run substreams auth first" >&2
  exit 1
fi

# shellcheck disable=SC1091
. ./.substreams.env

echo "[1/5] warming Substreams stores through block ${target_block}"
./bin/substreams-sink-sql run \
  "$noop_database_url" "$package" "${target_block}:${stop_block}" \
  -e "$endpoint" --noop-mode --max-retries -1 \
  -H "X-Substreams-Parallel-Workers: ${parallel_workers}"

echo "[2/5] writing the complete fixed-height PostgreSQL snapshot"
./bin/substreams-sink-sql run \
  "$database_url" "$package" "${start_block}:${stop_block}" \
  -e "$endpoint" --max-retries -1 \
  -H "X-Substreams-Parallel-Workers: ${parallel_workers}"

echo "[3/5] running strict legacy reconciliation"
docker exec -i "$postgres_container" \
  psql -U "$postgres_user" -d "$postgres_database" \
  < ./scripts/reconcile-fixed-15217318.sql

echo "[4/5] proving cursor/idempotent replay"
./bin/substreams-sink-sql run \
  "$database_url" "$package" "${start_block}:${stop_block}" \
  -e "$endpoint" --max-retries -1 \
  -H "X-Substreams-Parallel-Workers: ${parallel_workers}"

echo "[5/5] validating PostgreSQL I/U/D rollback semantics"
docker exec -i "$postgres_container" \
  psql -U "$postgres_user" -d "$postgres_database" \
  < ./scripts/test-reorg.sql

echo "full fixed-height acceptance passed"
