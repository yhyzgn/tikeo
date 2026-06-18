#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${1:-/tmp/tikeo-migrate-fixtures}"
mkdir -p "$OUT_DIR"

create_with_python() {
  local db="$1"
  local sql="$2"
  python3 - "$db" "$sql" <<'PY'
import pathlib
import sqlite3
import sys

db_path = pathlib.Path(sys.argv[1])
sql_path = pathlib.Path(sys.argv[2])
conn = sqlite3.connect(db_path)
conn.executescript(sql_path.read_text())
conn.commit()
conn.close()
print(db_path)
PY
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
create_with_python "$OUT_DIR/xxl-job.db" "$SCRIPT_DIR/xxl-job-sqlite.sql"
create_with_python "$OUT_DIR/powerjob.db" "$SCRIPT_DIR/powerjob-sqlite.sql"

cat <<MSG
Created demo fixture databases:
  $OUT_DIR/xxl-job.db
  $OUT_DIR/powerjob.db

Try:
  tikeo-migrate plan --from xxl-job --legacy-db-url sqlite:$OUT_DIR/xxl-job.db --output-dir $OUT_DIR/xxl-bundle
  tikeo-migrate plan --from powerjob --legacy-db-url sqlite:$OUT_DIR/powerjob.db --output-dir $OUT_DIR/powerjob-bundle
MSG
