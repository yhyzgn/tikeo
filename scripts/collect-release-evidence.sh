#!/usr/bin/env bash
set -u -o pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="${TIKEO_RELEASE_EVIDENCE_RUN_ID:-release-evidence-$(date -u +%Y%m%dT%H%M%SZ)}"
REPORT_DIR="${TIKEO_RELEASE_EVIDENCE_DIR:-$ROOT_DIR/.dev/reports/$RUN_ID}"
mkdir -p "$REPORT_DIR/logs"

COMMANDS=()
LABELS=()
REQUIRED=()

add_cmd() {
  LABELS+=("$1")
  COMMANDS+=("$2")
  REQUIRED+=("$3")
}

add_cmd "git-diff-check" "git diff --check" "1"
add_cmd "release-version-script-test" "python3 .github/tests/release_version_script_test.py" "1"
add_cmd "docs-contract-test" "python3 .github/tests/docs_site_contract_test.py" "1"
add_cmd "docs-build" "npm --prefix docs run build" "1"

if [[ "${TIKEO_RELEASE_EVIDENCE_INCLUDE_SOURCE_SIZE:-0}" == "1" ]]; then
  add_cmd "source-size" "python3 scripts/check-source-size.py" "1"
fi
if [[ "${TIKEO_RELEASE_EVIDENCE_INCLUDE_KIND:-0}" == "1" ]]; then
  add_cmd "kind-raft-ha-e2e" "TIKEO_KIND_E2E_KEEP=${TIKEO_KIND_E2E_KEEP:-0} TIKEO_KIND_E2E_REBUILD_SERVER=${TIKEO_KIND_E2E_REBUILD_SERVER:-1} scripts/kind-raft-ha-e2e.sh" "1"
fi
if [[ "${TIKEO_RELEASE_EVIDENCE_INCLUDE_RAFT_WORKER:-0}" == "1" ]]; then
  add_cmd "raft-worker-failover-e2e" "TIKEO_RAFT_WORKER_E2E_KEEP=${TIKEO_RAFT_WORKER_E2E_KEEP:-0} TIKEO_RAFT_WORKER_E2E_REBUILD_SERVER=${TIKEO_RAFT_WORKER_E2E_REBUILD_SERVER:-0} scripts/raft-worker-failover-e2e.sh" "1"
fi
if [[ -n "${TIKEO_SERVER_URL:-}" && -n "${TIKEO_MANAGEMENT_API_KEY:-}" ]]; then
  add_cmd "raft-ha-rollout" "scripts/verify-raft-ha-rollout.sh" "0"
  if [[ "${TIKEO_RELEASE_EVIDENCE_INCLUDE_FAULT_DRY_RUN:-1}" == "1" ]]; then
    add_cmd "raft-ha-fault-dry-run" "scripts/raft-ha-fault-injection-drill.sh" "0"
  fi
fi

printf '{\n  "runId": "%s",\n  "reportDir": "%s",\n  "startedAt": "%s",\n  "commands": [\n' "$RUN_ID" "$REPORT_DIR" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$REPORT_DIR/summary.json"

overall=0
for i in "${!COMMANDS[@]}"; do
  label="${LABELS[$i]}"
  cmd="${COMMANDS[$i]}"
  required="${REQUIRED[$i]}"
  log="$REPORT_DIR/logs/$label.log"
  printf '[release-evidence] running %s\n' "$label"
  started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  (
    cd "$ROOT_DIR" && bash -lc "$cmd"
  ) > "$log" 2>&1
  status=$?
  ended="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  if [[ "$status" -ne 0 && "$required" == "1" ]]; then
    overall=1
  fi
  comma=","; [[ "$i" == "$((${#COMMANDS[@]} - 1))" ]] && comma=""
  printf '    {"label":"%s","command":%s,"required":%s,"status":%s,"startedAt":"%s","endedAt":"%s","log":"logs/%s.log"}%s\n' \
    "$label" "$(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$cmd")" "$required" "$status" "$started" "$ended" "$label" "$comma" >> "$REPORT_DIR/summary.json"
  if [[ "$status" -ne 0 ]]; then
    printf '[release-evidence] %s failed with status %s; see %s\n' "$label" "$status" "$log" >&2
  fi
done

cat >> "$REPORT_DIR/summary.json" <<EOF2
  ],
  "finishedAt": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "status": $overall
}
EOF2

git -C "$ROOT_DIR" status --short --branch > "$REPORT_DIR/git-status.txt"
printf '[release-evidence] summary: %s\n' "$REPORT_DIR/summary.json"
exit "$overall"
