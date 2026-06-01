#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=deploy/smoke/lib/tikee-smoke-lib.sh
source "$ROOT_DIR/deploy/smoke/lib/tikee-smoke-lib.sh"

API_URL="${TIKEE_HTTP_URL:-http://127.0.0.1:19090}"
WEB_URL="${TIKEE_WEB_URL:-http://127.0.0.1:15173}"
WEB_PORT="${WEB_URL##*:}"
WEB_PORT="${WEB_PORT%%/*}"
REPORT_DIR="${TIKEE_WEB_REPORT_DIR:-$TIKEE_SMOKE_REPORT_DIR}"
RUN_ID="${TIKEE_WEB_RUN_ID:-web-live-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
WEB_LOG="$REPORT_DIR/${RUN_ID}-web.log"
WEB_PID=""
OWN_WEB=0
mkdir -p "$REPORT_DIR"

cleanup() {
  local code=$?
  if [[ "$OWN_WEB" == "1" && -n "$WEB_PID" ]] && kill -0 "$WEB_PID" >/dev/null 2>&1; then
    kill "$WEB_PID" >/dev/null 2>&1 || true
    wait "$WEB_PID" 2>/dev/null || true
  fi
  exit "$code"
}
trap cleanup EXIT INT TERM

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || { echo "missing command: $1" >&2; exit 127; }
}
need_cmd curl
need_cmd python3
need_cmd bun

start_web_if_needed() {
  if curl -fsS "$WEB_URL" >/dev/null 2>&1; then
    return
  fi
  OWN_WEB=1
  (cd "$ROOT_DIR/web" && bun run dev -- --host 127.0.0.1 --port "$WEB_PORT" >"$WEB_LOG" 2>&1) &
  WEB_PID=$!
  tikee_smoke_wait_for_http web "$WEB_URL" 90 || {
    tail -n 120 "$WEB_LOG" >&2 || true
    return 1
  }
}

fetch_page() {
  local path="$1"
  local output="$2"
  curl -fsS "$WEB_URL$path" -o "$output"
}

main() {
  start_web_if_needed
  local route_auth_log root_html login_html api_keys_html workers_html jobs_topology_html workflow_new_html workflow_edit_html gitops_html
  route_auth_log="$REPORT_DIR/${RUN_ID}-route-auth-test.log"
  root_html="$REPORT_DIR/${RUN_ID}-root.html"
  login_html="$REPORT_DIR/${RUN_ID}-login.html"
  api_keys_html="$REPORT_DIR/${RUN_ID}-api-keys.html"
  workers_html="$REPORT_DIR/${RUN_ID}-workers.html"
  jobs_topology_html="$REPORT_DIR/${RUN_ID}-jobs-topology.html"
  workflow_new_html="$REPORT_DIR/${RUN_ID}-workflow-new.html"
  workflow_edit_html="$REPORT_DIR/${RUN_ID}-workflow-edit.html"
  gitops_html="$REPORT_DIR/${RUN_ID}-gitops.html"

  (cd "$ROOT_DIR/web" && bun test src/pages/__tests__/RouteAuth.test.tsx) > "$route_auth_log" 2>&1 || {
    cat "$route_auth_log" >&2 || true
    return 1
  }

  fetch_page / "$root_html"
  fetch_page /login "$login_html"
  fetch_page /api-keys "$api_keys_html"
  fetch_page /workers "$workers_html"
  fetch_page /jobs/topology "$jobs_topology_html"
  fetch_page /workflows/new "$workflow_new_html"
  fetch_page /workflows/wf_smoke/edit "$workflow_edit_html"
  fetch_page /gitops "$gitops_html"

  tikee_smoke_assert web "$root_html" --require-text '<div id="root"></div>' >/dev/null
  tikee_smoke_assert web "$login_html" --require-text '<div id="root"></div>' >/dev/null
  tikee_smoke_assert web "$api_keys_html" --require-text '<div id="root"></div>' --forbid-text '404 Not Found' >/dev/null
  tikee_smoke_assert web "$workers_html" --require-text '<div id="root"></div>' --forbid-text '404 Not Found' >/dev/null
  tikee_smoke_assert web "$jobs_topology_html" --require-text '<div id="root"></div>' --forbid-text '404 Not Found' >/dev/null
  tikee_smoke_assert web "$workflow_new_html" --require-text '<div id="root"></div>' --forbid-text '404 Not Found' >/dev/null
  tikee_smoke_assert web "$workflow_edit_html" --require-text '<div id="root"></div>' --forbid-text '404 Not Found' >/dev/null
  tikee_smoke_assert web "$gitops_html" --require-text '<div id="root"></div>' --forbid-text '404 Not Found' >/dev/null

  tikee_smoke_record_case web-root passed "$root_html" "SPA root route returned app shell"
  tikee_smoke_record_case web-route-auth passed "$route_auth_log" "RouteAuth unit coverage asserts root dashboard redirect and authenticated login bypass"
  tikee_smoke_record_case web-login-route passed "$login_html" "login route returned SPA shell"
  tikee_smoke_record_case web-api-keys-refresh passed "$api_keys_html" "api-keys secondary route refresh did not return 404"
  tikee_smoke_record_case web-workers-route passed "$workers_html" "workers route returned app shell"
  tikee_smoke_record_case web-jobs-topology-refresh passed "$jobs_topology_html" "jobs topology secondary route refresh did not return 404"
  tikee_smoke_record_case web-workflow-new-refresh passed "$workflow_new_html" "workflow new secondary route refresh did not return 404"
  tikee_smoke_record_case web-workflow-edit-refresh passed "$workflow_edit_html" "workflow edit secondary route refresh did not return 404"
  tikee_smoke_record_case web-gitops-route passed "$gitops_html" "gitops route returned app shell"

  local report="$REPORT_DIR/${RUN_ID}.json"
  tikee_smoke_finalize_report "$report" passed >/dev/null
  echo "report: $report"
}

main "$@"
