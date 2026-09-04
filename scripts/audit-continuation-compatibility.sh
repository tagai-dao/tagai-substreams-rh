#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 4 ]]; then
  echo "usage: $0 <production.spkg> <candidate.spkg> [production-output] [candidate-output]" >&2
  exit 2
fi

production_package="$1"
candidate_package="$2"
production_output="${3:-db_out}"
candidate_output="${4:-db_out}"

for package in "$production_package" "$candidate_package"; do
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
    ' | sort
}

temporary_directory="$(mktemp -d)"
trap 'rm -rf "$temporary_directory"' EXIT

extract_hashes "$production_package" "$production_output" \
  >"$temporary_directory/production"
extract_hashes "$candidate_package" "$candidate_output" \
  >"$temporary_directory/candidate"

join -t $'\t' \
  "$temporary_directory/production" \
  "$temporary_directory/candidate" \
  >"$temporary_directory/shared"

shared_count=0
unchanged_count=0
changed_count=0

printf '%-44s %-10s %s\n' MODULE STATUS HASHES
while IFS=$'\t' read -r name production_hash candidate_hash; do
  ((shared_count += 1))
  if [[ "$production_hash" == "$candidate_hash" ]]; then
    ((unchanged_count += 1))
    printf '%-44s %-10s %s\n' "$name" unchanged "$production_hash"
  else
    ((changed_count += 1))
    printf '%-44s %-10s %s -> %s\n' \
      "$name" changed "$production_hash" "$candidate_hash"
  fi
done <"$temporary_directory/shared"

echo
printf 'shared=%d unchanged=%d changed=%d\n' \
  "$shared_count" "$unchanged_count" "$changed_count"

if (( changed_count > 0 )); then
  cat >&2 <<'EOF'
continuation compatibility failed: shared module hashes changed

Do not copy or promote the production cursor into this candidate package.
Preserve the exact production modules in a continuation package, isolate the
new contract modules, or perform a full blue/green replay.
EOF
  exit 1
fi

echo "continuation compatibility passed: every shared module hash is unchanged"
