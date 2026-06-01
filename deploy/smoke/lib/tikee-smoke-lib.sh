#!/usr/bin/env bash
# Shared helpers for tikee smoke tests. Source this file from scripts that run
# with `set -euo pipefail`.

TIKEE_SMOKE_ROOT_DIR="${TIKEE_SMOKE_ROOT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)}"
TIKEE_SMOKE_ASSERT="${TIKEE_SMOKE_ASSERT:-$TIKEE_SMOKE_ROOT_DIR/deploy/smoke/assert_tikee_expectations.py}"
TIKEE_SMOKE_REPORT_DIR="${TIKEE_SMOKE_REPORT_DIR:-$TIKEE_SMOKE_ROOT_DIR/.dev/reports}"
TIKEE_SMOKE_RUN_ID="${TIKEE_SMOKE_RUN_ID:-smoke-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
TIKEE_SMOKE_CASES_FILE="${TIKEE_SMOKE_CASES_FILE:-$TIKEE_SMOKE_REPORT_DIR/${TIKEE_SMOKE_RUN_ID}-cases.jsonl}"
TIKEE_SMOKE_AUTH_TOKEN="${TIKEE_SMOKE_AUTH_TOKEN:-}"
mkdir -p "$TIKEE_SMOKE_REPORT_DIR"
: > "$TIKEE_SMOKE_CASES_FILE"

tikee_smoke_need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing command: $1" >&2
    return 127
  fi
}

tikee_smoke_api_path() {
  local api_url="$1"
  local path="$2"
  printf '%s%s' "$api_url" "$path"
}

tikee_smoke_json_get() {
  python3 -c 'import json,sys
cur=json.load(sys.stdin)
for part in sys.argv[1].split("."):
    if part:
        cur = cur[int(part)] if isinstance(cur, list) else cur[part]
print(cur)' "$1"
}

tikee_smoke_record_case() {
  local id="$1"
  local status="$2"
  local evidence="${3:-}"
  local message="${4:-}"
  python3 - "$TIKEE_SMOKE_CASES_FILE" "$id" "$status" "$evidence" "$message" <<'PY'
import json, sys, datetime
path, case_id, status, evidence, message = sys.argv[1:]
with open(path, 'a', encoding='utf-8') as fh:
    json.dump({
        'id': case_id,
        'status': status,
        'evidence': evidence,
        'message': message,
        'recorded_at': datetime.datetime.now(datetime.UTC).isoformat(),
    }, fh, ensure_ascii=False)
    fh.write('\n')
PY
}

tikee_smoke_api() {
  local api_url="$1"
  local method="$2"
  local path="$3"
  local body="${4:-}"
  if (( $# >= 4 )); then
    shift 4
  else
    shift "$#"
  fi
  local headers=(-H "authorization: Bearer $TIKEE_SMOKE_AUTH_TOKEN")
  if [[ -n "$body" ]]; then
    curl -fsS -X "$method" "$(tikee_smoke_api_path "$api_url" "$path")" \
      "${headers[@]}" -H 'content-type: application/json' -d "$body" "$@"
  else
    curl -fsS -X "$method" "$(tikee_smoke_api_path "$api_url" "$path")" \
      "${headers[@]}" "$@"
  fi
}

tikee_smoke_api_json_get() {
  local api_url="$1"
  local method="$2"
  local path="$3"
  local selector="$4"
  local body="${5:-}"
  if [[ -n "$body" ]]; then
    tikee_smoke_api "$api_url" "$method" "$path" "$body" | tikee_smoke_json_get "$selector"
  else
    tikee_smoke_api "$api_url" "$method" "$path" | tikee_smoke_json_get "$selector"
  fi
}

tikee_smoke_wait_for_http() {
  local label="$1"
  local url="$2"
  local timeout_seconds="${3:-90}"
  local deadline=$((SECONDS + timeout_seconds))
  until curl -fsS "$url" >/dev/null 2>&1; do
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for $label at $url" >&2
      return 1
    fi
    sleep 1
  done
}

tikee_smoke_login() {
  local api_url="$1"
  local username="${2:-smoke_admin}"
  local password="${3:-Tikee@2026!}"
  local email="${TIKEE_SMOKE_ADMIN_EMAIL:-smoke.admin@example.com}"
  local registration_open
  registration_open="$(curl -fsS "$(tikee_smoke_api_path "$api_url" /api/v1/auth/bootstrap)" | tikee_smoke_json_get data.registrationOpen)"
  if [[ "$registration_open" == "True" || "$registration_open" == "true" ]]; then
    TIKEE_SMOKE_AUTH_TOKEN="$(curl -fsS -X POST "$(tikee_smoke_api_path "$api_url" /api/v1/auth/bootstrap/register)" \
      -H 'content-type: application/json' \
      -d "{\"username\":\"$username\",\"email\":\"$email\",\"password\":\"$password\",\"confirmPassword\":\"$password\"}" | tikee_smoke_json_get data.token)"
  else
    TIKEE_SMOKE_AUTH_TOKEN="$(curl -fsS -X POST "$(tikee_smoke_api_path "$api_url" /api/v1/auth/login)" \
      -H 'content-type: application/json' \
      -d "{\"username\":\"$username\",\"password\":\"$password\"}" | tikee_smoke_json_get data.token)"
  fi
  export TIKEE_SMOKE_AUTH_TOKEN
}

tikee_smoke_write_api() {
  local api_url="$1"
  local method="$2"
  local path="$3"
  local output="$4"
  local body="${5:-}"
  tikee_smoke_api "$api_url" "$method" "$path" "$body" > "$output"
}

tikee_smoke_assert() {
  python3 "$TIKEE_SMOKE_ASSERT" "$@"
}

tikee_smoke_wait_instance_status() {
  local api_url="$1"
  local instance_id="$2"
  local expected="$3"
  local output_file="$4"
  local timeout_seconds="${5:-90}"
  local deadline=$((SECONDS + timeout_seconds))
  local status=""
  until [[ "$status" == "$expected" ]]; do
    tikee_smoke_write_api "$api_url" GET "/api/v1/instances/$instance_id" "$output_file"
    status="$(tikee_smoke_json_get data.status < "$output_file")"
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for instance $instance_id status $expected, got $status" >&2
      cat "$output_file" >&2 || true
      tikee_smoke_api "$api_url" GET "/api/v1/instances/$instance_id/logs" >&2 || true
      return 1
    fi
    sleep 1
  done
}

tikee_smoke_wait_job_instance_status() {
  local api_url="$1"
  local job_id="$2"
  local expected="$3"
  local trigger_type="${4:-}"
  local output_file="$5"
  local timeout_seconds="${6:-90}"
  local deadline=$((SECONDS + timeout_seconds))
  local found=""
  until [[ -n "$found" ]]; do
    tikee_smoke_write_api "$api_url" GET "/api/v1/jobs/$job_id/instances" "$output_file"
    found="$(python3 - "$output_file" "$expected" "$trigger_type" <<'PY'
import json, sys
path, expected, trigger = sys.argv[1:4]
payload = json.load(open(path, encoding='utf-8'))
for item in payload.get('data', {}).get('items', []):
    if item.get('status') == expected and (not trigger or item.get('triggerType') == trigger or item.get('trigger_type') == trigger):
        print(item['id'])
        break
PY
)"
    if [[ -n "$found" ]]; then
      break
    fi
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for job $job_id instance status $expected trigger=$trigger_type" >&2
      cat "$output_file" >&2 || true
      return 1
    fi
    sleep 1
  done
  printf '%s' "$found"
}

tikee_smoke_finalize_report() {
  local output_file="$1"
  local status="${2:-passed}"
  python3 - "$TIKEE_SMOKE_CASES_FILE" "$output_file" "$TIKEE_SMOKE_RUN_ID" "$status" <<'PY'
import json, sys, datetime
cases_path, output_path, run_id, status = sys.argv[1:]
cases = []
try:
    with open(cases_path, encoding='utf-8') as fh:
        cases = [json.loads(line) for line in fh if line.strip()]
except FileNotFoundError:
    pass
report = {
    'run_id': run_id,
    'generated_at': datetime.datetime.now(datetime.UTC).isoformat(),
    'status': status,
    'cases': cases,
}
with open(output_path, 'w', encoding='utf-8') as fh:
    json.dump(report, fh, ensure_ascii=False, indent=2)
print(json.dumps(report, ensure_ascii=False, indent=2))
PY
}
