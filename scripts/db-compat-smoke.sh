#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/deploy/compose/database-compat-compose.yml"
RUN_EXTERNAL="${TIKEO_DB_COMPAT_EXTERNAL:-auto}"
RUN_SQLITE="${TIKEO_DB_COMPAT_SQLITE:-true}"
START_COMPOSE="${TIKEO_DB_COMPAT_COMPOSE:-auto}"
POSTGRES_PORT="${TIKEO_TEST_POSTGRES_PORT:-15432}"
MYSQL_PORT="${TIKEO_TEST_MYSQL_PORT:-13306}"
export TIKEO_TEST_POSTGRES_URL="${TIKEO_TEST_POSTGRES_URL:-postgres://tikeo:tikeo@127.0.0.1:${POSTGRES_PORT}/tikeo}"
export TIKEO_TEST_MYSQL_URL="${TIKEO_TEST_MYSQL_URL:-mysql://tikeo:tikeo@127.0.0.1:${MYSQL_PORT}/tikeo}"

cd "$ROOT_DIR"

cleanup() {
  if [[ "${COMPOSE_STARTED:-false}" == "true" ]]; then
    docker compose -f "$COMPOSE_FILE" down -v >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

if [[ "$RUN_SQLITE" == "true" ]]; then
  echo "[db-compat] running SQLite storage compatibility smoke"
  cargo test -p tikeo-storage --test database_compat sqlite_database_compatibility_smoke -- --nocapture
fi

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

if [[ "$RUN_EXTERNAL" == "false" ]]; then
  echo "[db-compat] external PostgreSQL/MySQL smoke disabled"
  exit 0
fi

if [[ "$should_start_compose" == "true" ]]; then
  echo "[db-compat] starting PostgreSQL/MySQL compatibility services"
  docker compose -f "$COMPOSE_FILE" up -d --wait
  COMPOSE_STARTED=true
elif [[ "$RUN_EXTERNAL" == "auto" && -z "${TIKEO_TEST_CONNECTION_URLS:-}" && -z "${TIKEO_TEST_POSTGRES_URL:-}" && -z "${TIKEO_TEST_MYSQL_URL:-}" ]]; then
  echo "[db-compat] docker unavailable and no external DB URLs provided; SQLite smoke completed"
  exit 0
fi

echo "[db-compat] running PostgreSQL/MySQL storage compatibility smoke"
cargo test -p tikeo-storage --test database_compat external_database_compatibility_smoke -- --nocapture
