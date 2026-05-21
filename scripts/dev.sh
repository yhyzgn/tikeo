#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG_FILE="${SCHEDULER_CONFIG:-$ROOT_DIR/config/dev.toml}"
API_PORT="${SCHEDULER_API_PORT:-9090}"
WEB_PORT="${SCHEDULER_WEB_PORT:-5173}"
API_URL="${SCHEDULER_API_URL:-http://localhost:$API_PORT}"
WEB_URL="${SCHEDULER_WEB_URL:-http://localhost:$WEB_PORT}"
LOG_DIR="$ROOT_DIR/.dev"

export SCHEDULER_DEV_ADMIN_USERNAME="${SCHEDULER_DEV_ADMIN_USERNAME:-scheduler_init}"
export SCHEDULER_DEV_ADMIN_PASSWORD="${SCHEDULER_DEV_ADMIN_PASSWORD:-Scheduler@2026!}"
export SCHEDULER_DEV_ADMIN_TOKEN="${SCHEDULER_DEV_ADMIN_TOKEN:-scheduler-init-token}"

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

cleanup() {
  local code=$?
  echo
  echo "正在停止 scheduler 开发进程..."
  if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
  fi
  if [[ -n "${WEB_PID:-}" ]] && kill -0 "$WEB_PID" >/dev/null 2>&1; then
    kill "$WEB_PID" >/dev/null 2>&1 || true
  fi
  wait "${SERVER_PID:-0}" 2>/dev/null || true
  wait "${WEB_PID:-0}" 2>/dev/null || true
  exit "$code"
}
trap cleanup INT TERM EXIT

if [[ ! -f "$CONFIG_FILE" ]]; then
  echo "配置文件不存在：$CONFIG_FILE" >&2
  exit 1
fi

if [[ ! -d "$ROOT_DIR/web/node_modules" ]]; then
  echo "首次启动：安装 Web 依赖..."
  (cd "$ROOT_DIR/web" && bun install)
fi

echo "启动后端：$API_URL"
(
  cd "$ROOT_DIR"
  cargo run --bin scheduler -- serve --config "$CONFIG_FILE"
) >"$LOG_DIR/server.log" 2>&1 &
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
    tail -n 80 "$LOG_DIR/server.log" >&2 || true
    exit 1
  fi
  echo -n "."
  sleep 1
done

if ! curl -fsS "$API_URL/healthz" >/dev/null 2>&1; then
  echo
  echo "后端健康检查超时，最近日志：" >&2
  tail -n 80 "$LOG_DIR/server.log" >&2 || true
  exit 1
fi

echo "启动 Web：$WEB_URL"
(
  cd "$ROOT_DIR/web"
  bun run dev -- --port "$WEB_PORT"
) >"$LOG_DIR/web.log" 2>&1 &
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
    tail -n 80 "$LOG_DIR/web.log" >&2 || true
    exit 1
  fi
  echo -n "."
  sleep 1
done

if ! curl -fsS "$WEB_URL" >/dev/null 2>&1; then
  echo
  echo "Web 健康检查超时，最近日志：" >&2
  tail -n 80 "$LOG_DIR/web.log" >&2 || true
  exit 1
fi

echo
echo "开发环境已启动："
echo "  Web UI:       $WEB_URL"
echo "  Backend API:  $API_URL"
echo "  OpenAPI JSON: $API_URL/api-docs/openapi.json"
echo "  初始化账号:  $SCHEDULER_DEV_ADMIN_USERNAME"
echo "  初始化密码:  $SCHEDULER_DEV_ADMIN_PASSWORD"
echo "  后端日志:    $LOG_DIR/server.log"
echo "  前端日志:    $LOG_DIR/web.log"
echo
echo "按 Ctrl+C 停止全部进程。"

wait -n "$SERVER_PID" "$WEB_PID"
