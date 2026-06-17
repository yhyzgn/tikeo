# Production Readiness Follow-up Status

Date: 2026-06-17
Owner: Codex / OMX session
Scope: first follow-up pass after the Raft FSOD Cluster documentation and cluster hardening work.

Legend: ✅ completed and locally verified; 🟡 partially completed / requires environment-specific validation; ⏳ planned backlog.

## Status table

| Item | Status | What changed in this pass | Verification evidence | Remaining boundary |
| --- | --- | --- | --- | --- |
| Version and release example consistency | ✅ | User-facing Docker examples now use `${TIKEO_VERSION}`; Helm examples use `v${TIKEO_VERSION}`; SDK docs describe `${TIKEO_VERSION}` as the release/tag placeholder instead of hardcoding repo development versions; `scripts/set-release-version.py` now syncs `docs/package.json` and release placeholders in README/Helm/docs examples during release workspace versioning. | `.github/tests/release_version_script_test.py` asserts workspace sync updates Cargo/Helm/docs package plus README/Helm/docs placeholders. | `Cargo.toml`, `docs/package.json`, and chart versions remain repo development defaults until release workflows rewrite them from a tag. |
| Alert rule to Notification Center migration state | ✅ | Source inspection confirms alert-rule compatibility is implemented: server startup runs `backfill_alert_rule_notification_policies`, alert create/update calls `ensure_alert_rule_notification_policy_from_channels`, and tests cover idempotent backfill from `alert_rules.channels_json` into `notification_policies(owner_type='alert_rule')`. Design docs were updated from unchecked TODO to implemented compatibility path. | `crates/tikeo-server/src/notification/tests/part_02.rs` covers `alert_rule_create_backfills_notification_policy_for_legacy_channels` and backfill idempotency. | `alert_rules.channels_json` remains a backward-compatible read/write field until a documented breaking release removes or freezes it. |
| Release evidence bundle | ✅ | Added `scripts/collect-release-evidence.sh` to run/check the non-mutating release gate set and write a timestamped evidence directory with `summary.json`, command logs, `git-status.txt`, and command statuses. | Script supports default quick gate and opt-in heavier gates through env flags. | Live Kind, live Raft, and cloud LB/WAF/TLS checks are intentionally opt-in because they need Docker/Kubernetes/cloud credentials and can be expensive. |
| Kind 4-Pod HA local Kubernetes validation | ✅ | Ran the local Kind harness with four Server pods, external PostgreSQL inside Kind, API traffic pinned to a non-Leader pod, Worker Tunnel pinned to another non-Leader pod, leader-pod deletion, and before/after job execution. | `.dev/reports/kind-raft-ha-e2e-20260617T072226Z-4055720/kind-raft-ha-e2e-20260617T072226Z-4055720.json` reports `status=passed`, `leaderBefore=tikeo-server-1`, `leaderAfter=tikeo-server-3`, `apiPod=tikeo-server-0`, `workerGatewayPod=tikeo-server-2`. | Kind proves Kubernetes object semantics on one machine; it does not prove cloud LB/WAF/TLS/multi-zone behavior. |
| Real cloud production HA validation | 🟡 | Documentation and scripts name the required checks; local Kind evidence is now available, but cloud-specific infrastructure still needs a staging/prod-like run. | `scripts/verify-raft-ha-rollout.sh`, `scripts/raft-ha-fault-injection-drill.sh`, and the Kind evidence above. | Must still be run in the actual target environment: ingress/LB/WAF/TLS, HTTP/2 Worker Tunnel, SSE, external DB HA, multi-zone failure. |
| Security policy center / policy engine | ⏳ | No product implementation in this pass. Existing script governance, grants, signing, and route placeholder remain separate from a full policy-center UX. | Existing `web/src/routes.tsx` still keeps `securityNext` disabled. | Needs separate design/implementation pass for OPA/Rego or built-in DSL, UI, RBAC, audit, and runtime enforcement map. |
| Workflow replay/canary/Smart Gateway/migration tools | ⏳ | No implementation in this pass. | Existing docs/code already expose replay bundle, canary foundation, FSOD routing, and migration backlog. | Remain next-wave enhancements after production-readiness P0/P1. |

## Recommended next execution order

1. Run the quick release evidence bundle locally on every release candidate.
2. Re-run Kind 4-Pod HA evidence before claiming Kubernetes semantics for a new release candidate.
3. Run `verify-raft-ha-rollout.sh` and a dry-run fault drill against staging/prod-like clusters.
4. Plan the Security Policy Center as a separate vertical slice; do not mix it with release hygiene.
5. Keep migration tools as backlog until service operation remains stable across several releases.

## Latest verification

2026-06-17 local evidence:

- ✅ `scripts/collect-release-evidence.sh` → `.dev/reports/release-evidence-20260617T071606Z/summary.json`
- ✅ `python3 .github/tests/release_version_script_test.py`
- ✅ `python3 .github/tests/docs_site_contract_test.py`
- ✅ `git diff --check`
- ✅ `npm --prefix docs run build` through the release evidence script
- ✅ `cargo test -p tikeo-server alert_rule_create_backfills_notification_policy_for_legacy_channels -- --nocapture`
- ✅ `cargo test -p tikeo-server alert_rule_legacy_backfill_migrates_all_existing_rules_idempotently -- --nocapture`
- ✅ `cargo test -p tikeo-server governance_alert_materialization_backfills_legacy_policy_and_attempt -- --nocapture`

- ✅ `TIKEO_KIND_E2E_KEEP=0 TIKEO_KIND_E2E_REBUILD_SERVER=0 scripts/kind-raft-ha-e2e.sh` → `.dev/reports/kind-raft-ha-e2e-20260617T072226Z-4055720/kind-raft-ha-e2e-20260617T072226Z-4055720.json`

Intentional non-claims:

- Live Raft rollout checks, cloud ingress/LB/WAF/TLS validation, multi-zone failure, and real external DB HA were not rerun in this pass. They remain staging/production evidence gates.
