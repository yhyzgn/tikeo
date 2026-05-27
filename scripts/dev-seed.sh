#!/usr/bin/env sh
set -eu

DB_PATH="${1:-tikee-dev.db}"
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SQL_FILE="$SCRIPT_DIR/dev-seed.sql"

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "sqlite3 is required to apply $SQL_FILE" >&2
  exit 127
fi

if [ ! -f "$DB_PATH" ]; then
  echo "Database not found: $DB_PATH" >&2
  echo "Start tikee once first so migrations create the schema, then re-run this script." >&2
  exit 1
fi

if ! sqlite3 "$DB_PATH" "SELECT 1 FROM sqlite_master WHERE type='table' AND name='jobs';" | grep -q 1; then
  echo "Database exists but tikee tables are missing: $DB_PATH" >&2
  echo "Start tikee once first so migrations create the schema, then re-run this script." >&2
  exit 1
fi

sqlite3 "$DB_PATH" < "$SQL_FILE"

sqlite3 "$DB_PATH" <<'SQL'
.headers on
.mode column
SELECT 'namespaces' AS table_name, COUNT(*) AS rows FROM namespaces WHERE id LIKE 'ns-dev-%'
UNION ALL SELECT 'apps', COUNT(*) FROM apps WHERE id LIKE 'app-dev-%'
UNION ALL SELECT 'jobs', COUNT(*) FROM jobs WHERE id LIKE 'job-dev-%'
UNION ALL SELECT 'scripts', COUNT(*) FROM scripts WHERE id LIKE 'script-dev-%'
UNION ALL SELECT 'script_language_examples', COUNT(*) FROM scripts WHERE id LIKE 'script-dev-%-example'
UNION ALL SELECT 'script_jobs', COUNT(*) FROM jobs WHERE id LIKE 'job-dev-script-%-example'
UNION ALL SELECT 'workflows', COUNT(*) FROM workflows WHERE id LIKE 'wf-dev-%'
UNION ALL SELECT 'queue', COUNT(*) FROM dispatch_queue WHERE id LIKE 'queue-dev-%';
SQL
