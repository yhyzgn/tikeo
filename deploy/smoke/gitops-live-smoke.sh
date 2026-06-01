#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=deploy/smoke/lib/tikee-smoke-lib.sh
source "$ROOT_DIR/deploy/smoke/lib/tikee-smoke-lib.sh"

API_URL="${TIKEE_HTTP_URL:-http://127.0.0.1:19090}"
REPORT_DIR="${TIKEE_GITOPS_REPORT_DIR:-$TIKEE_SMOKE_REPORT_DIR}"
RUN_ID="${TIKEE_GITOPS_RUN_ID:-gitops-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
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
  tikee_smoke_wait_for_http server "$API_URL/readyz" 120 || {
    tail -n 160 "$SERVER_LOG" >&2 || true
    return 1
  }
}

start_server_if_needed
tikee_smoke_login "$API_URL"

job_file="$REPORT_DIR/${RUN_ID}-job.json"
job_body="$(python3 - "$RUN_ID" <<'PY'
import json, sys
run_id=sys.argv[1]
print(json.dumps({
  'namespace':'default',
  'app':'billing',
  'name':f'{run_id}-gitops-echo',
  'scheduleType':'api',
  'processorType':'sdk',
  'processorName':'demo.echo',
  'enabled': True,
}))
PY
)"
tikee_smoke_api "$API_URL" POST /api/v1/jobs "$job_body" > "$job_file"

manifest_file="$REPORT_DIR/${RUN_ID}-manifest.json"
tikee_smoke_api "$API_URL" GET "/api/v1/gitops/manifest?namespace=default&app=billing&format=yaml" > "$manifest_file"
python3 - "$manifest_file" "$RUN_ID" <<'PY'
import json, sys
payload=json.load(open(sys.argv[1], encoding='utf-8'))
run_id=sys.argv[2]
data=payload['data']
assert data['format']=='yaml'
assert data['manifestYaml'] and 'apiVersion' in data['manifestYaml']
assert data['checksum'] and data['checksum'].startswith('sha256:') and len(data['checksum']) == 71
resources=data['manifest']['resources']
jobs=[item for item in resources if item['kind']=='Job' and item['metadata']['name']==f'{run_id}-gitops-echo']
assert jobs, 'manifest should contain seeded Job resource'
print('gitops manifest export expectation passed')
PY

desired_file="$REPORT_DIR/${RUN_ID}-desired.json"
python3 - "$manifest_file" "$desired_file" <<'PY'
import json, sys
source, target=sys.argv[1:3]
manifest=json.load(open(source, encoding='utf-8'))['data']['manifest']
for resource in manifest['resources']:
    if resource['kind'] == 'Job':
        resource['spec']['enabled'] = False
        break
json.dump({'manifest': manifest}, open(target, 'w', encoding='utf-8'), ensure_ascii=False)
PY

diff_file="$REPORT_DIR/${RUN_ID}-diff.json"
curl -fsS -X POST "$API_URL/api/v1/gitops/diff" \
  -H "authorization: Bearer $TIKEE_SMOKE_AUTH_TOKEN" \
  -H 'content-type: application/json' \
  --data-binary "@$desired_file" > "$diff_file"
python3 - "$diff_file" <<'PY'
import json, sys
payload=json.load(open(sys.argv[1], encoding='utf-8'))
data=payload['data']
assert data['currentChecksum'] and data['desiredChecksum']
assert data['currentChecksum'] != data['desiredChecksum']
assert data['summary'].get('update', 0) >= 1
changes=data['changes']
assert any(change['action']=='update' and change['kind']=='Job' and 'enabled' in change['diff'] for change in changes)
print('gitops manifest diff expectation passed')
PY

tikee_smoke_record_case gitops-manifest-export passed "$manifest_file" "exported YAML manifest with checksum and seeded Job resource"
tikee_smoke_record_case gitops-manifest-diff passed "$diff_file" "reported review-first drift diff for desired manifest change"
report="$REPORT_DIR/${RUN_ID}.json"
tikee_smoke_finalize_report "$report" passed >/dev/null
echo "report: $report"
