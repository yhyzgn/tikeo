#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="${TIKEO_MANAGEMENT_TRIGGER_RUN_ID:-management-trigger-e2e-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
REPORT_DIR="${TIKEO_MANAGEMENT_TRIGGER_REPORT_DIR:-$ROOT_DIR/.dev/reports/$RUN_ID}"
API_URL="${TIKEO_HTTP_URL:-http://127.0.0.1:19093}"
WORKER_ENDPOINT="${TIKEO_WORKER_ENDPOINT:-http://127.0.0.1:19993}"
SERVER_CONFIG="$REPORT_DIR/$RUN_ID-config.toml"
SERVER_LOG="$REPORT_DIR/$RUN_ID-server.log"
WORKER_LOG="$REPORT_DIR/$RUN_ID-nodejs-worker.log"
CLIENT_SCRIPT="$REPORT_DIR/$RUN_ID-nodejs-management-client.ts"
SERVER_BIN="$ROOT_DIR/target/debug/tikeo"
DB_PATH="$REPORT_DIR/$RUN_ID.db"
NAMESPACE="${TIKEO_MANAGEMENT_TRIGGER_NAMESPACE:-sdk-smoke}"
APP="${TIKEO_MANAGEMENT_TRIGGER_APP:-management}"
WORKER_POOL="${TIKEO_MANAGEMENT_TRIGGER_WORKER_POOL:-nodejs-blue}"
CLIENT_INSTANCE_ID="${TIKEO_MANAGEMENT_TRIGGER_CLIENT_INSTANCE_ID:-nodejs-management-trigger-smoke}"
REPORT_JSON="$REPORT_DIR/$RUN_ID.json"
SUMMARY_JSON="$REPORT_DIR/$RUN_ID-summary.json"
mkdir -p "$REPORT_DIR"

export TIKEO_SMOKE_REPORT_DIR="$REPORT_DIR"
export TIKEO_SMOKE_RUN_ID="$RUN_ID"
export TIKEO_SMOKE_CASES_FILE="$REPORT_DIR/$RUN_ID-cases.jsonl"
# shellcheck source=../deploy/smoke/lib/tikeo-smoke-lib.sh
source "$ROOT_DIR/deploy/smoke/lib/tikeo-smoke-lib.sh"
: > "$TIKEO_SMOKE_CASES_FILE"

SERVER_PID=""
WORKER_PID=""

cleanup() {
  local code=$?
  if [[ -n "$WORKER_PID" ]] && kill -0 "$WORKER_PID" >/dev/null 2>&1; then
    kill "$WORKER_PID" >/dev/null 2>&1 || true
    wait "$WORKER_PID" 2>/dev/null || true
  fi
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  if (( code != 0 )); then
    echo "management trigger smoke failed; evidence: $REPORT_DIR" >&2
    echo "--- server log tail ---" >&2
    tail -n 160 "$SERVER_LOG" >&2 || true
    echo "--- worker log tail ---" >&2
    tail -n 160 "$WORKER_LOG" >&2 || true
  fi
  exit "$code"
}
trap cleanup EXIT INT TERM

need_cmd() {
  tikeo_smoke_need_cmd "$1"
}

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
listen_addr = "${API_URL#http://}"
worker_tunnel_addr = "${WORKER_ENDPOINT#http://}"

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
  : > "$SERVER_LOG"
  if [[ ! -x "$SERVER_BIN" || "${TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER:-1}" == "1" ]]; then
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
  local namespace="$1" app="$2" pool="$3"
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
  api PATCH "/api/v1/worker-pools/$pool_id/quota" '{"max_queue_depth":100,"max_concurrency":4}' >/dev/null
}

seed_scope() {
  create_namespace "$NAMESPACE"
  create_app "$NAMESPACE" "$APP"
  create_pool "$NAMESPACE" "$APP" "$WORKER_POOL"
  tikeo_smoke_record_case management-scope-seed passed "$REPORT_DIR" "seeded namespace/app/worker_pool for app-scoped SDK smoke"
}

create_sdk_api_key() {
  local service_account_file="$REPORT_DIR/$RUN_ID-service-account.json"
  local api_key_file="$REPORT_DIR/$RUN_ID-api-key.json"
  local service_account_id
  api POST /api/v1/management/service-accounts "$(python3 - "$RUN_ID" "$NAMESPACE" "$APP" "$WORKER_POOL" <<'PY'
import json, sys
run_id, namespace, app, worker_pool = sys.argv[1:5]
print(json.dumps({
    "name": f"{run_id}-sa",
    "description": "Management trigger e2e smoke machine identity",
    "namespace": namespace,
    "app": app,
    "workerPool": worker_pool,
}, ensure_ascii=False, separators=(",", ":")))
PY
)" > "$service_account_file"
  service_account_id="$(tikeo_smoke_json_get data.id < "$service_account_file")"
  api POST /api/v1/management/api-keys "$(python3 - "$RUN_ID" "$NAMESPACE" "$APP" "$service_account_id" <<'PY'
import json, sys
run_id, namespace, app, service_account_id = sys.argv[1:5]
print(json.dumps({
    "name": f"{run_id}-management-trigger-key",
    "namespace": namespace,
    "app": app,
    "service_account_id": service_account_id,
    "scopes": ["jobs:read", "jobs:write", "instances:execute"],
    "expires_at": None,
}, ensure_ascii=False, separators=(",", ":")))
PY
)" > "$api_key_file"
  local api_key
  api_key="$(tikeo_smoke_json_get data.api_key < "$api_key_file")"
  curl -fsS "$API_URL/api/v1/jobs" -H "x-tikeo-api-key: $api_key" > "$REPORT_DIR/$RUN_ID-sdk-key-jobs-list.json"
  tikeo_smoke_record_case management-sdk-api-key passed "$service_account_file $api_key_file" "created app-scoped service account and x-tikeo-api-key"
  printf '%s' "$api_key"
}

ensure_node_dependencies() {
  if [[ ! -d "$ROOT_DIR/examples/nodejs/worker-demo/node_modules" ]]; then
    (cd "$ROOT_DIR/examples/nodejs/worker-demo" && bun install --frozen-lockfile >"$REPORT_DIR/$RUN_ID-bun-install.log" 2>&1)
  fi
}

start_nodejs_worker() {
  : > "$WORKER_LOG"
  (
    cd "$ROOT_DIR/examples/nodejs/worker-demo"
    TIKEO_WORKER_ENDPOINT="$WORKER_ENDPOINT" \
    TIKEO_WORKER_CONNECT=1 \
    TIKEO_WORKER_NAMESPACE="$NAMESPACE" \
    TIKEO_WORKER_APP="$APP" \
    TIKEO_WORKER_POOL="$WORKER_POOL" \
    TIKEO_WORKER_CLUSTER=local \
    TIKEO_WORKER_REGION=local \
    TIKEO_WORKER_CLIENT_INSTANCE_ID="$CLIENT_INSTANCE_ID" \
    TIKEO_WORKER_SDK_PROCESSORS=demo.echo \
    TIKEO_ENABLE_PLUGIN_SQL=0 \
    TIKEO_SANDBOX_AUTO_INSTALL=0 \
    exec bun start >"$WORKER_LOG" 2>&1
  ) &
  WORKER_PID=$!
}

wait_worker_online() {
  local output="$REPORT_DIR/$RUN_ID-workers.json"
  local deadline=$((SECONDS + 180))
  until api GET /api/v1/workers > "$output" && python3 - "$output" "$CLIENT_INSTANCE_ID" "$NAMESPACE" "$APP" "$WORKER_POOL" >/dev/null 2>&1 <<'PY'
import json, sys
path, client_id, namespace, app, worker_pool = sys.argv[1:6]
payload = json.load(open(path, encoding="utf-8"))
items = payload.get("data", {}).get("items", [])
for item in items:
    if item.get("clientInstanceId") != client_id or item.get("status") != "online":
        continue
    if item.get("namespace") != namespace or item.get("app") != app:
        raise SystemExit(f"worker scope mismatch: {item.get('namespace')}/{item.get('app')}")
    structured = item.get("structuredCapabilities") or {}
    if "demo.echo" not in (structured.get("sdkProcessors") or []):
        raise SystemExit(f"worker missing demo.echo capability: {structured}")
    # Worker labels intentionally are not exposed in the public worker DTO;
    # dispatch is verified below through the instance result/log transition.
    if not worker_pool:
        raise SystemExit("worker pool expectation must be non-empty")
    print("nodejs management smoke worker online")
    sys.exit(0)
raise SystemExit(f"missing online worker with clientInstanceId={client_id}")
PY
  do
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for nodejs worker" >&2
      cat "$output" >&2 || true
      tail -n 160 "$WORKER_LOG" >&2 || true
      return 1
    fi
    sleep 2
  done
  tikeo_smoke_record_case management-worker-online passed "$output" "demo worker registered over outbound Worker Tunnel"
}

write_management_client_script() {
  local import_url
  import_url="$(python3 - "$ROOT_DIR/sdks/nodejs/tikeo/src/management.ts" <<'PY'
from pathlib import Path
import sys
print(Path(sys.argv[1]).resolve().as_uri())
PY
)"
  cat > "$CLIENT_SCRIPT" <<'TS'
import { ManagementClient, apiJob, apiTrigger } from "__TIKEO_MANAGEMENT_IMPORT_URL__";

const endpoint = process.env.TIKEO_HTTP_URL ?? "";
const apiKey = process.env.TIKEO_API_KEY ?? "";
const namespace = process.env.TIKEO_NAMESPACE ?? "default";
const app = process.env.TIKEO_APP ?? "default";
const runId = process.env.TIKEO_RUN_ID ?? "management-trigger-e2e";

if (!endpoint || !apiKey) {
  throw new Error("TIKEO_HTTP_URL and TIKEO_API_KEY are required");
}

const client = new ManagementClient(endpoint, apiKey, namespace, app);
const job = await client.createJob(apiJob(
  `${runId}-nodejs-echo-api`,
  "demo.echo",
));
const instance = await client.triggerJob(job.id, apiTrigger());
console.log(JSON.stringify({ job, instance }, null, 2));
TS
  python3 - "$CLIENT_SCRIPT" "$import_url" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
path.write_text(path.read_text().replace("__TIKEO_MANAGEMENT_IMPORT_URL__", sys.argv[2]), encoding="utf-8")
PY
}

create_and_trigger_with_sdk() {
  local api_key="$1"
  local output="$REPORT_DIR/$RUN_ID-sdk-create-trigger.json"
  write_management_client_script
  TIKEO_HTTP_URL="$API_URL" \
  TIKEO_API_KEY="$api_key" \
  TIKEO_NAMESPACE="$NAMESPACE" \
  TIKEO_APP="$APP" \
  TIKEO_RUN_ID="$RUN_ID" \
    bun "$CLIENT_SCRIPT" > "$output"
  python3 - "$output" "$NAMESPACE" "$APP" <<'PY'
import json, sys
path, namespace, app = sys.argv[1:4]
payload = json.load(open(path, encoding="utf-8"))
job = payload["job"]
instance = payload["instance"]
assert job["namespace"] == namespace, job
assert job["app"] == app, job
assert job["name"].endswith("-nodejs-echo-api"), job
assert job["processorName"] == "demo.echo", job
assert instance["jobId"] == job["id"], instance
assert instance["triggerType"] == "api", instance
assert instance["executionMode"] == "single", instance
print(instance["id"])
PY
  tikeo_smoke_record_case management-sdk-create-trigger passed "$output" "Node.js SDK ManagementClient created and triggered an API job"
}

wait_instance_result() {
  local instance_id="$1"
  local instance_file="$REPORT_DIR/$RUN_ID-instance.json"
  local logs_file="$REPORT_DIR/$RUN_ID-instance-logs.json"
  tikeo_smoke_wait_instance_status "$API_URL" "$instance_id" succeeded "$instance_file" 180
  api GET "/api/v1/instances/$instance_id" > "$instance_file"
  api GET "/api/v1/instances/$instance_id/logs" > "$logs_file"
  python3 - "$instance_file" "$logs_file" <<'PY'
import json, sys
instance = json.load(open(sys.argv[1], encoding="utf-8"))["data"]
logs = json.load(open(sys.argv[2], encoding="utf-8"))["data"]["items"]
result = instance.get("result") or {}
if instance.get("status") != "succeeded":
    raise SystemExit(f"expected succeeded instance, got {instance.get('status')}")
if result.get("success") is not True:
    raise SystemExit(f"result.success was not true: {result}")
if result.get("message") != "nodejs demo echo processed":
    raise SystemExit(f"unexpected result message: {result}")
messages = "\n".join(str(item.get("message", "")) for item in logs)
if "nodejs demo echo processed" not in messages:
    raise SystemExit(f"missing nodejs demo echo processed log: {messages}")
if not logs:
    raise SystemExit("expected at least one instance log")
print("instance/result/log assertion passed")
PY
  tikeo_smoke_record_case management-instance-result passed "$instance_file $logs_file" "instance reached succeeded with result.success=true and worker logs"
}

write_summary() {
  python3 - "$REPORT_DIR" "$RUN_ID" "$SUMMARY_JSON" <<'PY'
import datetime, json, pathlib, sys
report_dir = pathlib.Path(sys.argv[1])
run_id = sys.argv[2]
summary = {
    "runId": run_id,
    "generatedAt": datetime.datetime.now(datetime.UTC).isoformat(),
    "reportDir": str(report_dir),
    "evidence": sorted(path.name for path in report_dir.iterdir() if path.is_file()),
}
pathlib.Path(sys.argv[3]).write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding="utf-8")
print(json.dumps(summary, ensure_ascii=False, indent=2))
PY
}

main() {
  need_cmd cargo
  need_cmd curl
  need_cmd python3
  need_cmd bun
  start_server
  tikeo_smoke_login "$API_URL"
  seed_scope
  local api_key instance_id
  api_key="$(create_sdk_api_key)"
  ensure_node_dependencies
  start_nodejs_worker
  wait_worker_online
  instance_id="$(create_and_trigger_with_sdk "$api_key" | tail -n 1)"
  wait_instance_result "$instance_id"
  tikeo_smoke_finalize_report "$REPORT_JSON" passed >/dev/null
  write_summary
  echo "management trigger e2e report: $REPORT_JSON"
  echo "management trigger e2e evidence: $REPORT_DIR"
}

main "$@"
