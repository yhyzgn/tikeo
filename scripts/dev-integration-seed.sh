#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
API_URL="${TIKEE_HTTP_URL:-${TIKEE_API_URL:-http://127.0.0.1:9090}}"
ADMIN_USER="${TIKEE_SMOKE_ADMIN_USERNAME:-${TIKEE_ADMIN_USERNAME:-smoke_admin}}"
ADMIN_PASSWORD="${TIKEE_SMOKE_ADMIN_PASSWORD:-${TIKEE_ADMIN_PASSWORD:-Tikee@2026!}}"

# shellcheck source=../deploy/smoke/lib/tikee-smoke-lib.sh
source "$ROOT_DIR/deploy/smoke/lib/tikee-smoke-lib.sh"

tikee_smoke_need_cmd curl
tikee_smoke_need_cmd python3

json_field() {
  python3 -c 'import json, sys
payload = json.load(sys.stdin)
cur = payload
for part in sys.argv[1].split("."):
    if part:
        cur = cur[int(part)] if isinstance(cur, list) else cur[part]
print(cur)' "$1"
}

json_body() {
  python3 - "$@" <<'PY'
import json, sys
pairs = [arg.split('=', 1) for arg in sys.argv[1:]]
print(json.dumps({k: v for k, v in pairs}, ensure_ascii=False, separators=(',', ':')))
PY
}

job_body() {
  python3 - "$@" <<'PY'
import json, sys
values = dict(arg.split('=', 1) for arg in sys.argv[1:])
body = {
    'namespace': values['namespace'],
    'app': values['app'],
    'name': values['name'],
    'scheduleType': values.get('scheduleType', 'api'),
    'misfirePolicy': values.get('misfirePolicy', 'fire_once'),
    'processorName': values['processorName'],
    'enabled': True,
}
if values.get('processorType'):
    body['processorType'] = values['processorType']
print(json.dumps(body, ensure_ascii=False, separators=(',', ':')))
PY
}

exists_in_list() {
  local path="$1"
  shift
  tikee_smoke_api "$API_URL" GET "$path" | python3 -c 'import json, sys
payload = json.load(sys.stdin)
criteria = dict(arg.split("=", 1) for arg in sys.argv[1:])
data = payload.get("data") or []
items = data.get("items", []) if isinstance(data, dict) else data
for item in items:
    if all(str(item.get(k)) == v for k, v in criteria.items()):
        sys.exit(0)
sys.exit(1)' "$@"
}

find_job_id() {
  local namespace="$1" app="$2" name="$3"
  tikee_smoke_api "$API_URL" GET /api/v1/jobs | python3 -c 'import json, sys
namespace, app, name = sys.argv[1:4]
payload = json.load(sys.stdin)
for item in payload.get("data", {}).get("items", []):
    if item.get("namespace") == namespace and item.get("app") == app and item.get("name") == name:
        print(item.get("id", ""))
        break' "$namespace" "$app" "$name"
}

create_namespace() {
  local namespace="$1"
  if exists_in_list /api/v1/namespaces name="$namespace"; then
    echo "✅ namespace exists: $namespace"
    return 0
  fi
  tikee_smoke_api "$API_URL" POST /api/v1/namespaces "$(json_body name="$namespace")" >/dev/null
  echo "✅ namespace created: $namespace"
}

create_app() {
  local namespace="$1" app="$2"
  if exists_in_list "/api/v1/apps?namespace=$namespace" namespace="$namespace" name="$app"; then
    echo "✅ app exists: $namespace/$app"
    return 0
  fi
  tikee_smoke_api "$API_URL" POST /api/v1/apps "$(json_body namespace="$namespace" name="$app")" >/dev/null
  echo "✅ app created: $namespace/$app"
}

create_pool() {
  local namespace="$1" app="$2" pool="$3" queue_depth="$4" concurrency="$5"
  if exists_in_list "/api/v1/worker-pools?namespace=$namespace&app=$app" namespace="$namespace" app="$app" name="$pool"; then
    local pool_id
    pool_id="$(tikee_smoke_api "$API_URL" GET "/api/v1/worker-pools?namespace=$namespace&app=$app" | python3 -c 'import json, sys
pool = sys.argv[1]
payload = json.load(sys.stdin)
for item in payload.get("data", []):
    if item.get("name") == pool:
        print(item["id"])
        break' "$pool")"
    if [[ -n "$pool_id" ]]; then
      tikee_smoke_api "$API_URL" PATCH "/api/v1/worker-pools/$pool_id/quota" \
        "{\"max_queue_depth\":$queue_depth,\"max_concurrency\":$concurrency}" >/dev/null
    fi
    echo "✅ worker pool exists: $namespace/$app/$pool"
    return 0
  fi
  local created pool_id
  created="$(tikee_smoke_api "$API_URL" POST /api/v1/worker-pools "$(json_body namespace="$namespace" app="$app" name="$pool")")"
  pool_id="$(printf '%s' "$created" | json_field data.id)"
  tikee_smoke_api "$API_URL" PATCH "/api/v1/worker-pools/$pool_id/quota" \
    "{\"max_queue_depth\":$queue_depth,\"max_concurrency\":$concurrency}" >/dev/null
  echo "✅ worker pool created: $namespace/$app/$pool queue=$queue_depth concurrency=$concurrency"
}

create_job() {
  local namespace="$1" app="$2" name="$3" processor="$4" processor_type="${5:-}"
  local existing
  existing="$(find_job_id "$namespace" "$app" "$name")"
  if [[ -n "$existing" ]]; then
    echo "✅ job exists: $namespace/$app/$name -> $processor ($existing)"
    return 0
  fi
  local created job_id
  created="$(tikee_smoke_api "$API_URL" POST /api/v1/jobs \
    "$(job_body namespace="$namespace" app="$app" name="$name" processorName="$processor" processorType="$processor_type")")"
  job_id="$(printf '%s' "$created" | json_field data.id)"
  echo "✅ job created: $namespace/$app/$name -> $processor ($job_id)"
}

create_plugin_processor() {
  local plugin_type="$1" processor_name="$2"
  if tikee_smoke_api "$API_URL" GET /api/v1/plugins | python3 -c 'import json, sys
plugin_type, processor_name = sys.argv[1:3]
payload = json.load(sys.stdin)
for plugin in payload.get("data", []):
    for processor in plugin.get("processorTypes", []) or plugin.get("processor_types", []):
        names = processor.get("processorNames") or processor.get("processor_names") or []
        if processor.get("type") == plugin_type and processor_name in names:
            sys.exit(0)
sys.exit(1)' "$plugin_type" "$processor_name"; then
    echo "✅ plugin processor exists: $plugin_type/$processor_name"
    return 0
  fi
  tikee_smoke_api "$API_URL" POST /api/v1/plugins "$(python3 - "$plugin_type" "$processor_name" <<'PY'
import json, sys
plugin_type, processor_name = sys.argv[1:3]
print(json.dumps({
  "name": "Demo SQL Processor Plugin",
  "kind": "processor",
  "processorTypes": [{
    "type": plugin_type,
    "label": "SQL Processor",
    "capability": plugin_type,
    "processorNames": [processor_name],
    "description": "Runs demo SQL sync processor tasks"
  }],
  "alertChannelTypes": [],
  "enabled": True
}, ensure_ascii=False, separators=(",", ":")))
PY
)" >/dev/null
  echo "✅ plugin processor created: $plugin_type/$processor_name"
}

printf '等待 tikee server: %s\n' "$API_URL"
tikee_smoke_wait_for_http tikee "$API_URL/healthz" 30

if [[ -n "${TIKEE_SMOKE_AUTH_TOKEN:-}" ]]; then
  export TIKEE_SMOKE_AUTH_TOKEN
  echo "✅ using existing TIKEE_SMOKE_AUTH_TOKEN"
elif [[ -n "${TIKEE_ADMIN_TOKEN:-}" ]]; then
  TIKEE_SMOKE_AUTH_TOKEN="$TIKEE_ADMIN_TOKEN"
  export TIKEE_SMOKE_AUTH_TOKEN
  echo "✅ using existing TIKEE_ADMIN_TOKEN"
else
  if ! tikee_smoke_login "$API_URL" "$ADMIN_USER" "$ADMIN_PASSWORD"; then
    cat >&2 <<ERR
❌ failed to authenticate against $API_URL.
   Provide a valid admin bearer token with TIKEE_SMOKE_AUTH_TOKEN or TIKEE_ADMIN_TOKEN,
   or set TIKEE_ADMIN_USERNAME/TIKEE_ADMIN_PASSWORD for an existing admin account.
ERR
    exit 1
  fi
  echo "✅ authenticated as $ADMIN_USER"
fi

# Integration topology used by scripts/start-java-demo-workers.sh.
create_namespace dev-alpha
create_namespace dev-beta
create_namespace dev-ops

create_app dev-alpha orders
create_app dev-alpha billing
create_app dev-beta analytics
create_app dev-ops automation

create_pool dev-alpha orders boot2-blue 200 8
create_pool dev-alpha orders boot3-blue 200 8
create_pool dev-alpha billing boot4-green 100 4
create_pool dev-beta analytics boot3-batch 150 6
create_pool dev-ops automation boot4-ops 80 3

create_plugin_processor sql billing.sql-sync

create_job dev-alpha orders echo-api demo.echo
create_job dev-alpha orders context-api demo.context
create_job dev-alpha orders bytes-api demo.bytes
create_job dev-alpha billing report-api demo.report
create_job dev-alpha billing sql-sync-api billing.sql-sync sql
create_job dev-beta analytics workflow-step-api demo.workflow.step
create_job dev-beta analytics heartbeat-api demo.heartbeat
create_job dev-ops automation fail-api demo.fail

echo
echo "联调数据已就绪："
echo "  API:        $API_URL"
echo "  namespaces: dev-alpha, dev-beta, dev-ops"
echo "  worker pools: dev-alpha/orders/{boot2-blue,boot3-blue}, dev-alpha/billing/boot4-green, dev-beta/analytics/boot3-batch, dev-ops/automation/boot4-ops"
echo "  next:       scripts/start-java-demo-workers.sh"
