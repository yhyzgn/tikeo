#!/usr/bin/env bash
set -euo pipefail

# Kind-backed 4-Pod Raft HA + Worker Tunnel E2E.
#
# Validates on one developer machine:
#   1. four Kubernetes Server pods form one Raft scheduling control plane;
#   2. multi-node Kind + required pod anti-affinity spreads Server pods across
#      separate Kind worker nodes to approximate production failure domains;
#   3. cluster diagnostics can probe every pod through stable headless DNS;
#   4. API requests can enter one pod while the Worker Tunnel long connection is
#      pinned to another pod;
#   5. jobs dispatch before and after deleting the current schedulable leader;
#   6. hard-killing the Worker gateway pod forces Worker reconnect and durable
#      outbox reroute to the new gateway;
#   7. durable shard ownership / worker outbox evidence is persisted in Postgres.
#
# The script is intentionally self-contained: if kind/kubectl are not installed
# globally, it downloads known-good local binaries under .dev/tools/bin.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="${TIKEO_KIND_E2E_RUN_ID:-kind-raft-ha-e2e-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
REPORT_DIR="${TIKEO_KIND_E2E_REPORT_DIR:-$ROOT_DIR/.dev/reports/$RUN_ID}"
if [[ "$REPORT_DIR" != /* ]]; then
  REPORT_DIR="$ROOT_DIR/$REPORT_DIR"
fi
TOOLS_DIR="${TIKEO_KIND_E2E_TOOLS_DIR:-$ROOT_DIR/.dev/tools/bin}"
CLUSTER_NAME="${TIKEO_KIND_CLUSTER_NAME:-tikeo-raft-ha}"
NAMESPACE="${TIKEO_KIND_NAMESPACE:-tikeo-kind-ha}"
SERVER_REPLICAS="${TIKEO_KIND_SERVER_REPLICAS:-4}"
POSTGRES_IMAGE="${TIKEO_KIND_POSTGRES_IMAGE:-postgres:16-alpine}"
SERVER_IMAGE="${TIKEO_KIND_SERVER_IMAGE:-tikeo-server:kind-e2e-$RUN_ID}"
KEEP="${TIKEO_KIND_E2E_KEEP:-0}"
REBUILD_SERVER="${TIKEO_KIND_E2E_REBUILD_SERVER:-1}"
INSTALL_TOOLS="${TIKEO_KIND_E2E_INSTALL_TOOLS:-1}"
KIND_VERSION="${TIKEO_KIND_VERSION:-v0.29.0}"
KUBECTL_VERSION="${TIKEO_KUBECTL_VERSION:-}"
KIND_NODE_IMAGE="${TIKEO_KIND_NODE_IMAGE:-kindest/node:v1.33.1}"
KIND_WORKER_NODES="${TIKEO_KIND_WORKER_NODES:-4}"
ENABLE_POD_ANTI_AFFINITY="${TIKEO_KIND_ENABLE_POD_ANTI_AFFINITY:-1}"
LOAD_POSTGRES_IMAGE="${TIKEO_KIND_LOAD_POSTGRES_IMAGE:-1}"
TOKEN="${TIKEO_KIND_RAFT_TOKEN:-kind-raft-$(od -An -N12 -tx1 /dev/urandom | tr -d ' \n')}"
POSTGRES_PASSWORD="${TIKEO_KIND_POSTGRES_PASSWORD:-tikeo}"
API_PORT="${TIKEO_KIND_API_PORT:-}"
TUNNEL_PORT="${TIKEO_KIND_TUNNEL_PORT:-}"
NAMESPACE_NAME="${TIKEO_KIND_E2E_NAMESPACE_NAME:-kind-e2e}"
APP_NAME="${TIKEO_KIND_E2E_APP:-failover}"
WORKER_POOL="${TIKEO_KIND_E2E_WORKER_POOL:-nodejs-blue}"
CLIENT_INSTANCE_ID="${TIKEO_KIND_E2E_CLIENT_INSTANCE_ID:-nodejs-kind-worker}"
SERVER_BIN="$ROOT_DIR/target/debug/tikeo"
CASES_FILE="$REPORT_DIR/$RUN_ID-cases.jsonl"
REPORT_JSON="$REPORT_DIR/$RUN_ID.json"
SUMMARY_JSON="$REPORT_DIR/$RUN_ID-summary.json"

mkdir -p "$REPORT_DIR" "$TOOLS_DIR"
: > "$CASES_FILE"
export TIKEO_SMOKE_REPORT_DIR="$REPORT_DIR"
export TIKEO_SMOKE_RUN_ID="$RUN_ID"
export TIKEO_SMOKE_CASES_FILE="$CASES_FILE"
# shellcheck source=../deploy/smoke/lib/tikeo-smoke-lib.sh
source "$ROOT_DIR/deploy/smoke/lib/tikeo-smoke-lib.sh"

KIND_BIN=""
KUBECTL_BIN=""
API_PF_PID=""
TUNNEL_PF_PID=""
BOOTSTRAP_PF_PID=""
WORKER_PID=""
API_POD=""
GATEWAY_POD=""
API_KEY=""
LEADER_BEFORE=""
LEADER_AFTER=""
GATEWAY_POD_BEFORE=""
GATEWAY_POD_AFTER=""

log() { printf '[kind-raft-ha-e2e] %s\n' "$*"; }
record() { tikeo_smoke_record_case "$1" "$2" "${3:-}" "${4:-}"; }

free_port() {
  python3 - <<'PY'
import socket
s=socket.socket()
s.bind(('127.0.0.1', 0))
print(s.getsockname()[1])
s.close()
PY
}

need_cmd_or_installable() {
  local cmd="$1"
  local resolved
  resolved="$(type -P "$cmd" || true)"
  if [[ -n "$resolved" ]]; then
    printf '%s\n' "$resolved"
    return 0
  fi
  local candidate="$TOOLS_DIR/$cmd"
  if [[ -x "$candidate" ]]; then
    printf '%s\n' "$candidate"
    return 0
  fi
  return 1
}

os_arch() {
  local os arch
  os="$(uname | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64) arch=amd64 ;;
    aarch64|arm64) arch=arm64 ;;
    *) echo "unsupported architecture: $arch" >&2; return 2 ;;
  esac
  printf '%s %s\n' "$os" "$arch"
}

install_kind() {
  local os arch
  read -r os arch < <(os_arch)
  log "installing kind $KIND_VERSION to $TOOLS_DIR/kind"
  if ! curl --connect-timeout 15 --max-time 180 --retry 2 --retry-delay 2 -fsSL "https://kind.sigs.k8s.io/dl/${KIND_VERSION}/kind-${os}-${arch}" -o "$TOOLS_DIR/kind"; then
    if type -P go >/dev/null 2>&1; then
      log "kind binary download failed; falling back to: go install sigs.k8s.io/kind@${KIND_VERSION}"
      GOBIN="$TOOLS_DIR" go install "sigs.k8s.io/kind@${KIND_VERSION}"
    else
      echo "failed to download kind and Go fallback is unavailable" >&2
      return 1
    fi
  fi
  chmod +x "$TOOLS_DIR/kind"
}

install_kubectl() {
  local os arch version
  read -r os arch < <(os_arch)
  version="$KUBECTL_VERSION"
  if [[ -z "$version" ]]; then
    version="$(curl --connect-timeout 15 --max-time 60 --retry 2 --retry-delay 2 -fsSL https://dl.k8s.io/release/stable.txt)"
  fi
  log "installing kubectl $version to $TOOLS_DIR/kubectl"
  curl --connect-timeout 15 --max-time 180 --retry 2 --retry-delay 2 -fsSL "https://dl.k8s.io/release/${version}/bin/${os}/${arch}/kubectl" -o "$TOOLS_DIR/kubectl"
  chmod +x "$TOOLS_DIR/kubectl"
}

resolve_tools() {
  type -P docker >/dev/null 2>&1 || { echo "missing required command: docker" >&2; exit 2; }
  type -P curl >/dev/null 2>&1 || { echo "missing required command: curl" >&2; exit 2; }
  type -P python3 >/dev/null 2>&1 || { echo "missing required command: python3" >&2; exit 2; }
  type -P jq >/dev/null 2>&1 || { echo "missing required command: jq" >&2; exit 2; }
  type -P bun >/dev/null 2>&1 || { echo "missing required command: bun" >&2; exit 2; }

  if ! KIND_BIN="$(need_cmd_or_installable kind)"; then
    [[ "$INSTALL_TOOLS" == "1" ]] || { echo "missing kind and TIKEO_KIND_E2E_INSTALL_TOOLS=0" >&2; exit 2; }
    install_kind
    KIND_BIN="$TOOLS_DIR/kind"
  fi
  if ! KUBECTL_BIN="$(need_cmd_or_installable kubectl)"; then
    [[ "$INSTALL_TOOLS" == "1" ]] || { echo "missing kubectl and TIKEO_KIND_E2E_INSTALL_TOOLS=0" >&2; exit 2; }
    install_kubectl
    KUBECTL_BIN="$TOOLS_DIR/kubectl"
  fi
  export PATH="$TOOLS_DIR:$PATH"
  log "using kind=$KIND_BIN kubectl=$KUBECTL_BIN"
}

cleanup() {
  local code=$?
  for pid in "$WORKER_PID" "$TUNNEL_PF_PID" "$API_PF_PID" "$BOOTSTRAP_PF_PID"; do
    if [[ -n "$pid" ]] && kill -0 "$pid" >/dev/null 2>&1; then
      kill "$pid" >/dev/null 2>&1 || true
      wait "$pid" 2>/dev/null || true
    fi
  done
  if [[ "$KEEP" == "1" ]]; then
    log "keeping Kind cluster '$CLUSTER_NAME' and reports for inspection: $REPORT_DIR"
  else
    if [[ -n "$KIND_BIN" ]] && "$KIND_BIN" get clusters 2>/dev/null | grep -Fxq "$CLUSTER_NAME"; then
      "$KIND_BIN" delete cluster --name "$CLUSTER_NAME" > "$REPORT_DIR/kind-delete-cluster.log" 2>&1 || true
    fi
  fi
  exit "$code"
}
trap cleanup EXIT INT TERM

kubectl() { "$KUBECTL_BIN" "$@"; }
kind() { "$KIND_BIN" "$@"; }

api_url() { printf 'http://127.0.0.1:%s' "$API_PORT"; }
api() { tikeo_smoke_api "$(api_url)" "$@"; }
api_key_request() {
  local method="$1" path="$2" body="${3:-}"
  if [[ -n "$body" ]]; then
    curl -fsS -X "$method" "$(api_url)$path" -H "x-tikeo-api-key: $API_KEY" -H 'content-type: application/json' -d "$body"
  else
    curl -fsS -X "$method" "$(api_url)$path" -H "x-tikeo-api-key: $API_KEY"
  fi
}
json_get_file() {
  python3 -c 'import json,sys
cur=json.load(open(sys.argv[1], encoding="utf-8"))
for part in sys.argv[2].split("."):
    cur = cur[int(part)] if isinstance(cur, list) else cur[part]
print(cur)' "$1" "$2"
}

create_kind_cluster() {
  if kind get clusters 2>/dev/null | grep -Fxq "$CLUSTER_NAME"; then
    log "reusing existing Kind cluster: $CLUSTER_NAME"
  else
    {
      cat <<'YAML'
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
nodes:
  - role: control-plane
YAML
      for _ in $(seq 1 "$KIND_WORKER_NODES"); do
        printf '  - role: worker\n'
      done
    } > "$REPORT_DIR/kind-config.yaml"
    kind create cluster --name "$CLUSTER_NAME" --image "$KIND_NODE_IMAGE" --config "$REPORT_DIR/kind-config.yaml" 2>&1 | tee "$REPORT_DIR/kind-create-cluster.log"
  fi
  kubectl cluster-info > "$REPORT_DIR/kubectl-cluster-info.txt"
  record kind-cluster-ready passed "$REPORT_DIR/kubectl-cluster-info.txt" "Kind cluster is reachable"
}

build_server_image() {
  if [[ "$REBUILD_SERVER" == "1" || ! -x "$SERVER_BIN" ]]; then
    log "building debug server binary"
    (cd "$ROOT_DIR" && cargo build --bin tikeo 2>&1 | tee "$REPORT_DIR/cargo-build.log")
  fi
  cat > "$REPORT_DIR/Dockerfile.tikeo-kind" <<'DOCKERFILE'
FROM ubuntu:24.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl libgcc-s1 \
    && rm -rf /var/lib/apt/lists/*
COPY tikeo /usr/local/bin/tikeo
EXPOSE 9090 9998
ENTRYPOINT ["/usr/local/bin/tikeo"]
DOCKERFILE
  cp "$SERVER_BIN" "$REPORT_DIR/tikeo"
  docker build --platform linux/amd64 --provenance=false -t "$SERVER_IMAGE" -f "$REPORT_DIR/Dockerfile.tikeo-kind" "$REPORT_DIR" 2>&1 | tee "$REPORT_DIR/docker-build-server.log"
  kind load docker-image "$SERVER_IMAGE" --name "$CLUSTER_NAME" 2>&1 | tee "$REPORT_DIR/kind-load-server-image.log"
  if [[ "$LOAD_POSTGRES_IMAGE" == "1" ]]; then
    docker pull --platform linux/amd64 "$POSTGRES_IMAGE" 2>&1 | tee "$REPORT_DIR/docker-pull-postgres.log"
    docker save --platform linux/amd64 "$POSTGRES_IMAGE" -o "$REPORT_DIR/postgres-image.tar"
    kind load image-archive "$REPORT_DIR/postgres-image.tar" --name "$CLUSTER_NAME" 2>&1 | tee "$REPORT_DIR/kind-load-postgres-image.log"
  else
    printf 'postgres image load skipped; Kind node will pull %s if needed\n' "$POSTGRES_IMAGE" | tee "$REPORT_DIR/kind-load-postgres-image.log"
  fi
  record kind-images-loaded passed "$REPORT_DIR/kind-load-server-image.log $REPORT_DIR/kind-load-postgres-image.log" "server image loaded into Kind; postgres load mode=$LOAD_POSTGRES_IMAGE"
}

write_manifests() {
  cat > "$REPORT_DIR/k8s-manifest.yaml" <<YAML
apiVersion: v1
kind: Namespace
metadata:
  name: ${NAMESPACE}
---
apiVersion: v1
kind: Secret
metadata:
  name: tikeo-raft-transport
  namespace: ${NAMESPACE}
type: Opaque
stringData:
  transport-token: "${TOKEN}"
---
apiVersion: v1
kind: Secret
metadata:
  name: tikeo-database
  namespace: ${NAMESPACE}
type: Opaque
stringData:
  type: "postgres"
  host: "postgres"
  port: "5432"
  username: "tikeo"
  password: "${POSTGRES_PASSWORD}"
  database: "tikeo"
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: tikeo-config
  namespace: ${NAMESPACE}
data:
  tikeo.toml: |
    [server]
    listen_addr = "0.0.0.0:9090"
    worker_tunnel_addr = "0.0.0.0:9998"

    [storage]
    timestamp_offset = "+00:00"

    [storage.database]
    type = "postgres"
    host = "postgres"
    port = 5432
    username = "tikeo"
    database = "tikeo"

    [cluster]
    mode = "raft"
    node_id = "tikeo-server-0"
    scheduler_shard_map_version = 1
    scheduler_shard_count = 64
$(for i in $(seq 0 $((SERVER_REPLICAS - 1))); do printf '
    [[cluster.peers]]
    node_id = "tikeo-server-%s"
    endpoint = "http://tikeo-server-%s.tikeo-server-headless:9090"
' "$i" "$i"; done)

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

    [notification_delivery]
    enabled = false

---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: postgres
  namespace: ${NAMESPACE}
spec:
  replicas: 1
  selector:
    matchLabels:
      app: postgres
  template:
    metadata:
      labels:
        app: postgres
    spec:
      containers:
        - name: postgres
          image: ${POSTGRES_IMAGE}
          imagePullPolicy: IfNotPresent
          env:
            - name: POSTGRES_USER
              value: tikeo
            - name: POSTGRES_PASSWORD
              value: "${POSTGRES_PASSWORD}"
            - name: POSTGRES_DB
              value: tikeo
          ports:
            - containerPort: 5432
          readinessProbe:
            exec:
              command: ["pg_isready", "-U", "tikeo", "-d", "tikeo"]
            periodSeconds: 3
            timeoutSeconds: 2
            failureThreshold: 40
---
apiVersion: v1
kind: Service
metadata:
  name: postgres
  namespace: ${NAMESPACE}
spec:
  selector:
    app: postgres
  ports:
    - name: postgres
      port: 5432
      targetPort: 5432
---
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: tikeo-server
  namespace: ${NAMESPACE}
  labels:
    app.kubernetes.io/name: tikeo
    app.kubernetes.io/instance: kind-e2e
    app.kubernetes.io/component: server
spec:
  serviceName: tikeo-server-headless
  replicas: ${SERVER_REPLICAS}
  selector:
    matchLabels:
      app.kubernetes.io/name: tikeo
      app.kubernetes.io/instance: kind-e2e
      app.kubernetes.io/component: server
  template:
    metadata:
      labels:
        app.kubernetes.io/name: tikeo
        app.kubernetes.io/instance: kind-e2e
        app.kubernetes.io/component: server
    spec:
      affinity:
        podAntiAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
            - labelSelector:
                matchExpressions:
                  - key: app.kubernetes.io/component
                    operator: In
                    values: ["server"]
              topologyKey: kubernetes.io/hostname
      topologySpreadConstraints:
        - maxSkew: 1
          topologyKey: kubernetes.io/hostname
          whenUnsatisfiable: DoNotSchedule
          labelSelector:
            matchLabels:
              app.kubernetes.io/name: tikeo
              app.kubernetes.io/instance: kind-e2e
              app.kubernetes.io/component: server
      containers:
        - name: tikeo
          image: ${SERVER_IMAGE}
          imagePullPolicy: IfNotPresent
          args: ["serve", "--config", "/config/tikeo.toml"]
          env:
            - name: TIKEO__STORAGE__DATABASE__TYPE
              valueFrom:
                secretKeyRef:
                  name: tikeo-database
                  key: type
            - name: TIKEO__STORAGE__DATABASE__HOST
              valueFrom:
                secretKeyRef:
                  name: tikeo-database
                  key: host
            - name: TIKEO__STORAGE__DATABASE__PORT
              valueFrom:
                secretKeyRef:
                  name: tikeo-database
                  key: port
            - name: TIKEO__STORAGE__DATABASE__USERNAME
              valueFrom:
                secretKeyRef:
                  name: tikeo-database
                  key: username
            - name: TIKEO__STORAGE__DATABASE__PASSWORD
              valueFrom:
                secretKeyRef:
                  name: tikeo-database
                  key: password
            - name: TIKEO__STORAGE__DATABASE__DATABASE
              valueFrom:
                secretKeyRef:
                  name: tikeo-database
                  key: database
            - name: TIKEO__CLUSTER__MODE
              value: raft
            - name: TIKEO__CLUSTER__NODE_ID
              valueFrom:
                fieldRef:
                  fieldPath: metadata.name
            - name: TIKEO__CLUSTER__TRANSPORT_TOKEN
              valueFrom:
                secretKeyRef:
                  name: tikeo-raft-transport
                  key: transport-token
          ports:
            - name: http
              containerPort: 9090
            - name: worker-tunnel
              containerPort: 9998
          readinessProbe:
            httpGet:
              path: /readyz
              port: http
            initialDelaySeconds: 3
            periodSeconds: 5
            timeoutSeconds: 2
            failureThreshold: 48
          livenessProbe:
            httpGet:
              path: /healthz
              port: http
            initialDelaySeconds: 15
            periodSeconds: 20
            timeoutSeconds: 2
            failureThreshold: 6
          resources:
            requests:
              cpu: 100m
              memory: 256Mi
            limits:
              cpu: "2"
              memory: 2Gi
          volumeMounts:
            - name: config
              mountPath: /config
              readOnly: true
      volumes:
        - name: config
          configMap:
            name: tikeo-config
---
apiVersion: v1
kind: Service
metadata:
  name: tikeo-server-headless
  namespace: ${NAMESPACE}
spec:
  clusterIP: None
  publishNotReadyAddresses: true
  selector:
    app.kubernetes.io/name: tikeo
    app.kubernetes.io/instance: kind-e2e
    app.kubernetes.io/component: server
  ports:
    - name: http
      port: 9090
      targetPort: http
---
apiVersion: v1
kind: Service
metadata:
  name: tikeo
  namespace: ${NAMESPACE}
spec:
  type: ClusterIP
  selector:
    app.kubernetes.io/name: tikeo
    app.kubernetes.io/instance: kind-e2e
    app.kubernetes.io/component: server
  ports:
    - name: http
      port: 9090
      targetPort: http
---
apiVersion: v1
kind: Service
metadata:
  name: tikeo-worker-tunnel
  namespace: ${NAMESPACE}
  annotations:
    tikeo.yhyzgn.com/worker-networking: "workers-connect-outbound-only"
spec:
  type: ClusterIP
  selector:
    app.kubernetes.io/name: tikeo
    app.kubernetes.io/instance: kind-e2e
    app.kubernetes.io/component: server
  ports:
    - name: grpc-worker-tunnel
      port: 9998
      targetPort: worker-tunnel
---
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: tikeo-server
  namespace: ${NAMESPACE}
spec:
  minAvailable: 3
  selector:
    matchLabels:
      app.kubernetes.io/name: tikeo
      app.kubernetes.io/instance: kind-e2e
      app.kubernetes.io/component: server
YAML
}

run_epoch_fencing_unit_evidence() {
  (cd "$ROOT_DIR" && cargo test -p tikeo-server failover_epoch_rejects_stale_fencing_token --all-features -- --nocapture 2>&1 | tee "$REPORT_DIR/epoch-fencing-unit-test.log")
  record epoch-fencing-unit passed "$REPORT_DIR/epoch-fencing-unit-test.log" "targeted stale owner epoch fencing unit test passed"
}

deploy_stack() {
  write_manifests
  kubectl apply -f "$REPORT_DIR/k8s-manifest.yaml" 2>&1 | tee "$REPORT_DIR/kubectl-apply.log"
  kubectl -n "$NAMESPACE" rollout status deployment/postgres --timeout=240s 2>&1 | tee "$REPORT_DIR/postgres-rollout.log"
  kubectl -n "$NAMESPACE" rollout status statefulset/tikeo-server --timeout=420s 2>&1 | tee "$REPORT_DIR/server-rollout.log"
  kubectl -n "$NAMESPACE" get pods -o wide > "$REPORT_DIR/pods-after-rollout.txt"
  kubectl -n "$NAMESPACE" get svc -o wide > "$REPORT_DIR/services-after-rollout.txt"
  collect_node_spread_evidence initial
}

collect_node_spread_evidence() {
  local label="$1"
  kubectl get nodes -o wide > "$REPORT_DIR/kind-nodes-$label.txt"
  kubectl -n "$NAMESPACE" get pods -l app.kubernetes.io/component=server -o json > "$REPORT_DIR/server-pod-placement-$label.json"
  python3 - "$REPORT_DIR/server-pod-placement-$label.json" "$SERVER_REPLICAS" "$REPORT_DIR/server-pod-placement-$label-summary.json" <<'PY'
import json, sys, pathlib
path, replicas, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
payload = json.load(open(path, encoding='utf-8'))
placements = []
for item in payload.get('items', []):
    placements.append({
        'pod': item.get('metadata', {}).get('name'),
        'node': item.get('spec', {}).get('nodeName'),
        'phase': item.get('status', {}).get('phase'),
        'podIP': item.get('status', {}).get('podIP'),
    })
nodes = sorted({p['node'] for p in placements if p.get('node')})
summary = {
    'serverReplicas': replicas,
    'scheduledPods': len([p for p in placements if p.get('node')]),
    'uniqueNodes': len(nodes),
    'antiAffinitySatisfied': len(placements) == replicas and len(nodes) == replicas,
    'nodes': nodes,
    'placements': placements,
}
pathlib.Path(out).write_text(json.dumps(summary, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
if not summary['antiAffinitySatisfied']:
    raise SystemExit(f"pod anti-affinity not satisfied: {summary}")
PY
  record "pod-anti-affinity-$label" passed "$REPORT_DIR/server-pod-placement-$label-summary.json $REPORT_DIR/kind-nodes-$label.txt" "server pods are spread across ${SERVER_REPLICAS} distinct Kind nodes"
}

start_port_forward() {
  local resource="$1" local_port="$2" remote_port="$3" name="$4"
  local log_file="$REPORT_DIR/port-forward-${name}.log"
  : > "$log_file"
  kubectl -n "$NAMESPACE" port-forward "$resource" "${local_port}:${remote_port}" > "$log_file" 2>&1 &
  local pid=$!
  for _ in $(seq 1 60); do
    if grep -q "Forwarding from" "$log_file"; then
      printf '%s' "$pid"
      return 0
    fi
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      cat "$log_file" >&2 || true
      return 1
    fi
    sleep 1
  done
  cat "$log_file" >&2 || true
  return 1
}

stop_pid() {
  local pid="$1"
  if [[ -n "$pid" ]] && kill -0 "$pid" >/dev/null 2>&1; then
    kill "$pid" >/dev/null 2>&1 || true
    wait "$pid" 2>/dev/null || true
  fi
}

wait_http() {
  local label="$1" url="$2" timeout="${3:-120}"
  local deadline=$((SECONDS + timeout))
  until curl -fsS "$url" >/dev/null 2>&1; do
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for $label at $url" >&2
      return 1
    fi
    sleep 1
  done
}

collect_api_snapshot() {
  local label="$1"
  curl -fsS "$(api_url)/api/v1/cluster/diagnostics" -H "x-tikeo-api-key: ${API_KEY:-kind-e2e}" > "$REPORT_DIR/cluster-diagnostics-$label.json"
  curl -fsS "$(api_url)/api/v1/metrics/summary" -H "x-tikeo-api-key: ${API_KEY:-kind-e2e}" > "$REPORT_DIR/metrics-summary-$label.json"
  python3 -m json.tool "$REPORT_DIR/cluster-diagnostics-$label.json" > "$REPORT_DIR/cluster-diagnostics-$label.json.tmp" && mv "$REPORT_DIR/cluster-diagnostics-$label.json.tmp" "$REPORT_DIR/cluster-diagnostics-$label.json"
  python3 -m json.tool "$REPORT_DIR/metrics-summary-$label.json" > "$REPORT_DIR/metrics-summary-$label.json.tmp" && mv "$REPORT_DIR/metrics-summary-$label.json.tmp" "$REPORT_DIR/metrics-summary-$label.json"
}

leader_from_diagnostics() {
  local file="$1"
  jq -r '.data.nodes[] | select((.observedCanSchedule // .canSchedule // false) == true) | .nodeId' "$file" | head -1
}

choose_api_and_gateway_pods() {
  local diag="$1" leader="$2"
  mapfile -t pods < <(jq -r '.data.nodes[].nodeId' "$diag" | grep -Fxv "$leader" | sort)
  if (( ${#pods[@]} < 2 )); then
    echo "expected at least two non-leader pods in ${diag}; leader=${leader}" >&2
    return 1
  fi
  API_POD="${pods[0]}"
  GATEWAY_POD="${pods[1]}"
  printf 'leader=%s\napiPod=%s\ngatewayPod=%s\n' "$leader" "$API_POD" "$GATEWAY_POD" > "$REPORT_DIR/pod-selection.txt"
  record pod-selection passed "$REPORT_DIR/pod-selection.txt" "API requests and Worker Tunnel are pinned to different non-leader pods"
}

bootstrap_api_pod_and_auth() {
  API_PORT="${API_PORT:-$(free_port)}"
  local bootstrap_port
  bootstrap_port="$(free_port)"
  BOOTSTRAP_PF_PID="$(start_port_forward svc/tikeo "$bootstrap_port" 9090 bootstrap-service)"
  wait_http bootstrap-service "http://127.0.0.1:${bootstrap_port}/readyz" 120
  curl -fsS "http://127.0.0.1:${bootstrap_port}/api/v1/cluster/diagnostics" > "$REPORT_DIR/cluster-diagnostics-bootstrap.json"
  python3 -m json.tool "$REPORT_DIR/cluster-diagnostics-bootstrap.json" > "$REPORT_DIR/cluster-diagnostics-bootstrap.json.tmp" && mv "$REPORT_DIR/cluster-diagnostics-bootstrap.json.tmp" "$REPORT_DIR/cluster-diagnostics-bootstrap.json"
  LEADER_BEFORE="$(leader_from_diagnostics "$REPORT_DIR/cluster-diagnostics-bootstrap.json")"
  [[ -n "$LEADER_BEFORE" && "$LEADER_BEFORE" != "null" ]] || { echo "could not determine initial leader" >&2; return 1; }
  choose_api_and_gateway_pods "$REPORT_DIR/cluster-diagnostics-bootstrap.json" "$LEADER_BEFORE"
  stop_pid "$BOOTSTRAP_PF_PID"; BOOTSTRAP_PF_PID=""

  API_PF_PID="$(start_port_forward "pod/${API_POD}" "$API_PORT" 9090 api-pod)"
  wait_http api-pod "$(api_url)/readyz" 120
  tikeo_smoke_login "$(api_url)"
  record auth-bootstrap passed "$REPORT_DIR/port-forward-api-pod.log" "bootstrap admin token acquired through API pod ${API_POD}"
}

exists_in_list() {
  local path="$1"
  shift
  api GET "$path" | python3 -c 'import json, sys
payload = json.load(sys.stdin)
criteria = dict(arg.split("=", 1) for arg in sys.argv[1:])
data = payload.get("data") or []
items = data.get("items", []) if isinstance(data, dict) else data
for item in items:
    if all(str(item.get(k)) == v for k, v in criteria.items()):
        sys.exit(0)
sys.exit(1)' "$@"
}

seed_scope() {
  exists_in_list /api/v1/namespaces name="$NAMESPACE_NAME" || api POST /api/v1/namespaces "$(tikeo_smoke_json_object name "$NAMESPACE_NAME")" >/dev/null
  exists_in_list "/api/v1/apps?namespace=$NAMESPACE_NAME" namespace="$NAMESPACE_NAME" name="$APP_NAME" || \
    api POST /api/v1/apps "$(python3 - "$NAMESPACE_NAME" "$APP_NAME" <<'PY'
import json, sys
print(json.dumps({'namespace': sys.argv[1], 'name': sys.argv[2]}, separators=(',', ':')))
PY
)" >/dev/null
  if ! exists_in_list "/api/v1/worker-pools?namespace=$NAMESPACE_NAME&app=$APP_NAME" namespace="$NAMESPACE_NAME" app="$APP_NAME" name="$WORKER_POOL"; then
    api POST /api/v1/worker-pools "$(python3 - "$NAMESPACE_NAME" "$APP_NAME" "$WORKER_POOL" <<'PY'
import json, sys
print(json.dumps({'namespace': sys.argv[1], 'app': sys.argv[2], 'name': sys.argv[3]}, separators=(',', ':')))
PY
)" >/dev/null
  fi
  record management-scope-seed passed "$REPORT_DIR" "seeded namespace/app/worker_pool through API pod ${API_POD}"
}

create_sdk_api_key() {
  local service_account_file="$REPORT_DIR/service-account.json"
  local api_key_file="$REPORT_DIR/api-key.json"
  api POST /api/v1/management/service-accounts "$(python3 - "$RUN_ID" "$NAMESPACE_NAME" "$APP_NAME" "$WORKER_POOL" <<'PY'
import json, sys
run_id, namespace, app, worker_pool = sys.argv[1:5]
print(json.dumps({
    'name': f'{run_id}-sa',
    'description': 'Kind Raft HA E2E service account',
    'namespace': namespace,
    'app': app,
    'workerPool': worker_pool,
}, separators=(',', ':')))
PY
)" > "$service_account_file"
  local service_account_id
  service_account_id="$(json_get_file "$service_account_file" data.id)"
  api POST /api/v1/management/api-keys "$(python3 - "$RUN_ID" "$NAMESPACE_NAME" "$APP_NAME" "$service_account_id" <<'PY'
import json, sys
run_id, namespace, app, service_account_id = sys.argv[1:5]
print(json.dumps({
    'name': f'{run_id}-key',
    'namespace': namespace,
    'app': app,
    'service_account_id': service_account_id,
    'scopes': ['jobs:read', 'jobs:write', 'instances:execute', 'system:read'],
    'expires_at': None,
}, separators=(',', ':')))
PY
)" > "$api_key_file"
  API_KEY="$(json_get_file "$api_key_file" data.api_key)"
  api_key_request GET /api/v1/jobs > "$REPORT_DIR/sdk-key-jobs-list.json"
  record sdk-api-key passed "$service_account_file $api_key_file" "created app-scoped x-tikeo-api-key and verified it through API pod ${API_POD}"
}

start_worker() {
  TUNNEL_PORT="${TUNNEL_PORT:-$(free_port)}"
  TUNNEL_PF_PID="$(start_port_forward "pod/${GATEWAY_POD}" "$TUNNEL_PORT" 9998 worker-gateway-pod)"
  : > "$REPORT_DIR/worker.log"
  (
    cd "$ROOT_DIR/examples/nodejs/worker-demo"
    if [[ ! -d node_modules ]]; then bun install --frozen-lockfile >>"$REPORT_DIR/worker.log" 2>&1; fi
    TIKEO_WORKER_ENDPOINT="http://127.0.0.1:${TUNNEL_PORT}" \
    TIKEO_WORKER_CONNECT=1 \
    TIKEO_WORKER_NAMESPACE="$NAMESPACE_NAME" \
    TIKEO_WORKER_APP="$APP_NAME" \
    TIKEO_WORKER_POOL="$WORKER_POOL" \
    TIKEO_WORKER_CLUSTER=kind-raft-e2e \
    TIKEO_WORKER_REGION=kind \
    TIKEO_WORKER_CLIENT_INSTANCE_ID="$CLIENT_INSTANCE_ID" \
    TIKEO_WORKER_SDK_PROCESSORS=demo.echo,demo.sleep \
    TIKEO_DEMO_SLEEP_MS=18000 \
    TIKEO_ENABLE_PLUGIN_SQL=0 \
    TIKEO_SANDBOX_AUTO_INSTALL=0 \
    exec bun start >>"$REPORT_DIR/worker.log" 2>&1
  ) &
  WORKER_PID=$!
}

wait_worker_online() {
  local output="$REPORT_DIR/workers-online.json"
  local deadline=$((SECONDS + 180))
  until api GET /api/v1/workers > "$output" && python3 - "$output" "$CLIENT_INSTANCE_ID" "$NAMESPACE_NAME" "$APP_NAME" "$GATEWAY_POD" <<'PY'
import json, sys
path, client_id, namespace, app, gateway = sys.argv[1:6]
payload=json.load(open(path, encoding='utf-8'))
items=(payload.get('data') or {}).get('items', [])
for item in items:
    if item.get('clientInstanceId') == client_id and item.get('status') == 'online':
        if item.get('namespace') != namespace or item.get('app') != app:
            raise SystemExit(f'scope mismatch: {item}')
        caps=item.get('structuredCapabilities') or {}
        processors = caps.get('sdkProcessors') or []
        if 'demo.echo' not in processors or 'demo.sleep' not in processors:
            raise SystemExit(f'missing demo.echo/demo.sleep capability: {caps}')
        gateway_node = item.get('gatewayNodeId') or item.get('gateway_node_id')
        if gateway_node and gateway_node != gateway:
            raise SystemExit(f'expected gateway {gateway}, got {gateway_node}: {item}')
        raise SystemExit(0)
raise SystemExit(1)
PY
  do
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for worker online via gateway pod $GATEWAY_POD" >&2
      cat "$output" >&2 || true
      tail -n 200 "$REPORT_DIR/worker.log" >&2 || true
      return 1
    fi
    sleep 2
  done
  python3 -m json.tool "$output" > "$output.tmp" && mv "$output.tmp" "$output"
  record worker-online passed "$output" "worker long connection is pinned to ${GATEWAY_POD} while API uses ${API_POD}"
}

run_rollout_gate() {
  local label="$1"
  TIKEO_SERVER_URL="$(api_url)" \
  TIKEO_MANAGEMENT_API_KEY="$API_KEY" \
  TIKEO_EXPECTED_SERVER_REPLICAS="$SERVER_REPLICAS" \
  TIKEO_MAX_SHARD_SKEW="$SERVER_REPLICAS" \
  TIKEO_MAX_PENDING_AGE_SECONDS=120 \
  TIKEO_MAX_OUTBOX_AGE_SECONDS=120 \
  TIKEO_ROLLOUT_REPORT="$REPORT_DIR/rollout-${label}-raw.json" \
    "$ROOT_DIR/scripts/verify-raft-ha-rollout.sh" > "$REPORT_DIR/rollout-${label}.json"
  record "rollout-${label}" passed "$REPORT_DIR/rollout-${label}.json $REPORT_DIR/rollout-${label}-raw.json" "rollout gate passed for ${SERVER_REPLICAS} pods"
}

create_and_trigger_job() {
  local suffix="$1"
  local processor="${2:-demo.echo}"
  local payload="${3:-}"
  local job_file="$REPORT_DIR/job-$suffix.json"
  local trigger_file="$REPORT_DIR/trigger-$suffix.json"
  api_key_request POST /api/v1/jobs "$(python3 - "$NAMESPACE_NAME" "$APP_NAME" "$RUN_ID-$suffix" "$processor" <<'PY'
import json, sys
namespace, app, name, processor = sys.argv[1:5]
print(json.dumps({
  'namespace': namespace,
  'app': app,
  'name': name,
  'scheduleType': 'api',
  'processorName': processor,
  'enabled': True,
  'retryPolicy': {'enabled': True, 'maxAttempts': 3, 'initialDelaySeconds': 1, 'backoffMultiplier': 1, 'maxDelaySeconds': 5},
}, separators=(',', ':')))
PY
)" > "$job_file"
  local job_id
  job_id="$(json_get_file "$job_file" data.id)"
  api_key_request POST "/api/v1/jobs/${job_id}:trigger" "$(python3 - "$payload" <<'PY'
import json, sys
payload = sys.argv[1]
body = {'triggerType': 'api', 'executionMode': 'single'}
if payload:
    body['payload'] = payload
print(json.dumps(body, separators=(',', ':')))
PY
)" > "$trigger_file"
  json_get_file "$trigger_file" data.id
}

assert_instance_succeeded() {
  local suffix="$1" instance_id="$2"
  local instance_file="$REPORT_DIR/instance-result-$suffix.json"
  local logs_file="$REPORT_DIR/instance-logs-$suffix.json"
  tikeo_smoke_wait_instance_status "$(api_url)" "$instance_id" succeeded "$instance_file" 240
  api GET "/api/v1/instances/${instance_id}" > "$instance_file"
  api GET "/api/v1/instances/${instance_id}/logs" > "$logs_file"
  python3 - "$instance_file" "$logs_file" <<'PY'
import json, sys
instance=json.load(open(sys.argv[1], encoding='utf-8'))['data']
logs=json.load(open(sys.argv[2], encoding='utf-8'))['data']['items']
if instance.get('status') != 'succeeded':
    raise SystemExit(instance)
message = (instance.get('result') or {}).get('message') or ''
if not (message == 'nodejs demo echo processed' or message.startswith('nodejs demo sleep processed')):
    raise SystemExit(f"unexpected result: {instance.get('result')}")
joined = '\n'.join(str(item.get('message', '')) for item in logs)
if 'nodejs demo echo processed' not in joined and 'nodejs demo sleep processed' not in joined:
    raise SystemExit(f"missing worker log: {logs}")
PY
  python3 -m json.tool "$instance_file" > "$instance_file.tmp" && mv "$instance_file.tmp" "$instance_file"
  python3 -m json.tool "$logs_file" > "$logs_file.tmp" && mv "$logs_file.tmp" "$logs_file"
}

wait_outbox_for_instance_status() {
  local instance_id="$1" status="$2" timeout="${3:-60}"
  local deadline=$((SECONDS + timeout))
  local output="$REPORT_DIR/outbox-status-${instance_id}-${status}.json"
  until kubectl -n "$NAMESPACE" exec -i deploy/postgres -- psql -U tikeo -d tikeo -t -A > "$output" <<SQL
SELECT COALESCE((SELECT status FROM worker_dispatch_outbox WHERE instance_id = '$instance_id' ORDER BY created_at DESC LIMIT 1), 'missing');
SQL
  do
    sleep 1
  done
  while ! grep -Fxq "$status" "$output"; do
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for outbox instance $instance_id status $status, got $(cat "$output" 2>/dev/null || true)" >&2
      return 1
    fi
    sleep 1
    kubectl -n "$NAMESPACE" exec -i deploy/postgres -- psql -U tikeo -d tikeo -t -A > "$output" <<SQL
SELECT COALESCE((SELECT status FROM worker_dispatch_outbox WHERE instance_id = '$instance_id' ORDER BY created_at DESC LIMIT 1), 'missing');
SQL
  done
}

collect_db_evidence() {
  local label="$1"
  local out="$REPORT_DIR/db-evidence-$label.json"
  kubectl -n "$NAMESPACE" exec -i deploy/postgres -- psql -U tikeo -d tikeo -t -A > "$out" <<SQL
SELECT jsonb_pretty(jsonb_build_object(
  'label', '$label',
  'capturedAt', now(),
  'clusterShardOwnership', COALESCE((SELECT jsonb_agg(jsonb_build_object('shardId', shard_id, 'ownerNodeId', owner_node_id, 'epoch', epoch, 'raftTerm', raft_term, 'status', status, 'hasFencingToken', fencing_token IS NOT NULL AND fencing_token <> '', 'updatedAt', updated_at) ORDER BY shard_id) FROM cluster_shard_ownership), '[]'::jsonb),
  'workerSessions', COALESCE((SELECT jsonb_agg(jsonb_build_object('workerId', worker_id, 'logicalInstanceId', logical_instance_id, 'gatewayNodeId', gateway_node_id, 'generation', generation, 'status', status, 'updatedAt', updated_at) ORDER BY updated_at, worker_id) FROM worker_sessions), '[]'::jsonb),
  'workerDispatchOutbox', COALESCE((SELECT jsonb_agg(jsonb_build_object('id', id, 'instanceId', instance_id, 'workerId', worker_id, 'gatewayNodeId', gateway_node_id, 'gatewayGeneration', gateway_generation, 'shardId', shard_id, 'ownerNodeId', owner_node_id, 'ownerEpoch', owner_epoch, 'status', status, 'deliveryAttempts', delivery_attempts, 'lastError', last_error, 'updatedAt', updated_at) ORDER BY created_at, id) FROM worker_dispatch_outbox), '[]'::jsonb),
  'dispatchQueue', COALESCE((SELECT jsonb_agg(jsonb_build_object('id', id, 'jobInstanceId', job_instance_id, 'shardId', shard_id, 'status', status, 'leaseOwner', lease_owner, 'updatedAt', updated_at) ORDER BY created_at, id) FROM dispatch_queue), '[]'::jsonb)
));
SQL
  python3 -m json.tool "$out" > "$out.tmp" && mv "$out.tmp" "$out"
  record "db-evidence-$label" passed "$out" "captured durable shard/session/outbox state"
}

run_incluster_service_probe() {
  local label="$1"
  local pod="service-probe-$label"
  kubectl -n "$NAMESPACE" run "$pod" --restart=Never --image="$SERVER_IMAGE" --image-pull-policy=IfNotPresent --command -- sh -c 'for i in $(seq 1 48); do printf "request=%s " "$i"; curl -fsS -D /tmp/headers.txt http://tikeo:9090/api/v1/cluster -o /tmp/body.json; grep -i "^x-tikeo-node-id:" /tmp/headers.txt | tr -d "\015" || true; done; cat /tmp/body.json' > "$REPORT_DIR/service-probe-$label-create.log" 2>&1 || true
  kubectl -n "$NAMESPACE" wait --for=condition=Ready "pod/$pod" --timeout=60s > "$REPORT_DIR/service-probe-$label-wait-ready.log" 2>&1 || true
  kubectl -n "$NAMESPACE" wait --for=jsonpath='{.status.phase}'=Succeeded "pod/$pod" --timeout=120s > "$REPORT_DIR/service-probe-$label-wait.log" 2>&1 || true
  kubectl -n "$NAMESPACE" logs "$pod" > "$REPORT_DIR/service-probe-$label.log" 2>&1 || true
  python3 - "$REPORT_DIR/service-probe-$label.log" "$SERVER_REPLICAS" "$REPORT_DIR/service-probe-$label-summary.json" <<'PY'
import collections, json, re, sys, pathlib
log, replicas, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
text = pathlib.Path(log).read_text(encoding='utf-8', errors='replace')
nodes = re.findall(r'x-tikeo-node-id:\s*([^\s]+)', text, flags=re.I)
counts = collections.Counter(nodes)
total = sum(counts.values())
expected = total / replicas if replicas else 0
max_skew = max((abs(v - expected) for v in counts.values()), default=0)
summary = {
    'requests': total,
    'uniqueRespondingNodes': len(counts),
    'serverReplicas': replicas,
    'countsByNode': dict(sorted(counts.items())),
    'expectedPerNode': expected,
    'maxAbsoluteSkew': max_skew,
    'coverageRatio': (len(counts) / replicas) if replicas else 0,
    'passed': total >= replicas and len(counts) >= max(2, replicas // 2),
}
pathlib.Path(out).write_text(json.dumps(summary, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
if not summary['passed']:
    raise SystemExit(f"service load balancing probe did not cover enough pods: {summary}")
PY
  kubectl -n "$NAMESPACE" delete pod "$pod" --ignore-not-found=true > /dev/null 2>&1 || true
  record "incluster-service-probe-$label" passed "$REPORT_DIR/service-probe-$label.log $REPORT_DIR/service-probe-$label-summary.json" "in-cluster client called the ClusterIP API service repeatedly"
}

run_gateway_poweroff_drill() {
  GATEWAY_POD_BEFORE="$GATEWAY_POD"
  local old_gateway="$GATEWAY_POD_BEFORE"
  local before="$REPORT_DIR/db-evidence-before-gateway-poweroff.json"
  local after="$REPORT_DIR/db-evidence-after-gateway-poweroff.json"
  collect_db_evidence before-gateway-poweroff
  cp "$before" "$REPORT_DIR/gateway-poweroff-before.json" 2>/dev/null || true
  stop_pid "$TUNNEL_PF_PID"; TUNNEL_PF_PID=""
  kubectl -n "$NAMESPACE" get pod "$old_gateway" -o yaml > "$REPORT_DIR/gateway-pod-before-poweroff.yaml"
  kubectl -n "$NAMESPACE" delete pod "$old_gateway" --grace-period=0 --force --wait=false 2>&1 | tee "$REPORT_DIR/gateway-poweroff-delete.log"
  kubectl -n "$NAMESPACE" rollout status statefulset/tikeo-server --timeout=300s 2>&1 | tee "$REPORT_DIR/gateway-poweroff-rollout.log" || true
  collect_node_spread_evidence after-gateway-poweroff
  local deadline=$((SECONDS + 240))
  while (( SECONDS < deadline )); do
    local diag="$REPORT_DIR/cluster-diagnostics-gateway-reselect.json"
    collect_api_snapshot gateway-reselect || true
    mapfile -t candidates < <(jq -r --arg api "$API_POD" '.data.nodes[].nodeId | select(. != $api)' "$diag" 2>/dev/null | grep -Fxv "$old_gateway" | sort || true)
    if (( ${#candidates[@]} > 0 )); then
      GATEWAY_POD="${candidates[0]}"
      break
    fi
    sleep 3
  done
  [[ -n "$GATEWAY_POD" && "$GATEWAY_POD" != "$old_gateway" ]] || { echo "could not select replacement gateway after deleting $old_gateway" >&2; return 1; }
  TUNNEL_PF_PID="$(start_port_forward "pod/${GATEWAY_POD}" "$TUNNEL_PORT" 9998 worker-gateway-pod-reroute)"
  wait_worker_online
  collect_db_evidence after-gateway-reroute
  python3 - "$before" "$REPORT_DIR/db-evidence-after-gateway-reroute.json" "$old_gateway" "$GATEWAY_POD" "$REPORT_DIR/gateway-reroute-summary.json" <<'PY'
import json, sys, pathlib
before, after, old_gateway, new_gateway, out = sys.argv[1:]
def rows(path):
    return json.load(open(path, encoding='utf-8')).get('workerDispatchOutbox') or []
before_rows = rows(before)
after_rows = rows(after)
old_nonterminal = [r for r in before_rows if r.get('gatewayNodeId') == old_gateway and r.get('status') != 'completed']
new_rows = [r for r in after_rows if r.get('gatewayNodeId') == new_gateway]
completed = [r for r in after_rows if r.get('status') == 'completed']
summary = {
    'oldGateway': old_gateway,
    'newGateway': new_gateway,
    'oldGatewayNonTerminalBefore': len(old_nonterminal),
    'newGatewayRowsAfter': len(new_rows),
    'completedRowsAfter': len(completed),
    'rerouteObserved': any(r.get('gatewayNodeId') == new_gateway and r.get('gatewayGeneration', 0) >= 2 for r in after_rows),
    'statusByGatewayAfter': {},
}
for r in after_rows:
    summary['statusByGatewayAfter'].setdefault(r.get('gatewayNodeId'), {}).setdefault(r.get('status'), 0)
    summary['statusByGatewayAfter'][r.get('gatewayNodeId')][r.get('status')] += 1
pathlib.Path(out).write_text(json.dumps(summary, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
if not summary['rerouteObserved']:
    raise SystemExit(f"outbox reroute was not observed after gateway poweroff: {summary}")
PY
  printf 'oldGateway=%s\nnewGateway=%s\n' "$old_gateway" "$GATEWAY_POD" > "$REPORT_DIR/gateway-reroute-pods.txt"
  record gateway-poweroff-reroute passed "$REPORT_DIR/gateway-reroute-summary.json $REPORT_DIR/gateway-poweroff-delete.log" "gateway pod was force deleted; worker reconnected and durable outbox rows moved to new gateway"
}

run_fault_drill() {
  TIKEO_K8S_NAMESPACE="$NAMESPACE" \
  TIKEO_SERVER_LABEL_SELECTOR='app.kubernetes.io/component=server' \
  TIKEO_SERVER_URL="$(api_url)" \
  TIKEO_MANAGEMENT_API_KEY="$API_KEY" \
  TIKEO_FAULT_MODE=apply \
  TIKEO_FAULT=leader-pod-delete \
  TIKEO_EXPECTED_SERVER_REPLICAS="$SERVER_REPLICAS" \
  TIKEO_MAX_SHARD_SKEW="$SERVER_REPLICAS" \
  TIKEO_MAX_PENDING_AGE_SECONDS=120 \
  TIKEO_MAX_OUTBOX_AGE_SECONDS=120 \
  TIKEO_FAULT_REPORT_DIR="$REPORT_DIR/fault-drill" \
  TIKEO_RECOVERY_TIMEOUT_SECONDS=300 \
  TIKEO_VERIFY_RAFT_HA_SCRIPT="$ROOT_DIR/scripts/verify-raft-ha-rollout.sh" \
    "$ROOT_DIR/scripts/raft-ha-fault-injection-drill.sh" 2>&1 | tee "$REPORT_DIR/fault-drill.log"
  collect_api_snapshot after-fault-drill
  LEADER_AFTER="$(leader_from_diagnostics "$REPORT_DIR/cluster-diagnostics-after-fault-drill.json")"
  printf 'leaderBefore=%s\nleaderAfter=%s\napiPod=%s\ngatewayPod=%s\n' "$LEADER_BEFORE" "$LEADER_AFTER" "$API_POD" "$GATEWAY_POD" > "$REPORT_DIR/failover-summary.txt"
  record fault-drill passed "$REPORT_DIR/fault-drill.log $REPORT_DIR/failover-summary.txt" "leader pod was deleted and rollout gate recovered"
}

collect_k8s_evidence() {
  local label="$1"
  kubectl -n "$NAMESPACE" get pods -o wide > "$REPORT_DIR/pods-$label.txt" || true
  kubectl -n "$NAMESPACE" get sts,deploy,svc,pdb -o wide > "$REPORT_DIR/workloads-$label.txt" || true
  kubectl -n "$NAMESPACE" get events --sort-by=.lastTimestamp > "$REPORT_DIR/events-$label.txt" || true
  for pod in $(kubectl -n "$NAMESPACE" get pods -l app.kubernetes.io/component=server -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}' 2>/dev/null || true); do
    kubectl -n "$NAMESPACE" logs "$pod" --tail=260 > "$REPORT_DIR/${pod}-$label.log" 2>&1 || true
  done
  kubectl -n "$NAMESPACE" logs deploy/postgres --tail=160 > "$REPORT_DIR/postgres-$label.log" 2>&1 || true
}

write_summary() {
  python3 - "$SUMMARY_JSON" "$RUN_ID" "$CLUSTER_NAME" "$KIND_NODE_IMAGE" "$KIND_WORKER_NODES" "$NAMESPACE" "$SERVER_REPLICAS" "$LEADER_BEFORE" "$LEADER_AFTER" "$API_POD" "$GATEWAY_POD_BEFORE" "$GATEWAY_POD_AFTER" "$GATEWAY_POD" "$REPORT_DIR" <<'PY'
import json, sys, datetime, pathlib
out, run_id, cluster, node_image, kind_worker_nodes, namespace, replicas, leader_before, leader_after, api_pod, gateway_before, gateway_after, gateway_current, report_dir = sys.argv[1:]
summary = {
    'runId': run_id,
    'status': 'passed',
    'generatedAt': datetime.datetime.now(datetime.UTC).isoformat(),
    'kindCluster': cluster,
    'kindNodeImage': node_image,
    'namespace': namespace,
    'serverReplicas': int(replicas),
    'kindWorkerNodes': int(kind_worker_nodes),
    'leaderBefore': leader_before,
    'leaderAfter': leader_after,
    'apiPod': api_pod,
    'workerGatewayPodBeforePoweroff': gateway_before,
    'workerGatewayPodAfterPoweroff': gateway_after,
    'workerGatewayPod': gateway_current,
    'evidenceDirectory': report_dir,
    'keyEvidence': sorted(str(p.name) for p in pathlib.Path(report_dir).glob('*') if p.is_file())[:200],
}
pathlib.Path(out).write_text(json.dumps(summary, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps(summary, ensure_ascii=False, indent=2))
PY
}

main() {
  resolve_tools
  run_epoch_fencing_unit_evidence
  create_kind_cluster
  build_server_image
  deploy_stack
  bootstrap_api_pod_and_auth
  seed_scope
  create_sdk_api_key
  collect_api_snapshot initial
  run_incluster_service_probe initial
  start_worker
  wait_worker_online
  collect_db_evidence worker-online

  local initial_instance failover_instance
  initial_instance="$(create_and_trigger_job before-failover)"
  assert_instance_succeeded before-failover "$initial_instance"
  record pre-failover-dispatch passed "$REPORT_DIR/instance-result-before-failover.json" "SDK/API key request entered ${API_POD}; worker connected to ${GATEWAY_POD}; job succeeded before failover"
  collect_api_snapshot before-failover
  collect_db_evidence before-failover
  run_rollout_gate before-failover

  run_fault_drill
  run_rollout_gate after-failover
  wait_worker_online
  run_incluster_service_probe after-failover

  local gateway_pending_instance
  gateway_pending_instance="$(create_and_trigger_job before-gateway-poweroff demo.sleep)"
  wait_outbox_for_instance_status "$gateway_pending_instance" acked 60
  sleep 1
  run_gateway_poweroff_drill
  assert_instance_succeeded gateway-reroute "$gateway_pending_instance"
  record gateway-reroute-dispatch passed "$REPORT_DIR/instance-result-gateway-reroute.json" "pending job completed after old gateway pod was force deleted and Worker reconnected"
  GATEWAY_POD_AFTER="$GATEWAY_POD"

  failover_instance="$(create_and_trigger_job after-failover)"
  assert_instance_succeeded after-failover "$failover_instance"
  record post-failover-dispatch passed "$REPORT_DIR/instance-result-after-failover.json" "job succeeded after deleting initial leader pod"
  collect_api_snapshot after-failover
  collect_db_evidence after-failover
  collect_k8s_evidence final
  write_summary | tee "$REPORT_DIR/summary.stdout.json"
  tikeo_smoke_finalize_report "$REPORT_JSON" passed >/dev/null
  python3 "$ROOT_DIR/scripts/generate-kind-ha-report.py" "$REPORT_DIR" > "$REPORT_DIR/kind-ha-validation-summary.stdout.json"
  log "PASS: Kind Raft HA E2E succeeded"
  log "smoke report: $REPORT_JSON"
  log "validation report: $REPORT_DIR/kind-ha-validation-report.md"
}

main "$@"
