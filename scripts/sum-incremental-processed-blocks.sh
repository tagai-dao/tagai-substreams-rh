#!/usr/bin/env bash
set -euo pipefail

SINCE="${1:?Usage: $0 <journalctl-since>}"
UNIT="${UNIT:-tiptag-unified-incremental.service}"

journalctl -u "${UNIT}" --since "${SINCE}" --no-pager -o cat |
  jq -Rr '
    fromjson?
    | if .message == "sinker configured" then
        "START"
      elif .message == "substreams stream stats"
        and (.progress_total_processed_blocks != null) then
        "STAT\t\(.progress_total_processed_blocks)\t\(.last_block)"
      else
        empty
      end
  ' |
  awk -F '\t' '
    $1 == "START" {
      if (started) {
        total += run_max
      }
      started = 1
      runs += 1
      run_max = 0
      next
    }
    $1 == "STAT" {
      if (($2 + 0) > run_max) {
        run_max = $2 + 0
      }
      last_block = $3
    }
    END {
      if (started) {
        total += run_max
      }
      printf "{\"runs\":%d,\"processedBlocks\":%d,\"lastBlock\":\"%s\"}\n",
        runs, total, last_block
    }
  '
