#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/deploy/compose/database-compat-compose.yml"
RUN_ID="${TIKEO_DB_SEED_COMPAT_RUN_ID:-db-seed-compat-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
REPORT_DIR="${TIKEO_DB_SEED_COMPAT_REPORT_DIR:-$ROOT_DIR/.dev/reports/$RUN_ID}"
POSTGRES_URL="${TIKEO_TEST_POSTGRES_URL:-postgres://tikeo:tikeo@127.0.0.1:${TIKEO_TEST_POSTGRES_PORT:-15432}/tikeo}"
MYSQL_URL="${TIKEO_TEST_MYSQL_URL:-mysql://tikeo:tikeo@127.0.0.1:${TIKEO_TEST_MYSQL_PORT:-13306}/tikeo}"
START_COMPOSE="${TIKEO_DB_COMPAT_COMPOSE:-auto}"
mkdir -p "$REPORT_DIR"

# shellcheck source=../deploy/smoke/lib/tikeo-smoke-lib.sh
source "$ROOT_DIR/deploy/smoke/lib/tikeo-smoke-lib.sh"

cleanup() {
  if [[ "${SERVER_PID:-}" ]]; then
    kill -TERM -- "-$SERVER_PID" >/dev/null 2>&1 || kill -TERM "$SERVER_PID" >/dev/null 2>&1 || true
  fi
  if [[ "${COMPOSE_STARTED:-false}" == "true" ]]; then
    docker compose -f "$COMPOSE_FILE" down -v >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

should_start_compose=false
case "$START_COMPOSE" in
  true) should_start_compose=true ;;
  false) should_start_compose=false ;;
  auto)
    if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
      should_start_compose=true
    fi
    ;;
  *) echo "TIKEO_DB_COMPAT_COMPOSE must be auto, true, or false" >&2; exit 2 ;;
esac

if [[ "$should_start_compose" == "true" ]]; then
  docker compose -f "$COMPOSE_FILE" up -d --wait | tee "$REPORT_DIR/compose-up.log"
  COMPOSE_STARTED=true
fi

write_config() {
  local name="$1" url="$2" api_port="$3" tunnel_port="$4" output="$5"
  cat >"$output" <<EOF
[server]
listen_addr = "127.0.0.1:$api_port"
worker_tunnel_addr = "127.0.0.1:$tunnel_port"

[storage]
timestamp_offset = "+08:00"

$(case "$url" in
  sqlite://*)
    db_path="${url#sqlite://}"; db_path="${db_path%%\?*}"
    printf '[storage.database]
type = "sqlite"
path = "%s"

[storage.database.params]
mode = "rwc"
' "$db_path"
    ;;
  postgres://*)
    printf '[storage.database]
type = "postgres"
host = "127.0.0.1"
port = %s
username = "tikeo"
password = "tikeo"
database = "tikeo"
' "${TIKEO_TEST_POSTGRES_PORT:-15432}"
    ;;
  mysql://*)
    printf '[storage.database]
type = "mysql"
host = "127.0.0.1"
port = %s
username = "tikeo"
password = "tikeo"
database = "tikeo"
' "${TIKEO_TEST_MYSQL_PORT:-13306}"
    ;;
  *) echo "unsupported connection URL: $url" >&2; exit 2 ;;
esac)

[cluster]
mode = "standalone"
node_id = "$name-seed-compat"
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
enabled = true
interval_seconds = 60
batch_size = 50
max_attempts = 3
backoff_seconds = 300

[script_governance]
EOF
}

run_one() {
  local name="$1" url="$2" api_port="$3" tunnel_port="$4"
  local config="$REPORT_DIR/$name.toml" log="$REPORT_DIR/$name-server.log"
  write_config "$name" "$url" "$api_port" "$tunnel_port" "$config"
  (cd "$ROOT_DIR" && setsid cargo run --bin tikeo -- serve --config "$config" >"$log" 2>&1 & echo $! >"$REPORT_DIR/$name-server.pid")
  SERVER_PID="$(cat "$REPORT_DIR/$name-server.pid")"
  for _ in $(seq 1 90); do
    if curl -fsS "http://127.0.0.1:$api_port/healthz" >/dev/null 2>&1; then
      break
    fi
    if ! kill -0 "$SERVER_PID" >/dev/null 2>&1; then
      tail -120 "$log" >&2 || true
      return 1
    fi
    sleep 1
  done
  curl -fsS "http://127.0.0.1:$api_port/healthz" >/dev/null
  tikeo_smoke_login "http://127.0.0.1:$api_port"
  local token
  token="$TIKEO_SMOKE_AUTH_TOKEN"
  (cd "$ROOT_DIR" && TIKEO_HTTP_URL="http://127.0.0.1:$api_port" TIKEO_SMOKE_AUTH_TOKEN="$token" scripts/dev-integration-seed.sh) | tee "$REPORT_DIR/$name-seed.log"
  python3 - <<'PY_LOGIN_RECORD' >"$REPORT_DIR/$name-login.json"
import json, os
print(json.dumps({"code": 0, "data": {"token": os.environ["TIKEO_SMOKE_AUTH_TOKEN"]}}))
PY_LOGIN_RECORD
  curl -fsS "http://127.0.0.1:$api_port/api/v1/jobs" -H "authorization: Bearer $token" >"$REPORT_DIR/$name-jobs.json"
  curl -fsS "http://127.0.0.1:$api_port/api/v1/worker-pools" -H "authorization: Bearer $token" >"$REPORT_DIR/$name-worker-pools.json"
  curl -fsS "http://127.0.0.1:$api_port/api/v1/plugins" -H "authorization: Bearer $token" >"$REPORT_DIR/$name-plugins.json"
  python3 - "$REPORT_DIR/$name-jobs.json" "$REPORT_DIR/$name-worker-pools.json" "$REPORT_DIR/$name-plugins.json" <<'PY'
import json, sys
jobs=json.load(open(sys.argv[1]))['data']['items']
pools=json.load(open(sys.argv[2]))['data']
plugins=json.load(open(sys.argv[3]))['data']
assert len(jobs)==8, len(jobs)
assert len(pools)==9, len(pools)
assert any(any(pt.get('type')=='sql' and 'billing.sql-sync' in pt.get('processorNames',[]) for pt in pl.get('processorTypes',[])) for pl in plugins), plugins
PY
  kill -TERM -- "-$SERVER_PID" >/dev/null 2>&1 || kill -TERM "$SERVER_PID" >/dev/null 2>&1 || true
  wait "$SERVER_PID" 2>/dev/null || true
  SERVER_PID=""
  echo "✅ $name seed API compatibility passed"
}

run_one postgres "$POSTGRES_URL" "${TIKEO_DB_SEED_POSTGRES_API_PORT:-19090}" "${TIKEO_DB_SEED_POSTGRES_TUNNEL_PORT:-19998}"
run_one mysql "$MYSQL_URL" "${TIKEO_DB_SEED_MYSQL_API_PORT:-19091}" "${TIKEO_DB_SEED_MYSQL_TUNNEL_PORT:-19999}"
echo "reports: $REPORT_DIR"
