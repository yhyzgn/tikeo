#!/usr/bin/env bash
set -euo pipefail

# Docker bridge smoke for scheduler raft-rs HTTP transport.
# This intentionally does NOT use --network host. It validates container-DNS peer
# endpoints, the internal x-scheduler-raft-token path, and safe follower/runtime
# inbox behavior across Docker bridge networking; if a leader is elected, it must be unique and fenced.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NETWORK="${SCHEDULER_RAFT_E2E_NETWORK:-scheduler-raft-e2e}"
IMAGE="${SCHEDULER_RAFT_E2E_IMAGE:-scheduler:raft-bridge-e2e}"
NODE_COUNT="${SCHEDULER_RAFT_E2E_NODES:-3}"
TOKEN="${SCHEDULER_RAFT_E2E_TOKEN:-}"
TMP_DIR="${SCHEDULER_RAFT_E2E_TMP:-}"
KEEP="${SCHEDULER_RAFT_E2E_KEEP:-0}"
HTTP_PORT=9090
TUNNEL_PORT=9998

if ! command -v docker >/dev/null 2>&1; then
  echo "[raft-e2e] docker CLI not found; install Docker to run bridge E2E" >&2
  exit 127
fi
if ! docker info >/dev/null 2>&1; then
  echo "[raft-e2e] docker daemon unavailable; start Docker to run bridge E2E" >&2
  exit 125
fi
if [[ ! "$NODE_COUNT" =~ ^[0-9]+$ ]] || (( NODE_COUNT < 2 )); then
  echo "[raft-e2e] SCHEDULER_RAFT_E2E_NODES must be an integer >= 2" >&2
  exit 2
fi
if [[ -z "$TOKEN" ]]; then
  TOKEN="dev-raft-$(od -An -N12 -tx1 /dev/urandom | tr -d ' \n')"
fi
if [[ -z "$TMP_DIR" ]]; then
  TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/scheduler-raft-e2e.XXXXXX")"
else
  mkdir -p "$TMP_DIR"
fi

log() { printf '[raft-e2e] %s\n' "$*"; }

cleanup() {
  local code=$?
  if [[ "$KEEP" == "1" ]]; then
    log "keeping containers/network/tmp for inspection: network=$NETWORK tmp=$TMP_DIR"
    exit "$code"
  fi
  for ((i=0; i<NODE_COUNT; i++)); do
    docker rm -f "scheduler-$i" >/dev/null 2>&1 || true
  done
  docker network rm "$NETWORK" >/dev/null 2>&1 || true
  rm -rf "$TMP_DIR"
  exit "$code"
}
trap cleanup EXIT INT TERM

log "building image $IMAGE"
docker build -t "$IMAGE" "$ROOT_DIR"

log "creating bridge network $NETWORK"
docker network rm "$NETWORK" >/dev/null 2>&1 || true
docker network create --driver bridge "$NETWORK" >/dev/null

peers_toml() {
  for ((i=0; i<NODE_COUNT; i++)); do
    printf '  { node_id = "scheduler-%d", endpoint = "http://scheduler-%d:%d" }' "$i" "$i" "$HTTP_PORT"
    if (( i + 1 < NODE_COUNT )); then
      printf ',\n'
    else
      printf '\n'
    fi
  done
}

for ((i=0; i<NODE_COUNT; i++)); do
  node="scheduler-$i"
  node_dir="$TMP_DIR/$node"
  mkdir -p "$node_dir/data" "$node_dir/config"
  cat > "$node_dir/config/raft-e2e.toml" <<CONFIG
[server]
listen_addr = "0.0.0.0:${HTTP_PORT}"
worker_tunnel_addr = "0.0.0.0:${TUNNEL_PORT}"

[storage]
database_url = "sqlite:///data/scheduler.db?mode=rwc"

[cluster]
mode = "raft"
node_id = "${node}"
peers = [
$(peers_toml)
]
CONFIG
  log "starting $node on bridge network (API :$HTTP_PORT, tunnel :$TUNNEL_PORT)"
  docker rm -f "$node" >/dev/null 2>&1 || true
  docker run -d \
    --name "$node" \
    --network "$NETWORK" \
    -e SCHEDULER__CLUSTER__TRANSPORT_TOKEN="$TOKEN" \
    -v "$node_dir/data:/data" \
    -v "$node_dir/config/raft-e2e.toml:/app/config/raft-e2e.toml:ro" \
    "$IMAGE" serve --config /app/config/raft-e2e.toml >/dev/null
done

curl_in_bridge() {
  docker run --rm --network "$NETWORK" curlimages/curl:8.10.1 -fsS "$@"
}

post_in_bridge() {
  docker run --rm --network "$NETWORK" curlimages/curl:8.10.1 -fsS \
    -H 'content-type: application/json' \
    -H "x-scheduler-raft-token: $TOKEN" \
    "$@"
}

json_expect() {
  local json="$1"
  local path="$2"
  local expected="$3"
  JSON_INPUT="$json" JSON_PATH="$path" JSON_EXPECTED="$expected" python3 - <<'PYJSON'
import json, os, sys
value = json.loads(os.environ["JSON_INPUT"])
for part in os.environ["JSON_PATH"].split('.'):
    value = value[part]
expected_raw = os.environ["JSON_EXPECTED"]
try:
    expected = json.loads(expected_raw)
except json.JSONDecodeError:
    expected = expected_raw
if value != expected:
    print(f"JSON assertion failed: {os.environ['JSON_PATH']} expected {expected!r}, got {value!r}", file=sys.stderr)
    print(os.environ["JSON_INPUT"], file=sys.stderr)
    sys.exit(1)
PYJSON
}

wait_for_node() {
  local node="$1"
  for _ in $(seq 1 60); do
    if curl_in_bridge "http://${node}:${HTTP_PORT}/healthz" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  log "logs for $node after health timeout:"
  docker logs "$node" || true
  return 1
}

for ((i=0; i<NODE_COUNT; i++)); do
  wait_for_node "scheduler-$i"
done

for ((i=0; i<NODE_COUNT; i++)); do
  node="scheduler-$i"
  log "checking $node /api/v1/cluster"
  cluster_json="$(curl_in_bridge "http://${node}:${HTTP_PORT}/api/v1/cluster")"
  json_expect "$cluster_json" "code" "0"
  json_expect "$cluster_json" "data.mode" "raft"
  printf '%s' "$cluster_json" > "$TMP_DIR/${node}.cluster.json"

  log "checking $node /api/v1/cluster/diagnostics"
  diag_json="$(curl_in_bridge "http://${node}:${HTTP_PORT}/api/v1/cluster/diagnostics")"
  json_expect "$diag_json" "code" "0"
  json_expect "$diag_json" "data.transport.status" "runtime_inbox_enabled"
done


log "checking cluster has at most one schedulable fenced leader"
TMP_DIR="$TMP_DIR" NODE_COUNT="$NODE_COUNT" python3 - <<'PYLEADER'
import json, os, pathlib, sys
root = pathlib.Path(os.environ["TMP_DIR"])
leaders = []
for i in range(int(os.environ["NODE_COUNT"])):
    data = json.loads((root / f"scheduler-{i}.cluster.json").read_text())["data"]
    if data["can_schedule"]:
        if data["role"] != "leader" or not data.get("leader_fencing_token"):
            print(f"schedulable node lacks leader role/fencing: {data}", file=sys.stderr)
            sys.exit(1)
        leaders.append(data["node_id"])
if len(leaders) > 1:
    print(f"multiple schedulable leaders observed: {leaders}", file=sys.stderr)
    sys.exit(1)
print(f"[raft-e2e] schedulable leaders observed: {leaders or 'none'}")
PYLEADER

append_body='{"from":1,"to":2,"term":1,"message_type":"MsgHeartbeat","index":0,"log_term":0,"commit":0,"snapshot_index":null,"snapshot_term":null,"entries":[],"context":null,"reject":false,"reject_hint":null,"leader_fencing_token":null}'
log "checking raft append-entries over bridge DNS with internal token"
append_json="$(post_in_bridge -X POST "http://scheduler-0:${HTTP_PORT}/api/v1/raft/append-entries" -d "$append_body")"
json_expect "$append_json" "code" "0"
json_expect "$append_json" "data.accepted" "true"

log "checking wrong raft token is rejected without admin session"
wrong_status="$(docker run --rm --network "$NETWORK" curlimages/curl:8.10.1 -sS -o /tmp/raft-wrong.json -w '%{http_code}' \
  -H 'content-type: application/json' \
  -H 'x-scheduler-raft-token: wrong-token' \
  -X POST "http://scheduler-0:${HTTP_PORT}/api/v1/raft/append-entries" \
  -d "$append_body")"
if [[ "$wrong_status" != "401" ]]; then
  echo "[raft-e2e] expected wrong-token status 401, got $wrong_status" >&2
  exit 1
fi

log "PASS: bridge-network raft HTTP smoke succeeded without host networking"
