#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUN_ID="${TIKEO_CROSS_RUN_ID:-cross-language-workers-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
REPORT_DIR="${TIKEO_CROSS_REPORT_DIR:-$ROOT_DIR/.dev/reports/$RUN_ID}"
API_URL="${TIKEO_HTTP_URL:-http://127.0.0.1:19092}"
WORKER_ENDPOINT="${TIKEO_WORKER_ENDPOINT:-http://127.0.0.1:19992}"
WEB_URL="${TIKEO_WEB_URL:-http://127.0.0.1:15174}"
WEB_PORT="${WEB_URL##*:}"
WEB_PORT="${WEB_PORT%%/*}"
SERVER_CONFIG="$REPORT_DIR/$RUN_ID-config.toml"
SERVER_LOG="$REPORT_DIR/$RUN_ID-server.log"
SERVER_BIN="$ROOT_DIR/target/debug/tikeo"
WEB_LOG="$REPORT_DIR/$RUN_ID-web.log"
DB_PATH="$REPORT_DIR/$RUN_ID.db"
SUMMARY_JSON="$REPORT_DIR/$RUN_ID-summary.json"
REPORT_JSON="$REPORT_DIR/$RUN_ID.json"
mkdir -p "$REPORT_DIR"
export TIKEO_SMOKE_REPORT_DIR="$REPORT_DIR"
export TIKEO_SMOKE_RUN_ID="$RUN_ID"
export TIKEO_SMOKE_CASES_FILE="$REPORT_DIR/$RUN_ID-cases.jsonl"
# shellcheck source=deploy/smoke/lib/tikeo-smoke-lib.sh
source "$ROOT_DIR/deploy/smoke/lib/tikeo-smoke-lib.sh"
: > "$TIKEO_SMOKE_CASES_FILE"

SERVER_PID=""
WEB_PID=""
WORKER_PIDS=()
WORKER_NAMES=()
WORKER_LOGS=()
AUTH_TOKEN=""
POOL_READER_TOKEN=""

need_cmd() {
  tikeo_smoke_need_cmd "$1"
}

cleanup() {
  local code=$?
  stop_workers || true
  stop_web || true
  stop_server || true
  exit "$code"
}
trap cleanup EXIT INT TERM

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

write_config() {
  cat > "$SERVER_CONFIG" <<CFG
[server]
listen_addr = "127.0.0.1:19092"
worker_tunnel_addr = "127.0.0.1:19992"

[storage]
database_url = "sqlite://$DB_PATH?mode=rwc"

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
}

build_server_binary() {
  if [[ ! -x "$SERVER_BIN" || "${TIKEO_CROSS_REBUILD_SERVER:-1}" == "1" ]]; then
    (cd "$ROOT_DIR" && cargo build --bin tikeo >>"$SERVER_LOG" 2>&1)
  fi
}

start_server() {
  write_config
  build_server_binary
  (cd "$ROOT_DIR" && exec "$SERVER_BIN" serve --config "$SERVER_CONFIG" >>"$SERVER_LOG" 2>&1) &
  SERVER_PID=$!
  tikeo_smoke_wait_for_http server "$API_URL/readyz" 180 || {
    tail -n 180 "$SERVER_LOG" >&2 || true
    return 1
  }
}

stop_server() {
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  SERVER_PID=""
  local deadline=$((SECONDS + 30))
  while curl -fsS "$API_URL/readyz" >/dev/null 2>&1; do
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for server to stop at $API_URL" >&2
      return 1
    fi
    sleep 1
  done
}

start_web() {
  if [[ "${TIKEO_CROSS_SKIP_WEB:-0}" == "1" ]]; then
    return 0
  fi
  (cd "$ROOT_DIR/web" && bun run dev -- --host 127.0.0.1 --port "$WEB_PORT" >"$WEB_LOG" 2>&1) &
  WEB_PID=$!
  tikeo_smoke_wait_for_http web "$WEB_URL" 120 || {
    tail -n 120 "$WEB_LOG" >&2 || true
    return 1
  }
}

stop_web() {
  if [[ -n "$WEB_PID" ]] && kill -0 "$WEB_PID" >/dev/null 2>&1; then
    kill "$WEB_PID" >/dev/null 2>&1 || true
    wait "$WEB_PID" 2>/dev/null || true
  fi
  WEB_PID=""
}

login() {
  local username="${TIKEO_SMOKE_ADMIN_USERNAME:-smoke_admin}"
  local password="${TIKEO_SMOKE_ADMIN_PASSWORD:-Tikeo@2026!}"
  tikeo_smoke_login "$API_URL" "$username" "$password"
  AUTH_TOKEN="$TIKEO_SMOKE_AUTH_TOKEN"
  export AUTH_TOKEN
}

create_namespace() {
  local namespace="$1"
  exists_in_list /api/v1/namespaces name="$namespace" || api POST /api/v1/namespaces "$(json_body name="$namespace")" >/dev/null
}

create_app() {
  local namespace="$1" app="$2"
  exists_in_list "/api/v1/apps?namespace=$namespace" namespace="$namespace" name="$app" || \
    api POST /api/v1/apps "$(json_body namespace="$namespace" name="$app")" >/dev/null
}

create_pool() {
  local namespace="$1" app="$2" pool="$3" depth="$4" concurrency="$5"
  local pool_id
  if exists_in_list "/api/v1/worker-pools?namespace=$namespace&app=$app" namespace="$namespace" app="$app" name="$pool"; then
    pool_id="$(api GET "/api/v1/worker-pools?namespace=$namespace&app=$app" | python3 -c 'import json,sys
pool=sys.argv[1]
payload=json.load(sys.stdin)
for item in payload.get("data", []):
    if item.get("name") == pool:
        print(item["id"])
        break' "$pool")"
  else
    pool_id="$(api_json_get POST /api/v1/worker-pools data.id "$(json_body namespace="$namespace" app="$app" name="$pool")")"
  fi
  api PATCH "/api/v1/worker-pools/$pool_id/quota" "{\"max_queue_depth\":$depth,\"max_concurrency\":$concurrency}" >/dev/null
}

create_plugin_processor() {
  if api GET /api/v1/plugins | python3 -c 'import json, sys
payload=json.load(sys.stdin)
for plugin in payload.get("data", []):
    for processor in plugin.get("processorTypes", []) or plugin.get("processor_types", []):
        names=processor.get("processorNames") or processor.get("processor_names") or []
        if processor.get("type") == "sql" and "billing.sql-sync" in names:
            sys.exit(0)
sys.exit(1)'; then
    return 0
  fi
  api POST /api/v1/plugins "$(python3 - <<'PY'
import json
print(json.dumps({
  "name": "Cross Language SQL Processor Plugin",
  "kind": "processor",
  "processorTypes": [{
    "type": "sql",
    "label": "SQL Processor",
    "capability": "sql",
    "processorNames": ["billing.sql-sync"],
    "description": "SQL plugin processor used by cross-language worker smoke"
  }],
  "alertChannelTypes": [],
  "enabled": True
}, ensure_ascii=False, separators=(",", ":")))
PY
)" >/dev/null
}

seed_scopes() {
  create_namespace dev-alpha
  create_namespace dev-beta
  create_namespace dev-ops
  create_app dev-alpha orders
  create_app dev-alpha billing
  create_app dev-beta analytics
  create_app dev-ops automation
  create_pool dev-alpha orders boot2-blue 200 8
  create_pool dev-alpha orders boot3-blue 200 8
  create_pool dev-alpha orders go-blue 200 8
  create_pool dev-alpha orders rust-blue 200 8
  create_pool dev-alpha orders python-blue 200 8
  create_pool dev-alpha orders nodejs-blue 200 8
  create_pool dev-alpha billing boot4-green 100 4
  create_pool dev-beta analytics boot3-batch 150 6
  create_pool dev-ops automation boot4-ops 80 3
  create_plugin_processor
}

job_body() {
  python3 - "$@" <<'PY'
import json, sys
values = dict(arg.split('=', 1) for arg in sys.argv[1:])
body = {
    'namespace': values['namespace'],
    'app': values['app'],
    'name': values['name'],
    'scheduleType': 'api',
    'processorName': values['processor'],
    'enabled': True,
}
if values.get('processorType'):
    body['processorType'] = values['processorType']
print(json.dumps(body, ensure_ascii=False, separators=(',', ':')))
PY
}

create_job() {
  local namespace="$1" app="$2" name="$3" processor="$4" processor_type="${5:-}"
  api_json_get POST /api/v1/jobs data.id "$(job_body namespace="$namespace" app="$app" name="$RUN_ID-$name" processor="$processor" processorType="$processor_type")"
}

trigger_broadcast() {
  local job_id="$1" selector_json="$2"
  api_json_get POST "/api/v1/jobs/$job_id:trigger" data.id "$(python3 - "$selector_json" <<'PY'
import json, sys
selector = json.loads(sys.argv[1])
print(json.dumps({'triggerType': 'api', 'executionMode': 'broadcast', 'broadcastSelector': selector}, separators=(',', ':')))
PY
)"
}

wait_instance_status() {
  local instance_id="$1" expected="$2" output="$3" timeout="${4:-120}"
  tikeo_smoke_wait_instance_status "$API_URL" "$instance_id" "$expected" "$output" "$timeout"
}

start_java_worker() {
  local family="$1" dir="$2" client="$3" namespace="$4" app="$5" pool="$6" port="$7" priority="$8"
  local log="$REPORT_DIR/$RUN_ID-$client.log"
  (
    cd "$ROOT_DIR/examples/java/$dir"
    TIKEO_WORKER_DRY_RUN=false \
    TIKEO_WORKER_ENDPOINT="$WORKER_ENDPOINT" \
    TIKEO_WORKER_NAMESPACE="$namespace" \
    TIKEO_WORKER_APP="$app" \
    TIKEO_WORKER_POOL="$pool" \
    TIKEO_WORKER_CLUSTER="local" \
    TIKEO_WORKER_REGION="local" \
    TIKEO_DEMO_SERVER_PORT="$port" \
    TIKEO_WORKER_CLIENT_INSTANCE_ID="$client" \
    TIKEO_WORKER_STATE_DIR="$REPORT_DIR/$client-state" \
    TIKEO_WORKER_ELECTION_DOMAIN="$namespace/$app/$pool/local" \
    TIKEO_WORKER_ELECTION_PRIORITY="$priority" \
    TIKEO_WORKER_SCRIPT_RUNTIME_CHECK=false \
    TIKEO_WORKER_SCRIPT_AUTO_INSTALL_TOOLS=false \
    TIKEO_WORKER_WASM_AUTO_INSTALL=false \
    exec ./scripts/run-demo-worker.sh >"$log" 2>&1
  ) &
  WORKER_PIDS+=("$!")
  WORKER_NAMES+=("$client")
  WORKER_LOGS+=("$log")
  tikeo_smoke_wait_for_http "$family-$client" "http://127.0.0.1:$port/demo/health" 180 || {
    tail -n 180 "$log" >&2 || true
    return 1
  }
}

start_go_worker() {
  local log="$REPORT_DIR/$RUN_ID-go-worker-demo-local.log"
  (
    cd "$ROOT_DIR/examples/go/worker-demo"
    TIKEO_WORKER_DRY_RUN=false \
    TIKEO_WORKER_CONNECT=1 \
    TIKEO_WORKER_ENDPOINT="$WORKER_ENDPOINT" \
    TIKEO_WORKER_NAMESPACE=dev-alpha \
    TIKEO_WORKER_APP=orders \
    TIKEO_WORKER_POOL=go-blue \
    TIKEO_WORKER_CLUSTER=local \
    TIKEO_WORKER_REGION=local \
    TIKEO_WORKER_CLIENT_INSTANCE_ID=go-worker-demo-local \
    TIKEO_WORKER_SDK_PROCESSORS=demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail \
    TIKEO_SANDBOX_AUTO_INSTALL=0 \
    exec go run . >"$log" 2>&1
  ) &
  WORKER_PIDS+=("$!")
  WORKER_NAMES+=("go-worker-demo-local")
  WORKER_LOGS+=("$log")
}

start_rust_worker() {
  local log="$REPORT_DIR/$RUN_ID-rust-worker-demo-local.log"
  (
    cd "$ROOT_DIR/examples/rust/worker-demo"
    TIKEO_WORKER_DRY_RUN=false \
    TIKEO_WORKER_CONNECT=1 \
    TIKEO_WORKER_ENDPOINT="$WORKER_ENDPOINT" \
    TIKEO_WORKER_NAMESPACE=dev-alpha \
    TIKEO_WORKER_APP=orders \
    TIKEO_WORKER_POOL=rust-blue \
    TIKEO_WORKER_CLUSTER=local \
    TIKEO_WORKER_REGION=local \
    TIKEO_WORKER_CLIENT_INSTANCE_ID=rust-worker-demo-local \
    TIKEO_WORKER_SDK_PROCESSORS=demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail \
    TIKEO_SANDBOX_AUTO_INSTALL=0 \
    exec cargo run >"$log" 2>&1
  ) &
  WORKER_PIDS+=("$!")
  WORKER_NAMES+=("rust-worker-demo-local")
  WORKER_LOGS+=("$log")
}


start_python_worker() {
  local log="$REPORT_DIR/$RUN_ID-python-worker-demo-local.log"
  (
    cd "$ROOT_DIR/examples/python/worker-demo"
    TIKEO_WORKER_DRY_RUN=false \
    TIKEO_WORKER_CONNECT=1 \
    TIKEO_WORKER_ENDPOINT="$WORKER_ENDPOINT" \
    TIKEO_WORKER_NAMESPACE=dev-alpha \
    TIKEO_WORKER_APP=orders \
    TIKEO_WORKER_POOL=python-blue \
    TIKEO_WORKER_CLUSTER=local \
    TIKEO_WORKER_REGION=local \
    TIKEO_WORKER_CLIENT_INSTANCE_ID=python-worker-demo-local \
    TIKEO_WORKER_SDK_PROCESSORS=demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail \
    TIKEO_SANDBOX_AUTO_INSTALL=0 \
    exec python3 -m tikeo_python_worker_demo >"$log" 2>&1
  ) &
  WORKER_PIDS+=("$!")
  WORKER_NAMES+=("python-worker-demo-local")
  WORKER_LOGS+=("$log")
}

start_nodejs_worker() {
  local log="$REPORT_DIR/$RUN_ID-nodejs-worker-demo-local.log"
  (
    cd "$ROOT_DIR/examples/nodejs/worker-demo"
    TIKEO_WORKER_DRY_RUN=false \
    TIKEO_WORKER_CONNECT=1 \
    TIKEO_WORKER_ENDPOINT="$WORKER_ENDPOINT" \
    TIKEO_WORKER_NAMESPACE=dev-alpha \
    TIKEO_WORKER_APP=orders \
    TIKEO_WORKER_POOL=nodejs-blue \
    TIKEO_WORKER_CLUSTER=local \
    TIKEO_WORKER_REGION=local \
    TIKEO_WORKER_CLIENT_INSTANCE_ID=nodejs-worker-demo-local \
    TIKEO_WORKER_SDK_PROCESSORS=demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail \
    TIKEO_SANDBOX_AUTO_INSTALL=0 \
    exec bun start >"$log" 2>&1
  ) &
  WORKER_PIDS+=("$!")
  WORKER_NAMES+=("nodejs-worker-demo-local")
  WORKER_LOGS+=("$log")
}

start_all_workers() {
  start_java_worker boot2 spring-boot2-worker-demo java-boot2-orders-blue dev-alpha orders boot2-blue 18282 30
  start_java_worker boot3 spring-boot3-worker-demo java-boot3-orders-blue dev-alpha orders boot3-blue 18283 40
  start_java_worker boot4 spring-boot4-worker-demo java-boot4-billing-green dev-alpha billing boot4-green 18284 50
  start_go_worker
  start_rust_worker
  start_python_worker
  start_nodejs_worker
}

stop_workers() {
  local pid
  for pid in "${WORKER_PIDS[@]:-}"; do
    if [[ -n "$pid" ]] && kill -0 "$pid" >/dev/null 2>&1; then
      kill "$pid" >/dev/null 2>&1 || true
    fi
  done
  for pid in "${WORKER_PIDS[@]:-}"; do
    wait "$pid" 2>/dev/null || true
  done
  WORKER_PIDS=()
  WORKER_NAMES=()
  WORKER_LOGS=()
}

wait_workers() {
  local output="$1" mode="${2:-live}" timeout="${3:-180}"
  local deadline=$((SECONDS + timeout))
  until api GET /api/v1/workers > "$output" && python3 - "$output" "$mode" <<'PY'
import json, sys
path, mode = sys.argv[1:3]
payload = json.load(open(path, encoding='utf-8'))
items = payload.get('data', {}).get('items', [])
expected = {
    'java-boot2-orders-blue': {'namespace': 'dev-alpha', 'app': 'orders', 'pool': 'boot2-blue', 'processor': 'demo.echo'},
    'java-boot3-orders-blue': {'namespace': 'dev-alpha', 'app': 'orders', 'pool': 'boot3-blue', 'processor': 'demo.echo'},
    'java-boot4-billing-green': {'namespace': 'dev-alpha', 'app': 'billing', 'pool': 'boot4-green', 'processor': 'demo.echo'},
    'go-worker-demo-local': {'namespace': 'dev-alpha', 'app': 'orders', 'pool': 'go-blue', 'processor': 'demo.echo', 'tag': 'go'},
    'rust-worker-demo-local': {'namespace': 'dev-alpha', 'app': 'orders', 'pool': 'rust-blue', 'processor': 'demo.echo', 'tag': 'rust'},
}
by_client = {item.get('clientInstanceId'): item for item in items if item.get('status') == 'online'}
missing = [client for client in expected if client not in by_client]
if missing:
    raise SystemExit(f'missing online workers: {missing}; got={list(by_client)}')
for client, rule in expected.items():
    item = by_client[client]
    if item.get('namespace') != rule['namespace'] or item.get('app') != rule['app']:
        raise SystemExit(f'{client} scope mismatch: {item.get("namespace")}/{item.get("app")}')
    structured = item.get('structuredCapabilities') or {}
    processors = structured.get('sdkProcessors') or []
    if rule['processor'] not in processors:
        raise SystemExit(f'{client} missing sdk processor {rule["processor"]}: {processors}')
    if rule.get('tag') and rule['tag'] not in (structured.get('tags') or []):
        raise SystemExit(f'{client} missing tag {rule["tag"]}')
    runners = structured.get('scriptRunners') or []
    allowed_backends = {'srt', 'deno', 'v8', 'wasmtime', 'wasmedge', 'docker', 'podman'}
    unexpected = sorted({runner.get('sandboxBackend') for runner in runners} - allowed_backends)
    if unexpected:
        raise SystemExit(f'{client} advertises unexpected script sandbox backends: {unexpected}')
masters = [item for item in items if item.get('status') == 'online' and (item.get('master') or {}).get('isMaster')]
if not masters:
    raise SystemExit('no online master state visible')
print(f'{mode} workers assertion passed: online={len(items)}')
PY
  do
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for expected workers ($mode)" >&2
      cat "$output" >&2 || true
      for log in "${WORKER_LOGS[@]:-}"; do
        echo "--- tail $log ---" >&2
        tail -n 120 "$log" >&2 || true
      done
      return 1
    fi
    sleep 2
  done
}

create_pool_reader_token() {
  POOL_READER_TOKEN="$(api_json_get POST /api/v1/auth/api-tokens data.access_token '{"name":"cross-language boot2 worker reader","scopes":["workers:read"],"scope_bindings":[{"namespace":"dev-alpha","app":"orders","worker_pool":"boot2-blue"}]}')"
}

capture_pool_filtered_workers() {
  local output="$1"
  local old_token="$TIKEO_SMOKE_AUTH_TOKEN"
  TIKEO_SMOKE_AUTH_TOKEN="$POOL_READER_TOKEN"
  export TIKEO_SMOKE_AUTH_TOKEN
  api GET /api/v1/workers > "$output"
  TIKEO_SMOKE_AUTH_TOKEN="$old_token"
  export TIKEO_SMOKE_AUTH_TOKEN
  python3 - "$output" <<'PY'
import json, sys
payload = json.load(open(sys.argv[1], encoding='utf-8'))
items = payload.get('data', {}).get('items', [])
clients = [item.get('clientInstanceId') for item in items]
if payload.get('data', {}).get('online') != 1 or clients != ['java-boot2-orders-blue']:
    raise SystemExit(f'worker_pool scoped token expected only boot2 worker, got online={payload.get("data", {}).get("online")} clients={clients}')
print('worker_pool scoped token assertion passed')
PY
}

assert_instance_logs() {
  local instance_id="$1" expected_status="$2" log_text="$3" evidence_prefix="$4"
  local instance_file="$REPORT_DIR/$RUN_ID-$evidence_prefix-instance.json"
  local logs_file="$REPORT_DIR/$RUN_ID-$evidence_prefix-logs.json"
  wait_instance_status "$instance_id" "$expected_status" "$instance_file" 180
  api GET "/api/v1/instances/$instance_id/logs" > "$logs_file"
  tikeo_smoke_assert instance "$instance_file" --expected-status "$expected_status" --min-log-count 1 --logs-file "$logs_file" --require-log-text "$log_text" --forbid-duplicate-logs >/dev/null
  python3 - "$logs_file" "$log_text" <<'PY'
import json, sys
path, text = sys.argv[1:3]
payload = json.load(open(path, encoding='utf-8')).get('data')
items = payload.get('items') if isinstance(payload, dict) else payload
messages = '\n'.join(str(item.get('message', '')) for item in items)
if text not in messages:
    raise SystemExit(f'missing expected log text {text!r}: {messages}')
print(f'log assertion passed for {text}')
PY
}

run_language_jobs() {
  local boot2_job boot3_job boot4_job go_job rust_job python_job nodejs_job
  boot2_job="$(create_job dev-alpha orders boot2-echo demo.echo)"
  boot3_job="$(create_job dev-alpha orders boot3-echo demo.echo)"
  boot4_job="$(create_job dev-alpha billing boot4-echo demo.echo)"
  go_job="$(create_job dev-alpha orders go-echo demo.echo)"
  rust_job="$(create_job dev-alpha orders rust-echo demo.echo)"
  python_job="$(create_job dev-alpha orders python-echo demo.echo)"
  nodejs_job="$(create_job dev-alpha orders nodejs-echo demo.echo)"

  local boot2_instance boot3_instance boot4_instance go_instance rust_instance python_instance nodejs_instance
  boot2_instance="$(trigger_broadcast "$boot2_job" '{"labels":{"worker_pool":"boot2-blue"}}')"
  boot3_instance="$(trigger_broadcast "$boot3_job" '{"labels":{"worker_pool":"boot3-blue"}}')"
  boot4_instance="$(trigger_broadcast "$boot4_job" '{"labels":{"worker_pool":"boot4-green"}}')"
  go_instance="$(trigger_broadcast "$go_job" '{"tags":["go"],"labels":{"worker_pool":"go-blue"}}')"
  rust_instance="$(trigger_broadcast "$rust_job" '{"tags":["rust"],"labels":{"worker_pool":"rust-blue"}}')"
  python_instance="$(trigger_broadcast "$python_job" '{"tags":["python"],"labels":{"worker_pool":"python-blue"}}')"
  nodejs_instance="$(trigger_broadcast "$nodejs_job" '{"tags":["nodejs"],"labels":{"worker_pool":"nodejs-blue"}}')"

  assert_instance_logs "$boot2_instance" succeeded demo.echo boot2
  assert_instance_logs "$boot3_instance" succeeded demo.echo boot3
  assert_instance_logs "$boot4_instance" succeeded demo.echo boot4
  assert_instance_logs "$go_instance" succeeded "go demo echo processed" go
  assert_instance_logs "$rust_instance" succeeded "rust demo echo processed" rust
  assert_instance_logs "$python_instance" succeeded "python demo echo processed" python
  assert_instance_logs "$nodejs_instance" succeeded "nodejs demo echo processed" nodejs
  tikeo_smoke_record_case cross-language-dispatch passed "$REPORT_DIR/$RUN_ID-go-logs.json $REPORT_DIR/$RUN_ID-rust-logs.json $REPORT_DIR/$RUN_ID-python-logs.json $REPORT_DIR/$RUN_ID-nodejs-logs.json" "Java Boot2/Boot3/Boot4, Go, Rust, Python and Node.js jobs reached expected terminal states"
}

verify_restart_snapshot() {
  local before="$REPORT_DIR/$RUN_ID-workers-before-restart.json"
  local filtered_live="$REPORT_DIR/$RUN_ID-worker-pool-filter-live.json"
  local after_restart="$REPORT_DIR/$RUN_ID-workers-after-restart-snapshot.json"
  local filtered_persisted="$REPORT_DIR/$RUN_ID-worker-pool-filter-persisted.json"
  local after_reconnect="$REPORT_DIR/$RUN_ID-workers-after-reconnect.json"
  wait_workers "$before" live 30
  capture_pool_filtered_workers "$filtered_live"
  stop_server
  stop_workers
  start_server
  login
  wait_workers "$after_restart" persisted 45
  capture_pool_filtered_workers "$filtered_persisted"
  tikeo_smoke_record_case worker-restart-persisted-snapshot passed "$after_restart $filtered_persisted" "workers remained visible from persisted session snapshots after server restart with workers stopped"
  start_all_workers
  wait_workers "$after_reconnect" reconnected 240
  tikeo_smoke_record_case worker-reconnect-supersedes-snapshot passed "$after_reconnect" "live workers reconnected and remained structured after persisted snapshot visibility"
}

verify_web_routes() {
  if [[ "${TIKEO_CROSS_SKIP_WEB:-0}" == "1" ]]; then
    return 0
  fi
  local workers_html="$REPORT_DIR/$RUN_ID-web-workers.html"
  local queue_html="$REPORT_DIR/$RUN_ID-web-dispatch-queue.html"
  curl -fsS "$WEB_URL/workers" -o "$workers_html"
  curl -fsS "$WEB_URL/workers/dispatch-queue" -o "$queue_html"
  tikeo_smoke_assert web "$workers_html" --require-text '<div id="root"></div>' --forbid-text '404 Not Found' >/dev/null
  tikeo_smoke_assert web "$queue_html" --require-text '<div id="root"></div>' --forbid-text '404 Not Found' >/dev/null
  tikeo_smoke_record_case web-worker-secondary-routes passed "$workers_html $queue_html" "Workers page and dispatch queue secondary route return SPA shell"
}

write_summary() {
  python3 - "$REPORT_DIR" "$RUN_ID" "$SUMMARY_JSON" <<'PY'
import json, pathlib, sys, datetime
report_dir = pathlib.Path(sys.argv[1])
run_id = sys.argv[2]
summary = {
    'runId': run_id,
    'generatedAt': datetime.datetime.now(datetime.UTC).isoformat(),
    'reportDir': str(report_dir),
    'evidence': sorted(p.name for p in report_dir.iterdir() if p.is_file()),
}
pathlib.Path(sys.argv[3]).write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding='utf-8')
print(json.dumps(summary, ensure_ascii=False, indent=2))
PY
}

main() {
  need_cmd cargo
  need_cmd curl
  need_cmd python3
  need_cmd go
  need_cmd bun
  start_server
  login
  seed_scopes
  create_pool_reader_token
  start_web
  start_all_workers
  wait_workers "$REPORT_DIR/$RUN_ID-workers-initial.json" initial 240
  run_language_jobs
  verify_restart_snapshot
  verify_web_routes
  tikeo_smoke_finalize_report "$REPORT_JSON" passed >/dev/null
  write_summary
  echo "cross-language worker parity report: $REPORT_JSON"
  echo "cross-language worker parity evidence: $REPORT_DIR"
}

main "$@"
