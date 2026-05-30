#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=deploy/smoke/lib/tikee-smoke-lib.sh
source "$ROOT_DIR/deploy/smoke/lib/tikee-smoke-lib.sh"
API_URL="${TIKEE_HTTP_URL:-http://127.0.0.1:19090}"
REPORT_DIR="${TIKEE_API_KEY_REPORT_DIR:-$TIKEE_SMOKE_REPORT_DIR}"
RUN_ID="${TIKEE_API_KEY_RUN_ID:-sdk-api-key-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
mkdir -p "$REPORT_DIR"

tikee_smoke_wait_for_http server "$API_URL/readyz" 30
tikee_smoke_login "$API_URL"

create_body="$(python3 - "$RUN_ID" <<'PY'
import json, sys
run_id = sys.argv[1]
print(json.dumps({
  'name': f'{run_id}-management-key',
  'namespace': 'default',
  'app': 'default',
  'service_account_name': f'{run_id}-sa',
  'scopes': ['jobs:read', 'jobs:manage', 'jobs:execute'],
  'expires_at': None,
}))
PY
)"
create_file="$REPORT_DIR/${RUN_ID}-create.json"
tikee_smoke_api "$API_URL" POST /api/v1/management/api-keys "$create_body" > "$create_file"
api_key="$(tikee_smoke_json_get data.api_key < "$create_file")"
key_id="$(tikee_smoke_json_get data.key.id < "$create_file")"
python3 - "$create_file" <<'PY'
import json, re, sys
payload=json.load(open(sys.argv[1], encoding='utf-8'))
key=payload['data']['api_key']
assert re.fullmatch(r'tk-[A-Za-z0-9]{64}', key), key
summary=payload['data']['key']
assert summary['namespace']=='default'
assert summary['app']=='default'
assert 'jobs:manage' in summary['scopes']
print('api key creation expectation passed')
PY

job_body="$(python3 - "$RUN_ID" <<'PY'
import json, sys
run_id=sys.argv[1]
print(json.dumps({'namespace':'default','app':'default','name':f'{run_id}-sdk-job','schedule_type':'api','processor_name':'demo.echo','enabled':True}))
PY
)"
job_file="$REPORT_DIR/${RUN_ID}-sdk-job.json"
curl -fsS -X POST "$API_URL/api/v1/jobs" -H "x-tikee-api-key: $api_key" -H 'content-type: application/json' -d "$job_body" > "$job_file"
python3 - "$job_file" <<'PY'
import json, sys
payload=json.load(open(sys.argv[1], encoding='utf-8'))
assert payload['data']['name'].endswith('-sdk-job')
assert payload['data']['namespace']=='default'
assert payload['data']['app']=='default'
print('sdk api key scoped job creation expectation passed')
PY

update_body='{"name":"updated-management-key","scopes":["jobs:read","jobs:execute"],"expires_at":null}'
update_file="$REPORT_DIR/${RUN_ID}-update.json"
tikee_smoke_api "$API_URL" PATCH "/api/v1/management/api-keys/$key_id" "$update_body" > "$update_file"
python3 - "$update_file" <<'PY'
import json, sys
summary=json.load(open(sys.argv[1], encoding='utf-8'))['data']
assert summary['name']=='updated-management-key'
assert summary['scopes']==['jobs:read','jobs:execute']
print('api key metadata update expectation passed')
PY

tikee_smoke_record_case sdk-api-key-create passed "$create_file" "created tk-* key and verified app scope"
tikee_smoke_record_case sdk-api-key-use passed "$job_file" "created job using x-tikee-api-key"
tikee_smoke_record_case sdk-api-key-update passed "$update_file" "updated metadata without rotating key"
report="$REPORT_DIR/${RUN_ID}.json"
tikee_smoke_finalize_report "$report" passed >/dev/null
echo "report: $report"
