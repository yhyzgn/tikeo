#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
API_URL="${TIKEE_HTTP_URL:-http://127.0.0.1:19090}"
WORKER_ENDPOINT="${TIKEE_WORKER_ENDPOINT:-http://127.0.0.1:19998}"
DEMO_URL="${TIKEE_DEMO_URL:-http://127.0.0.1:18080}"
REPORT_DIR="${TIKEE_INTEGRATION_REPORT_DIR:-$ROOT_DIR/.dev/reports}"
RUN_ID="${TIKEE_INTEGRATION_RUN_ID:-java-demo-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
REPORT_FILE="$REPORT_DIR/${RUN_ID}.json"
SERVER_LOG="$REPORT_DIR/${RUN_ID}-server.log"
JAVA_LOG="$REPORT_DIR/${RUN_ID}-java-demo.log"
SERVER_PID=""
JAVA_PID=""
OWN_SERVER=0
AUTH_TOKEN=""

mkdir -p "$REPORT_DIR"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing command: $1" >&2
    exit 127
  fi
}

cleanup() {
  local code=$?
  if [[ -n "$JAVA_PID" ]] && kill -0 "$JAVA_PID" >/dev/null 2>&1; then
    kill "$JAVA_PID" >/dev/null 2>&1 || true
    wait "$JAVA_PID" 2>/dev/null || true
  fi
  if [[ "$OWN_SERVER" == "1" && -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  exit "$code"
}
trap cleanup EXIT INT TERM

need_cmd cargo
need_cmd curl
need_cmd python3

api_path() {
  local path="$1"
  printf '%s%s' "$API_URL" "$path"
}

api() {
  local method="$1"
  local path="$2"
  local body="${3:-}"
  if [[ -n "$body" ]]; then
    curl -fsS -X "$method" "$(api_path "$path")" \
      -H "authorization: Bearer $AUTH_TOKEN" \
      -H 'content-type: application/json' \
      -d "$body"
  else
    curl -fsS -X "$method" "$(api_path "$path")" \
      -H "authorization: Bearer $AUTH_TOKEN"
  fi
}

json_get() {
  python3 -c 'import json,sys; cur=json.load(sys.stdin)
for part in sys.argv[1].split("."):
    if part:
        cur = cur[int(part)] if isinstance(cur, list) else cur[part]
print(cur)' "$1"
}

api_json_get() {
  local method="$1"
  local path="$2"
  local selector="$3"
  local body="${4:-}"
  if [[ -n "$body" ]]; then
    api "$method" "$path" "$body" | json_get "$selector"
  else
    api "$method" "$path" | json_get "$selector"
  fi
}

wait_for_ready() {
  local label="$1"
  local url="$2"
  local deadline=$((SECONDS + 90))
  until curl -fsS "$url" >/dev/null 2>&1; do
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for $label at $url" >&2
      [[ -f "$SERVER_LOG" ]] && tail -n 120 "$SERVER_LOG" >&2 || true
      [[ -f "$JAVA_LOG" ]] && tail -n 120 "$JAVA_LOG" >&2 || true
      return 1
    fi
    sleep 1
  done
}

wait_for_worker() {
  local deadline=$((SECONDS + 90))
  until python3 - "$API_URL" "$AUTH_TOKEN" <<'PY'
import json, sys, urllib.request
api_url, token = sys.argv[1:3]
request = urllib.request.Request(f"{api_url}/api/v1/workers", headers={"authorization": f"Bearer {token}"})
try:
    with urllib.request.urlopen(request, timeout=5) as response:
        payload = json.load(response)
except Exception:
    sys.exit(1)
items = payload.get('data', {}).get('items', [])
for worker in items:
    if worker.get('client_instance_id') == 'spring-demo-worker' and worker.get('status') == 'online':
        caps = set(worker.get('capabilities') or [])
        if {'java', 'spring-boot'} <= caps:
            sys.exit(0)
sys.exit(1)
PY
  do
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for spring-demo-worker online" >&2
      api GET /api/v1/workers >&2 || true
      tail -n 120 "$JAVA_LOG" >&2 || true
      return 1
    fi
    sleep 1
  done
}

wait_instance_status() {
  local instance_id="$1"
  local expected="$2"
  local deadline=$((SECONDS + 90))
  local status=""
  until [[ "$status" == "$expected" ]]; do
    status="$(api_json_get GET "/api/v1/instances/$instance_id" data.status)"
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for instance $instance_id status $expected, got $status" >&2
      api GET "/api/v1/instances/$instance_id" >&2 || true
      api GET "/api/v1/instances/$instance_id/logs" >&2 || true
      tail -n 120 "$SERVER_LOG" >&2 || true
      tail -n 120 "$JAVA_LOG" >&2 || true
      return 1
    fi
    sleep 1
  done
}

wait_job_instance_status() {
  local job_id="$1"
  local expected="$2"
  local trigger_type="${3:-}"
  local deadline=$((SECONDS + 90))
  local found=""
  until [[ -n "$found" ]]; do
    found="$(python3 - "$API_URL" "$AUTH_TOKEN" "$job_id" "$expected" "$trigger_type" <<'PY'
import json, sys, urllib.request
api_url, token, job_id, expected, trigger = sys.argv[1:6]
request = urllib.request.Request(f"{api_url}/api/v1/jobs/{job_id}/instances", headers={"authorization": f"Bearer {token}"})
try:
    with urllib.request.urlopen(request, timeout=5) as response:
        payload = json.load(response)
except Exception:
    sys.exit(0)
for item in payload.get('data', {}).get('items', []):
    if item.get('status') == expected and (not trigger or item.get('trigger_type') == trigger):
        print(item['id'])
        break
PY
)"
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for job $job_id instance status $expected trigger=$trigger_type" >&2
      api GET "/api/v1/jobs/$job_id/instances" >&2 || true
      return 1
    fi
    sleep 1
  done
  printf '%s' "$found"
}

create_job() {
  local name="$1"
  local schedule_type="$2"
  local processor="$3"
  local schedule_expr="${4:-}"
  local body
  body="$(python3 - "$RUN_ID" "$name" "$schedule_type" "$processor" "$schedule_expr" <<'PY'
import json, sys
run_id, name, schedule_type, processor, expr = sys.argv[1:]
body = {
    'namespace': 'default',
    'app': 'default',
    'name': f'{run_id}-{name}',
    'schedule_type': schedule_type,
    'processor_name': processor,
    'enabled': True,
}
if expr:
    body['schedule_expr'] = expr
print(json.dumps(body))
PY
)"
  api_json_get POST /api/v1/jobs data.id "$body"
}

trigger_job() {
  local job_id="$1"
  local mode="${2:-single}"
  api_json_get POST "/api/v1/jobs/$job_id:trigger" data.id "{\"trigger_type\":\"api\",\"execution_mode\":\"$mode\"}"
}

start_server_if_needed() {
  if curl -fsS "$(api_path /readyz)" >/dev/null 2>&1; then
    return
  fi
  OWN_SERVER=1
  local config="$REPORT_DIR/${RUN_ID}-config.toml"
  cat > "$config" <<CFG
[server]
listen_addr = "127.0.0.1:19090"
worker_tunnel_addr = "127.0.0.1:19998"

[storage]
database_url = "sqlite://$REPORT_DIR/${RUN_ID}.db?mode=rwc"

[cluster]
mode = "standalone"
node_id = "standalone"
peers = []

[auth]
local_login_enabled = true

[auth.api_tokens]
default_ttl_seconds = 43200
min_ttl_seconds = 300
max_ttl_seconds = 2592000

[auth.oidc]
enabled = false
scopes = ["openid", "profile", "email"]

[transport_security.http]
tls_enabled = false
mtls_required = false

[transport_security.worker_tunnel]
tls_enabled = false
mtls_required = false

[observability.tracing]
enabled = false
headers = []

[alert_retry]
enabled = false
interval_seconds = 60
batch_size = 50
max_attempts = 3
backoff_seconds = 300

[script_governance]
CFG
  (cd "$ROOT_DIR" && cargo run --bin tikee -- serve --config "$config" >"$SERVER_LOG" 2>&1) &
  SERVER_PID=$!
  wait_for_ready server "$(api_path /readyz)"
}

login() {
  AUTH_TOKEN="$(curl -fsS -X POST "$(api_path /api/v1/auth/login)" \
    -H 'content-type: application/json' \
    -d '{"username":"tikee_init","password":"Tikee@2026!"}' | json_get data.token)"
}

start_java_demo() {
  (
    cd "$ROOT_DIR/examples/java/spring-worker-demo"
    TIKEE_WORKER_DRY_RUN=false \
    TIKEE_WORKER_ENDPOINT="$WORKER_ENDPOINT" \
    TIKEE_DEMO_SERVER_PORT="${DEMO_URL##*:}" \
    ./gradlew bootRun --no-daemon >"$JAVA_LOG" 2>&1
  ) &
  JAVA_PID=$!
  wait_for_ready java-demo "$DEMO_URL/demo/health"
  wait_for_worker
}

main() {
  start_server_if_needed
  login
  start_java_demo

  local echo_job fail_job broadcast_job fixed_job cron_job workflow_job
  echo_job="$(create_job api-echo api demo.echo)"
  fail_job="$(create_job api-fail api demo.fail)"
  broadcast_job="$(create_job broadcast-context api demo.context)"
  fixed_job="$(create_job fixed-heartbeat fixed_rate demo.heartbeat 1s)"
  cron_job="$(create_job cron-report cron demo.report '0/2 * * * * * *')"
  workflow_job="$(create_job workflow-step api demo.workflow.step)"

  local echo_instance fail_instance broadcast_instance fixed_instance cron_instance workflow_id workflow_instance materialized_job_instance
  echo_instance="$(trigger_job "$echo_job" single)"
  fail_instance="$(trigger_job "$fail_job" single)"
  broadcast_instance="$(trigger_job "$broadcast_job" broadcast)"
  wait_instance_status "$echo_instance" succeeded
  wait_instance_status "$fail_instance" failed
  wait_instance_status "$broadcast_instance" succeeded

  fixed_instance="$(wait_job_instance_status "$fixed_job" succeeded fixed_rate)"
  cron_instance="$(wait_job_instance_status "$cron_job" succeeded cron)"

  local wf_body
  wf_body="$(python3 - "$RUN_ID" "$workflow_job" <<'PY'
import json, sys
run_id, job_id = sys.argv[1:]
print(json.dumps({
  'name': f'{run_id}-workflow',
  'definition': {
    'nodes': [{
      'key': 'java-step',
      'name': 'Java step',
      'kind': 'job',
      'job_id': job_id,
      'processor_name': 'demo.workflow.step',
      'child_workflow_id': None,
      'map_items': None,
      'config': None,
    }],
    'edges': [],
  },
}))
PY
)"
  workflow_id="$(api_json_get POST /api/v1/workflows data.id "$wf_body")"
  workflow_instance="$(api_json_get POST "/api/v1/workflows/$workflow_id/run" data.id '{"trigger_type":"api"}')"
  materialized_job_instance="$(api_json_get POST /api/v1/workflow-instances/materialize-next data.node.job_instance_id '{}')"
  wait_instance_status "$materialized_job_instance" succeeded

  local workflow_status=""
  local deadline=$((SECONDS + 90))
  until [[ "$workflow_status" == "succeeded" ]]; do
    workflow_status="$(api_json_get GET "/api/v1/workflow-instances/$workflow_instance" data.status)"
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for workflow $workflow_instance succeeded, got $workflow_status" >&2
      api GET "/api/v1/workflow-instances/$workflow_instance" >&2 || true
      return 1
    fi
    sleep 1
  done

  python3 - "$REPORT_FILE" "$RUN_ID" "$API_URL" "$WORKER_ENDPOINT" "$DEMO_URL" \
    "$echo_job" "$echo_instance" "$fail_job" "$fail_instance" \
    "$broadcast_job" "$broadcast_instance" "$fixed_job" "$fixed_instance" \
    "$cron_job" "$cron_instance" "$workflow_id" "$workflow_instance" "$materialized_job_instance" <<'PY'
import json, sys, datetime
(
    report_file, run_id, api_url, worker_endpoint, demo_url,
    echo_job, echo_instance, fail_job, fail_instance,
    broadcast_job, broadcast_instance, fixed_job, fixed_instance,
    cron_job, cron_instance, workflow_id, workflow_instance, workflow_job_instance,
) = sys.argv[1:]
report = {
    'run_id': run_id,
    'generated_at': datetime.datetime.now(datetime.UTC).isoformat(),
    'api_url': api_url,
    'worker_endpoint': worker_endpoint,
    'demo_url': demo_url,
    'status': 'passed',
    'cases': [
        {'name': 'spring-boot-web-health', 'status': 'passed', 'url': demo_url + '/demo/health'},
        {'name': 'worker-registration', 'status': 'passed'},
        {'name': 'api-single-success', 'status': 'passed', 'job_id': echo_job, 'instance_id': echo_instance},
        {'name': 'api-single-failure', 'status': 'passed', 'job_id': fail_job, 'instance_id': fail_instance},
        {'name': 'api-broadcast-success', 'status': 'passed', 'job_id': broadcast_job, 'instance_id': broadcast_instance},
        {'name': 'fixed-rate-success', 'status': 'passed', 'job_id': fixed_job, 'instance_id': fixed_instance},
        {'name': 'cron-success', 'status': 'passed', 'job_id': cron_job, 'instance_id': cron_instance},
        {'name': 'workflow-job-success', 'status': 'passed', 'workflow_id': workflow_id, 'workflow_instance_id': workflow_instance, 'job_instance_id': workflow_job_instance},
    ],
}
with open(report_file, 'w', encoding='utf-8') as fh:
    json.dump(report, fh, ensure_ascii=False, indent=2)
print(json.dumps(report, ensure_ascii=False, indent=2))
PY
  echo "report: $REPORT_FILE"
}

main "$@"
