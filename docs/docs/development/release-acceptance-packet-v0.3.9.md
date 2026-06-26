---
title: v0.3.9 release acceptance packet
slug: release-acceptance-packet-v0.3.9
description: Evidence packet for the v0.3.9 release plus the immediate post-release cross-language Worker soak gate.
keywords: [tikeo v0.3.9, release acceptance, handoff, raft fsod, worker soak]
---

# v0.3.9 release acceptance packet

Use this packet when taking over development after the `v0.3.9` release. It records what was released, which evidence was collected locally, which GitHub workflows produced artifacts, and which production checks still need a real cloud environment.

## Version boundary

| Item | Value |
| --- | --- |
| Release tag | `v0.3.9` |
| Release page | [v0.3.9 release](https://github.com/yhyzgn/tikeo/releases/tag/v0.3.9) |
| Release state | Published, non-draft, non-prerelease |
| Release asset count observed | `31` uploaded assets |
| Release commit evidence | `ee895ba7 chore: close ha follow-up gates` |
| Latest main follow-up | `c00a0902 ci: add release candidate worker soak gate` |

`affb4605` and `c00a0902` are intentionally listed as post-release follow-ups. They add the reusable release-candidate soak gate, workflow wrapper, and documentation contracts after the `v0.3.9` assets were produced; do not claim those script changes are inside the `v0.3.9` binaries unless a later tag includes them.

## Release assets observed

The `v0.3.9` GitHub Release contained these uploaded asset families at the last handoff check:

| Asset family | Observed files |
| --- | --- |
| SDK archives | `go-sdk-0.3.9.tar.gz`, `java-sdk-0.3.9.tar.gz`, `nodejs-sdk-0.3.9.tar.gz`, `python-sdk-0.3.9.tar.gz`, `rust-sdk-0.3.9.tar.gz` |
| Server binaries | `tikeo-server-0.3.9-aarch64-apple-darwin.tar.gz`, `tikeo-server-0.3.9-x86_64-apple-darwin.tar.gz`, `tikeo-server-0.3.9-x86_64-pc-windows-msvc.zip`, `tikeo-server-0.3.9-x86_64-unknown-linux-gnu.tar.gz` |
| Migration CLI binaries | `tikeo-migrate-0.3.9-aarch64-apple-darwin.tar.gz`, `tikeo-migrate-0.3.9-x86_64-apple-darwin.tar.gz`, `tikeo-migrate-0.3.9-x86_64-pc-windows-msvc.zip`, `tikeo-migrate-0.3.9-x86_64-unknown-linux-gnu.tar.gz` |
| Operator binaries | `tikeo-operator-0.3.9-darwin-amd64.tar.gz`, `tikeo-operator-0.3.9-darwin-arm64.tar.gz`, `tikeo-operator-0.3.9-linux-amd64.tar.gz`, `tikeo-operator-0.3.9-linux-arm64.tar.gz`, `tikeo-operator-0.3.9-windows-amd64.zip` |
| Terraform provider | `terraform-provider-tikeo_v0.3.9_darwin_amd64.tar.gz`, `terraform-provider-tikeo_v0.3.9_darwin_arm64.tar.gz`, `terraform-provider-tikeo_v0.3.9_linux_amd64.tar.gz`, `terraform-provider-tikeo_v0.3.9_linux_arm64.tar.gz`, `terraform-provider-tikeo_v0.3.9_windows_amd64.exe.zip` |
| Deploy and Web bundles | `tikeo-0.3.9.tgz`, `tikeo-deploy-sources-0.3.9.tar.gz`, `tikeo-web-dist-0.3.9.tar.gz`, Compose YAMLs, Kubernetes manifest, CRD manifest |

## Workflow evidence

| Workflow group | Observed result |
| --- | --- |
| Release / GitHub assets for `v0.3.9` | ✅ success |
| Publish / Java SDK | ✅ success |
| Publish / Python SDK | ✅ success |
| Publish / Node.js SDK | ✅ success |
| Publish / Go SDK | ✅ success |
| Publish / Rust SDK | ✅ success |
| Publish / Docker docs | ✅ success |
| Publish / Docker web | ✅ success |
| Publish / Docker server | ✅ success |
| Main `Coverage` for `affb4605` | ✅ success |
| Main `CI` for `c00a0902` | ✅ success |
| Main `Coverage` for `c00a0902` | ✅ success |

Before declaring a later release ready, rerun:

```bash
gh run list --branch main --limit 20
gh release view v0.3.9 --json tagName,url,isDraft,isPrerelease,assets
```

## Local HA evidence

The latest local HA acceptance evidence used a multi-node Kind cluster with required Pod anti-affinity to approximate production failure domains on a single developer machine.

| Signal | Result |
| --- | ---: |
| HA confidence index | `99/100` |
| Server replicas / Kind worker nodes | `4 / 4` |
| Server Pod spread | `4 / 4` distinct Kind worker nodes before and after gateway force-delete |
| Raft shard ownership | `64` active rows, `4` owners, ownership skew `0` in rollout gate |
| Epoch fencing | `100/100`, stale owner token rejection covered by unit evidence plus Leader Pod deletion recovery |
| Worker gateway reroute | `100/100`, old gateway force-deleted, Worker reconnected through a new gateway, outbox reroute observed |
| Web/API Service load balancing | `96` in-cluster requests, `4 / 4` Server Pods reached, coverage ratio `1.0`, distribution index `94/100` |
| Evidence completeness | `26` passed cases, `0` failed cases |

Canonical local report repository path: `design/reports/kind-raft-ha-e2e-20260622.md`

Reproduce locally:

```bash
TIKEO_KIND_E2E_KEEP=0 \
TIKEO_KIND_E2E_REBUILD_SERVER=1 \
scripts/kind-raft-ha-e2e.sh
```

## Cross-language Worker soak follow-up

The post-release main commit `affb4605` adds a repeatable cross-language Worker soak gate to `deploy/smoke/cross-language-worker-parity-smoke.sh`. The follow-up workflow `.github/workflows/release-candidate-worker-soak.yml` exposes it as a manual release-candidate gate with configurable `ref`, `soak_seconds`, `soak_interval_seconds`, `rebuild_server`, and `skip_web` inputs. It is disabled by default in normal CI and can also be run locally:

```bash
TIKEO_CROSS_SKIP_WEB=1 \
TIKEO_CROSS_REBUILD_SERVER=0 \
TIKEO_CROSS_SOAK_SECONDS=120 \
TIKEO_CROSS_SOAK_INTERVAL_SECONDS=10 \
deploy/smoke/cross-language-worker-parity-smoke.sh
```

Short local evidence from the post-release follow-up run:

| Signal | Result |
| --- | ---: |
| Evidence directory | `.dev/reports/cross-language-workers-20260622T065243Z-596956` |
| Soak rounds | `2` |
| Dispatches | `8` total, Go/Rust/Python/Node |
| Succeeded / failed | `8 / 0` |
| Max duration | `2s` |
| Average duration | `2s` |
| Max queue pending | `0` |
| Max outbox pending | `0` |
| Minimum online workers | `7` |
| Verdict | ✅ passed |

Evidence files are written as `*-soak-summary.json`, `*-soak-summary.csv`, and `*-soak-metrics.jsonl` next to the parity report; the manual RC workflow uploads them as the `cross-language-worker-soak` artifact and writes key numbers to the GitHub step summary.

## Post-release evidence automation

The follow-up evidence scripts are intentionally outside the `v0.3.9` binary boundary unless a later tag includes them. They are the current main-branch way to close local handoff evidence for the three previously open areas:

```bash
./scripts/release-readiness-evidence.sh
```

| Area | Script | Evidence written | Boundary |
| --- | --- | --- | --- |
| Notification Center provider/e2e | `scripts/notification-provider-e2e-smoke.sh` | `REPORT.md`, `summary.json`, provider receipt JSONL, channel test responses, message/attempt/queue snapshots | Uses a local protocol-real HTTP mock provider; real scope-specific providers still need deployment-specific test-send evidence. |
| Migration CLI old-project chain | `scripts/migration-cli-full-chain-smoke.sh` | `REPORT.md`, generated legacy project, migrated Worker project, `.tikeo-migration/`, `code-apply-evidence.json`, reviewed import payloads | Proves CLI detection, bundle generation, local code/config apply, and reviewed import payload preparation locally; domain semantic equivalence still needs a representative business project. |
| Real cloud HA | `scripts/cloud-raft-ha-acceptance.sh` via `scripts/release-readiness-evidence.sh` | Cloud `REPORT.md` and `summary.json`, or an explicit `deferred_cloud_endpoint_missing` boundary report | Requires `TIKEO_CLOUD_HA_SERVER_URL`; local Kind evidence remains the substitute when no cloud target exists. |

## Migration CLI evidence

`tikeo-migrate` is release-ready as a review-first migration assistant. The release includes Linux, macOS Intel, macOS Apple Silicon, and Windows archives. The expected operator flow remains:

1. Run `tikeo-migrate plan` from the legacy project root.
2. Review `.tikeo-migration/manifest.json`, `jobs.tikeo.md`, `data-import-plan.json`, and generated Java patch guidance.
3. Run `tikeo-migrate apply --bundle ./.tikeo-migration` and compile/test the migrated Worker project.
4. Fill the generated `tikeo.worker.*` / `tikeo.management.*` endpoint and API-key placeholders in the migrated Spring config.
5. Import only reviewed `ready` jobs into staging through the console, Management API, or GitOps, then trigger at least one migrated job with a matching Tikeo Worker before cutover.

For local full-chain rehearsal without a real legacy project, run `scripts/migration-cli-full-chain-smoke.sh`; it creates a throwaway Spring Boot + XXL-JOB project, auto-exports from a local scheduler DB, runs local in-place `apply` on the legacy Worker project, verifies in-place Tikeo config placeholders, and archives reviewed import payloads.

Relevant docs: [Migrate from legacy schedulers](../integrations/migrating-from-legacy-schedulers).

## Notification Center evidence boundary

Notification Center is ready for local/staging acceptance when provider test-send evidence exists for each active provider family in the target environment. The implementation and docs cover channel-row secrets, provider-specific template types, list/drawer test actions, retry/DLQ evidence, and redaction. `scripts/notification-provider-e2e-smoke.sh` now proves the local delivery state machine end-to-end with one delivered loopback request and one forced dead-letter failure. Production readiness still depends on real scope/provider calls for the deployment environment.

Relevant docs: [Notifications](../user-guide/notifications), [Notification Center reference](../reference/notification-center), and [Product readiness acceptance checklist](./product-readiness-acceptance).

## Remaining work for the next owner

| Priority | Work | Stop condition |
| --- | --- | --- |
| P0 when cloud environment is available | Run real cloud HA acceptance with external DB, ingress/LB/WAF/TLS, NetworkPolicy, and managed database HA. | `scripts/cloud-raft-ha-acceptance.sh` report archived with `summary.json`, `REPORT.md`, cluster diagnostics, and explicit pass/fail notes. |
| P1 before next release candidate | Run the manual cross-language soak gate from `.github/workflows/release-candidate-worker-soak.yml` for longer than the short local proof. | `TIKEO_CROSS_SOAK_SECONDS=120` or longer produces the `cross-language-worker-soak` artifact with `failed=0`, stable `workersOnline`, bounded `queuePending`, and no growing `outboxPending`. |
| P1 before provider production sign-off | Execute real Notification Center provider test-send for the channels used by that deployment. Local protocol evidence can be produced with `scripts/notification-provider-e2e-smoke.sh`. | Provider response, message trace, retry/DLQ state, and redaction evidence archived. |
| P2 before broad migration promotion | Exercise `tikeo-migrate` against a representative legacy XXL-JOB or PowerJob project. Local rehearsal can be produced with `scripts/migration-cli-full-chain-smoke.sh`. | Dry-run apply plus at least one live staging trigger with behavior comparison. |
| P2 ongoing | Keep public docs and release evidence synced. | Docs build, docs contract tests, search/LLM indexes, README links, and release asset checks pass. |
