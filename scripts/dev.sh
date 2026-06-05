#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG_FILE="${TIKEE_CONFIG:-$ROOT_DIR/config/dev.toml}"
API_PORT="${TIKEE_API_PORT:-9090}"
WEB_PORT="${TIKEE_WEB_PORT:-5173}"
API_URL="${TIKEE_API_URL:-http://localhost:$API_PORT}"
WEB_URL="${TIKEE_WEB_URL:-http://localhost:$WEB_PORT}"
LOG_DIR="$ROOT_DIR/.dev"

mkdir -p "$LOG_DIR"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "缺少命令：$1" >&2
    exit 127
  fi
}

need_cmd cargo
need_cmd bun
need_cmd curl
need_cmd python3

extract_sqlite_db_path() {
  python3 -c 'import re, sys
path = sys.argv[1]
try:
    text = open(path, encoding="utf-8").read()
except OSError:
    sys.exit(0)
match = re.search(r"^\s*database_url\s*=\s*\"(sqlite://[^\"]+)\"", text, re.M)
if not match:
    sys.exit(0)
url = match.group(1)[len("sqlite://"):]
url = url.split("?", 1)[0]
print(url)' "$CONFIG_FILE"
}

extract_config_port() {
  local key="$1"
  python3 -c 'import re, sys
path, key = sys.argv[1], sys.argv[2]
try:
    text = open(path, encoding="utf-8").read()
except OSError:
    sys.exit(0)
match = re.search(r"^\s*" + re.escape(key) + r"\s*=\s*\"[^\"]*:(\d+)\"", text, re.M)
if not match:
    sys.exit(0)
print(match.group(1))' "$CONFIG_FILE" "$key"
}

port_in_use() {
  local port="$1"
  python3 -c 'import socket, sys
port = int(sys.argv[1])
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.settimeout(0.2)
    sys.exit(0 if sock.connect_ex(("127.0.0.1", port)) == 0 else 1)' "$port"
}

describe_port_owner() {
  local port="$1"
  if command -v ss >/dev/null 2>&1; then
    ss -ltnp 2>/dev/null | awk -v p=":$port" '$4 ~ p "$" { print "  " $0 }' >&2 || true
  fi
}

ensure_port_free() {
  local label="$1"
  local port="$2"
  if port_in_use "$port"; then
    echo "$label 端口 $port 已被占用，dev.sh 无法启动。" >&2
    describe_port_owner "$port"
    echo "请先停止占用该端口的旧 tikee/dev 进程，或通过环境变量指定其他端口。" >&2
    echo "示例：TIKEE_API_PORT=9091 TIKEE_WEB_PORT=5174 ./scripts/dev.sh" >&2
    exit 1
  fi
}

backup_malformed_sqlite_db() {
  local db_path="$1"
  [[ -n "$db_path" && -f "$db_path" ]] || return 0
  if ! command -v sqlite3 >/dev/null 2>&1; then
    return 0
  fi
  local check_output
  check_output="$(sqlite3 "$db_path" 'PRAGMA integrity_check;' 2>&1 || true)"
  if [[ "$check_output" != "ok" ]]; then
    local stamp backup_dir base
    stamp="$(date +%Y%m%d-%H%M%S)"
    backup_dir="$LOG_DIR/db-backups"
    base="$(basename "$db_path")"
    mkdir -p "$backup_dir"
    echo "检测到 dev SQLite schema 损坏，自动备份并重建：$check_output" >&2
    for suffix in "" "-shm" "-wal"; do
      if [[ -f "$db_path$suffix" ]]; then
        cp -a "$db_path$suffix" "$backup_dir/$base$suffix.$stamp.bak" || true
        rm -f "$db_path$suffix"
      fi
    done
    echo "损坏数据库已备份到：$backup_dir" >&2
  fi
}

start_log_console() {
  local label="$1"
  local log_file="$2"
  tail -n +1 -F "$log_file" 2>/dev/null | sed -u "s/^/[$label] /"
}

terminate_process_tree() {
  local pid="${1:-}"
  [[ -n "$pid" ]] || return 0
  if ! kill -0 "$pid" >/dev/null 2>&1; then
    return 0
  fi

  # Prefer killing the process group so cargo/bun children and log pipelines cannot survive Ctrl+C.
  kill -TERM -- "-$pid" >/dev/null 2>&1 || kill -TERM "$pid" >/dev/null 2>&1 || true
}

cleanup() {
  local code=$?
  trap - INT TERM EXIT
  if [[ "${TIKEE_DEV_CLEANING_UP:-0}" == "1" ]]; then
    exit "$code"
  fi
  TIKEE_DEV_CLEANING_UP=1
  echo
  echo "正在停止 tikee 开发进程..."
  terminate_process_tree "${SERVER_PID:-}"
  terminate_process_tree "${WEB_PID:-}"
  terminate_process_tree "${SERVER_LOG_CONSOLE_PID:-}"
  terminate_process_tree "${WEB_LOG_CONSOLE_PID:-}"

  sleep 1
  kill -KILL -- "-${SERVER_PID:-0}" >/dev/null 2>&1 || true
  kill -KILL -- "-${WEB_PID:-0}" >/dev/null 2>&1 || true
  kill -KILL -- "-${SERVER_LOG_CONSOLE_PID:-0}" >/dev/null 2>&1 || true
  kill -KILL -- "-${WEB_LOG_CONSOLE_PID:-0}" >/dev/null 2>&1 || true

  wait "${SERVER_PID:-0}" 2>/dev/null || true
  wait "${WEB_PID:-0}" 2>/dev/null || true
  wait "${SERVER_LOG_CONSOLE_PID:-0}" 2>/dev/null || true
  wait "${WEB_LOG_CONSOLE_PID:-0}" 2>/dev/null || true
  echo "已停止。"
  exit "$code"
}
trap cleanup INT TERM EXIT

if [[ ! -f "$CONFIG_FILE" ]]; then
  echo "配置文件不存在：$CONFIG_FILE" >&2
  exit 1
fi

API_PORT="${TIKEE_API_PORT:-$(extract_config_port listen_addr || true)}"
API_PORT="${API_PORT:-9090}"
TUNNEL_PORT="${TIKEE_TUNNEL_PORT:-$(extract_config_port worker_tunnel_addr || true)}"
TUNNEL_PORT="${TUNNEL_PORT:-9998}"
API_URL="${TIKEE_API_URL:-http://localhost:$API_PORT}"
WEB_URL="${TIKEE_WEB_URL:-http://localhost:$WEB_PORT}"

ensure_port_free "后端 HTTP" "$API_PORT"
ensure_port_free "Worker Tunnel" "$TUNNEL_PORT"
ensure_port_free "Web" "$WEB_PORT"

DB_PATH="$(extract_sqlite_db_path)"
if [[ -n "$DB_PATH" && "$DB_PATH" != /* ]]; then
  DB_PATH="$ROOT_DIR/$DB_PATH"
fi
backup_malformed_sqlite_db "$DB_PATH"

if [[ ! -d "$ROOT_DIR/web/node_modules" ]]; then
  echo "首次启动：安装 Web 依赖..."
  (cd "$ROOT_DIR/web" && bun install)
fi

SERVER_LOG="$LOG_DIR/server.log"
WEB_LOG="$LOG_DIR/web.log"
: >"$SERVER_LOG"
: >"$WEB_LOG"

export MIMALLOC_PURGE_DELAY="${MIMALLOC_PURGE_DELAY:-0}"
export MIMALLOC_PURGE_DECOMMITS="${MIMALLOC_PURGE_DECOMMITS:-1}"
export MIMALLOC_ABANDONED_PAGE_PURGE="${MIMALLOC_ABANDONED_PAGE_PURGE:-1}"

echo "启动后端：$API_URL"
setsid bash -c 'tail -n +1 -F "$1" 2>/dev/null | sed -u "s/^/[$2] /"' _ "$SERVER_LOG" server &
SERVER_LOG_CONSOLE_PID=$!
setsid bash -c 'cd "$1" && exec cargo run --bin tikee -- serve --config "$2"' _ "$ROOT_DIR" "$CONFIG_FILE" >"$SERVER_LOG" 2>&1 &
SERVER_PID=$!

echo -n "等待后端健康检查"
for _ in $(seq 1 60); do
  if curl -fsS "$API_URL/healthz" >/dev/null 2>&1; then
    echo " OK"
    break
  fi
  if ! kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    echo
    echo "后端启动失败，最近日志：" >&2
    tail -n 80 "$SERVER_LOG" >&2 || true
    exit 1
  fi
  echo -n "."
  sleep 1
done

if ! curl -fsS "$API_URL/healthz" >/dev/null 2>&1; then
  echo
  echo "后端健康检查超时，最近日志：" >&2
  tail -n 80 "$SERVER_LOG" >&2 || true
  exit 1
fi

echo "启动 Web：$WEB_URL"
setsid bash -c 'tail -n +1 -F "$1" 2>/dev/null | sed -u "s/^/[$2] /"' _ "$WEB_LOG" web &
WEB_LOG_CONSOLE_PID=$!
setsid bash -c 'cd "$1/web" && exec bun run dev -- --port "$2" --strictPort' _ "$ROOT_DIR" "$WEB_PORT" >"$WEB_LOG" 2>&1 &
WEB_PID=$!

echo -n "等待 Web dev server"
for _ in $(seq 1 60); do
  if curl -fsS "$WEB_URL" >/dev/null 2>&1; then
    echo " OK"
    break
  fi
  if ! kill -0 "$WEB_PID" >/dev/null 2>&1; then
    echo
    echo "Web 启动失败，最近日志：" >&2
    tail -n 80 "$WEB_LOG" >&2 || true
    exit 1
  fi
  echo -n "."
  sleep 1
done

if ! curl -fsS "$WEB_URL" >/dev/null 2>&1; then
  echo
  echo "Web 健康检查超时，最近日志：" >&2
  tail -n 80 "$WEB_LOG" >&2 || true
  exit 1
fi

echo
echo "开发环境已启动："
echo "  Web UI:       $WEB_URL"
echo "  Backend API:  $API_URL"
echo "  OpenAPI JSON: $API_URL/api-docs/openapi.json"
echo "  首次访问:    打开 Web UI 后注册初始化管理员（注册成功后入口自动关闭）"
echo "  后端日志:    $SERVER_LOG（同时输出到当前控制台，前缀 [server]）"
echo "  前端日志:    $WEB_LOG（同时输出到当前控制台，前缀 [web]）"
echo
echo "按 Ctrl+C 停止全部进程。"

wait -n "$SERVER_PID" "$WEB_PID"
