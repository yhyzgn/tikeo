---
title: Product readiness acceptance checklist
description: Acceptance and release-readiness checklist for Notification Center, tikeo-migrate, and Raft FSOD Server HA.
keywords: [tikeo acceptance, notification center, tikeo migrate, raft fsod, release readiness]
---

# Product readiness acceptance checklist

Use this page when a maintainer, release operator, or handover owner needs to answer a practical question: can the latest Notification Center, legacy migration CLI, and Raft FSOD Server HA work be accepted as product-ready for the next development owner? It does not replace the feature manuals. It is the cross-feature gate list that points to the canonical pages, evidence commands, and known production risks.

## Scope and status

| Area | Current readiness | Canonical operator docs | Primary evidence |
| --- | --- | --- | --- |
| Notification Center | Ready for local/staging acceptance when at least one real provider per active channel family is tested and delivery queue evidence is archived. | [Notifications](../user-guide/notifications), [Notification Center reference](../reference/notification-center), [Configuration reference](../reference/configuration#notification-center-delivery) | Channel test-send result, policy materialization, `notification_delivery_attempts`, message trace, redacted API response. |
| Legacy scheduler migration CLI | Ready as a review-first migration assistant. `plan` is non-mutating; `apply` is local-only and rewrites an isolated Worker copy; staging import is a separate reviewed console/API/GitOps step. | [Migrate from legacy schedulers](../integrations/migrating-from-legacy-schedulers) | `.tikeo-migration/manifest.json`, `jobs.tikeo.md`, `data-import-plan.json`, `CHECKLIST.md`, `code-apply-evidence.json`, reviewed import evidence. |
| Raft FSOD Server HA | Ready for local Kind and staging acceptance with external DB, stable StatefulSet identity, Raft transport token, and Worker Tunnel gRPC/HTTP2 path. | [Server HA and Raft FSOD Cluster](../deployment/server-ha), [Kubernetes and Helm](../deployment/kubernetes), [Production deployment](../deployment/production) | `scripts/verify-raft-ha-rollout.sh`, `scripts/kind-raft-ha-e2e.sh`, `scripts/cloud-raft-ha-acceptance.sh`, cluster diagnostics, FSOD DB snapshots, before/after failover instance results. |
| Cross-language Worker soak | Optional release-candidate runtime gate for repeated Go/Rust/Python/Node dispatch, task logs, and queue/outbox metrics. | [SDK and API integration](../integrations/sdk-and-api), [Workers guide](../user-guide/workers) | Manual workflow `.github/workflows/release-candidate-worker-soak.yml`, `TIKEO_CROSS_SOAK_SECONDS=120 deploy/smoke/cross-language-worker-parity-smoke.sh`, `cross-language-worker-soak` artifact, `*-soak-summary.json`, `*-soak-summary.csv`, `*-soak-metrics.jsonl`. |

The stop condition is evidence, not wording: each accepted area must have a reproducible command or UI action, the route/file inspected, the observed pass/fail result, and the evidence directory or artifact path.

For an all-in-one local handoff packet, run:

```bash
./scripts/release-readiness-evidence.sh
```

The wrapper writes `.dev/reports/release-readiness-evidence-*/REPORT.md` plus per-area `summary.json` files. It proves Notification Center delivery through a protocol-real loopback provider, rehearses a full `tikeo-migrate` legacy-project chain into an isolated migrated Worker copy, and either runs the real cloud HA probe or records why cloud HA is deferred until `TIKEO_CLOUD_HA_SERVER_URL` is available.

## Notification Center acceptance

Acceptance proves that channel configuration, template rendering, policy materialization, retry/DLQ behavior, and redaction all work together.

| Gate | Acceptance check | Evidence to keep |
| --- | --- | --- |
| Channel configuration | Create or edit a channel from the Web drawer for each provider family used in the environment: webhook-compatible, chat robot, PagerDuty, and email. Provider secrets such as webhook URL, signing secret, routing key, SMTP host/port/user/password/from are configured on the channel row and take effect without restart. | Screenshot or API response with `targetConfigured=true`, redacted `targetRedacted`, and no raw secret in `configJson`. |
| Test button | Use the list-row **Test** action and the drawer **Test** action. Success shows provider response detail; failure shows HTTP/status/error body instead of an empty JSON parse error. | Test response JSON or UI detail panel for at least one success and one safe failure case. |
| Template coverage | Validate at least one provider-specific non-text template when supported: Slack Block Kit, DingTalk action/feed card, Feishu interactive card, WeCom template card, PagerDuty incident, webhook JSON body, and email subject/body. | Render preview from `/api/v1/notification-templates/{id}/render` and delivered provider payload trace. |
| Policy materialization | Bind a policy to a job instance event and trigger a job that reaches the selected status. | `notification_messages` row or API summary with `policyId`, `eventType`, `resourceId`, and `payload.template` when a template is used. |
| Delivery queue | Confirm delivered, retry, and dead-letter paths are visible. | Notification Center delivery tab, `notification-delivery-attempts:queue-status`, or DB snapshot with `retry_pending`, `retry_consumed`, `dead_letter`, and delivered rows. |
| Redaction | Fetch channel summaries and message traces. | Raw webhook URL, signing secret, SMTP password, routing key, auth headers, and URL path/query are absent from responses/logs. |

Suggested local verification commands:

```bash
./scripts/notification-provider-e2e-smoke.sh
python3 .github/tests/docs_site_contract_test.py
python3 .github/tests/demo_seed_topology_contract_test.py
cargo test -p tikeo-server notification --all-features
```

`notification-provider-e2e-smoke.sh` starts a local Server and mock HTTP provider, sends one successful test notification and one forced provider failure, then verifies provider receipt, `notification_messages`, delivery attempts, queue aggregates, dead-letter state, and target redaction. It is local protocol evidence, not a substitute for tenant-specific Slack/Feishu/DingTalk/WeCom/PagerDuty/SMTP sign-off.

If a real provider cannot be called from the environment, mark only the provider-delivery gate as deferred and keep render, validation, redaction, and queue evidence. Do not claim production readiness for that provider until a real outbound call has been observed.

## Migration CLI acceptance

`tikeo-migrate` is intentionally conservative. It should reduce migration effort while forcing humans to review semantic gaps before data is imported or code is changed.

| Gate | Acceptance check | Evidence to keep |
| --- | --- | --- |
| Auto-detection | Run `tikeo-migrate plan` from a legacy Java/Spring worker root. It detects XXL-JOB or PowerJob dependencies/source, Spring Boot major version, datasource settings when present, and scheduler tables when reachable. | CLI output plus `.tikeo-migration/manifest.json` with detected framework, DB source, and Java project plan. |
| Non-mutating plan | Confirm `plan` does not edit legacy source and does not call Tikeo Server. | Clean `git diff` before/after `plan`, and bundle files only under `.tikeo-migration/` or the selected output directory. |
| Data review | Review generated job drafts and semantic warnings. | `jobs.tikeo.md`, `data-import-plan.json`, counts for `ready`, `needs_review`, and `skipped`. |
| Code migration guidance | Review generated Java dependency/processor patch guidance. | `java-project-plan.md`, `.json`, and `java-patches/*.patch` on a migration branch. |
| Local apply | Run `tikeo-migrate apply --bundle ./.tikeo-migration --output-project ../legacy-worker-tikeo`; compile/test the migrated copy and inspect generated config placeholders. | `code-apply-evidence.json`, `CODE_MIGRATION_REPORT.md`, migrated source diff, and original legacy scheduler config file with appended Tikeo placeholders. |
| Live staging import | Import only reviewed `ready` jobs into staging, start matching Tikeo Workers, and trigger at least one migrated job. | Tikeo job ids, instance result/logs, Worker processor name, and comparison with legacy behavior. |
| Release artifacts | Confirm release includes platform-specific `tikeo-migrate` archives and checksums. | GitHub Release asset list for Linux, macOS Intel, macOS Apple Silicon, and Windows. |

Suggested verification commands:

```bash
./scripts/migration-cli-full-chain-smoke.sh
cargo test -p tikeo-migrate
python3 .github/tests/workflow_contract_test.py
python3 scripts/check-source-size.py
```

`migration-cli-full-chain-smoke.sh` creates a throwaway legacy Spring Boot + XXL-JOB project, populates a local legacy scheduler DB, runs zero-parameter `tikeo-migrate plan`, verifies the generated bundle, runs local `apply` into an isolated Worker copy, checks in-place config placeholders, and archives `reviewed-import-payloads.json`.

A migration is not accepted when all jobs are merely imported. It is accepted when unsupported legacy semantics have explicit decisions, imported jobs can be triggered in staging, and the legacy scheduler can be disabled according to a rollback-aware cutover plan.

## Raft FSOD Server HA acceptance

Server HA acceptance proves the system is not hiding correctness in pod memory. API/Web traffic may land on different pods, Workers may connect to a different gateway pod, and dispatch still goes through Raft fencing, shard ownership, durable outbox rows, and assignment-token validation.

| Gate | Acceptance check | Evidence to keep |
| --- | --- | --- |
| Deployment shape | Multi-pod Server uses StatefulSet, headless peer DNS, external DB, identical shard config, and a Raft transport token. | Rendered Helm manifest or `kubectl get statefulset,svc,secret`. |
| Scheduler fencing | Exactly one node reports `canSchedule=true`; stale terms/tokens fail closed. | `/api/v1/cluster`, `/api/v1/cluster/diagnostics`, and `scripts/verify-raft-ha-rollout.sh` output. |
| Shard ownership | `cluster_shard_ownership` has active rows with bounded skew and matching map version/count. | DB snapshot, metrics summary, `shardOwnership` diagnostics. |
| Durable dispatch | `worker_dispatch_outbox` records dispatch intent before stream delivery; queued/delivered rows recover after gateway or Worker reconnect. | FSOD DB snapshot before and after failover, outbox metrics, Worker logs. |
| Cross-pod API/Web reads | Business APIs read shared persistent state rather than pod-local memory. | Repeated requests through the Service show consistent job/instance/message state. |
| API pod different from Worker gateway | Trigger a job through pod A while the Worker stream is held by pod B. | Diagnostics showing local/remote Worker counts, gateway node id, and successful instance result. |
| Leader failover | Delete or restart the current Leader; a new Leader projects ownership and a post-failover job completes. | Kind/staging fault-drill report, before/after instance results, Kubernetes events. |
| Network path | Worker Tunnel path supports gRPC/HTTP2, idle timeout, TLS/mTLS, and SSE dashboards are configured separately from gRPC. | Ingress/Gateway/LB config plus [SSE realtime](../deployment/sse-realtime) checks. |

Local Kind acceptance:

```bash
TIKEO_KIND_E2E_KEEP=0 TIKEO_KIND_E2E_REBUILD_SERVER=1 scripts/kind-raft-ha-e2e.sh
```

Staging rollout gate:

```bash
TIKEO_SERVER_URL="https://tikeo.example.com" TIKEO_MANAGEMENT_API_KEY="$TIKEO_MANAGEMENT_API_KEY" TIKEO_EXPECTED_SERVER_REPLICAS=3 TIKEO_MAX_SHARD_SKEW=1 scripts/verify-raft-ha-rollout.sh
```

Read-only cloud acceptance probe:

```bash
TIKEO_CLOUD_HA_SERVER_URL="https://tikeo.example.com" TIKEO_CLOUD_HA_EXPECTED_REPLICAS=4 TIKEO_CLOUD_HA_WORKER_TUNNEL_HOST="worker-tunnel.example.com" scripts/cloud-raft-ha-acceptance.sh
```

Kind validates local Kubernetes semantics. It does not replace cloud-specific acceptance for multi-zone node failure, managed load balancer behavior, WAF/gateway idle timeouts, TLS certificates, or external database HA. When no cloud target is available, `scripts/release-readiness-evidence.sh` records this as a cloud-boundary report instead of silently marking it passed.

## Cross-feature release gate

Before handing the work to another owner or publishing a release, collect one short evidence packet:

| Item | Required artifact |
| --- | --- |
| Version and commit | `git rev-parse HEAD`, release tag, and release asset list. |
| Documentation | README links, docs sidebar entry, docs search/LLM entries, and this checklist. |
| Notification Center | provider test-send evidence, message trace, retry/DLQ snapshot, redaction check. |
| Migration CLI | Sample `.tikeo-migration` bundle, local apply evidence, reviewed import payloads, release assets. |
| HA | Kind or staging HA report directory, rollout gate output, failover instance results. |
| Regression checks | `docs_site_contract_test.py`, relevant Rust/package tests, source-size check, and `git diff --check`. |

## Remaining risks and next work

- Real cloud HA still needs environment-specific proof for ingress class, LB/WAF behavior, TLS/mTLS, network policies, and managed database HA.
- Provider delivery behavior can differ by tenant policy; keep real Slack/DingTalk/Feishu/WeCom/PagerDuty/email evidence per deployment environment.
- Legacy migration semantic equivalence is domain-specific. Route/block/concurrency/script semantics should remain review-required instead of being auto-converted silently.
- Release asset availability should be checked from the actual GitHub Release page after the pipeline uploads artifacts.

## Prerequisites

Use a clean working tree, a running staging or local Server, and credentials stored in environment variables. For HA checks, use an external database and stable Kubernetes identity. For notification checks, use test-safe provider destinations.

## Verify

A valid verification packet includes the command or UI action, inspected route/file, observed status, and artifact path. At minimum run:

```bash
python3 .github/tests/docs_site_contract_test.py
python3 scripts/check-source-size.py
git diff --check
```

## Troubleshooting

If a checklist item fails, do not weaken the checklist. Follow the linked canonical page for the failing area, capture the failing response/log window, fix the code/config/doc drift, and rerun the smallest check that proves the claim.

## Production checklist

- [ ] Each enabled notification provider has a real test-send record or an explicit environment exception.
- [ ] Every migrated legacy job is either accepted, deferred with reason, or recreated manually.
- [ ] Multi-pod Server HA uses Raft FSOD with external DB and stable pod identity, not standalone replicas.
- [ ] Evidence artifacts are stored outside ephemeral shell history.
- [ ] Known environment-specific risks are assigned to an owner before production rollout.
