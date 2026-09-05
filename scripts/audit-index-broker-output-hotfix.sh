#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <exact-cutover.spkg> <hotfix.spkg>" >&2
  exit 2
fi

base_package="$1"
hotfix_package="$2"
output_module="v11_continuation_db_out"

for package in "$base_package" "$hotfix_package"; do
  if [[ ! -f "$package" ]]; then
    echo "package not found: $package" >&2
    exit 2
  fi
done

extract_hashes() {
  local package="$1"

  substreams info "$package" "$output_module" |
    awk '
      /^Name: / {
        name = substr($0, 7)
        next
      }
      /^Hash: / && name != "" {
        print name "\t" $2
        name = ""
      }
    '
}

declare -A base_hashes
declare -A hotfix_hashes

while IFS=$'\t' read -r name hash; do
  base_hashes["$name"]="$hash"
done < <(extract_hashes "$base_package")

while IFS=$'\t' read -r name hash; do
  hotfix_hashes["$name"]="$hash"
done < <(extract_hashes "$hotfix_package")

failed=0
changed=0
printf '%-44s %-10s %s\n' MODULE STATUS HASHES
for name in "${!base_hashes[@]}"; do
  base_hash="${base_hashes[$name]}"
  hotfix_hash="${hotfix_hashes[$name]:-}"
  if [[ -z "$hotfix_hash" ]]; then
    printf '%-44s %-10s %s\n' "$name" missing "$base_hash"
    failed=1
    continue
  fi

  if [[ "$base_hash" == "$hotfix_hash" ]]; then
    printf '%-44s %-10s %s\n' "$name" unchanged "$base_hash"
    case "$name" in
    index_broker_db_out | v11_backfill_db_out | v11_continuation_db_out)
      echo "required hotfix module did not change: $name" >&2
      failed=1
      ;;
    esac
    continue
  fi

  ((changed += 1))
  printf '%-44s %-10s %s -> %s\n' "$name" changed "$base_hash" "$hotfix_hash"
  case "$name" in
  index_broker_db_out | v11_backfill_db_out | v11_continuation_db_out) ;;
  *)
    echo "unexpected changed module: $name" >&2
    failed=1
    ;;
  esac
done

for name in "${!hotfix_hashes[@]}"; do
  if [[ -z "${base_hashes[$name]:-}" ]]; then
    echo "unexpected module added by hotfix: $name" >&2
    failed=1
  fi
done

if (( changed != 3 )); then
  echo "expected exactly 3 changed hashes, found $changed" >&2
  failed=1
fi
if (( failed != 0 )); then
  echo "IndexBroker output hotfix compatibility failed" >&2
  exit 1
fi

echo
echo "IndexBroker output hotfix compatibility passed"
echo "only index_broker_db_out and its two stateless merge outputs changed"
