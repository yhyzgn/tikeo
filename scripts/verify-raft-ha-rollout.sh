#!/usr/bin/env bash
set -euo pipefail

SERVER_URL="${TIKEO_SERVER_URL:-}"
API_KEY="${TIKEO_MANAGEMENT_API_KEY:-}"
EXPECTED_REPLICAS="${TIKEO_EXPECTED_SERVER_REPLICAS:-}"
MAX_SHARD_SKEW="${TIKEO_MAX_SHARD_SKEW:-1}"
MAX_PENDING_AGE_SECONDS="${TIKEO_MAX_PENDING_AGE_SECONDS:-0}"
MAX_OUTBOX_AGE_SECONDS="${TIKEO_MAX_OUTBOX_AGE_SECONDS:-0}"
OUTPUT_FILE="${TIKEO_ROLLOUT_REPORT:-}"

usage() {
  cat >&2 <<'USAGE'
Usage:
  TIKEO_SERVER_URL=https://tikeo.example.com \
  TIKEO_MANAGEMENT_API_KEY=... \
  scripts/verify-raft-ha-rollout.sh

Optional checks:
  TIKEO_EXPECTED_SERVER_REPLICAS=3
  TIKEO_MAX_SHARD_SKEW=1
  TIKEO_MAX_PENDING_AGE_SECONDS=120
  TIKEO_MAX_OUTBOX_AGE_SECONDS=120
  TIKEO_ROLLOUT_REPORT=.dev/reports/raft-ha-rollout.json
USAGE
}

require() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 2
  }
}

if [[ -z "$SERVER_URL" || -z "$API_KEY" ]]; then
  usage
  exit 2
fi
require curl
require python3

SERVER_URL="${SERVER_URL%/}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

api_get() {
  local path="$1"
  curl -fsS "$SERVER_URL$path" \
    -H "x-tikeo-api-key: $API_KEY" \
    -H 'accept: application/json'
}

cluster_file="$TMP_DIR/cluster.json"
metrics_file="$TMP_DIR/metrics.json"
api_get /api/v1/cluster/diagnostics > "$cluster_file"
api_get /api/v1/metrics/summary > "$metrics_file"

python3 - "$cluster_file" "$metrics_file" "$EXPECTED_REPLICAS" "$MAX_SHARD_SKEW" "$MAX_PENDING_AGE_SECONDS" "$MAX_OUTBOX_AGE_SECONDS" "$SERVER_URL" <<'PY'
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

cluster_path, metrics_path, expected_replicas, max_skew, max_pending_age, max_outbox_age, server_url = sys.argv[1:]
cluster = json.loads(Path(cluster_path).read_text())
metrics = json.loads(Path(metrics_path).read_text())
errors = []
warnings = []

def data(payload, name):
    if payload.get("code") != 0:
        errors.append(f"{name} returned non-zero code: {payload.get('code')} {payload.get('message')}")
        return {}
    return payload.get("data") or {}

cluster_data = data(cluster, "cluster diagnostics")
metrics_data = data(metrics, "metrics summary")
nodes = cluster_data.get("nodes") or []
can_schedule = [node for node in nodes if (node.get("observedCanSchedule") if node.get("observedCanSchedule") is not None else node.get("canSchedule", node.get("can_schedule")))]
if len(can_schedule) != 1:
    errors.append(f"expected exactly one schedulable Raft control-plane node, got {len(can_schedule)}")
if expected_replicas:
    try:
        expected = int(expected_replicas)
        if len(nodes) != expected:
            errors.append(f"expected {expected} diagnostic nodes, got {len(nodes)}")
    except ValueError:
        errors.append(f"TIKEO_EXPECTED_SERVER_REPLICAS must be an integer, got {expected_replicas!r}")
for node in nodes:
    probe_status = node.get("probeStatus") or node.get("probe_status")
    if probe_status and probe_status not in {"ok", "local"}:
        errors.append(f"node {node.get('nodeId') or node.get('node_id')} cluster-status probe is {probe_status}: {node.get('probeError') or node.get('probe_error')}")

shard = metrics_data.get("shard_ownership") or metrics_data.get("shardOwnership") or {}
active = int(shard.get("active") or 0)
owner_count = int(shard.get("activeOwnerCount") or shard.get("active_owner_count") or 0)
skew = int(shard.get("ownershipSkew") or shard.get("ownership_skew") or 0)
try:
    skew_limit = int(max_skew)
except ValueError:
    errors.append(f"TIKEO_MAX_SHARD_SKEW must be an integer, got {max_skew!r}")
    skew_limit = 1
if active <= 0:
    errors.append(f"expected active shard ownership rows, got shardOwnership={shard}")
if owner_count <= 0:
    errors.append(f"expected at least one active shard owner, got shardOwnership={shard}")
if skew > skew_limit:
    errors.append(f"shard ownership skew {skew} exceeds limit {skew_limit}")

queue = metrics_data.get("queue") or {}
outbox = metrics_data.get("outbox") or {}
oldest_pending = int(queue.get("oldestPendingAgeSeconds") or queue.get("oldest_pending_age_seconds") or 0)
oldest_outbox = int(outbox.get("oldestQueuedAgeSeconds") or outbox.get("oldest_queued_age_seconds") or 0)
if int(max_pending_age or 0) > 0 and oldest_pending > int(max_pending_age):
    errors.append(f"oldest pending dispatch queue age {oldest_pending}s exceeds {max_pending_age}s")
if int(max_outbox_age or 0) > 0 and oldest_outbox > int(max_outbox_age):
    errors.append(f"oldest queued worker outbox age {oldest_outbox}s exceeds {max_outbox_age}s")

pending_by_owner = queue.get("pendingByShardOwner") or queue.get("pending_by_shard_owner") or {}
if active > 0 and not isinstance(pending_by_owner, dict):
    warnings.append("pendingByShardOwner missing or not an object; verify server version exposes owner-aware queue metrics")

report = {
    "ok": not errors,
    "checkedAt": datetime.now(timezone.utc).isoformat(),
    "serverUrl": server_url,
    "summary": {
        "nodeCount": len(nodes),
        "schedulableNodes": [node.get("nodeId") or node.get("node_id") for node in can_schedule],
        "probeStatuses": {node.get("nodeId") or node.get("node_id"): node.get("probeStatus") or node.get("probe_status") for node in nodes},
        "shardOwnership": {
            "active": active,
            "activeOwnerCount": owner_count,
            "ownershipSkew": skew,
            "activeByOwner": shard.get("activeByOwner") or shard.get("active_by_owner") or {},
        },
        "queue": {
            "pending": queue.get("pending"),
            "running": queue.get("running"),
            "oldestPendingAgeSeconds": oldest_pending,
            "pendingByShardOwner": pending_by_owner,
            "oldestPendingAgeByShardOwner": queue.get("oldestPendingAgeByShardOwner") or queue.get("oldest_pending_age_by_shard_owner") or {},
        },
        "outbox": {
            "total": outbox.get("total"),
            "oldestQueuedAgeSeconds": oldest_outbox,
            "byStatus": outbox.get("byStatus") or outbox.get("by_status") or {},
        },
    },
    "errors": errors,
    "warnings": warnings,
}
print(json.dumps(report, ensure_ascii=False, indent=2))
if errors:
    raise SystemExit(1)
PY

if [[ -n "$OUTPUT_FILE" ]]; then
  mkdir -p "$(dirname "$OUTPUT_FILE")"
  python3 - "$cluster_file" "$metrics_file" "$OUTPUT_FILE" <<'PY'
import json
import sys
from pathlib import Path
cluster = json.loads(Path(sys.argv[1]).read_text())
metrics = json.loads(Path(sys.argv[2]).read_text())
Path(sys.argv[3]).write_text(json.dumps({"clusterDiagnostics": cluster, "metricsSummary": metrics}, ensure_ascii=False, indent=2) + "\n")
PY
  echo "wrote raw rollout evidence: $OUTPUT_FILE" >&2
fi
