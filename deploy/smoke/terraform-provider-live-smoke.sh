#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=deploy/smoke/lib/tikee-smoke-lib.sh
source "$ROOT_DIR/deploy/smoke/lib/tikee-smoke-lib.sh"

API_URL="${TIKEE_HTTP_URL:-http://127.0.0.1:19090}"
REPORT_DIR="${TIKEE_GITOPS_REPORT_DIR:-$TIKEE_SMOKE_REPORT_DIR}"
RUN_ID="${TIKEE_GITOPS_RUN_ID:-gitops-tf-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
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

# Seed a job using HTTP API so the manifest has resources to export
job_file="$REPORT_DIR/${RUN_ID}-job.json"
job_body="$(python3 - "$RUN_ID" <<'PY'
import json, sys
run_id=sys.argv[1]
print(json.dumps({
  'namespace':'default',
  'app':'billing',
  'name':f'{run_id}-tf-echo',
  'scheduleType':'api',
  'processorType':'sdk',
  'processorName':'demo.echo',
  'enabled': True,
}))
PY
)"
tikee_smoke_api "$API_URL" POST /api/v1/jobs "$job_body" > "$job_file"

# Run the live test with actual environment variables
cd "$ROOT_DIR/deploy/terraform/provider"
export TIKEE_TEST_HTTP_URL="$API_URL"
export TIKEE_TEST_API_TOKEN="$TIKEE_SMOKE_AUTH_TOKEN"

echo "Running live terraform provider integration tests against server at $API_URL..."
go test ./internal/provider -v -run TestLiveProviderDriftReview

echo "Live terraform provider integration tests passed successfully!"
tikee_smoke_record_case gitops-terraform-provider-live-smoke passed "$REPORT_DIR/${RUN_ID}-job.json" "verified terraform provider manifest export & drift diff client calls"
