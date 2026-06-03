#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_DIR="${TIKEE_DEMO_WORKER_RUN_DIR:-$ROOT_DIR/.dev/java-demo-workers}"
WORKER_ENDPOINT="${TIKEE_WORKER_ENDPOINT:-http://127.0.0.1:9998}"
MANAGEMENT_ENDPOINT="${TIKEE_MANAGEMENT_ENDPOINT:-${TIKEE_HTTP_URL:-${TIKEE_API_URL:-http://127.0.0.1:9090}}}"
MODE="start"
DETACH=0

usage() {
  cat <<USAGE
Usage: $0 [--detach] [--stop|--status]

Starts the Java demo worker matrix against one tikee server.

Environment:
  TIKEE_WORKER_ENDPOINT      Worker tunnel endpoint, default: http://127.0.0.1:9998
  TIKEE_MANAGEMENT_ENDPOINT  Management API endpoint, default: http://127.0.0.1:9090
  TIKEE_MANAGEMENT_API_KEY   Demo management API key, optional
  TIKEE_WORKER_DRY_RUN       Pass-through to demo workers, default: false
  TIKEE_DEMO_WORKER_RUN_DIR  PID/log/state root, default: .dev/java-demo-workers
USAGE
}

while (($#)); do
  case "$1" in
    --detach) DETACH=1 ;;
    --stop) MODE="stop" ;;
    --status) MODE="status" ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

mkdir -p "$RUN_DIR/logs" "$RUN_DIR/state"

# name|demo-dir|port|namespace|app|worker-pool|priority
WORKERS=(
  "java-boot2-orders-blue|examples/java/spring-boot2-worker-demo|18182|dev-alpha|orders|boot2-blue|100"
  "java-boot3-orders-blue|examples/java/spring-boot3-worker-demo|18183|dev-alpha|orders|boot3-blue|110"
  "java-boot4-billing-green|examples/java/spring-boot4-worker-demo|18184|dev-alpha|billing|boot4-green|120"
  "java-boot3-analytics-batch|examples/java/spring-boot3-worker-demo|18185|dev-beta|analytics|boot3-batch|90"
  "java-boot4-ops|examples/java/spring-boot4-worker-demo|18186|dev-ops|automation|boot4-ops|80"
)

pid_file_for() { printf '%s/%s.pid' "$RUN_DIR" "$1"; }
log_file_for() { printf '%s/logs/%s.log' "$RUN_DIR" "$1"; }
state_dir_for() { printf '%s/state/%s' "$RUN_DIR" "$1"; }

is_running() {
  local pid_file="$1"
  [[ -f "$pid_file" ]] || return 1
  local pid
  pid="$(cat "$pid_file" 2>/dev/null || true)"
  [[ -n "$pid" ]] || return 1
  kill -0 "$pid" >/dev/null 2>&1
}

stop_worker() {
  local name="$1" pid_file pid
  pid_file="$(pid_file_for "$name")"
  if ! [[ -f "$pid_file" ]]; then
    echo "○ not running: $name"
    return 0
  fi
  pid="$(cat "$pid_file" 2>/dev/null || true)"
  if [[ -n "$pid" ]] && kill -0 "$pid" >/dev/null 2>&1; then
    kill -TERM -- "-$pid" >/dev/null 2>&1 || kill -TERM "$pid" >/dev/null 2>&1 || true
    sleep 1
    kill -KILL -- "-$pid" >/dev/null 2>&1 || true
    echo "✅ stopped: $name ($pid)"
  else
    echo "○ stale pid removed: $name"
  fi
  rm -f "$pid_file"
}

status_workers() {
  local entry name dir port namespace app pool priority pid_file pid status
  for entry in "${WORKERS[@]}"; do
    IFS='|' read -r name dir port namespace app pool priority <<<"$entry"
    pid_file="$(pid_file_for "$name")"
    if is_running "$pid_file"; then
      pid="$(cat "$pid_file")"
      status="running pid=$pid"
    else
      status="stopped"
    fi
    printf '%-28s %-8s %-18s %s/%s/%s log=%s\n' "$name" "$status" "port=$port" "$namespace" "$app" "$pool" "$(log_file_for "$name")"
  done
}

wait_health() {
  local name="$1" port="$2" log_file="$3" deadline=$((SECONDS + 240))
  until curl -fsS "http://127.0.0.1:$port/demo/health" >/dev/null 2>&1; do
    if (( SECONDS >= deadline )); then
      echo "❌ timed out waiting for $name health on port $port" >&2
      tail -n 120 "$log_file" >&2 || true
      return 1
    fi
    sleep 1
  done
  echo "✅ healthy: $name http://127.0.0.1:$port/demo/health"
}

start_worker() {
  local entry="$1" name dir port namespace app pool priority pid_file log_file state_dir script
  IFS='|' read -r name dir port namespace app pool priority <<<"$entry"
  pid_file="$(pid_file_for "$name")"
  log_file="$(log_file_for "$name")"
  state_dir="$(state_dir_for "$name")"
  script="$ROOT_DIR/$dir/scripts/run-demo-worker.sh"

  if is_running "$pid_file"; then
    echo "✅ already running: $name ($(cat "$pid_file"))"
    return 0
  fi
  if [[ ! -x "$script" ]]; then
    echo "missing executable demo script: $script" >&2
    return 1
  fi

  mkdir -p "$state_dir"
  : >"$log_file"
  setsid env \
    TIKEE_WORKER_ENDPOINT="$WORKER_ENDPOINT" \
    TIKEE_MANAGEMENT_ENDPOINT="$MANAGEMENT_ENDPOINT" \
    TIKEE_WORKER_NAMESPACE="$namespace" \
    TIKEE_WORKER_APP="$app" \
    TIKEE_WORKER_POOL="$pool" \
    TIKEE_WORKER_CLUSTER="local" \
    TIKEE_WORKER_REGION="local" \
    TIKEE_WORKER_CLIENT_INSTANCE_ID="$name" \
    TIKEE_DEMO_SERVER_PORT="$port" \
    TIKEE_WORKER_STATE_DIR="$state_dir" \
    TIKEE_WORKER_ELECTION_DOMAIN="$namespace/$app/$pool/local" \
    TIKEE_WORKER_ELECTION_PRIORITY="$priority" \
    TIKEE_MANAGEMENT_NAMESPACE="$namespace" \
    TIKEE_MANAGEMENT_APP="$app" \
    "$script" >"$log_file" 2>&1 &
  echo "$!" >"$pid_file"
  echo "▶ started: $name pid=$(cat "$pid_file") scope=$namespace/$app/$pool port=$port log=$log_file"
  wait_health "$name" "$port" "$log_file"
}

case "$MODE" in
  stop)
    for entry in "${WORKERS[@]}"; do
      IFS='|' read -r name _ <<<"$entry"
      stop_worker "$name"
    done
    exit 0
    ;;
  status)
    status_workers
    exit 0
    ;;
esac

cleanup() {
  local code=$?
  trap - INT TERM EXIT
  if (( DETACH == 0 )); then
    echo
    echo "正在停止 Java demo workers..."
    for entry in "${WORKERS[@]}"; do
      IFS='|' read -r name _ <<<"$entry"
      stop_worker "$name"
    done
  fi
  exit "$code"
}
trap cleanup INT TERM EXIT

for entry in "${WORKERS[@]}"; do
  start_worker "$entry"
done

echo
echo "Java demo worker 矩阵已启动："
status_workers
echo
echo "建议先执行一次联调数据初始化：scripts/dev-integration-seed.sh"
echo "日志目录：$RUN_DIR/logs"

if (( DETACH == 1 )); then
  trap - INT TERM EXIT
  echo "已后台运行；停止命令：$0 --stop"
  exit 0
fi

echo "前台保持运行，按 Ctrl+C 停止全部 demo workers。"
while true; do
  sleep 3600 &
  wait $!
done
