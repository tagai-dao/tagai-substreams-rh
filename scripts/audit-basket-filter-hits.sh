#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: $0 <from-block> <to-block> [chunk-size]" >&2
}

if (( $# < 2 || $# > 3 )); then
  usage
  exit 1
fi

FROM_BLOCK="$1"
TO_BLOCK="$2"
CHUNK_SIZE="${3:-500}"
RPC_URL="${RPC_URL:-https://rpc.mainnet.chain.robinhood.com}"

for value in "${FROM_BLOCK}" "${TO_BLOCK}" "${CHUNK_SIZE}"; do
  if ! [[ "${value}" =~ ^[0-9]+$ ]]; then
    usage
    exit 1
  fi
done

if (( FROM_BLOCK > TO_BLOCK || CHUNK_SIZE == 0 )); then
  usage
  exit 1
fi

fixed_addresses='[
  "0x1f997deb6c8ac7bb4134bc7c6bf23f623cda25c6",
  "0xc6c999fa94199da470a17806f04de85036f02a88",
  "0xd96e197f139b78e9f74555701f699aa051e0a50e",
  "0xc2526404423ed03ce8d2608f5b94300f0aafa1a2",
  "0x773c71be8b5e3c0c49d9576211d06e2f316aaf4a"
]'

dynamic_topics='[
  "0xc8ebb3117899d4977e166f3afca866eab0f7c92ff7ff409a7f0aae513f8ea415",
  "0xb178954b2b8d72198f669fcc42c6ab219225ed856f78566adab738f9fc0fca04",
  "0xd046302bdc39599e60a5981e59227dcd1c643f512f56f2e984390c3ac6eadedb",
  "0x69a8abd422e0487bf4c2dba25bc00ea123691a6f14db12c2eb6aa9ab1ecc51ec",
  "0x44193e6589368f66700032705cb4b8a760ac8eb5ece58921310857bcdede1e22",
  "0x00690ba9c38e6521dc26bc0d24ca6605f2ce1b07d886f650f8682d0f9f5ed051"
]'

labels=(
  "fixed_all"
  "registry"
  "hook"
  "router"
  "fee_auction"
  "rebalance_executor"
  "dynamic_all"
  "FeeAccrued"
  "HolderFeesClaimed"
  "CreatorFeesClaimed"
  "LauncherFeesClaimed"
  "FrontendFeesClaimed"
  "RedeemedInKind"
)

modes=(
  "addresses"
  "address"
  "address"
  "address"
  "address"
  "address"
  "topics"
  "topic"
  "topic"
  "topic"
  "topic"
  "topic"
  "topic"
)

values=(
  "${fixed_addresses}"
  "0x1f997deb6c8ac7bb4134bc7c6bf23f623cda25c6"
  "0xc6c999fa94199da470a17806f04de85036f02a88"
  "0xd96e197f139b78e9f74555701f699aa051e0a50e"
  "0xc2526404423ed03ce8d2608f5b94300f0aafa1a2"
  "0x773c71be8b5e3c0c49d9576211d06e2f316aaf4a"
  "${dynamic_topics}"
  "0xc8ebb3117899d4977e166f3afca866eab0f7c92ff7ff409a7f0aae513f8ea415"
  "0xb178954b2b8d72198f669fcc42c6ab219225ed856f78566adab738f9fc0fca04"
  "0xd046302bdc39599e60a5981e59227dcd1c643f512f56f2e984390c3ac6eadedb"
  "0x69a8abd422e0487bf4c2dba25bc00ea123691a6f14db12c2eb6aa9ab1ecc51ec"
  "0x44193e6589368f66700032705cb4b8a760ac8eb5ece58921310857bcdede1e22"
  "0x00690ba9c38e6521dc26bc0d24ca6605f2ce1b07d886f650f8682d0f9f5ed051"
)

build_payload() {
  local mode="$1"
  local value="$2"
  local from_hex="$3"
  local to_hex="$4"

  case "${mode}" in
    address)
      jq -cn \
        --arg from "${from_hex}" \
        --arg to "${to_hex}" \
        --arg value "${value}" \
        '{jsonrpc:"2.0",id:1,method:"eth_getLogs",params:[{
          fromBlock:$from,toBlock:$to,address:$value
        }]}'
      ;;
    addresses)
      jq -cn \
        --arg from "${from_hex}" \
        --arg to "${to_hex}" \
        --argjson value "${value}" \
        '{jsonrpc:"2.0",id:1,method:"eth_getLogs",params:[{
          fromBlock:$from,toBlock:$to,address:$value
        }]}'
      ;;
    topic)
      jq -cn \
        --arg from "${from_hex}" \
        --arg to "${to_hex}" \
        --arg value "${value}" \
        '{jsonrpc:"2.0",id:1,method:"eth_getLogs",params:[{
          fromBlock:$from,toBlock:$to,topics:[$value]
        }]}'
      ;;
    topics)
      jq -cn \
        --arg from "${from_hex}" \
        --arg to "${to_hex}" \
        --argjson value "${value}" \
        '{jsonrpc:"2.0",id:1,method:"eth_getLogs",params:[{
          fromBlock:$from,toBlock:$to,topics:[$value]
        }]}'
      ;;
    *)
      echo "Unsupported filter mode: ${mode}" >&2
      exit 1
      ;;
  esac
}

query_chunk() {
  local payload="$1"
  local response=""
  local attempt

  for attempt in 1 2 3; do
    if response="$(
      curl --fail --silent --show-error \
        --connect-timeout 10 \
        --max-time 45 \
        -X POST "${RPC_URL}" \
        -H 'Content-Type: application/json' \
        --data "${payload}"
    )" &&
      jq -e '.error == null and (.result | type == "array")' \
        >/dev/null <<<"${response}"; then
      jq -r '
        [
          (.result | length),
          ([.result[]?.blockNumber] | unique | length)
        ] | @tsv
      ' <<<"${response}"
      return 0
    fi

    if (( attempt < 3 )); then
      sleep 2
    fi
  done

  echo "RPC query failed after 3 attempts: ${response}" >&2
  return 1
}

total_range=$((TO_BLOCK - FROM_BLOCK + 1))
printf 'range=%d:%d blocks=%d chunk_size=%d\n' \
  "${FROM_BLOCK}" "${TO_BLOCK}" "${total_range}" "${CHUNK_SIZE}"
printf '%-24s %12s %14s %12s\n' \
  "filter" "logs" "unique_blocks" "block_pct"

for index in "${!labels[@]}"; do
  label="${labels[$index]}"
  mode="${modes[$index]}"
  value="${values[$index]}"
  total_logs=0
  total_blocks=0

  chunk_from="${FROM_BLOCK}"
  while (( chunk_from <= TO_BLOCK )); do
    chunk_to=$((chunk_from + CHUNK_SIZE - 1))
    if (( chunk_to > TO_BLOCK )); then
      chunk_to="${TO_BLOCK}"
    fi

    printf -v from_hex '0x%x' "${chunk_from}"
    printf -v to_hex '0x%x' "${chunk_to}"
    payload="$(build_payload "${mode}" "${value}" "${from_hex}" "${to_hex}")"
    read -r chunk_logs chunk_blocks < <(query_chunk "${payload}")
    total_logs=$((total_logs + chunk_logs))
    total_blocks=$((total_blocks + chunk_blocks))
    chunk_from=$((chunk_to + 1))
  done

  percentage="$(
    awk -v matched="${total_blocks}" -v total="${total_range}" \
      'BEGIN { printf "%.4f%%", (matched * 100) / total }'
  )"
  printf '%-24s %12d %14d %12s\n' \
    "${label}" "${total_logs}" "${total_blocks}" "${percentage}"
done
