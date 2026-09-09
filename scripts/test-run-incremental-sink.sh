#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT

FAKE_BIN="${TEST_DIR}/bin"
mkdir -p "${FAKE_BIN}"

printf '%s\n' \
  '#!/usr/bin/env bash' \
  'printf '\''{"jsonrpc":"2.0","id":1,"result":"%s"}\n'\'' "${FAKE_LATEST_HEX}"' \
  >"${FAKE_BIN}/curl"

printf '%s\n' \
  '#!/usr/bin/env bash' \
  'printf '\''%s\n'\'' "$*" >>"${FAKE_PSQL_LOG}"' \
  'cat "${FAKE_CURSOR_FILE}"' \
  >"${FAKE_BIN}/psql"

printf '%s\n' \
  '#!/usr/bin/env bash' \
  'if [[ -n "${FAKE_CURSOR_AFTER:-}" ]]; then' \
  '  printf '\''%s\n'\'' "${FAKE_CURSOR_AFTER}" >"${FAKE_CURSOR_FILE}"' \
  'fi' \
  'exit "${FAKE_SINK_STATUS:-0}"' \
  >"${FAKE_BIN}/fake-sink"

chmod +x "${FAKE_BIN}/curl" "${FAKE_BIN}/psql" "${FAKE_BIN}/fake-sink"

CURSOR_FILE="${TEST_DIR}/cursor"
STATE_FILE="${TEST_DIR}/window.state"
PSQL_LOG="${TEST_DIR}/psql.log"

run_incremental() {
  env \
    PATH="${FAKE_BIN}:${PATH}" \
    DATABASE_URL='postgres://test@localhost/test' \
    PACKAGE_PATH='/tmp/test.spkg' \
    START_BLOCK=100 \
    LATEST_LAG_BLOCKS=0 \
    INCREMENTAL_MAX_BLOCKS=1000 \
    INCREMENTAL_MAX_BLOCKS_CEILING=4000 \
    INCREMENTAL_WINDOW_GROWTH_FACTOR=2 \
    INCREMENTAL_CURSOR_ID='0123456789abcdef0123456789abcdef01234567' \
    INCREMENTAL_WINDOW_STATE_FILE="${STATE_FILE}" \
    INCREMENTAL_SINK_RUNNER="${FAKE_BIN}/fake-sink" \
    FAKE_CURSOR_FILE="${CURSOR_FILE}" \
    FAKE_PSQL_LOG="${PSQL_LOG}" \
    FAKE_LATEST_HEX='0xf4240' \
    FAKE_CURSOR_AFTER="${FAKE_CURSOR_AFTER:-}" \
    FAKE_SINK_STATUS="${FAKE_SINK_STATUS:-0}" \
    bash "${PROJECT_DIR}/scripts/run-incremental-sink.sh"
}

assert_contains() {
  local haystack="$1"
  local needle="$2"

  if [[ "${haystack}" != *"${needle}"* ]]; then
    printf 'expected output to contain: %s\nactual output:\n%s\n' "${needle}" "${haystack}" >&2
    exit 1
  fi
}

printf '99\n' >"${CURSOR_FILE}"
first_output="$(run_incremental)"
assert_contains "${first_output}" '"maxBlocks":1000'
assert_contains "${first_output}" '"nextWindowBlocks":2000'
assert_contains "$(<"${STATE_FILE}")" 'next_window=2000'
assert_contains "$(<"${PSQL_LOG}")" "WHERE id = '0123456789abcdef0123456789abcdef01234567'"
if grep -q ":'cursor_id'" "${PSQL_LOG}"; then
  echo "psql command still contains an unsubstituted cursor variable" >&2
  exit 1
fi

second_output="$(run_incremental)"
assert_contains "${second_output}" '"maxBlocks":2000'
assert_contains "${second_output}" '"nextWindowBlocks":4000'

FAKE_CURSOR_AFTER=1500
export FAKE_CURSOR_AFTER
advanced_output="$(run_incremental)"
unset FAKE_CURSOR_AFTER
assert_contains "${advanced_output}" '"cursorBlockBefore":99'
assert_contains "${advanced_output}" '"cursorBlockAfter":1500'
assert_contains "${advanced_output}" '"nextWindowBlocks":1000'
assert_contains "${advanced_output}" '"reason":"cursor_advanced"'

reset_output="$(run_incremental)"
assert_contains "${reset_output}" '"maxBlocks":1000'

state_before_error="$(<"${STATE_FILE}")"
FAKE_SINK_STATUS=7
export FAKE_SINK_STATUS
set +e
error_output="$(run_incremental 2>&1)"
error_status=$?
set -e
unset FAKE_SINK_STATUS

if (( error_status != 7 )); then
  printf 'expected sink failure status 7, got %d\n' "${error_status}" >&2
  exit 1
fi
assert_contains "${error_output}" '"event":"incremental_window_unchanged_after_error"'
if [[ "$(<"${STATE_FILE}")" != "${state_before_error}" ]]; then
  echo 'window state changed after a failed sink run' >&2
  exit 1
fi

echo 'incremental adaptive window tests passed'
