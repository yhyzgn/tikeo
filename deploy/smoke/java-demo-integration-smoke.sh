#!/usr/bin/env bash
set -euo pipefail


ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=deploy/smoke/lib/tikee-smoke-lib.sh
source "$ROOT_DIR/deploy/smoke/lib/tikee-smoke-lib.sh"
API_URL="${TIKEE_HTTP_URL:-http://127.0.0.1:19090}"
WORKER_ENDPOINT="${TIKEE_WORKER_ENDPOINT:-http://127.0.0.1:19998}"
DEMO_URL="${TIKEE_DEMO_URL:-http://127.0.0.1:18080}"
REPORT_DIR="${TIKEE_INTEGRATION_REPORT_DIR:-$ROOT_DIR/.dev/reports}"
RUN_ID="${TIKEE_INTEGRATION_RUN_ID:-java-demo-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
REPORT_FILE="$REPORT_DIR/${RUN_ID}.json"
WORKERS_FILE="$REPORT_DIR/${RUN_ID}-workers.json"
SERVER_LOG="$REPORT_DIR/${RUN_ID}-server.log"
JAVA_LOG="$REPORT_DIR/${RUN_ID}-java-demo.log"
SERVER_PID=""
JAVA_PID=""
OWN_SERVER=0
AUTH_TOKEN=""

mkdir -p "$REPORT_DIR"
TIKEE_SMOKE_RUN_ID="$RUN_ID"
TIKEE_SMOKE_CASES_FILE="$REPORT_DIR/${RUN_ID}-cases.jsonl"
export TIKEE_SMOKE_RUN_ID TIKEE_SMOKE_CASES_FILE
: > "$TIKEE_SMOKE_CASES_FILE"

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
need_cmd tee

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
  until api GET /api/v1/workers > "$WORKERS_FILE" && tikee_smoke_assert workers "$WORKERS_FILE" --client-instance spring-demo-worker --require-capability java --require-capability spring-boot --require-sdk-processor demo.echo --require-plugin-processor sql:billing.sql-sync >/dev/null
  do
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for spring-demo-worker online" >&2
      api GET /api/v1/workers >&2 || true
      [[ -f "$WORKERS_FILE" ]] && cat "$WORKERS_FILE" >&2 || true
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

wait_instance_terminal() {
  local instance_id="$1"
  local deadline=$((SECONDS + 90))
  local status=""
  while true; do
    status="$(api_json_get GET "/api/v1/instances/$instance_id" data.status)"
    case "$status" in
      succeeded|failed|cancelled|skipped)
        return 0
        ;;
    esac
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for instance $instance_id terminal status, got $status" >&2
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
    'scheduleType': schedule_type,
    'processorName': processor,
    'enabled': True,
}
if expr:
    body['scheduleExpr'] = expr
print(json.dumps(body))
PY
)"
  api_json_get POST /api/v1/jobs data.id "$body"
}

create_plugin_declaration() {
  local body
  body="$(python3 - "$RUN_ID" <<'PY'
import json, sys
run_id = sys.argv[1]
print(json.dumps({
    'name': f'{run_id}-billing-sql-plugin',
    'kind': 'processor',
    'processorTypes': [{
        'type': 'sql',
        'label': 'SQL Sync',
        'capability': 'sql',
        'processorNames': ['billing.sql-sync'],
        'description': 'Billing SQL sync processor used by Java demo',
        'artifactRef': None,
        'containerImage': None,
        'entrypoint': None,
        'checksum': None,
    }],
    'alertChannelTypes': [],
    'enabled': True,
}))
PY
)"
  api_json_get POST /api/v1/plugins data.id "$body"
}

create_plugin_job() {
  local name="$1"
  local processor_type="$2"
  local processor_name="$3"
  local body
  body="$(python3 - "$RUN_ID" "$name" "$processor_type" "$processor_name" <<'PY'
import json, sys
run_id, name, processor_type, processor_name = sys.argv[1:]
print(json.dumps({
    'namespace': 'default',
    'app': 'default',
    'name': f'{run_id}-{name}',
    'scheduleType': 'api',
    'processorType': processor_type,
    'processorName': processor_name,
    'enabled': True,
}))
PY
)"
  api_json_get POST /api/v1/jobs data.id "$body"
}

assert_invalid_plugin_job_rejected() {
  local body status response
  body="$(python3 - "$RUN_ID" <<'PY'
import json, sys
run_id = sys.argv[1]
print(json.dumps({
    'namespace': 'default',
    'app': 'default',
    'name': f'{run_id}-bad-plugin-job',
    'scheduleType': 'api',
    'processorType': 'sql',
    'processorName': 'mixed.sql',
    'enabled': True,
}))
PY
)"
  response="$REPORT_DIR/${RUN_ID}-bad-plugin-job-response.json"
  status="$(curl -sS -o "$response" -w '%{http_code}' -X POST "$(api_path /api/v1/jobs)" \
    -H "authorization: Bearer $AUTH_TOKEN" \
    -H 'content-type: application/json' \
    -d "$body")"
  if [[ "$status" == "200" || "$status" == "201" ]]; then
    echo "invalid plugin processor job unexpectedly succeeded" >&2
    cat "$response" >&2 || true
    return 1
  fi
  python3 - "$response" <<'PY'
import json, sys
payload = json.load(open(sys.argv[1], encoding='utf-8'))
message = str(payload.get('message', ''))
if 'plugin processorName is not declared' not in message:
    raise SystemExit(f'unexpected invalid plugin rejection message: {message}')
print('invalid plugin processor rejection expectation passed')
PY
}

trigger_job() {
  local job_id="$1"
  local mode="${2:-single}"
  api_json_get POST "/api/v1/jobs/$job_id:trigger" data.id "{\"triggerType\":\"api\",\"executionMode\":\"$mode\"}"
}

disable_job() {
  local job_id="$1"
  api PATCH "/api/v1/jobs/$job_id" '{"enabled":false}' >/dev/null
}

create_script() {
  local name="$1"
  local language="$2"
  local content="$3"
  local body
  body="$(python3 - "$RUN_ID" "$name" "$language" "$content" <<'PY'
import json, sys
run_id, name, language, content = sys.argv[1:]
print(json.dumps({
    'name': f'{run_id}-{name}',
    'language': language,
    'version': '1.0.0',
    'content': content,
    'timeout_seconds': 30,
    'max_memory_bytes': 67108864,
    'allow_network': False,
}))
PY
)"
  api_json_get POST /api/v1/scripts data.id "$body"
}

publish_script() {
  local script_id="$1"
  api_json_get POST "/api/v1/scripts/$script_id/publish" data.status '{}'
}

create_script_job() {
  local name="$1"
  local script_id="$2"
  local body
  body="$(python3 - "$RUN_ID" "$name" "$script_id" <<'PY'
import json, sys
run_id, name, script_id = sys.argv[1:]
print(json.dumps({
    'namespace': 'default',
    'app': 'default',
    'name': f'{run_id}-{name}',
    'scheduleType': 'api',
    'scriptId': script_id,
    'enabled': True,
}))
PY
)"
  api_json_get POST /api/v1/jobs data.id "$body"
}

assert_script_terminal() {
  local instance_file="$1"
  local logs_file="$2"
  local success_text="$3"
  python3 - "$instance_file" "$logs_file" "$success_text" <<'PY'
import json, sys
inst = json.load(open(sys.argv[1], encoding='utf-8'))['data']
logs = json.load(open(sys.argv[2], encoding='utf-8'))['data']
success_text = sys.argv[3]
status = inst.get('status')
if status not in {'succeeded', 'failed'}:
    raise SystemExit(f'script instance did not reach a governed terminal state: {status}')
if not inst.get('workerId') and not inst.get('worker_id'):
    raise SystemExit('script instance has no worker id')
items = logs.get('items') if isinstance(logs, dict) else logs
if not items:
    raise SystemExit('script terminal instance has no execution logs')
messages = '\n'.join(str(item.get('message', '')) for item in items)
if status == 'succeeded' and success_text not in messages:
    raise SystemExit(f'succeeded script logs do not contain expected text: {success_text}')
print(f'script terminal status={status}')
PY
}

run_governed_script_case() {
  local language="$1"
  local name="$2"
  local content="$3"
  local success_text="$4"
  local script publish_status job instance file logs status
  script="$(create_script "$name" "$language" "$content")"
  publish_status="$(publish_script "$script")"
  if [[ "$publish_status" != "approved" ]]; then
    echo "$language script publish did not approve script: $publish_status" >&2
    return 1
  fi
  job="$(create_script_job "$name-job" "$script")"
  instance="$(trigger_job "$job" single)"
  wait_instance_terminal "$instance"
  file="$REPORT_DIR/${RUN_ID}-${instance}.json"
  logs="$REPORT_DIR/${RUN_ID}-${instance}-logs.json"
  api GET "/api/v1/instances/$instance" > "$file"
  api GET "/api/v1/instances/$instance/logs" > "$logs"
  assert_script_terminal "$file" "$logs" "$success_text"
  status="$(api_json_get GET "/api/v1/instances/$instance" data.status)"
  tikee_smoke_record_case "script-${language}-terminal" passed "$file $logs" "$language script reached $status without queue starvation"
}

assert_queue_drained() {
  local queue_file="$1"
  api GET /api/v1/dispatch-queue > "$queue_file"
  python3 - "$queue_file" <<'PY'
import json, sys
payload = json.load(open(sys.argv[1], encoding='utf-8'))['data']
pending = int(payload.get('pending', 0))
running = int(payload.get('running', 0))
if pending != 0 or running != 0:
    raise SystemExit(f'dispatch queue not drained: pending={pending} running={running}')
print(f'dispatch queue drained: done={payload.get("done", 0)} failed={payload.get("failed", 0)}')
PY
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
  local username="${TIKEE_SMOKE_ADMIN_USERNAME:-smoke_admin}"
  local email="${TIKEE_SMOKE_ADMIN_EMAIL:-smoke.admin@example.com}"
  local password="${TIKEE_SMOKE_ADMIN_PASSWORD:-Tikee@2026!}"
  local registration_open
  registration_open="$(curl -fsS "$(api_path /api/v1/auth/bootstrap)" | json_get data.registrationOpen)"
  if [[ "$registration_open" == "True" || "$registration_open" == "true" ]]; then
    AUTH_TOKEN="$(curl -fsS -X POST "$(api_path /api/v1/auth/bootstrap/register)" \
      -H 'content-type: application/json' \
      -d "{\"username\":\"$username\",\"email\":\"$email\",\"password\":\"$password\",\"confirmPassword\":\"$password\"}" | json_get data.token)"
  else
    AUTH_TOKEN="$(curl -fsS -X POST "$(api_path /api/v1/auth/login)" \
      -H 'content-type: application/json' \
      -d "{\"username\":\"$username\",\"password\":\"$password\"}" | json_get data.token)"
  fi
  TIKEE_SMOKE_AUTH_TOKEN="$AUTH_TOKEN"
  export TIKEE_SMOKE_AUTH_TOKEN
}

start_java_demo() {
  (
    cd "$ROOT_DIR/examples/java/spring-boot3-worker-demo"
    TIKEE_WORKER_DRY_RUN=false \
    TIKEE_WORKER_ENDPOINT="$WORKER_ENDPOINT" \
    TIKEE_DEMO_SERVER_PORT="${DEMO_URL##*:}" \
    TIKEE_WORKER_CLIENT_INSTANCE_ID="${TIKEE_WORKER_CLIENT_INSTANCE_ID:-spring-demo-worker}" \
    TIKEE_WORKER_STATE_DIR="${TIKEE_WORKER_STATE_DIR:-$REPORT_DIR/${RUN_ID}-worker-state}" \
    ./scripts/run-demo-worker.sh >"$JAVA_LOG" 2>&1
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

  local echo_file echo_logs fail_file fail_logs broadcast_file broadcast_attempts
  echo_file="$REPORT_DIR/${RUN_ID}-${echo_instance}.json"
  echo_logs="$REPORT_DIR/${RUN_ID}-${echo_instance}-logs.json"
  fail_file="$REPORT_DIR/${RUN_ID}-${fail_instance}.json"
  fail_logs="$REPORT_DIR/${RUN_ID}-${fail_instance}-logs.json"
  broadcast_file="$REPORT_DIR/${RUN_ID}-${broadcast_instance}.json"
  broadcast_attempts="$REPORT_DIR/${RUN_ID}-${broadcast_instance}-attempts.json"
  api GET "/api/v1/instances/$echo_instance" > "$echo_file"
  api GET "/api/v1/instances/$echo_instance/logs" > "$echo_logs"
  api GET "/api/v1/instances/$fail_instance" > "$fail_file"
  api GET "/api/v1/instances/$fail_instance/logs" > "$fail_logs"
  api GET "/api/v1/instances/$broadcast_instance" > "$broadcast_file"
  api GET "/api/v1/instances/$broadcast_instance/attempts" > "$broadcast_attempts"
  tikee_smoke_assert instance "$echo_file" --expected-status succeeded --require-worker --min-log-count 1 --logs-file "$echo_logs" --require-log-text demo.echo --forbid-duplicate-logs >/dev/null
  tikee_smoke_assert instance "$fail_file" --expected-status failed --min-log-count 1 --logs-file "$fail_logs" --require-log-text demo.fail --forbid-duplicate-logs >/dev/null
  tikee_smoke_assert attempts "$broadcast_attempts" --min-attempts 1 --expected-status succeeded >/dev/null
  tikee_smoke_record_case worker-registration passed "$WORKERS_FILE" "spring demo worker registered with structured capabilities"
  tikee_smoke_record_case api-single-success passed "$echo_file $echo_logs" "demo.echo reached succeeded and emitted logs"
  tikee_smoke_record_case api-single-failure passed "$fail_file $fail_logs" "demo.fail reached failed with logs"
  tikee_smoke_record_case api-broadcast-success passed "$broadcast_file $broadcast_attempts" "broadcast attempt succeeded"

  fixed_instance="$(wait_job_instance_status "$fixed_job" succeeded fixed_rate)"
  cron_instance="$(wait_job_instance_status "$cron_job" succeeded cron)"
  disable_job "$fixed_job"
  disable_job "$cron_job"

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
      'jobId': job_id,
      'processorName': 'demo.workflow.step',
      'childWorkflowId': None,
      'mapItems': None,
      'config': None,
    }],
    'edges': [],
  },
}))
PY
)"
  workflow_id="$(api_json_get POST /api/v1/workflows data.id "$wf_body")"
  workflow_instance="$(api_json_get POST "/api/v1/workflows/$workflow_id/run" data.id '{"triggerType":"api"}')"
  materialized_job_instance="$(api_json_get POST /api/v1/workflow-instances/materialize-next data.node.jobInstanceId '{}')"
  wait_instance_status "$materialized_job_instance" succeeded
  local workflow_job_file workflow_job_logs
  workflow_job_file="$REPORT_DIR/${RUN_ID}-${materialized_job_instance}.json"
  workflow_job_logs="$REPORT_DIR/${RUN_ID}-${materialized_job_instance}-logs.json"
  api GET "/api/v1/instances/$materialized_job_instance" > "$workflow_job_file"
  api GET "/api/v1/instances/$materialized_job_instance/logs" > "$workflow_job_logs"
  tikee_smoke_assert instance "$workflow_job_file" --expected-status succeeded --require-worker --min-log-count 1 --logs-file "$workflow_job_logs" --require-log-text demo.workflow.step --forbid-duplicate-logs >/dev/null
  tikee_smoke_record_case workflow-job-success passed "$workflow_job_file $workflow_job_logs" "workflow materialized Java job succeeded with logs"

  local plugin_id plugin_file plugin_job plugin_instance plugin_instance_file plugin_logs
  plugin_id="$(create_plugin_declaration)"
  plugin_file="$REPORT_DIR/${RUN_ID}-plugin.json"
  api GET "/api/v1/plugins" > "$plugin_file"
  python3 - "$plugin_file" <<'PY'
import json, sys
items = json.load(open(sys.argv[1], encoding='utf-8'))['data']
plugin = next(
    item for item in items
    if any(pt.get('type') == 'sql' and 'billing.sql-sync' in pt.get('processorNames', [])
           for pt in item.get('processorTypes', []))
)
assert plugin['enabled'] is True
print('plugin structured processor expectation passed')
PY
  plugin_job="$(create_plugin_job plugin-sql-job sql billing.sql-sync)"
  assert_invalid_plugin_job_rejected
  plugin_instance="$(trigger_job "$plugin_job" single)"
  wait_instance_status "$plugin_instance" succeeded
  plugin_instance_file="$REPORT_DIR/${RUN_ID}-${plugin_instance}.json"
  plugin_logs="$REPORT_DIR/${RUN_ID}-${plugin_instance}-logs.json"
  api GET "/api/v1/instances/$plugin_instance" > "$plugin_instance_file"
  api GET "/api/v1/instances/$plugin_instance/logs" > "$plugin_logs"
  tikee_smoke_assert instance "$plugin_instance_file" --expected-status succeeded --require-worker --min-log-count 1 --logs-file "$plugin_logs" --require-log-text billing.sql-sync --forbid-duplicate-logs >/dev/null
  tikee_smoke_record_case plugin-processor-registration passed "$plugin_file" "created structured sql plugin processor declaration id=$plugin_id"
  tikee_smoke_record_case plugin-job-validation passed "$REPORT_DIR/${RUN_ID}-bad-plugin-job-response.json" "invalid plugin processor mixed.sql was rejected"
  tikee_smoke_record_case plugin-job-success passed "$plugin_instance_file $plugin_logs" "plugin processor billing.sql-sync executed and persisted logs"

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

  local shell_script shell_publish_status shell_job shell_instance shell_file shell_logs queue_file
  shell_script="$(create_script shell-smoke shell 'echo tikee-shell-smoke')"
  shell_publish_status="$(publish_script "$shell_script")"
  if [[ "$shell_publish_status" != "approved" ]]; then
    echo "shell script publish did not approve script: $shell_publish_status" >&2
    return 1
  fi
  shell_job="$(create_script_job shell-script-job "$shell_script")"
  shell_instance="$(trigger_job "$shell_job" single)"
  wait_instance_status "$shell_instance" succeeded
  shell_file="$REPORT_DIR/${RUN_ID}-${shell_instance}.json"
  shell_logs="$REPORT_DIR/${RUN_ID}-${shell_instance}-logs.json"
  api GET "/api/v1/instances/$shell_instance" > "$shell_file"
  api GET "/api/v1/instances/$shell_instance/logs" > "$shell_logs"
  tikee_smoke_assert instance "$shell_file" --expected-status succeeded --require-worker --min-log-count 1 --logs-file "$shell_logs" --require-log-text tikee-shell-smoke --forbid-duplicate-logs >/dev/null
  tikee_smoke_record_case script-shell-success passed "$shell_file $shell_logs" "shell script executed through worker sandbox and persisted stdout logs"

  run_governed_script_case python python-smoke 'print("tikee-python-smoke")' tikee-python-smoke
  run_governed_script_case javascript js-smoke 'console.log("tikee-js-smoke");' tikee-js-smoke
  run_governed_script_case typescript ts-smoke 'const msg: string = "tikee-ts-smoke"; console.log(msg);' tikee-ts-smoke
  run_governed_script_case rhai rhai-smoke 'print("tikee-rhai-smoke");' tikee-rhai-smoke

  queue_file="$REPORT_DIR/${RUN_ID}-dispatch-queue.json"
  assert_queue_drained "$queue_file"
  tikee_smoke_record_case dispatch-queue-drained passed "$queue_file" "dispatch queue has zero pending/running items after smoke"

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
extra_cases = []
case_file = report_file.rsplit("/", 1)[0] + "/" + run_id + "-cases.jsonl"
try:
    with open(case_file, encoding="utf-8") as fh:
        extra_cases = [json.loads(line) for line in fh if line.strip()]
except FileNotFoundError:
    pass
report["functional_cases"] = extra_cases
with open(report_file, 'w', encoding='utf-8') as fh:
    json.dump(report, fh, ensure_ascii=False, indent=2)
print(json.dumps(report, ensure_ascii=False, indent=2))
PY
  echo "report: $REPORT_FILE"
}

main "$@"
