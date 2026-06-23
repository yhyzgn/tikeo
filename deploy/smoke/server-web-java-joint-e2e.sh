#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=deploy/smoke/lib/tikeo-smoke-lib.sh
source "$ROOT_DIR/deploy/smoke/lib/tikeo-smoke-lib.sh"

API_URL="${TIKEO_HTTP_URL:-http://127.0.0.1:19090}"
WORKER_ENDPOINT="${TIKEO_WORKER_ENDPOINT:-http://127.0.0.1:19998}"
WEB_URL="${TIKEO_WEB_URL:-http://127.0.0.1:15173}"
WEB_PORT="${WEB_URL##*:}"
WEB_PORT="${WEB_PORT%%/*}"
REPORT_DIR="${TIKEO_JOINT_REPORT_DIR:-$TIKEO_SMOKE_REPORT_DIR}"
RUN_ID="${TIKEO_JOINT_RUN_ID:-joint-e2e-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
SERVER_LOG="$REPORT_DIR/${RUN_ID}-server.log"
WEB_LOG="$REPORT_DIR/${RUN_ID}-web.log"
JAVA_A_LOG="$REPORT_DIR/${RUN_ID}-java-a.log"
JAVA_B_LOG="$REPORT_DIR/${RUN_ID}-java-b.log"
JAVA_A_PORT=18080
JAVA_B_PORT=18081
SERVER_PID=""
WEB_PID=""
JAVA_A_PID=""
JAVA_B_PID=""
STARTED_JAVA_PID=""
OWN_SERVER=0
OWN_WEB=0
DEMO_NAMESPACE="${TIKEO_JOINT_DEMO_NAMESPACE:-dev-alpha}"
DEMO_APP="${TIKEO_JOINT_DEMO_APP:-orders}"
DEMO_WORKER_POOL="${TIKEO_JOINT_DEMO_WORKER_POOL:-boot3-blue}"
mkdir -p "$REPORT_DIR"

cleanup() {
  local code=$?
  stop_java_demo "$JAVA_A_PID" "$JAVA_A_PORT"
  stop_java_demo "$JAVA_B_PID" "$JAVA_B_PORT"
  if [[ "$OWN_WEB" == "1" && -n "$WEB_PID" ]] && kill -0 "$WEB_PID" >/dev/null 2>&1; then
    kill "$WEB_PID" >/dev/null 2>&1 || true
    wait "$WEB_PID" 2>/dev/null || true
  fi
  if [[ "$OWN_SERVER" == "1" && -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  exit "$code"
}
trap cleanup EXIT INT TERM

stop_java_demo() {
  local pid="${1:-}"
  local port="${2:-}"
  if [[ -n "$pid" ]] && kill -0 "$pid" >/dev/null 2>&1; then
    kill "$pid" >/dev/null 2>&1 || true
    wait "$pid" 2>/dev/null || true
  fi
  if [[ -n "$port" ]]; then
    fuser -k "${port}/tcp" >/dev/null 2>&1 || true
  fi
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || { echo "missing command: $1" >&2; exit 127; }
}
need_cmd cargo
need_cmd curl
need_cmd python3
need_cmd bun
need_cmd fuser

api() {
  tikeo_smoke_api "$API_URL" "$@"
}

api_json_get() {
  tikeo_smoke_api_json_get "$API_URL" "$@"
}

json_body() {
  python3 - "$@" <<'PY'
import json, sys
pairs = [arg.split('=', 1) for arg in sys.argv[1:]]
print(json.dumps({k: v for k, v in pairs}, ensure_ascii=False, separators=(',', ':')))
PY
}

exists_in_list() {
  local path="$1"
  shift
  api GET "$path" | python3 -c 'import json, sys
payload = json.load(sys.stdin)
criteria = dict(arg.split("=", 1) for arg in sys.argv[1:])
data = payload.get("data") or []
items = data.get("items", []) if isinstance(data, dict) else data
for item in items:
    if all(str(item.get(k)) == v for k, v in criteria.items()):
        sys.exit(0)
sys.exit(1)' "$@"
}

ensure_demo_scope() {
  if ! exists_in_list /api/v1/namespaces name="$DEMO_NAMESPACE"; then
    api POST /api/v1/namespaces "$(json_body name="$DEMO_NAMESPACE")" >/dev/null
  fi
  if ! exists_in_list "/api/v1/apps?namespace=$DEMO_NAMESPACE" namespace="$DEMO_NAMESPACE" name="$DEMO_APP"; then
    api POST /api/v1/apps "$(json_body namespace="$DEMO_NAMESPACE" name="$DEMO_APP")" >/dev/null
  fi
  if ! exists_in_list "/api/v1/worker-pools?namespace=$DEMO_NAMESPACE&app=$DEMO_APP" namespace="$DEMO_NAMESPACE" app="$DEMO_APP" name="$DEMO_WORKER_POOL"; then
    local created pool_id
    created="$(api POST /api/v1/worker-pools "$(json_body namespace="$DEMO_NAMESPACE" app="$DEMO_APP" name="$DEMO_WORKER_POOL")")"
    pool_id="$(printf '%s' "$created" | tikeo_smoke_json_get data.id)"
    api PATCH "/api/v1/worker-pools/$pool_id/quota" '{"max_queue_depth":200,"max_concurrency":8}' >/dev/null
  fi
}

start_server_if_needed() {
  if curl -fsS "$API_URL/readyz" >/dev/null 2>&1; then
    return
  fi
  OWN_SERVER=1
  local config="$REPORT_DIR/${RUN_ID}-config.toml"
  cat > "$config" <<CFG
[server]
listen_addr = "127.0.0.1:19090"
worker_tunnel_addr = "127.0.0.1:19998"

[storage]

[storage.database]
type = "sqlite"
path = "$REPORT_DIR/${RUN_ID}.db"

[storage.database.params]
mode = "rwc"

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
  (cd "$ROOT_DIR" && cargo run --bin tikeo -- serve --config "$config" >"$SERVER_LOG" 2>&1) &
  SERVER_PID=$!
  tikeo_smoke_wait_for_http server "$API_URL/readyz" 120 || {
    tail -n 160 "$SERVER_LOG" >&2 || true
    return 1
  }
}

start_web_if_needed() {
  if curl -fsS "$WEB_URL" >/dev/null 2>&1; then
    return
  fi
  OWN_WEB=1
  (cd "$ROOT_DIR/web" && bun run dev -- --host 127.0.0.1 --port "$WEB_PORT" >"$WEB_LOG" 2>&1) &
  WEB_PID=$!
  tikeo_smoke_wait_for_http web "$WEB_URL" 120 || {
    tail -n 160 "$WEB_LOG" >&2 || true
    return 1
  }
}

start_java_demo() {
  local name="$1"
  local port="$2"
  local priority="$3"
  local log_file="$4"
  (
    cd "$ROOT_DIR/examples/java/spring-boot3-worker-demo"
    TIKEO_WORKER_DRY_RUN=false \
    TIKEO_WORKER_ENDPOINT="$WORKER_ENDPOINT" \
    TIKEO_WORKER_NAMESPACE="$DEMO_NAMESPACE" \
    TIKEO_WORKER_APP="$DEMO_APP" \
    TIKEO_WORKER_POOL="$DEMO_WORKER_POOL" \
    TIKEO_DEMO_SERVER_PORT="$port" \
    TIKEO_WORKER_CLIENT_INSTANCE_ID="$name" \
    TIKEO_WORKER_STATE_DIR="$REPORT_DIR/$name-state" \
    TIKEO_WORKER_ELECTION_DOMAIN="joint-default-domain" \
    TIKEO_WORKER_ELECTION_PRIORITY="$priority" \
    TIKEO_WORKER_SCRIPTS_ENABLED=false \
    exec ./scripts/run-demo-worker.sh >"$log_file" 2>&1
  ) &
  local pid=$!
  tikeo_smoke_wait_for_http "java-$name" "http://127.0.0.1:$port/demo/health" 120 || {
    tail -n 160 "$log_file" >&2 || true
    return 1
  }
  STARTED_JAVA_PID="$pid"
}

wait_workers_asserted() {
  local output="$1"
  local min_online="$2"
  local deadline=$((SECONDS + 120))
  until api GET /api/v1/workers > "$output" \
    && tikeo_smoke_assert workers "$output" --min-online "$min_online" --require-sdk-processor demo.echo --require-plugin-processor sql:billing.sql-sync >/dev/null; do
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for workers assertion min_online=$min_online" >&2
      cat "$output" >&2 || true
      tail -n 120 "$JAVA_A_LOG" >&2 || true
      tail -n 120 "$JAVA_B_LOG" >&2 || true
      return 1
    fi
    sleep 1
  done
}

worker_field() {
  local workers_file="$1"
  local selector="$2"
  python3 - "$workers_file" "$selector" <<'PY'
import json, sys
path, selector = sys.argv[1:3]
items = json.load(open(path, encoding='utf-8')).get('data', {}).get('items', [])
online = [w for w in items if w.get('status') == 'online']
masters = [w for w in online if (w.get('master') or {}).get('isMaster') is True]
if not masters:
    raise SystemExit('no online master')
master = masters[0]
if selector == 'master_worker_id':
    print(master.get('workerId') or master.get('worker_id'))
elif selector == 'master_client_instance_id':
    print(master.get('clientInstanceId') or master.get('client_instance_id'))
else:
    raise SystemExit(f'unknown selector {selector}')
PY
}

create_job() {
  local name="$1"
  local processor="$2"
  local body
  body="$(python3 - "$RUN_ID" "$DEMO_NAMESPACE" "$DEMO_APP" "$name" "$processor" <<'PY'
import json, sys
run_id, namespace, app, name, processor = sys.argv[1:]
print(json.dumps({
  'namespace': namespace,
  'app': app,
  'name': f'{run_id}-{name}',
  'scheduleType': 'api',
  'processorName': processor,
  'enabled': True,
}))
PY
)"
  api_json_get POST /api/v1/jobs data.id "$body"
}

trigger_job() {
  local job_id="$1"
  local mode="$2"
  api_json_get POST "/api/v1/jobs/$job_id:trigger" data.id "{\"triggerType\":\"api\",\"executionMode\":\"$mode\"}"
}

main() {
  start_server_if_needed
  tikeo_smoke_login "$API_URL"
  AUTH_TOKEN="$TIKEO_SMOKE_AUTH_TOKEN"
  export AUTH_TOKEN
  ensure_demo_scope
  start_web_if_needed

  start_java_demo spring-demo-worker-a "$JAVA_A_PORT" 10 "$JAVA_A_LOG"
  JAVA_A_PID="$STARTED_JAVA_PID"
  start_java_demo spring-demo-worker-b "$JAVA_B_PORT" 20 "$JAVA_B_LOG"
  JAVA_B_PID="$STARTED_JAVA_PID"

  local workers_before master_worker master_client echo_job echo_instance echo_file echo_logs
  workers_before="$REPORT_DIR/${RUN_ID}-workers-before.json"
  wait_workers_asserted "$workers_before" 2
  tikeo_smoke_assert workers "$workers_before" --client-instance spring-demo-worker-a --client-instance spring-demo-worker-b --min-online 2 --require-sdk-processor demo.echo --require-plugin-processor sql:billing.sql-sync >/dev/null
  master_worker="$(worker_field "$workers_before" master_worker_id)"
  master_client="$(worker_field "$workers_before" master_client_instance_id)"
  tikeo_smoke_record_case joint-worker-election passed "$workers_before" "two Java workers online with exactly one master: $master_client/$master_worker"

  echo_job="$(create_job echo demo.echo)"
  echo_instance="$(trigger_job "$echo_job" single)"
  echo_file="$REPORT_DIR/${RUN_ID}-${echo_instance}.json"
  echo_logs="$REPORT_DIR/${RUN_ID}-${echo_instance}-logs.json"
  tikeo_smoke_wait_instance_status "$API_URL" "$echo_instance" succeeded "$echo_file" 120
  api GET "/api/v1/instances/$echo_instance/logs" > "$echo_logs"
  tikeo_smoke_assert instance "$echo_file" --expected-status succeeded --expected-worker "$master_worker" --min-log-count 1 --logs-file "$echo_logs" --require-log-text demo.echo --forbid-duplicate-logs >/dev/null
  tikeo_smoke_record_case joint-single-master-dispatch passed "$echo_file $echo_logs" "single dispatch executed by current master $master_worker"

  local broadcast_job broadcast_instance broadcast_file broadcast_attempts
  broadcast_job="$(create_job broadcast demo.context)"
  broadcast_instance="$(trigger_job "$broadcast_job" broadcast)"
  broadcast_file="$REPORT_DIR/${RUN_ID}-${broadcast_instance}.json"
  broadcast_attempts="$REPORT_DIR/${RUN_ID}-${broadcast_instance}-attempts.json"
  tikeo_smoke_wait_instance_status "$API_URL" "$broadcast_instance" succeeded "$broadcast_file" 120
  api GET "/api/v1/instances/$broadcast_instance/attempts" > "$broadcast_attempts"
  tikeo_smoke_assert attempts "$broadcast_attempts" --min-attempts 2 --expected-status succeeded >/dev/null
  tikeo_smoke_record_case joint-broadcast-all-workers passed "$broadcast_attempts" "broadcast created successful attempts for both workers"

  if [[ "$master_client" == "spring-demo-worker-a" ]]; then
    stop_java_demo "$JAVA_A_PID" "$JAVA_A_PORT"
    JAVA_A_PID=""
  else
    stop_java_demo "$JAVA_B_PID" "$JAVA_B_PORT"
    JAVA_B_PID=""
  fi

  local workers_after new_master_worker new_master_client failover_job failover_instance failover_file failover_logs
  workers_after="$REPORT_DIR/${RUN_ID}-workers-after-failover.json"
  wait_workers_asserted "$workers_after" 1
  new_master_worker="$(worker_field "$workers_after" master_worker_id)"
  new_master_client="$(worker_field "$workers_after" master_client_instance_id)"
  if [[ "$new_master_worker" == "$master_worker" ]]; then
    echo "expected a different master after killing $master_client, still got $new_master_worker" >&2
    cat "$workers_after" >&2 || true
    return 1
  fi
  tikeo_smoke_record_case joint-worker-failover passed "$workers_after" "follower promoted to master: $new_master_client/$new_master_worker"

  failover_job="$(create_job failover-echo demo.echo)"
  failover_instance="$(trigger_job "$failover_job" single)"
  failover_file="$REPORT_DIR/${RUN_ID}-${failover_instance}.json"
  failover_logs="$REPORT_DIR/${RUN_ID}-${failover_instance}-logs.json"
  tikeo_smoke_wait_instance_status "$API_URL" "$failover_instance" succeeded "$failover_file" 120
  api GET "/api/v1/instances/$failover_instance/logs" > "$failover_logs"
  tikeo_smoke_assert instance "$failover_file" --expected-status succeeded --expected-worker "$new_master_worker" --min-log-count 1 --logs-file "$failover_logs" --require-log-text demo.echo --forbid-duplicate-logs >/dev/null
  tikeo_smoke_record_case joint-failover-dispatch passed "$failover_file $failover_logs" "single dispatch after failover executed by new master $new_master_worker"

  local workers_html api_keys_html
  workers_html="$REPORT_DIR/${RUN_ID}-workers.html"
  api_keys_html="$REPORT_DIR/${RUN_ID}-api-keys.html"
  curl -fsS "$WEB_URL/workers" -o "$workers_html"
  curl -fsS "$WEB_URL/api-keys" -o "$api_keys_html"
  tikeo_smoke_assert web "$workers_html" --require-text '<div id="root"></div>' --forbid-text '404 Not Found' >/dev/null
  tikeo_smoke_assert web "$api_keys_html" --require-text '<div id="root"></div>' --forbid-text '404 Not Found' >/dev/null
  tikeo_smoke_record_case joint-web-routes passed "$workers_html $api_keys_html" "web secondary routes returned SPA shell"

  local report="$REPORT_DIR/${RUN_ID}.json"
  tikeo_smoke_finalize_report "$report" passed >/dev/null
  python3 "$ROOT_DIR/deploy/smoke/collect-joint-report.py" "$REPORT_DIR" --run-id "$RUN_ID" --json-output "$REPORT_DIR/${RUN_ID}-joint-report.json" --markdown-output "$REPORT_DIR/${RUN_ID}-joint-report.md" >/dev/null
  echo "report: $report"
  echo "joint report: $REPORT_DIR/${RUN_ID}-joint-report.md"
}

main "$@"
