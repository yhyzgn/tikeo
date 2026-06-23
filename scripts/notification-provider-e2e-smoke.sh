#!/usr/bin/env bash
# Local protocol-real Notification Center provider e2e smoke.
# It starts a local Tikeo server plus an HTTP mock provider, sends a real
# /test-send request, verifies delivery/trace/queue evidence, and records a
# failed-provider dead-letter case. It does not require external SaaS credentials.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="${TIKEO_NOTIFICATION_E2E_RUN_ID:-notification-provider-e2e-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
REPORT_DIR="${TIKEO_NOTIFICATION_E2E_REPORT_DIR:-$ROOT_DIR/.dev/reports/$RUN_ID}"
API_URL="${TIKEO_HTTP_URL:-http://127.0.0.1:19094}"
WORKER_ENDPOINT="${TIKEO_WORKER_ENDPOINT:-http://127.0.0.1:19994}"
SERVER_CONFIG="$REPORT_DIR/$RUN_ID-config.toml"
SERVER_LOG="$REPORT_DIR/$RUN_ID-server.log"
SERVER_BIN="$ROOT_DIR/target/debug/tikeo"
DB_PATH="$REPORT_DIR/$RUN_ID.db"
MOCK_SCRIPT="$REPORT_DIR/mock-provider.py"
MOCK_LOG="$REPORT_DIR/$RUN_ID-mock-provider.log"
MOCK_PORT_FILE="$REPORT_DIR/mock-provider-port.txt"
PROVIDER_RECEIVED="$REPORT_DIR/provider-received.jsonl"
SUMMARY_JSON="$REPORT_DIR/summary.json"
REPORT_MD="$REPORT_DIR/REPORT.md"
mkdir -p "$REPORT_DIR"

export TIKEO_SMOKE_REPORT_DIR="$REPORT_DIR"
export TIKEO_SMOKE_RUN_ID="$RUN_ID"
export TIKEO_SMOKE_CASES_FILE="$REPORT_DIR/$RUN_ID-cases.jsonl"
# shellcheck source=../deploy/smoke/lib/tikeo-smoke-lib.sh
source "$ROOT_DIR/deploy/smoke/lib/tikeo-smoke-lib.sh"
: > "$TIKEO_SMOKE_CASES_FILE"
: > "$SERVER_LOG"
: > "$PROVIDER_RECEIVED"

SERVER_PID=""
MOCK_PID=""

cleanup() {
  local code=$?
  if [[ -n "$MOCK_PID" ]] && kill -0 "$MOCK_PID" >/dev/null 2>&1; then
    kill "$MOCK_PID" >/dev/null 2>&1 || true
    wait "$MOCK_PID" 2>/dev/null || true
  fi
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  if (( code != 0 )); then
    echo "notification provider e2e smoke failed; evidence: $REPORT_DIR" >&2
    echo "--- server log tail ---" >&2
    tail -n 160 "$SERVER_LOG" >&2 || true
    echo "--- mock provider log tail ---" >&2
    tail -n 160 "$MOCK_LOG" >&2 || true
  fi
  exit "$code"
}
trap cleanup EXIT INT TERM

need_cmd() { tikeo_smoke_need_cmd "$1"; }
api() { tikeo_smoke_api "$API_URL" "$@"; }

json_body() {
  python3 - "$@" <<'PY'
import json, sys
pairs = [arg.split('=', 1) for arg in sys.argv[1:]]
print(json.dumps({k: v for k, v in pairs}, ensure_ascii=False, separators=(',', ':')))
PY
}

write_config() {
  cat > "$SERVER_CONFIG" <<CFG
[server]
listen_addr = "${API_URL#http://}"
worker_tunnel_addr = "${WORKER_ENDPOINT#http://}"

[storage]

[storage.database]
type = "sqlite"
path = "$DB_PATH"

[storage.database.params]
mode = "rwc"

[cluster]
mode = "standalone"
node_id = "standalone"
peers = []

[auth]
local_login_enabled = true

[auth.api_tokens]
default_ttl_seconds = 43200
min_ttl_seconds = 300
max_ttl_seconds = 2592000

[auth.oidc]
enabled = false
scopes = ["openid", "profile", "email"]

[transport_security.http]
tls_enabled = false
mtls_required = false

[transport_security.worker_tunnel]
tls_enabled = false
mtls_required = false

[observability.tracing]
enabled = false
headers = []

[alert_retry]
enabled = false
interval_seconds = 60
batch_size = 50
max_attempts = 3
backoff_seconds = 300

[script_governance]
CFG
}

build_server_binary() {
  if [[ ! -x "$SERVER_BIN" || "${TIKEO_NOTIFICATION_E2E_REBUILD_SERVER:-1}" == "1" ]]; then
    (cd "$ROOT_DIR" && cargo build --bin tikeo >>"$SERVER_LOG" 2>&1)
  fi
}

start_server() {
  write_config
  build_server_binary
  (cd "$ROOT_DIR" && exec "$SERVER_BIN" serve --config "$SERVER_CONFIG" >>"$SERVER_LOG" 2>&1) &
  SERVER_PID=$!
  tikeo_smoke_wait_for_http server "$API_URL/readyz" 180 || {
    tail -n 180 "$SERVER_LOG" >&2 || true
    return 1
  }
}

start_mock_provider() {
  cat > "$MOCK_SCRIPT" <<'PY'
import datetime, json, pathlib, sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

out = pathlib.Path(sys.argv[1])
port_file = pathlib.Path(sys.argv[2])

class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get('content-length') or 0)
        body = self.rfile.read(length).decode('utf-8', errors='replace')
        event = {
            'path': self.path,
            'receivedAt': datetime.datetime.now(datetime.UTC).isoformat(),
            'headers': {k: v for k, v in self.headers.items() if k.lower() not in {'authorization'}},
            'body': body,
        }
        with out.open('a', encoding='utf-8') as fh:
            json.dump(event, fh, ensure_ascii=False)
            fh.write('\n')
        if self.path.startswith('/fail'):
            payload = b'{"ok":false,"error":"forced provider failure"}'
            self.send_response(500)
        else:
            payload = b'{"ok":true}'
            self.send_response(202)
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
  python3 "$MOCK_SCRIPT" "$PROVIDER_RECEIVED" "$MOCK_PORT_FILE" >"$MOCK_LOG" 2>&1 &
  MOCK_PID=$!
  local deadline=$((SECONDS + 30))
  until [[ -s "$MOCK_PORT_FILE" ]]; do
    if (( SECONDS >= deadline )); then
      echo "timed out waiting for mock provider port file" >&2
      return 1
    fi
    sleep 0.2
  done
}

create_channel_payload() {
  local name="$1" url="$2" output="$3"
  python3 - "$name" "$url" > "$output" <<'PY'
import json, sys
name, url = sys.argv[1:3]
print(json.dumps({
    'scopeType': 'global',
    'name': name,
    'provider': 'webhook',
    'enabled': True,
    'config': {
        'url': url,
        'messageType': 'json',
        'template': {
            'body': {
                'subject': '{{subject}}',
                'body': '{{body}}',
                'event': '{{eventType}}',
                'severity': '{{severity}}',
                'resourceId': '{{resourceId}}'
            }
        }
    },
    'safetyPolicy': {'allowInsecureLoopback': True}
}, ensure_ascii=False, separators=(',', ':')))
PY
}

test_send_payload() {
  local subject="$1" event_type="$2" resource_id="$3" output="$4"
  python3 - "$subject" "$event_type" "$resource_id" > "$output" <<'PY'
import json, sys
subject, event_type, resource_id = sys.argv[1:4]
print(json.dumps({
    'subject': subject,
    'body': f'body for {subject}',
    'severity': 'info',
    'eventType': event_type,
    'resourceType': 'notification_channel',
    'resourceId': resource_id,
    'payload': {'smokeRunId': 'local-provider-e2e'}
}, ensure_ascii=False, separators=(',', ':')))
PY
}

verify_evidence() {
  python3 - "$REPORT_DIR" <<'PY'
import json, pathlib, sys
report = pathlib.Path(sys.argv[1])

def load(name):
    with (report / name).open(encoding='utf-8') as fh:
        return json.load(fh)

def api_data(name):
    payload = load(name)
    if payload.get('code') != 0:
        raise SystemExit(f'{name} code != 0: {payload}')
    return payload.get('data')

success = api_data('success-test-send.json')
failure = api_data('failure-test-send.json')
messages = api_data('notification-messages.json')
attempts = api_data('notification-delivery-attempts.json')
queue = api_data('queue-status.json')
received = [json.loads(line) for line in (report / 'provider-received.jsonl').read_text(encoding='utf-8').splitlines() if line.strip()]

checks = []
def add(name, passed, detail, value=None):
    checks.append({'name': name, 'passed': bool(passed), 'detail': detail, 'value': value})

add('success test-send delivered', success.get('delivered') is True and success.get('statusCode') == 202 and success.get('retryState') == 'delivered', str(success))
add('success has message and attempt ids', str(success.get('messageId','')).startswith('notification-message_') and str(success.get('attemptId','')).startswith('notification-delivery_'), f"messageId={success.get('messageId')} attemptId={success.get('attemptId')}")
redacted = success.get('targetRedacted') or ''
add('target redacted', redacted.startswith('http://127.0.0.1:') and redacted.endswith('/...') and '/notify' not in json.dumps(success), redacted)
add('provider received rendered payload', any('/notify' in item.get('path','') and 'Smoke success subject' in item.get('body','') for item in received), f'received={len(received)}')
add('failure test-send dead-lettered', failure.get('delivered') is False and failure.get('statusCode') == 500 and failure.get('retryState') == 'dead_letter', str(failure))
add('messages persisted', isinstance(messages, list) and len(messages) >= 2 and {'delivered','dead_letter'}.issubset({m.get('status') for m in messages}), f"statuses={[m.get('status') for m in messages] if isinstance(messages, list) else messages}")
add('attempts persisted', isinstance(attempts, list) and len(attempts) >= 2 and {'delivered','dead_letter'}.issubset({a.get('retryState') for a in attempts}), f"retryStates={[a.get('retryState') for a in attempts] if isinstance(attempts, list) else attempts}")
add('queue status aggregates attempts', (queue.get('totalAttempts') or queue.get('total_attempts') or 0) >= 2 and (queue.get('delivered') or 0) >= 1 and (queue.get('deadLetter') or queue.get('dead_letter') or 0) >= 1, str(queue))
score = round(sum(1 for c in checks if c['passed']) / len(checks) * 100, 2)
summary = {
    'status': 'passed' if all(c['passed'] for c in checks) else 'failed',
    'score': score,
    'scope': 'local protocol-real provider loopback e2e; external SaaS tenant credentials are intentionally not required',
    'checks': checks,
    'metrics': {
        'providerRequestsReceived': len(received),
        'messagesObserved': len(messages) if isinstance(messages, list) else 0,
        'attemptsObserved': len(attempts) if isinstance(attempts, list) else 0,
        'queueTotalAttempts': queue.get('totalAttempts') or queue.get('total_attempts'),
        'queueDelivered': queue.get('delivered'),
        'queueDeadLetter': queue.get('deadLetter') or queue.get('dead_letter')
    }
}
(report / 'summary.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding='utf-8')
md = ['# Notification Center provider e2e smoke report', '', f"- Status: `{summary['status']}`", f"- Score: `{score}`", f"- Scope: {summary['scope']}", '', '## Metrics', '']
for key, value in summary['metrics'].items():
    md.append(f'- {key}: `{value}`')
md += ['', '## Checks', '', '| Check | Pass | Detail |', '|---|---:|---|']
for c in checks:
    detail = str(c['detail']).replace('|', '\\|')[:260]
    md.append(f"| {c['name']} | {'✅' if c['passed'] else '❌'} | {detail} |")
md += ['', '## Production boundary', '', '- This local smoke proves the Tikeo Notification Center delivery state machine, rendered payload, HTTP provider contract, redaction, message persistence, delivery attempts, and dead-letter accounting.', '- Real Slack/Feishu/DingTalk/WeCom/PagerDuty/SMTP tenant sign-off still requires operator-supplied credentials and outbound network access in staging/production.']
(report / 'REPORT.md').write_text('\n'.join(md) + '\n', encoding='utf-8')
print(json.dumps(summary, ensure_ascii=False, indent=2))
if summary['status'] != 'passed':
    raise SystemExit(1)
PY
}

need_cmd curl
need_cmd python3
start_mock_provider
start_server
tikeo_smoke_login "$API_URL"
tikeo_smoke_record_case notification-server-bootstrap passed "$SERVER_CONFIG" "local Tikeo server started and admin token acquired"

MOCK_PORT="$(cat "$MOCK_PORT_FILE")"
SUCCESS_CHANNEL_PAYLOAD="$REPORT_DIR/success-channel-payload.json"
FAILURE_CHANNEL_PAYLOAD="$REPORT_DIR/failure-channel-payload.json"
create_channel_payload "Loopback provider success" "http://127.0.0.1:$MOCK_PORT/notify" "$SUCCESS_CHANNEL_PAYLOAD"
create_channel_payload "Loopback provider failure" "http://127.0.0.1:$MOCK_PORT/fail" "$FAILURE_CHANNEL_PAYLOAD"
api POST /api/v1/notification-channels "$(cat "$SUCCESS_CHANNEL_PAYLOAD")" > "$REPORT_DIR/success-channel.json"
api POST /api/v1/notification-channels "$(cat "$FAILURE_CHANNEL_PAYLOAD")" > "$REPORT_DIR/failure-channel.json"
SUCCESS_CHANNEL_ID="$(tikeo_smoke_json_get data.id < "$REPORT_DIR/success-channel.json")"
FAILURE_CHANNEL_ID="$(tikeo_smoke_json_get data.id < "$REPORT_DIR/failure-channel.json")"
tikeo_smoke_record_case notification-channels-created passed "$REPORT_DIR/success-channel.json $REPORT_DIR/failure-channel.json" "created loopback success and failure webhook channels"

SUCCESS_TEST_PAYLOAD="$REPORT_DIR/success-test-payload.json"
FAILURE_TEST_PAYLOAD="$REPORT_DIR/failure-test-payload.json"
test_send_payload "Smoke success subject" notification.provider_e2e.success "$SUCCESS_CHANNEL_ID" "$SUCCESS_TEST_PAYLOAD"
test_send_payload "Smoke failure subject" notification.provider_e2e.failure "$FAILURE_CHANNEL_ID" "$FAILURE_TEST_PAYLOAD"
api POST "/api/v1/notification-channels/$SUCCESS_CHANNEL_ID/test-send" "$(cat "$SUCCESS_TEST_PAYLOAD")" > "$REPORT_DIR/success-test-send.json"
api POST "/api/v1/notification-channels/$FAILURE_CHANNEL_ID/test-send" "$(cat "$FAILURE_TEST_PAYLOAD")" > "$REPORT_DIR/failure-test-send.json"
tikeo_smoke_record_case notification-test-send passed "$REPORT_DIR/success-test-send.json $REPORT_DIR/failure-test-send.json" "sent success and forced-failure test notifications"

api GET "/api/v1/notification-messages?source_type=channel_test" > "$REPORT_DIR/notification-messages.json"
api GET /api/v1/notification-delivery-attempts > "$REPORT_DIR/notification-delivery-attempts.json"
api GET /api/v1/notification-delivery-attempts:queue-status > "$REPORT_DIR/queue-status.json"
verify_evidence
tikeo_smoke_record_case notification-evidence-verified passed "$SUMMARY_JSON" "verified delivery, dead-letter, redaction, persistence and queue status"
tikeo_smoke_finalize_report "$REPORT_DIR/smoke-cases.json" passed >/dev/null

echo "Notification provider e2e smoke passed: $REPORT_MD"
