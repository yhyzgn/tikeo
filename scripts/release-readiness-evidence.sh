#!/usr/bin/env bash
# Aggregate release-readiness evidence for handoff/release items:
# 1) Notification Center provider e2e
# 2) Real Notification Center provider acceptance boundary/probe
# 3) Migration CLI full-chain rehearsal
# 4) Real cloud HA read-only probe when a cloud endpoint is supplied; otherwise
#    an explicit deferred cloud-boundary report tied to existing Kind evidence.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="${TIKEO_RELEASE_EVIDENCE_RUN_ID:-release-readiness-evidence-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
REPORT_DIR="${TIKEO_RELEASE_EVIDENCE_REPORT_DIR:-$ROOT_DIR/.dev/reports/$RUN_ID}"
SUMMARY_JSON="$REPORT_DIR/summary.json"
REPORT_MD="$REPORT_DIR/REPORT.md"
mkdir -p "$REPORT_DIR"

run_notification() {
  local out="$REPORT_DIR/notification-provider-e2e"
  TIKEO_NOTIFICATION_E2E_REPORT_DIR="$out" "$ROOT_DIR/scripts/notification-provider-e2e-smoke.sh" > "$REPORT_DIR/notification-provider-e2e.stdout" 2> "$REPORT_DIR/notification-provider-e2e.stderr"
}

run_notification_real_provider() {
  local out="$REPORT_DIR/notification-real-provider"
  TIKEO_NOTIFICATION_REAL_REPORT_DIR="$out" "$ROOT_DIR/scripts/notification-real-provider-acceptance.sh" > "$REPORT_DIR/notification-real-provider.stdout" 2> "$REPORT_DIR/notification-real-provider.stderr"
}

run_migration() {
  local out="$REPORT_DIR/migration-cli-full-chain"
  TIKEO_MIGRATE_SMOKE_REPORT_DIR="$out" "$ROOT_DIR/scripts/migration-cli-full-chain-smoke.sh" > "$REPORT_DIR/migration-cli-full-chain.stdout" 2> "$REPORT_DIR/migration-cli-full-chain.stderr"
}

run_cloud_or_defer() {
  local out="$REPORT_DIR/cloud-ha-acceptance"
  if [[ -n "${TIKEO_CLOUD_HA_SERVER_URL:-}" || $# -gt 0 ]]; then
    TIKEO_CLOUD_HA_REPORT_DIR="$out" "$ROOT_DIR/scripts/cloud-raft-ha-acceptance.sh" "$@" > "$REPORT_DIR/cloud-ha-acceptance.stdout" 2> "$REPORT_DIR/cloud-ha-acceptance.stderr"
  else
    mkdir -p "$out"
    python3 - "$ROOT_DIR" "$out" <<'PY'
import datetime, json, pathlib, sys
root = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
kind_report = root / 'design/reports/kind-raft-ha-e2e-20260622.md'
kind_bundle = root / '.dev/reports/kind-raft-ha-e2e-20260622T040236Z-260434'
summary = {
    'status': 'deferred_cloud_endpoint_missing',
    'score': None,
    'scope': 'real cloud HA acceptance requires TIKEO_CLOUD_HA_SERVER_URL; no destructive cloud action is performed without an explicit target',
    'generatedAt': datetime.datetime.now(datetime.UTC).isoformat(),
    'requiredInputs': {
        'TIKEO_CLOUD_HA_SERVER_URL': 'public/staging Tikeo Server base URL',
        'TIKEO_CLOUD_HA_API_KEY': 'optional API key for protected endpoints',
        'TIKEO_CLOUD_HA_EXPECTED_REPLICAS': 'expected server pod count, default 4',
        'TIKEO_CLOUD_HA_WORKER_TUNNEL_HOST': 'optional Worker Tunnel host for TCP reachability probe',
        'TIKEO_CLOUD_HA_KUBECTL_CONTEXT': 'optional kubectl context for pod/networking evidence'
    },
    'substituteEvidence': {
        'kindReport': str(kind_report),
        'kindReportExists': kind_report.exists(),
        'kindBundle': str(kind_bundle),
        'kindBundleExists': kind_bundle.exists()
    },
    'checks': [
        {'name': 'cloud endpoint supplied', 'passed': False, 'detail': 'TIKEO_CLOUD_HA_SERVER_URL is not set'},
        {'name': 'Kind destructive HA substitute exists', 'passed': kind_report.exists() or kind_bundle.exists(), 'detail': str(kind_report)},
        {'name': 'cloud probe script available', 'passed': (root / 'scripts/cloud-raft-ha-acceptance.sh').exists(), 'detail': 'scripts/cloud-raft-ha-acceptance.sh'}
    ]
}
(out / 'summary.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding='utf-8')
md = ['# Cloud HA acceptance boundary report', '', f"- Status: `{summary['status']}`", '- Real cloud score: `not run`', f"- Scope: {summary['scope']}", '', '## Required inputs for real cloud acceptance', '']
for key, value in summary['requiredInputs'].items():
    md.append(f'- `{key}`: {value}')
md += ['', '## Substitute evidence', '', f"- Kind report exists: `{summary['substituteEvidence']['kindReportExists']}` — `{summary['substituteEvidence']['kindReport']}`", f"- Kind bundle exists: `{summary['substituteEvidence']['kindBundleExists']}` — `{summary['substituteEvidence']['kindBundle']}`", '', '## Checks', '', '| Check | Pass | Detail |', '|---|---:|---|']
for c in summary['checks']:
    md.append(f"| {c['name']} | {'✅' if c['passed'] else '❌'} | {c['detail']} |")
md += ['', '## Operator command', '', '```bash', 'TIKEO_CLOUD_HA_SERVER_URL=https://tikeo.example.com \\', 'TIKEO_CLOUD_HA_EXPECTED_REPLICAS=4 \\', 'TIKEO_CLOUD_HA_WORKER_TUNNEL_HOST=worker-tunnel.example.com \\', './scripts/cloud-raft-ha-acceptance.sh', '```']
(out / 'REPORT.md').write_text('\n'.join(md) + '\n', encoding='utf-8')
print(json.dumps(summary, ensure_ascii=False, indent=2))
PY
    cp "$out/summary.json" "$REPORT_DIR/cloud-ha-acceptance.stdout"
    : > "$REPORT_DIR/cloud-ha-acceptance.stderr"
  fi
}

run_notification
run_notification_real_provider
run_migration
run_cloud_or_defer "$@"

python3 - "$REPORT_DIR" <<'PY'
import json, pathlib, sys
report = pathlib.Path(sys.argv[1])
items = {
    'notificationProviderE2e': report / 'notification-provider-e2e/summary.json',
    'notificationRealProviderAcceptance': report / 'notification-real-provider/summary.json',
    'migrationCliFullChain': report / 'migration-cli-full-chain/summary.json',
    'cloudHaAcceptance': report / 'cloud-ha-acceptance/summary.json',
}
summaries = {key: json.loads(path.read_text(encoding='utf-8')) for key, path in items.items()}
required_passed = summaries['notificationProviderE2e'].get('status') == 'passed' and summaries['migrationCliFullChain'].get('status') == 'passed'
cloud_status = summaries['cloudHaAcceptance'].get('status')
provider_status = summaries['notificationRealProviderAcceptance'].get('status')
external_deferred = {cloud_status, provider_status} & {'deferred_cloud_endpoint_missing', 'deferred_real_provider_inputs_missing'}
if required_passed and cloud_status in {'passed', 'deferred_cloud_endpoint_missing'} and provider_status in {'passed', 'deferred_real_provider_inputs_missing'}:
    status = 'passed_with_external_deferred' if external_deferred else 'passed'
else:
    status = 'failed'
summary = {
    'status': status,
    'items': summaries,
    'evidencePaths': {key: str(path.parent) for key, path in items.items()},
}
(report / 'summary.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding='utf-8')
md = ['# Release readiness follow-up evidence', '', f"- Status: `{status}`", '', '| Item | Status | Score | Evidence |', '|---|---|---:|---|']
for key, value in summaries.items():
    md.append(f"| {key} | `{value.get('status')}` | `{value.get('score')}` | `{items[key].parent}` |")
md += ['', '## Cloud boundary', '', '- Notification Center protocol delivery and migration CLI are fully exercised locally with protocol-real mocks.', '- Real provider notification acceptance remains environment-bound when no `TIKEO_NOTIFICATION_REAL_SERVER_URL` / `TIKEO_NOTIFICATION_REAL_CHANNEL_IDS` are supplied; attach the generated provider boundary report or rerun `scripts/notification-real-provider-acceptance.sh` against staging.', '- Real cloud HA remains environment-bound when no `TIKEO_CLOUD_HA_SERVER_URL` is supplied; attach the generated cloud boundary report and run `scripts/cloud-raft-ha-acceptance.sh` against staging/production when credentials/network are available.']
(report / 'REPORT.md').write_text('\n'.join(md) + '\n', encoding='utf-8')
print(json.dumps(summary, ensure_ascii=False, indent=2))
if status == 'failed':
    raise SystemExit(1)
PY

echo "Release readiness evidence bundle: $REPORT_MD"
