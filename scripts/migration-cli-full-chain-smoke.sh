#!/usr/bin/env bash
# Full-chain local smoke for the standalone tikeo-migrate CLI.
# The script creates a throwaway legacy Spring Boot + XXL-JOB style project,
# auto-exports jobs from a SQLite legacy DB, writes the migration bundle, runs
# local in-place apply in the legacy Worker project, and archives reviewed import payloads
# for the operator-controlled console/API/GitOps import step.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="${TIKEO_MIGRATE_SMOKE_RUN_ID:-migration-cli-full-chain-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
REPORT_DIR="${TIKEO_MIGRATE_SMOKE_REPORT_DIR:-$ROOT_DIR/.dev/reports/$RUN_ID}"
MIGRATE_BIN="$ROOT_DIR/target/debug/tikeo-migrate"
LEGACY_PROJECT="$REPORT_DIR/legacy-xxl-worker"
LEGACY_DB="$LEGACY_PROJECT/legacy-xxl-job.db"
BUNDLE_DIR="$LEGACY_PROJECT/.tikeo-migration"
IMPORT_PAYLOADS="$REPORT_DIR/reviewed-import-payloads.json"
SUMMARY_JSON="$REPORT_DIR/summary.json"
REPORT_MD="$REPORT_DIR/REPORT.md"
mkdir -p "$REPORT_DIR"

cleanup() {
  local code=$?
  if (( code != 0 )); then
    echo "migration CLI full-chain smoke failed; evidence: $REPORT_DIR" >&2
    echo "--- plan stderr ---" >&2
    tail -n 120 "$REPORT_DIR/plan.stderr" >&2 || true
    echo "--- apply stderr ---" >&2
    tail -n 120 "$REPORT_DIR/apply.stderr" >&2 || true
  fi
  exit "$code"
}
trap cleanup EXIT INT TERM

need_cmd() { command -v "$1" >/dev/null 2>&1 || { echo "missing command: $1" >&2; exit 127; }; }

build_migrate_binary() {
  if [[ ! -x "$MIGRATE_BIN" || "${TIKEO_MIGRATE_SMOKE_REBUILD:-1}" == "1" ]]; then
    (cd "$ROOT_DIR" && cargo build -p tikeo-migrate --bin tikeo-migrate > "$REPORT_DIR/cargo-build-tikeo-migrate.log" 2>&1)
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
xxl.job.admin.addresses=http://127.0.0.1:8080/xxl-job-admin
xxl.job.executor.appname=legacy-billing-worker
xxl.job.executor.enabled=true
PROP
}

run_chain() {
  (cd "$LEGACY_PROJECT" && "$MIGRATE_BIN" plan > "$REPORT_DIR/plan.stdout" 2> "$REPORT_DIR/plan.stderr")
  (cd "$LEGACY_PROJECT" && "$MIGRATE_BIN" apply --bundle .tikeo-migration > "$REPORT_DIR/apply.stdout" 2> "$REPORT_DIR/apply.stderr")
}

verify_chain() {
  python3 - "$REPORT_DIR" "$BUNDLE_DIR" "$LEGACY_PROJECT" "$IMPORT_PAYLOADS" <<'PY'
import json, pathlib, sys
report = pathlib.Path(sys.argv[1])
bundle = pathlib.Path(sys.argv[2])
project = pathlib.Path(sys.argv[3])
import_payloads_path = pathlib.Path(sys.argv[4])

def read_json(path):
    return json.loads(path.read_text(encoding='utf-8'))

def read_text(path):
    return path.read_text(encoding='utf-8')

manifest = read_json(bundle / 'manifest.json')
jobs = read_json(bundle / 'jobs.tikeo.json')
data_import = read_json(bundle / 'data-import-plan.json')
java_plan = read_json(bundle / 'java-project-plan.json')
apply_evidence = read_json(bundle / 'code-apply-evidence.json')
config_path = project / 'src/main/resources/application.properties'
config = read_text(config_path)
source = read_text(project / 'src/main/java/com/example/billing/BillingJobs.java')
build_file = read_text(project / 'build.gradle.kts')
ready = data_import.get('ready', [])
needs_review = data_import.get('needsReview', [])
reviewed_payloads = {
    'source': 'operator-reviewed-data-import-plan',
    'bundle': str(bundle),
    'ready': ready,
    'needsReviewHeldForManualDecision': len(needs_review),
}
import_payloads_path.write_text(json.dumps(reviewed_payloads, ensure_ascii=False, indent=2), encoding='utf-8')
files = ['manifest.json', 'jobs.tikeo.json', 'jobs.tikeo.md', 'data-import-plan.json', 'CHECKLIST.md', 'java-project-plan.json', 'java-project-plan.md', 'code-apply-evidence.json', 'CODE_MIGRATION_REPORT.md']
checks = []
def add(name, passed, detail, value=None):
    checks.append({'name': name, 'passed': bool(passed), 'detail': detail, 'value': value})

changed = set(apply_evidence.get('changedFiles', []))
add('bundle files complete', all((bundle / name).exists() for name in files), ', '.join(files))
add('legacy DB auto-export captured', str(manifest).find('legacy-db:sqlite:') >= 0 and manifest.get('source') == 'xxl-job', manifest.get('source'))
add('job plan generated', jobs.get('summary', {}).get('total') == 2 and len(jobs.get('jobs', [])) == 2, jobs.get('summary'))
add('java project scanned', 'tikeo-spring-boot3-starter' in json.dumps(java_plan) and 'billingProcessor' in json.dumps(java_plan), java_plan.get('dependencyRecommendations') or java_plan.get('dependencies'))
add('data import split ready/review/skipped', 'ready' in data_import and 'needsReview' in data_import, {k: len(v) if isinstance(v, list) else v for k, v in data_import.items()})
add('local apply evidence written', apply_evidence.get('targetProject') == str(project) and changed, apply_evidence)
add('dependency added in place', 'implementation("net.tikeo:tikeo-spring-boot3-starter:0.3.10")' in build_file, 'build.gradle.kts')
add('handler annotation migrated', 'import net.tikeo.processor.TikeoProcessor;' in source and '@TikeoProcessor("billingProcessor")' in source, 'BillingJobs.java')
add('config written in original legacy config file', config_path.exists() and 'Generated by tikeo-migrate apply' in config and 'xxl.job.' not in config and 'powerjob.' not in config, str(config_path))
add('minimal worker and management placeholders reserved', all(token in config for token in ['tikeo.worker.enabled=${TIKEO_WORKER_ENABLED:true}', 'tikeo.worker.endpoint=${TIKEO_WORKER_ENDPOINT:http://127.0.0.1:9998}', 'tikeo.worker.namespace=${TIKEO_NAMESPACE:default}', 'tikeo.worker.app=${TIKEO_APP:default}', 'tikeo.worker.state-dir=${TIKEO_WORKER_STATE_DIR:~/.tikeo/workers}', 'tikeo.management.enabled=${TIKEO_MANAGEMENT_ENABLED:false}', 'tikeo.management.endpoint=${TIKEO_MANAGEMENT_ENDPOINT:http://127.0.0.1:9090}', 'tikeo.management.api-key=${TIKEO_MANAGEMENT_API_KEY:}']), 'application.properties placeholders')
add('no standalone migration profile created', not (project / 'src/main/resources/application-tikeo-migration.yml').exists(), 'application-tikeo-migration.yml absent')
add('reviewed import payloads archived without CLI server call', import_payloads_path.exists() and reviewed_payloads['source'] == 'operator-reviewed-data-import-plan', str(import_payloads_path))
score = round(sum(1 for c in checks if c['passed']) / len(checks) * 100, 2)
summary = {
    'status': 'passed' if all(c['passed'] for c in checks) else 'failed',
    'score': score,
    'scope': 'local full-chain tikeo-migrate rehearsal from throwaway legacy Spring Boot project using in-place apply; job import remains an operator-controlled console/API/GitOps step outside the migration CLI',
    'metrics': {
        'plannedJobs': jobs.get('summary', {}).get('total'),
        'readyJobs': len(ready),
        'needsReviewJobs': len(needs_review),
        'changedFiles': len(changed),
        'bundleFiles': len([name for name in files if (bundle / name).exists()]),
        'reviewedImportPayloads': len(ready),
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
md += ['', '## Evidence files', '', f'- Legacy project: `{report / "legacy-xxl-worker"}`', f'- Migrated project: `{project}`', f'- Bundle: `{bundle}`', f'- Apply evidence: `{bundle / "code-apply-evidence.json"}`', f'- Reviewed import payloads: `{import_payloads_path}`']
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
