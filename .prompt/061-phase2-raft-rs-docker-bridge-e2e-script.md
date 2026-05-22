# 061 — Phase 2 raft-rs Docker bridge / K8s Service E2E script

## Context
Current raft-rs transport coverage:
- Route-level smoke validates `/api/v1/raft/append-entries` DTO/envelope/runtime-inbox path.
- Correct `x-tikee-raft-token` can bypass human session auth for internal transport only; wrong token is rejected via normal auth envelope.
- Runtime startup restores persisted HardState/log entries and clears stale fencing.
- In-process RawNode harness validates real election/fencing and membership ConfChange apply.

## Required next work
1. Add a developer E2E script (prefer `scripts/raft-bridge-e2e.sh`) that starts multiple tikee containers/services on Docker bridge networking, not host mode.
2. Use service/container DNS or explicit bridge IPs for `cluster.peers[*].endpoint`; keep `0.0.0.0` bind and `9998` port defaults.
3. Inject `TIKEE__CLUSTER__TRANSPORT_TOKEN` via environment/secret-like local variable; do not commit production secrets.
4. Smoke-check `/healthz`, `/api/v1/cluster`, `/api/v1/cluster/diagnostics`, and `/api/v1/raft/append-entries` behavior through the bridge network. If real election is not yet enabled by runtime policy, document that expected result is safe follower/runtime-inbox behavior, not production leadership.
5. Ensure script is idempotent and cleans up containers/networks on exit; do not use host networking.
6. Update design roadmap, `.memory/progress.md`, `.memory/session-log.md`, and create `.prompt/062-*.md` for the next Phase2 slice.
7. Run verification: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `cargo run -- --help`, `cd web && bun run typecheck && bun run build`; run the E2E script if local Docker is available, otherwise record the exact blocker.
8. Commit with Lore-style trailers and push to origin/main.
