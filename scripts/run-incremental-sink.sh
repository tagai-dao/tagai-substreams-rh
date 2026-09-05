#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HEAD_RPC_URL="${HEAD_RPC_URL:-https://rpc.mainnet.chain.robinhood.com}"
LATEST_LAG_BLOCKS="${LATEST_LAG_BLOCKS:-100}"
INCREMENTAL_MAX_BLOCKS="${INCREMENTAL_MAX_BLOCKS:-100000}"
MAX_RETRIES="${MAX_RETRIES:-3}"

if ! [[ "${LATEST_LAG_BLOCKS}" =~ ^[0-9]+$ ]]; then
  echo "LATEST_LAG_BLOCKS must be a non-negative integer" >&2
  exit 1
fi

if ! [[ "${INCREMENTAL_MAX_BLOCKS}" =~ ^[0-9]+$ ]] || (( INCREMENTAL_MAX_BLOCKS == 0 )); then
  echo "INCREMENTAL_MAX_BLOCKS must be a positive integer" >&2
  exit 1
fi

if ! [[ "${START_BLOCK:-}" =~ ^[0-9]+$ ]]; then
  echo "START_BLOCK must be a non-negative integer" >&2
  exit 1
fi

: "${DATABASE_URL:?Set DATABASE_URL to the PostgreSQL sink DSN}"

CURSORS_TABLE="${CURSORS_TABLE:-cursors}"
HISTORY_TABLE="${HISTORY_TABLE:-substreams_history}"

if ! [[ "${CURSORS_TABLE}" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
  echo "CURSORS_TABLE must be a valid unqualified PostgreSQL identifier" >&2
  exit 1
fi

head_response="$(
  curl --fail --silent --show-error \
    --connect-timeout 10 \
    --max-time 30 \
    -X POST "${HEAD_RPC_URL}" \
    -H 'Content-Type: application/json' \
    --data '{"jsonrpc":"2.0","id":1,"method":"eth_blockNumber","params":[]}'
)"

latest_hex="$(jq -er '.result' <<<"${head_response}")"
if ! [[ "${latest_hex}" =~ ^0x[0-9a-fA-F]+$ ]]; then
  echo "RPC returned an invalid latest block: ${latest_hex}" >&2
  exit 1
fi

latest_block=$((16#${latest_hex#0x}))
if (( latest_block <= LATEST_LAG_BLOCKS )); then
  echo "latest block ${latest_block} is not greater than lag ${LATEST_LAG_BLOCKS}" >&2
  exit 1
fi

chain_target_block=$((latest_block - LATEST_LAG_BLOCKS))

cursor_block="$(
  psql \
    "${DATABASE_URL}" \
    --no-psqlrc \
    --tuples-only \
    --no-align \
    --set ON_ERROR_STOP=1 \
    --command "SELECT COALESCE(MAX(block_num), 0) FROM \"${CURSORS_TABLE}\";"
)"
cursor_block="${cursor_block//[[:space:]]/}"

if ! [[ "${cursor_block}" =~ ^[0-9]+$ ]]; then
  echo "PostgreSQL returned an invalid cursor block: ${cursor_block}" >&2
  exit 1
fi

resolved_start_block="${START_BLOCK}"
if [[ -n "${cursor_block}" ]] && (( cursor_block > 0 )); then
  resolved_start_block=$((cursor_block + 1))
fi

if (( resolved_start_block > chain_target_block )); then
  printf \
    '{"event":"incremental_up_to_date","latestBlock":%d,"lagBlocks":%d,"chainTargetBlock":%d,"resolvedStartBlock":%d}\n' \
    "${latest_block}" \
    "${LATEST_LAG_BLOCKS}" \
    "${chain_target_block}" \
    "${resolved_start_block}"
  exit 0
fi

batch_target_block=$((resolved_start_block + INCREMENTAL_MAX_BLOCKS - 1))
target_block="${chain_target_block}"
if (( target_block > batch_target_block )); then
  target_block="${batch_target_block}"
fi

# substreams-sink-sql treats stop block as exclusive.
export STOP_BLOCK=$((target_block + 1))
export MAX_RETRIES

printf \
  '{"event":"incremental_target","latestBlock":%d,"lagBlocks":%d,"chainTargetBlock":%d,"resolvedStartBlock":%d,"maxBlocks":%d,"targetBlock":%d,"stopBlockExclusive":%d}\n' \
  "${latest_block}" \
  "${LATEST_LAG_BLOCKS}" \
  "${chain_target_block}" \
  "${resolved_start_block}" \
  "${INCREMENTAL_MAX_BLOCKS}" \
  "${target_block}" \
  "${STOP_BLOCK}"

exec "${PROJECT_DIR}/scripts/run-sink.sh"
