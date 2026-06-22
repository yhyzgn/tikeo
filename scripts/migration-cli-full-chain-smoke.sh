#!/usr/bin/env bash
# Full-chain local smoke for the standalone tikeo-migrate CLI.
# The script creates a throwaway legacy Spring Boot + XXL-JOB style project,
# auto-exports jobs from a SQLite legacy DB, writes the migration bundle, runs
# dry-run apply, then live-applies ready jobs to a local mock Tikeo Management API.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="${TIKEO_MIGRATE_SMOKE_RUN_ID:-migration-cli-full-chain-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
REPORT_DIR="${TIKEO_MIGRATE_SMOKE_REPORT_DIR:-$ROOT_DIR/.dev/reports/$RUN_ID}"
MIGRATE_BIN="$ROOT_DIR/target/debug/tikeo-migrate"
LEGACY_PROJECT="$REPORT_DIR/legacy-xxl-worker"
LEGACY_DB="$LEGACY_PROJECT/legacy-xxl-job.db"
BUNDLE_DIR="$LEGACY_PROJECT/.tikeo-migration"
MOCK_SCRIPT="$REPORT_DIR/mock-management-api.py"
MOCK_LOG="$REPORT_DIR/$RUN_ID-mock-management.log"
MOCK_PORT_FILE="$REPORT_DIR/mock-management-port.txt"
MOCK_REQUESTS="$REPORT_DIR/management-api-requests.jsonl"
SUMMARY_JSON="$REPORT_DIR/summary.json"
REPORT_MD="$REPORT_DIR/REPORT.md"
mkdir -p "$REPORT_DIR"
: > "$MOCK_REQUESTS"

MOCK_PID=""
cleanup() {
  local code=$?
  if [[ -n "$MOCK_PID" ]] && kill -0 "$MOCK_PID" >/dev/null 2>&1; then
    kill "$MOCK_PID" >/dev/null 2>&1 || true
    wait "$MOCK_PID" 2>/dev/null || true
  fi
  if (( code != 0 )); then
    echo "migration CLI full-chain smoke failed; evidence: $REPORT_DIR" >&2
    echo "--- mock management log tail ---" >&2
    tail -n 120 "$MOCK_LOG" >&2 || true
  fi
  exit "$code"
}
trap cleanup EXIT INT TERM

need_cmd() { command -v "$1" >/dev/null 2>&1 || { echo "missing command: $1" >&2; exit 127; }; }

build_migrate_binary() {
  if [[ ! -x "$MIGRATE_BIN" || "${TIKEO_MIGRATE_SMOKE_REBUILD:-1}" == "1" ]]; then
    (cd "$ROOT_DIR" && cargo build --bin tikeo-migrate > "$REPORT_DIR/cargo-build-tikeo-migrate.log" 2>&1)
  fi
}

create_legacy_project() {
  mkdir -p "$LEGACY_PROJECT/src/main/java/com/example/billing" "$LEGACY_PROJECT/src/main/resources"
  cat > "$LEGACY_PROJECT/build.gradle.kts" <<'GRADLE'
plugins {
    id("org.springframework.boot") version "3.5.8"
    id("io.spring.dependency-management") version "1.1.7"
    java
}

repositories { mavenCentral() }

dependencies {
    implementation("com.xuxueli:xxl-job-core:2.4.1")
}
GRADLE
  cat > "$LEGACY_PROJECT/src/main/java/com/example/billing/BillingJobs.java" <<'JAVA'
package com.example.billing;

import com.xxl.job.core.handler.annotation.XxlJob;

public class BillingJobs {
    @XxlJob("billingProcessor")
    public void billingProcessor() {
        System.out.println("legacy billing job");
    }

    @XxlJob("reportRebuildProcessor")
    public void reportRebuildProcessor() {
        System.out.println("legacy report rebuild job");
    }
}
JAVA
  python3 - "$LEGACY_DB" "$ROOT_DIR/examples/migration/legacy-scheduler-fixtures/xxl-job-sqlite.sql" <<'PY'
import pathlib, sqlite3, sys
db = pathlib.Path(sys.argv[1])
sql = pathlib.Path(sys.argv[2]).read_text(encoding='utf-8')
conn = sqlite3.connect(db)
conn.executescript(sql)
conn.commit()
conn.close()
PY
  cat > "$LEGACY_PROJECT/src/main/resources/application.properties" <<PROP
spring.application.name=legacy-billing-worker
spring.datasource.url=sqlite:$LEGACY_DB
PROP
}

start_mock_management_api() {
  cat > "$MOCK_SCRIPT" <<'PY'
import datetime, json, pathlib, sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

out = pathlib.Path(sys.argv[1])
port_file = pathlib.Path(sys.argv[2])

class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get('content-length') or 0)
        body_text = self.rfile.read(length).decode('utf-8', errors='replace')
        try:
            body = json.loads(body_text)
        except Exception:
            body = body_text
        event = {
            'path': self.path,
            'method': 'POST',
            'receivedAt': datetime.datetime.now(datetime.UTC).isoformat(),
            'apiKey': self.headers.get('x-tikeo-api-key'),
            'body': body,
        }
        with out.open('a', encoding='utf-8') as fh:
            json.dump(event, fh, ensure_ascii=False)
            fh.write('\n')
        response = {'code': 0, 'message': 'ok', 'data': {'id': f"mock-{len(out.read_text(encoding='utf-8').splitlines())}", 'name': body.get('name') if isinstance(body, dict) else 'imported'}}
        payload = json.dumps(response, ensure_ascii=False).encode('utf-8')
        self.send_response(201)
        self.send_header('content-type', 'application/json')
        self.send_header('content-length', str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)
    def log_message(self, fmt, *args):
        sys.stderr.write(fmt % args + '\n')

server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
port_file.write_text(str(server.server_address[1]), encoding='utf-8')
server.serve_forever()
PY
  python3 "$MOCK_SCRIPT" "$MOCK_REQUESTS" "$MOCK_PORT_FILE" >"$MOCK_LOG" 2>&1 &
  MOCK_PID=$!
  local deadline=$((SECONDS + 30))
  until [[ -s "$MOCK_PORT_FILE" ]]; do
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for mock management API port file" >&2
      return 1
    fi
    sleep 0.2
  done
}

run_chain() {
  (cd "$LEGACY_PROJECT" && "$MIGRATE_BIN" plan > "$REPORT_DIR/plan.stdout" 2> "$REPORT_DIR/plan.stderr")
  (cd "$LEGACY_PROJECT" && "$MIGRATE_BIN" apply --bundle .tikeo-migration --endpoint http://127.0.0.1:1 --api-key dry-run-key --include-needs-review --dry-run > "$REPORT_DIR/apply-dry-run.stdout" 2> "$REPORT_DIR/apply-dry-run.stderr")
  cp "$BUNDLE_DIR/apply-evidence.json" "$REPORT_DIR/apply-dry-run-evidence.json"
  start_mock_management_api
  local endpoint="http://127.0.0.1:$(cat "$MOCK_PORT_FILE")"
  (cd "$LEGACY_PROJECT" && "$MIGRATE_BIN" apply --bundle .tikeo-migration --endpoint "$endpoint" --api-key live-smoke-key --include-needs-review > "$REPORT_DIR/apply-live.stdout" 2> "$REPORT_DIR/apply-live.stderr")
  cp "$BUNDLE_DIR/apply-evidence.json" "$REPORT_DIR/apply-live-evidence.json"
}

verify_chain() {
  python3 - "$REPORT_DIR" "$BUNDLE_DIR" <<'PY'
import json, pathlib, sys
report = pathlib.Path(sys.argv[1])
bundle = pathlib.Path(sys.argv[2])

def read_json(path):
    return json.loads(path.read_text(encoding='utf-8'))

manifest = read_json(bundle / 'manifest.json')
jobs = read_json(bundle / 'jobs.tikeo.json')
data_import = read_json(bundle / 'data-import-plan.json')
java_plan = read_json(bundle / 'java-project-plan.json')
dry = read_json(report / 'apply-dry-run-evidence.json')
live = read_json(report / 'apply-live-evidence.json')
requests = [json.loads(line) for line in (report / 'management-api-requests.jsonl').read_text(encoding='utf-8').splitlines() if line.strip()]
files = ['manifest.json', 'jobs.tikeo.json', 'jobs.tikeo.md', 'data-import-plan.json', 'CHECKLIST.md', 'java-project-plan.json', 'java-project-plan.md']
checks = []
def add(name, passed, detail, value=None):
    checks.append({'name': name, 'passed': bool(passed), 'detail': detail, 'value': value})

add('bundle files complete', all((bundle / name).exists() for name in files), ', '.join(files))
add('legacy DB auto-export captured', str(manifest).find('legacy-db:sqlite:') >= 0 and manifest.get('source') == 'xxl-job', manifest.get('source'))
add('job plan generated', jobs.get('summary', {}).get('total') == 2 and len(jobs.get('jobs', [])) == 2, jobs.get('summary'))
add('java project scanned', 'tikeo-spring-boot3-starter' in json.dumps(java_plan) and 'billingProcessor' in json.dumps(java_plan), java_plan.get('dependencyRecommendations') or java_plan.get('dependencies'))
add('data import split ready/review/skipped', 'ready' in data_import and len(data_import.get('needsReview', [])) >= 1, {k: len(v) if isinstance(v, list) else v for k, v in data_import.items()})
add('dry-run evidence planned after review override', dry.get('dryRun') is True and all(r.get('status') == 'planned' for r in dry.get('requests', [])) and len(dry.get('requests', [])) >= 1, dry)
add('live apply posted reviewed jobs', live.get('dryRun') is False and len(requests) == len(live.get('requests', [])) >= 1 and all(r.get('httpStatus') == 201 for r in live.get('requests', [])), {'requests': len(requests), 'live': live})
add('live apply used API key and /api/v1/jobs', all(r.get('path') == '/api/v1/jobs' and r.get('apiKey') == 'live-smoke-key' for r in requests), requests)
add('processor names preserved', any('billingProcessor' in json.dumps(r.get('body')) for r in requests), requests)
score = round(sum(1 for c in checks if c['passed']) / len(checks) * 100, 2)
summary = {
    'status': 'passed' if all(c['passed'] for c in checks) else 'failed',
    'score': score,
    'scope': 'local full-chain tikeo-migrate rehearsal from throwaway legacy Spring Boot project to mock Management API; needs_review jobs are applied with explicit --include-needs-review to model operator approval',
    'metrics': {
        'plannedJobs': jobs.get('summary', {}).get('total'),
        'readyJobs': len(data_import.get('ready', [])),
        'needsReviewJobs': len(data_import.get('needsReview', [])),
        'reviewOverrideUsed': True,
        'liveApiRequests': len(requests),
        'bundleFiles': len([name for name in files if (bundle / name).exists()])
    },
    'checks': checks,
}
(report / 'summary.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding='utf-8')
md = ['# Migration CLI full-chain smoke report', '', f"- Status: `{summary['status']}`", f"- Score: `{score}`", f"- Scope: {summary['scope']}", '', '## Metrics', '']
for key, value in summary['metrics'].items():
    md.append(f'- {key}: `{value}`')
md += ['', '## Checks', '', '| Check | Pass | Detail |', '|---|---:|---|']
for c in checks:
    md.append(f"| {c['name']} | {'✅' if c['passed'] else '❌'} | {str(c['detail']).replace('|', '\\|')[:260]} |")
md += ['', '## Evidence files', '', f'- Legacy project: `{report / "legacy-xxl-worker"}`', f'- Bundle: `{bundle}`', f'- Dry-run evidence: `{report / "apply-dry-run-evidence.json"}`', f'- Live apply evidence: `{report / "apply-live-evidence.json"}`', f'- Mock Management API requests: `{report / "management-api-requests.jsonl"}`']
(report / 'REPORT.md').write_text('\n'.join(md) + '\n', encoding='utf-8')
print(json.dumps(summary, ensure_ascii=False, indent=2))
if summary['status'] != 'passed':
    raise SystemExit(1)
PY
}

need_cmd python3
build_migrate_binary
create_legacy_project
run_chain
verify_chain

echo "Migration CLI full-chain smoke passed: $REPORT_MD"
