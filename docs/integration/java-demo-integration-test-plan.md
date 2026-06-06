# Java demo integration test plan

## Goal

Verify that the tikeo server, Java SDK, and Java Spring worker demo can run together as one service framework:

1. The Java worker registers through the outbound Worker Tunnel and appears in server worker APIs.
2. API-triggered, broadcast-triggered, fixed-rate, cron, and workflow-materialized job dispatches reach Java processors by `processor_name`.
3. Java processor success and failure outcomes are persisted back to server instance status/log APIs.
4. The demo remains unit-testable without a live server.

## Scope

In scope:

- Local tikeo server with `config/dev.toml` or an equivalent temporary config.
- Java SDK modules under `sdks/java`.
- Java Spring demo under `examples/java/spring-boot3-worker-demo`.
- HTTP API automation for auth, workers, jobs, instances, logs, and workflow materialization.

Out of scope for this plan:

- Python/Node SDKs.
- Go SDK run-loop.
- Helm/K8s production deployment.
- External OIDC, TLS/mTLS, and external observability stacks.

Database compatibility is covered by the dedicated storage matrix in `docs/operations/database-compatibility-test-plan.md`; Java demo integration may keep using SQLite unless the test objective is full deployment matrix validation.

## Test matrix

| Case | Server feature | Java demo processor | Expected result |
| --- | --- | --- | --- |
| Worker registration | Worker Tunnel register/heartbeat | demo app startup | `/api/v1/workers` shows `spring-demo-worker` online with `java` and `spring-boot` capabilities. |
| API single job | `POST /jobs`, `POST /jobs/{id}:trigger` | `demo.echo` | Instance becomes `succeeded`; log contains echo payload handling. |
| API failure job | same as above | `demo.fail` | Instance becomes `failed`; result message is persisted through status/log evidence. |
| Broadcast job | `execution_mode=broadcast` | `demo.context` | Parent instance and per-worker attempt become `succeeded`. |
| Fixed-rate job | scheduler tick loop | `demo.heartbeat` | At least one fixed-rate instance is created and succeeds. |
| Cron job | scheduler tick loop | `demo.report` | At least one cron instance is created and succeeds. |
| Workflow job node | workflow create/run/materialize + dispatcher | `demo.workflow.step` | Materialized workflow node dispatches to Java worker and workflow instance reaches `succeeded`. |

## Automation strategy

Use `deploy/smoke/java-demo-integration-smoke.sh` as the executable verifier. The script should:

1. Start or reuse a local server.
2. Start the Java Spring demo with `TIKEO_WORKER_DRY_RUN=false`.
3. Login with the development init account.
4. Create uniquely named integration jobs and one workflow.
5. Trigger/materialize dispatch paths.
6. Poll server APIs until statuses reach expected terminal states.
7. Write a timestamped JSON report under `.dev/reports/`.

## Exit criteria

- Java demo unit tests pass.
- Java SDK tests pass.
- Smoke script exits 0 and emits a report with all cases passed.
- No source file exceeds 1500 lines.

## Latest execution status (2026-06-01)

| Case | Evidence | Result | Status |
| --- | --- | --- | --- |
| Java demo unit tests | `rtk bash -lc 'cd examples/java/spring-boot3-worker-demo && ./gradlew test --no-daemon'` | BUILD SUCCESSFUL | ✅ 通过 |
| Java SDK worker client targeted test | `rtk bash -lc 'cd sdks/java && ./gradlew :tikeo:test --tests net.tikeo.worker.client.GrpcTikeoWorkerClientTest --no-daemon'` | BUILD SUCCESSFUL | ✅ 通过 |
| Server + Java demo smoke | `.dev/reports/java-demo-20260601T033026Z-286798.json` | worker registration、single success/failure、broadcast、fixed_rate、cron、workflow job 全部 passed | ✅ 通过 |
| Shell/Python/JS/TS/Rhai script live matrix | 当前 smoke 未覆盖 | 需要后续补充脚本沙箱矩阵 live 用例 | ⏳ 待执行 |

## 2026-06-04 Follow-up: cross-language integration automation

Add a cross-language smoke harness that extends the existing Java demo checks:

1. Start tikeo server with isolated DB and ports.
2. Seed structured jobs/processors for Java Boot2/Boot3/Boot4, Go, and Rust demos.
3. Start all five worker demo families with explicit namespace/app/cluster/region/clientInstanceId and worker_pool labels.
4. Trigger Go/Rust/Java processor jobs and assert instance status plus task logs.
5. Restart server and assert worker visibility falls back to persisted `worker_sessions` snapshot before live reconnect.
6. Assert worker_pool scope filtering is identical for live and persisted workers.
7. Save all API responses and logs under `.dev/reports/cross-language-workers-<run-id>/`.

Do not use naming conventions as selectors; all matching must use structured fields, labels, or structured capabilities.
