#!/usr/bin/env bash
set -euo pipefail

NAMESPACE="${TIKEO_K8S_NAMESPACE:-tikeo}"
APP_LABEL="${TIKEO_SERVER_LABEL_SELECTOR:-app.kubernetes.io/component=server}"
SERVER_URL="${TIKEO_SERVER_URL:-}"
API_KEY="${TIKEO_MANAGEMENT_API_KEY:-}"
MODE="${TIKEO_FAULT_MODE:-dry-run}"
FAULT="${TIKEO_FAULT:-leader-pod-delete}"
REPORT_DIR="${TIKEO_FAULT_REPORT_DIR:-.dev/reports/raft-ha-fault-$(date -u +%Y%m%dT%H%M%SZ)}"
VERIFY_SCRIPT="${TIKEO_VERIFY_RAFT_HA_SCRIPT:-scripts/verify-raft-ha-rollout.sh}"
RECOVERY_TIMEOUT="${TIKEO_RECOVERY_TIMEOUT_SECONDS:-180}"

usage() {
  cat >&2 <<'USAGE'
Usage:
  TIKEO_SERVER_URL=https://tikeo.example.com \
  TIKEO_MANAGEMENT_API_KEY=... \
  TIKEO_FAULT_MODE=apply \
  scripts/raft-ha-fault-injection-drill.sh

Defaults to dry-run. Supported faults:
  leader-pod-delete       Delete the currently observed schedulable Server pod.
  random-server-pod-delete Delete one Server pod selected from the label selector.

Environment:
  TIKEO_K8S_NAMESPACE=tikeo
  TIKEO_SERVER_LABEL_SELECTOR=app.kubernetes.io/component=server
  TIKEO_EXPECTED_SERVER_REPLICAS=3
  TIKEO_MAX_SHARD_SKEW=1
  TIKEO_MAX_PENDING_AGE_SECONDS=120
  TIKEO_MAX_OUTBOX_AGE_SECONDS=120
  TIKEO_FAULT_REPORT_DIR=.dev/reports/...
  TIKEO_RECOVERY_TIMEOUT_SECONDS=180
USAGE
}

require() { command -v "$1" >/dev/null 2>&1 || { echo "missing required command: $1" >&2; exit 2; }; }
require kubectl
require jq
require curl
require python3

if [[ -z "$SERVER_URL" || -z "$API_KEY" ]]; then
  usage
  exit 2
fi
if [[ "$MODE" != "dry-run" && "$MODE" != "apply" ]]; then
  echo "TIKEO_FAULT_MODE must be dry-run or apply" >&2
  exit 2
fi
case "$FAULT" in
  leader-pod-delete|random-server-pod-delete) ;;
  *) echo "unsupported TIKEO_FAULT=$FAULT" >&2; usage; exit 2 ;;
esac

mkdir -p "$REPORT_DIR"
export TIKEO_ROLLOUT_REPORT="$REPORT_DIR/precheck-raw.json"
"$VERIFY_SCRIPT" | tee "$REPORT_DIR/precheck.json"

cluster_json="$REPORT_DIR/cluster-diagnostics-before.json"
curl -fsS "${SERVER_URL%/}/api/v1/cluster/diagnostics" \
  -H "x-tikeo-api-key: $API_KEY" \
  -H 'accept: application/json' > "$cluster_json"
python3 -m json.tool "$cluster_json" > "$cluster_json.tmp" && mv "$cluster_json.tmp" "$cluster_json"

if [[ "$FAULT" == "leader-pod-delete" ]]; then
  target_node="$(jq -r '.data.nodes[] | select((.observedCanSchedule // .canSchedule // false) == true) | .nodeId' "$cluster_json" | head -1)"
  if [[ -z "$target_node" || "$target_node" == "null" ]]; then
    echo "could not determine schedulable leader pod from diagnostics" >&2
    exit 1
  fi
  target_pod="$target_node"
else
  target_pod="$(kubectl -n "$NAMESPACE" get pod -l "$APP_LABEL" -o jsonpath='{.items[0].metadata.name}')"
fi
if [[ -z "$target_pod" ]]; then
  echo "could not select target pod" >&2
  exit 1
fi

echo "selected fault=$FAULT targetPod=$target_pod mode=$MODE" | tee "$REPORT_DIR/fault-selection.txt"
if [[ "$MODE" == "dry-run" ]]; then
  cat > "$REPORT_DIR/dry-run-plan.json" <<JSON
{
  "mode": "dry-run",
  "fault": "$FAULT",
  "namespace": "$NAMESPACE",
  "targetPod": "$target_pod",
  "nextApplyCommand": "TIKEO_FAULT_MODE=apply TIKEO_FAULT=$FAULT scripts/raft-ha-fault-injection-drill.sh"
}
JSON
  echo "dry-run complete; no Kubernetes resources were mutated"
  exit 0
fi

kubectl -n "$NAMESPACE" get pod "$target_pod" -o yaml > "$REPORT_DIR/target-pod-before.yaml"
kubectl -n "$NAMESPACE" delete pod "$target_pod" --wait=false | tee "$REPORT_DIR/kubectl-delete-pod.log"
kubectl -n "$NAMESPACE" rollout status statefulset/tikeo-server --timeout="${RECOVERY_TIMEOUT}s" | tee "$REPORT_DIR/rollout-status.log" || true

deadline=$((SECONDS + RECOVERY_TIMEOUT))
last_status=1
while (( SECONDS < deadline )); do
  export TIKEO_ROLLOUT_REPORT="$REPORT_DIR/postcheck-raw.json"
  if "$VERIFY_SCRIPT" > "$REPORT_DIR/postcheck.json" 2> "$REPORT_DIR/postcheck.stderr"; then
    last_status=0
    break
  fi
  sleep 5
done
if (( last_status != 0 )); then
  echo "fault drill failed to recover within ${RECOVERY_TIMEOUT}s; see $REPORT_DIR/postcheck.stderr" >&2
  exit 1
fi

echo "PASS: fault drill recovered; evidence=$REPORT_DIR"
