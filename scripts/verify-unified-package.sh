#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 <legacy-continuation.spkg> <basket.spkg> <unified.spkg>" >&2
  exit 2
fi

legacy_package="$1"
basket_package="$2"
unified_package="$3"

for package in "$legacy_package" "$basket_package" "$unified_package"; do
  if [[ ! -f "$package" ]]; then
    echo "package not found: $package" >&2
    exit 2
  fi
done

extract_hashes() {
  local package="$1"
  local output_module="$2"

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

temporary_directory="$(mktemp -d)"
trap 'rm -rf "$temporary_directory"' EXIT

extract_hashes "$legacy_package" db_out >"$temporary_directory/legacy"
extract_hashes "$basket_package" basket_db_out >"$temporary_directory/basket"
extract_hashes "$unified_package" db_out >"$temporary_directory/unified"

declare -A unified_hashes
while IFS=$'\t' read -r name hash; do
  unified_hashes["$name"]="$hash"
done <"$temporary_directory/unified"

compare_module() {
  local source_label="$1"
  local module_name="$2"
  local expected_hash="$3"
  local actual_hash="${unified_hashes[$module_name]:-}"

  if [[ -z "$actual_hash" ]]; then
    echo "unified package is missing $source_label module: $module_name" >&2
    return 1
  fi
  if [[ "$actual_hash" != "$expected_hash" ]]; then
    echo "$source_label module hash mismatch: $module_name" >&2
    echo "  expected: $expected_hash" >&2
    echo "  actual:   $actual_hash" >&2
    return 1
  fi
}

while IFS=$'\t' read -r name hash; do
  case "$name" in
  db_out | map_basket_registry_events | store_basket_addresses | map_basket_events)
    continue
    ;;
  esac
  compare_module legacy "$name" "$hash"
done <"$temporary_directory/legacy"

while IFS=$'\t' read -r name hash; do
  case "$name" in
  map_basket_registry_events | store_basket_addresses | map_basket_events)
    compare_module Basket "$name" "$hash"
    ;;
  esac
done <"$temporary_directory/basket"

echo "unified package verification passed"
echo "all legacy upstream hashes and Basket upstream hashes were preserved"
