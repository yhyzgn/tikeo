#!/usr/bin/env sh
set -eu

REFRESH="${TIKEO_DEV_SEED_REFRESH:-0}"
if [ "${1:-}" = "--refresh" ]; then
  REFRESH=1
  shift
fi

DB_PATH="${1:-.dev/tikeo-dev.db}"
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SQL_FILE="$SCRIPT_DIR/dev-seed.sql"

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "sqlite3 is required to apply $SQL_FILE" >&2
  exit 127
fi

if [ ! -f "$DB_PATH" ]; then
  echo "Database not found: $DB_PATH" >&2
  echo "Start tikeo once first so migrations create the schema, then re-run this script." >&2
  exit 1
fi

if ! sqlite3 "$DB_PATH" "SELECT 1 FROM sqlite_master WHERE type='table' AND name='jobs';" | grep -q 1; then
  echo "Database exists but tikeo tables are missing: $DB_PATH" >&2
  echo "Start tikeo once first so migrations create the schema, then re-run this script." >&2
  exit 1
fi

if [ "$REFRESH" != "1" ]; then
  existing_seed_rows="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM namespaces WHERE id LIKE 'ns-dev-%';")"
  if [ "${existing_seed_rows:-0}" -gt 0 ]; then
    echo "Development seed data already exists in $DB_PATH; leaving local rows unchanged." >&2
    echo "Use TIKEO_DEV_SEED_REFRESH=1 $0 $DB_PATH or $0 --refresh $DB_PATH only when you intentionally want to refresh the seeded demo rows." >&2
    sqlite3 "$DB_PATH" <<'SQL'
.headers on
.mode column
SELECT 'namespaces' AS table_name, COUNT(*) AS rows FROM namespaces WHERE id LIKE 'ns-dev-%'
UNION ALL SELECT 'apps', COUNT(*) FROM apps WHERE id LIKE 'app-dev-%'
UNION ALL SELECT 'worker_pools', COUNT(*) FROM worker_pools WHERE id LIKE 'wp-dev-%'
UNION ALL SELECT 'jobs', COUNT(*) FROM jobs WHERE id LIKE 'job-dev-%'
UNION ALL SELECT 'scripts', COUNT(*) FROM scripts WHERE id LIKE 'script-dev-%'
UNION ALL SELECT 'notification_channels', COUNT(*) FROM notification_channels WHERE id LIKE 'notif-channel-dev-%'
UNION ALL SELECT 'notification_templates', COUNT(*) FROM notification_templates WHERE id LIKE 'notif-template-dev-%'
UNION ALL SELECT 'notification_policies', COUNT(*) FROM notification_policies WHERE id LIKE 'notif-policy-dev-%';
SQL
    exit 0
  fi
fi

sqlite3 -bail "$DB_PATH" < "$SQL_FILE"

sqlite3 "$DB_PATH" <<'SQL'
.headers on
.mode column
SELECT 'namespaces' AS table_name, COUNT(*) AS rows FROM namespaces WHERE id LIKE 'ns-dev-%'
UNION ALL SELECT 'apps', COUNT(*) FROM apps WHERE id LIKE 'app-dev-%'
UNION ALL SELECT 'worker_pools', COUNT(*) FROM worker_pools WHERE id LIKE 'wp-dev-%'
UNION ALL SELECT 'jobs', COUNT(*) FROM jobs WHERE id LIKE 'job-dev-%'
UNION ALL SELECT 'scripts', COUNT(*) FROM scripts WHERE id LIKE 'script-dev-%'
UNION ALL SELECT 'script_language_examples', COUNT(*) FROM scripts WHERE id LIKE 'script-dev-%-example'
UNION ALL SELECT 'script_jobs', COUNT(*) FROM jobs WHERE id LIKE 'job-dev-script-%-example'
UNION ALL SELECT 'workflows', COUNT(*) FROM workflows WHERE id LIKE 'wf-dev-%'
UNION ALL SELECT 'notification_channels', COUNT(*) FROM notification_channels WHERE id LIKE 'notif-channel-dev-%'
UNION ALL SELECT 'notification_templates', COUNT(*) FROM notification_templates WHERE id LIKE 'notif-template-dev-%'
UNION ALL SELECT 'notification_policies', COUNT(*) FROM notification_policies WHERE id LIKE 'notif-policy-dev-%'
UNION ALL SELECT 'notification_messages', COUNT(*) FROM notification_messages WHERE id LIKE 'notif-msg-dev-%'
UNION ALL SELECT 'notification_attempts', COUNT(*) FROM notification_delivery_attempts WHERE id LIKE 'notif-attempt-dev-%'
UNION ALL SELECT 'queue', COUNT(*) FROM dispatch_queue WHERE id LIKE 'queue-dev-%';
SQL
