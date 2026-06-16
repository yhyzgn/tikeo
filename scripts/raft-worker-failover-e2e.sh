#!/usr/bin/env bash
set -euo pipefail

# Multi-process E2E for Server Raft HA + Worker Tunnel leader-local registration.
# Starts Docker Postgres, 3 local tikeo server processes, a local TCP round-robin
# proxy for API and Worker Tunnel, and the Node.js worker demo.
# Verifies: exactly one schedulable leader, worker registers on the leader, killing
# the leader elects a new leader, worker reconnects through the tunnel LB, and a
# post-failover job succeeds.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="${TIKEO_RAFT_WORKER_E2E_RUN_ID:-raft-worker-failover-$(date -u +%Y%m%dt%H%M%Sz)-$$}"
REPORT_DIR="${TIKEO_RAFT_WORKER_E2E_REPORT_DIR:-$ROOT_DIR/.dev/reports/$RUN_ID}"
if [[ "$REPORT_DIR" != /* ]]; then
  REPORT_DIR="$ROOT_DIR/$REPORT_DIR"
fi
POSTGRES_NAME="${TIKEO_RAFT_WORKER_E2E_POSTGRES_NAME:-$RUN_ID-postgres}"
POSTGRES_IMAGE="${TIKEO_RAFT_WORKER_E2E_POSTGRES_IMAGE:-postgres:16-alpine}"
KEEP="${TIKEO_RAFT_WORKER_E2E_KEEP:-0}"
TOKEN="${TIKEO_RAFT_WORKER_E2E_TOKEN:-dev-raft-worker-$(od -An -N12 -tx1 /dev/urandom | tr -d ' \n')}"
NODE_COUNT=3
NAMESPACE="${TIKEO_RAFT_WORKER_E2E_NAMESPACE:-raft-ha}"
APP="${TIKEO_RAFT_WORKER_E2E_APP:-failover}"
WORKER_POOL="${TIKEO_RAFT_WORKER_E2E_WORKER_POOL:-nodejs-blue}"
CLIENT_INSTANCE_ID="${TIKEO_RAFT_WORKER_E2E_CLIENT_INSTANCE_ID:-nodejs-raft-failover-worker}"
SERVER_BIN="$ROOT_DIR/target/debug/tikeo"
CASES_FILE="$REPORT_DIR/$RUN_ID-cases.jsonl"
REPORT_JSON="$REPORT_DIR/$RUN_ID.json"

mkdir -p "$REPORT_DIR"
: > "$CASES_FILE"
export TIKEO_SMOKE_REPORT_DIR="$REPORT_DIR"
export TIKEO_SMOKE_RUN_ID="$RUN_ID"
export TIKEO_SMOKE_CASES_FILE="$CASES_FILE"
# shellcheck source=../deploy/smoke/lib/tikeo-smoke-lib.sh
source "$ROOT_DIR/deploy/smoke/lib/tikeo-smoke-lib.sh"

SERVER_PIDS=()
PROXY_PID=""
WORKER_PID=""
POSTGRES_PORT=""
API_PROXY_PORT=""
TUNNEL_PROXY_PORT=""

auth_header=()
log() { printf '[raft-worker-e2e] %s\n' "$*"; }
need_cmd() { tikeo_smoke_need_cmd "$1"; }

free_port() {
  python3 - <<'PY'
import socket
s=socket.socket()
s.bind(('127.0.0.1', 0))
print(s.getsockname()[1])
s.close()
PY
}

cleanup() {
  local code=$?
  if [[ -n "$WORKER_PID" ]] && kill -0 "$WORKER_PID" >/dev/null 2>&1; then
    kill "$WORKER_PID" >/dev/null 2>&1 || true
    wait "$WORKER_PID" 2>/dev/null || true
  fi
  if [[ -n "$PROXY_PID" ]] && kill -0 "$PROXY_PID" >/dev/null 2>&1; then
    kill "$PROXY_PID" >/dev/null 2>&1 || true
    wait "$PROXY_PID" 2>/dev/null || true
  fi
  for pid in "${SERVER_PIDS[@]:-}"; do
    if [[ -n "$pid" ]] && kill -0 "$pid" >/dev/null 2>&1; then
      kill "$pid" >/dev/null 2>&1 || true
      wait "$pid" 2>/dev/null || true
    fi
  done
  if [[ "$KEEP" == "1" ]]; then
    log "keeping postgres container and reports for inspection: $POSTGRES_NAME report=$REPORT_DIR"
    exit "$code"
  fi
  docker rm -f "$POSTGRES_NAME" >/dev/null 2>&1 || true
  exit "$code"
}
trap cleanup EXIT INT TERM

api() {
  tikeo_smoke_api "http://127.0.0.1:$API_PROXY_PORT" "$@"
}

json_get_file() {
  python3 -c 'import json,sys
cur=json.load(open(sys.argv[1], encoding="utf-8"))
for part in sys.argv[2].split("."):
    cur = cur[int(part)] if isinstance(cur, list) else cur[part]
print(cur)' "$1" "$2"
}

record() { tikeo_smoke_record_case "$1" "$2" "${3:-}" "${4:-}"; }


collect_fsod_db_evidence() {
  local label="$1"
  local output="$REPORT_DIR/fsod-db-$label.json"
  docker exec -i "$POSTGRES_NAME" psql -U tikeo -d tikeo -t -A > "$output" <<SQL
SELECT jsonb_pretty(jsonb_build_object(
  'label', '$label',
  'capturedAt', now(),
  'clusterShardOwnership', COALESCE((
    SELECT jsonb_agg(jsonb_build_object(
      'shardId', shard_id,
      'shardMapVersion', shard_map_version,
      'shardCount', shard_count,
      'ownerNodeId', owner_node_id,
      'epoch', epoch,
      'raftTerm', raft_term,
      'status', status,
      'hasFencingToken', fencing_token IS NOT NULL AND fencing_token <> '',
      'leaseExpiresAt', lease_expires_at,
      'updatedAt', updated_at
    ) ORDER BY shard_id)
    FROM cluster_shard_ownership
  ), '[]'::jsonb),
  'workerSessions', COALESCE((
    SELECT jsonb_agg(jsonb_build_object(
      'workerId', worker_id,
      'logicalInstanceId', logical_instance_id,
      'gatewayNodeId', gateway_node_id,
      'generation', generation,
      'status', status,
      'clientInstanceId', (SELECT client_instance_id FROM worker_logical_instances WHERE worker_logical_instances.id = worker_sessions.logical_instance_id),
      'leaseExpiresAt', lease_expires_at,
      'updatedAt', updated_at
    ) ORDER BY updated_at, worker_id)
    FROM worker_sessions
  ), '[]'::jsonb),
  'workerDispatchOutbox', COALESCE((
    SELECT jsonb_agg(jsonb_build_object(
      'id', id,
      'instanceId', instance_id,
      'attemptId', attempt_id,
      'workerId', worker_id,
      'logicalInstanceId', logical_instance_id,
      'gatewayNodeId', gateway_node_id,
      'gatewayGeneration', gateway_generation,
      'shardId', shard_id,
      'shardMapVersion', shard_map_version,
      'shardCount', shard_count,
      'ownerNodeId', owner_node_id,
      'ownerEpoch', owner_epoch,
      'hasOwnerFencingToken', owner_fencing_token IS NOT NULL AND owner_fencing_token <> '',
      'hasAssignmentToken', assignment_token IS NOT NULL AND assignment_token <> '',
      'status', status,
      'deliveryAttempts', delivery_attempts,
      'visibilityDeadline', visibility_deadline,
      'lastError', last_error,
      'updatedAt', updated_at
    ) ORDER BY created_at, id)
    FROM worker_dispatch_outbox
  ), '[]'::jsonb),
  'dispatchQueue', COALESCE((
    SELECT jsonb_agg(jsonb_build_object(
      'id', id,
      'jobInstanceId', job_instance_id,
      'workflowNodeInstanceId', workflow_node_instance_id,
      'shardId', shard_id,
      'shardMapVersion', shard_map_version,
      'shardCount', shard_count,
      'ownerEpoch', owner_epoch,
      'hasOwnerFencingToken', owner_fencing_token IS NOT NULL AND owner_fencing_token <> '',
      'status', status,
      'leaseOwner', lease_owner,
      'updatedAt', updated_at
    ) ORDER BY created_at, id)
    FROM dispatch_queue
  ), '[]'::jsonb)
));
SQL
  python3 -m json.tool "$output" > "$output.tmp" && mv "$output.tmp" "$output"
  printf '%s' "$output"
}


assert_fsod_db_evidence() {
  local label="$1"
  local file="$2"
  python3 - "$label" "$file" <<'PYFSOD'
import json, sys
label, path = sys.argv[1], sys.argv[2]
payload = json.load(open(path, encoding='utf-8'))
outbox = payload.get('workerDispatchOutbox') or []
ownership = payload.get('clusterShardOwnership') or []
if label == 'initial-registration':
    if not ownership:
        raise SystemExit(f"{label}: expected projected shard ownership")
    if not all(row.get('epoch', 0) > 0 and row.get('hasFencingToken') for row in ownership):
        raise SystemExit(f"{label}: shard ownership rows must have positive epoch and fencing token")
    sys.exit(0)
if not outbox:
    raise SystemExit(f"{label}: expected durable worker dispatch outbox rows")
active_by_key = {
    (row.get('shardId'), row.get('shardMapVersion'), row.get('shardCount')): row
    for row in ownership
    if row.get('status') == 'active'
}
current_owner_completed = []
for row in outbox:
    key = (row.get('shardId'), row.get('shardMapVersion'), row.get('shardCount'))
    owner = active_by_key.get(key)
    if row.get('ownerEpoch', 0) <= 0:
        raise SystemExit(f"{label}: outbox row {row.get('id')} must bind positive ownerEpoch: {row}")
    if not row.get('hasOwnerFencingToken'):
        raise SystemExit(f"{label}: outbox row {row.get('id')} missing owner fencing token")
    if row.get('status') not in {'delivered', 'acked', 'completed'}:
        raise SystemExit(f"{label}: expected dispatched outbox terminal/progress status, got {row.get('status')} for {row.get('id')}")
    if owner and row.get('ownerNodeId') == owner.get('ownerNodeId') and row.get('ownerEpoch') == owner.get('epoch') and row.get('status') == 'completed':
        current_owner_completed.append(row)
completed = [row for row in outbox if row.get('status') == 'completed']
if not completed:
    raise SystemExit(f"{label}: expected at least one completed outbox row after successful instance")
if label in {'before-failover', 'after-failover'} and not current_owner_completed:
    raise SystemExit(f"{label}: expected at least one completed outbox row bound to the current active shard owner")
PYFSOD
}

collect_api_evidence() {
  local label="$1"
  local metrics_file="$REPORT_DIR/metrics-$label.json"
  local diagnostics_file="$REPORT_DIR/cluster-diagnostics-$label.json"
  api GET /api/v1/metrics/summary > "$metrics_file"
  api GET /api/v1/cluster/diagnostics > "$diagnostics_file"
  python3 -m json.tool "$metrics_file" > "$metrics_file.tmp" && mv "$metrics_file.tmp" "$metrics_file"
  python3 -m json.tool "$diagnostics_file" > "$diagnostics_file.tmp" && mv "$diagnostics_file.tmp" "$diagnostics_file"
  python3 - "$metrics_file" <<'PYMETRICS'
import json, sys
payload=json.load(open(sys.argv[1], encoding='utf-8'))['data']
if payload['outbox']['total'] < 1:
    raise SystemExit(f"expected at least one durable outbox row: {payload['outbox']}")
if payload['shard_ownership']['active'] < 1:
    raise SystemExit(f"expected active shard ownership projection: {payload['shard_ownership']}")
PYMETRICS
  record "fsod-api-evidence-$label" passed "$metrics_file" "metrics summary includes outbox and shard ownership after $label"
}

wait_postgres() {
  for _ in $(seq 1 90); do
    if docker exec "$POSTGRES_NAME" pg_isready -U tikeo -d tikeo >/dev/null 2>&1; then return 0; fi
    sleep 1
  done
  docker logs "$POSTGRES_NAME" >&2 || true
  return 1
}

wait_for_http() {
  local label="$1"
  local url="$2"
  local timeout="${3:-120}"
  local deadline=$((SECONDS + timeout))
  until curl -fsS "$url" >/dev/null 2>&1; do
    if (( SECONDS >= deadline )); then
      log "timed out waiting for $label at $url"
      for file in "$REPORT_DIR"/*-server.log; do echo "--- $file" >&2; tail -n 160 "$file" >&2 || true; done
      return 1
    fi
    sleep 1
  done
}

build_server_binary() {
  need_cmd cargo
  if [[ ! -x "$SERVER_BIN" || "${TIKEO_RAFT_WORKER_E2E_REBUILD_SERVER:-1}" == "1" ]]; then
    log "building local tikeo server binary"
    (cd "$ROOT_DIR" && cargo build --bin tikeo)
  fi
}

peers_toml() {
  for ((i=0; i<NODE_COUNT; i++)); do
    printf '  { node_id = "%s-%d", endpoint = "http://127.0.0.1:%s" }' "$RUN_ID" "$i" "${HTTP_PORTS[$i]}"
    if (( i + 1 < NODE_COUNT )); then printf ',\n'; else printf '\n'; fi
  done
}

write_node_config() {
  local i="$1"
  local node="$RUN_ID-$i"
  local config="$REPORT_DIR/$node.toml"
  cat > "$config" <<CONFIG
[server]
listen_addr = "0.0.0.0:${HTTP_PORTS[$i]}"
worker_tunnel_addr = "0.0.0.0:${TUNNEL_PORTS[$i]}"

[storage]
database_url = "postgres://tikeo:tikeo@127.0.0.1:${POSTGRES_PORT}/tikeo"

[cluster]
mode = "raft"
node_id = "${node}"
peers = [
$(peers_toml)
]

[alert_retry]
enabled = false

[notification_delivery]
enabled = false
CONFIG
}

start_node() {
  local i="$1"
  local node="$RUN_ID-$i"
  local config="$REPORT_DIR/$node.toml"
  local log_file="$REPORT_DIR/$node-server.log"
  write_node_config "$i"
  : > "$log_file"
  (
    cd "$ROOT_DIR"
    TIKEO__CLUSTER__TRANSPORT_TOKEN="$TOKEN" exec "$SERVER_BIN" serve --config "$config" >>"$log_file" 2>&1
  ) &
  SERVER_PIDS[$i]=$!
}

write_tcp_proxy() {
  local script="$REPORT_DIR/tcp_proxy.py"
  cat > "$script" <<'PY'
import asyncio, itertools, json, os, sys

api_port = int(os.environ['API_PROXY_PORT'])
tunnel_port = int(os.environ['TUNNEL_PROXY_PORT'])
api_backends = json.loads(os.environ['API_BACKENDS'])
tunnel_backends = json.loads(os.environ['TUNNEL_BACKENDS'])

async def pipe(reader, writer):
    try:
        while True:
            data = await reader.read(65536)
            if not data:
                break
            writer.write(data)
            await writer.drain()
    finally:
        try:
            writer.close()
            await writer.wait_closed()
        except Exception:
            pass

async def handle(client_reader, client_writer, backends, rr):
    last = None
    for _ in range(len(backends)):
        backend = next(rr)
        try:
            upstream_reader, upstream_writer = await asyncio.open_connection(backend[0], backend[1])
            await asyncio.gather(pipe(client_reader, upstream_writer), pipe(upstream_reader, client_writer))
            return
        except Exception as exc:
            last = exc
            continue
    client_writer.close()
    await client_writer.wait_closed()
    if last:
        print(f"all backends failed: {last}", file=sys.stderr, flush=True)

async def serve(port, backends):
    rr = itertools.cycle(backends)
    server = await asyncio.start_server(lambda r, w: handle(r, w, backends, rr), '127.0.0.1', port)
    async with server:
        await server.serve_forever()

async def main():
    await asyncio.gather(serve(api_port, api_backends), serve(tunnel_port, tunnel_backends))

asyncio.run(main())
PY
}

start_proxy() {
  write_tcp_proxy
  API_BACKENDS="$(python3 - "${HTTP_PORTS[@]}" <<'PY'
import json, sys
print(json.dumps([["127.0.0.1", int(port)] for port in sys.argv[1:]]))
PY
)"
  TUNNEL_BACKENDS="$(python3 - "${TUNNEL_PORTS[@]}" <<'PY'
import json, sys
print(json.dumps([["127.0.0.1", int(port)] for port in sys.argv[1:]]))
PY
)"
  API_PROXY_PORT="$API_PROXY_PORT" TUNNEL_PROXY_PORT="$TUNNEL_PROXY_PORT" API_BACKENDS="$API_BACKENDS" TUNNEL_BACKENDS="$TUNNEL_BACKENDS" \
    python3 "$REPORT_DIR/tcp_proxy.py" >"$REPORT_DIR/tcp-proxy.log" 2>&1 &
  PROXY_PID=$!
}

cluster_json() {
  local i="$1"
  curl -fsS "http://127.0.0.1:${HTTP_PORTS[$i]}/api/v1/cluster"
}

current_leader() {
  local out="$REPORT_DIR/current-leader.jsonl"
  : > "$out"
  for ((i=0; i<NODE_COUNT; i++)); do
    if cluster_json "$i" > "$REPORT_DIR/$RUN_ID-$i.cluster.json" 2>/dev/null; then
      python3 - "$REPORT_DIR/$RUN_ID-$i.cluster.json" "$out" <<'PY'
import json, sys
payload=json.load(open(sys.argv[1], encoding='utf-8'))['data']
can = payload.get('canSchedule', payload.get('can_schedule', False))
if can:
    with open(sys.argv[2], 'a', encoding='utf-8') as fh:
        fh.write(json.dumps(payload) + '\n')
PY
    fi
  done
  python3 - "$out" <<'PY'
import json, sys
lines=[line for line in open(sys.argv[1], encoding='utf-8') if line.strip()]
if len(lines) != 1:
    raise SystemExit(1)
payload=json.loads(lines[0])
print(payload.get('nodeId', payload.get('node_id')))
PY
}

wait_for_unique_leader() {
  local previous="${1:-}"
  local leader=""
  local deadline=$((SECONDS + 180))
  while (( SECONDS < deadline )); do
    if leader="$(current_leader 2>/dev/null)" && [[ -n "$leader" && "$leader" != "$previous" ]]; then
      printf '%s' "$leader"
      return 0
    fi
    sleep 2
  done
  log "timed out waiting for unique leader previous=$previous"
  for file in "$REPORT_DIR"/*-server.log; do echo "--- $file" >&2; tail -n 160 "$file" >&2 || true; done
  return 1
}

leader_index() {
  local leader="$1"
  for ((i=0; i<NODE_COUNT; i++)); do
    if [[ "$leader" == "$RUN_ID-$i" ]]; then printf '%s' "$i"; return 0; fi
  done
  return 1
}

start_worker() {
  need_cmd bun
  : > "$REPORT_DIR/worker.log"
  (
    cd "$ROOT_DIR/examples/nodejs/worker-demo"
    if [[ ! -d node_modules ]]; then bun install --frozen-lockfile >>"$REPORT_DIR/worker.log" 2>&1; fi
    TIKEO_WORKER_ENDPOINT="http://127.0.0.1:${TUNNEL_PROXY_PORT}" \
    TIKEO_WORKER_CONNECT=1 \
    TIKEO_WORKER_NAMESPACE="$NAMESPACE" \
    TIKEO_WORKER_APP="$APP" \
    TIKEO_WORKER_POOL="$WORKER_POOL" \
    TIKEO_WORKER_CLUSTER=raft-e2e \
    TIKEO_WORKER_REGION=local \
    TIKEO_WORKER_CLIENT_INSTANCE_ID="$CLIENT_INSTANCE_ID" \
    TIKEO_WORKER_SDK_PROCESSORS=demo.echo \
    TIKEO_ENABLE_PLUGIN_SQL=0 \
    TIKEO_SANDBOX_AUTO_INSTALL=0 \
    exec bun start >>"$REPORT_DIR/worker.log" 2>&1
  ) &
  WORKER_PID=$!
}

leader_api() {
  local leader="$1"
  local method="$2"
  local path="$3"
  local body="${4:-}"
  local idx
  idx="$(leader_index "$leader")"
  local url="http://127.0.0.1:${HTTP_PORTS[$idx]}${path}"
  if [[ -n "$body" ]]; then
    curl -fsS -X "$method" "$url" -H "authorization: Bearer $TIKEO_SMOKE_AUTH_TOKEN" -H 'content-type: application/json' -d "$body"
  else
    curl -fsS -X "$method" "$url" -H "authorization: Bearer $TIKEO_SMOKE_AUTH_TOKEN"
  fi
}

wait_worker_on_leader() {
  local leader="$1"
  local output="$REPORT_DIR/workers-$leader.json"
  local deadline=$((SECONDS + 180))
  until leader_api "$leader" GET /api/v1/workers > "$output" && python3 - "$output" "$CLIENT_INSTANCE_ID" "$NAMESPACE" "$APP" >/dev/null <<'PY'
import json, sys
path, client_id, namespace, app = sys.argv[1:5]
payload=json.load(open(path, encoding='utf-8'))
items=(payload.get('data') or {}).get('items', [])
for item in items:
    if item.get('clientInstanceId') == client_id and item.get('status') == 'online':
        if item.get('namespace') != namespace or item.get('app') != app:
            raise SystemExit(f"scope mismatch: {item}")
        caps=item.get('structuredCapabilities') or {}
        if 'demo.echo' not in (caps.get('sdkProcessors') or []):
            raise SystemExit(f"missing demo.echo capability: {caps}")
        raise SystemExit(0)
raise SystemExit(1)
PY
  do
    if (( SECONDS >= deadline )); then
      log "timed out waiting for worker on $leader"
      cat "$output" >&2 || true
      tail -n 160 "$REPORT_DIR/worker.log" >&2 || true
      return 1
    fi
    sleep 2
  done
}

seed_scope() {
  api POST /api/v1/namespaces "$(tikeo_smoke_json_object name "$NAMESPACE")" >/dev/null
  api POST /api/v1/apps "$(python3 - "$NAMESPACE" "$APP" <<'PY'
import json, sys
print(json.dumps({'namespace': sys.argv[1], 'name': sys.argv[2]}, separators=(',', ':')))
PY
)" >/dev/null
  api POST /api/v1/worker-pools "$(python3 - "$NAMESPACE" "$APP" "$WORKER_POOL" <<'PY'
import json, sys
print(json.dumps({'namespace': sys.argv[1], 'app': sys.argv[2], 'name': sys.argv[3]}, separators=(',', ':')))
PY
)" >/dev/null
}

create_and_trigger_job() {
  local suffix="$1"
  local job_file="$REPORT_DIR/job-$suffix.json"
  local instance_file="$REPORT_DIR/instance-$suffix.json"
  api POST /api/v1/jobs "$(python3 - "$NAMESPACE" "$APP" "$RUN_ID-$suffix" <<'PY'
import json, sys
namespace, app, name = sys.argv[1:4]
print(json.dumps({
  'namespace': namespace,
  'app': app,
  'name': name,
  'scheduleType': 'api',
  'processorName': 'demo.echo',
  'enabled': True,
  'retryPolicy': {'enabled': True, 'maxAttempts': 3, 'initialDelaySeconds': 1, 'backoffMultiplier': 1, 'maxDelaySeconds': 5},
}, separators=(',', ':')))
PY
)" > "$job_file"
  local job_id
  job_id="$(json_get_file "$job_file" data.id)"
  api POST "/api/v1/jobs/${job_id}:trigger" '{"triggerType":"api","executionMode":"single"}' > "$instance_file"
  json_get_file "$instance_file" data.id
}

assert_instance_succeeded() {
  local suffix="$1"
  local instance_id="$2"
  local instance_file="$REPORT_DIR/instance-result-$suffix.json"
  local logs_file="$REPORT_DIR/instance-logs-$suffix.json"
  tikeo_smoke_wait_instance_status "http://127.0.0.1:$API_PROXY_PORT" "$instance_id" succeeded "$instance_file" 180
  api GET "/api/v1/instances/${instance_id}" > "$instance_file"
  api GET "/api/v1/instances/${instance_id}/logs" > "$logs_file"
  python3 - "$instance_file" "$logs_file" <<'PY'
import json, sys
instance=json.load(open(sys.argv[1], encoding='utf-8'))['data']
logs=json.load(open(sys.argv[2], encoding='utf-8'))['data']['items']
if instance.get('status') != 'succeeded':
    raise SystemExit(instance)
if (instance.get('result') or {}).get('message') != 'nodejs demo echo processed':
    raise SystemExit(f"unexpected result: {instance.get('result')}")
if 'nodejs demo echo processed' not in '\n'.join(str(item.get('message', '')) for item in logs):
    raise SystemExit(f"missing worker log: {logs}")
PY
}

main() {
  need_cmd docker
  need_cmd curl
  need_cmd python3
  build_server_binary

  POSTGRES_PORT="$(free_port)"
  API_PROXY_PORT="$(free_port)"
  TUNNEL_PROXY_PORT="$(free_port)"
  HTTP_PORTS=("$(free_port)" "$(free_port)" "$(free_port)")
  TUNNEL_PORTS=("$(free_port)" "$(free_port)" "$(free_port)")

  log "starting postgres on 127.0.0.1:$POSTGRES_PORT"
  docker rm -f "$POSTGRES_NAME" >/dev/null 2>&1 || true
  docker run -d --name "$POSTGRES_NAME" -p "127.0.0.1:${POSTGRES_PORT}:5432" \
    -e POSTGRES_USER=tikeo -e POSTGRES_PASSWORD=tikeo -e POSTGRES_DB=tikeo \
    "$POSTGRES_IMAGE" >/dev/null
  wait_postgres

  log "starting first server for migrations"
  start_node 0
  wait_for_http "$RUN_ID-0" "http://127.0.0.1:${HTTP_PORTS[0]}/healthz" 180
  log "starting remaining raft servers"
  start_node 1
  start_node 2
  wait_for_http "$RUN_ID-1" "http://127.0.0.1:${HTTP_PORTS[1]}/healthz" 180
  wait_for_http "$RUN_ID-2" "http://127.0.0.1:${HTTP_PORTS[2]}/healthz" 180

  start_proxy
  wait_for_http api-proxy "http://127.0.0.1:${API_PROXY_PORT}/readyz" 60
  tikeo_smoke_login "http://127.0.0.1:$API_PROXY_PORT"
  seed_scope

  local leader initial_instance failover_instance new_leader idx
  leader="$(wait_for_unique_leader)"
  log "initial leader: $leader"
  record raft-initial-leader passed "$REPORT_DIR/$leader.cluster.json" "initial raft leader elected: $leader"

  start_worker
  wait_worker_on_leader "$leader"
  record worker-initial-leader-registration passed "$REPORT_DIR/workers-$leader.json" "worker registered on initial raft leader"
  db_evidence="$(collect_fsod_db_evidence initial-registration)"
  assert_fsod_db_evidence initial-registration "$db_evidence"
  record fsod-db-initial-registration passed "$db_evidence" "captured durable worker session and shard ownership state before dispatch"

  initial_instance="$(create_and_trigger_job before-failover)"
  assert_instance_succeeded before-failover "$initial_instance"
  record pre-failover-dispatch passed "$REPORT_DIR/instance-result-before-failover.json" "job dispatched through initial leader"
  db_evidence="$(collect_fsod_db_evidence before-failover)"
  assert_fsod_db_evidence before-failover "$db_evidence"
  record fsod-db-before-failover passed "$db_evidence" "captured durable outbox, dispatch queue, shard ownership, and worker session before leader kill"
  collect_api_evidence before-failover

  idx="$(leader_index "$leader")"
  log "killing initial leader $leader pid=${SERVER_PIDS[$idx]}"
  kill "${SERVER_PIDS[$idx]}" >/dev/null 2>&1 || true
  wait "${SERVER_PIDS[$idx]}" 2>/dev/null || true
  SERVER_PIDS[$idx]=""

  new_leader="$(wait_for_unique_leader "$leader")"
  log "new leader: $new_leader"
  record raft-failover-leader passed "$REPORT_DIR/$new_leader.cluster.json" "new raft leader elected after killing $leader"

  wait_worker_on_leader "$new_leader"
  record worker-post-failover-registration passed "$REPORT_DIR/workers-$new_leader.json" "worker reconnected to new raft leader"
  db_evidence="$(collect_fsod_db_evidence post-leader-election)"
  assert_fsod_db_evidence post-leader-election "$db_evidence"
  record fsod-db-post-leader-election passed "$db_evidence" "captured shard ownership and worker session after new leader election"

  failover_instance="$(create_and_trigger_job after-failover)"
  assert_instance_succeeded after-failover "$failover_instance"
  record post-failover-dispatch passed "$REPORT_DIR/instance-result-after-failover.json" "job dispatched successfully after leader failover"
  db_evidence="$(collect_fsod_db_evidence after-failover)"
  assert_fsod_db_evidence after-failover "$db_evidence"
  record fsod-db-after-failover passed "$db_evidence" "captured durable outbox, dispatch queue, shard ownership, and worker session after failover dispatch"
  collect_api_evidence after-failover

  tikeo_smoke_finalize_report "$REPORT_JSON" passed >/dev/null
  log "PASS: raft worker failover e2e succeeded"
  log "report: $REPORT_JSON"
}

main "$@"
