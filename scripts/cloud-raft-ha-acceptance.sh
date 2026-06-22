#!/usr/bin/env bash
# Production/staging-safe Raft FSOD cloud acceptance probe.
# Default mode is read-only: it checks public API/metrics/SSE/network shape and optional kubectl evidence.
set -euo pipefail

SERVER_URL="${TIKEO_CLOUD_HA_SERVER_URL:-${1:-}}"
API_KEY="${TIKEO_CLOUD_HA_API_KEY:-}"
NAMESPACE="${TIKEO_CLOUD_HA_NAMESPACE:-tikeo}"
EXPECTED_REPLICAS="${TIKEO_CLOUD_HA_EXPECTED_REPLICAS:-4}"
REPORT_DIR="${TIKEO_CLOUD_HA_REPORT_DIR:-.dev/reports/cloud-raft-ha-acceptance-$(date -u +%Y%m%dT%H%M%SZ)}"
WORKER_TUNNEL_HOST="${TIKEO_CLOUD_HA_WORKER_TUNNEL_HOST:-}"
WORKER_TUNNEL_PORT="${TIKEO_CLOUD_HA_WORKER_TUNNEL_PORT:-9998}"
SSE_PATH="${TIKEO_CLOUD_HA_SSE_PATH:-/api/v1/dispatch-queue/stream}"
STREAM_TOKEN="${TIKEO_STREAM_TOKEN:-}"
MUTATING="${TIKEO_CLOUD_HA_MUTATING:-0}"
TIMEOUT_SECONDS="${TIKEO_CLOUD_HA_TIMEOUT_SECONDS:-8}"
KUBECTL_CONTEXT="${TIKEO_CLOUD_HA_KUBECTL_CONTEXT:-}"
APP_LABEL_SELECTOR="${TIKEO_CLOUD_HA_APP_LABEL_SELECTOR:-app.kubernetes.io/name=tikeo}"

die() { echo "ERROR: $*" >&2; exit 1; }
log() { printf '[cloud-ha] %s\n' "$*" >&2; }
need() { command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"; }
json_get() {
  local file="$1" expr="$2"
  python3 - "$file" "$expr" <<'PY'
import json, sys
path, expr = sys.argv[1:3]
with open(path, encoding='utf-8') as f:
    data = json.load(f)
for part in expr.split('.'):
    if not part:
        continue
    if isinstance(data, dict):
        data = data.get(part)
    else:
        data = None
        break
print('' if data is None else data)
PY
}

[ -n "$SERVER_URL" ] || die "set TIKEO_CLOUD_HA_SERVER_URL or pass server URL as argv[1]"
SERVER_URL="${SERVER_URL%/}"
need curl
need python3
mkdir -p "$REPORT_DIR"

if [ "$MUTATING" != "0" ]; then
  die "mutating cloud HA drills are intentionally disabled in this script; run explicit chaos tooling after a written window/rollback is approved"
fi

AUTH_ARGS=()
if [ -n "$API_KEY" ]; then
  AUTH_ARGS=(-H "x-tikeo-api-key: $API_KEY")
fi

log "writing report bundle to $REPORT_DIR"
cat > "$REPORT_DIR/input.env" <<EOF_INPUT
SERVER_URL=$SERVER_URL
EXPECTED_REPLICAS=$EXPECTED_REPLICAS
NAMESPACE=$NAMESPACE
WORKER_TUNNEL_HOST=$WORKER_TUNNEL_HOST
WORKER_TUNNEL_PORT=$WORKER_TUNNEL_PORT
SSE_PATH=$SSE_PATH
MUTATING=$MUTATING
EOF_INPUT

curl_json() {
  local path="$1" out="$2"
  curl -fsS --max-time "$TIMEOUT_SECONDS" "${AUTH_ARGS[@]}" "$SERVER_URL$path" -o "$out"
}

log "probing /api/v1/system/info, /cluster, /cluster/diagnostics, /metrics/summary"
curl_json /api/v1/system/info "$REPORT_DIR/system-info.json"
curl_json /api/v1/cluster "$REPORT_DIR/cluster.json"
curl_json /api/v1/cluster/diagnostics "$REPORT_DIR/cluster-diagnostics.json"
curl_json /api/v1/metrics/summary "$REPORT_DIR/metrics-summary.json"

log "probing SSE endpoint headers"
SSE_URL="$SERVER_URL$SSE_PATH"
if [ -n "$STREAM_TOKEN" ]; then
  case "$SSE_URL" in
    *\?*) SSE_URL="$SSE_URL&token=$STREAM_TOKEN" ;;
    *) SSE_URL="$SSE_URL?token=$STREAM_TOKEN" ;;
  esac
fi
curl -fsS -N --max-time 3 "${AUTH_ARGS[@]}" -D "$REPORT_DIR/sse-headers.txt" "$SSE_URL" -o "$REPORT_DIR/sse-sample.txt" || true

if [ -n "$WORKER_TUNNEL_HOST" ]; then
  log "probing Worker Tunnel TCP reachability $WORKER_TUNNEL_HOST:$WORKER_TUNNEL_PORT"
  python3 - "$WORKER_TUNNEL_HOST" "$WORKER_TUNNEL_PORT" "$TIMEOUT_SECONDS" > "$REPORT_DIR/worker-tunnel-tcp.json" <<'PY'
import json, socket, sys, time
host, port, timeout = sys.argv[1], int(sys.argv[2]), float(sys.argv[3])
start = time.time()
result = {"host": host, "port": port, "reachable": False, "latencyMs": None, "error": None}
try:
    with socket.create_connection((host, port), timeout=timeout):
        result["reachable"] = True
        result["latencyMs"] = round((time.time() - start) * 1000, 2)
except Exception as exc:
    result["error"] = str(exc)
print(json.dumps(result, indent=2))
PY
fi

if command -v kubectl >/dev/null 2>&1; then
  KUBECTL=(kubectl)
  if [ -n "$KUBECTL_CONTEXT" ]; then
    KUBECTL+=(--context "$KUBECTL_CONTEXT")
  fi
  if "${KUBECTL[@]}" get ns "$NAMESPACE" >/dev/null 2>&1; then
    log "collecting optional kubectl evidence from namespace $NAMESPACE"
    "${KUBECTL[@]}" -n "$NAMESPACE" get pods -l "$APP_LABEL_SELECTOR" -o wide > "$REPORT_DIR/kubectl-pods.txt" || true
    "${KUBECTL[@]}" -n "$NAMESPACE" get statefulset,svc,ingress,httproute,grpcroute -o wide > "$REPORT_DIR/kubectl-networking.txt" 2>&1 || true
    "${KUBECTL[@]}" -n "$NAMESPACE" get events --sort-by=.lastTimestamp > "$REPORT_DIR/kubectl-events.txt" 2>&1 || true
  else
    echo "namespace $NAMESPACE not reachable in current kubectl context" > "$REPORT_DIR/kubectl-skipped.txt"
  fi
else
  echo "kubectl not installed; API-only acceptance completed" > "$REPORT_DIR/kubectl-skipped.txt"
fi

python3 - "$REPORT_DIR" "$EXPECTED_REPLICAS" <<'PY'
import json, re, sys
from pathlib import Path
report = Path(sys.argv[1])
expected = int(sys.argv[2])

def load(name):
    with (report / name).open(encoding='utf-8') as f:
        return json.load(f).get('data', json.load(f) if False else None)

def envelope(name):
    with (report / name).open(encoding='utf-8') as f:
        raw = json.load(f)
    return raw.get('data', raw)

cluster = envelope('cluster.json')
diag = envelope('cluster-diagnostics.json')
metrics = envelope('metrics-summary.json')
headers = (report / 'sse-headers.txt').read_text(encoding='utf-8', errors='ignore') if (report / 'sse-headers.txt').exists() else ''
sse_sample = (report / 'sse-sample.txt').read_text(encoding='utf-8', errors='ignore') if (report / 'sse-sample.txt').exists() else ''
node_count = len(diag.get('nodes') or [])
can_schedule_nodes = sum(1 for node in diag.get('nodes') or [] if node.get('observedCanSchedule') or node.get('canSchedule'))
responding = diag.get('respondingNode') or diag.get('status') or cluster
queue = metrics.get('queue') or {}
outbox = metrics.get('outbox') or {}
shards = metrics.get('shardOwnership') or {}
smart = diag.get('smartGateway') or {}
checks = []

def add(name, passed, detail, value=None):
    checks.append({'name': name, 'passed': bool(passed), 'detail': detail, 'value': value})

add('raft mode enabled', responding.get('mode') == 'raft', f"mode={responding.get('mode')}")
add('expected diagnostics nodes', node_count >= expected, f"nodes={node_count}, expected>={expected}", node_count)
add('single schedulable authority visible', can_schedule_nodes <= 1, f"canSchedule nodes={can_schedule_nodes}", can_schedule_nodes)
add('leader fencing token present when schedulable', bool(responding.get('leaderFencingToken')) or not responding.get('canSchedule'), 'leader fencing token is required on scheduling leader')
add('shard ownership active', (shards.get('active') or 0) > 0, f"active={shards.get('active')}", shards.get('active'))
add('shard ownership skew acceptable', (shards.get('ownershipSkew') or 0) <= max(1, (shards.get('maxActiveShardsPerOwner') or 1)), f"skew={shards.get('ownershipSkew')}", shards.get('ownershipSkew'))
add('outbox no stale queued backlog', (outbox.get('oldestQueuedAgeSeconds') or 0) <= 300, f"oldestQueuedAgeSeconds={outbox.get('oldestQueuedAgeSeconds')}", outbox.get('oldestQueuedAgeSeconds'))
add('dispatch queue no stale pending backlog', (queue.get('oldestPendingAgeSeconds') or 0) <= 300, f"oldestPendingAgeSeconds={queue.get('oldestPendingAgeSeconds')}", queue.get('oldestPendingAgeSeconds'))
add('smart gateway diagnostic present', smart.get('mode') == 'diagnostic_safe_optimization', f"mode={smart.get('mode')}")
add('SSE endpoint exposes event stream or auth gate', ('text/event-stream' in headers.lower()) or ('401' in headers or '403' in headers), 'SSE should stream or fail with auth gate, not buffer/HTML')
if (report / 'worker-tunnel-tcp.json').exists():
    wt = json.loads((report / 'worker-tunnel-tcp.json').read_text())
    add('Worker Tunnel TCP reachable', wt.get('reachable') is True, wt.get('error') or f"latencyMs={wt.get('latencyMs')}", wt.get('latencyMs'))
score = round(sum(1 for c in checks if c['passed']) / len(checks) * 100, 2) if checks else 0
summary = {'verdict': 'passed' if score >= 90 and all(c['passed'] for c in checks[:4]) else 'review', 'score': score, 'checks': checks}
(report / 'summary.json').write_text(json.dumps(summary, indent=2, ensure_ascii=False), encoding='utf-8')
md = ['# Cloud Raft FSOD acceptance report', '', f"- Verdict: `{summary['verdict']}`", f"- Score: `{score}`", f"- Expected replicas: `{expected}`", '', '| Check | Pass | Detail |', '|---|---:|---|']
for c in checks:
    md.append(f"| {c['name']} | {'✅' if c['passed'] else '❌'} | {c['detail']} |")
md += ['', '## Operator notes', '', '- This script is read-only by default and does not kill pods or import jobs.', '- Use the Kind harness for destructive local chaos drills; use a separately approved cloud chaos window for production-like pod/node/LB failures.', '- Attach this directory as release evidence together with ingress/LB/WAF timeout settings and external database HA evidence.']
(report / 'REPORT.md').write_text('\n'.join(md) + '\n', encoding='utf-8')
print(json.dumps(summary, indent=2, ensure_ascii=False))
PY

log "acceptance report: $REPORT_DIR/REPORT.md"
