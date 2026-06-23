#!/usr/bin/env bash
# Real Notification Center provider acceptance probe.
# This script is safe by default: without a staging Server URL and saved channel ids it writes
# an explicit deferred report. When inputs are supplied, it sends one test notification through
# each saved channel row and archives the redacted provider response evidence.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="${TIKEO_NOTIFICATION_REAL_RUN_ID:-notification-real-provider-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
REPORT_DIR="${TIKEO_NOTIFICATION_REAL_REPORT_DIR:-$ROOT_DIR/.dev/reports/$RUN_ID}"
SERVER_URL="${TIKEO_NOTIFICATION_REAL_SERVER_URL:-${TIKEO_HTTP_URL:-}}"
CHANNEL_IDS="${TIKEO_NOTIFICATION_REAL_CHANNEL_IDS:-}"
API_KEY="${TIKEO_NOTIFICATION_REAL_API_KEY:-${TIKEO_API_KEY:-}}"
BEARER_TOKEN="${TIKEO_NOTIFICATION_REAL_BEARER_TOKEN:-${TIKEO_AUTH_TOKEN:-}}"
TIMEOUT_SECONDS="${TIKEO_NOTIFICATION_REAL_TIMEOUT_SECONDS:-15}"
SUMMARY_JSON="$REPORT_DIR/summary.json"
REPORT_MD="$REPORT_DIR/REPORT.md"
mkdir -p "$REPORT_DIR"

write_deferred() {
  local reason="$1"
  python3 - "$REPORT_DIR" "$reason" <<'PY'
import datetime, json, pathlib, sys
out = pathlib.Path(sys.argv[1])
reason = sys.argv[2]
summary = {
    'status': 'deferred_real_provider_inputs_missing',
    'score': None,
    'scope': 'real SaaS/provider acceptance requires a reachable staging Tikeo Server and saved channel ids; no external notification is sent without explicit inputs',
    'generatedAt': datetime.datetime.now(datetime.UTC).isoformat(),
    'reason': reason,
    'requiredInputs': {
        'TIKEO_NOTIFICATION_REAL_SERVER_URL': 'staging/production Tikeo Server base URL',
        'TIKEO_NOTIFICATION_REAL_CHANNEL_IDS': 'comma-separated saved notification channel ids to test-send',
        'TIKEO_NOTIFICATION_REAL_API_KEY or TIKEO_NOTIFICATION_REAL_BEARER_TOKEN': 'credential with notifications:test permission',
    },
    'checks': [
        {'name': 'server url supplied', 'passed': bool('server' not in reason), 'detail': reason},
        {'name': 'channel ids supplied', 'passed': bool('channel' not in reason), 'detail': reason},
    ],
}
(out / 'summary.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding='utf-8')
md = ['# Real Notification Center provider acceptance boundary', '', f"- Status: `{summary['status']}`", f"- Scope: {summary['scope']}", f"- Reason: {reason}", '', '## Required inputs', '']
for key, value in summary['requiredInputs'].items():
    md.append(f'- `{key}`: {value}')
md += ['', '## Operator command', '', '```bash', 'TIKEO_NOTIFICATION_REAL_SERVER_URL=https://tikeo.example.com \\', 'TIKEO_NOTIFICATION_REAL_CHANNEL_IDS=channel_feishu,channel_email \\', 'TIKEO_NOTIFICATION_REAL_API_KEY=*** \\', './scripts/notification-real-provider-acceptance.sh', '```']
(out / 'REPORT.md').write_text('\n'.join(md) + '\n', encoding='utf-8')
print(json.dumps(summary, ensure_ascii=False, indent=2))
PY
}

if [[ -z "$SERVER_URL" && -z "$CHANNEL_IDS" ]]; then
  write_deferred "server url and channel ids are not set"
  exit 0
elif [[ -z "$SERVER_URL" ]]; then
  write_deferred "server url is not set"
  exit 0
elif [[ -z "$CHANNEL_IDS" ]]; then
  write_deferred "channel ids are not set"
  exit 0
fi

SERVER_URL="${SERVER_URL%/}"
AUTH_ARGS=()
if [[ -n "$API_KEY" ]]; then
  AUTH_ARGS=(-H "x-tikeo-api-key: $API_KEY")
elif [[ -n "$BEARER_TOKEN" ]]; then
  AUTH_ARGS=(-H "authorization: Bearer $BEARER_TOKEN")
fi

IFS=',' read -r -a IDS <<< "$CHANNEL_IDS"
: > "$REPORT_DIR/channel-results.jsonl"
index=0
for raw in "${IDS[@]}"; do
  channel_id="$(echo "$raw" | xargs)"
  [[ -n "$channel_id" ]] || continue
  index=$((index + 1))
  payload="$REPORT_DIR/test-payload-$index.json"
  response="$REPORT_DIR/test-response-$index.json"
  python3 - "$RUN_ID" "$channel_id" > "$payload" <<'PY'
import json, sys
run_id, channel_id = sys.argv[1:3]
print(json.dumps({
    'subject': f'Real provider acceptance {run_id}',
    'body': f'Tikeo real provider acceptance test for channel {channel_id}. If you received this, record the report directory as release evidence.',
    'severity': 'info',
    'eventType': 'notification.real_provider_acceptance',
    'resourceType': 'notification_channel',
    'resourceId': channel_id,
    'payload': {'runId': run_id, 'channelId': channel_id}
}, ensure_ascii=False, separators=(',', ':')))
PY
  status=0
  curl -fsS --max-time "$TIMEOUT_SECONDS" \
    -H 'content-type: application/json' "${AUTH_ARGS[@]}" \
    -X POST "$SERVER_URL/api/v1/notification-channels/$channel_id/test-send" \
    --data-binary "@$payload" -o "$response" || status=$?
  python3 - "$channel_id" "$status" "$response" >> "$REPORT_DIR/channel-results.jsonl" <<'PY'
import json, pathlib, sys
channel_id, status, response = sys.argv[1], int(sys.argv[2]), pathlib.Path(sys.argv[3])
record = {'channelId': channel_id, 'curlStatus': status, 'responseFile': str(response)}
try:
    payload = json.loads(response.read_text(encoding='utf-8'))
    record['apiCode'] = payload.get('code')
    data = payload.get('data') or {}
    record['delivered'] = data.get('delivered')
    record['retryState'] = data.get('retryState')
    record['statusCode'] = data.get('statusCode')
    record['targetRedacted'] = data.get('targetRedacted')
    record['messageId'] = data.get('messageId')
    record['attemptId'] = data.get('attemptId')
    record['error'] = data.get('error') or payload.get('message')
except Exception as exc:
    record['parseError'] = str(exc)
print(json.dumps(record, ensure_ascii=False))
PY
done

python3 - "$REPORT_DIR" <<'PY'
import json, pathlib, sys
report = pathlib.Path(sys.argv[1])
records = [json.loads(line) for line in (report / 'channel-results.jsonl').read_text(encoding='utf-8').splitlines() if line.strip()]
checks = []
def add(name, passed, detail, value=None):
    checks.append({'name': name, 'passed': bool(passed), 'detail': detail, 'value': value})
add('at least one channel tested', len(records) > 0, f'channels={len(records)}', len(records))
for item in records:
    add(f"{item.get('channelId')} delivered", item.get('curlStatus') == 0 and item.get('apiCode') == 0 and item.get('delivered') is True and item.get('retryState') == 'delivered', json.dumps(item, ensure_ascii=False))
score = round(sum(1 for c in checks if c['passed']) / len(checks) * 100, 2) if checks else 0
summary = {
    'status': 'passed' if checks and all(c['passed'] for c in checks) else 'failed',
    'score': score,
    'scope': 'real saved Notification Center channel test-send against operator supplied staging/production providers',
    'channelsTested': len(records),
    'delivered': sum(1 for item in records if item.get('delivered') is True),
    'failed': sum(1 for item in records if item.get('delivered') is not True),
    'checks': checks,
    'records': records,
}
(report / 'summary.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding='utf-8')
md = ['# Real Notification Center provider acceptance report', '', f"- Status: `{summary['status']}`", f"- Score: `{score}`", f"- Channels tested: `{summary['channelsTested']}`", f"- Delivered / failed: `{summary['delivered']} / {summary['failed']}`", '', '| Channel | Delivered | Retry state | Status code | Target | Evidence |', '|---|---:|---|---:|---|---|']
for item in records:
    target = str(item.get('targetRedacted') or '').replace('|', '\\|')
    md.append(f"| `{item.get('channelId')}` | {'✅' if item.get('delivered') is True else '❌'} | `{item.get('retryState')}` | `{item.get('statusCode')}` | {target} | `{item.get('responseFile')}` |")
md += ['', '## Boundary', '', '- This report is real-provider evidence only when operator-supplied channel rows point to real tenant endpoints and network egress is available.', '- The script stores redacted Tikeo responses and does not print provider secrets.']
(report / 'REPORT.md').write_text('\n'.join(md) + '\n', encoding='utf-8')
print(json.dumps(summary, ensure_ascii=False, indent=2))
if summary['status'] != 'passed':
    raise SystemExit(1)
PY

echo "Real notification provider acceptance report: $REPORT_MD"
