#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HEAD_RPC_URL="${HEAD_RPC_URL:-https://rpc.mainnet.chain.robinhood.com}"
LATEST_LAG_BLOCKS="${LATEST_LAG_BLOCKS:-100}"
MAX_RETRIES="${MAX_RETRIES:-3}"

if ! [[ "${LATEST_LAG_BLOCKS}" =~ ^[0-9]+$ ]]; then
  echo "LATEST_LAG_BLOCKS must be a non-negative integer" >&2
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

target_block=$((latest_block - LATEST_LAG_BLOCKS))

# substreams-sink-sql treats stop block as exclusive.
export STOP_BLOCK=$((target_block + 1))
export MAX_RETRIES

printf \
  '{"event":"incremental_target","latestBlock":%d,"lagBlocks":%d,"targetBlock":%d,"stopBlockExclusive":%d}\n' \
  "${latest_block}" \
  "${LATEST_LAG_BLOCKS}" \
  "${target_block}" \
  "${STOP_BLOCK}"

exec "${PROJECT_DIR}/scripts/run-sink.sh"
