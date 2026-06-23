#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=deploy/smoke/lib/tikeo-smoke-lib.sh
source "$ROOT_DIR/deploy/smoke/lib/tikeo-smoke-lib.sh"
API_URL="${TIKEO_HTTP_URL:-http://127.0.0.1:19090}"
REPORT_DIR="${TIKEO_API_KEY_REPORT_DIR:-$TIKEO_SMOKE_REPORT_DIR}"
RUN_ID="${TIKEO_API_KEY_RUN_ID:-sdk-api-key-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
SERVER_LOG="$REPORT_DIR/${RUN_ID}-server.log"
SERVER_PID=""
OWN_SERVER=0
mkdir -p "$REPORT_DIR"

cleanup() {
  local code=$?
  if [[ "$OWN_SERVER" == "1" && -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  exit "$code"
}
trap cleanup EXIT INT TERM

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

start_server_if_needed
tikeo_smoke_login "$API_URL"

service_account_body="$(python3 - "$RUN_ID" <<'PY'
import json, sys
run_id = sys.argv[1]
print(json.dumps({
  'name': f'{run_id}-sa',
  'description': 'Smoke-test SDK machine identity',
  'namespace': 'default',
  'app': 'default',
  'workerPool': 'default-pool',
}))
PY
)"
service_account_file="$REPORT_DIR/${RUN_ID}-service-account.json"
tikeo_smoke_api "$API_URL" POST /api/v1/management/service-accounts "$service_account_body" > "$service_account_file"
service_account_id="$(tikeo_smoke_json_get data.id < "$service_account_file")"
python3 - "$service_account_file" <<'PY'
import json, sys
summary=json.load(open(sys.argv[1], encoding='utf-8'))['data']
assert summary['name'].endswith('-sa')
assert summary['status']=='active'
assert summary['namespace']=='default'
assert summary['app']=='default'
assert summary.get('workerPool')=='default-pool'
print('service account creation expectation passed')
PY
tikeo_smoke_api "$API_URL" GET /api/v1/management/service-accounts > "$REPORT_DIR/${RUN_ID}-service-accounts-list.json"
service_account_update_file="$REPORT_DIR/${RUN_ID}-service-account-update.json"
tikeo_smoke_api "$API_URL" PATCH "/api/v1/management/service-accounts/$service_account_id" \
  '{"name":"updated-sdk-smoke-sa","description":"Updated smoke-test SDK machine identity","namespace":"default","app":"default","workerPool":"default-pool","status":"active"}' \
  > "$service_account_update_file"
python3 - "$service_account_update_file" <<'PY'
import json, sys
summary=json.load(open(sys.argv[1], encoding='utf-8'))['data']
assert summary['name']=='updated-sdk-smoke-sa'
assert summary['status']=='active'
assert summary['workerPool']=='default-pool'
print('service account update expectation passed')
PY

create_body="$(python3 - "$RUN_ID" "$service_account_id" <<'PY'
import json, sys
run_id, service_account_id = sys.argv[1:3]
print(json.dumps({
  'name': f'{run_id}-management-key',
  'namespace': 'default',
  'app': 'default',
  'service_account_id': service_account_id,
  'scopes': ['jobs:read', 'jobs:write', 'instances:execute'],
  'expires_at': None,
}))
PY
)"
create_file="$REPORT_DIR/${RUN_ID}-create.json"
tikeo_smoke_api "$API_URL" POST /api/v1/management/api-keys "$create_body" > "$create_file"
api_key="$(tikeo_smoke_json_get data.api_key < "$create_file")"
key_id="$(tikeo_smoke_json_get data.key.id < "$create_file")"
python3 - "$create_file" <<'PY'
import json, re, sys
payload=json.load(open(sys.argv[1], encoding='utf-8'))
key=payload['data']['api_key']
assert re.fullmatch(r'tk-[A-Za-z0-9]{64}', key), key
summary=payload['data']['key']
assert summary['namespace']=='default'
assert summary['app']=='default'
assert summary['service_account_id']
assert 'jobs:write' in summary['scopes']
print('api key creation expectation passed')
PY

revoke_create_body="$(python3 - "$RUN_ID" "$service_account_id" <<'PY'
import json, sys
run_id, service_account_id = sys.argv[1:3]
print(json.dumps({
  'name': f'{run_id}-revoke-key',
  'namespace': 'default',
  'app': 'default',
  'service_account_id': service_account_id,
  'scopes': ['jobs:read'],
  'expires_at': None,
}))
PY
)"
revoke_create_file="$REPORT_DIR/${RUN_ID}-revoke-key-create.json"
tikeo_smoke_api "$API_URL" POST /api/v1/management/api-keys "$revoke_create_body" > "$revoke_create_file"
revoke_key_id="$(tikeo_smoke_json_get data.key.id < "$revoke_create_file")"
revoke_file="$REPORT_DIR/${RUN_ID}-revoke-key.json"
tikeo_smoke_api "$API_URL" DELETE "/api/v1/management/api-keys/$revoke_key_id" > "$revoke_file"

list_file="$REPORT_DIR/${RUN_ID}-key-list.json"
tikeo_smoke_api "$API_URL" GET /api/v1/management/api-keys > "$list_file"
python3 - "$list_file" "$api_key" "$key_id" <<'PY'
import json, sys
payload=json.load(open(sys.argv[1], encoding='utf-8'))
plain=sys.argv[2]
key_id=sys.argv[3]
text=json.dumps(payload, ensure_ascii=False)
assert plain not in text
assert 'key_hash' not in text
items=payload['data']
match=next(item for item in items if item['id']==key_id)
display=match['key_prefix']
assert display.startswith(plain[:12]), display
assert display.endswith(plain[-8:]), display
assert '••••' in display, display
print('api key list redaction expectation passed')
PY

job_body="$(python3 - "$RUN_ID" <<'PY'
import json, sys
run_id=sys.argv[1]
print(json.dumps({'namespace':'default','app':'default','name':f'{run_id}-sdk-job','scheduleType':'api','processorName':'demo.echo','enabled':True}))
PY
)"
job_file="$REPORT_DIR/${RUN_ID}-sdk-job.json"
curl -fsS -X POST "$API_URL/api/v1/jobs" -H "x-tikeo-api-key: $api_key" -H 'content-type: application/json' -d "$job_body" > "$job_file"
python3 - "$job_file" <<'PY'
import json, sys
payload=json.load(open(sys.argv[1], encoding='utf-8'))
assert payload['data']['name'].endswith('-sdk-job')
assert payload['data']['namespace']=='default'
assert payload['data']['app']=='default'
print('sdk api key scoped job creation expectation passed')
PY

denied_file="$REPORT_DIR/${RUN_ID}-sdk-denied.json"
denied_status="$(curl -sS -o "$denied_file" -w '%{http_code}' -X POST "$API_URL/api/v1/jobs" -H "x-tikeo-api-key: $api_key" -H 'content-type: application/json' -d "{\"namespace\":\"default\",\"app\":\"other\",\"name\":\"${RUN_ID}-blocked\",\"scheduleType\":\"api\",\"processorName\":\"demo.echo\",\"enabled\":true}")"
if [[ "$denied_status" != "403" ]]; then
  echo "expected other app request to be forbidden, got $denied_status" >&2
  cat "$denied_file" >&2 || true
  exit 1
fi

java_report_dir="$REPORT_DIR/${RUN_ID}-java-test"
mkdir -p "$java_report_dir"
(
  cd "$ROOT_DIR/sdks/java"
  TIKEO_LIVE_MANAGEMENT_ENDPOINT="$API_URL" \
  TIKEO_LIVE_MANAGEMENT_API_KEY="$api_key" \
  TIKEO_LIVE_MANAGEMENT_NAMESPACE=default \
  TIKEO_LIVE_MANAGEMENT_APP=default \
  TIKEO_LIVE_MANAGEMENT_OTHER_APP=other \
  ./gradlew :tikeo:test --tests net.tikeo.management.client.HttpTikeoJobClientLiveTest --no-daemon --rerun-tasks
) > "$java_report_dir/gradle.log" 2>&1 || {
  cat "$java_report_dir/gradle.log" >&2 || true
  exit 1
}
cp "$ROOT_DIR/sdks/java/tikeo/build/test-results/test/TEST-net.tikeo.management.client.HttpTikeoJobClientLiveTest.xml" "$java_report_dir/TEST-HttpTikeoJobClientLiveTest.xml"

update_body='{"name":"updated-management-key","scopes":["jobs:read","instances:execute"],"expires_at":null}'
update_file="$REPORT_DIR/${RUN_ID}-update.json"
tikeo_smoke_api "$API_URL" PATCH "/api/v1/management/api-keys/$key_id" "$update_body" > "$update_file"
python3 - "$update_file" <<'PY'
import json, sys
summary=json.load(open(sys.argv[1], encoding='utf-8'))['data']
assert summary['name']=='updated-management-key'
assert summary['scopes']==['jobs:read','instances:execute']
print('api key metadata update expectation passed')
PY

post_update_forbidden_file="$REPORT_DIR/${RUN_ID}-post-update-forbidden.json"
post_update_status="$(curl -sS -o "$post_update_forbidden_file" -w '%{http_code}' -X POST "$API_URL/api/v1/jobs" -H "x-tikeo-api-key: $api_key" -H 'content-type: application/json' -d "{\"namespace\":\"default\",\"app\":\"default\",\"name\":\"${RUN_ID}-blocked-by-scope\",\"scheduleType\":\"api\",\"processorName\":\"demo.echo\",\"enabled\":true}")"
if [[ "$post_update_status" != "403" ]]; then
  echo "expected updated key without jobs:write to be forbidden, got $post_update_status" >&2
  cat "$post_update_forbidden_file" >&2 || true
  exit 1
fi

for action in sdk_api_key_create sdk_api_key_update sdk_api_key_authenticate; do
  audit_file="$REPORT_DIR/${RUN_ID}-audit-${action}.json"
  tikeo_smoke_api "$API_URL" GET "/api/v1/audit-logs?action=$action&resource_type=sdk_api_key&resource_id=$key_id&page_size=20" > "$audit_file"
  python3 - "$audit_file" "$action" "$key_id" <<'PY'
import json, sys
payload=json.load(open(sys.argv[1], encoding='utf-8'))
action=sys.argv[2]
key_id=sys.argv[3]
items=payload['data']['items']
assert items, f'missing audit action {action}'
assert any(item['action']==action and item['resource_id']==key_id for item in items)
print(f'audit expectation passed: {action}')
PY
done

revoke_audit_file="$REPORT_DIR/${RUN_ID}-audit-sdk_api_key_revoke.json"
tikeo_smoke_api "$API_URL" GET "/api/v1/audit-logs?action=sdk_api_key_revoke&resource_type=sdk_api_key&resource_id=$revoke_key_id&page_size=5" > "$revoke_audit_file"
python3 - "$revoke_audit_file" "$revoke_key_id" <<'PY'
import json, sys
items=json.load(open(sys.argv[1], encoding='utf-8'))['data']['items']
resource_id=sys.argv[2]
assert any(item['action']=='sdk_api_key_revoke' and item['resource_id']==resource_id for item in items)
print('api key revoke audit expectation passed')
PY

sa_audit_file="$REPORT_DIR/${RUN_ID}-audit-service-account-create.json"
tikeo_smoke_api "$API_URL" GET "/api/v1/audit-logs?action=service_account_create&resource_type=service_account&resource_id=$service_account_id&page_size=5" > "$sa_audit_file"
python3 - "$sa_audit_file" "$service_account_id" <<'PY'
import json, sys
items=json.load(open(sys.argv[1], encoding='utf-8'))['data']['items']
resource_id=sys.argv[2]
assert any(item['resource_id']==resource_id for item in items)
print('service account create audit expectation passed')
PY
sa_update_audit_file="$REPORT_DIR/${RUN_ID}-audit-service-account-update.json"
tikeo_smoke_api "$API_URL" GET "/api/v1/audit-logs?action=service_account_update&resource_type=service_account&resource_id=$service_account_id&page_size=5" > "$sa_update_audit_file"
python3 - "$sa_update_audit_file" "$service_account_id" <<'PY'
import json, sys
items=json.load(open(sys.argv[1], encoding='utf-8'))['data']['items']
resource_id=sys.argv[2]
assert any(item['resource_id']==resource_id for item in items)
print('service account update audit expectation passed')
PY

disable_file="$REPORT_DIR/${RUN_ID}-service-account-disable.json"
tikeo_smoke_api "$API_URL" DELETE "/api/v1/management/service-accounts/$service_account_id" > "$disable_file"
revoked_status="$(curl -sS -o "$REPORT_DIR/${RUN_ID}-revoked-key-rejected.json" -w '%{http_code}' "$API_URL/api/v1/jobs" -H "x-tikeo-api-key: $api_key")"
if [[ "$revoked_status" != "401" ]]; then
  echo "expected disabled service account key to be unauthorized, got $revoked_status" >&2
  cat "$REPORT_DIR/${RUN_ID}-revoked-key-rejected.json" >&2 || true
  exit 1
fi
disable_audit_file="$REPORT_DIR/${RUN_ID}-audit-service-account-disable.json"
tikeo_smoke_api "$API_URL" GET "/api/v1/audit-logs?action=service_account_disable&resource_type=service_account&resource_id=$service_account_id&page_size=5" > "$disable_audit_file"
python3 - "$disable_audit_file" "$service_account_id" <<'PY'
import json, sys
items=json.load(open(sys.argv[1], encoding='utf-8'))['data']['items']
resource_id=sys.argv[2]
assert any(item['resource_id']==resource_id for item in items)
print('service account disable audit expectation passed')
PY

tikeo_smoke_record_case service-account-create passed "$service_account_file" "created managed service account identity"
tikeo_smoke_record_case service-account-update passed "$service_account_update_file" "updated service account metadata without breaking key binding"
tikeo_smoke_record_case sdk-api-key-create passed "$create_file" "created tk-* key bound to existing service account and verified app scope"
tikeo_smoke_record_case sdk-api-key-revoke passed "$revoke_file" "revoked secondary SDK API key and verified revoke audit"
tikeo_smoke_record_case sdk-api-key-list-redacted passed "$list_file" "listed API keys without plaintext or hashes"
tikeo_smoke_record_case sdk-api-key-use passed "$job_file" "created job using x-tikeo-api-key"
tikeo_smoke_record_case sdk-api-key-scope-deny passed "$denied_file" "denied SDK key write outside bound app"
tikeo_smoke_record_case sdk-api-key-update passed "$update_file" "updated metadata without rotating key"
tikeo_smoke_record_case java-management-client-api-key passed "$java_report_dir/TEST-HttpTikeoJobClientLiveTest.xml" "Java management client used live x-tikeo-api-key against server"
tikeo_smoke_record_case sdk-api-key-audit passed "$REPORT_DIR/${RUN_ID}-audit-sdk_api_key_authenticate.json" "verified SDK key create/update/use audit events"
tikeo_smoke_record_case service-account-disable-cascade passed "$disable_file" "disabled service account revoked bound SDK API key"
report="$REPORT_DIR/${RUN_ID}.json"
tikeo_smoke_finalize_report "$report" passed >/dev/null
echo "report: $report"
