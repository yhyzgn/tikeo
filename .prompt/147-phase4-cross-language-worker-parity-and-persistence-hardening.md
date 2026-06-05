# Phase 4 Cross-language Worker parity and persistence hardening

## Current baseline

The 2026-06-04 slice completed manual parity and visibility fixes after the Java multi-worker integration work:

- Java demos are split into Spring Boot 2/3/4 projects with structured namespace/app/cluster/region/worker_pool configuration.
- Go SDK/demo now uses official gRPC/protobuf Worker Tunnel, defaults to live connection, has reconnect loop, emits assignment-token task logs, and documents `protoc` plus Dockerfile installation requirements.
- Rust SDK/demo now defaults to live connection, has reconnect loop, emits assignment-token task logs, and aligns script runner capabilities with Java sandbox naming.
- Worker visibility is no longer memory-only: `worker_sessions` persists capabilities, structuredCapabilities, labels, and master snapshots; `/api/v1/workers` merges live registry with persisted online sessions.
- Web Workers page groups by namespace/app and cluster/region; dispatch queue is on `/workers/dispatch-queue`.
- GitHub Actions CI run `26947829951` succeeded after the slice.

## Goal

Turn the current manual Java/Go/Rust worker parity and persisted worker visibility checks into repeatable automation, then harden any drift found by that automation.

## Required work

1. Build an executable cross-language integration harness for:
   - Java Spring Boot 2 demo
   - Java Spring Boot 3 demo
   - Java Spring Boot 4 demo
   - Go worker demo
   - Rust worker demo
2. Seed or reuse structured DB/API fixtures for all demos:
   - namespace/app/cluster/region/clientInstanceId
   - worker_pool label
   - processorName / processorType
   - script runner capability/sandbox metadata
3. Verify task execution and logs:
   - trigger at least one SDK processor job per language family
   - assert instance success/failure semantics
   - assert instance logs include received/completed entries for Go and Rust
   - assert Worker log/result writes include assignment token and are not accepted by convention-only fallback
4. Verify persistence after server restart:
   - capture workers before restart
   - restart server while demo workers are still running or reconnecting
   - assert `/api/v1/workers` can show persisted online snapshots before live registry is fully rebuilt
   - assert live registry supersedes snapshot after reconnect
5. Verify scope filtering:
   - namespace/app/worker_pool filtering must produce the same result for live workers and persisted snapshot workers
   - no matching by clientInstanceId/jobId naming convention
6. Verify Web Worker grouping:
   - Workers page shows namespace/app -> cluster/region -> node list
   - master/follower visible
   - dispatch queue only appears in `/workers/dispatch-queue` or an intentional drawer/secondary surface
7. Store evidence under `.dev/reports/cross-language-workers-<run-id>/` and update:
   - `design/java-demo-multi-worker-integration-test-report.md`
   - `design/server-web-java-joint-executable-test-status-plan.md`
   - `design/server-web-java-joint-automation-test-plan.md`
   - `.memory/*`

## Constraints

- No important Worker visibility state may be memory-only.
- No convention-based matching; use structured fields, labels, or structured capabilities.
- Chinese i18n must be complete Chinese wording; English i18n must be English. Do not render mixed labels such as mixed zh/en label.
- Go/Rust SDK and demos should remain one-to-one with Java where feasible. If parity is impossible, document the precise difference and why.
- Keep source files <=1500 lines and keep module entry files as declarations/re-exports only.

## Verification commands

Run targeted checks first, then broader checks before commit:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features -- --test-threads=1
cargo build --workspace --all-features
cd web && bun install --frozen-lockfile && bun run lint && bun run typecheck && bun test && bun run build
cd sdks/java && ./gradlew test jar sourcesJar
cd sdks/go/tikee && go test ./...
cd examples/go/worker-demo && go test ./...
cd sdks/rust/tikee && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features && cargo package --allow-dirty
cd examples/rust/worker-demo && cargo test
```

If the new harness is added, also run it locally and preserve the report path in the final response and docs.

## Completion requirements

- All new harness/test evidence is written to `.dev/reports/`.
- Design/test docs and `.memory` are updated.
- GitHub Actions CI is green after push.
- Commit message follows the Lore protocol trailers.
