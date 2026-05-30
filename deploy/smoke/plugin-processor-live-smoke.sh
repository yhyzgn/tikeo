#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=deploy/smoke/lib/tikee-smoke-lib.sh
source "$ROOT_DIR/deploy/smoke/lib/tikee-smoke-lib.sh"
API_URL="${TIKEE_HTTP_URL:-http://127.0.0.1:19090}"
REPORT_DIR="${TIKEE_PLUGIN_REPORT_DIR:-$TIKEE_SMOKE_REPORT_DIR}"
RUN_ID="${TIKEE_PLUGIN_RUN_ID:-plugin-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
mkdir -p "$REPORT_DIR"
tikee_smoke_wait_for_http server "$API_URL/readyz" 30
tikee_smoke_login "$API_URL"
body="$(python3 - "$RUN_ID" <<'PY'
import json, sys
run_id=sys.argv[1]
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
plugin_file="$REPORT_DIR/${RUN_ID}-plugin.json"
tikee_smoke_api "$API_URL" POST /api/v1/plugins "$body" > "$plugin_file"
python3 - "$plugin_file" <<'PY'
import json, sys
plugin=json.load(open(sys.argv[1], encoding='utf-8'))['data']
ptype=plugin['processorTypes'][0]
assert ptype['type']=='sql'
assert ptype['processorNames']==['billing.sql-sync']
assert plugin['enabled'] is True
print('plugin structured processor expectation passed')
PY
job_body="$(python3 - "$RUN_ID" <<'PY'
import json, sys
run_id=sys.argv[1]
print(json.dumps({'namespace':'default','app':'default','name':f'{run_id}-plugin-job','scheduleType':'api','processorType':'sql','processorName':'billing.sql-sync','enabled':True}))
PY
)"
job_file="$REPORT_DIR/${RUN_ID}-plugin-job.json"
tikee_smoke_api "$API_URL" POST /api/v1/jobs "$job_body" > "$job_file"
python3 - "$job_file" <<'PY'
import json, sys
job=json.load(open(sys.argv[1], encoding='utf-8'))['data']
assert job['processorType']=='sql' or job.get('processor_type')=='sql'
assert job['processorName']=='billing.sql-sync' or job.get('processor_name')=='billing.sql-sync'
print('plugin task creation expectation passed')
PY
tikee_smoke_record_case plugin-processor-create passed "$plugin_file" "created structured sql plugin processor"
tikee_smoke_record_case plugin-job-create passed "$job_file" "created plugin job using billing.sql-sync"
report="$REPORT_DIR/${RUN_ID}.json"
tikee_smoke_finalize_report "$report" passed >/dev/null
echo "report: $report"
