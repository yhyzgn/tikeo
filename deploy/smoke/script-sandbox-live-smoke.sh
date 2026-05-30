#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=deploy/smoke/lib/tikee-smoke-lib.sh
source "$ROOT_DIR/deploy/smoke/lib/tikee-smoke-lib.sh"
API_URL="${TIKEE_HTTP_URL:-http://127.0.0.1:19090}"
REPORT_DIR="${TIKEE_SCRIPT_REPORT_DIR:-$TIKEE_SMOKE_REPORT_DIR}"
RUN_ID="${TIKEE_SCRIPT_RUN_ID:-script-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
mkdir -p "$REPORT_DIR"
tikee_smoke_wait_for_http server "$API_URL/readyz" 30
tikee_smoke_login "$API_URL"
create_script() {
  local language="$1" content="$2"
  local file="$REPORT_DIR/${RUN_ID}-${language}.json"
  local body
  body="$(python3 - "$RUN_ID" "$language" "$content" <<'PY'
import json, sys
run_id, language, content = sys.argv[1:]
print(json.dumps({'name':f'{run_id}-{language}-script','language':language,'version':'1.0.0','content':content,'timeout_seconds':30,'max_memory_bytes':67108864,'allow_network':False}))
PY
)"
  tikee_smoke_api "$API_URL" POST /api/v1/scripts "$body" > "$file"
  python3 - "$file" "$language" <<'PY'
import json, sys
script=json.load(open(sys.argv[1], encoding='utf-8'))['data']
expected=sys.argv[2]
assert script['language']==expected
print(f'{expected} script creation expectation passed')
PY
  tikee_smoke_record_case "script-$language-create" passed "$file" "created $language script definition"
}
# This script validates server/web/sdk script definitions are accepted. Full execution
# depends on live worker sandbox runtimes and is covered by joint e2e when enabled.
create_script shell 'echo tikee-shell-smoke'
create_script python 'print("tikee-python-smoke")'
create_script javascript 'console.log("tikee-js-smoke")'
create_script typescript 'console.log("tikee-ts-smoke")'
create_script rhai 'print("tikee-rhai-smoke");'
report="$REPORT_DIR/${RUN_ID}.json"
tikee_smoke_finalize_report "$report" passed >/dev/null
echo "report: $report"
