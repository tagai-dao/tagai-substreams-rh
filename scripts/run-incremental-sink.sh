#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HEAD_RPC_URL="${HEAD_RPC_URL:-https://rpc.mainnet.chain.robinhood.com}"
LATEST_LAG_BLOCKS="${LATEST_LAG_BLOCKS:-100}"
INCREMENTAL_MAX_BLOCKS="${INCREMENTAL_MAX_BLOCKS:-100000}"
INCREMENTAL_MAX_BLOCKS_CEILING="${INCREMENTAL_MAX_BLOCKS_CEILING:-1600000}"
INCREMENTAL_WINDOW_GROWTH_FACTOR="${INCREMENTAL_WINDOW_GROWTH_FACTOR:-2}"
INCREMENTAL_CURSOR_ID="${INCREMENTAL_CURSOR_ID:-}"
INCREMENTAL_WINDOW_STATE_FILE="${INCREMENTAL_WINDOW_STATE_FILE:-${STATE_DIRECTORY:-${PROJECT_DIR}}/incremental-window.state}"
INCREMENTAL_SINK_RUNNER="${INCREMENTAL_SINK_RUNNER:-${PROJECT_DIR}/scripts/run-sink.sh}"
MAX_RETRIES="${MAX_RETRIES:-3}"

if ! [[ "${LATEST_LAG_BLOCKS}" =~ ^[0-9]+$ ]]; then
  echo "LATEST_LAG_BLOCKS must be a non-negative integer" >&2
  exit 1
fi

if ! [[ "${INCREMENTAL_MAX_BLOCKS}" =~ ^[0-9]+$ ]] || (( INCREMENTAL_MAX_BLOCKS == 0 )); then
  echo "INCREMENTAL_MAX_BLOCKS must be a positive integer" >&2
  exit 1
fi

if ! [[ "${INCREMENTAL_MAX_BLOCKS_CEILING}" =~ ^[0-9]+$ ]] ||
  (( INCREMENTAL_MAX_BLOCKS_CEILING < INCREMENTAL_MAX_BLOCKS )); then
  echo "INCREMENTAL_MAX_BLOCKS_CEILING must be an integer greater than or equal to INCREMENTAL_MAX_BLOCKS" >&2
  exit 1
fi

if ! [[ "${INCREMENTAL_WINDOW_GROWTH_FACTOR}" =~ ^[0-9]+$ ]] ||
  (( INCREMENTAL_WINDOW_GROWTH_FACTOR < 2 )); then
  echo "INCREMENTAL_WINDOW_GROWTH_FACTOR must be an integer greater than or equal to 2" >&2
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


read_cursor_block() {
  local query
  local value

  if [[ -n "${INCREMENTAL_CURSOR_ID}" ]]; then
    query="SELECT COALESCE(MAX(block_num), 0) FROM \"${CURSORS_TABLE}\" WHERE id = :'cursor_id';"
    value="$(
      psql \
        "${DATABASE_URL}" \
        --no-psqlrc \
        --tuples-only \
        --no-align \
        --set ON_ERROR_STOP=1 \
        --set "cursor_id=${INCREMENTAL_CURSOR_ID}" \
        --command "${query}"
    )"
  else
    query="SELECT COALESCE(MAX(block_num), 0) FROM \"${CURSORS_TABLE}\";"
    value="$(
      psql \
        "${DATABASE_URL}" \
        --no-psqlrc \
        --tuples-only \
        --no-align \
        --set ON_ERROR_STOP=1 \
        --command "${query}"
    )"
  fi

  value="${value//[[:space:]]/}"
  if ! [[ "${value}" =~ ^[0-9]+$ ]]; then
    echo "PostgreSQL returned an invalid cursor block: ${value}" >&2
    return 1
  fi

  printf '%s\n' "${value}"
}

grow_window() {
  local current="$1"

  if (( current >= INCREMENTAL_MAX_BLOCKS_CEILING )); then
    printf '%d\n' "${INCREMENTAL_MAX_BLOCKS_CEILING}"
  elif (( current > INCREMENTAL_MAX_BLOCKS_CEILING / INCREMENTAL_WINDOW_GROWTH_FACTOR )); then
    printf '%d\n' "${INCREMENTAL_MAX_BLOCKS_CEILING}"
  else
    printf '%d\n' "$((current * INCREMENTAL_WINDOW_GROWTH_FACTOR))"
  fi
}

state_scope="$(
  printf '%s\n' \
    "${DATABASE_URL}" \
    "${PACKAGE_PATH:-}" \
    "${START_BLOCK}" \
    "${CURSORS_TABLE}" \
    "${INCREMENTAL_CURSOR_ID}" |
    sha256sum |
    awk '{print $1}'
)"

effective_max_blocks="${INCREMENTAL_MAX_BLOCKS}"
if [[ -f "${INCREMENTAL_WINDOW_STATE_FILE}" ]]; then
  saved_scope="$(sed -n 's/^scope=//p' "${INCREMENTAL_WINDOW_STATE_FILE}" | head -n 1)"
  saved_next_window="$(sed -n 's/^next_window=//p' "${INCREMENTAL_WINDOW_STATE_FILE}" | head -n 1)"

  if [[ "${saved_scope}" == "${state_scope}" ]] &&
    [[ "${saved_next_window}" =~ ^[0-9]+$ ]] &&
    (( saved_next_window >= INCREMENTAL_MAX_BLOCKS )); then
    effective_max_blocks="${saved_next_window}"
    if (( effective_max_blocks > INCREMENTAL_MAX_BLOCKS_CEILING )); then
      effective_max_blocks="${INCREMENTAL_MAX_BLOCKS_CEILING}"
    fi
  fi
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

cursor_block="$(read_cursor_block)"

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

batch_target_block=$((resolved_start_block + effective_max_blocks - 1))
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
  "${effective_max_blocks}" \
  "${target_block}" \
  "${STOP_BLOCK}"

if "${INCREMENTAL_SINK_RUNNER}"; then
  sink_status=0
else
  sink_status=$?
fi

if (( sink_status != 0 )); then
  printf \
    '{"event":"incremental_window_unchanged_after_error","cursorBlock":%d,"windowBlocks":%d,"exitCode":%d}\n' \
    "${cursor_block}" \
    "${effective_max_blocks}" \
    "${sink_status}"
  exit "${sink_status}"
fi

cursor_block_after="$(read_cursor_block)"
if (( cursor_block_after < cursor_block )); then
  echo "PostgreSQL cursor moved backwards from ${cursor_block} to ${cursor_block_after}" >&2
  exit 1
fi

if (( cursor_block_after > cursor_block )); then
  next_window="${INCREMENTAL_MAX_BLOCKS}"
  window_reason="cursor_advanced"
else
  next_window="$(grow_window "${effective_max_blocks}")"
  window_reason="cursor_unchanged"
fi

state_dir="$(dirname "${INCREMENTAL_WINDOW_STATE_FILE}")"
mkdir -p "${state_dir}"
state_tmp="${INCREMENTAL_WINDOW_STATE_FILE}.tmp.$$"
umask 077
printf \
  'scope=%s\nnext_window=%d\nlast_cursor_block=%d\nlast_target_block=%d\nreason=%s\n' \
  "${state_scope}" \
  "${next_window}" \
  "${cursor_block_after}" \
  "${target_block}" \
  "${window_reason}" \
  >"${state_tmp}"
mv "${state_tmp}" "${INCREMENTAL_WINDOW_STATE_FILE}"

printf \
  '{"event":"incremental_window_state","cursorBlockBefore":%d,"cursorBlockAfter":%d,"windowBlocks":%d,"nextWindowBlocks":%d,"ceilingBlocks":%d,"reason":"%s"}\n' \
  "${cursor_block}" \
  "${cursor_block_after}" \
  "${effective_max_blocks}" \
  "${next_window}" \
  "${INCREMENTAL_MAX_BLOCKS_CEILING}" \
  "${window_reason}"
